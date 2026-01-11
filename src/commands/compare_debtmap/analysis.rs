//! Pure analysis functions for comparing debtmap states.
//!
//! All functions in this module are pure - they compute results
//! from inputs without side effects. Functions are kept under 20 lines.

use super::types::{
    extract_function_keys, extract_functions, extract_location_keys, is_critical,
    is_score_unchanged, is_significantly_improved, AnalysisSummary, DebtmapJsonInput,
    IdentifiedChanges, ImprovedItems, ItemInfo, NewItems, ResolvedItems, UnchangedCritical,
};
use crate::output::unified::{FunctionDebtItemOutput, UnifiedDebtItemOutput};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

// =============================================================================
// Summary Creation
// =============================================================================

/// Pure: Create summary from function items
pub fn create_summary(analysis: &DebtmapJsonInput) -> AnalysisSummary {
    let function_items: Vec<_> = extract_functions(&analysis.items).collect();
    let scores: Vec<f64> = function_items.iter().map(|f| f.score).collect();

    AnalysisSummary {
        total_items: function_items.len(),
        high_priority_items: count_critical_items(&scores),
        average_score: calculate_average(&scores),
    }
}

/// Pure: Count items with critical scores
fn count_critical_items(scores: &[f64]) -> usize {
    scores.iter().filter(|&&s| is_critical(s)).count()
}

/// Pure: Calculate average of scores
fn calculate_average(scores: &[f64]) -> f64 {
    if scores.is_empty() {
        0.0
    } else {
        scores.iter().sum::<f64>() / scores.len() as f64
    }
}

// =============================================================================
// Change Identification
// =============================================================================

/// Pure: Identify all changes between before and after debtmaps
pub fn identify_all_changes(
    before: &DebtmapJsonInput,
    after: &DebtmapJsonInput,
) -> IdentifiedChanges {
    IdentifiedChanges {
        resolved: identify_resolved_items(before, after),
        improved: identify_improved_items(before, after),
        new_items: identify_new_items(before, after),
        unchanged_critical: identify_unchanged_critical(before, after),
    }
}

// =============================================================================
// Resolved Items Analysis
// =============================================================================

/// Pure: Identify items that were resolved (removed from after)
pub fn identify_resolved_items(
    before: &DebtmapJsonInput,
    after: &DebtmapJsonInput,
) -> ResolvedItems {
    let after_keys: HashSet<_> = extract_location_keys(&after.items).collect();
    let resolved = find_removed_functions(&before.items, &after_keys);

    ResolvedItems {
        high_priority_count: count_high_priority(&resolved),
        total_count: resolved.len(),
    }
}

/// Pure: Find functions that exist in items but not in keys set
fn find_removed_functions<'a>(
    items: &'a [UnifiedDebtItemOutput],
    existing_keys: &HashSet<(PathBuf, String)>,
) -> Vec<&'a FunctionDebtItemOutput> {
    extract_functions(items)
        .filter(|f| {
            let key = (
                PathBuf::from(&f.location.file),
                f.location.function.clone().unwrap_or_default(),
            );
            !existing_keys.contains(&key)
        })
        .collect()
}

/// Pure: Count high priority items in resolved list
fn count_high_priority(items: &[&FunctionDebtItemOutput]) -> usize {
    items.iter().filter(|item| is_critical(item.score)).count()
}

// =============================================================================
// Improved Items Analysis
// =============================================================================

/// Pure: Identify items that improved between before and after
pub fn identify_improved_items(
    before: &DebtmapJsonInput,
    after: &DebtmapJsonInput,
) -> ImprovedItems {
    let before_map: HashMap<_, _> = extract_function_keys(&before.items).collect();
    let improvements = collect_improvements(&after.items, &before_map);

    aggregate_improvements(improvements)
}

/// Single item improvement metrics
struct ImprovementMetrics {
    complexity_reduction: f64,
    has_coverage_improvement: bool,
}

/// Pure: Collect improvement metrics for each improved item
fn collect_improvements(
    after_items: &[UnifiedDebtItemOutput],
    before_map: &HashMap<(PathBuf, String), &FunctionDebtItemOutput>,
) -> Vec<ImprovementMetrics> {
    extract_functions(after_items)
        .filter_map(|after| {
            let key = (
                PathBuf::from(&after.location.file),
                after.location.function.clone().unwrap_or_default(),
            );
            before_map
                .get(&key)
                .and_then(|before| compute_improvement_if_significant(before, after))
        })
        .collect()
}

/// Pure: Compute improvement metrics if the improvement is significant
fn compute_improvement_if_significant(
    before: &FunctionDebtItemOutput,
    after: &FunctionDebtItemOutput,
) -> Option<ImprovementMetrics> {
    let before_score = before.score;
    let after_score = after.score;

    if !is_significantly_improved(before_score, after_score) {
        return None;
    }

    Some(ImprovementMetrics {
        complexity_reduction: compute_complexity_reduction(before, after),
        has_coverage_improvement: has_coverage_improved(before, after),
    })
}

/// Pure: Compute complexity reduction ratio
fn compute_complexity_reduction(
    before: &FunctionDebtItemOutput,
    after: &FunctionDebtItemOutput,
) -> f64 {
    let before_cc = before.metrics.cyclomatic_complexity;
    let after_cc = after.metrics.cyclomatic_complexity;

    if after_cc >= before_cc {
        return 0.0;
    }
    let reduction = before_cc - after_cc;
    reduction as f64 / before_cc as f64
}

/// Pure: Check if coverage improved
fn has_coverage_improved(before: &FunctionDebtItemOutput, after: &FunctionDebtItemOutput) -> bool {
    let before_cov = before.metrics.coverage.unwrap_or(0.0);
    let after_cov = after.metrics.coverage.unwrap_or(0.0);
    after_cov > before_cov
}

/// Pure: Aggregate individual improvements into summary
fn aggregate_improvements(improvements: Vec<ImprovementMetrics>) -> ImprovedItems {
    if improvements.is_empty() {
        return ImprovedItems {
            complexity_reduction: 0.0,
            coverage_improvement: 0.0,
            coverage_improvement_count: 0,
        };
    }

    let total_reduction: f64 = improvements.iter().map(|i| i.complexity_reduction).sum();
    let coverage_count = improvements
        .iter()
        .filter(|i| i.has_coverage_improvement)
        .count();

    ImprovedItems {
        complexity_reduction: total_reduction / improvements.len() as f64,
        coverage_improvement: coverage_count as f64,
        coverage_improvement_count: coverage_count,
    }
}

// =============================================================================
// New Items Analysis
// =============================================================================

/// Pure: Identify new critical items introduced in after
pub fn identify_new_items(before: &DebtmapJsonInput, after: &DebtmapJsonInput) -> NewItems {
    let before_keys: HashSet<_> = extract_location_keys(&before.items).collect();
    let new_items = find_new_critical_items(&after.items, &before_keys);

    NewItems {
        critical_count: new_items.len(),
        items: new_items,
    }
}

/// Pure: Find new critical items not in before
fn find_new_critical_items(
    after_items: &[UnifiedDebtItemOutput],
    before_keys: &HashSet<(PathBuf, String)>,
) -> Vec<ItemInfo> {
    extract_functions(after_items)
        .filter(|f| {
            let key = (
                PathBuf::from(&f.location.file),
                f.location.function.clone().unwrap_or_default(),
            );
            !before_keys.contains(&key)
        })
        .filter(|f| is_critical(f.score))
        .map(function_to_item_info)
        .collect()
}

/// Pure: Convert FunctionDebtItemOutput to ItemInfo
fn function_to_item_info(item: &FunctionDebtItemOutput) -> ItemInfo {
    ItemInfo {
        file: PathBuf::from(&item.location.file),
        function: item.location.function.clone().unwrap_or_default(),
        line: item.location.line.unwrap_or(0),
        score: item.score,
    }
}

// =============================================================================
// Unchanged Critical Analysis
// =============================================================================

/// Pure: Identify critical items that remain unchanged
pub fn identify_unchanged_critical(
    before: &DebtmapJsonInput,
    after: &DebtmapJsonInput,
) -> UnchangedCritical {
    let after_map: HashMap<_, _> = extract_function_keys(&after.items).collect();
    let items = find_unchanged_critical(&before.items, &after_map);

    UnchangedCritical {
        count: items.len(),
        items,
    }
}

/// Pure: Find critical items that remained unchanged
fn find_unchanged_critical(
    before_items: &[UnifiedDebtItemOutput],
    after_map: &HashMap<(PathBuf, String), &FunctionDebtItemOutput>,
) -> Vec<ItemInfo> {
    extract_functions(before_items)
        .filter(|before| is_critical(before.score))
        .filter_map(|before| check_if_unchanged(before, after_map))
        .collect()
}

/// Pure: Check if a critical item remained unchanged in after
fn check_if_unchanged(
    before: &FunctionDebtItemOutput,
    after_map: &HashMap<(PathBuf, String), &FunctionDebtItemOutput>,
) -> Option<ItemInfo> {
    let key = (
        PathBuf::from(&before.location.file),
        before.location.function.clone().unwrap_or_default(),
    );
    let before_score = before.score;

    after_map.get(&key).and_then(|after| {
        let after_score = after.score;
        if is_score_unchanged(before_score, after_score) && is_critical(after_score) {
            Some(function_to_item_info(before))
        } else {
            None
        }
    })
}

// =============================================================================
// Helper for Tests
// =============================================================================

/// Build a map of (file, function) -> FunctionDebtItemOutput for quick lookup.
/// Used primarily in tests.
#[cfg(test)]
pub fn build_function_map(
    items: &[UnifiedDebtItemOutput],
) -> HashMap<(PathBuf, String), &FunctionDebtItemOutput> {
    extract_function_keys(items).collect()
}
