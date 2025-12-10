//! Pure Guard Functions for Analysis Workflow (Spec 202)
//!
//! Guards are pure functions that determine whether a state transition is valid.
//! They take `&AnalysisState` and return `bool`, with no side effects.
//!
//! ## Design Principles
//!
//! - **Pure functions** - Same input always produces same output
//! - **No side effects** - Guards only read state, never modify it
//! - **Explicit dependencies** - Each guard documents what it checks
//! - **Testable** - Easy to unit test without mocking

use super::state::{AnalysisPhase, AnalysisState};

/// Guard: Can transition from Initialized to CallGraphBuilding?
///
/// Requires:
/// - Phase is Initialized
/// - Metrics are available (from parsing phase)
///
/// # Example
/// ```ignore
/// let state = AnalysisState::with_metrics(config, metrics);
/// assert!(can_start_call_graph(&state));
/// ```
pub fn can_start_call_graph(state: &AnalysisState) -> bool {
    matches!(state.phase, AnalysisPhase::Initialized) && state.results.metrics.is_some()
}

/// Guard: Can transition from CallGraphComplete to CoverageLoading?
///
/// Requires:
/// - Phase is CallGraphComplete
/// - Call graph has been built
/// - Coverage file is configured
pub fn can_start_coverage(state: &AnalysisState) -> bool {
    matches!(state.phase, AnalysisPhase::CallGraphComplete)
        && state.results.call_graph.is_some()
        && state.config.coverage_file.is_some()
}

/// Guard: Can skip coverage phase?
///
/// True when coverage is not configured and we should proceed without it.
///
/// Requires:
/// - Phase is CallGraphComplete
/// - No coverage file configured
pub fn can_skip_coverage(state: &AnalysisState) -> bool {
    matches!(state.phase, AnalysisPhase::CallGraphComplete) && state.config.coverage_file.is_none()
}

/// Guard: Can transition to PurityAnalyzing?
///
/// Requires:
/// - Phase is CoverageComplete (either loaded or skipped)
/// - Call graph is available
pub fn can_start_purity(state: &AnalysisState) -> bool {
    matches!(state.phase, AnalysisPhase::CoverageComplete) && state.results.call_graph.is_some()
}

/// Guard: Can transition to ContextLoading?
///
/// Requires:
/// - Phase is PurityComplete
/// - Purity analysis is complete (enriched metrics available)
/// - Context is enabled in config
pub fn can_start_context(state: &AnalysisState) -> bool {
    matches!(state.phase, AnalysisPhase::PurityComplete)
        && state.results.enriched_metrics.is_some()
        && state.config.enable_context
}

/// Guard: Can skip context loading?
///
/// True when context is not enabled and we should proceed without it.
///
/// Requires:
/// - Phase is PurityComplete
/// - Context is not enabled
pub fn can_skip_context(state: &AnalysisState) -> bool {
    matches!(state.phase, AnalysisPhase::PurityComplete) && !state.config.enable_context
}

/// Guard: Can start scoring?
///
/// Requires all dependencies to be complete:
/// - Phase is ContextComplete
/// - Call graph is available
/// - Enriched metrics (with purity) are available
pub fn can_start_scoring(state: &AnalysisState) -> bool {
    matches!(state.phase, AnalysisPhase::ContextComplete)
        && state.results.call_graph.is_some()
        && state.results.enriched_metrics.is_some()
}

/// Guard: Can start filtering and ranking?
///
/// Requires:
/// - Phase is ScoringComplete
/// - Unified analysis results are available
pub fn can_start_filtering(state: &AnalysisState) -> bool {
    matches!(state.phase, AnalysisPhase::ScoringComplete)
        && state.results.unified_analysis.is_some()
}

/// Guard: Can transition to Complete?
///
/// Requires:
/// - Phase is FilteringInProgress
/// - Unified analysis results are available with sorted items
pub fn can_complete(state: &AnalysisState) -> bool {
    matches!(state.phase, AnalysisPhase::FilteringInProgress)
        && state.results.unified_analysis.is_some()
}

/// Guard: Is the state valid for resuming from checkpoint?
///
/// Validates that the state has all required data for its current phase.
/// This is used when loading a checkpoint to ensure we can continue.
pub fn is_valid_checkpoint(state: &AnalysisState) -> bool {
    match state.phase {
        AnalysisPhase::Initialized => true,

        AnalysisPhase::CallGraphBuilding => state.results.metrics.is_some(),

        AnalysisPhase::CallGraphComplete => {
            state.results.metrics.is_some() && state.results.call_graph.is_some()
        }

        AnalysisPhase::CoverageLoading => {
            state.results.metrics.is_some() && state.results.call_graph.is_some()
        }

        AnalysisPhase::CoverageComplete => {
            state.results.metrics.is_some() && state.results.call_graph.is_some()
            // Coverage is optional, so we don't require it
        }

        AnalysisPhase::PurityAnalyzing => {
            state.results.metrics.is_some() && state.results.call_graph.is_some()
        }

        AnalysisPhase::PurityComplete => {
            state.results.metrics.is_some()
                && state.results.call_graph.is_some()
                && state.results.enriched_metrics.is_some()
        }

        AnalysisPhase::ContextLoading | AnalysisPhase::ContextComplete => {
            state.results.metrics.is_some()
                && state.results.call_graph.is_some()
                && state.results.enriched_metrics.is_some()
        }

        AnalysisPhase::ScoringInProgress | AnalysisPhase::ScoringComplete => {
            state.results.metrics.is_some()
                && state.results.call_graph.is_some()
                && state.results.enriched_metrics.is_some()
        }

        AnalysisPhase::FilteringInProgress => {
            state.results.metrics.is_some()
                && state.results.call_graph.is_some()
                && state.results.enriched_metrics.is_some()
                && state.results.unified_analysis.is_some()
        }

        AnalysisPhase::Complete => state.results.unified_analysis.is_some(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::workflow::state::AnalysisConfig;
    use crate::core::FunctionMetrics;
    use crate::priority::call_graph::CallGraph;
    use std::path::PathBuf;

    fn create_test_state() -> AnalysisState {
        AnalysisState::new(AnalysisConfig::default())
    }

    fn create_test_metrics() -> Vec<FunctionMetrics> {
        vec![FunctionMetrics::new(
            "test_fn".to_string(),
            PathBuf::from("test.rs"),
            1,
        )]
    }

    // === can_start_call_graph tests ===

    #[test]
    fn test_can_start_call_graph_requires_metrics() {
        let mut state = create_test_state();

        // No metrics - can't start
        assert!(!can_start_call_graph(&state));

        // With metrics - can start
        state.results.metrics = Some(create_test_metrics());
        assert!(can_start_call_graph(&state));
    }

    #[test]
    fn test_can_start_call_graph_requires_initialized_phase() {
        let mut state = create_test_state();
        state.results.metrics = Some(create_test_metrics());

        // In Initialized phase - can start
        assert!(can_start_call_graph(&state));

        // In different phase - can't start
        state.phase = AnalysisPhase::CallGraphComplete;
        assert!(!can_start_call_graph(&state));
    }

    // === can_start_coverage / can_skip_coverage tests ===

    #[test]
    fn test_can_start_coverage_requires_config() {
        let mut state = create_test_state();
        state.phase = AnalysisPhase::CallGraphComplete;
        state.results.call_graph = Some(CallGraph::new());

        // No coverage file configured - can't start, can skip
        assert!(!can_start_coverage(&state));
        assert!(can_skip_coverage(&state));

        // With coverage file - can start, can't skip
        state.config.coverage_file = Some(PathBuf::from("coverage.lcov"));
        assert!(can_start_coverage(&state));
        assert!(!can_skip_coverage(&state));
    }

    #[test]
    fn test_can_start_coverage_requires_call_graph() {
        let mut state = create_test_state();
        state.phase = AnalysisPhase::CallGraphComplete;
        state.config.coverage_file = Some(PathBuf::from("coverage.lcov"));

        // No call graph - can't start
        assert!(!can_start_coverage(&state));

        // With call graph - can start
        state.results.call_graph = Some(CallGraph::new());
        assert!(can_start_coverage(&state));
    }

    // === can_start_purity tests ===

    #[test]
    fn test_can_start_purity_requires_call_graph() {
        let mut state = create_test_state();
        state.phase = AnalysisPhase::CoverageComplete;

        // No call graph - can't start
        assert!(!can_start_purity(&state));

        // With call graph - can start
        state.results.call_graph = Some(CallGraph::new());
        assert!(can_start_purity(&state));
    }

    #[test]
    fn test_can_start_purity_requires_coverage_complete_phase() {
        let mut state = create_test_state();
        state.results.call_graph = Some(CallGraph::new());

        // Wrong phase
        state.phase = AnalysisPhase::CallGraphComplete;
        assert!(!can_start_purity(&state));

        // Correct phase
        state.phase = AnalysisPhase::CoverageComplete;
        assert!(can_start_purity(&state));
    }

    // === can_start_context / can_skip_context tests ===

    #[test]
    fn test_can_start_context_requires_enable_flag() {
        let mut state = create_test_state();
        state.phase = AnalysisPhase::PurityComplete;
        state.results.enriched_metrics = Some(create_test_metrics());

        // Context not enabled - can't start, can skip
        assert!(!can_start_context(&state));
        assert!(can_skip_context(&state));

        // Context enabled - can start, can't skip
        state.config.enable_context = true;
        assert!(can_start_context(&state));
        assert!(!can_skip_context(&state));
    }

    // === can_start_scoring tests ===

    #[test]
    fn test_can_start_scoring_requires_all_dependencies() {
        let mut state = create_test_state();
        state.phase = AnalysisPhase::ContextComplete;

        // Missing dependencies
        assert!(!can_start_scoring(&state));

        // Add call graph
        state.results.call_graph = Some(CallGraph::new());
        assert!(!can_start_scoring(&state));

        // Add enriched metrics
        state.results.enriched_metrics = Some(create_test_metrics());
        assert!(can_start_scoring(&state));
    }

    // === Guard purity tests ===

    #[test]
    fn test_guards_are_pure() {
        // Pure functions always return same result for same input
        let state = create_test_state();

        let r1 = can_start_call_graph(&state);
        let r2 = can_start_call_graph(&state);

        assert_eq!(r1, r2, "Guards must be pure - same input, same output");
    }

    #[test]
    fn test_guards_do_not_modify_state() {
        let state = create_test_state();
        let initial_phase = state.phase;

        // Call all guards
        let _ = can_start_call_graph(&state);
        let _ = can_start_coverage(&state);
        let _ = can_skip_coverage(&state);
        let _ = can_start_purity(&state);
        let _ = can_start_context(&state);
        let _ = can_skip_context(&state);
        let _ = can_start_scoring(&state);
        let _ = can_start_filtering(&state);
        let _ = can_complete(&state);

        // State should be unchanged
        assert_eq!(state.phase, initial_phase);
    }

    // === is_valid_checkpoint tests ===

    #[test]
    fn test_is_valid_checkpoint_initialized() {
        let state = create_test_state();
        assert!(is_valid_checkpoint(&state));
    }

    #[test]
    fn test_is_valid_checkpoint_call_graph_complete() {
        let mut state = create_test_state();
        state.phase = AnalysisPhase::CallGraphComplete;

        // Missing data - invalid
        assert!(!is_valid_checkpoint(&state));

        // With required data - valid
        state.results.metrics = Some(create_test_metrics());
        state.results.call_graph = Some(CallGraph::new());
        assert!(is_valid_checkpoint(&state));
    }

    #[test]
    fn test_is_valid_checkpoint_complete() {
        let mut state = create_test_state();
        state.phase = AnalysisPhase::Complete;

        // Missing unified analysis - invalid
        assert!(!is_valid_checkpoint(&state));

        // With unified analysis - valid
        state.results.unified_analysis =
            Some(crate::priority::UnifiedAnalysis::new(CallGraph::new()));
        assert!(is_valid_checkpoint(&state));
    }
}
