//! Unified analysis module with pure core and effects-based orchestration.
//!
//! This module implements the "Pure Core, Imperative Shell" pattern:
//!
//! - **Pure computation modules** (`phases/`): No I/O, no progress reporting
//! - **Orchestration layer** (`orchestration.rs`): Effects-based composition
//! - **Options builder** (`options.rs`): Type-safe configuration
//!
//! # Module Organization
//!
//! ```text
//! unified_analysis/
//! ├── mod.rs              - This file, re-exports
//! ├── options.rs          - Configuration builder
//! ├── orchestration.rs    - Effects-based composition
//! └── phases/
//!     ├── call_graph.rs   - Pure call graph computation
//!     ├── file_analysis.rs - Pure file-level analysis
//!     ├── god_object.rs   - Pure god object detection
//!     ├── scoring.rs      - Pure debt scoring
//!     └── coverage.rs     - Coverage loading (I/O)
//! ```
//!
//! # Usage
//!
//! The main entry point is `perform_unified_analysis_with_options()` which
//! orchestrates all phases while maintaining backward compatibility.
//!
//! For new code, prefer using the pure functions directly from the `phases`
//! module for better testability.

pub mod options;
pub mod orchestration;
pub mod phases;

// Re-export options builder for convenient access
pub use options::{ConfigError, UnifiedAnalysisConfig, UnifiedAnalysisConfigBuilder};

// Re-export orchestration types
pub use orchestration::{AnalysisContext, AnalysisTimings};

// Re-export commonly used pure functions
pub use phases::call_graph::{
    apply_trait_patterns, build_initial_call_graph, enrich_metrics_with_call_graph,
    find_test_only_functions, is_closure, is_test_function, is_trivial_function,
    should_process_metric, CallGraphConfig, CallGraphEnrichmentResult,
};
pub use phases::coverage::{
    calculate_coverage_percent, get_overall_coverage, has_coverage_data, load_coverage_data,
    load_coverage_file,
};
pub use phases::file_analysis::{
    aggregate_file_metrics, calculate_uncovered_lines, create_file_debt_item, detect_file_context,
    enhance_metrics_with_line_count, group_functions_by_file, process_file_metrics,
    should_include_file, ProcessedFileData,
};
pub use phases::god_object::{
    analyze_file_git_context, calculate_god_object_risk, create_god_object_debt_item,
    create_god_object_recommendation, enrich_god_analysis_with_aggregates,
};
pub use phases::scoring::{
    calculate_average_complexity, calculate_total_complexity, create_debt_items_from_metric,
    create_function_mappings, metrics_to_purity_map, process_metrics_to_debt_items,
    setup_debt_aggregator, PriorityConfig, ScoringWeights,
};
