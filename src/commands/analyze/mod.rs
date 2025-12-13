//! Analyze command module.
//!
//! This module follows the "Pure Core, Imperative Shell" pattern:
//!
//! - **config.rs**: Shell - Environment & config setup (I/O)
//! - **pipeline.rs**: Core - Pure transformations (testable, no I/O)
//! - **orchestrator.rs**: Shell - Thin I/O composition
//! - **project_analysis.rs**: Shell - Project analysis (I/O)
//! - **diagnostics.rs**: Shell - Output formatting (I/O)

pub mod config;
mod diagnostics;
pub mod orchestrator;
mod pipeline;
mod project_analysis;

// Re-export public API (unchanged signatures)
pub use config::AnalyzeConfig;
pub use orchestrator::{analyze_project, handle_analyze};
