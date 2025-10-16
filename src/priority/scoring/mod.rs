// Scoring module - organizes scoring-related functionality

pub mod calculation;
pub mod classification;
pub mod computation;
pub mod construction;
pub mod debt_item;
pub mod formatting;
pub mod recommendation;
pub mod recommendation_extended;
pub mod rust_recommendations;
pub mod test_calculation;
pub mod validation;

// Re-export commonly used items
pub use calculation::{
    calculate_base_score, calculate_base_score_with_coverage_multiplier,
    calculate_complexity_factor, calculate_coverage_factor, calculate_coverage_multiplier,
    calculate_coverage_multiplier_with_test_flag, calculate_dependency_factor, denormalize_score,
    generate_normalization_curve, normalize_complexity, normalize_final_score,
    normalize_final_score_with_metadata, NormalizedScore, ScalingMethod,
};

pub use classification::{
    classify_risk_based_debt, classify_test_debt, determine_debt_type, is_complexity_hotspot,
    should_surface_untested_function,
};

pub use recommendation::{
    generate_dead_code_recommendation, generate_error_swallowing_recommendation,
    generate_test_debt_recommendation, generate_testing_gap_recommendation,
};

pub use test_calculation::{
    calculate_tests_needed, validate_recommendation_consistency, ComplexityTier, TestRecommendation,
};

// Note: debt_item functions are re-exported from unified_scorer.rs for backward compatibility
