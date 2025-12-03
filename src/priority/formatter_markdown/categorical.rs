//! Categorical formatting for markdown output
//!
//! Formats technical debt analysis results grouped by category
//! (Testing, Complexity, Organization, etc.)

use crate::priority::{
    CategorizedDebt, CategorySummary, CrossCategoryDependency, DebtCategory, DebtItem, ImpactLevel,
};
use std::fmt::Write;

use super::utilities::format_debt_type;

pub(crate) fn format_category_section(
    output: &mut String,
    category: &DebtCategory,
    summary: &CategorySummary,
    verbosity: u8,
) {
    writeln!(
        output,
        "### {} {} ({} items)",
        category.icon(),
        category.name(),
        summary.item_count
    )
    .unwrap();

    writeln!(
        output,
        "**Total Score:** {:.1} | **Average Severity:** {:.1}",
        summary.total_score, summary.average_severity
    )
    .unwrap();

    writeln!(output).unwrap();
    writeln!(
        output,
        "{}",
        category.strategic_guidance(summary.item_count, summary.estimated_effort_hours)
    )
    .unwrap();
    writeln!(output).unwrap();

    // Show top items in this category
    if !summary.top_items.is_empty() {
        writeln!(output, "#### Top Priority Items").unwrap();
        writeln!(output).unwrap();

        for (idx, item) in summary.top_items.iter().take(3).enumerate() {
            format_categorized_debt_item(output, idx + 1, item, verbosity);
        }

        if summary.item_count > summary.top_items.len() {
            writeln!(
                output,
                "\n_... and {} more items in this category_",
                summary.item_count - summary.top_items.len()
            )
            .unwrap();
        }
    }

    writeln!(output).unwrap();
}

pub(crate) fn format_categorized_debt_item(
    output: &mut String,
    rank: usize,
    item: &DebtItem,
    verbosity: u8,
) {
    match item {
        DebtItem::Function(func) => {
            writeln!(
                output,
                "{}. **{}** - Score: {:.1}",
                rank, func.location.function, func.unified_score.final_score
            )
            .unwrap();
            writeln!(
                output,
                "   - Location: `{}:{}`",
                func.location.file.display(),
                func.location.line
            )
            .unwrap();
            writeln!(output, "   - Type: {}", format_debt_type(&func.debt_type)).unwrap();
            if verbosity >= 1 {
                writeln!(
                    output,
                    "   - Action: {}",
                    func.recommendation.primary_action
                )
                .unwrap();
            }
        }
        DebtItem::File(file) => {
            let file_name = file
                .metrics
                .path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown");
            writeln!(
                output,
                "{}. **{}** - Score: {:.1}",
                rank, file_name, file.score
            )
            .unwrap();
            writeln!(output, "   - Path: `{}`", file.metrics.path.display()).unwrap();
            writeln!(
                output,
                "   - Metrics: {} lines, {} functions",
                file.metrics.total_lines, file.metrics.function_count
            )
            .unwrap();
            if verbosity >= 1 {
                writeln!(output, "   - Action: {}", file.recommendation).unwrap();
            }
        }
    }
}

pub(crate) fn format_cross_category_dependencies(
    output: &mut String,
    dependencies: &[CrossCategoryDependency],
) {
    writeln!(output, "### [PERF] Cross-Category Dependencies\n").unwrap();
    writeln!(
        output,
        "These relationships affect how you should prioritize improvements:\n"
    )
    .unwrap();

    for dep in dependencies {
        let impact_symbol = match dep.impact_level {
            ImpactLevel::Critical => "[ERROR]",
            ImpactLevel::High => "[WARN]",
            ImpactLevel::Medium => "[WARN]",
            ImpactLevel::Low => "[OK]",
        };

        writeln!(
            output,
            "{} **{} → {}**: {}",
            impact_symbol,
            dep.source_category.name(),
            dep.target_category.name(),
            dep.description
        )
        .unwrap();
    }
    writeln!(output).unwrap();
}

pub(crate) fn format_categorical_summary(output: &mut String, categorized: &CategorizedDebt) {
    writeln!(output, "---\n").unwrap();
    writeln!(output, "## Summary by Category\n").unwrap();

    let total_items: usize = categorized.categories.values().map(|c| c.item_count).sum();
    let total_effort: u32 = categorized
        .categories
        .values()
        .map(|c| c.estimated_effort_hours)
        .sum();

    writeln!(output, "**Total Debt Items:** {}", total_items).unwrap();
    writeln!(output, "**Total Estimated Effort:** {} hours", total_effort).unwrap();
    writeln!(output).unwrap();

    writeln!(
        output,
        "| Category | Items | Total Score | Effort (hours) |"
    )
    .unwrap();
    writeln!(output, "|----------|-------|------------|----------------|").unwrap();

    for (category, summary) in &categorized.categories {
        writeln!(
            output,
            "| {} {} | {} | {:.1} | {} |",
            category.icon(),
            category.name(),
            summary.item_count,
            summary.total_score,
            summary.estimated_effort_hours
        )
        .unwrap();
    }
}
