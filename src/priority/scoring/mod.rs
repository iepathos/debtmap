// Scoring module - organizes scoring-related functionality

pub mod calculation;
pub mod classification;
pub mod debt_item;
pub mod recommendation;
pub mod recommendation_extended;

// Re-export commonly used items
pub use calculation::{
    apply_interaction_bonus, calculate_base_score, calculate_complexity_factor,
    calculate_coverage_factor, calculate_dependency_factor, denormalize_score,
    normalize_complexity, normalize_final_score, normalize_final_score_with_metadata,
    NormalizedScore, ScalingMethod,
};

pub use classification::{
    classify_risk_based_debt, classify_test_debt, determine_debt_type, is_complexity_hotspot,
};

pub use recommendation::{
    generate_dead_code_recommendation, generate_error_swallowing_recommendation,
    generate_test_debt_recommendation, generate_testing_gap_recommendation,
};

// Note: debt_item functions are re-exported from unified_scorer.rs for backward compatibility
