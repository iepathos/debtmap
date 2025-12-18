//! Testing recommendation formatting functions
//!
//! Contains functions for analyzing testing gaps and generating
//! ROI-based testing recommendations.

use crate::priority::UnifiedDebtItem;

fn estimate_risk_reduction(coverage: f64) -> f64 {
    // Estimate risk reduction from improving coverage
    (1.0 - coverage) * 0.3
}

// Pure functions for testing recommendations
pub fn collect_testing_gaps(items: &[UnifiedDebtItem]) -> Vec<&UnifiedDebtItem> {
    items
        .iter()
        .filter(|item| matches!(item.debt_type, crate::priority::DebtType::TestingGap { .. }))
        .take(10)
        .collect()
}

fn format_testing_table_header() -> String {
    "### ROI-Based Testing Priorities\n\n\
     | Function | ROI | Complexity | Coverage | Risk Reduction |\n\
     |----------|-----|------------|----------|----------------|\n"
        .to_string()
}

fn format_testing_gap_row(item: &UnifiedDebtItem) -> Option<String> {
    if let crate::priority::DebtType::TestingGap {
        coverage,
        cyclomatic,
        cognitive: _,
    } = &item.debt_type
    {
        let risk_reduction = estimate_risk_reduction(*coverage);

        Some(format!(
            "| `{}` | {:.1} | {} | {:.0}% | {:.0}% |\n",
            item.location.function,
            0.0,
            cyclomatic,
            coverage * 100.0,
            risk_reduction * 100.0
        ))
    } else {
        None
    }
}

pub fn format_testing_recommendations(testing_gaps: &[&UnifiedDebtItem]) -> String {
    if testing_gaps.is_empty() {
        return "_All critical functions have adequate test coverage._\n\n".to_string();
    }

    let mut output = format_testing_table_header();

    for item in testing_gaps {
        if let Some(row) = format_testing_gap_row(item) {
            output.push_str(&row);
        }
    }
    output.push('\n');
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::priority::score_types::Score0To100;
    use crate::priority::unified_scorer::{Location, UnifiedScore};
    use crate::priority::{ActionableRecommendation, DebtType, FunctionRole, ImpactMetrics};

    fn create_testing_gap_item(
        function_name: &str,
        coverage: f64,
        cyclomatic: u32,
    ) -> UnifiedDebtItem {
        UnifiedDebtItem {
            location: Location {
                file: std::path::PathBuf::from("test.rs"),
                line: 10,
                function: function_name.to_string(),
            },
            unified_score: UnifiedScore {
                final_score: Score0To100::new(8.0),
                complexity_factor: 0.8,
                coverage_factor: 0.6,
                dependency_factor: 0.5,
                role_multiplier: 1.0,
                base_score: None,
                exponential_factor: None,
                risk_boost: None,
                pre_adjustment_score: None,
                adjustment_applied: None,
                purity_factor: None,
                refactorability_factor: None,
                pattern_factor: None,
                // Spec 260: Score transparency fields
                debt_adjustment: None,
                pre_normalization_score: None,
                structural_multiplier: Some(1.0),
                has_coverage_data: false,
                contextual_risk_multiplier: None,
                pre_contextual_score: None,
            },
            debt_type: DebtType::TestingGap {
                coverage,
                cyclomatic,
                cognitive: 20,
            },
            function_role: FunctionRole::PureLogic,
            recommendation: ActionableRecommendation {
                primary_action: "Add unit tests".to_string(),
                rationale: "Increase test coverage".to_string(),
                implementation_steps: vec![],
                related_items: vec![],
                steps: None,
                estimated_effort_hours: None,
            },
            expected_impact: ImpactMetrics {
                complexity_reduction: 0.0,
                coverage_improvement: 0.5,
                lines_reduction: 0,
                risk_reduction: 0.3,
            },
            transitive_coverage: None,
            file_context: None,
            upstream_dependencies: 1,
            downstream_dependencies: 2,
            upstream_callers: vec![],
            downstream_callees: vec![],
            nesting_depth: 0,
            function_length: 50,
            cyclomatic_complexity: cyclomatic,
            cognitive_complexity: 20,
            is_pure: None,
            purity_confidence: None,
            purity_level: None,
            entropy_details: None,
            entropy_adjusted_cognitive: None,
            entropy_dampening_factor: None,
            god_object_indicators: None,
            tier: None,
            function_context: None,
            context_confidence: None,
            contextual_recommendation: None,
            pattern_analysis: None,
            context_multiplier: None,
            context_type: None,
            language_specific: None, // spec 190
            detected_pattern: None,
            contextual_risk: None, // spec 203
            file_line_count: None,
            responsibility_category: None,
            error_swallowing_count: None,
            error_swallowing_patterns: None,
            entropy_analysis: None,
        }
    }

    fn create_test_item(function_name: &str, final_score: f64) -> UnifiedDebtItem {
        UnifiedDebtItem {
            location: Location {
                file: std::path::PathBuf::from("test.rs"),
                line: 10,
                function: function_name.to_string(),
            },
            unified_score: UnifiedScore {
                final_score: Score0To100::new(final_score),
                complexity_factor: 0.8,
                coverage_factor: 0.6,
                dependency_factor: 0.5,
                role_multiplier: 1.0,
                base_score: None,
                exponential_factor: None,
                risk_boost: None,
                pre_adjustment_score: None,
                adjustment_applied: None,
                purity_factor: None,
                refactorability_factor: None,
                pattern_factor: None,
                // Spec 260: Score transparency fields
                debt_adjustment: None,
                pre_normalization_score: None,
                structural_multiplier: Some(1.0),
                has_coverage_data: false,
                contextual_risk_multiplier: None,
                pre_contextual_score: None,
            },
            debt_type: DebtType::ComplexityHotspot {
                cyclomatic: 15,
                cognitive: 20,
            },
            function_role: FunctionRole::PureLogic,
            recommendation: ActionableRecommendation {
                primary_action: "Refactor to reduce complexity".to_string(),
                rationale: "Test recommendation".to_string(),
                implementation_steps: vec![],
                related_items: vec![],
                steps: None,
                estimated_effort_hours: None,
            },
            expected_impact: ImpactMetrics {
                complexity_reduction: 5.0,
                coverage_improvement: 0.1,
                lines_reduction: 10,
                risk_reduction: 0.2,
            },
            transitive_coverage: None,
            file_context: None,
            upstream_dependencies: 1,
            downstream_dependencies: 2,
            upstream_callers: vec![],
            downstream_callees: vec![],
            nesting_depth: 0,
            function_length: 50,
            cyclomatic_complexity: 15,
            cognitive_complexity: 20,
            is_pure: None,
            purity_confidence: None,
            purity_level: None,
            entropy_details: None,
            entropy_adjusted_cognitive: None,
            entropy_dampening_factor: None,
            god_object_indicators: None,
            tier: None,
            function_context: None,
            context_confidence: None,
            contextual_recommendation: None,
            pattern_analysis: None,
            context_multiplier: None,
            context_type: None,
            language_specific: None, // spec 190
            detected_pattern: None,
            contextual_risk: None, // spec 203
            file_line_count: None,
            responsibility_category: None,
            error_swallowing_count: None,
            error_swallowing_patterns: None,
            entropy_analysis: None,
        }
    }

    #[test]
    fn test_collect_testing_gaps() {
        let items = vec![
            create_testing_gap_item("func1", 0.2, 10),
            create_test_item("func2", 7.0), // Not a testing gap
            create_testing_gap_item("func3", 0.4, 15),
            create_testing_gap_item("func4", 0.1, 20),
        ];

        let gaps = collect_testing_gaps(&items);

        assert_eq!(gaps.len(), 3);
        assert_eq!(gaps[0].location.function, "func1");
        assert_eq!(gaps[1].location.function, "func3");
        assert_eq!(gaps[2].location.function, "func4");
    }

    #[test]
    fn test_collect_testing_gaps_empty() {
        let items = vec![
            create_test_item("func1", 7.0),
            create_test_item("func2", 8.0),
        ];

        let gaps = collect_testing_gaps(&items);

        assert!(gaps.is_empty());
    }

    #[test]
    fn test_collect_testing_gaps_limits_to_ten() {
        let mut items = Vec::new();
        for i in 0..15 {
            items.push(create_testing_gap_item(&format!("func{}", i), 0.2, 10));
        }

        let gaps = collect_testing_gaps(&items);

        assert_eq!(gaps.len(), 10);
    }

    #[test]
    fn test_format_testing_table_header() {
        let header = format_testing_table_header();

        assert!(header.contains("### ROI-Based Testing Priorities"));
        assert!(header.contains("| Function | ROI | Complexity | Coverage | Risk Reduction |"));
        assert!(header.contains("|----------|-----|------------|----------|----------------|"));
    }

    #[test]
    fn test_format_testing_gap_row() {
        let item = create_testing_gap_item("test_function", 0.3, 12);

        let row = format_testing_gap_row(&item).unwrap();

        assert!(row.contains("`test_function`"));
        assert!(row.contains("| 0.0 |")); // ROI removed from scoring
        assert!(row.contains("| 12 |")); // Cyclomatic complexity
        assert!(row.contains("| 30% |")); // Coverage
        assert!(row.contains("| 21% |")); // Risk reduction = (1-0.3)*0.3
    }

    #[test]
    fn test_format_testing_gap_row_non_testing_gap() {
        let item = create_test_item("test_function", 8.0);

        let row = format_testing_gap_row(&item);

        assert!(row.is_none());
    }

    #[test]
    fn test_format_testing_recommendations_empty() {
        let gaps: Vec<&UnifiedDebtItem> = vec![];

        let result = format_testing_recommendations(&gaps);

        assert!(result.contains("_All critical functions have adequate test coverage._"));
    }

    #[test]
    fn test_format_testing_recommendations_with_gaps() {
        let items = vec![
            create_testing_gap_item("func1", 0.2, 10),
            create_testing_gap_item("func2", 0.5, 15),
        ];
        let gaps: Vec<&UnifiedDebtItem> = items.iter().collect();

        let result = format_testing_recommendations(&gaps);

        assert!(result.contains("### ROI-Based Testing Priorities"));
        assert!(result.contains("`func1`"));
        assert!(result.contains("`func2`"));
        assert!(result.contains("| 10 |")); // func1 complexity
        assert!(result.contains("| 15 |")); // func2 complexity
    }

    #[test]
    fn test_estimate_risk_reduction() {
        assert_eq!(estimate_risk_reduction(0.0), 0.3); // (1.0 - 0.0) * 0.3
        assert_eq!(estimate_risk_reduction(0.5), 0.15); // (1.0 - 0.5) * 0.3
        assert_eq!(estimate_risk_reduction(1.0), 0.0); // (1.0 - 1.0) * 0.3
    }
}
