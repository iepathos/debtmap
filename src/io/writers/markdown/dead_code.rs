//! Dead code analysis formatting functions
//!
//! Contains functions for analyzing and formatting dead code sections,
//! including table generation and recommendations.

use crate::priority::{UnifiedAnalysis, UnifiedDebtItem};

use super::formatters::{format_visibility, get_dead_code_recommendation};

/// Extract dead code items from analysis
fn filter_dead_code_items(analysis: &UnifiedAnalysis) -> Vec<&UnifiedDebtItem> {
    use crate::priority::DebtType;
    analysis
        .items
        .iter()
        .filter(|item| matches!(item.debt_type, DebtType::DeadCode { .. }))
        .collect()
}

/// Format a single dead code table row
fn format_dead_code_row(item: &UnifiedDebtItem) -> Option<String> {
    use crate::priority::DebtType;
    if let DebtType::DeadCode {
        visibility,
        cyclomatic,
        ..
    } = &item.debt_type
    {
        let vis_str = format_visibility(visibility);
        let recommendation = get_dead_code_recommendation(visibility, *cyclomatic);
        Some(format!(
            "| `{}` | {} | {} | {} |",
            item.location.function, vis_str, cyclomatic, recommendation
        ))
    } else {
        None
    }
}

/// Generate dead code table headers
fn get_dead_code_table_headers() -> (&'static str, &'static str) {
    (
        "| Function | Visibility | Complexity | Recommendation |",
        "|----------|------------|------------|----------------|",
    )
}

/// Format the entire dead code section as a string
pub fn format_dead_code_section(analysis: &UnifiedAnalysis) -> String {
    let dead_code_items = filter_dead_code_items(analysis);

    if dead_code_items.is_empty() {
        return String::new();
    }

    let mut output = String::new();
    output.push_str("## Dead Code Detection\n\n");
    output.push_str(&format!(
        "### Unused Functions ({} found)\n\n",
        dead_code_items.len()
    ));

    // Format the table
    let table_content = format_dead_code_table(&dead_code_items);
    output.push_str(&table_content);
    output.push('\n');

    output
}

/// Format the dead code table with headers and rows
fn format_dead_code_table(items: &[&UnifiedDebtItem]) -> String {
    let mut output = String::new();
    let (header, separator) = get_dead_code_table_headers();

    output.push_str(header);
    output.push('\n');
    output.push_str(separator);
    output.push('\n');

    for item in items.iter().take(20) {
        if let Some(row) = format_dead_code_row(item) {
            output.push_str(&row);
            output.push('\n');
        }
    }

    output
}
