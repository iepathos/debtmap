//! Command handlers for CLI subcommands
//!
//! This module contains the implementations for each CLI subcommand,
//! providing a clean separation between argument parsing and command execution.

mod analyze;
mod compare;

pub use analyze::handle_analyze_command;
pub use compare::handle_compare_command;
