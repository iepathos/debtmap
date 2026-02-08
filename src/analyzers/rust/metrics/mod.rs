//! Metrics building and analysis
//!
//! Contains modules for building function and file metrics.

pub mod builder;
pub mod enhanced_analysis;

pub use builder::{build_file_metrics, calculate_total_complexity};
pub use enhanced_analysis::{create_analysis_result, perform_enhanced_analysis};
