//! Pure functions for debt scoring and prioritization.
//!
//! These functions score and prioritize debt items without side effects.

use super::debt::DebtItem;

/// Score a debt item based on its properties (pure).
///
/// Calculates a priority score for a debt item based on type and severity.
///
/// # Arguments
///
/// * `item` - Debt item to score
///
/// # Returns
///
/// Priority score (higher = more important)
pub fn score_debt_item(item: &DebtItem) -> f64 {
    item.severity
}

/// Prioritize debt items by score (pure).
///
/// Sorts debt items by priority score in descending order.
///
/// # Arguments
///
/// * `items` - Vector of debt items to prioritize
///
/// # Returns
///
/// Vector of debt items sorted by priority (highest first)
pub fn prioritize_debt(mut items: Vec<DebtItem>) -> Vec<DebtItem> {
    items.sort_by(|a, b| {
        score_debt_item(b)
            .partial_cmp(&score_debt_item(a))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    items
}

/// Take top N highest priority items (pure).
///
/// Filters to keep only the top N items by priority.
///
/// # Arguments
///
/// * `items` - Prioritized debt items
/// * `n` - Number of items to keep
///
/// # Returns
///
/// Top N items
pub fn take_top_n(items: Vec<DebtItem>, n: usize) -> Vec<DebtItem> {
    items.into_iter().take(n).collect()
}

/// Score debt items for pipeline integration (adapter).
///
/// TODO: Full integration with scoring system.
/// For now returns empty vector to allow compilation.
pub fn score_debt_items(
    _items: &[crate::priority::UnifiedDebtItem],
    _call_graph: Option<&crate::priority::call_graph::CallGraph>,
    _coverage: Option<&crate::pipeline::data::CoverageData>,
    _purity: Option<&crate::pipeline::data::PurityScores>,
) -> Vec<crate::pipeline::data::ScoredDebtItem> {
    // TODO: Implement full scoring using existing unified scorer
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::stages::debt::DebtItemType;

    fn test_item(name: &str, severity: f64) -> DebtItem {
        DebtItem {
            function_name: name.to_string(),
            debt_type: DebtItemType::HighComplexity,
            severity,
        }
    }

    #[test]
    fn test_score_debt_item() {
        let item = test_item("foo", 75.0);
        let score = score_debt_item(&item);
        assert_eq!(score, 75.0);
    }

    #[test]
    fn test_prioritize_debt_sorts() {
        let items = vec![
            test_item("low", 30.0),
            test_item("high", 70.0),
            test_item("medium", 50.0),
        ];

        let prioritized = prioritize_debt(items);

        assert_eq!(prioritized[0].function_name, "high");
        assert_eq!(prioritized[1].function_name, "medium");
        assert_eq!(prioritized[2].function_name, "low");
    }

    #[test]
    fn test_take_top_n() {
        let items = vec![
            test_item("high", 70.0),
            test_item("medium", 50.0),
            test_item("low", 30.0),
        ];

        let top_2 = take_top_n(items, 2);

        assert_eq!(top_2.len(), 2);
        assert_eq!(top_2[0].function_name, "high");
        assert_eq!(top_2[1].function_name, "medium");
    }
}
