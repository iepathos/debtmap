//! Filter transparency with metrics tracking.
//!
//! This module provides pure functional filtering with comprehensive metrics
//! about what was filtered and why. All functions are deterministic with no
//! side effects, following the functional programming principles from Spec 224.

use super::tiers::RecommendationTier;
use super::DebtItem;

/// Metrics tracking filtering decisions with tier awareness.
///
/// Pure, immutable data structure that records what was filtered and why.
/// This enables transparency in filtering decisions, making it clear to users
/// what items were excluded from the analysis output.
///
/// Tracks items filtered by tier (T4), score (T3/T4), and tier bypass (T1/T2).
#[derive(Debug, Clone, PartialEq)]
pub struct FilterMetrics {
    /// Total items before filtering
    pub total_items: usize,

    /// Items filtered because they're T4 Maintenance tier
    pub filtered_t4_maintenance: usize,

    /// Items filtered because score below threshold
    pub filtered_below_score: usize,

    /// Items filtered because debt type disabled
    pub filtered_by_debt_type: usize,

    /// Items with T1/T2 tier included despite score below threshold
    pub tier_critical_bypass: usize,

    /// Items included in final output
    pub included: usize,

    /// Minimum score threshold used
    pub min_score_threshold: f64,

    /// Whether T4 items were shown
    pub show_t4: bool,
}

impl FilterMetrics {
    /// Creates empty metrics.
    pub fn empty() -> Self {
        Self {
            total_items: 0,
            filtered_t4_maintenance: 0,
            filtered_below_score: 0,
            filtered_by_debt_type: 0,
            tier_critical_bypass: 0,
            included: 0,
            min_score_threshold: 0.0,
            show_t4: false,
        }
    }

    /// Creates metrics from configuration.
    pub fn new(total: usize, min_score: f64, show_t4: bool) -> Self {
        Self {
            total_items: total,
            min_score_threshold: min_score,
            show_t4,
            ..Self::empty()
        }
    }

    /// Total items filtered (all reasons).
    pub fn total_filtered(&self) -> usize {
        self.filtered_t4_maintenance + self.filtered_below_score + self.filtered_by_debt_type
    }

    /// Percentage of items included.
    pub fn inclusion_rate(&self) -> f64 {
        if self.total_items == 0 {
            0.0
        } else {
            (self.included as f64 / self.total_items as f64) * 100.0
        }
    }
}

/// Configuration for filtering debt items.
///
/// Pure data structure specifying filtering criteria.
#[derive(Debug, Clone)]
pub struct FilterConfig {
    /// Minimum score threshold (items below this are filtered)
    pub min_score: f64,

    /// Whether to show T4 maintenance tier items
    pub show_t4: bool,
}

impl Default for FilterConfig {
    fn default() -> Self {
        Self {
            min_score: 3.0,
            show_t4: false,
        }
    }
}

/// Item with tier classification for filtering.
///
/// Temporary structure used during filtering process to carry
/// both the debt item and its classified tier.
#[derive(Debug, Clone)]
pub struct ClassifiedItem {
    pub item: DebtItem,
    pub tier: RecommendationTier,
    pub score: f64,
}

/// Result of filtering with transparency metrics.
///
/// Pure data structure containing filtered items and metrics about filtering.
#[derive(Debug, Clone)]
pub struct FilterResult {
    /// Items that passed all filters
    pub included: Vec<DebtItem>,

    /// Metrics about what was filtered
    pub metrics: FilterMetrics,
}

impl FilterResult {
    /// Creates new filter result.
    pub fn new(included: Vec<DebtItem>, metrics: FilterMetrics) -> Self {
        Self { included, metrics }
    }

    /// Creates empty result.
    pub fn empty() -> Self {
        Self {
            included: Vec::new(),
            metrics: FilterMetrics::empty(),
        }
    }
}

/// Checks if a tier is architecturally critical (T1 or T2).
///
/// Critical tier items bypass score threshold filtering because they
/// represent architectural issues (error handling, god objects, extreme
/// complexity) that should always be visible regardless of calculated score.
///
/// # Arguments
/// * `tier` - The recommendation tier to check
///
/// # Returns
/// `true` if tier is T1CriticalArchitecture or T2ComplexUntested
///
/// # Examples
/// ```
/// use debtmap::priority::filtering::is_critical_tier;
/// use debtmap::priority::tiers::RecommendationTier;
///
/// assert!(is_critical_tier(RecommendationTier::T1CriticalArchitecture));
/// assert!(is_critical_tier(RecommendationTier::T2ComplexUntested));
/// assert!(!is_critical_tier(RecommendationTier::T3TestingGaps));
/// ```
pub fn is_critical_tier(tier: RecommendationTier) -> bool {
    matches!(
        tier,
        RecommendationTier::T1CriticalArchitecture | RecommendationTier::T2ComplexUntested
    )
}

/// Filters items with metric collection (pure, functional).
///
/// This is a pure function that partitions items and tracks filtering decisions.
/// No side effects, fully deterministic.
///
/// # Arguments
///
/// * `items` - Classified items to filter
/// * `config` - Filter configuration (thresholds, enabled tiers)
///
/// # Returns
///
/// FilterResult containing included items and filtering metrics
///
/// # Examples
///
/// ```no_run
/// use debtmap::priority::filtering::{filter_with_metrics, FilterConfig, ClassifiedItem};
///
/// # let items: Vec<ClassifiedItem> = vec![];
/// # let config = FilterConfig::default();
/// let result = filter_with_metrics(items, &config);
/// println!("Included {} of {} items", result.metrics.included, result.metrics.total_items);
/// ```
pub fn filter_with_metrics(items: Vec<ClassifiedItem>, config: &FilterConfig) -> FilterResult {
    let total = items.len();
    let mut metrics = FilterMetrics::new(total, config.min_score, config.show_t4);

    // Partition items into included/excluded with tier-aware filtering
    let included: Vec<_> = items
        .into_iter()
        .filter_map(|classified_item| {
            // Step 1: Critical tiers (T1/T2) bypass score filter
            if is_critical_tier(classified_item.tier) {
                if classified_item.score < config.min_score {
                    metrics.tier_critical_bypass += 1;
                }
                return Some(classified_item.item);
            }

            // Step 2: T4 maintenance filtered by tier flag
            if !tier_passes(classified_item.tier, config) {
                metrics.filtered_t4_maintenance += 1;
                return None;
            }

            // Step 3: T3 (and T4 if shown) filtered by score
            if !score_passes(classified_item.score, config.min_score) {
                metrics.filtered_below_score += 1;
                return None;
            }

            Some(classified_item.item)
        })
        .collect();

    metrics.included = included.len();

    FilterResult::new(included, metrics)
}

/// Checks if tier should be included based on config (pure).
fn tier_passes(tier: RecommendationTier, config: &FilterConfig) -> bool {
    tier != RecommendationTier::T4Maintenance || config.show_t4
}

/// Checks if score passes threshold (pure).
fn score_passes(score: f64, threshold: f64) -> bool {
    score >= threshold
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_metrics() {
        let m = FilterMetrics::empty();
        assert_eq!(m.total_items, 0);
        assert_eq!(m.total_filtered(), 0);
        assert_eq!(m.inclusion_rate(), 0.0);
    }

    #[test]
    fn test_inclusion_rate() {
        let m = FilterMetrics {
            total_items: 100,
            included: 25,
            ..FilterMetrics::empty()
        };
        assert_eq!(m.inclusion_rate(), 25.0);
    }

    #[test]
    fn test_total_filtered() {
        let m = FilterMetrics {
            filtered_t4_maintenance: 10,
            filtered_below_score: 5,
            filtered_by_debt_type: 3,
            ..FilterMetrics::empty()
        };
        assert_eq!(m.total_filtered(), 18);
    }

    #[test]
    fn test_tier_passes_t4_hidden() {
        let config = FilterConfig {
            show_t4: false,
            ..Default::default()
        };
        assert!(!tier_passes(RecommendationTier::T4Maintenance, &config));
        assert!(tier_passes(
            RecommendationTier::T1CriticalArchitecture,
            &config
        ));
    }

    #[test]
    fn test_tier_passes_t4_shown() {
        let config = FilterConfig {
            show_t4: true,
            ..Default::default()
        };
        assert!(tier_passes(RecommendationTier::T4Maintenance, &config));
        assert!(tier_passes(
            RecommendationTier::T1CriticalArchitecture,
            &config
        ));
    }

    #[test]
    fn test_score_passes() {
        assert!(score_passes(5.0, 3.0));
        assert!(score_passes(3.0, 3.0));
        assert!(!score_passes(2.9, 3.0));
        assert!(!score_passes(0.0, 3.0));
    }

    // Helper function for creating test debt items
    fn create_test_debt_item() -> super::super::DebtItem {
        use super::super::{
            ActionableRecommendation, DebtType, ImpactMetrics, Location, UnifiedDebtItem,
            UnifiedScore,
        };
        use crate::priority::semantic_classifier::FunctionRole;

        super::super::DebtItem::Function(Box::new(UnifiedDebtItem {
            location: Location {
                file: "test.rs".into(),
                function: "test_fn".to_string(),
                line: 1,
            },
            debt_type: DebtType::ComplexityHotspot {
                cyclomatic: 10,
                cognitive: 15,
                adjusted_cyclomatic: None,
            },
            unified_score: UnifiedScore {
                complexity_factor: 0.0,
                coverage_factor: 0.0,
                dependency_factor: 0.0,
                role_multiplier: 1.0,
                final_score: 5.0,
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
                primary_action: "Refactor".to_string(),
                rationale: "Test".to_string(),
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
            nesting_depth: 0,
            function_length: 10,
            cyclomatic_complexity: 1,
            cognitive_complexity: 1,
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
        }))
    }

    // Tier-aware filtering tests (Spec 205)

    #[test]
    fn test_t1_bypasses_score_filter() {
        let items = vec![ClassifiedItem {
            item: create_test_debt_item(),
            tier: RecommendationTier::T1CriticalArchitecture,
            score: 2.5, // Below default threshold of 3.0
        }];

        let config = FilterConfig::default();
        let result = filter_with_metrics(items, &config);

        assert_eq!(result.metrics.included, 1, "T1 item should be included");
        assert_eq!(
            result.metrics.tier_critical_bypass, 1,
            "Should track bypass"
        );
        assert_eq!(
            result.metrics.filtered_below_score, 0,
            "Score filter not applied"
        );
    }

    #[test]
    fn test_t2_bypasses_high_threshold() {
        let items = vec![ClassifiedItem {
            item: create_test_debt_item(),
            tier: RecommendationTier::T2ComplexUntested,
            score: 2.0, // Far below threshold
        }];

        let config = FilterConfig {
            min_score: 5.0, // High threshold
            show_t4: false,
        };
        let result = filter_with_metrics(items, &config);

        assert_eq!(
            result.metrics.included, 1,
            "T2 item should bypass threshold"
        );
        assert_eq!(result.metrics.tier_critical_bypass, 1);
    }

    #[test]
    fn test_t3_respects_score_threshold() {
        let items = vec![ClassifiedItem {
            item: create_test_debt_item(),
            tier: RecommendationTier::T3TestingGaps,
            score: 2.5, // Below threshold
        }];

        let config = FilterConfig::default(); // threshold 3.0
        let result = filter_with_metrics(items, &config);

        assert_eq!(result.metrics.included, 0, "T3 should be filtered by score");
        assert_eq!(result.metrics.filtered_below_score, 1);
        assert_eq!(result.metrics.tier_critical_bypass, 0);
    }

    #[test]
    fn test_t4_filtered_by_tier_flag() {
        let items = vec![ClassifiedItem {
            item: create_test_debt_item(),
            tier: RecommendationTier::T4Maintenance,
            score: 5.0, // High score, but T4
        }];

        let config = FilterConfig {
            min_score: 3.0,
            show_t4: false, // T4 hidden
        };
        let result = filter_with_metrics(items, &config);

        assert_eq!(result.metrics.included, 0, "T4 should be filtered by tier");
        assert_eq!(result.metrics.filtered_t4_maintenance, 1);
        assert_eq!(result.metrics.filtered_below_score, 0);
    }

    #[test]
    fn test_metrics_track_tier_bypass() {
        let items = vec![
            ClassifiedItem {
                tier: RecommendationTier::T1CriticalArchitecture,
                score: 1.0,
                item: create_test_debt_item(),
            },
            ClassifiedItem {
                tier: RecommendationTier::T1CriticalArchitecture,
                score: 2.0,
                item: create_test_debt_item(),
            },
            ClassifiedItem {
                tier: RecommendationTier::T2ComplexUntested,
                score: 1.5,
                item: create_test_debt_item(),
            },
        ];

        let config = FilterConfig::default();
        let result = filter_with_metrics(items, &config);

        assert_eq!(
            result.metrics.included, 3,
            "All critical tier items included"
        );
        assert_eq!(
            result.metrics.tier_critical_bypass, 3,
            "All 3 bypassed score"
        );
        assert_eq!(result.metrics.filtered_below_score, 0);
    }

    #[test]
    fn test_is_critical_tier_predicate() {
        assert!(is_critical_tier(RecommendationTier::T1CriticalArchitecture));
        assert!(is_critical_tier(RecommendationTier::T2ComplexUntested));
        assert!(!is_critical_tier(RecommendationTier::T3TestingGaps));
        assert!(!is_critical_tier(RecommendationTier::T4Maintenance));
    }

    #[test]
    fn test_mixed_tiers_filtered_correctly() {
        let items = vec![
            // T1 with low score → included
            ClassifiedItem {
                tier: RecommendationTier::T1CriticalArchitecture,
                score: 1.0,
                item: create_test_debt_item(),
            },
            // T2 with low score → included
            ClassifiedItem {
                tier: RecommendationTier::T2ComplexUntested,
                score: 2.0,
                item: create_test_debt_item(),
            },
            // T3 with low score → excluded
            ClassifiedItem {
                tier: RecommendationTier::T3TestingGaps,
                score: 2.5,
                item: create_test_debt_item(),
            },
            // T3 with high score → included
            ClassifiedItem {
                tier: RecommendationTier::T3TestingGaps,
                score: 5.0,
                item: create_test_debt_item(),
            },
            // T4 with show_t4=false → excluded
            ClassifiedItem {
                tier: RecommendationTier::T4Maintenance,
                score: 10.0,
                item: create_test_debt_item(),
            },
        ];

        let config = FilterConfig {
            min_score: 3.0,
            show_t4: false,
        };
        let result = filter_with_metrics(items, &config);

        assert_eq!(result.metrics.included, 3, "T1, T2, and high-score T3");
        assert_eq!(result.metrics.tier_critical_bypass, 2, "T1 and T2 bypassed");
        assert_eq!(result.metrics.filtered_below_score, 1, "Low-score T3");
        assert_eq!(result.metrics.filtered_t4_maintenance, 1, "T4 by tier");
    }

    #[test]
    fn test_t1_above_threshold_not_counted_as_bypass() {
        let items = vec![ClassifiedItem {
            item: create_test_debt_item(),
            tier: RecommendationTier::T1CriticalArchitecture,
            score: 5.0, // Above threshold
        }];

        let config = FilterConfig::default();
        let result = filter_with_metrics(items, &config);

        assert_eq!(result.metrics.included, 1, "T1 item should be included");
        assert_eq!(
            result.metrics.tier_critical_bypass, 0,
            "No bypass needed for high score"
        );
        assert_eq!(result.metrics.filtered_below_score, 0);
    }

    #[test]
    fn test_t4_shown_respects_score_threshold() {
        let items = vec![ClassifiedItem {
            item: create_test_debt_item(),
            tier: RecommendationTier::T4Maintenance,
            score: 2.0, // Below threshold
        }];

        let config = FilterConfig {
            min_score: 3.0,
            show_t4: true, // T4 shown, so subject to score filter
        };
        let result = filter_with_metrics(items, &config);

        assert_eq!(
            result.metrics.included, 0,
            "T4 with show_t4=true should be filtered by score"
        );
        assert_eq!(result.metrics.filtered_t4_maintenance, 0);
        assert_eq!(
            result.metrics.filtered_below_score, 1,
            "T4 filtered by score, not tier"
        );
    }
}
