//! Detailed recommendations formatting for priority analysis
//!
//! This module provides formatted recommendations for technical debt items,
//! showing detailed information about each priority item with context.

use crate::formatting::FormattingConfig;
use crate::priority::{self, UnifiedAnalysis, UnifiedAnalysisQueries};
use colored::*;
use std::fmt::Write;

use super::verbosity;

/// Format legend explaining header tags
/// Displayed once at the start of recommendations when verbosity >= 1
fn generate_legend(verbosity: u8, has_coverage_data: bool) -> String {
    if verbosity == 0 || !has_coverage_data {
        return String::new();
    }

    let mut legend = String::new();
    writeln!(legend, "{}", "Legend:".bright_white().bold()).unwrap();
    writeln!(
        legend,
        "  {} Numeric priority (higher = more important)",
        "SCORE:".bright_yellow()
    )
    .unwrap();

    if has_coverage_data {
        writeln!(
            legend,
            "  {} Coverage status (how well tested)",
            "[ERROR/WARN/INFO/OK]:".bright_cyan()
        )
        .unwrap();
    }

    writeln!(
        legend,
        "  {} Item severity (fix urgency)",
        "[CRITICAL/HIGH/MEDIUM/LOW]:".bright_magenta()
    )
    .unwrap();
    writeln!(legend).unwrap();

    legend
}

/// Format top priority items with detailed recommendations
pub fn format_default_with_config(
    analysis: &UnifiedAnalysis,
    limit: usize,
    verbosity: u8,
    config: FormattingConfig,
) -> String {
    // Check if summary mode is explicitly requested
    // TODO: Add --summary flag to CLI to enable this
    // For now, always use detailed format to preserve existing functionality
    let mut output = String::new();
    let version = env!("CARGO_PKG_VERSION");

    let divider = "=".repeat(44);
    writeln!(output, "{}", divider.bright_blue()).unwrap();
    writeln!(
        output,
        "    {}",
        format!("Debtmap v{}", version).bright_white().bold()
    )
    .unwrap();
    writeln!(output, "{}", divider.bright_blue()).unwrap();
    writeln!(output).unwrap();

    let top_items = analysis.get_top_mixed_priorities(limit);
    let count = top_items.len().min(limit);
    writeln!(
        output,
        "{}",
        format!("TOP {count} RECOMMENDATIONS")
            .bright_yellow()
            .bold()
    )
    .unwrap();
    writeln!(output).unwrap();

    // Add legend if verbosity >= 1 and coverage data is available
    let legend = generate_legend(verbosity, analysis.has_coverage_data);
    if !legend.is_empty() {
        output.push_str(&legend);
    }

    for (idx, item) in top_items.iter().enumerate() {
        format_mixed_priority_item(
            &mut output,
            idx + 1,
            item,
            verbosity,
            config,
            analysis.has_coverage_data,
        );
        writeln!(output).unwrap();
    }

    // Add summary
    writeln!(
        output,
        "{}",
        format!("TOTAL DEBT SCORE: {:.0}", analysis.total_debt_score).bright_cyan()
    )
    .unwrap();

    writeln!(
        output,
        "{}",
        format!(
            "DEBT DENSITY: {:.1} per 1K LOC ({} total LOC)",
            analysis.debt_density, analysis.total_lines_of_code
        )
        .bright_yellow()
    )
    .unwrap();

    // Only show overall coverage if coverage data was provided (spec 108)
    if analysis.has_coverage_data {
        if let Some(coverage) = analysis.overall_coverage {
            writeln!(
                output,
                "{}",
                format!("OVERALL COVERAGE: {:.2}%", coverage).bright_green()
            )
            .unwrap();
        }
    }

    output
}

/// Format a single mixed priority item (function or file)
fn format_mixed_priority_item(
    output: &mut String,
    rank: usize,
    item: &priority::DebtItem,
    verbosity: u8,
    config: FormattingConfig,
    has_coverage_data: bool,
) {
    match item {
        priority::DebtItem::Function(func_item) => {
            verbosity::format_priority_item_with_config(
                output,
                rank,
                func_item,
                verbosity,
                config,
                has_coverage_data,
            );
        }
        priority::DebtItem::File(file_item) => {
            // Call the parent module's function
            crate::priority::formatter::format_file_priority_item_with_verbosity(
                output, rank, file_item, config, verbosity,
            );
        }
    }
}
