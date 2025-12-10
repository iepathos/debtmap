//! Checkpoint Support for Analysis Workflow (Spec 202)
//!
//! Provides save/load functionality for analysis state, enabling:
//! - Pausing and resuming long-running analysis
//! - Recovery from interruptions
//! - Debugging by inspecting intermediate state
//!
//! ## Limitations
//!
//! Due to the complexity of serializing certain types (CallGraph, LcovData, etc.),
//! checkpoints save the configuration and phase, but some computed results may
//! need to be recomputed on resume.

use super::{
    env::AnalysisEnv,
    guards::is_valid_checkpoint,
    state::{AnalysisConfig, AnalysisPhase, AnalysisState},
};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Serializable checkpoint data.
///
/// This is a subset of AnalysisState that can be reliably serialized.
/// Non-serializable fields (call_graph, coverage, etc.) are marked with `#[serde(skip)]`
/// in AnalysisState and will need to be recomputed on resume.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    /// Version for forward compatibility.
    pub version: u32,

    /// The phase at which the checkpoint was created.
    pub phase: AnalysisPhase,

    /// Configuration for this analysis run.
    pub config: AnalysisConfig,

    /// Timestamp when checkpoint was created.
    pub created_at: chrono::DateTime<chrono::Utc>,

    /// Optional metrics (if they fit in memory and are serializable).
    pub metrics_count: Option<usize>,
}

impl Checkpoint {
    /// Current checkpoint version.
    pub const CURRENT_VERSION: u32 = 1;

    /// Create a checkpoint from current state.
    pub fn from_state(state: &AnalysisState) -> Self {
        Self {
            version: Self::CURRENT_VERSION,
            phase: state.phase,
            config: state.config.clone(),
            created_at: chrono::Utc::now(),
            metrics_count: state.results.metrics.as_ref().map(|m| m.len()),
        }
    }

    /// Restore a basic state from checkpoint.
    ///
    /// Note: This only restores config and phase. Computed results like
    /// call_graph, coverage, etc. will need to be recomputed.
    pub fn into_state(self) -> AnalysisState {
        let mut state = AnalysisState::new(self.config);

        // Restore to a recoverable phase
        // If we were mid-computation, restart that phase
        state.phase = match self.phase {
            AnalysisPhase::CallGraphBuilding => AnalysisPhase::Initialized,
            AnalysisPhase::CoverageLoading => AnalysisPhase::CallGraphComplete,
            AnalysisPhase::PurityAnalyzing => AnalysisPhase::CoverageComplete,
            AnalysisPhase::ContextLoading => AnalysisPhase::PurityComplete,
            AnalysisPhase::ScoringInProgress => AnalysisPhase::ContextComplete,
            AnalysisPhase::FilteringInProgress => AnalysisPhase::ScoringComplete,
            other => other, // Keep completed phases as-is
        };

        state
    }
}

/// Save analysis state to checkpoint file.
///
/// Creates a JSON checkpoint file that can be used to resume analysis.
pub fn save_checkpoint(state: &AnalysisState, path: &Path) -> Result<()> {
    let checkpoint = Checkpoint::from_state(state);
    let json = serde_json::to_string_pretty(&checkpoint)
        .context("Failed to serialize checkpoint to JSON")?;

    std::fs::write(path, json).context("Failed to write checkpoint file")?;

    Ok(())
}

/// Load analysis state from checkpoint file.
///
/// Returns a state that can be used to resume analysis. Note that
/// computed results (call_graph, coverage, etc.) will need to be recomputed.
pub fn load_checkpoint(path: &Path) -> Result<AnalysisState> {
    let json = std::fs::read_to_string(path).context("Failed to read checkpoint file")?;

    let checkpoint: Checkpoint =
        serde_json::from_str(&json).context("Failed to parse checkpoint JSON")?;

    // Validate version
    if checkpoint.version > Checkpoint::CURRENT_VERSION {
        anyhow::bail!(
            "Checkpoint version {} is newer than supported version {}",
            checkpoint.version,
            Checkpoint::CURRENT_VERSION
        );
    }

    Ok(checkpoint.into_state())
}

/// Resume analysis from checkpoint.
///
/// Loads the checkpoint, validates it, and continues the workflow.
pub fn resume_analysis<Env: AnalysisEnv>(
    checkpoint_path: &Path,
    metrics: Vec<crate::core::FunctionMetrics>,
    mut env: Env,
) -> Result<AnalysisState> {
    use super::actions::WorkflowRunner;

    // Load checkpoint
    let mut state = load_checkpoint(checkpoint_path)?;

    // Restore metrics (required to continue)
    state.results.metrics = Some(metrics);

    // Log resume point
    env.info(&format!("Resuming from phase: {:?}", state.phase));

    // Validate that we have what we need
    if !is_valid_checkpoint(&state) {
        // Reset to Initialized if invalid
        env.warn("Checkpoint state invalid, restarting from beginning");
        state.phase = AnalysisPhase::Initialized;
    }

    // Run the workflow
    WorkflowRunner::new(state, env).run()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::NamedTempFile;

    fn create_test_config() -> AnalysisConfig {
        AnalysisConfig {
            project_path: PathBuf::from("src"),
            enable_context: false,
            ..Default::default()
        }
    }

    #[test]
    fn test_checkpoint_creation() {
        let config = create_test_config();
        let state = AnalysisState::new(config);

        let checkpoint = Checkpoint::from_state(&state);

        assert_eq!(checkpoint.version, Checkpoint::CURRENT_VERSION);
        assert_eq!(checkpoint.phase, AnalysisPhase::Initialized);
    }

    #[test]
    fn test_checkpoint_roundtrip() {
        let config = create_test_config();
        let state = AnalysisState::new(config.clone());

        // Save checkpoint
        let file = NamedTempFile::new().unwrap();
        save_checkpoint(&state, file.path()).unwrap();

        // Load checkpoint
        let loaded = load_checkpoint(file.path()).unwrap();

        assert_eq!(loaded.phase, state.phase);
        assert_eq!(loaded.config.project_path, config.project_path);
    }

    #[test]
    fn test_checkpoint_phase_recovery() {
        // Mid-computation phases should reset to their starting point
        let cases = vec![
            (AnalysisPhase::CallGraphBuilding, AnalysisPhase::Initialized),
            (
                AnalysisPhase::CoverageLoading,
                AnalysisPhase::CallGraphComplete,
            ),
            (
                AnalysisPhase::PurityAnalyzing,
                AnalysisPhase::CoverageComplete,
            ),
            (AnalysisPhase::ContextLoading, AnalysisPhase::PurityComplete),
            (
                AnalysisPhase::ScoringInProgress,
                AnalysisPhase::ContextComplete,
            ),
            (
                AnalysisPhase::FilteringInProgress,
                AnalysisPhase::ScoringComplete,
            ),
            // Completed phases stay as-is
            (
                AnalysisPhase::CallGraphComplete,
                AnalysisPhase::CallGraphComplete,
            ),
            (AnalysisPhase::Complete, AnalysisPhase::Complete),
        ];

        for (input_phase, expected_phase) in cases {
            let checkpoint = Checkpoint {
                version: Checkpoint::CURRENT_VERSION,
                phase: input_phase,
                config: create_test_config(),
                created_at: chrono::Utc::now(),
                metrics_count: None,
            };

            let state = checkpoint.into_state();
            assert_eq!(
                state.phase, expected_phase,
                "Phase {:?} should recover to {:?}",
                input_phase, expected_phase
            );
        }
    }

    #[test]
    fn test_checkpoint_version_validation() {
        let config = create_test_config();
        let state = AnalysisState::new(config);

        // Save a valid checkpoint
        let file = NamedTempFile::new().unwrap();
        save_checkpoint(&state, file.path()).unwrap();

        // Manually create a future version checkpoint
        let future_checkpoint = Checkpoint {
            version: Checkpoint::CURRENT_VERSION + 10,
            phase: AnalysisPhase::Initialized,
            config: create_test_config(),
            created_at: chrono::Utc::now(),
            metrics_count: None,
        };

        let json = serde_json::to_string(&future_checkpoint).unwrap();
        let future_file = NamedTempFile::new().unwrap();
        std::fs::write(future_file.path(), json).unwrap();

        // Loading should fail with version error
        let result = load_checkpoint(future_file.path());
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("newer than supported"));
    }

    #[test]
    fn test_checkpoint_file_not_found() {
        let result = load_checkpoint(Path::new("/nonexistent/checkpoint.json"));
        assert!(result.is_err());
    }

    #[test]
    fn test_checkpoint_invalid_json() {
        let file = NamedTempFile::new().unwrap();
        std::fs::write(file.path(), "not valid json").unwrap();

        let result = load_checkpoint(file.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_checkpoint_with_coverage_config() {
        let mut config = create_test_config();
        config.coverage_file = Some(PathBuf::from("coverage.lcov"));
        config.enable_context = true;

        let state = AnalysisState::new(config);
        let file = NamedTempFile::new().unwrap();

        save_checkpoint(&state, file.path()).unwrap();
        let loaded = load_checkpoint(file.path()).unwrap();

        assert_eq!(
            loaded.config.coverage_file,
            Some(PathBuf::from("coverage.lcov"))
        );
        assert!(loaded.config.enable_context);
    }
}
