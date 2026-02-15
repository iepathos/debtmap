//! Metrics calculation for TypeScript/JavaScript
//!
//! Functions for building file metrics from analysis results.

pub mod builder;
pub mod complexity;

pub use builder::build_file_metrics;
pub use complexity::calculate_total_complexity;
