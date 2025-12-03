//! Summary table formatting for priority analysis
//!
//! This module contains functions for generating summary views of technical
//! debt analysis, including tiered displays and compact item formatting.

use crate::formatting::{ColoredFormatter, FormattingConfig};
use crate::priority::{self, DisplayGroup, Tier, UnifiedAnalysis, UnifiedAnalysisQueries};
use colored::*;
use std::fmt::Write;

/// Format priorities with tiered display for terminal output (summary mode)
pub fn format_summary_terminal(analysis: &UnifiedAnalysis, limit: usize, verbosity: u8) -> String {
    format_tiered_terminal(analysis, limit, verbosity, FormattingConfig::default())
}

/// Internal implementation of tiered display for terminal output
fn format_tiered_terminal(
    analysis: &UnifiedAnalysis,
    limit: usize,
    verbosity: u8,
    config: FormattingConfig,
) -> String {
    let mut output = String::new();
    let version = env!("CARGO_PKG_VERSION");
    let _formatter = ColoredFormatter::new(config);

    // Header
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

    // Get tiered display
    let tiered_display = analysis.get_tiered_display(limit);

    writeln!(
        output,
        "{}",
        "TECHNICAL DEBT ANALYSIS - PRIORITY TIERS"
            .bright_yellow()
            .bold()
    )
    .unwrap();
    writeln!(output).unwrap();

    // Format each tier
    format_tier_terminal(
        &mut output,
        &tiered_display.critical,
        Tier::Critical,
        verbosity,
        config,
    );
    format_tier_terminal(
        &mut output,
        &tiered_display.high,
        Tier::High,
        verbosity,
        config,
    );
    format_tier_terminal(
        &mut output,
        &tiered_display.moderate,
        Tier::Moderate,
        verbosity,
        config,
    );
    format_tier_terminal(
        &mut output,
        &tiered_display.low,
        Tier::Low,
        verbosity,
        config,
    );

    // Summary section
    let critical_count: usize = tiered_display.critical.iter().map(|g| g.items.len()).sum();
    let high_count: usize = tiered_display.high.iter().map(|g| g.items.len()).sum();
    let moderate_count: usize = tiered_display.moderate.iter().map(|g| g.items.len()).sum();
    let low_count: usize = tiered_display.low.iter().map(|g| g.items.len()).sum();

    writeln!(output, "{}", divider.bright_blue()).unwrap();
    writeln!(output, "[SUMMARY] DEBT DISTRIBUTION").unwrap();

    if critical_count > 0 {
        writeln!(
            output,
            "  [!] Critical: {} items",
            critical_count.to_string().bright_red()
        )
        .unwrap();
    }
    if high_count > 0 {
        writeln!(
            output,
            "  [*] High: {} items",
            high_count.to_string().bright_yellow()
        )
        .unwrap();
    }
    if moderate_count > 0 {
        writeln!(
            output,
            "  [+] Moderate: {} items",
            moderate_count.to_string().bright_blue()
        )
        .unwrap();
    }
    if low_count > 0 {
        writeln!(output, "  [-] Low: {} items", low_count.to_string().white()).unwrap();
    }

    writeln!(output).unwrap();
    writeln!(
        output,
        "[TOTAL] {}",
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
                "[COVERAGE] {}",
                format!("OVERALL COVERAGE: {:.2}%", coverage).bright_green()
            )
            .unwrap();
        }
    }

    output
}

/// Format a single tier for terminal output
fn format_tier_terminal(
    output: &mut String,
    groups: &[DisplayGroup],
    tier: Tier,
    verbosity: u8,
    config: FormattingConfig,
) {
    if groups.is_empty() {
        return;
    }

    let _formatter = ColoredFormatter::new(config);

    // Tier header with color based on tier level
    let tier_header = match tier {
        Tier::Critical => format!(
            "{} {} - {}",
            "[CRITICAL]",
            "CRITICAL".bright_red().bold(),
            "Immediate Action Required".red()
        ),
        Tier::High => format!(
            "{} {} - {}",
            "[HIGH]",
            "HIGH PRIORITY",
            "Current Sprint".yellow()
        ),
        Tier::Moderate => format!(
            "{} {} - {}",
            "[MODERATE]",
            "MODERATE".bright_blue().bold(),
            "Next Sprint".blue()
        ),
        Tier::Low => format!(
            "{} {} - {}",
            "[LOW]",
            "LOW".white().bold(),
            "Backlog".white()
        ),
    };

    writeln!(output, "{}", tier_header).unwrap();
    writeln!(output, "{}", tier.effort_estimate()).unwrap();
    writeln!(output).unwrap();

    let max_items_per_tier = if verbosity >= 2 { 999 } else { 5 };
    let mut items_shown = 0;

    for group in groups {
        if items_shown >= max_items_per_tier {
            let remaining: usize = groups.iter().skip(items_shown).map(|g| g.items.len()).sum();
            if remaining > 0 {
                writeln!(
                    output,
                    "  [+] ... and {} more items in this tier",
                    remaining
                )
                .unwrap();
            }
            break;
        }

        format_display_group_terminal(output, group, &mut items_shown, verbosity, config);
    }

    writeln!(output).unwrap();
}

/// Format a display group for terminal output
fn format_display_group_terminal(
    output: &mut String,
    group: &DisplayGroup,
    items_shown: &mut usize,
    verbosity: u8,
    config: FormattingConfig,
) {
    let _formatter = ColoredFormatter::new(config);

    if group.items.len() > 1 && group.batch_action.is_some() {
        // Grouped similar items
        writeln!(
            output,
            "  [GROUP] {} ({} similar items)",
            group.debt_type.bright_cyan(),
            group.items.len().to_string().yellow()
        )
        .unwrap();

        if let Some(action) = &group.batch_action {
            writeln!(output, "    -> {}", action.green()).unwrap();
        }

        // Show first item as example if verbose
        if verbosity >= 1 && !group.items.is_empty() {
            writeln!(
                output,
                "    [eg] Example: {}",
                format_item_location(&group.items[0])
            )
            .unwrap();
        }

        *items_shown += group.items.len();
    } else {
        // Individual items
        for item in &group.items {
            if *items_shown >= 5 && verbosity < 2 {
                return;
            }

            // Use compact format for tiered display
            format_compact_item(output, *items_shown + 1, item, verbosity, config);
            *items_shown += 1;
        }
    }
}

/// Format an item in compact mode for tiered display
fn format_compact_item(
    output: &mut String,
    index: usize,
    item: &priority::DebtItem,
    verbosity: u8,
    config: FormattingConfig,
) {
    let _formatter = ColoredFormatter::new(config);

    match item {
        priority::DebtItem::Function(func) => {
            writeln!(
                output,
                "  > #{} [{}] {}:{} {}",
                index,
                format!("{:.1}", func.unified_score.final_score).yellow(),
                func.location.file.display(),
                func.location.line,
                func.location.function.bright_green()
            )
            .unwrap();

            // Show brief action
            writeln!(
                output,
                "      -> {}",
                func.recommendation.primary_action.green()
            )
            .unwrap();
        }
        priority::DebtItem::File(file) => {
            writeln!(
                output,
                "  [F] #{} [{}] {} ({} lines)",
                index,
                format!("{:.1}", file.score).yellow(),
                file.metrics.path.display(),
                file.metrics.total_lines
            )
            .unwrap();

            // Show brief action from recommendation
            if !file.recommendation.is_empty() {
                writeln!(output, "      -> {}", file.recommendation.green()).unwrap();
            }
        }
    }

    // Show additional details if verbose
    if verbosity >= 2 {
        match item {
            priority::DebtItem::Function(func) => {
                if let Some(ref pattern) = func.detected_pattern {
                    writeln!(output, "        Pattern: {:?}", pattern.pattern_type).unwrap();
                }
            }
            priority::DebtItem::File(_) => {
                // File details shown inline above
            }
        }
    }
}

/// Format location of a debt item for display
fn format_item_location(item: &priority::DebtItem) -> String {
    match item {
        priority::DebtItem::Function(func) => {
            format!(
                "{}:{} {}",
                func.location.file.display(),
                func.location.line,
                func.location.function
            )
        }
        priority::DebtItem::File(file) => {
            format!(
                "{} ({} lines)",
                file.metrics.path.display(),
                file.metrics.total_lines
            )
        }
    }
}
