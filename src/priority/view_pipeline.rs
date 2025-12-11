//! Pure transformation pipeline for view preparation.
//!
//! This module implements the "still water" - pure transformations
//! with no I/O. All functions are deterministic and testable.
//!
//! # Architecture
//!
//! The pipeline transforms `UnifiedAnalysis` into `PreparedDebtView` through
//! composable, pure stages:
//!
//! ```text
//! UnifiedAnalysis
//!        │
//!        ▼
//! ┌──────────────────┐
//! │  prepare_view()  │ ← ViewConfig, TierConfig (params, not env vars)
//! └──────────────────┘
//!        │
//!        ├─→ combine_items()       ← Merge function + file items
//!        ├─→ classify_all_tiers()  ← Assign recommendation tiers
//!        ├─→ filter_items()        ← Apply score/tier filters
//!        ├─→ sort_items()          ← Sort by criteria
//!        ├─→ limit_items()         ← Apply optional limit
//!        ├─→ compute_groups()      ← Create location groups
//!        └─→ calculate_summary()   ← Aggregate statistics
//!        │
//!        ▼
//! PreparedDebtView
//! ```
//!
//! # Purity Guarantee
//!
//! All stages are pure functions:
//! - No environment variable access
//! - No file I/O
//! - Deterministic results
//! - No side effects
//!
//! Configuration is passed as parameters, not read from environment.

use crate::priority::{
    classification::Severity,
    file_metrics::FileDebtItem,
    tiers::{classify_tier, RecommendationTier, TierConfig},
    unified_scorer::UnifiedDebtItem,
    view::{
        CategoryCounts, LocationGroup, PreparedDebtView, ScoreDistribution, SortCriteria,
        ViewConfig, ViewItem, ViewSummary,
    },
    DebtCategory, UnifiedAnalysis,
};
use std::collections::HashMap;
use std::path::PathBuf;

/// Prepares a canonical view from analysis results.
///
/// This is the **single entry point** for transforming `UnifiedAnalysis`
/// into `PreparedDebtView`. All output formats should use this function.
///
/// # Pure Function
///
/// This function has no side effects:
/// - No environment variable access
/// - No file I/O
/// - No logging or printing
/// - Deterministic: same inputs always produce same outputs
///
/// # Arguments
///
/// * `analysis` - The analysis results to transform
/// * `config` - View configuration (thresholds, limits, sorting)
/// * `tier_config` - Tier classification configuration
///
/// # Returns
///
/// A `PreparedDebtView` ready for consumption by any output format.
///
/// # Examples
///
/// ```ignore
/// let config = ViewConfig::default();
/// let tier_config = TierConfig::default();
/// let view = prepare_view(&analysis, &config, &tier_config);
///
/// // All output formats use the same view
/// render_tui(&view);
/// render_json(&view);
/// render_markdown(&view);
/// ```
pub fn prepare_view(
    analysis: &UnifiedAnalysis,
    config: &ViewConfig,
    tier_config: &TierConfig,
) -> PreparedDebtView {
    // Stage 1: Combine function and file items (pure)
    let combined = combine_items(&analysis.items, &analysis.file_items);
    let total_before_filter = combined.len();

    // Stage 2: Classify tiers (pure)
    let classified = classify_all_tiers(combined, tier_config);

    // Stage 3: Filter (pure)
    let (filtered, filter_stats) = filter_items(classified, config);

    // Stage 4: Sort (pure)
    let sorted = sort_items(filtered, config.sort_by);

    // Stage 5: Limit (pure)
    let limited = limit_items(sorted, config.limit);

    // Stage 6: Compute groups (pure)
    let groups = if config.compute_groups {
        compute_groups(&limited, config.sort_by)
    } else {
        vec![]
    };

    // Stage 7: Calculate summary (pure)
    let summary = calculate_summary(
        &limited,
        total_before_filter,
        filter_stats,
        analysis.total_lines_of_code,
        analysis.overall_coverage,
    );

    PreparedDebtView {
        items: limited,
        groups,
        summary,
        config: config.clone(),
    }
}

// ============================================================================
// STAGE 1: COMBINE ITEMS
// ============================================================================

/// Combines function and file items into unified ViewItems.
///
/// Pure function - operates on input slices, returns new Vec.
fn combine_items(
    function_items: &im::Vector<UnifiedDebtItem>,
    file_items: &im::Vector<FileDebtItem>,
) -> Vec<ViewItem> {
    let mut combined = Vec::with_capacity(function_items.len() + file_items.len());

    for item in function_items.iter() {
        combined.push(ViewItem::Function(Box::new(item.clone())));
    }

    for item in file_items.iter() {
        combined.push(ViewItem::File(Box::new(item.clone())));
    }

    combined
}

// ============================================================================
// STAGE 2: TIER CLASSIFICATION
// ============================================================================

/// Classifies tiers for all items.
///
/// For function items, uses the tier classification logic.
/// For file items, assigns T1CriticalArchitecture (god objects are always critical).
fn classify_all_tiers(items: Vec<ViewItem>, tier_config: &TierConfig) -> Vec<ViewItem> {
    items
        .into_iter()
        .map(|item| classify_item_tier(item, tier_config))
        .collect()
}

/// Classifies tier for a single item.
fn classify_item_tier(mut item: ViewItem, tier_config: &TierConfig) -> ViewItem {
    if let ViewItem::Function(ref mut func) = item {
        let tier = classify_tier(func, tier_config);
        func.tier = Some(tier);
    }
    // File items don't need tier classification (always T1)
    item
}

// ============================================================================
// STAGE 3: FILTERING
// ============================================================================

/// Statistics about filtered items.
#[derive(Debug, Default)]
struct FilterStats {
    filtered_by_score: usize,
    filtered_by_tier: usize,
}

/// Filters items based on configuration.
///
/// Pure function - returns new Vec and filter statistics.
fn filter_items(items: Vec<ViewItem>, config: &ViewConfig) -> (Vec<ViewItem>, FilterStats) {
    let mut stats = FilterStats::default();

    let filtered = items
        .into_iter()
        .filter(|item| {
            if !passes_score_threshold(item, config.min_score_threshold) {
                stats.filtered_by_score += 1;
                return false;
            }
            if !passes_tier_filter(item, config.exclude_t4_maintenance) {
                stats.filtered_by_tier += 1;
                return false;
            }
            true
        })
        .collect();

    (filtered, stats)
}

/// Checks if item passes score threshold.
fn passes_score_threshold(item: &ViewItem, threshold: f64) -> bool {
    item.score() >= threshold
}

/// Checks if item passes tier filter.
fn passes_tier_filter(item: &ViewItem, exclude_t4: bool) -> bool {
    if !exclude_t4 {
        return true;
    }

    !matches!(item.tier(), Some(RecommendationTier::T4Maintenance))
}

// ============================================================================
// STAGE 4: SORTING
// ============================================================================

/// Sorts items by the specified criteria.
///
/// Pure function - returns new sorted Vec.
fn sort_items(mut items: Vec<ViewItem>, criteria: SortCriteria) -> Vec<ViewItem> {
    match criteria {
        SortCriteria::Score => sort_by_score(&mut items),
        SortCriteria::Coverage => sort_by_coverage(&mut items),
        SortCriteria::Complexity => sort_by_complexity(&mut items),
        SortCriteria::FilePath => sort_by_file_path(&mut items),
        SortCriteria::FunctionName => sort_by_function_name(&mut items),
    }
    items
}

/// Sorts by score descending (highest first).
fn sort_by_score(items: &mut [ViewItem]) {
    items.sort_by(|a, b| {
        b.score()
            .partial_cmp(&a.score())
            .unwrap_or(std::cmp::Ordering::Equal)
    });
}

/// Sorts by coverage ascending (lowest first).
fn sort_by_coverage(items: &mut [ViewItem]) {
    items.sort_by(|a, b| {
        let cov_a = get_coverage(a);
        let cov_b = get_coverage(b);
        match (cov_a, cov_b) {
            (None, None) => std::cmp::Ordering::Equal,
            (None, Some(_)) => std::cmp::Ordering::Less, // No coverage is worst
            (Some(_), None) => std::cmp::Ordering::Greater,
            (Some(a_val), Some(b_val)) => a_val
                .partial_cmp(&b_val)
                .unwrap_or(std::cmp::Ordering::Equal),
        }
    });
}

/// Sorts by complexity descending (highest first).
fn sort_by_complexity(items: &mut [ViewItem]) {
    items.sort_by_key(|item| std::cmp::Reverse(get_complexity(item)));
}

/// Sorts by file path alphabetically.
fn sort_by_file_path(items: &mut [ViewItem]) {
    items.sort_by(|a, b| a.location().file.cmp(&b.location().file));
}

/// Sorts by function name alphabetically.
fn sort_by_function_name(items: &mut [ViewItem]) {
    items.sort_by(|a, b| {
        let loc_a = a.location();
        let loc_b = b.location();
        let name_a = loc_a.function.as_deref().unwrap_or("");
        let name_b = loc_b.function.as_deref().unwrap_or("");
        name_a.cmp(name_b)
    });
}

/// Extracts coverage from item (if available).
fn get_coverage(item: &ViewItem) -> Option<f64> {
    match item {
        ViewItem::Function(f) => f.transitive_coverage.as_ref().map(|c| c.direct),
        ViewItem::File(f) => Some(f.metrics.coverage_percent),
    }
}

/// Extracts complexity from item.
fn get_complexity(item: &ViewItem) -> u32 {
    match item {
        ViewItem::Function(f) => f.cognitive_complexity,
        ViewItem::File(f) => f.metrics.max_complexity,
    }
}

// ============================================================================
// STAGE 5: LIMITING
// ============================================================================

/// Limits items to specified count.
///
/// Pure function - returns new Vec with at most `limit` items.
fn limit_items(items: Vec<ViewItem>, limit: Option<usize>) -> Vec<ViewItem> {
    match limit {
        Some(n) => items.into_iter().take(n).collect(),
        None => items,
    }
}

// ============================================================================
// STAGE 6: GROUPING
// ============================================================================

/// Computes location groups from items.
///
/// Groups items by (file, function, line) and calculates combined scores.
fn compute_groups(items: &[ViewItem], sort_by: SortCriteria) -> Vec<LocationGroup> {
    let mut groups_map: HashMap<(PathBuf, String, usize), Vec<ViewItem>> = HashMap::new();

    for item in items {
        let loc = item.location();
        let key = loc.group_key();
        groups_map.entry(key).or_default().push(item.clone());
    }

    let mut groups: Vec<LocationGroup> = groups_map
        .into_values()
        .map(|group_items| {
            let location = group_items[0].location();
            LocationGroup::new(location, group_items)
        })
        .collect();

    // Sort groups by same criteria as items
    sort_groups(&mut groups, sort_by);

    groups
}

/// Sorts groups by criteria.
fn sort_groups(groups: &mut [LocationGroup], criteria: SortCriteria) {
    match criteria {
        SortCriteria::Score => {
            groups.sort_by(|a, b| {
                b.combined_score
                    .partial_cmp(&a.combined_score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        }
        SortCriteria::FilePath => {
            groups.sort_by(|a, b| a.location.file.cmp(&b.location.file));
        }
        SortCriteria::FunctionName => {
            groups.sort_by(|a, b| {
                let name_a = a.location.function.as_deref().unwrap_or("");
                let name_b = b.location.function.as_deref().unwrap_or("");
                name_a.cmp(name_b)
            });
        }
        // For coverage/complexity, use combined score as fallback
        _ => {
            groups.sort_by(|a, b| {
                b.combined_score
                    .partial_cmp(&a.combined_score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        }
    }
}

// ============================================================================
// STAGE 7: SUMMARY CALCULATION
// ============================================================================

/// Calculates summary statistics from items.
///
/// Pure function - aggregates data from items list.
fn calculate_summary(
    items: &[ViewItem],
    total_before_filter: usize,
    filter_stats: FilterStats,
    total_loc: usize,
    overall_coverage: Option<f64>,
) -> ViewSummary {
    let total_debt_score: f64 = items.iter().map(|i| i.score()).sum();

    let score_distribution = calculate_score_distribution(items);
    let category_counts = calculate_category_counts(items);

    let debt_density = if total_loc > 0 {
        (total_debt_score / total_loc as f64) * 1000.0
    } else {
        0.0
    };

    ViewSummary {
        total_items_before_filter: total_before_filter,
        total_items_after_filter: items.len(),
        filtered_by_tier: filter_stats.filtered_by_tier,
        filtered_by_score: filter_stats.filtered_by_score,
        total_debt_score,
        score_distribution,
        category_counts,
        total_lines_of_code: total_loc,
        debt_density,
        overall_coverage,
    }
}

/// Calculates score distribution by severity.
fn calculate_score_distribution(items: &[ViewItem]) -> ScoreDistribution {
    let mut dist = ScoreDistribution::default();

    for item in items {
        match item.severity() {
            Severity::Critical => dist.critical += 1,
            Severity::High => dist.high += 1,
            Severity::Medium => dist.medium += 1,
            Severity::Low => dist.low += 1,
        }
    }

    dist
}

/// Calculates category counts.
fn calculate_category_counts(items: &[ViewItem]) -> CategoryCounts {
    let mut counts = CategoryCounts::default();

    for item in items {
        match item.category() {
            DebtCategory::Architecture => counts.architecture += 1,
            DebtCategory::Testing => counts.testing += 1,
            DebtCategory::Performance => counts.performance += 1,
            DebtCategory::CodeQuality => counts.code_quality += 1,
        }
    }

    counts
}

// ============================================================================
// CONVENIENCE FUNCTIONS
// ============================================================================

/// Creates a view with default configuration.
///
/// Useful for quick usage without custom configuration.
pub fn prepare_view_default(analysis: &UnifiedAnalysis) -> PreparedDebtView {
    prepare_view(analysis, &ViewConfig::default(), &TierConfig::default())
}

/// Creates a view with TUI-optimized configuration.
///
/// Mirrors current TUI behavior:
/// - No score threshold (show all)
/// - No T4 filtering
/// - Grouping enabled
pub fn prepare_view_for_tui(analysis: &UnifiedAnalysis) -> PreparedDebtView {
    let config = ViewConfig {
        min_score_threshold: 0.0,
        exclude_t4_maintenance: false,
        compute_groups: true,
        ..Default::default()
    };
    prepare_view(analysis, &config, &TierConfig::default())
}

/// Creates a view with terminal-optimized configuration.
///
/// Mirrors current --no-tui behavior:
/// - Score threshold 3.0
/// - T4 filtering enabled
/// - No grouping
pub fn prepare_view_for_terminal(
    analysis: &UnifiedAnalysis,
    limit: Option<usize>,
) -> PreparedDebtView {
    let config = ViewConfig {
        min_score_threshold: 3.0,
        exclude_t4_maintenance: true,
        limit,
        compute_groups: false,
        ..Default::default()
    };
    prepare_view(analysis, &config, &TierConfig::default())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::priority::unified_scorer::Location;
    use crate::priority::{
        file_metrics::{FileDebtItem, FileDebtMetrics, FileImpact},
        score_types::Score0To100,
        semantic_classifier::FunctionRole,
        ActionableRecommendation, DebtType, ImpactMetrics, UnifiedScore,
    };

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

        let (filtered, stats) = filter_items(items, &config);

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

        let (filtered, stats) = filter_items(items, &config);

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

        let (filtered, stats) = filter_items(items, &config);

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

        let sorted = sort_items(items, SortCriteria::Score);

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

        let sorted = sort_items(items, SortCriteria::FilePath);

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

        let sorted = sort_items(items, SortCriteria::FunctionName);

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

        let sorted = sort_items(items, SortCriteria::Score);

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

        let groups = compute_groups(&items, SortCriteria::Score);

        assert_eq!(groups.len(), 2);
        // Groups are sorted by combined score descending
        // "other.rs" has 50.0, "file.rs" has 30.0 + 20.0 = 50.0
        // Both have 50.0, so order may vary - check totals
        let total_combined: f64 = groups.iter().map(|g| g.combined_score).sum();
        assert_eq!(total_combined, 100.0);
    }

    #[test]
    fn test_compute_groups_empty() {
        let groups = compute_groups(&[], SortCriteria::Score);
        assert!(groups.is_empty());
    }

    #[test]
    fn test_compute_groups_single_item() {
        let items = vec![create_view_item_at("file.rs", "func", 10, 50.0)];

        let groups = compute_groups(&items, SortCriteria::Score);

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
        let filter_stats = FilterStats::default();

        let summary = calculate_summary(&items, 10, filter_stats, 1000, Some(0.5));

        assert_eq!(summary.total_items_before_filter, 10);
        assert_eq!(summary.total_items_after_filter, 4);
        assert_eq!(summary.total_debt_score, 205.0);
        assert_eq!(summary.total_lines_of_code, 1000);
        assert_eq!(summary.overall_coverage, Some(0.5));
    }

    #[test]
    fn test_calculate_score_distribution() {
        let items = vec![
            create_view_item(85.0), // Critical (>= 70)
            create_view_item(75.0), // Critical
            create_view_item(60.0), // High (>= 50)
            create_view_item(40.0), // Medium (>= 30)
            create_view_item(35.0), // Medium
            create_view_item(20.0), // Low (< 30)
        ];

        let dist = calculate_score_distribution(&items);

        assert_eq!(dist.critical, 2);
        assert_eq!(dist.high, 1);
        assert_eq!(dist.medium, 2);
        assert_eq!(dist.low, 1);
    }

    #[test]
    fn test_calculate_category_counts() {
        // Note: create_view_item creates TestingGap which maps to Testing category
        let items = vec![
            create_view_item(50.0), // Testing
            create_view_item(40.0), // Testing
            create_view_item(30.0), // Testing
        ];

        let counts = calculate_category_counts(&items);

        assert_eq!(counts.testing, 3);
        assert_eq!(counts.architecture, 0);
        assert_eq!(counts.performance, 0);
        assert_eq!(counts.code_quality, 0);
    }

    #[test]
    fn test_debt_density_calculation() {
        let items = vec![create_view_item(100.0)];
        let filter_stats = FilterStats::default();

        let summary = calculate_summary(&items, 1, filter_stats, 1000, None);

        // debt_density = (total_score / loc) * 1000 = (100 / 1000) * 1000 = 100
        assert_eq!(summary.debt_density, 100.0);
    }

    #[test]
    fn test_debt_density_zero_loc() {
        let items = vec![create_view_item(100.0)];
        let filter_stats = FilterStats::default();

        let summary = calculate_summary(&items, 1, filter_stats, 0, None);

        assert_eq!(summary.debt_density, 0.0);
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

        assert!(passes_score_threshold(&item, 3.0));
        assert!(passes_score_threshold(&item, 5.0));
        assert!(!passes_score_threshold(&item, 6.0));
    }

    #[test]
    fn test_passes_tier_filter() {
        let t1_item = create_view_item_with_tier(50.0, RecommendationTier::T1CriticalArchitecture);
        let t4_item = create_view_item_with_tier(50.0, RecommendationTier::T4Maintenance);

        // When not excluding T4
        assert!(passes_tier_filter(&t1_item, false));
        assert!(passes_tier_filter(&t4_item, false));

        // When excluding T4
        assert!(passes_tier_filter(&t1_item, true));
        assert!(!passes_tier_filter(&t4_item, true));
    }

    #[test]
    fn test_get_coverage_function() {
        let item = create_view_item(50.0);
        // Our test function doesn't have transitive_coverage set
        assert!(get_coverage(&item).is_none());
    }

    #[test]
    fn test_get_coverage_file() {
        let file_item = ViewItem::File(Box::new(create_test_file_item(50.0)));
        let coverage = get_coverage(&file_item);
        assert!(coverage.is_some());
        assert_eq!(coverage.unwrap(), 0.3); // Set in create_test_file_item
    }

    #[test]
    fn test_get_complexity() {
        let func_item = create_view_item(50.0);
        let file_item = ViewItem::File(Box::new(create_test_file_item(50.0)));

        assert_eq!(get_complexity(&func_item), 15); // cognitive_complexity from helper
        assert_eq!(get_complexity(&file_item), 25); // max_complexity from helper
    }
}
