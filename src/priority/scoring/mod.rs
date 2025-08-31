// Scoring module - organizes scoring-related functionality

pub mod calculation;
pub mod classification;
pub mod debt_item;
pub mod recommendation;
pub mod recommendation_extended;

// Re-export commonly used items
pub use calculation::{
    calculate_coverage_factor,
    calculate_complexity_factor,
    calculate_dependency_factor,
    calculate_base_score,
    apply_interaction_bonus,
    normalize_final_score,
    normalize_complexity,
};

pub use classification::{
    determine_debt_type,
    classify_test_debt,
    classify_risk_based_debt,
    is_complexity_hotspot,
};

pub use recommendation::{
    generate_testing_gap_recommendation,
    generate_dead_code_recommendation,
    generate_error_swallowing_recommendation,
    generate_test_debt_recommendation,
};

// Note: debt_item functions are re-exported from unified_scorer.rs for backward compatibility