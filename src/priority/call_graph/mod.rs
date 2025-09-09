//! Call graph analysis module for function relationship tracking
//!
//! This module provides functionality for building and analyzing call graphs,
//! including pattern detection, criticality analysis, and cross-file resolution.
//! 
//! This module has been refactored into smaller, focused submodules
//! to improve maintainability and comply with module size limits (<500 lines each).

mod types;
mod graph_operations;
mod pattern_detection;
mod criticality;
mod cross_file;
mod test_analysis;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod pure_function_tests;

pub use types::{FunctionId, FunctionCall, CallType, CallGraph};
#[cfg(test)]
pub(crate) use types::FunctionNode;

// Re-export commonly used functions from CallGraph
pub use types::CallGraph as Graph;