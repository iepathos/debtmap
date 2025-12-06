//! Filter transparency with metrics tracking.
//!
//! This module provides pure functional filtering with comprehensive metrics
//! about what was filtered and why. All functions are deterministic with no
//! side effects, following the functional programming principles from Spec 224.

use super::tiers::RecommendationTier;
use super::DebtItem;

/// Metrics tracking filtering decisions.
///
/// Pure, immutable data structure that records what was filtered and why.
/// This enables transparency in filtering decisions, making it clear to users
/// what items were excluded from the analysis output.
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

    // Partition items into included/excluded (pure)
    let included: Vec<_> = items
        .into_iter()
        .filter_map(|classified_item| {
            // Track why items are filtered
            if !tier_passes(classified_item.tier, config) {
                metrics.filtered_t4_maintenance += 1;
                return None;
            }

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
}
