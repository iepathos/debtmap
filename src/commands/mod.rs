//! CLI command implementations for debtmap operations.
//!
//! This module contains the implementation of all CLI commands exposed by debtmap.
//! Each submodule handles a specific command with its configuration, validation,
//! and execution logic.
//!
//! Available commands:
//! - **analyze**: Run technical debt analysis on a codebase
//! - **compare**: Compare two debtmap analysis results
//! - **diagnose-coverage**: Debug coverage calculation for specific files
//! - **explain-coverage**: Explain how coverage scores are calculated
//! - **init**: Initialize a new debtmap configuration file
//! - **validate**: Validate codebase against configured thresholds
//! - **validate-improvement**: Verify that changes improved debt metrics
//!
//! Commands follow a type-state pattern for configuration validation,
//! ensuring that only validated configurations can be executed.

pub mod analyze;
pub mod compare_debtmap;
pub mod diagnose_coverage;
pub mod explain_coverage;
pub mod init;
pub mod state;
pub mod validate;
pub mod validate_improvement;

pub use analyze::handle_analyze;
pub use compare_debtmap::{compare_debtmaps, CompareConfig};
pub use diagnose_coverage::diagnose_coverage_file;
pub use explain_coverage::{explain_coverage, ExplainCoverageConfig};
pub use init::init_config;
pub use state::{AnalyzeConfig, Unvalidated, Validated};
pub use validate::{validate_project, ValidateConfig, ValidationDetails};
pub use validate_improvement::{validate_improvement, ValidateImprovementConfig};
