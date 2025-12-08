//! Pure message builder functions for validation output.
//!
//! All functions in this module are pure - they take inputs and return
//! formatted strings without any side effects.

use super::types::{GapDetail, ImprovedItems, ItemInfo, NewItems, ResolvedItems, UnchangedCritical};
use std::collections::HashMap;

// =============================================================================
// Improvement Message Builders
// =============================================================================

/// Pure: Build resolved items improvement message
pub fn build_resolved_message(resolved: &ResolvedItems) -> Option<String> {
    if resolved.high_priority_count > 0 {
        Some(format!(
            "Resolved {} high-priority debt items",
            resolved.high_priority_count
        ))
    } else {
        None
    }
}

/// Pure: Build complexity reduction improvement message
pub fn build_complexity_message(improved: &ImprovedItems) -> Option<String> {
    if improved.complexity_reduction > 0.0 {
        Some(format!(
            "Reduced average cyclomatic complexity by {:.0}%",
            improved.complexity_reduction * 100.0
        ))
    } else {
        None
    }
}

/// Pure: Build coverage improvement message
pub fn build_coverage_message(improved: &ImprovedItems) -> Option<String> {
    if improved.coverage_improvement > 0.0 {
        Some(format!(
            "Added test coverage for {} critical functions",
            improved.coverage_improvement_count
        ))
    } else {
        None
    }
}

/// Pure: Build all improvement messages
pub fn build_all_improvement_messages(
    resolved: &ResolvedItems,
    improved: &ImprovedItems,
) -> Vec<String> {
    [
        build_resolved_message(resolved),
        build_complexity_message(improved),
        build_coverage_message(improved),
    ]
    .into_iter()
    .flatten()
    .collect()
}

// =============================================================================
// Issue Message Builders
// =============================================================================

/// Pure: Build unchanged critical items issue message
pub fn build_unchanged_critical_message(unchanged_critical: &UnchangedCritical) -> Option<String> {
    if unchanged_critical.count > 0 {
        Some(format!(
            "{} critical debt item{} still present",
            unchanged_critical.count,
            if unchanged_critical.count == 1 { "" } else { "s" }
        ))
    } else {
        None
    }
}

/// Pure: Build regression issue message
pub fn build_regression_message(new_items: &NewItems) -> Option<String> {
    if new_items.critical_count > 0 {
        Some(format!(
            "{} new critical debt items introduced",
            new_items.critical_count
        ))
    } else {
        None
    }
}

/// Pure: Build all issue messages
pub fn build_all_issue_messages(
    unchanged_critical: &UnchangedCritical,
    new_items: &NewItems,
) -> Vec<String> {
    [
        build_unchanged_critical_message(unchanged_critical),
        build_regression_message(new_items),
    ]
    .into_iter()
    .flatten()
    .collect()
}

// =============================================================================
// Gap Detail Builders
// =============================================================================

/// Pure: Build a single critical debt gap detail
pub fn build_critical_debt_gap(item: &ItemInfo, idx: usize) -> (String, GapDetail) {
    let key = format!("critical_debt_remaining_{}", idx);
    let detail = GapDetail {
        description: format!("High-priority debt item still present in {}", item.function),
        location: format!("{}:{}:{}", item.file.display(), item.function, item.line),
        severity: "high".to_string(),
        suggested_fix: "Apply functional programming patterns to reduce complexity".to_string(),
        original_score: Some(item.score),
        current_score: Some(item.score),
        original_complexity: None,
        current_complexity: None,
        target_complexity: None,
    };
    (key, detail)
}

/// Pure: Build regression gap detail
pub fn build_regression_gap(new_items: &NewItems) -> Option<(String, GapDetail)> {
    if new_items.critical_count == 0 {
        return None;
    }

    let detail = GapDetail {
        description: "New complexity introduced during refactoring".to_string(),
        location: new_items
            .items
            .first()
            .map(|i| format!("{}:{}:{}", i.file.display(), i.function, i.line))
            .unwrap_or_default(),
        severity: "high".to_string(),
        suggested_fix: "Simplify the newly added conditional logic".to_string(),
        original_score: None,
        current_score: new_items.items.first().map(|i| i.score),
        original_complexity: None,
        current_complexity: None,
        target_complexity: None,
    };
    Some(("regression_detected".to_string(), detail))
}

/// Pure: Build all gaps from analysis results
pub fn build_all_gaps(
    unchanged_critical: &UnchangedCritical,
    new_items: &NewItems,
) -> HashMap<String, GapDetail> {
    let mut gaps = HashMap::new();

    // Add critical debt gaps (max 2)
    for (idx, item) in unchanged_critical.items.iter().take(2).enumerate() {
        let (key, detail) = build_critical_debt_gap(item, idx);
        gaps.insert(key, detail);
    }

    // Add regression gap if applicable
    if let Some((key, detail)) = build_regression_gap(new_items) {
        gaps.insert(key, detail);
    }

    gaps
}
