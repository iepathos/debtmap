//! Parallel context propagation for rayon iterators.
//!
//! This module provides utilities for propagating tracing spans and analysis context
//! into rayon worker threads. Without explicit propagation, context is lost because
//! rayon's thread pool doesn't inherit thread-local storage from the calling thread.
//!
//! ## Problem
//!
//! Rayon uses a thread pool. When you call `par_iter()`, the closure runs on
//! arbitrary worker threads, not the calling thread. This means:
//!
//! 1. Thread-locals (like our AnalysisContext) aren't inherited
//! 2. Tracing's current span isn't inherited
//! 3. The worker has no knowledge of the parent's context
//!
//! ## Solution
//!
//! Capture context before entering parallel section, then explicitly enter it in each worker.
//!
//! ## Usage
//!
//! ```ignore
//! use debtmap::observability::parallel::{ParallelContext, process_file_with_context};
//!
//! // Capture context before parallel execution
//! let ctx = ParallelContext::capture();
//!
//! files
//!     .par_iter()
//!     .map(|path| {
//!         process_file_with_context(path, &ctx, || {
//!             // Context propagated from parent thread
//!             // If panic here, crash report shows file being processed
//!             analyze_file(path)
//!         })
//!     })
//!     .collect()
//! ```
//!
//! ## Performance Considerations
//!
//! - Context capture is cheap (clone a few pointers)
//! - Context entry is cheap (set thread-local, enter span)
//! - Main overhead is the per-item closure call
//! - For large items, overhead is negligible
//! - For tiny items (e.g., summing numbers), use raw par_iter
//!
//! ## When to Use Context Propagation
//!
//! **Use context propagation** when:
//! - Processing files or significant work items
//! - Debugging/observability is important
//! - Crash context is valuable
//!
//! **Use raw `map`** when:
//! - Processing trivial items (numbers, small transforms)
//! - Maximum performance is critical
//! - No need for crash context

use super::context::{get_current_context, increment_processed, set_current_file, AnalysisContext};
use rayon::prelude::*;
use std::path::Path;
use tracing::{debug_span, Span};

// Re-export thread-local for tests
thread_local! {
    /// Thread-local storage for the current analysis context.
    /// This allows each rayon worker to have its own copy of the context.
    pub(crate) static PARALLEL_CONTEXT: std::cell::RefCell<Option<AnalysisContext>> =
        const { std::cell::RefCell::new(None) };
}

/// Combined context for parallel propagation.
///
/// Captures both the tracing span and analysis context for propagation
/// into rayon worker threads.
#[derive(Clone)]
pub struct ParallelContext {
    /// Tracing span to propagate
    span: Span,
    /// Analysis context to propagate
    analysis_context: AnalysisContext,
}

impl ParallelContext {
    /// Capture current context for propagation.
    ///
    /// Call this on the main thread before entering a parallel section.
    /// The captured context can then be entered in each worker thread.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let _phase = set_phase(AnalysisPhase::Parsing);
    /// let ctx = ParallelContext::capture();
    ///
    /// files.par_iter().for_each(|file| {
    ///     let _guard = ctx.enter();
    ///     // Context is now available in this worker
    /// });
    /// ```
    #[must_use]
    pub fn capture() -> Self {
        Self {
            span: Span::current(),
            analysis_context: get_current_context(),
        }
    }

    /// Enter this context in the current thread.
    ///
    /// Returns a guard that restores the previous context on drop.
    /// Use this at the start of each parallel task.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let ctx = ParallelContext::capture();
    ///
    /// files.par_iter().for_each(|file| {
    ///     let _guard = ctx.enter();
    ///     // Work with context...
    /// });
    /// ```
    #[must_use]
    pub fn enter(&self) -> ParallelContextGuard {
        let span_guard = self.span.clone().entered();

        // Set thread-local analysis context
        super::context::CURRENT_CONTEXT.with(|ctx| {
            *ctx.borrow_mut() = self.analysis_context.clone();
        });

        ParallelContextGuard { _span: span_guard }
    }

    /// Get the captured analysis context.
    ///
    /// Useful for inspecting what context was captured.
    #[must_use]
    pub fn analysis_context(&self) -> &AnalysisContext {
        &self.analysis_context
    }

    /// Get the captured span.
    ///
    /// Useful for creating child spans.
    #[must_use]
    pub fn span(&self) -> &Span {
        &self.span
    }
}

/// RAII guard for entered parallel context.
///
/// When dropped, the span exit is handled automatically.
/// Thread-local context remains until next `enter()` call on this thread.
pub struct ParallelContextGuard {
    _span: tracing::span::EnteredSpan,
}

/// Execute closure with propagated context.
///
/// This is a convenience function for the common pattern of entering
/// context and immediately executing a closure.
///
/// # Example
///
/// ```ignore
/// let ctx = ParallelContext::capture();
///
/// files.par_iter().map(|file| {
///     with_parallel_context(&ctx, || analyze_file(file))
/// }).collect()
/// ```
#[inline]
pub fn with_parallel_context<T, F>(ctx: &ParallelContext, f: F) -> T
where
    F: FnOnce() -> T,
{
    let _guard = ctx.enter();
    f()
}

/// Process a file with full context setup.
///
/// This combines:
/// 1. Entering the parent context (span + analysis context)
/// 2. Setting the current file in context
/// 3. Creating a debug span for the file
/// 4. Incrementing the processed count
///
/// Use this for the common pattern of processing files in parallel.
///
/// # Arguments
///
/// * `path` - The file being processed
/// * `parent_ctx` - The context captured from the parent thread
/// * `f` - The closure to execute
///
/// # Example
///
/// ```ignore
/// let ctx = ParallelContext::capture();
///
/// files.par_iter().map(|path| {
///     process_file_with_context(path, &ctx, || {
///         // If panic here, crash report shows:
///         // - Phase from parent context
///         // - File: /path/to/current/file.rs
///         // - Span: parent_span > process_file
///         analyze_file(path)
///     })
/// }).collect()
/// ```
pub fn process_file_with_context<T, F>(path: &Path, parent_ctx: &ParallelContext, f: F) -> T
where
    F: FnOnce() -> T,
{
    let _parent = parent_ctx.enter();
    let _file = set_current_file(path);
    let _span = debug_span!("process_file", path = %path.display()).entered();

    increment_processed();

    f()
}

/// Extension trait for parallel iterators with full context propagation.
///
/// This trait provides convenience methods for common parallel patterns
/// with automatic context propagation.
///
/// # Example
///
/// ```ignore
/// use debtmap::observability::parallel::ParallelContextExt;
///
/// files
///     .par_iter()
///     .map_with_context(|path| {
///         // Context propagated from parent thread
///         analyze_file(path)
///     })
///     .collect()
/// ```
pub trait ParallelContextExt<T>: ParallelIterator<Item = T> + Sized {
    /// Map with context propagation.
    ///
    /// Each item is processed with the parent context propagated.
    fn map_with_context<R, F>(self, f: F) -> impl ParallelIterator<Item = R>
    where
        F: Fn(T) -> R + Sync + Send,
        R: Send,
    {
        let ctx = ParallelContext::capture();
        self.map(move |item| with_parallel_context(&ctx, || f(item)))
    }

    /// Filter-map with context propagation.
    ///
    /// Each item is processed with the parent context propagated.
    fn filter_map_with_context<R, F>(self, f: F) -> impl ParallelIterator<Item = R>
    where
        F: Fn(T) -> Option<R> + Sync + Send,
        R: Send,
    {
        let ctx = ParallelContext::capture();
        self.filter_map(move |item| with_parallel_context(&ctx, || f(item)))
    }

    /// For-each with context propagation.
    ///
    /// Each item is processed with the parent context propagated.
    fn for_each_with_context<F>(self, f: F)
    where
        F: Fn(T) + Sync + Send,
    {
        let ctx = ParallelContext::capture();
        self.for_each(move |item| with_parallel_context(&ctx, || f(item)));
    }
}

impl<T, I: ParallelIterator<Item = T> + Sized> ParallelContextExt<T> for I {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::observability::context::{reset_context, reset_progress, set_phase, AnalysisPhase};
    use std::path::PathBuf;

    #[test]
    fn test_context_capture() {
        reset_context();
        reset_progress();

        let _phase = set_phase(AnalysisPhase::Parsing);
        let ctx = ParallelContext::capture();

        assert_eq!(
            ctx.analysis_context().phase,
            Some(AnalysisPhase::Parsing),
            "Captured context should have Parsing phase"
        );
    }

    #[test]
    fn test_context_propagates_to_workers() {
        reset_context();
        reset_progress();

        let _phase = set_phase(AnalysisPhase::DebtScoring);
        let ctx = ParallelContext::capture();

        let results: Vec<_> = (0..10)
            .into_par_iter()
            .map(|i| {
                let _guard = ctx.enter();
                // Context should be available
                let context = get_current_context();
                (i, context.phase)
            })
            .collect();

        for (_, phase) in results {
            assert_eq!(
                phase,
                Some(AnalysisPhase::DebtScoring),
                "Phase should propagate to workers"
            );
        }
    }

    #[test]
    fn test_file_context_per_item() {
        reset_context();
        reset_progress();

        let ctx = ParallelContext::capture();
        let files = vec![
            PathBuf::from("a.rs"),
            PathBuf::from("b.rs"),
            PathBuf::from("c.rs"),
        ];

        files.par_iter().for_each(|path| {
            process_file_with_context(path, &ctx, || {
                let context = get_current_context();
                assert_eq!(
                    context.current_file.as_ref(),
                    Some(path),
                    "Current file should be set in context"
                );
            });
        });
    }

    #[test]
    fn test_with_parallel_context_helper() {
        reset_context();
        reset_progress();

        let _phase = set_phase(AnalysisPhase::Parsing);
        let ctx = ParallelContext::capture();

        let result: i32 = (0..100)
            .into_par_iter()
            .map(|x| with_parallel_context(&ctx, || x * 2))
            .sum();

        assert_eq!(
            result, 9900,
            "Computation should work correctly with context"
        );
    }

    #[test]
    fn test_map_with_context_extension() {
        reset_context();
        reset_progress();

        let _phase = set_phase(AnalysisPhase::PurityAnalysis);

        let results: Vec<_> = (0..10)
            .into_par_iter()
            .map_with_context(|i| {
                let ctx = get_current_context();
                (i, ctx.phase)
            })
            .collect();

        for (_, phase) in results {
            assert_eq!(
                phase,
                Some(AnalysisPhase::PurityAnalysis),
                "map_with_context should propagate phase"
            );
        }
    }

    #[test]
    fn test_filter_map_with_context_extension() {
        reset_context();
        reset_progress();

        let _phase = set_phase(AnalysisPhase::CoverageLoading);

        let results: Vec<_> = (0..20)
            .into_par_iter()
            .filter_map_with_context(|i| {
                let ctx = get_current_context();
                if i % 2 == 0 {
                    Some((i, ctx.phase))
                } else {
                    None
                }
            })
            .collect();

        assert_eq!(results.len(), 10, "Should filter half the items");
        for (_, phase) in results {
            assert_eq!(
                phase,
                Some(AnalysisPhase::CoverageLoading),
                "filter_map_with_context should propagate phase"
            );
        }
    }

    #[test]
    fn test_for_each_with_context_extension() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        reset_context();
        reset_progress();

        let _phase = set_phase(AnalysisPhase::OutputGeneration);
        let count = AtomicUsize::new(0);

        (0..10).into_par_iter().for_each_with_context(|_| {
            let ctx = get_current_context();
            if ctx.phase == Some(AnalysisPhase::OutputGeneration) {
                count.fetch_add(1, Ordering::Relaxed);
            }
        });

        assert_eq!(
            count.load(Ordering::Relaxed),
            10,
            "All items should have correct phase"
        );
    }

    #[test]
    fn test_nested_context_in_parallel() {
        reset_context();
        reset_progress();

        let _phase = set_phase(AnalysisPhase::Parsing);
        let ctx = ParallelContext::capture();

        let results: Vec<_> = (0..5)
            .into_par_iter()
            .map(|i| {
                let _guard = ctx.enter();
                let _file = set_current_file(format!("file_{}.rs", i));

                let inner_ctx = get_current_context();
                (
                    i,
                    inner_ctx.phase,
                    inner_ctx
                        .current_file
                        .map(|p| p.to_string_lossy().to_string()),
                )
            })
            .collect();

        for (i, phase, file) in results {
            assert_eq!(phase, Some(AnalysisPhase::Parsing));
            assert_eq!(file, Some(format!("file_{}.rs", i)));
        }
    }
}
