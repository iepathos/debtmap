//! Tests for debtmap comparison module.

use super::analysis::{build_function_map, identify_improved_items, identify_unchanged_critical};
use super::messages::{
    build_all_gaps, build_all_improvement_messages, build_complexity_message,
    build_coverage_message, build_critical_debt_gap, build_regression_gap,
    build_regression_message, build_resolved_message, build_unchanged_critical_message,
};
use super::perform_validation;
use super::scoring::{
    apply_minimum_threshold, apply_unchanged_penalty, determine_status, score_complexity_reduction,
    score_high_priority_progress, score_overall_improvement, score_regression_penalty,
};
use super::types::{
    is_critical, is_score_unchanged, AnalysisSummary, DebtmapJsonInput, ImprovedItems, ItemInfo,
    NewItems, ResolvedItems, UnchangedCritical, CRITICAL_SCORE_THRESHOLD,
};
use crate::output::unified::{
    Dependencies, FunctionDebtItemOutput, FunctionImpactOutput, FunctionMetricsOutput, Priority,
    RecommendationOutput, UnifiedDebtItemOutput, UnifiedLocation,
};
use crate::priority::semantic_classifier::FunctionRole;
use crate::priority::{DebtType, ImpactMetrics};
use std::path::PathBuf;

// =============================================================================
// Test Helper Functions
// =============================================================================

fn create_empty_output() -> DebtmapJsonInput {
    DebtmapJsonInput {
        items: vec![],
        total_impact: ImpactMetrics {
            complexity_reduction: 0.0,
            coverage_improvement: 0.0,
            risk_reduction: 0.0,
            lines_reduction: 0,
        },
        total_debt_score: 0.0,
        debt_density: 0.0,
        total_lines_of_code: 0,
        overall_coverage: None,
    }
}

fn create_output_with_items(items: Vec<UnifiedDebtItemOutput>) -> DebtmapJsonInput {
    DebtmapJsonInput {
        items,
        total_impact: ImpactMetrics {
            complexity_reduction: 0.0,
            coverage_improvement: 0.0,
            risk_reduction: 0.0,
            lines_reduction: 0,
        },
        total_debt_score: 0.0,
        debt_density: 0.0,
        total_lines_of_code: 1000,
        overall_coverage: None,
    }
}

/// Create a function item in the output format (UnifiedDebtItemOutput)
fn create_function_item(
    file: &str,
    function: &str,
    line: usize,
    score: f64,
    complexity: u32,
    coverage: Option<f64>,
) -> UnifiedDebtItemOutput {
    UnifiedDebtItemOutput::Function(Box::new(FunctionDebtItemOutput {
        score: score.max(0.0),
        category: "Complexity".to_string(),
        priority: if score >= 8.0 {
            Priority::Critical
        } else if score >= 5.0 {
            Priority::High
        } else if score >= 2.0 {
            Priority::Medium
        } else {
            Priority::Low
        },
        location: UnifiedLocation {
            file: file.to_string(),
            line: Some(line),
            function: Some(function.to_string()),
            file_context_label: None,
        },
        metrics: FunctionMetricsOutput {
            cyclomatic_complexity: complexity,
            cognitive_complexity: 0,
            length: 50,
            nesting_depth: 0,
            coverage,
            uncovered_lines: None,
            entropy_score: None,
            pattern_repetition: None,
            branch_similarity: None,
            entropy_adjusted_cognitive: None,
            transitive_coverage: None,
        },
        debt_type: DebtType::ComplexityHotspot {
            cyclomatic: complexity,
            cognitive: 0,
        },
        function_role: FunctionRole::Unknown,
        purity_analysis: None,
        dependencies: Dependencies::default(),
        recommendation: RecommendationOutput {
            action: "Refactor".to_string(),
            priority: None,
            implementation_steps: vec![],
        },
        impact: FunctionImpactOutput {
            complexity_reduction: 0.0,
            coverage_improvement: 0.0,
            risk_reduction: 0.0,
        },
        scoring_details: None,
        adjusted_complexity: None,
        complexity_pattern: None,
        pattern_type: None,
        pattern_confidence: None,
        pattern_details: None,
        context: None,
        git_history: None,
    }))
}

/// Create a test function item with coverage support
fn create_test_function_item(
    file: &str,
    function: &str,
    score: f64,
    complexity: u32,
    coverage: Option<f64>,
) -> UnifiedDebtItemOutput {
    create_function_item(file, function, 1, score, complexity, coverage)
}

/// Create a test debt item (returns UnifiedDebtItemOutput for use in tests)
fn create_test_debt_item(
    file: &str,
    function: &str,
    line: usize,
    score: f64,
) -> UnifiedDebtItemOutput {
    create_function_item(file, function, line, score, 5, None)
}

fn create_test_output(items: Vec<UnifiedDebtItemOutput>) -> DebtmapJsonInput {
    DebtmapJsonInput {
        items,
        total_impact: ImpactMetrics {
            risk_reduction: 0.0,
            complexity_reduction: 0.0,
            coverage_improvement: 0.0,
            lines_reduction: 0,
        },
        total_debt_score: 0.0,
        debt_density: 0.0,
        total_lines_of_code: 1000,
        overall_coverage: Some(50.0),
    }
}

// =============================================================================
// Perform Validation Tests
// =============================================================================

#[test]
fn test_perform_validation_no_improvements_or_issues() {
    let before = create_test_output(vec![]);
    let after = create_test_output(vec![]);

    let result = perform_validation(&before, &after).unwrap();

    assert_eq!(result.status, "complete");
    assert_eq!(result.improvements.len(), 0);
    assert_eq!(result.remaining_issues.len(), 0);
    assert_eq!(result.gaps.len(), 0);
    assert!(result.completion_percentage >= 75.0);
}

#[test]
fn test_perform_validation_resolved_high_priority() {
    let before = create_test_output(vec![create_function_item(
        "src/test.rs",
        "complex_fn",
        10,
        10.0,
        15,
        Some(0.0),
    )]);
    let after = create_test_output(vec![]);

    let result = perform_validation(&before, &after).unwrap();

    assert_eq!(result.status, "complete");
    assert!(result
        .improvements
        .iter()
        .any(|i| i.contains("Resolved 1 high-priority")));
    assert_eq!(result.remaining_issues.len(), 0);
    assert!(result.completion_percentage >= 75.0);
}

#[test]
fn test_perform_validation_complexity_reduction() {
    let before = create_test_output(vec![create_function_item(
        "src/test.rs",
        "fn1",
        10,
        10.0,
        20,
        Some(0.5),
    )]);
    let after = create_test_output(vec![create_function_item(
        "src/test.rs",
        "fn1",
        10,
        8.0,
        10,
        Some(0.5),
    )]);

    let result = perform_validation(&before, &after).unwrap();

    assert!(result
        .improvements
        .iter()
        .any(|i| i.contains("Reduced average cyclomatic complexity")));
}

#[test]
fn test_perform_validation_coverage_improvement() {
    let before = create_test_output(vec![create_function_item(
        "src/test.rs",
        "fn1",
        10,
        10.0,
        10,
        Some(0.0),
    )]);
    let after = create_test_output(vec![create_function_item(
        "src/test.rs",
        "fn1",
        10,
        8.0,
        10,
        Some(0.8),
    )]);

    let result = perform_validation(&before, &after).unwrap();

    assert!(result
        .improvements
        .iter()
        .any(|i| i.contains("Added test coverage")));
}

#[test]
fn test_perform_validation_unchanged_critical() {
    let before = create_test_output(vec![create_function_item(
        "src/test.rs",
        "complex_fn",
        10,
        10.0,
        15,
        Some(0.0),
    )]);
    let after = create_test_output(vec![create_function_item(
        "src/test.rs",
        "complex_fn",
        10,
        10.0,
        15,
        Some(0.0),
    )]);

    let result = perform_validation(&before, &after).unwrap();

    assert!(result
        .remaining_issues
        .iter()
        .any(|i| i.contains("critical debt item")));
    assert!(result.gaps.contains_key("critical_debt_remaining_0"));
}

#[test]
fn test_perform_validation_new_critical_regression() {
    let before = create_test_output(vec![]);
    let after = create_test_output(vec![create_function_item(
        "src/new.rs",
        "bad_fn",
        20,
        12.0,
        20,
        Some(0.0),
    )]);

    let result = perform_validation(&before, &after).unwrap();

    assert!(result
        .remaining_issues
        .iter()
        .any(|i| i.contains("new critical debt items")));
    assert!(result.gaps.contains_key("regression_detected"));
    assert_eq!(result.status, "failed");
}

#[test]
fn test_perform_validation_combined_improvements() {
    let before = create_test_output(vec![
        create_function_item("src/test.rs", "fn1", 10, 10.0, 20, Some(0.0)),
        create_function_item("src/test.rs", "fn2", 30, 9.0, 15, Some(0.2)),
    ]);
    let after = create_test_output(vec![create_function_item(
        "src/test.rs",
        "fn2",
        30,
        7.0,
        10,
        Some(0.8),
    )]);

    let result = perform_validation(&before, &after).unwrap();

    assert!(result.improvements.len() >= 2);
    assert!(result.improvements.iter().any(|i| i.contains("Resolved")));
    assert!(result
        .improvements
        .iter()
        .any(|i| i.contains("complexity") || i.contains("coverage")));
    assert_eq!(result.status, "complete");
}

#[test]
fn test_perform_validation_status_complete() {
    let before = create_test_output(vec![create_function_item(
        "src/test.rs",
        "fn1",
        10,
        10.0,
        15,
        Some(0.0),
    )]);
    let after = create_test_output(vec![]);

    let result = perform_validation(&before, &after).unwrap();

    assert_eq!(result.status, "complete");
    assert!(result.completion_percentage >= 75.0);
}

#[test]
fn test_perform_validation_status_incomplete() {
    let before = create_test_output(vec![
        create_function_item("src/test.rs", "fn1", 10, 10.0, 15, Some(0.0)),
        create_function_item("src/test.rs", "fn2", 20, 11.0, 20, Some(0.0)),
    ]);
    let after = create_test_output(vec![
        create_function_item("src/test.rs", "fn1", 10, 8.0, 10, Some(0.5)),
        create_function_item("src/test.rs", "fn2", 20, 11.0, 20, Some(0.0)),
    ]);

    let result = perform_validation(&before, &after).unwrap();

    assert!(result.completion_percentage >= 40.0 && result.completion_percentage < 75.0);
    assert_eq!(result.status, "incomplete");
}

#[test]
fn test_perform_validation_status_failed() {
    let before = create_test_output(vec![create_function_item(
        "src/test.rs",
        "fn1",
        10,
        10.0,
        15,
        Some(0.0),
    )]);
    let after = create_test_output(vec![
        create_function_item("src/test.rs", "fn1", 10, 10.0, 15, Some(0.0)),
        create_function_item("src/test.rs", "fn2", 20, 12.0, 20, Some(0.0)),
    ]);

    let result = perform_validation(&before, &after).unwrap();

    assert!(result.completion_percentage < 40.0);
    assert_eq!(result.status, "failed");
}

#[test]
fn test_perform_validation_gap_detail_generation() {
    let before = create_test_output(vec![create_function_item(
        "src/test.rs",
        "critical_fn",
        10,
        10.0,
        15,
        Some(0.0),
    )]);
    let after = create_test_output(vec![create_function_item(
        "src/test.rs",
        "critical_fn",
        10,
        10.0,
        15,
        Some(0.0),
    )]);

    let result = perform_validation(&before, &after).unwrap();

    assert!(result.gaps.contains_key("critical_debt_remaining_0"));
    let gap = result.gaps.get("critical_debt_remaining_0").unwrap();
    assert_eq!(gap.severity, "high");
    assert!(gap.location.contains("src/test.rs"));
    assert!(gap.location.contains("critical_fn"));
    assert_eq!(gap.original_score, Some(10.0));
    assert_eq!(gap.current_score, Some(10.0));
}

#[test]
fn test_perform_validation_multiple_unchanged_critical() {
    let before = create_test_output(vec![
        create_function_item("src/test.rs", "fn1", 10, 10.0, 15, Some(0.0)),
        create_function_item("src/test.rs", "fn2", 20, 11.0, 20, Some(0.0)),
        create_function_item("src/test.rs", "fn3", 30, 12.0, 25, Some(0.0)),
    ]);
    let after = create_test_output(vec![
        create_function_item("src/test.rs", "fn1", 10, 10.0, 15, Some(0.0)),
        create_function_item("src/test.rs", "fn2", 20, 11.0, 20, Some(0.0)),
        create_function_item("src/test.rs", "fn3", 30, 12.0, 25, Some(0.0)),
    ]);

    let result = perform_validation(&before, &after).unwrap();

    assert!(result
        .remaining_issues
        .iter()
        .any(|i| i.contains("3 critical debt items")));
    assert_eq!(result.gaps.len(), 2); // Only first 2 are added
    assert!(result.gaps.contains_key("critical_debt_remaining_0"));
    assert!(result.gaps.contains_key("critical_debt_remaining_1"));
}

// =============================================================================
// Identify Improved Items Tests
// =============================================================================

#[test]
fn test_empty_before_and_after() {
    let before = create_empty_output();
    let after = create_empty_output();

    let result = identify_improved_items(&before, &after);

    assert_eq!(result.complexity_reduction, 0.0);
    assert_eq!(result.coverage_improvement, 0.0);
    assert_eq!(result.coverage_improvement_count, 0);
}

#[test]
fn test_no_improvements_below_threshold() {
    let before = create_output_with_items(vec![create_test_function_item(
        "test.rs", "func1", 5.0, 10, None,
    )]);
    let after = create_output_with_items(vec![create_test_function_item(
        "test.rs", "func1", 4.6, 10, None,
    )]);

    let result = identify_improved_items(&before, &after);

    assert_eq!(result.complexity_reduction, 0.0);
    assert_eq!(result.coverage_improvement, 0.0);
    assert_eq!(result.coverage_improvement_count, 0);
}

#[test]
fn test_score_improvement_above_threshold_with_complexity_reduction() {
    let before = create_output_with_items(vec![create_test_function_item(
        "test.rs", "func1", 10.0, 20, None,
    )]);
    let after = create_output_with_items(vec![create_test_function_item(
        "test.rs", "func1", 9.0, 10, None,
    )]);

    let result = identify_improved_items(&before, &after);

    assert!(result.complexity_reduction > 0.0);
    assert_eq!((result.complexity_reduction * 100.0).round() / 100.0, 0.5);
    assert_eq!(result.coverage_improvement_count, 0);
}

#[test]
fn test_score_improvement_with_coverage_increase() {
    let before = create_output_with_items(vec![create_test_function_item(
        "test.rs",
        "func1",
        10.0,
        10,
        Some(0.3),
    )]);
    let after = create_output_with_items(vec![create_test_function_item(
        "test.rs",
        "func1",
        9.0,
        10,
        Some(0.8),
    )]);

    let result = identify_improved_items(&before, &after);

    assert_eq!(result.coverage_improvement_count, 1);
    assert_eq!(result.coverage_improvement, 1.0);
}

#[test]
fn test_multiple_improvements() {
    let before = create_output_with_items(vec![
        create_test_function_item("test.rs", "func1", 10.0, 20, None),
        create_test_function_item("test.rs", "func2", 8.0, 15, None),
        create_test_function_item("test.rs", "func3", 6.0, 10, None),
    ]);
    let after = create_output_with_items(vec![
        create_test_function_item("test.rs", "func1", 9.0, 10, None),
        create_test_function_item("test.rs", "func2", 7.0, 8, None),
        create_test_function_item("test.rs", "func3", 5.0, 5, None),
    ]);

    let result = identify_improved_items(&before, &after);

    assert!(result.complexity_reduction > 0.0);
}

// =============================================================================
// Identify Unchanged Critical Tests
// =============================================================================

#[test]
fn test_identify_unchanged_critical_empty_inputs() {
    let before = create_test_output(vec![]);
    let after = create_test_output(vec![]);

    let result = identify_unchanged_critical(&before, &after);

    assert_eq!(result.count, 0);
    assert_eq!(result.items.len(), 0);
}

#[test]
fn test_identify_unchanged_critical_no_critical_items() {
    let before = create_test_output(vec![
        create_test_debt_item("src/foo.rs", "low_score", 10, 5.0),
        create_test_debt_item("src/bar.rs", "another_low", 20, 7.5),
    ]);
    let after = create_test_output(vec![
        create_test_debt_item("src/foo.rs", "low_score", 10, 5.2),
        create_test_debt_item("src/bar.rs", "another_low", 20, 7.3),
    ]);

    let result = identify_unchanged_critical(&before, &after);

    assert_eq!(result.count, 0);
    assert_eq!(result.items.len(), 0);
}

#[test]
fn test_identify_unchanged_critical_items_resolved() {
    let before = create_test_output(vec![create_test_debt_item(
        "src/foo.rs",
        "critical_fn",
        10,
        9.0,
    )]);
    let after = create_test_output(vec![]);

    let result = identify_unchanged_critical(&before, &after);

    assert_eq!(result.count, 0);
    assert_eq!(result.items.len(), 0);
}

#[test]
fn test_identify_unchanged_critical_items_unchanged() {
    let before = create_test_output(vec![
        create_test_debt_item("src/foo.rs", "critical_fn", 10, 9.0),
        create_test_debt_item("src/bar.rs", "another_critical", 20, 10.5),
    ]);
    let after = create_test_output(vec![
        create_test_debt_item("src/foo.rs", "critical_fn", 10, 9.2),
        create_test_debt_item("src/bar.rs", "another_critical", 20, 10.3),
    ]);

    let result = identify_unchanged_critical(&before, &after);

    assert_eq!(result.count, 2);
    assert_eq!(result.items.len(), 2);
    assert_eq!(result.items[0].function, "critical_fn");
    assert_eq!(result.items[0].score, 9.0);
    assert_eq!(result.items[1].function, "another_critical");
    assert_eq!(result.items[1].score, 10.5);
}

// =============================================================================
// Is Critical / Is Score Unchanged Tests
// =============================================================================

#[test]
fn test_is_critical_below_threshold() {
    assert!(!is_critical(7.9));
    assert!(!is_critical(0.0));
    assert!(!is_critical(5.5));
}

#[test]
fn test_is_critical_at_threshold() {
    assert!(is_critical(8.0));
}

#[test]
fn test_is_critical_above_threshold() {
    assert!(is_critical(8.1));
    assert!(is_critical(10.0));
    assert!(is_critical(15.5));
}

#[test]
fn test_is_score_unchanged_exactly_equal() {
    assert!(is_score_unchanged(9.0, 9.0));
    assert!(is_score_unchanged(0.0, 0.0));
}

#[test]
fn test_is_score_unchanged_within_tolerance() {
    assert!(is_score_unchanged(9.0, 9.3));
    assert!(is_score_unchanged(9.3, 9.0));
    assert!(is_score_unchanged(10.0, 10.49));
    assert!(is_score_unchanged(10.49, 10.0));
}

#[test]
fn test_is_score_unchanged_at_boundary() {
    assert!(!is_score_unchanged(9.0, 8.5));
    assert!(!is_score_unchanged(8.5, 9.0));
}

#[test]
fn test_is_score_unchanged_outside_tolerance() {
    assert!(!is_score_unchanged(9.0, 8.4));
    assert!(!is_score_unchanged(8.4, 9.0));
    assert!(!is_score_unchanged(10.0, 11.0));
    assert!(!is_score_unchanged(5.0, 7.0));
}

// =============================================================================
// Build Function Map Tests
// =============================================================================

#[test]
fn test_build_function_map_empty() {
    let items: Vec<UnifiedDebtItemOutput> = vec![];
    let result = build_function_map(&items);
    assert_eq!(result.len(), 0);
}

#[test]
fn test_build_function_map_only_functions() {
    let items = vec![
        create_test_debt_item("src/foo.rs", "func1", 10, 9.0),
        create_test_debt_item("src/bar.rs", "func2", 20, 8.5),
    ];

    let result = build_function_map(&items);

    assert_eq!(result.len(), 2);
    assert!(result.contains_key(&(PathBuf::from("src/foo.rs"), "func1".to_string())));
    assert!(result.contains_key(&(PathBuf::from("src/bar.rs"), "func2".to_string())));
}

// =============================================================================
// Scoring Tests
// =============================================================================

#[test]
fn test_score_high_priority_progress_all_resolved() {
    let before = AnalysisSummary {
        total_items: 5,
        high_priority_items: 5,
        average_score: 10.0,
    };
    let after = AnalysisSummary {
        total_items: 0,
        high_priority_items: 0,
        average_score: 0.0,
    };
    let resolved = ResolvedItems {
        high_priority_count: 5,
        total_count: 5,
    };

    let score = score_high_priority_progress(&before, &after, &resolved);
    assert_eq!(score, 100.0);
}

#[test]
fn test_score_high_priority_progress_partial() {
    let before = AnalysisSummary {
        total_items: 10,
        high_priority_items: 10,
        average_score: 10.0,
    };
    let after = AnalysisSummary {
        total_items: 5,
        high_priority_items: 5,
        average_score: 8.0,
    };
    let resolved = ResolvedItems {
        high_priority_count: 3,
        total_count: 5,
    };

    let score = score_high_priority_progress(&before, &after, &resolved);
    assert_eq!(score, 50.0);
}

#[test]
fn test_score_high_priority_progress_no_high_priority() {
    let before = AnalysisSummary {
        total_items: 10,
        high_priority_items: 0,
        average_score: 5.0,
    };
    let after = AnalysisSummary {
        total_items: 10,
        high_priority_items: 0,
        average_score: 5.0,
    };
    let resolved = ResolvedItems {
        high_priority_count: 0,
        total_count: 0,
    };

    let score = score_high_priority_progress(&before, &after, &resolved);
    assert_eq!(score, 100.0);
}

#[test]
fn test_score_overall_improvement_significant() {
    let before = AnalysisSummary {
        total_items: 10,
        high_priority_items: 5,
        average_score: 10.0,
    };
    let after = AnalysisSummary {
        total_items: 10,
        high_priority_items: 3,
        average_score: 8.0,
    };

    let score = score_overall_improvement(&before, &after);
    assert_eq!(score, 20.0);
}

#[test]
fn test_score_overall_improvement_zero_before() {
    let before = AnalysisSummary {
        total_items: 0,
        high_priority_items: 0,
        average_score: 0.0,
    };
    let after = AnalysisSummary {
        total_items: 5,
        high_priority_items: 2,
        average_score: 5.0,
    };

    let score = score_overall_improvement(&before, &after);
    assert_eq!(score, 0.0);
}

#[test]
fn test_score_complexity_reduction() {
    let improved = ImprovedItems {
        complexity_reduction: 0.5,
        coverage_improvement: 0.0,
        coverage_improvement_count: 0,
    };

    let score = score_complexity_reduction(&improved);
    assert_eq!(score, 50.0);
}

#[test]
fn test_score_regression_penalty_no_regressions() {
    let new_items = NewItems {
        critical_count: 0,
        items: vec![],
    };
    assert_eq!(score_regression_penalty(&new_items), 100.0);
}

#[test]
fn test_score_regression_penalty_with_regressions() {
    let new_items = NewItems {
        critical_count: 3,
        items: vec![],
    };
    assert_eq!(score_regression_penalty(&new_items), 0.0);
}

#[test]
fn test_apply_unchanged_penalty_no_unchanged() {
    let unchanged = UnchangedCritical {
        count: 0,
        items: vec![],
    };
    let score = apply_unchanged_penalty(80.0, &unchanged, true);
    assert_eq!(score, 80.0);
}

#[test]
fn test_apply_unchanged_penalty_with_improvements() {
    let unchanged = UnchangedCritical {
        count: 2,
        items: vec![],
    };
    let score = apply_unchanged_penalty(80.0, &unchanged, true);
    assert_eq!(score, 72.0);
}

#[test]
fn test_apply_unchanged_penalty_without_improvements() {
    let unchanged = UnchangedCritical {
        count: 2,
        items: vec![],
    };
    let score = apply_unchanged_penalty(80.0, &unchanged, false);
    assert_eq!(score, 64.0);
}

#[test]
fn test_apply_minimum_threshold_boosts_low_score() {
    let score = apply_minimum_threshold(30.0, true, 10.0);
    assert_eq!(score, 40.0);
}

#[test]
fn test_apply_minimum_threshold_no_boost_when_no_improvements() {
    let score = apply_minimum_threshold(30.0, false, 10.0);
    assert_eq!(score, 30.0);
}

#[test]
fn test_apply_minimum_threshold_clamps_to_100() {
    let score = apply_minimum_threshold(150.0, true, 10.0);
    assert_eq!(score, 100.0);
}

#[test]
fn test_determine_status_complete_all_high_priority_addressed() {
    let new_items = NewItems {
        critical_count: 0,
        items: vec![],
    };
    let before = AnalysisSummary {
        total_items: 10,
        high_priority_items: 5,
        average_score: 10.0,
    };
    let after = AnalysisSummary {
        total_items: 5,
        high_priority_items: 0,
        average_score: 5.0,
    };

    let status = determine_status(60.0, &new_items, &before, &after);
    assert_eq!(status, "complete");
}

#[test]
fn test_determine_status_failed_with_regressions() {
    let new_items = NewItems {
        critical_count: 2,
        items: vec![],
    };
    let before = AnalysisSummary {
        total_items: 10,
        high_priority_items: 5,
        average_score: 10.0,
    };
    let after = AnalysisSummary {
        total_items: 10,
        high_priority_items: 3,
        average_score: 8.0,
    };

    let status = determine_status(80.0, &new_items, &before, &after);
    assert_eq!(status, "failed");
}

#[test]
fn test_determine_status_incomplete() {
    let new_items = NewItems {
        critical_count: 0,
        items: vec![],
    };
    let before = AnalysisSummary {
        total_items: 10,
        high_priority_items: 5,
        average_score: 10.0,
    };
    let after = AnalysisSummary {
        total_items: 8,
        high_priority_items: 3,
        average_score: 8.0,
    };

    let status = determine_status(55.0, &new_items, &before, &after);
    assert_eq!(status, "incomplete");
}

// =============================================================================
// Message Builder Tests
// =============================================================================

#[test]
fn test_build_resolved_message_with_count() {
    let resolved = ResolvedItems {
        high_priority_count: 3,
        total_count: 5,
    };
    let msg = build_resolved_message(&resolved);
    assert_eq!(msg, Some("Resolved 3 high-priority debt items".to_string()));
}

#[test]
fn test_build_resolved_message_zero() {
    let resolved = ResolvedItems {
        high_priority_count: 0,
        total_count: 0,
    };
    let msg = build_resolved_message(&resolved);
    assert_eq!(msg, None);
}

#[test]
fn test_build_complexity_message_with_reduction() {
    let improved = ImprovedItems {
        complexity_reduction: 0.25,
        coverage_improvement: 0.0,
        coverage_improvement_count: 0,
    };
    let msg = build_complexity_message(&improved);
    assert_eq!(
        msg,
        Some("Reduced average cyclomatic complexity by 25%".to_string())
    );
}

#[test]
fn test_build_coverage_message_with_improvement() {
    let improved = ImprovedItems {
        complexity_reduction: 0.0,
        coverage_improvement: 2.0,
        coverage_improvement_count: 2,
    };
    let msg = build_coverage_message(&improved);
    assert_eq!(
        msg,
        Some("Added test coverage for 2 critical functions".to_string())
    );
}

#[test]
fn test_build_all_improvement_messages() {
    let resolved = ResolvedItems {
        high_priority_count: 2,
        total_count: 3,
    };
    let improved = ImprovedItems {
        complexity_reduction: 0.3,
        coverage_improvement: 1.0,
        coverage_improvement_count: 1,
    };

    let messages = build_all_improvement_messages(&resolved, &improved);
    assert_eq!(messages.len(), 3);
}

#[test]
fn test_build_unchanged_critical_message_singular() {
    let unchanged = UnchangedCritical {
        count: 1,
        items: vec![],
    };
    let msg = build_unchanged_critical_message(&unchanged);
    assert_eq!(msg, Some("1 critical debt item still present".to_string()));
}

#[test]
fn test_build_unchanged_critical_message_plural() {
    let unchanged = UnchangedCritical {
        count: 3,
        items: vec![],
    };
    let msg = build_unchanged_critical_message(&unchanged);
    assert_eq!(msg, Some("3 critical debt items still present".to_string()));
}

#[test]
fn test_build_regression_message_with_regressions() {
    let new_items = NewItems {
        critical_count: 2,
        items: vec![],
    };
    let msg = build_regression_message(&new_items);
    assert_eq!(
        msg,
        Some("2 new critical debt items introduced".to_string())
    );
}

// =============================================================================
// Gap Builder Tests
// =============================================================================

#[test]
fn test_build_critical_debt_gap() {
    let item = ItemInfo {
        file: PathBuf::from("src/test.rs"),
        function: "complex_fn".to_string(),
        line: 42,
        score: 9.5,
    };

    let (key, detail) = build_critical_debt_gap(&item, 0);
    assert_eq!(key, "critical_debt_remaining_0");
    assert_eq!(detail.severity, "high");
    assert!(detail.location.contains("src/test.rs"));
    assert!(detail.location.contains("complex_fn"));
    assert_eq!(detail.original_score, Some(9.5));
}

#[test]
fn test_build_regression_gap_none_when_no_regressions() {
    let new_items = NewItems {
        critical_count: 0,
        items: vec![],
    };
    let gap = build_regression_gap(&new_items);
    assert!(gap.is_none());
}

#[test]
fn test_build_regression_gap_with_regressions() {
    let new_items = NewItems {
        critical_count: 1,
        items: vec![ItemInfo {
            file: PathBuf::from("src/new.rs"),
            function: "bad_fn".to_string(),
            line: 10,
            score: 12.0,
        }],
    };
    let gap = build_regression_gap(&new_items);
    assert!(gap.is_some());
    let (key, detail) = gap.unwrap();
    assert_eq!(key, "regression_detected");
    assert_eq!(detail.current_score, Some(12.0));
}

#[test]
fn test_build_all_gaps_combined() {
    let unchanged = UnchangedCritical {
        count: 2,
        items: vec![
            ItemInfo {
                file: PathBuf::from("src/a.rs"),
                function: "fn1".to_string(),
                line: 10,
                score: 9.0,
            },
            ItemInfo {
                file: PathBuf::from("src/b.rs"),
                function: "fn2".to_string(),
                line: 20,
                score: 10.0,
            },
        ],
    };
    let new_items = NewItems {
        critical_count: 1,
        items: vec![ItemInfo {
            file: PathBuf::from("src/c.rs"),
            function: "fn3".to_string(),
            line: 30,
            score: 11.0,
        }],
    };

    let gaps = build_all_gaps(&unchanged, &new_items);
    assert_eq!(gaps.len(), 3);
    assert!(gaps.contains_key("critical_debt_remaining_0"));
    assert!(gaps.contains_key("critical_debt_remaining_1"));
    assert!(gaps.contains_key("regression_detected"));
}

// =============================================================================
// Serialization Format Tests
// =============================================================================

/// Test that UnifiedDebtItemOutput JSON can be deserialized into DebtmapJsonInput.
/// This ensures the analyze output format is compatible with compare input.
#[test]
fn test_output_json_deserializes_for_compare() {
    use crate::output::unified::{
        DebtSummary, OutputMetadata, ScoreDistribution, TypeBreakdown, UnifiedOutput,
    };

    // Create a minimal UnifiedOutput similar to what debtmap analyze produces
    let unified_output = UnifiedOutput {
        format_version: "3.0".to_string(),
        metadata: OutputMetadata {
            debtmap_version: "0.12.0".to_string(),
            generated_at: "2024-01-01T00:00:00Z".to_string(),
            project_root: None,
            analysis_type: "unified".to_string(),
        },
        summary: DebtSummary {
            total_items: 0,
            total_debt_score: 0.0,
            debt_density: 0.0,
            total_loc: 0,
            by_type: TypeBreakdown {
                file: 0,
                function: 0,
            },
            by_category: std::collections::HashMap::new(),
            score_distribution: ScoreDistribution {
                critical: 0,
                high: 0,
                medium: 0,
                low: 0,
            },
            cohesion: None,
        },
        items: vec![],
    };

    // Serialize to JSON (what analyze writes)
    let json = serde_json::to_string_pretty(&unified_output).expect("Failed to serialize");

    // Deserialize as DebtmapJsonInput (what compare reads)
    let result: Result<DebtmapJsonInput, _> = serde_json::from_str(&json);
    assert!(
        result.is_ok(),
        "UnifiedOutput JSON should deserialize as DebtmapJsonInput: {:?}",
        result.err()
    );
}

// =============================================================================
// Property-based Tests
// =============================================================================

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn prop_is_critical_threshold_consistent(score in 0.0f64..20.0f64) {
            let result = is_critical(score);
            if score >= CRITICAL_SCORE_THRESHOLD {
                prop_assert!(result);
            } else {
                prop_assert!(!result);
            }
        }

        #[test]
        fn prop_is_score_unchanged_symmetric(before in 0.0f64..20.0f64, after in 0.0f64..20.0f64) {
            let result1 = is_score_unchanged(before, after);
            let result2 = is_score_unchanged(after, before);
            prop_assert_eq!(result1, result2, "is_score_unchanged should be symmetric");
        }

        #[test]
        fn prop_is_score_unchanged_reflexive(score in 0.0f64..20.0f64) {
            prop_assert!(is_score_unchanged(score, score), "score should be unchanged from itself");
        }

        #[test]
        fn prop_count_equals_length(count in 0usize..50) {
            let items: Vec<UnifiedDebtItemOutput> = (0..count)
                .map(|i| {
                    create_test_debt_item(
                        "src/test.rs",
                        &format!("fn_{}", i),
                        i * 10,
                        8.5 + (i as f64 % 5.0),
                    )
                })
                .collect();

            let before_output = create_test_output(items.clone());
            let after_output = create_test_output(items);

            let result = identify_unchanged_critical(&before_output, &after_output);

            prop_assert_eq!(result.count, result.items.len());
        }
    }
}
