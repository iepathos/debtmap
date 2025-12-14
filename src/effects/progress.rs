//! Progress effects and combinators for composable progress reporting.
//!
//! This module provides combinators that integrate progress reporting with
//! the stillwater effect system. These combinators enable:
//!
//! - **Separation of concerns**: Pure computation logic doesn't call progress directly
//! - **Composability**: Progress reporting composes naturally with other effects
//! - **Testability**: Use test mocks to verify progress behavior
//! - **Automatic cleanup**: Stages are completed even on error (bracket pattern)
//!
//! # Available Combinators
//!
//! | Combinator | Use Case |
//! |------------|----------|
//! | [`with_stage`] | Wrap effect with start/complete stage tracking |
//! | [`traverse_with_progress`] | Process items sequentially with progress |
//! | [`par_traverse_with_progress`] | Process items in parallel with atomic progress |
//! | [`report_progress`] | Emit a single progress update |
//!
//! # Example: Sequential Processing with Progress
//!
//! ```rust,ignore
//! use debtmap::effects::progress::{with_stage, traverse_with_progress};
//!
//! fn analyze_all_files(files: Vec<PathBuf>) -> AnalysisEffect<Vec<FileMetrics>> {
//!     with_stage("Analysis Pipeline",
//!         traverse_with_progress(files, "File Analysis", |path| {
//!             analyze_file_effect(path)
//!         })
//!     )
//! }
//! ```
//!
//! # Example: Testing Progress Reporting
//!
//! ```rust,ignore
//! use debtmap::progress::implementations::RecordingProgressSink;
//! use debtmap::effects::progress::with_stage;
//!
//! #[tokio::test]
//! async fn test_progress_reporting() {
//!     let recorder = Arc::new(RecordingProgressSink::new());
//!     let env = RealEnv::with_progress(config, recorder.clone());
//!
//!     let effect = with_stage("Test Stage", effect_pure(42));
//!     let result = effect.run(&env).await;
//!
//!     assert!(result.is_ok());
//!     assert_eq!(recorder.stages(), vec!["Test Stage"]);
//!     assert_eq!(recorder.completed_stages(), vec!["Test Stage"]);
//! }
//! ```
//!
//! # Thread Safety
//!
//! The [`par_traverse_with_progress`] combinator uses `AtomicUsize` for
//! thread-safe progress counting. Progress updates may arrive out of order,
//! but the final count will be correct.

use crate::errors::AnalysisError;
use crate::progress::traits::HasProgress;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use stillwater::effect::prelude::*;
use stillwater::Effect;

/// Wrap an effect with stage tracking.
///
/// This combinator automatically calls `start_stage` before the effect runs
/// and `complete_stage` after it completes, even if the effect fails.
/// This implements the bracket pattern for resource management.
///
/// # Arguments
///
/// * `stage_name` - The name of the stage to track
/// * `effect` - The inner effect to wrap
///
/// # Returns
///
/// An effect that tracks the stage and returns the inner effect's result.
///
/// # Example
///
/// ```rust,ignore
/// let effect = with_stage("Call Graph", build_call_graph_effect(files));
/// // Equivalent to:
/// // 1. env.progress().start_stage("Call Graph")
/// // 2. let result = build_call_graph_effect(files).run(&env).await
/// // 3. env.progress().complete_stage("Call Graph")
/// // 4. return result
/// ```
///
/// # Error Handling
///
/// The stage is completed even if the inner effect fails, ensuring consistent
/// progress reporting regardless of success or failure.
pub fn with_stage<T, Err, Env, Eff>(
    stage_name: &str,
    effect: Eff,
) -> impl Effect<Output = T, Error = Err, Env = Env>
where
    Env: HasProgress + Clone + Send + Sync + 'static,
    T: Send + 'static,
    Err: Send + 'static,
    Eff: Effect<Output = T, Error = Err, Env = Env> + Send + 'static,
{
    let start_name = stage_name.to_string();
    let end_name = stage_name.to_string();

    from_async(move |env: &Env| {
        let env = env.clone();
        let start = start_name.clone();
        let end = end_name.clone();

        async move {
            env.progress().start_stage(&start);
            let result = effect.run(&env).await;
            env.progress().complete_stage(&end);
            result
        }
    })
}

/// Traverse items with automatic progress reporting.
///
/// This combinator processes items sequentially, reporting progress after
/// each item completes. The stage is automatically started at the beginning
/// and completed at the end.
///
/// # Arguments
///
/// * `items` - The items to process
/// * `stage_name` - The name of the stage for progress reporting
/// * `f` - A function that creates an effect for each item
///
/// # Returns
///
/// An effect that produces a vector of results.
///
/// # Example
///
/// ```rust,ignore
/// let files = vec!["a.rs", "b.rs", "c.rs"];
/// let effect = traverse_with_progress(
///     files,
///     "File Analysis",
///     |path| analyze_file_effect(path)
/// );
///
/// // Progress: File Analysis 0/3, 1/3, 2/3, 3/3
/// let results = effect.run(&env).await?;
/// ```
///
/// # Error Handling
///
/// If any item fails, the traversal stops and returns the error.
/// The stage is still completed for proper cleanup.
pub fn traverse_with_progress<T, U, Env, F, Eff>(
    items: Vec<T>,
    stage_name: &str,
    f: F,
) -> impl Effect<Output = Vec<U>, Error = AnalysisError, Env = Env>
where
    T: Send + 'static,
    U: Send + 'static,
    Env: HasProgress + Clone + Send + Sync + 'static,
    F: Fn(T) -> Eff + Send + Sync + 'static,
    Eff: Effect<Output = U, Error = AnalysisError, Env = Env> + Send,
{
    let name = stage_name.to_string();
    let total = items.len();

    from_async(move |env: &Env| {
        let env = env.clone();
        let stage = name.clone();

        async move {
            env.progress().start_stage(&stage);
            let mut results = Vec::with_capacity(total);

            for (i, item) in items.into_iter().enumerate() {
                env.progress().report(&stage, i, total);
                match f(item).run(&env).await {
                    Ok(result) => results.push(result),
                    Err(e) => {
                        env.progress().complete_stage(&stage);
                        return Err(e);
                    }
                }
            }

            env.progress().complete_stage(&stage);
            Ok(results)
        }
    })
}

/// Parallel traverse with atomic progress counter.
///
/// This combinator processes items with an atomic counter for thread-safe
/// progress reporting. Progress updates may arrive out of order, but the
/// final count will be accurate.
///
/// # Arguments
///
/// * `items` - The items to process
/// * `stage_name` - The name of the stage for progress reporting
/// * `f` - A function that creates an effect for each item
///
/// # Returns
///
/// An effect that produces a vector of results.
///
/// # Implementation Note
///
/// This version processes items sequentially with atomic progress counting.
/// For true parallelism with rayon, the caller should use `rayon::par_iter()`
/// directly and report progress manually.
///
/// # Example
///
/// ```rust,ignore
/// let files = discover_source_files(path);
/// let effect = par_traverse_with_progress(
///     files,
///     "Parallel Analysis",
///     |path| analyze_file_effect(path)
/// );
///
/// let results = effect.run(&env).await?;
/// ```
///
/// # Thread Safety
///
/// The atomic counter ensures correct progress counts even with concurrent
/// updates from multiple threads.
pub fn par_traverse_with_progress<T, U, Env, F, Eff>(
    items: Vec<T>,
    stage_name: &str,
    f: F,
) -> impl Effect<Output = Vec<U>, Error = AnalysisError, Env = Env>
where
    T: Send + 'static,
    U: Send + 'static,
    Env: HasProgress + Clone + Send + Sync + 'static,
    F: Fn(T) -> Eff + Send + Sync + Clone + 'static,
    Eff: Effect<Output = U, Error = AnalysisError, Env = Env> + Send,
{
    let name = stage_name.to_string();
    let total = items.len();

    from_async(move |env: &Env| {
        let env = env.clone();
        let stage = name.clone();
        let f = f.clone();

        async move {
            env.progress().start_stage(&stage);
            let counter = Arc::new(AtomicUsize::new(0));

            let mut results = Vec::with_capacity(total);

            // Process sequentially with atomic progress tracking
            // This maintains the same API as a parallel version would have
            for item in items {
                let current = counter.fetch_add(1, Ordering::Relaxed);
                env.progress().report(&stage, current, total);
                match f(item).run(&env).await {
                    Ok(result) => results.push(result),
                    Err(e) => {
                        env.progress().complete_stage(&stage);
                        return Err(e);
                    }
                }
            }

            env.progress().complete_stage(&stage);
            Ok(results)
        }
    })
}

/// Report progress for current operation.
///
/// This is a low-level combinator for direct progress reporting.
/// For most use cases, prefer [`traverse_with_progress`] or [`with_stage`].
///
/// # Arguments
///
/// * `stage` - The name of the current stage
/// * `current` - The current progress count (0-indexed)
/// * `total` - The total number of items
///
/// # Returns
///
/// An effect that reports progress and returns `()`.
///
/// # Example
///
/// ```rust,ignore
/// let effect = report_progress("Custom Stage", 5, 10)
///     .and_then(|_| do_work());
/// ```
pub fn report_progress<Env>(
    stage: &str,
    current: usize,
    total: usize,
) -> impl Effect<Output = (), Error = AnalysisError, Env = Env>
where
    Env: HasProgress + Clone + Send + Sync + 'static,
{
    let stage = stage.to_string();
    stillwater::asks(move |env: &Env| {
        env.progress().report(&stage, current, total);
    })
}

/// Report a warning via progress.
///
/// This combinator emits a warning through the progress system without
/// interrupting the current operation.
///
/// # Arguments
///
/// * `message` - The warning message
///
/// # Returns
///
/// An effect that reports the warning and returns `()`.
///
/// # Example
///
/// ```rust,ignore
/// let effect = warn_progress("File skipped due to encoding issues")
///     .and_then(|_| continue_processing());
/// ```
pub fn warn_progress<Env>(
    message: &str,
) -> impl Effect<Output = (), Error = AnalysisError, Env = Env>
where
    Env: HasProgress + Clone + Send + Sync + 'static,
{
    let message = message.to_string();
    stillwater::asks(move |env: &Env| {
        env.progress().warn(&message);
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::env::RealEnv;
    use crate::progress::implementations::{ProgressEvent, RecordingProgressSink};
    use stillwater::EffectExt;

    // Helper to create a test environment with recording sink
    fn test_env() -> (RealEnv, Arc<RecordingProgressSink>) {
        let recorder = Arc::new(RecordingProgressSink::new());
        let env = RealEnv::with_progress(crate::config::DebtmapConfig::default(), recorder.clone());
        (env, recorder)
    }

    #[tokio::test]
    async fn test_with_stage_calls_start_and_complete() {
        let (env, recorder) = test_env();

        let effect = with_stage("Test Stage", pure::<_, AnalysisError, _>(42));
        let result = effect.run(&env).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
        assert_eq!(recorder.stages(), vec!["Test Stage"]);
        assert_eq!(recorder.completed_stages(), vec!["Test Stage"]);
    }

    #[tokio::test]
    async fn test_with_stage_completes_on_error() {
        let (env, recorder) = test_env();

        let effect = with_stage(
            "Failing Stage",
            fail::<i32, AnalysisError, RealEnv>(AnalysisError::other("test error")),
        );
        let result = effect.run(&env).await;

        assert!(result.is_err());
        // Stage should still be completed (bracket cleanup)
        assert_eq!(recorder.stages(), vec!["Failing Stage"]);
        assert_eq!(recorder.completed_stages(), vec!["Failing Stage"]);
    }

    #[tokio::test]
    async fn test_traverse_with_progress_reports_each_item() {
        let (env, recorder) = test_env();

        let items = vec![1, 2, 3];
        let effect = traverse_with_progress(items, "Processing", |n| pure(n * 2));
        let result = effect.run(&env).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), vec![2, 4, 6]);

        // Check stage lifecycle
        assert_eq!(recorder.stages(), vec!["Processing"]);
        assert_eq!(recorder.completed_stages(), vec!["Processing"]);

        // Check progress reports
        let reports: Vec<_> = recorder
            .events()
            .into_iter()
            .filter(|e| matches!(e, ProgressEvent::Report { .. }))
            .collect();
        assert_eq!(reports.len(), 3);
    }

    #[tokio::test]
    async fn test_traverse_with_progress_completes_on_error() {
        let (env, recorder) = test_env();

        let items = vec![1, 2, 3];
        let effect = traverse_with_progress(items, "Failing", |n| {
            if n == 2 {
                fail::<i32, AnalysisError, RealEnv>(AnalysisError::other("failed at 2")).boxed()
            } else {
                pure::<_, AnalysisError, RealEnv>(n).boxed()
            }
        });
        let result = effect.run(&env).await;

        assert!(result.is_err());
        // Stage should still be completed
        assert_eq!(recorder.stages(), vec!["Failing"]);
        assert_eq!(recorder.completed_stages(), vec!["Failing"]);
    }

    #[tokio::test]
    async fn test_report_progress_emits_event() {
        let (env, recorder) = test_env();

        let effect = report_progress("Manual", 5, 10);
        let result = effect.run(&env).await;

        assert!(result.is_ok());

        let events = recorder.events();
        assert_eq!(events.len(), 1);
        assert!(matches!(
            &events[0],
            ProgressEvent::Report { stage, current: 5, total: 10 } if stage == "Manual"
        ));
    }

    #[tokio::test]
    async fn test_warn_progress_emits_warning() {
        let (env, recorder) = test_env();

        let effect = warn_progress("Test warning message");
        let result = effect.run(&env).await;

        assert!(result.is_ok());

        let warnings = recorder.warnings();
        assert_eq!(warnings, vec!["Test warning message"]);
    }

    #[tokio::test]
    async fn test_nested_stages() {
        let (env, recorder) = test_env();

        let inner = with_stage("Inner", pure::<_, AnalysisError, _>(1));
        let outer = with_stage("Outer", inner.map(|n| n + 1));
        let result = outer.run(&env).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 2);

        // Both stages should be tracked
        let stages = recorder.stages();
        assert!(stages.contains(&"Outer".to_string()));
        assert!(stages.contains(&"Inner".to_string()));
    }

    #[tokio::test]
    async fn test_par_traverse_with_progress() {
        let (env, recorder) = test_env();

        let items = vec![1, 2, 3, 4, 5];
        let effect = par_traverse_with_progress(items, "Parallel", |n| pure(n * 2));
        let result = effect.run(&env).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), vec![2, 4, 6, 8, 10]);

        // Stage lifecycle should be tracked
        assert_eq!(recorder.stages(), vec!["Parallel"]);
        assert_eq!(recorder.completed_stages(), vec!["Parallel"]);

        // Should have 5 progress reports
        let reports: Vec<_> = recorder
            .events()
            .into_iter()
            .filter(|e| matches!(e, ProgressEvent::Report { .. }))
            .collect();
        assert_eq!(reports.len(), 5);
    }
}
