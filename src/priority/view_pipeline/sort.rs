//! Stage 4: Sorting - Pure sorting functions for view items.
//!
//! This module provides pure functions for sorting view items by various criteria.

use crate::priority::view::{SortCriteria, ViewItem};

/// Sorts items by the specified criteria.
///
/// Pure function - returns new sorted Vec.
pub fn sort_items(mut items: Vec<ViewItem>, criteria: SortCriteria) -> Vec<ViewItem> {
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
pub fn get_coverage(item: &ViewItem) -> Option<f64> {
    match item {
        ViewItem::Function(f) => f.transitive_coverage.as_ref().map(|c| c.direct),
        ViewItem::File(f) => Some(f.metrics.coverage_percent),
    }
}

/// Extracts complexity from item.
pub fn get_complexity(item: &ViewItem) -> u32 {
    match item {
        ViewItem::Function(f) => f.cognitive_complexity,
        ViewItem::File(f) => f.metrics.max_complexity,
    }
}
