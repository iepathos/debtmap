// Scoring module - organizes scoring-related functionality
// Spec 262: Recommendation generation has been removed from debtmap.
// Debtmap now focuses on identification and severity quantification.

pub mod calculation;
pub mod classification;
pub mod computation;
pub mod construction;
pub mod context_aware;
pub mod coverage_expectations;
pub mod coverage_scoring;
pub mod debt_item;
pub mod effects; // Spec 268: Effect-based scoring with Reader pattern
pub mod facade_scoring; // Spec 170: Module facade detection and scoring adjustment
pub mod file_context_scoring; // Spec 166: Test file detection and context-aware scoring
pub mod formatting;
pub mod orchestration_adjustment;
pub mod rebalanced; // Spec 136: Rebalanced debt scoring algorithm
pub mod scaling; // Spec 171: Exponential scaling and risk boosting
pub mod test_calculation;
pub mod validation;

// Spec 262: The following recommendation modules have been removed:
// - concise_recommendation
// - recommendation
// - complexity_classification
// - complexity_generators
// - dead_code_hints
// - heuristic_generators
// - pattern_generators
// - recommendation_complexity
// - recommendation_debt_specific
// - recommendation_extended
// - recommendation_helpers
// - rust_recommendations

// Re-export commonly used items
pub use calculation::{
    calculate_base_score, calculate_base_score_with_coverage_multiplier,
    calculate_complexity_factor, calculate_coverage_factor, calculate_coverage_multiplier,
    calculate_coverage_multiplier_with_test_flag, calculate_dependency_factor, denormalize_score,
    generate_normalization_curve, normalize_complexity, normalize_final_score_with_metadata,
    NormalizedScore, ScalingMethod,
};

pub use classification::{
    classify_risk_based_debt, classify_test_debt, is_complexity_hotspot,
    should_surface_untested_function,
};

// Spec 262: Recommendation re-exports removed

pub use test_calculation::{calculate_tests_needed, ComplexityTier, TestRecommendation};

pub use orchestration_adjustment::{
    adjust_score, extract_composition_metrics, CompositionMetrics, OrchestrationAdjustmentConfig,
    ReductionPercent, ScoreAdjustment,
};

pub use coverage_expectations::{CoverageExpectations, CoverageGap, CoverageRange, GapSeverity};

pub use coverage_scoring::calculate_coverage_score;

pub use context_aware::{
    ContextRecommendationEngine, ContextualRecommendation, Severity as ContextSeverity,
};

pub use rebalanced::{DebtScore, ScoreComponents, ScoreWeights, ScoringRationale, Severity};

pub use file_context_scoring::{
    apply_context_adjustments, context_label, context_reduction_factor, is_test_context,
};

pub use facade_scoring::adjust_score_for_facade;

pub use scaling::{calculate_final_score, ScalingConfig};

pub use effects::{
    calculate_score_effect, calculate_scores_effect, get_weights_effect, ScoringEnv, TestScoringEnv,
};

// Note: debt_item functions are re-exported from unified_scorer.rs for backward compatibility
