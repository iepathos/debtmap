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
