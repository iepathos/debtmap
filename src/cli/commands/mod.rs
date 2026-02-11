//! Command handlers for CLI subcommands
//!
//! This module contains the implementations for each CLI subcommand,
//! providing a clean separation between argument parsing and command execution.

mod analyze;
mod compare;
mod explain_coverage;
mod validate;
mod validate_improvement;

pub use analyze::handle_analyze_command;
pub use compare::handle_compare_command;
pub use explain_coverage::handle_explain_coverage_command;
pub use validate::handle_validate_command;
pub use validate_improvement::handle_validate_improvement_command;
