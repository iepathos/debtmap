//! Stage 3: Filtering - Pure filter predicates and item filtering.
//!
//! This module provides pure functions for filtering view items based on
//! score thresholds and tier configuration.

use crate::priority::{
    tiers::RecommendationTier,
    view::{ViewConfig, ViewItem},
};

/// Statistics about filtered items.
#[derive(Debug, Default)]
pub struct FilterStats {
    pub filtered_by_score: usize,
    pub filtered_by_tier: usize,
}

/// Filters items based on configuration.
///
/// Pure function - returns new Vec and filter statistics.
pub fn filter_items(items: Vec<ViewItem>, config: &ViewConfig) -> (Vec<ViewItem>, FilterStats) {
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
pub fn passes_score_threshold(item: &ViewItem, threshold: f64) -> bool {
    item.score() >= threshold
}

/// Checks if item passes tier filter.
pub fn passes_tier_filter(item: &ViewItem, exclude_t4: bool) -> bool {
    if !exclude_t4 {
        return true;
    }

    !matches!(item.tier(), Some(RecommendationTier::T4Maintenance))
}
