//! Thread-local context tracking for crash reports.
//!
//! Provides context information about the current analysis phase and file
//! being processed. Uses thread-local storage for per-thread context (works
//! with rayon parallel iterators) and atomic counters for global progress.
//!
//! ## Thread Safety
//!
//! - Thread-local context: Each thread has its own context (via `thread_local!`)
//! - Global progress: Atomic counters for files processed/total
//! - Context guards use RAII for automatic cleanup on drop

use std::cell::RefCell;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Global progress counters (atomic for thread-safety)
static FILES_PROCESSED: AtomicUsize = AtomicUsize::new(0);
static FILES_TOTAL: AtomicUsize = AtomicUsize::new(0);

// Thread-local context for the current operation
// pub(crate) allows the parallel module to access this for context propagation
thread_local! {
    pub(crate) static CURRENT_CONTEXT: RefCell<AnalysisContext> = const { RefCell::new(AnalysisContext::new()) };
}

/// Context snapshot for the current analysis operation.
///
/// Captures information about what debtmap was doing when an error occurred.
#[derive(Debug, Clone, Default)]
pub struct AnalysisContext {
    /// Current analysis phase
    pub phase: Option<AnalysisPhase>,
    /// File currently being analyzed
    pub current_file: Option<PathBuf>,
    /// Function currently being analyzed (if applicable)
    pub current_function: Option<String>,
}

impl AnalysisContext {
    /// Create a new empty context.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            phase: None,
            current_file: None,
            current_function: None,
        }
    }
}

/// Analysis phases for context tracking.
///
/// These phases represent the major stages of debtmap analysis,
/// helping identify where in the pipeline a crash occurred.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnalysisPhase {
    /// Discovering files to analyze
    FileDiscovery,
    /// Parsing source files into ASTs
    Parsing,
    /// Building the call graph
    CallGraphBuilding,
    /// Analyzing function purity
    PurityAnalysis,
    /// Loading coverage data from LCOV files
    CoverageLoading,
    /// Scoring technical debt items
    DebtScoring,
    /// Prioritizing debt items by impact
    Prioritization,
    /// Generating output reports
    OutputGeneration,
}

impl std::fmt::Display for AnalysisPhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FileDiscovery => write!(f, "file_discovery"),
            Self::Parsing => write!(f, "parsing"),
            Self::CallGraphBuilding => write!(f, "call_graph_building"),
            Self::PurityAnalysis => write!(f, "purity_analysis"),
            Self::CoverageLoading => write!(f, "coverage_loading"),
            Self::DebtScoring => write!(f, "debt_scoring"),
            Self::Prioritization => write!(f, "prioritization"),
            Self::OutputGeneration => write!(f, "output_generation"),
        }
    }
}

/// RAII guard for restoring analysis context on drop.
///
/// When the guard is dropped, it restores the previous context,
/// enabling nested context tracking (e.g., file within phase).
pub struct ContextGuard {
    previous: AnalysisContext,
}

impl Drop for ContextGuard {
    fn drop(&mut self) {
        CURRENT_CONTEXT.with(|ctx| {
            *ctx.borrow_mut() = self.previous.clone();
        });
    }
}

/// Set the current analysis phase.
///
/// Returns a guard that restores the previous phase on drop.
///
/// # Example
///
/// ```ignore
/// let _phase = set_phase(AnalysisPhase::Parsing);
/// // Do parsing work...
/// // Phase automatically restored when _phase drops
/// ```
#[must_use]
pub fn set_phase(phase: AnalysisPhase) -> ContextGuard {
    CURRENT_CONTEXT.with(|ctx| {
        let previous = ctx.borrow().clone();
        ctx.borrow_mut().phase = Some(phase);
        ContextGuard { previous }
    })
}

/// Set the current analysis phase without returning a guard.
///
/// Unlike `set_phase`, this function does not restore the previous phase
/// when the function returns. Use this for top-level phase transitions
/// where you want the phase to persist until explicitly changed.
///
/// # Example
///
/// ```ignore
/// // Phase persists until next set_phase_persistent call
/// set_phase_persistent(AnalysisPhase::CallGraphBuilding);
/// // ... do work ...
/// set_phase_persistent(AnalysisPhase::DebtScoring);
/// ```
pub fn set_phase_persistent(phase: AnalysisPhase) {
    CURRENT_CONTEXT.with(|ctx| {
        ctx.borrow_mut().phase = Some(phase);
    });
}

/// Set the current file being analyzed.
///
/// Returns a guard that restores the previous file on drop.
///
/// # Example
///
/// ```ignore
/// for file in files {
///     let _file_guard = set_current_file(&file);
///     analyze_file(&file)?;
///     // File context restored when _file_guard drops
/// }
/// ```
#[must_use]
pub fn set_current_file(path: impl Into<PathBuf>) -> ContextGuard {
    CURRENT_CONTEXT.with(|ctx| {
        let previous = ctx.borrow().clone();
        ctx.borrow_mut().current_file = Some(path.into());
        ContextGuard { previous }
    })
}

/// Set the current function being analyzed.
///
/// Returns a guard that restores the previous function on drop.
#[must_use]
pub fn set_current_function(name: impl Into<String>) -> ContextGuard {
    CURRENT_CONTEXT.with(|ctx| {
        let previous = ctx.borrow().clone();
        ctx.borrow_mut().current_function = Some(name.into());
        ContextGuard { previous }
    })
}

/// Set the progress counters (processed and total files).
///
/// Thread-safe; can be called from any thread.
pub fn set_progress(processed: usize, total: usize) {
    FILES_PROCESSED.store(processed, Ordering::Relaxed);
    FILES_TOTAL.store(total, Ordering::Relaxed);
}

/// Increment the processed file count.
///
/// Thread-safe; can be called from parallel iterators.
pub fn increment_processed() {
    FILES_PROCESSED.fetch_add(1, Ordering::Relaxed);
}

/// Get the current context snapshot.
///
/// Called by the panic hook to include context in crash reports.
#[must_use]
pub fn get_current_context() -> AnalysisContext {
    CURRENT_CONTEXT.with(|ctx| ctx.borrow().clone())
}

/// Get the current progress (processed, total).
///
/// Called by the panic hook to show progress in crash reports.
#[must_use]
pub fn get_progress() -> (usize, usize) {
    (
        FILES_PROCESSED.load(Ordering::Relaxed),
        FILES_TOTAL.load(Ordering::Relaxed),
    )
}

/// Reset progress counters to zero.
///
/// Useful for testing or between analysis runs.
pub fn reset_progress() {
    FILES_PROCESSED.store(0, Ordering::Relaxed);
    FILES_TOTAL.store(0, Ordering::Relaxed);
}

/// Reset the current thread's context to empty.
///
/// Useful for testing.
pub fn reset_context() {
    CURRENT_CONTEXT.with(|ctx| {
        *ctx.borrow_mut() = AnalysisContext::new();
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_guard_restores_previous() {
        reset_context();

        let _phase1 = set_phase(AnalysisPhase::Parsing);
        assert_eq!(
            get_current_context().phase,
            Some(AnalysisPhase::Parsing),
            "Phase should be Parsing"
        );

        {
            let _phase2 = set_phase(AnalysisPhase::DebtScoring);
            assert_eq!(
                get_current_context().phase,
                Some(AnalysisPhase::DebtScoring),
                "Phase should be DebtScoring"
            );
        }

        // Should restore to Parsing after inner guard drops
        assert_eq!(
            get_current_context().phase,
            Some(AnalysisPhase::Parsing),
            "Phase should be restored to Parsing"
        );
    }

    #[test]
    fn test_nested_context_guards() {
        reset_context();

        let _phase = set_phase(AnalysisPhase::Parsing);
        let _file = set_current_file("/path/to/test.rs");
        let _func = set_current_function("test_function");

        let ctx = get_current_context();
        assert_eq!(ctx.phase, Some(AnalysisPhase::Parsing));
        assert_eq!(ctx.current_file, Some(PathBuf::from("/path/to/test.rs")));
        assert_eq!(ctx.current_function, Some("test_function".to_string()));
    }

    #[test]
    fn test_progress_tracking() {
        reset_progress();

        set_progress(50, 100);
        let (processed, total) = get_progress();
        assert_eq!(processed, 50);
        assert_eq!(total, 100);
    }

    #[test]
    fn test_increment_processed() {
        reset_progress();

        set_progress(0, 100);
        increment_processed();
        increment_processed();
        increment_processed();

        let (processed, total) = get_progress();
        assert_eq!(processed, 3);
        assert_eq!(total, 100);
    }

    #[test]
    fn test_analysis_phase_display() {
        assert_eq!(
            format!("{}", AnalysisPhase::FileDiscovery),
            "file_discovery"
        );
        assert_eq!(format!("{}", AnalysisPhase::Parsing), "parsing");
        assert_eq!(
            format!("{}", AnalysisPhase::CallGraphBuilding),
            "call_graph_building"
        );
        assert_eq!(
            format!("{}", AnalysisPhase::PurityAnalysis),
            "purity_analysis"
        );
        assert_eq!(
            format!("{}", AnalysisPhase::CoverageLoading),
            "coverage_loading"
        );
        assert_eq!(format!("{}", AnalysisPhase::DebtScoring), "debt_scoring");
        assert_eq!(
            format!("{}", AnalysisPhase::Prioritization),
            "prioritization"
        );
        assert_eq!(
            format!("{}", AnalysisPhase::OutputGeneration),
            "output_generation"
        );
    }

    #[test]
    fn test_empty_context_by_default() {
        reset_context();

        let ctx = get_current_context();
        assert!(ctx.phase.is_none());
        assert!(ctx.current_file.is_none());
        assert!(ctx.current_function.is_none());
    }
}
