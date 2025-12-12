//! Tests for the view pipeline module.

use super::*;
use crate::priority::unified_scorer::Location;
use crate::priority::{
    file_metrics::{FileDebtItem, FileDebtMetrics, FileImpact},
    score_types::Score0To100,
    semantic_classifier::FunctionRole,
    tiers::RecommendationTier,
    view::{SortCriteria, ViewConfig},
    ActionableRecommendation, DebtType, ImpactMetrics, UnifiedScore,
};
use std::path::PathBuf;

// ========================================================================
// TEST HELPERS
// ========================================================================

fn create_test_function_item(score: f64) -> UnifiedDebtItem {
    create_test_function_item_at("test.rs", "test_fn", 10, score)
}

fn create_test_function_item_at(
    file: &str,
    func: &str,
    line: usize,
    score: f64,
) -> UnifiedDebtItem {
    UnifiedDebtItem {
        location: Location {
            file: file.into(),
            function: func.into(),
            line,
        },
        debt_type: DebtType::TestingGap {
            coverage: 0.0,
            cyclomatic: 10,
            cognitive: 15,
        },
        unified_score: UnifiedScore {
            complexity_factor: 5.0,
            coverage_factor: 5.0,
            dependency_factor: 2.0,
            role_multiplier: 1.0,
            final_score: Score0To100::new(score),
            base_score: None,
            exponential_factor: None,
            risk_boost: None,
            pre_adjustment_score: None,
            adjustment_applied: None,
            purity_factor: None,
            refactorability_factor: None,
            pattern_factor: None,
        },
        function_role: FunctionRole::PureLogic,
        recommendation: ActionableRecommendation {
            primary_action: "Add tests".into(),
            rationale: "Improve coverage".into(),
            implementation_steps: vec![],
            related_items: vec![],
            steps: None,
            estimated_effort_hours: None,
        },
        expected_impact: ImpactMetrics {
            risk_reduction: 0.0,
            complexity_reduction: 0.0,
            coverage_improvement: 20.0,
            lines_reduction: 0,
        },
        transitive_coverage: None,
        file_context: None,
        upstream_dependencies: 3,
        downstream_dependencies: 2,
        upstream_callers: vec![],
        downstream_callees: vec![],
        nesting_depth: 2,
        function_length: 50,
        cyclomatic_complexity: 10,
        cognitive_complexity: 15,
        entropy_details: None,
        entropy_adjusted_cognitive: None,
        entropy_dampening_factor: None,
        is_pure: Some(false),
        purity_confidence: Some(0.8),
        purity_level: None,
        god_object_indicators: None,
        tier: None,
        function_context: None,
        context_confidence: None,
        contextual_recommendation: None,
        pattern_analysis: None,
        context_multiplier: None,
        context_type: None,
        language_specific: None,
        detected_pattern: None,
        contextual_risk: None,
        file_line_count: None,
        responsibility_category: None,
        error_swallowing_count: None,
        error_swallowing_patterns: None,
    }
}

fn create_test_file_item(score: f64) -> FileDebtItem {
    FileDebtItem {
        metrics: FileDebtMetrics {
            path: "test_file.rs".into(),
            total_lines: 500,
            function_count: 20,
            class_count: 1,
            avg_complexity: 8.0,
            max_complexity: 25,
            total_complexity: 160,
            coverage_percent: 0.3,
            uncovered_lines: 350,
            god_object_analysis: None,
            function_scores: vec![5.0; 20],
            god_object_type: None,
            file_type: None,
            ..Default::default()
        },
        score,
        priority_rank: 1,
        recommendation: "Split into multiple modules".into(),
        impact: FileImpact {
            complexity_reduction: 50.0,
            maintainability_improvement: 30.0,
            test_effort: 20.0,
        },
    }
}

fn create_view_item(score: f64) -> ViewItem {
    ViewItem::Function(Box::new(create_test_function_item(score)))
}

fn create_view_item_with_tier(score: f64, tier: RecommendationTier) -> ViewItem {
    let mut item = create_test_function_item(score);
    item.tier = Some(tier);
    ViewItem::Function(Box::new(item))
}

fn create_view_item_at(file: &str, func: &str, line: usize, score: f64) -> ViewItem {
    ViewItem::Function(Box::new(create_test_function_item_at(
        file, func, line, score,
    )))
}

// ========================================================================
// STAGE 1: COMBINE ITEMS TESTS
// ========================================================================

#[test]
fn test_combine_items_preserves_all() {
    let functions: im::Vector<UnifiedDebtItem> = (0..3)
        .map(|i| create_test_function_item(50.0 + i as f64))
        .collect();
    let files: im::Vector<FileDebtItem> = (0..2)
        .map(|i| create_test_file_item(60.0 + i as f64))
        .collect();

    let combined = combine_items(&functions, &files);

    assert_eq!(combined.len(), 5);
}

#[test]
fn test_combine_items_empty() {
    let combined = combine_items(&im::Vector::new(), &im::Vector::new());
    assert!(combined.is_empty());
}

#[test]
fn test_combine_items_only_functions() {
    let functions: im::Vector<UnifiedDebtItem> = (0..3)
        .map(|i| create_test_function_item(50.0 + i as f64))
        .collect();
    let files: im::Vector<FileDebtItem> = im::Vector::new();

    let combined = combine_items(&functions, &files);

    assert_eq!(combined.len(), 3);
    assert!(combined
        .iter()
        .all(|item| matches!(item, ViewItem::Function(_))));
}

#[test]
fn test_combine_items_only_files() {
    let functions: im::Vector<UnifiedDebtItem> = im::Vector::new();
    let files: im::Vector<FileDebtItem> = (0..2)
        .map(|i| create_test_file_item(60.0 + i as f64))
        .collect();

    let combined = combine_items(&functions, &files);

    assert_eq!(combined.len(), 2);
    assert!(combined
        .iter()
        .all(|item| matches!(item, ViewItem::File(_))));
}

// ========================================================================
// STAGE 3: FILTER TESTS
// ========================================================================

#[test]
fn test_filter_by_score_threshold() {
    let items = vec![
        create_view_item(10.0), // Above threshold
        create_view_item(2.0),  // Below threshold
        create_view_item(5.0),  // Above threshold
    ];
    let config = ViewConfig {
        min_score_threshold: 3.0,
        ..Default::default()
    };

    let (filtered, stats) = filter::filter_items(items, &config);

    assert_eq!(filtered.len(), 2);
    assert_eq!(stats.filtered_by_score, 1);
}

#[test]
fn test_filter_by_tier() {
    let items = vec![
        create_view_item_with_tier(50.0, RecommendationTier::T1CriticalArchitecture),
        create_view_item_with_tier(30.0, RecommendationTier::T4Maintenance),
        create_view_item_with_tier(40.0, RecommendationTier::T2ComplexUntested),
    ];
    let config = ViewConfig {
        min_score_threshold: 0.0,
        exclude_t4_maintenance: true,
        ..Default::default()
    };

    let (filtered, stats) = filter::filter_items(items, &config);

    assert_eq!(filtered.len(), 2);
    assert_eq!(stats.filtered_by_tier, 1);
}

#[test]
fn test_filter_no_exclusions() {
    let items = vec![
        create_view_item_with_tier(50.0, RecommendationTier::T1CriticalArchitecture),
        create_view_item_with_tier(30.0, RecommendationTier::T4Maintenance),
    ];
    let config = ViewConfig {
        min_score_threshold: 0.0,
        exclude_t4_maintenance: false,
        ..Default::default()
    };

    let (filtered, stats) = filter::filter_items(items, &config);

    assert_eq!(filtered.len(), 2);
    assert_eq!(stats.filtered_by_tier, 0);
    assert_eq!(stats.filtered_by_score, 0);
}

// ========================================================================
// STAGE 4: SORT TESTS
// ========================================================================

#[test]
fn test_sort_by_score_descending() {
    let items = vec![
        create_view_item(30.0),
        create_view_item(50.0),
        create_view_item(10.0),
    ];

    let sorted = sort::sort_items(items, SortCriteria::Score);

    assert_eq!(sorted[0].score(), 50.0);
    assert_eq!(sorted[1].score(), 30.0);
    assert_eq!(sorted[2].score(), 10.0);
}

#[test]
fn test_sort_by_file_path() {
    let items = vec![
        create_view_item_at("z_file.rs", "fn1", 10, 50.0),
        create_view_item_at("a_file.rs", "fn2", 10, 30.0),
        create_view_item_at("m_file.rs", "fn3", 10, 40.0),
    ];

    let sorted = sort::sort_items(items, SortCriteria::FilePath);

    assert_eq!(sorted[0].location().file, PathBuf::from("a_file.rs"));
    assert_eq!(sorted[1].location().file, PathBuf::from("m_file.rs"));
    assert_eq!(sorted[2].location().file, PathBuf::from("z_file.rs"));
}

#[test]
fn test_sort_by_function_name() {
    let items = vec![
        create_view_item_at("file.rs", "zebra", 10, 50.0),
        create_view_item_at("file.rs", "alpha", 20, 30.0),
        create_view_item_at("file.rs", "monkey", 30, 40.0),
    ];

    let sorted = sort::sort_items(items, SortCriteria::FunctionName);

    assert_eq!(sorted[0].location().function, Some("alpha".to_string()));
    assert_eq!(sorted[1].location().function, Some("monkey".to_string()));
    assert_eq!(sorted[2].location().function, Some("zebra".to_string()));
}

#[test]
fn test_sort_preserves_count() {
    let items = vec![
        create_view_item(30.0),
        create_view_item(50.0),
        create_view_item(10.0),
    ];
    let original_count = items.len();

    let sorted = sort::sort_items(items, SortCriteria::Score);

    assert_eq!(sorted.len(), original_count);
}

// ========================================================================
// STAGE 5: LIMIT TESTS
// ========================================================================

#[test]
fn test_limit_items() {
    let items = vec![
        create_view_item(50.0),
        create_view_item(40.0),
        create_view_item(30.0),
        create_view_item(20.0),
    ];

    let limited = limit_items(items, Some(2));

    assert_eq!(limited.len(), 2);
}

#[test]
fn test_limit_none_returns_all() {
    let items = vec![create_view_item(50.0), create_view_item(40.0)];

    let limited = limit_items(items, None);

    assert_eq!(limited.len(), 2);
}

#[test]
fn test_limit_greater_than_count() {
    let items = vec![create_view_item(50.0), create_view_item(40.0)];

    let limited = limit_items(items, Some(10));

    assert_eq!(limited.len(), 2);
}

#[test]
fn test_limit_zero() {
    let items = vec![create_view_item(50.0), create_view_item(40.0)];

    let limited = limit_items(items, Some(0));

    assert!(limited.is_empty());
}

// ========================================================================
// STAGE 6: GROUPING TESTS
// ========================================================================

#[test]
fn test_compute_groups_combines_same_location() {
    let items = vec![
        create_view_item_at("file.rs", "func", 10, 30.0),
        create_view_item_at("file.rs", "func", 10, 20.0),
        create_view_item_at("other.rs", "func", 10, 50.0),
    ];

    let groups = group::compute_groups(&items, SortCriteria::Score);

    assert_eq!(groups.len(), 2);
    // Groups are sorted by combined score descending
    // "other.rs" has 50.0, "file.rs" has 30.0 + 20.0 = 50.0
    // Both have 50.0, so order may vary - check totals
    let total_combined: f64 = groups.iter().map(|g| g.combined_score).sum();
    assert_eq!(total_combined, 100.0);
}

#[test]
fn test_compute_groups_empty() {
    let groups = group::compute_groups(&[], SortCriteria::Score);
    assert!(groups.is_empty());
}

#[test]
fn test_compute_groups_single_item() {
    let items = vec![create_view_item_at("file.rs", "func", 10, 50.0)];

    let groups = group::compute_groups(&items, SortCriteria::Score);

    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].combined_score, 50.0);
    assert_eq!(groups[0].item_count, 1);
}

// ========================================================================
// STAGE 7: SUMMARY TESTS
// ========================================================================

#[test]
fn test_calculate_summary_scores() {
    let items = vec![
        create_view_item(85.0), // Critical
        create_view_item(60.0), // High
        create_view_item(40.0), // Medium
        create_view_item(20.0), // Low
    ];
    let filter_stats = filter::FilterStats::default();

    let sum = summary::calculate_summary(&items, 10, filter_stats, 1000, Some(0.5));

    assert_eq!(sum.total_items_before_filter, 10);
    assert_eq!(sum.total_items_after_filter, 4);
    assert_eq!(sum.total_debt_score, 205.0);
    assert_eq!(sum.total_lines_of_code, 1000);
    assert_eq!(sum.overall_coverage, Some(0.5));
}

#[test]
fn test_debt_density_calculation() {
    let items = vec![create_view_item(100.0)];
    let filter_stats = filter::FilterStats::default();

    let sum = summary::calculate_summary(&items, 1, filter_stats, 1000, None);

    // debt_density = (total_score / loc) * 1000 = (100 / 1000) * 1000 = 100
    assert_eq!(sum.debt_density, 100.0);
}

#[test]
fn test_debt_density_zero_loc() {
    let items = vec![create_view_item(100.0)];
    let filter_stats = filter::FilterStats::default();

    let sum = summary::calculate_summary(&items, 1, filter_stats, 0, None);

    assert_eq!(sum.debt_density, 0.0);
}

// ========================================================================
// FULL PIPELINE TESTS
// ========================================================================

#[test]
fn test_prepare_view_deterministic() {
    use crate::priority::call_graph::CallGraph;

    let call_graph = CallGraph::new();
    let mut analysis = UnifiedAnalysis::new(call_graph);

    // Add some items
    analysis.items = (0..5)
        .map(|i| create_test_function_item(50.0 + i as f64 * 10.0))
        .collect();

    let config = ViewConfig::default();
    let tier_config = TierConfig::default();

    let view1 = prepare_view(&analysis, &config, &tier_config);
    let view2 = prepare_view(&analysis, &config, &tier_config);

    assert_eq!(view1.items.len(), view2.items.len());
    for (a, b) in view1.items.iter().zip(view2.items.iter()) {
        assert_eq!(a.score(), b.score());
    }
}

#[test]
fn test_prepare_view_for_tui() {
    use crate::priority::call_graph::CallGraph;

    let call_graph = CallGraph::new();
    let mut analysis = UnifiedAnalysis::new(call_graph);
    analysis.items = (0..3)
        .map(|i| create_test_function_item(50.0 + i as f64 * 10.0))
        .collect();

    let view = prepare_view_for_tui(&analysis);

    // TUI config: no score threshold, no T4 filtering, grouping enabled
    assert!(view.summary.filtered_by_score == 0);
    assert!(view.config.compute_groups);
}

#[test]
fn test_prepare_view_for_terminal() {
    use crate::priority::call_graph::CallGraph;

    let call_graph = CallGraph::new();
    let mut analysis = UnifiedAnalysis::new(call_graph);
    analysis.items = (0..10)
        .map(|i| create_test_function_item(1.0 + i as f64 * 2.0))
        .collect();

    let view = prepare_view_for_terminal(&analysis, Some(5));

    // Terminal config: score threshold 3.0, T4 filtering, limit 5, no grouping
    assert!(view.items.len() <= 5);
    assert!(!view.config.compute_groups);
    assert!(view.groups.is_empty());
}

#[test]
fn test_prepare_view_empty_analysis() {
    use crate::priority::call_graph::CallGraph;

    let call_graph = CallGraph::new();
    let analysis = UnifiedAnalysis::new(call_graph);

    let view = prepare_view_default(&analysis);

    assert!(view.is_empty());
    assert_eq!(view.summary.total_items_before_filter, 0);
    assert_eq!(view.summary.total_items_after_filter, 0);
}

// ========================================================================
// PREDICATE FUNCTION TESTS
// ========================================================================

#[test]
fn test_passes_score_threshold() {
    let item = create_view_item(5.0);

    assert!(filter::passes_score_threshold(&item, 3.0));
    assert!(filter::passes_score_threshold(&item, 5.0));
    assert!(!filter::passes_score_threshold(&item, 6.0));
}

#[test]
fn test_passes_tier_filter() {
    let t1_item = create_view_item_with_tier(50.0, RecommendationTier::T1CriticalArchitecture);
    let t4_item = create_view_item_with_tier(50.0, RecommendationTier::T4Maintenance);

    // When not excluding T4
    assert!(filter::passes_tier_filter(&t1_item, false));
    assert!(filter::passes_tier_filter(&t4_item, false));

    // When excluding T4
    assert!(filter::passes_tier_filter(&t1_item, true));
    assert!(!filter::passes_tier_filter(&t4_item, true));
}

#[test]
fn test_get_coverage_function() {
    let item = create_view_item(50.0);
    // Our test function doesn't have transitive_coverage set
    assert!(sort::get_coverage(&item).is_none());
}

#[test]
fn test_get_coverage_file() {
    let file_item = ViewItem::File(Box::new(create_test_file_item(50.0)));
    let coverage = sort::get_coverage(&file_item);
    assert!(coverage.is_some());
    assert_eq!(coverage.unwrap(), 0.3); // Set in create_test_file_item
}

#[test]
fn test_get_complexity() {
    let func_item = create_view_item(50.0);
    let file_item = ViewItem::File(Box::new(create_test_file_item(50.0)));

    assert_eq!(sort::get_complexity(&func_item), 15); // cognitive_complexity from helper
    assert_eq!(sort::get_complexity(&file_item), 25); // max_complexity from helper
}
