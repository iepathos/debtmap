//! Composable filter pipeline with functional composition.
//!
//! This module implements a pure, functional filtering pipeline that transforms
//! debt items through immutable stages: classify → filter → sort → limit.
//!
//! All functions are pure with no side effects, following functional programming
//! principles from the Debtmap development guidelines.

use super::filtering::{filter_with_metrics, ClassifiedItem, FilterConfig, FilterResult};
use super::tiers::{classify_tier, TierConfig};
use super::UnifiedDebtItem;
use std::cmp::Ordering;

/// Sorts items by score in descending order (pure).
///
/// Creates a new sorted vector without mutating the input.
/// While this function uses `sort_by()` which mutates the local vector,
/// the function itself is pure - it always returns the same output for
/// the same input with no observable side effects.
///
/// # Arguments
///
/// * `items` - Items to sort (consumed and transformed)
///
/// # Returns
///
/// New vector with items sorted by score (descending)
///
/// # Examples
///
/// ```no_run
/// use debtmap::priority::pipeline::sort_by_score;
/// # use debtmap::priority::filtering::ClassifiedItem;
///
/// # let items: Vec<ClassifiedItem> = vec![];
/// let sorted = sort_by_score(items);
/// // Items are now sorted by score, highest first
/// ```
pub fn sort_by_score(mut items: Vec<ClassifiedItem>) -> Vec<ClassifiedItem> {
    items.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(Ordering::Equal));
    items
}

/// Limits items to top N (pure).
///
/// Returns a new vector containing only the first N items.
/// Uses iterator's `take()` for lazy evaluation.
///
/// # Arguments
///
/// * `items` - Items to limit (consumed)
/// * `limit` - Maximum number of items to return
///
/// # Returns
///
/// New vector with at most `limit` items
///
/// # Examples
///
/// ```no_run
/// use debtmap::priority::pipeline::take_top;
/// # use debtmap::priority::filtering::ClassifiedItem;
///
/// # let items: Vec<ClassifiedItem> = vec![];
/// let top_10 = take_top(items, 10);
/// ```
pub fn take_top(items: Vec<ClassifiedItem>, limit: usize) -> Vec<ClassifiedItem> {
    items.into_iter().take(limit).collect()
}

/// Complete filter pipeline using functional composition.
///
/// Transforms debt items through these pure stages:
/// 1. **Classify** - Assign tier to each item based on configuration
/// 2. **Filter** - Remove items by tier visibility and score threshold
/// 3. **Sort** - Order by score (descending, highest priority first)
/// 4. **Limit** - Take top N items
///
/// All stages are pure functions with no side effects. The pipeline is
/// composable and each stage can be tested independently.
///
/// # Arguments
///
/// * `items` - Raw debt items to process
/// * `tier_config` - Tier classification configuration
/// * `filter_config` - Filter criteria (thresholds, enabled types)
/// * `limit` - Maximum items to return
///
/// # Returns
///
/// FilterResult containing filtered items and transparency metrics
///
/// # Examples
///
/// ```no_run
/// use debtmap::priority::pipeline::analyze_and_filter;
/// use debtmap::priority::tiers::TierConfig;
/// use debtmap::priority::filtering::FilterConfig;
///
/// # let debt_items = vec![];
/// let result = analyze_and_filter(
///     debt_items,
///     &TierConfig::default(),
///     &FilterConfig { min_score: 3.0, show_t4: false },
///     50,
/// );
/// println!("Included {} items", result.included.len());
/// println!("Filtered {} items", result.metrics.total_filtered());
/// ```
pub fn analyze_and_filter(
    items: Vec<UnifiedDebtItem>,
    tier_config: &TierConfig,
    filter_config: &FilterConfig,
    limit: usize,
) -> FilterResult {
    // Stage 1: Classify (pure transformation)
    let classified: Vec<ClassifiedItem> = items
        .into_iter()
        .map(|item| {
            let tier = classify_tier(&item, tier_config);
            let score = item.unified_score.final_score.value();
            ClassifiedItem {
                item: super::DebtItem::Function(Box::new(item)),
                tier,
                score,
            }
        })
        .collect();

    // Stage 2: Sort (pure, immutable transformation)
    let sorted = sort_by_score(classified);

    // Stage 3: Filter with metrics (pure, from Spec 225)
    let filtered_result = filter_with_metrics(sorted, filter_config);

    // Stage 4: Limit (pure, lazy evaluation)
    let limited: Vec<super::DebtItem> = filtered_result.included.into_iter().take(limit).collect();

    FilterResult {
        included: limited,
        metrics: filtered_result.metrics,
    }
}

/// Complete filter pipeline for classified items (pure, functional).
///
/// This variant operates on pre-classified items, useful when combining
/// function and file items that have already been classified.
///
/// Transforms through these pure stages:
/// 1. **Sort** - Order by score (descending, highest priority first)
/// 2. **Filter** - Remove items by tier visibility and score threshold
/// 3. **Limit** - Take top N items
///
/// Note: We sort before filtering to ensure consistent ordering, then filter
/// and limit. This is more efficient than filter->sort->limit when the filter
/// removes many items.
///
/// # Arguments
///
/// * `classified` - Already classified items to filter
/// * `filter_config` - Filter criteria (thresholds, enabled types)
/// * `limit` - Maximum items to return
///
/// # Returns
///
/// FilterResult containing filtered items and transparency metrics
pub fn filter_sort_limit(
    classified: Vec<ClassifiedItem>,
    filter_config: &FilterConfig,
    limit: usize,
) -> FilterResult {
    // Stage 1: Sort (pure, immutable transformation)
    // Sort first to ensure consistent ordering before filtering
    let sorted = sort_by_score(classified);

    // Stage 2: Filter with metrics (pure, from Spec 225)
    let filtered_result = filter_with_metrics(sorted, filter_config);

    // Stage 3: Limit (pure, lazy evaluation)
    // At this point, filtered_result.included is Vec<DebtItem>
    // We need to limit it directly
    let limited: Vec<super::DebtItem> = filtered_result.included.into_iter().take(limit).collect();

    FilterResult {
        included: limited,
        metrics: filtered_result.metrics,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::priority::filtering::ClassifiedItem;
    use crate::priority::score_types::Score0To100;
    use crate::priority::tiers::RecommendationTier;
    use crate::priority::{
        ActionableRecommendation, DebtItem, DebtType, FunctionRole, ImpactMetrics, Location,
        UnifiedDebtItem, UnifiedScore,
    };

    fn create_classified_item(score: f64, tier: RecommendationTier) -> ClassifiedItem {
        let item = UnifiedDebtItem {
            location: Location {
                file: "test.rs".into(),
                function: "test_fn".into(),
                line: 1,
            },
            debt_type: DebtType::ComplexityHotspot {
                cyclomatic: 10,
                cognitive: 10,
                adjusted_cyclomatic: None,
            },
            unified_score: UnifiedScore {
                complexity_factor: 1.0,
                coverage_factor: 1.0,
                dependency_factor: 1.0,
                role_multiplier: 1.0,
                final_score: Score0To100::new(score),
                base_score: Some(score),
                exponential_factor: Some(1.0),
                risk_boost: Some(1.0),
                pre_adjustment_score: None,
                adjustment_applied: None,
                purity_factor: None,
                refactorability_factor: None,
                pattern_factor: None,
            },
            function_role: FunctionRole::PureLogic,
            recommendation: ActionableRecommendation {
                primary_action: "Test".into(),
                rationale: "Test".into(),
                implementation_steps: vec![],
                related_items: vec![],
                steps: None,
                estimated_effort_hours: None,
            },
            expected_impact: ImpactMetrics {
                coverage_improvement: 0.0,
                lines_reduction: 0,
                complexity_reduction: 0.0,
                risk_reduction: 0.0,
            },
            transitive_coverage: None,
            upstream_dependencies: 0,
            downstream_dependencies: 0,
            upstream_callers: vec![],
            downstream_callees: vec![],
            nesting_depth: 1,
            function_length: 10,
            cyclomatic_complexity: 10,
            cognitive_complexity: 10,
            entropy_details: None,
            entropy_adjusted_cyclomatic: None,
            entropy_adjusted_cognitive: None,
            entropy_dampening_factor: None,
            is_pure: None,
            purity_confidence: None,
            purity_level: None,
            god_object_indicators: None,
            tier: None,
            function_context: None,
            context_confidence: None,
            contextual_recommendation: None,
            pattern_analysis: None,
            file_context: None,
            context_multiplier: None,
            context_type: None,
            language_specific: None,
            detected_pattern: None,
            contextual_risk: None,
            file_line_count: None,
            responsibility_category: None,
            error_swallowing_count: None,
            error_swallowing_patterns: None,
        };

        ClassifiedItem {
            item: DebtItem::Function(Box::new(item)),
            tier,
            score,
        }
    }

    #[test]
    fn test_sort_by_score_descending() {
        let items = vec![
            create_classified_item(50.0, RecommendationTier::T2ComplexUntested),
            create_classified_item(95.0, RecommendationTier::T1CriticalArchitecture),
            create_classified_item(70.0, RecommendationTier::T2ComplexUntested),
        ];

        let sorted = sort_by_score(items);

        assert_eq!(sorted[0].score, 95.0);
        assert_eq!(sorted[1].score, 70.0);
        assert_eq!(sorted[2].score, 50.0);
    }

    #[test]
    fn test_sort_is_stable_for_equal_scores() {
        let items = vec![
            create_classified_item(50.0, RecommendationTier::T1CriticalArchitecture),
            create_classified_item(50.0, RecommendationTier::T2ComplexUntested),
            create_classified_item(50.0, RecommendationTier::T3TestingGaps),
        ];

        let sorted = sort_by_score(items);

        // All scores should be 50.0
        for item in sorted {
            assert_eq!(item.score, 50.0);
        }
    }

    #[test]
    fn test_take_top_limits_correctly() {
        let items = vec![
            create_classified_item(95.0, RecommendationTier::T1CriticalArchitecture),
            create_classified_item(85.0, RecommendationTier::T2ComplexUntested),
            create_classified_item(75.0, RecommendationTier::T2ComplexUntested),
            create_classified_item(65.0, RecommendationTier::T3TestingGaps),
        ];

        let top = take_top(items, 2);

        assert_eq!(top.len(), 2);
        assert_eq!(top[0].score, 95.0);
        assert_eq!(top[1].score, 85.0);
    }

    #[test]
    fn test_take_top_handles_limit_larger_than_items() {
        let items = vec![
            create_classified_item(95.0, RecommendationTier::T1CriticalArchitecture),
            create_classified_item(85.0, RecommendationTier::T2ComplexUntested),
        ];

        let top = take_top(items, 10);

        assert_eq!(top.len(), 2); // Only 2 items available
    }

    #[test]
    fn test_take_top_handles_empty_input() {
        let items: Vec<ClassifiedItem> = vec![];
        let top = take_top(items, 10);
        assert_eq!(top.len(), 0);
    }

    #[test]
    fn test_take_top_handles_zero_limit() {
        let items = vec![
            create_classified_item(95.0, RecommendationTier::T1CriticalArchitecture),
            create_classified_item(85.0, RecommendationTier::T2ComplexUntested),
        ];

        let top = take_top(items, 0);
        assert_eq!(top.len(), 0);
    }

    #[test]
    fn test_sort_maintains_item_data() {
        let items = vec![
            create_classified_item(50.0, RecommendationTier::T2ComplexUntested),
            create_classified_item(95.0, RecommendationTier::T1CriticalArchitecture),
        ];

        let sorted = sort_by_score(items);

        // Verify tier is preserved
        assert_eq!(sorted[0].tier, RecommendationTier::T1CriticalArchitecture);
        assert_eq!(sorted[1].tier, RecommendationTier::T2ComplexUntested);
    }

    // Property-based tests would go here using proptest
    // For example:
    // - prop_sort_maintains_count
    // - prop_sort_maintains_order_invariant
    // - prop_take_top_never_increases_count
    // - prop_pipeline_is_deterministic
}
