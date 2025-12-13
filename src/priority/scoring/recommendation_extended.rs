// Extended recommendation generation functions for debt items
// This module re-exports from focused submodules for backward compatibility.
//
// The functionality has been split into:
// - recommendation_complexity: Complexity classification and refactoring recommendations
// - recommendation_debt_specific: Specific debt type recommendations (code smells, resource issues, etc.)

// Re-export complexity-related functions
pub use super::recommendation_complexity::{
    classify_complexity_level, generate_complexity_hotspot_recommendation,
    generate_complexity_recommendation_with_patterns_and_coverage,
    generate_complexity_risk_recommendation,
    generate_heuristic_recommendations_with_line_estimates,
    generate_infrastructure_recommendation_with_coverage, generate_usage_hints, ComplexityLevel,
};

// Re-export specific debt type recommendations
pub use super::recommendation_debt_specific::{
    generate_assertion_complexity_recommendation, generate_async_misuse_recommendation,
    generate_collection_inefficiency_recommendation, generate_data_structure_recommendation,
    generate_feature_envy_recommendation, generate_flaky_test_recommendation,
    generate_god_object_recommendation, generate_magic_values_recommendation,
    generate_nested_loops_recommendation, generate_primitive_obsession_recommendation,
    generate_resource_leak_recommendation, generate_resource_management_recommendation,
    generate_string_concat_recommendation,
};
