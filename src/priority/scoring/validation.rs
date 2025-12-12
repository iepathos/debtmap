// Validation functions for debt type detection
//
// This module contains pure functions for classifying different types of technical debt
// based on function metrics, coverage data, and call graph analysis.

// Re-export for backward compatibility
pub use super::debt_item::{determine_visibility, is_dead_code, is_dead_code_with_exclusions};
