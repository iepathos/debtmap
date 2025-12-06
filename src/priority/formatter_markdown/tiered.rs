//! Tiered display formatting for markdown output
//!
//! Formats technical debt analysis results grouped by severity tier
//! (Critical, High, Moderate, Low)

use crate::priority::{DebtItem, DisplayGroup, FileDebtItem, Tier, UnifiedDebtItem};
use std::fmt::Write;

use super::utilities::{
    extract_complexity_info, format_debt_type, format_file_impact, format_impact,
};

pub(crate) fn format_tier_section(
    output: &mut String,
    groups: &[DisplayGroup],
    tier: Tier,
    verbosity: u8,
) {
    if groups.is_empty() {
        return;
    }

    writeln!(output, "### {}", tier.header()).unwrap();
    writeln!(output, "_Estimated effort: {}_\n", tier.effort_estimate()).unwrap();

    let max_items_per_tier = 5;
    let mut items_shown = 0;

    for group in groups {
        if items_shown >= max_items_per_tier && verbosity < 2 {
            let remaining: usize = groups.iter().skip(items_shown).map(|g| g.items.len()).sum();
            if remaining > 0 {
                writeln!(
                    output,
                    "\n_... and {} more items in this tier_\n",
                    remaining
                )
                .unwrap();
            }
            break;
        }

        format_display_group(output, group, verbosity);
        items_shown += group.items.len();
    }

    writeln!(output).unwrap();
}

pub(crate) fn format_display_group(output: &mut String, group: &DisplayGroup, verbosity: u8) {
    if group.items.len() > 1 && group.batch_action.is_some() {
        // Format as grouped items
        writeln!(
            output,
            "#### {} ({} items)",
            group.debt_type,
            group.items.len()
        )
        .unwrap();

        if let Some(action) = &group.batch_action {
            writeln!(output, "**Batch Action:** {}\n", action).unwrap();
        }

        if verbosity >= 1 {
            writeln!(output, "**Items:**").unwrap();
            for (idx, item) in group.items.iter().take(3).enumerate() {
                format_debt_item_brief(output, idx + 1, item);
            }
            if group.items.len() > 3 {
                writeln!(
                    output,
                    "- _... and {} more similar items_",
                    group.items.len() - 3
                )
                .unwrap();
            }
        } else {
            let total_score: f64 = group.items.iter().map(|i| i.score()).sum();
            writeln!(output, "- Combined Score: {:.1}", total_score).unwrap();
            writeln!(output, "- Count: {} items", group.items.len()).unwrap();
        }
    } else {
        // Format as individual item
        for item in &group.items {
            format_debt_item_detailed(output, item, verbosity);
        }
    }
    writeln!(output).unwrap();
}

pub(crate) fn format_debt_item_brief(output: &mut String, rank: usize, item: &DebtItem) {
    match item {
        DebtItem::Function(func) => {
            writeln!(
                output,
                "- #{} `{}` (Score: {:.1})",
                rank, func.location.function, func.unified_score.final_score
            )
            .unwrap();
        }
        DebtItem::File(file) => {
            writeln!(
                output,
                "- #{} `{}` (Score: {:.1})",
                rank,
                file.metrics.path.display(),
                file.score
            )
            .unwrap();
        }
    }
}

pub(crate) fn format_debt_item_detailed(output: &mut String, item: &DebtItem, verbosity: u8) {
    match item {
        DebtItem::Function(func) => {
            format_function_debt_item(output, func, verbosity);
        }
        DebtItem::File(file) => {
            format_file_debt_item(output, file, verbosity);
        }
    }
}

pub(crate) fn format_function_debt_item(
    output: &mut String,
    item: &UnifiedDebtItem,
    verbosity: u8,
) {
    let score = item.unified_score.final_score;
    writeln!(
        output,
        "#### {} - Score: {:.1}",
        item.location.function, score
    )
    .unwrap();

    writeln!(
        output,
        "**Location:** `{}:{}`",
        item.location.file.display(),
        item.location.line
    )
    .unwrap();

    writeln!(output, "**Type:** {}", format_debt_type(&item.debt_type)).unwrap();

    writeln!(output, "**Action:** {}", item.recommendation.primary_action).unwrap();

    if let Some(complexity) = extract_complexity_info(&item.debt_type) {
        writeln!(output, "**Complexity:** {}", complexity).unwrap();
    }

    if verbosity >= 1 {
        writeln!(
            output,
            "**Impact:** {}",
            format_impact(&item.expected_impact)
        )
        .unwrap();
        writeln!(output, "**Why:** {}", item.recommendation.rationale).unwrap();
    }
}

pub(crate) fn format_file_debt_item(output: &mut String, item: &FileDebtItem, verbosity: u8) {
    let score = item.score;
    let file_name = item
        .metrics
        .path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown");

    writeln!(output, "#### {} - Score: {:.1}", file_name, score).unwrap();

    writeln!(
        output,
        "**File:** `{}` ({} lines, {} functions)",
        item.metrics.path.display(),
        item.metrics.total_lines,
        item.metrics.function_count
    )
    .unwrap();

    let is_god_object = item
        .metrics
        .god_object_analysis
        .as_ref()
        .is_some_and(|a| a.is_god_object);

    if is_god_object {
        let god_analysis = item.metrics.god_object_analysis.as_ref().unwrap();
        writeln!(output, "**Type:** GOD OBJECT").unwrap();
        writeln!(
            output,
            "**Metrics:** {} methods, {} fields, {} responsibilities",
            god_analysis.method_count, god_analysis.field_count, god_analysis.responsibility_count
        )
        .unwrap();
    } else {
        // Use context-aware threshold if available (spec 135)
        let type_str = if let Some(ref file_type) = item.metrics.file_type {
            use crate::organization::get_threshold;
            let threshold = get_threshold(
                file_type,
                item.metrics.function_count,
                item.metrics.total_lines,
            );
            if item.metrics.total_lines > threshold.base_threshold {
                format!("**Type:** LARGE FILE ({:?})", file_type)
            } else {
                format!("**Type:** COMPLEX FILE ({:?})", file_type)
            }
        } else {
            // Legacy behavior if no file type
            if item.metrics.total_lines > 500 {
                "**Type:** LARGE FILE".to_string()
            } else {
                "**Type:** COMPLEX FILE".to_string()
            }
        };
        writeln!(output, "{}", type_str).unwrap();
    }

    writeln!(output, "**Recommendation:** {}", item.recommendation).unwrap();

    if verbosity >= 1 {
        writeln!(output, "**Impact:** {}", format_file_impact(&item.impact)).unwrap();
    }
}
