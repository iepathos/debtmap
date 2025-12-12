//! Stage 7: Summary - Pure summary calculation functions.
//!
//! This module provides pure functions for calculating summary statistics
//! from view items.

use super::filter::FilterStats;
use crate::priority::{
    classification::Severity,
    view::{CategoryCounts, ScoreDistribution, ViewItem, ViewSummary},
    DebtCategory,
};

/// Calculates summary statistics from items.
///
/// Pure function - aggregates data from items list.
pub fn calculate_summary(
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
