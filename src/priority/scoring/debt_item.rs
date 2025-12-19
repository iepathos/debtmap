// Functions for creating UnifiedDebtItem instances
// Spec 262: Recommendation generation removed - debtmap focuses on identification and severity

use crate::core::FunctionMetrics;
use crate::priority::{
    call_graph::{CallGraph, FunctionId},
    semantic_classifier::classify_function_role,
    ActionableRecommendation, DebtType, FunctionRole, TransitiveCoverage, UnifiedScore,
};

// Re-export construction functions for backward compatibility
pub use super::construction::{
    create_unified_debt_item_enhanced, create_unified_debt_item_with_aggregator,
    create_unified_debt_item_with_aggregator_and_data_flow,
    create_unified_debt_item_with_exclusions,
    create_unified_debt_item_with_exclusions_and_data_flow,
};

// Re-export computation functions for backward compatibility
pub(super) use super::computation::{calculate_entropy_details, calculate_expected_impact};

// Import computation functions for tests
#[cfg(test)]
use super::computation::{
    calculate_coverage_improvement, calculate_lines_reduction, calculate_risk_factor,
    is_function_complex,
};

// Import types for tests
#[cfg(test)]
use crate::priority::FunctionVisibility;

// Re-export formatting and helper functions for backward compatibility
pub use super::formatting::determine_visibility;

// Import and re-export classification functions for backward compatibility
pub use super::classification::{
    classify_debt_type_with_exclusions, classify_risk_based_debt, classify_simple_function_risk,
    classify_test_debt, is_complexity_hotspot, is_dead_code, is_dead_code_with_exclusions,
};

/// Enhanced version of debt type classification (legacy - kept for compatibility)
pub fn classify_debt_type_enhanced(
    func: &FunctionMetrics,
    call_graph: &CallGraph,
    func_id: &FunctionId,
) -> DebtType {
    // Test functions are special debt cases
    if func.is_test {
        return classify_test_debt(func);
    }

    let role = classify_function_role(func, func_id, call_graph);

    // Check for complexity hotspots
    if let Some(debt) = is_complexity_hotspot(func, &role) {
        return debt;
    }

    // Check for dead code
    // Spec 262: usage_hints removed - just provide empty vector
    if is_dead_code(func, call_graph, func_id, None) {
        return DebtType::DeadCode {
            visibility: determine_visibility(func),
            cyclomatic: func.cyclomatic,
            cognitive: func.cognitive,
            usage_hints: vec![], // Spec 262: hints removed
        };
    }

    // Check for simple functions that aren't debt
    if let Some(debt) = classify_simple_function_risk(func, &role) {
        return debt;
    }

    // Default to risk-based classification
    classify_risk_based_debt(func, &role)
}

/// Generate recommendation for a debt item (spec 262: returns empty recommendation)
///
/// Recommendations have been removed from debtmap. This function returns an empty
/// recommendation for backward compatibility. AI agents should determine appropriate
/// fixes based on the debt type and metrics provided.
pub(super) fn generate_recommendation(
    _func: &FunctionMetrics,
    _debt_type: &DebtType,
    _role: FunctionRole,
    _score: &UnifiedScore,
) -> Option<ActionableRecommendation> {
    // Spec 262: Recommendations removed - return empty recommendation
    Some(ActionableRecommendation::default())
}

/// Generate recommendation with coverage and data flow (spec 262: returns empty)
pub(super) fn generate_recommendation_with_coverage_and_data_flow(
    _func: &FunctionMetrics,
    _debt_type: &DebtType,
    _role: FunctionRole,
    _score: &UnifiedScore,
    _coverage: &Option<TransitiveCoverage>,
    _data_flow: Option<&crate::data_flow::DataFlowGraph>,
) -> Option<ActionableRecommendation> {
    // Spec 262: Recommendations removed - return empty recommendation
    Some(ActionableRecommendation::default())
}

// Spec 262: The above recommendation generation code has been removed.
// Debtmap now focuses on identification and severity quantification.
// AI agents should determine appropriate fixes based on the debt type and metrics.

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_float_eq(left: f64, right: f64, epsilon: f64) {
        if (left - right).abs() > epsilon {
            panic!("assertion failed: `(left == right)`\n  left: `{}`,\n right: `{}`\n  diff: `{}`\nepsilon: `{}`", left, right, (left - right).abs(), epsilon);
        }
    }

    // Spec 262: Recommendation-related tests have been removed since
    // recommendation generation is no longer part of debtmap

    #[test]
    fn test_classify_test_debt() {
        let test_func = FunctionMetrics {
            name: "test_something".to_string(),
            file: std::path::PathBuf::from("tests/test.rs"),
            line: 10,
            length: 20,
            cyclomatic: 4,
            cognitive: 6,
            nesting: 1,
            visibility: Some("pub".to_string()),
            is_test: true,
            is_trait_method: false,
            in_test_module: false,
            entropy_score: None,
            is_pure: Some(false),
            purity_confidence: Some(0.3),
            purity_reason: None,
            call_dependencies: None,
            detected_patterns: None,
            upstream_callers: None,
            downstream_callees: None,
            mapping_pattern_result: None,
            adjusted_complexity: None,
            composition_metrics: None,
            language_specific: None,
            purity_level: None,
            error_swallowing_count: None,
            error_swallowing_patterns: None,
            entropy_analysis: None,
        };

        let debt = classify_test_debt(&test_func);
        match debt {
            DebtType::TestingGap {
                coverage,
                cyclomatic,
                cognitive,
            } => {
                assert_float_eq(coverage, 0.0, 0.01);
                assert_eq!(cyclomatic, 4);
                assert_eq!(cognitive, 6);
            }
            _ => panic!("Expected TestingGap debt type for test function"),
        }
    }

    #[test]
    fn test_generate_recommendation_returns_empty() {
        // Spec 262: Recommendations now return empty defaults
        let func = FunctionMetrics {
            name: "test_func".to_string(),
            file: std::path::PathBuf::from("test.rs"),
            line: 1,
            cyclomatic: 5,
            cognitive: 8,
            nesting: 2,
            length: 20,
            is_test: false,
            visibility: None,
            is_trait_method: false,
            in_test_module: false,
            entropy_score: None,
            is_pure: Some(true),
            purity_confidence: Some(1.0),
            purity_reason: None,
            call_dependencies: None,
            detected_patterns: None,
            upstream_callers: None,
            downstream_callees: None,
            mapping_pattern_result: None,
            adjusted_complexity: None,
            composition_metrics: None,
            language_specific: None,
            purity_level: None,
            error_swallowing_count: None,
            error_swallowing_patterns: None,
            entropy_analysis: None,
        };

        let debt_type = DebtType::ComplexityHotspot {
            cyclomatic: 15,
            cognitive: 20,
        };

        let score = UnifiedScore {
            complexity_factor: 7.5,
            coverage_factor: 6.0,
            dependency_factor: 2.0,
            role_multiplier: 1.2,
            final_score: 8.5,
            base_score: None,
            exponential_factor: None,
            risk_boost: None,
            pre_adjustment_score: None,
            adjustment_applied: None,
            purity_factor: None,
            refactorability_factor: None,
            pattern_factor: None,
            debt_adjustment: None,
            pre_normalization_score: None,
            structural_multiplier: Some(1.0),
            has_coverage_data: false,
            contextual_risk_multiplier: None,
            pre_contextual_score: None,
        };

        let recommendation =
            generate_recommendation(&func, &debt_type, FunctionRole::PureLogic, &score);

        // Should return Some with empty recommendation
        assert!(recommendation.is_some());
        let rec = recommendation.unwrap();
        assert!(rec.primary_action.is_empty());
        assert!(rec.rationale.is_empty());
        assert!(rec.implementation_steps.is_empty());
    }

    #[test]
    fn test_is_function_complex() {
        // Test not complex
        assert!(!is_function_complex(5, 10));
        assert!(!is_function_complex(10, 15));

        // Test complex based on cyclomatic
        assert!(is_function_complex(11, 10));
        assert!(is_function_complex(20, 5));

        // Test complex based on cognitive
        assert!(is_function_complex(5, 16));
        assert!(is_function_complex(10, 20));

        // Test complex based on both
        assert!(is_function_complex(15, 20));
    }

    #[test]
    fn test_calculate_risk_factor() {
        // Test various debt types
        assert_eq!(
            calculate_risk_factor(&DebtType::TestingGap {
                coverage: 0.5,
                cyclomatic: 10,
                cognitive: 15
            }),
            0.42
        );

        assert_eq!(
            calculate_risk_factor(&DebtType::ComplexityHotspot {
                cyclomatic: 20,
                cognitive: 25,
            }),
            0.35
        );

        assert_eq!(
            calculate_risk_factor(&DebtType::ErrorSwallowing {
                pattern: "unwrap_or_default".to_string(),
                context: None
            }),
            0.35
        );

        assert_eq!(
            calculate_risk_factor(&DebtType::DeadCode {
                visibility: FunctionVisibility::Private,
                cyclomatic: 5,
                cognitive: 8,
                usage_hints: vec![]
            }),
            0.3
        );
    }

    #[test]
    fn test_calculate_coverage_improvement() {
        // Test simple function
        assert_float_eq(calculate_coverage_improvement(0.0, false), 100.0, 1e-10);
        assert_float_eq(calculate_coverage_improvement(0.5, false), 50.0, 1e-10);
        assert_float_eq(calculate_coverage_improvement(0.8, false), 20.0, 1e-10);
        assert_float_eq(calculate_coverage_improvement(1.0, false), 0.0, 1e-10);

        // Test complex function (50% reduction)
        assert_float_eq(calculate_coverage_improvement(0.0, true), 50.0, 1e-10);
        assert_float_eq(calculate_coverage_improvement(0.5, true), 25.0, 1e-10);
        assert_float_eq(calculate_coverage_improvement(0.8, true), 10.0, 1e-10);
        assert_float_eq(calculate_coverage_improvement(1.0, true), 0.0, 1e-10);
    }

    #[test]
    fn test_calculate_lines_reduction() {
        // Test dead code
        let dead_code = DebtType::DeadCode {
            visibility: FunctionVisibility::Private,
            cyclomatic: 10,
            cognitive: 15,
            usage_hints: vec![],
        };
        assert_eq!(calculate_lines_reduction(&dead_code), 25);

        // Test duplication
        let duplication = DebtType::Duplication {
            instances: 4,
            total_lines: 100,
        };
        assert_eq!(calculate_lines_reduction(&duplication), 75);

        // Test other types
        let complexity = DebtType::ComplexityHotspot {
            cyclomatic: 20,
            cognitive: 25,
        };
        assert_eq!(calculate_lines_reduction(&complexity), 0);
    }

}
