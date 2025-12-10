//! Analysis Workflow State Types (Spec 202)
//!
//! Defines the state machine types for the analysis workflow:
//! - `AnalysisPhase` - Enum representing each phase of analysis
//! - `AnalysisState` - Complete state including phase and accumulated results
//! - `AnalysisConfig` - Configuration for the analysis run
//! - `AnalysisResults` - Accumulated results from completed phases

use crate::core::FunctionMetrics;
use crate::priority::call_graph::CallGraph;
use crate::risk::lcov::LcovData;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;

/// Analysis workflow phases.
///
/// Each phase represents a distinct step in the analysis pipeline.
/// Transitions between phases are validated by pure guard functions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum AnalysisPhase {
    /// Initial state after config validation.
    #[default]
    Initialized,

    /// Building the function call graph.
    CallGraphBuilding,

    /// Call graph built successfully.
    CallGraphComplete,

    /// Loading LCOV coverage data.
    CoverageLoading,

    /// Coverage loaded (or skipped if not provided).
    CoverageComplete,

    /// Analyzing function purity.
    PurityAnalyzing,

    /// Purity analysis complete.
    PurityComplete,

    /// Loading context providers.
    ContextLoading,

    /// Context loaded (or skipped).
    ContextComplete,

    /// Computing debt scores.
    ScoringInProgress,

    /// Scores computed.
    ScoringComplete,

    /// Filtering and ranking results.
    FilteringInProgress,

    /// Analysis complete.
    Complete,
}

impl AnalysisPhase {
    /// Returns true if this is a final state (no more transitions possible)
    pub fn is_final(&self) -> bool {
        matches!(self, AnalysisPhase::Complete)
    }

    /// Returns the display name for progress reporting
    pub fn display_name(&self) -> &'static str {
        match self {
            AnalysisPhase::Initialized => "Initialized",
            AnalysisPhase::CallGraphBuilding => "Building call graph",
            AnalysisPhase::CallGraphComplete => "Call graph complete",
            AnalysisPhase::CoverageLoading => "Loading coverage data",
            AnalysisPhase::CoverageComplete => "Coverage complete",
            AnalysisPhase::PurityAnalyzing => "Analyzing function purity",
            AnalysisPhase::PurityComplete => "Purity analysis complete",
            AnalysisPhase::ContextLoading => "Loading project context",
            AnalysisPhase::ContextComplete => "Context complete",
            AnalysisPhase::ScoringInProgress => "Computing debt scores",
            AnalysisPhase::ScoringComplete => "Scoring complete",
            AnalysisPhase::FilteringInProgress => "Filtering and ranking",
            AnalysisPhase::Complete => "Complete",
        }
    }

    /// Returns the TUI stage index for this phase (0-based)
    pub fn tui_stage_index(&self) -> Option<usize> {
        match self {
            AnalysisPhase::CallGraphBuilding | AnalysisPhase::CallGraphComplete => Some(1),
            AnalysisPhase::CoverageLoading | AnalysisPhase::CoverageComplete => Some(2),
            AnalysisPhase::PurityAnalyzing | AnalysisPhase::PurityComplete => Some(3),
            AnalysisPhase::ContextLoading | AnalysisPhase::ContextComplete => Some(4),
            AnalysisPhase::ScoringInProgress
            | AnalysisPhase::ScoringComplete
            | AnalysisPhase::FilteringInProgress => Some(5),
            _ => None,
        }
    }
}

/// Configuration for the analysis run.
///
/// This struct mirrors `UnifiedAnalysisOptions` but is serializable
/// for checkpoint support.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AnalysisConfig {
    /// Path to the project being analyzed.
    pub project_path: PathBuf,

    /// Optional LCOV coverage file.
    pub coverage_file: Option<PathBuf>,

    /// Whether to enable context providers (git history, etc.)
    pub enable_context: bool,

    /// Specific context providers to enable.
    pub context_providers: Option<Vec<String>>,

    /// Context providers to disable.
    pub disable_context: Option<Vec<String>>,

    /// Whether to run in parallel mode.
    pub parallel: bool,

    /// Number of jobs for parallel execution (0 = auto).
    pub jobs: usize,

    /// Whether to enable multi-pass analysis.
    pub multi_pass: bool,

    /// Whether to show attribution information.
    pub show_attribution: bool,

    /// Whether to skip aggregation.
    pub no_aggregation: bool,

    /// Aggregation method to use.
    pub aggregation_method: Option<String>,

    /// Minimum number of problematic functions for aggregation.
    pub min_problematic: Option<usize>,

    /// Whether to disable god object detection.
    pub no_god_object: bool,

    /// Whether to suppress coverage tip.
    pub suppress_coverage_tip: bool,

    /// Verbose macro warnings.
    pub verbose_macro_warnings: bool,

    /// Show macro stats.
    pub show_macro_stats: bool,
}

/// Accumulated results from analysis phases.
///
/// Each field is populated as its corresponding phase completes.
/// All fields are `Option` to support partial state for checkpointing.
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct AnalysisResults {
    /// Function metrics from parsing.
    pub metrics: Option<Vec<FunctionMetrics>>,

    /// Call graph from dependency analysis.
    #[serde(skip)]
    pub call_graph: Option<CallGraph>,

    /// Framework exclusions identified during call graph building.
    #[serde(skip)]
    pub framework_exclusions: Option<HashSet<crate::priority::call_graph::FunctionId>>,

    /// Functions used via function pointers.
    #[serde(skip)]
    pub function_pointer_used_functions: Option<HashSet<crate::priority::call_graph::FunctionId>>,

    /// Coverage data if provided.
    #[serde(skip)]
    pub coverage: Option<LcovData>,

    /// Enriched metrics with purity information.
    pub enriched_metrics: Option<Vec<FunctionMetrics>>,

    /// Context data if enabled (not Debug due to RiskAnalyzer).
    #[serde(skip)]
    pub risk_analyzer: Option<crate::risk::RiskAnalyzer>,

    /// Computed debt scores.
    #[serde(skip)]
    pub unified_analysis: Option<crate::priority::UnifiedAnalysis>,
}

impl std::fmt::Debug for AnalysisResults {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AnalysisResults")
            .field("metrics", &self.metrics.as_ref().map(|m| m.len()))
            .field("call_graph", &self.call_graph.is_some())
            .field(
                "framework_exclusions",
                &self.framework_exclusions.as_ref().map(|s| s.len()),
            )
            .field(
                "function_pointer_used_functions",
                &self
                    .function_pointer_used_functions
                    .as_ref()
                    .map(|s| s.len()),
            )
            .field("coverage", &self.coverage.is_some())
            .field(
                "enriched_metrics",
                &self.enriched_metrics.as_ref().map(|m| m.len()),
            )
            .field("risk_analyzer", &self.risk_analyzer.is_some())
            .field("unified_analysis", &self.unified_analysis.is_some())
            .finish()
    }
}

/// Complete analysis state including phase and accumulated data.
///
/// This struct holds all state needed for checkpoint/resume functionality.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisState {
    /// Current phase.
    pub phase: AnalysisPhase,

    /// Configuration for this analysis run.
    pub config: AnalysisConfig,

    /// Accumulated results from completed phases.
    #[serde(skip)]
    pub results: AnalysisResults,

    /// Timestamp of the last phase transition (for debugging).
    pub last_transition: Option<chrono::DateTime<chrono::Utc>>,
}

impl AnalysisState {
    /// Create a new analysis state with the given configuration.
    pub fn new(config: AnalysisConfig) -> Self {
        Self {
            phase: AnalysisPhase::Initialized,
            config,
            results: AnalysisResults::default(),
            last_transition: Some(chrono::Utc::now()),
        }
    }

    /// Create a new analysis state with metrics pre-populated.
    ///
    /// This is the typical entry point when starting analysis after parsing.
    pub fn with_metrics(config: AnalysisConfig, metrics: Vec<FunctionMetrics>) -> Self {
        let mut state = Self::new(config);
        state.results.metrics = Some(metrics);
        state
    }

    /// Transition to a new phase.
    ///
    /// This method updates the phase and timestamp but does not validate
    /// the transition - guards should be called before this.
    pub fn transition_to(&mut self, new_phase: AnalysisPhase) {
        self.phase = new_phase;
        self.last_transition = Some(chrono::Utc::now());
    }

    /// Returns true if the analysis is in a final state.
    pub fn is_complete(&self) -> bool {
        self.phase.is_final()
    }

    /// Returns the percentage progress through the workflow (0.0-1.0).
    pub fn progress_percent(&self) -> f64 {
        match self.phase {
            AnalysisPhase::Initialized => 0.0,
            AnalysisPhase::CallGraphBuilding => 0.10,
            AnalysisPhase::CallGraphComplete => 0.29,
            AnalysisPhase::CoverageLoading => 0.35,
            AnalysisPhase::CoverageComplete => 0.43,
            AnalysisPhase::PurityAnalyzing => 0.50,
            AnalysisPhase::PurityComplete => 0.57,
            AnalysisPhase::ContextLoading => 0.65,
            AnalysisPhase::ContextComplete => 0.83,
            AnalysisPhase::ScoringInProgress => 0.90,
            AnalysisPhase::ScoringComplete => 0.95,
            AnalysisPhase::FilteringInProgress => 0.98,
            AnalysisPhase::Complete => 1.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_phase_is_final() {
        assert!(!AnalysisPhase::Initialized.is_final());
        assert!(!AnalysisPhase::CallGraphBuilding.is_final());
        assert!(!AnalysisPhase::PurityComplete.is_final());
        assert!(AnalysisPhase::Complete.is_final());
    }

    #[test]
    fn test_state_new() {
        let config = AnalysisConfig {
            project_path: PathBuf::from("src"),
            ..Default::default()
        };
        let state = AnalysisState::new(config);

        assert_eq!(state.phase, AnalysisPhase::Initialized);
        assert!(!state.is_complete());
        assert!(state.last_transition.is_some());
    }

    #[test]
    fn test_state_with_metrics() {
        let config = AnalysisConfig::default();
        let metrics = vec![FunctionMetrics::new(
            "test".to_string(),
            PathBuf::from("test.rs"),
            1,
        )];

        let state = AnalysisState::with_metrics(config, metrics);

        assert!(state.results.metrics.is_some());
        assert_eq!(state.results.metrics.as_ref().unwrap().len(), 1);
    }

    #[test]
    fn test_state_transition() {
        let mut state = AnalysisState::new(AnalysisConfig::default());
        let initial_time = state.last_transition;

        // Small delay to ensure timestamp changes
        std::thread::sleep(std::time::Duration::from_millis(10));

        state.transition_to(AnalysisPhase::CallGraphBuilding);

        assert_eq!(state.phase, AnalysisPhase::CallGraphBuilding);
        assert!(state.last_transition > initial_time);
    }

    #[test]
    fn test_progress_percent_ordering() {
        let phases = [
            AnalysisPhase::Initialized,
            AnalysisPhase::CallGraphBuilding,
            AnalysisPhase::CallGraphComplete,
            AnalysisPhase::CoverageLoading,
            AnalysisPhase::CoverageComplete,
            AnalysisPhase::PurityAnalyzing,
            AnalysisPhase::PurityComplete,
            AnalysisPhase::ContextLoading,
            AnalysisPhase::ContextComplete,
            AnalysisPhase::ScoringInProgress,
            AnalysisPhase::ScoringComplete,
            AnalysisPhase::FilteringInProgress,
            AnalysisPhase::Complete,
        ];

        let mut prev_progress = -1.0;
        for phase in phases {
            // Create a temp state to get progress for this phase
            let mut state = AnalysisState::new(AnalysisConfig::default());
            state.phase = phase;
            let current_progress = state.progress_percent();

            assert!(
                current_progress >= prev_progress,
                "Progress should increase monotonically: {:?} ({}) < {}",
                phase,
                current_progress,
                prev_progress
            );
            prev_progress = current_progress;
        }
    }

    #[test]
    fn test_tui_stage_indices() {
        assert_eq!(AnalysisPhase::CallGraphBuilding.tui_stage_index(), Some(1));
        assert_eq!(AnalysisPhase::CoverageLoading.tui_stage_index(), Some(2));
        assert_eq!(AnalysisPhase::PurityAnalyzing.tui_stage_index(), Some(3));
        assert_eq!(AnalysisPhase::ContextLoading.tui_stage_index(), Some(4));
        assert_eq!(AnalysisPhase::ScoringInProgress.tui_stage_index(), Some(5));
        assert_eq!(AnalysisPhase::Initialized.tui_stage_index(), None);
        assert_eq!(AnalysisPhase::Complete.tui_stage_index(), None);
    }
}
