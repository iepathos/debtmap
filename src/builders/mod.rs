//! Builder patterns for constructing analysis pipelines and aggregating results.
//!
//! This module provides builders for creating complex analysis structures through
//! composable, type-safe APIs. Builders follow functional patterns with immutable
//! transformations and validated state transitions.
//!
//! Key components:
//! - **Call graph builders**: Construct call graphs from parsed code
//! - **Effect pipelines**: Chain analysis operations with effect tracking
//! - **Unified analysis**: Combine multiple analysis passes into single results
//! - **Parallel builders**: Build structures using parallel processing
//!
//! Builders support both sequential and parallel construction, with validation
//! to ensure correct assembly of complex analysis results.

pub mod call_graph;
pub mod effect_pipeline;
pub mod parallel_call_graph;
pub mod parallel_unified_analysis;
pub mod unified_analysis;
pub mod unified_analysis_phases;
pub mod validated_analysis;

// Re-export effect pipeline functions for convenient access
pub use effect_pipeline::{
    analyze_directory_effect, analyze_file, analyze_file_cached_effect, analyze_file_effect,
    analyze_file_with_coverage_effect, analyze_files, analyze_files_effect,
    analyze_files_parallel_effect, FileAnalysisWithCoverage,
};
