//! Analysis Workflow State Machine (Spec 202)
//!
//! This module implements an explicit state machine for the analysis workflow,
//! providing:
//!
//! - **Explicit phases** - Each analysis phase is an enum variant
//! - **Pure guards** - Transition validation is pure (testable)
//! - **Effectful actions** - Side effects isolated via environment traits
//! - **Checkpoint support** - Save/restore state for resume capability
//! - **Clear dependencies** - Phase prerequisites explicit in guards
//!
//! ## Architecture
//!
//! The workflow follows the "pure guards, effectful actions" pattern:
//!
//! ```text
//! AnalysisState { phase: Initialized }
//!     ↓ can_start_call_graph() guard
//! AnalysisState { phase: CallGraphBuilding }
//!     ↓ build_call_graph() action
//! AnalysisState { phase: CallGraphComplete }
//!     ↓ can_start_coverage() | can_skip_coverage()
//! AnalysisState { phase: CoverageComplete }
//!     ↓ ...
//! AnalysisState { phase: Complete }
//! ```

pub mod actions;
pub mod checkpoint;
pub mod env;
pub mod guards;
pub mod state;

pub use actions::{run_analysis, WorkflowRunner};
pub use checkpoint::{load_checkpoint, save_checkpoint};
pub use env::{AnalysisEnv, FileSystem, ProgressReporter, RealAnalysisEnv};
pub use guards::*;
pub use state::{AnalysisConfig, AnalysisPhase, AnalysisResults, AnalysisState};

#[cfg(test)]
pub use env::MockAnalysisEnv;
