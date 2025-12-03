//! Markdown formatting for priority analysis results
//!
//! This module has been refactored (Spec 206) to use shared classification
//! modules and a modular structure. It provides three main formatting modes:
//!
//! - **Mixed priorities**: Top recommendations across all debt types
//! - **Categorical**: Debt grouped by category (Testing, Complexity, etc.)
//! - **Tiered**: Debt grouped by severity tier (Critical, High, Moderate, Low)
//!
//! ## Shared Components (Spec 202)
//!
//! Uses shared classification modules for consistent behavior:
//! - `Severity::from_score()` for severity levels (8.0/6.0/4.0 thresholds)
//! - Consistent with terminal formatter
//!
//! ## Module Structure
//!
//! - `utilities`: Pure logic helpers (categorization, extraction)
//! - `categorical`: Category-based formatting
//! - `tiered`: Tier-based formatting
//! - `priority_item`: Mixed priority item formatting
//! - `details`: Detailed score breakdowns and dependencies

use crate::formatting::FormattingConfig;
use crate::priority::{UnifiedAnalysis, UnifiedAnalysisQueries};
use std::fmt::Write;

// Module declarations
mod categorical;
mod details;
mod priority_item;
mod tiered;
mod utilities;

// Re-export for internal use
use categorical::{
    format_categorical_summary, format_category_section, format_cross_category_dependencies,
};
use priority_item::format_mixed_priority_item_markdown;
use tiered::format_tier_section;

/// Format priorities for markdown output without ANSI color codes
pub fn format_priorities_markdown(
    analysis: &UnifiedAnalysis,
    limit: usize,
    verbosity: u8,
    config: FormattingConfig,
) -> String {
    let mut output = String::new();

    let version = env!("CARGO_PKG_VERSION");
    writeln!(output, "# Debtmap v{}\n", version).unwrap();

    let top_items = analysis.get_top_mixed_priorities(limit);
    let count = top_items.len().min(limit);

    writeln!(output, "## Top {} Recommendations\n", count).unwrap();

    for (idx, item) in top_items.iter().enumerate() {
        format_mixed_priority_item_markdown(&mut output, idx + 1, item, verbosity, &config);
        writeln!(output).unwrap();
    }

    // Add summary
    writeln!(output, "---\n").unwrap();
    writeln!(
        output,
        "**Total Debt Score:** {:.0}",
        analysis.total_debt_score
    )
    .unwrap();

    writeln!(
        output,
        "**Debt Density:** {:.1} per 1K LOC ({} total LOC)",
        analysis.debt_density, analysis.total_lines_of_code
    )
    .unwrap();

    if let Some(coverage) = analysis.overall_coverage {
        writeln!(output, "**Overall Coverage:** {:.2}%", coverage).unwrap();
    }

    output
}

/// Format priorities for markdown output with categorical grouping
pub fn format_priorities_categorical_markdown(
    analysis: &UnifiedAnalysis,
    limit: usize,
    verbosity: u8,
) -> String {
    let mut output = String::new();

    let version = env!("CARGO_PKG_VERSION");
    writeln!(output, "# Debtmap v{}\n", version).unwrap();

    let categorized = analysis.get_categorized_debt(limit);

    writeln!(output, "## Technical Debt Analysis - By Category\n").unwrap();

    // Sort categories by total score (highest first)
    let mut sorted_categories: Vec<_> = categorized.categories.iter().collect();
    sorted_categories.sort_by(|a, b| {
        b.1.total_score
            .partial_cmp(&a.1.total_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Format each category
    for (category, summary) in sorted_categories {
        format_category_section(&mut output, category, summary, verbosity);
    }

    // Add cross-category dependencies if any
    if !categorized.cross_category_dependencies.is_empty() {
        format_cross_category_dependencies(&mut output, &categorized.cross_category_dependencies);
    }

    // Add summary
    format_categorical_summary(&mut output, &categorized);

    writeln!(output).unwrap();
    writeln!(
        output,
        "**Total Debt Score:** {:.0}",
        analysis.total_debt_score
    )
    .unwrap();

    writeln!(
        output,
        "**Debt Density:** {:.1} per 1K LOC ({} total LOC)",
        analysis.debt_density, analysis.total_lines_of_code
    )
    .unwrap();

    if let Some(coverage) = analysis.overall_coverage {
        writeln!(output, "**Overall Coverage:** {:.2}%", coverage).unwrap();
    }

    output
}

/// Format priorities for markdown output with tiered display
pub fn format_priorities_tiered_markdown(
    analysis: &UnifiedAnalysis,
    limit: usize,
    verbosity: u8,
) -> String {
    let mut output = String::new();

    let version = env!("CARGO_PKG_VERSION");
    writeln!(output, "# Debtmap v{}\n", version).unwrap();

    let tiered_display = analysis.get_tiered_display(limit);

    writeln!(output, "## Technical Debt Analysis - Priority Tiers\n").unwrap();

    // Format each tier
    use crate::priority::Tier;
    format_tier_section(
        &mut output,
        &tiered_display.critical,
        Tier::Critical,
        verbosity,
    );
    format_tier_section(&mut output, &tiered_display.high, Tier::High, verbosity);
    format_tier_section(
        &mut output,
        &tiered_display.moderate,
        Tier::Moderate,
        verbosity,
    );
    format_tier_section(&mut output, &tiered_display.low, Tier::Low, verbosity);

    // Add summary
    writeln!(output, "---\n").unwrap();
    writeln!(output, "## Summary\n").unwrap();

    let critical_count: usize = tiered_display.critical.iter().map(|g| g.items.len()).sum();
    let high_count: usize = tiered_display.high.iter().map(|g| g.items.len()).sum();
    let moderate_count: usize = tiered_display.moderate.iter().map(|g| g.items.len()).sum();
    let low_count: usize = tiered_display.low.iter().map(|g| g.items.len()).sum();

    writeln!(
        output,
        "**Total Debt Items:** {}",
        critical_count + high_count + moderate_count + low_count
    )
    .unwrap();
    writeln!(output, "- [CRITICAL] Critical: {} items", critical_count).unwrap();
    writeln!(output, "- [WARN] High: {} items", high_count).unwrap();
    writeln!(output, "- Moderate: {} items", moderate_count).unwrap();
    writeln!(output, "- [INFO] Low: {} items", low_count).unwrap();

    writeln!(output).unwrap();
    writeln!(
        output,
        "**Total Debt Score:** {:.0}",
        analysis.total_debt_score
    )
    .unwrap();

    writeln!(
        output,
        "**Debt Density:** {:.1} per 1K LOC ({} total LOC)",
        analysis.debt_density, analysis.total_lines_of_code
    )
    .unwrap();

    if let Some(coverage) = analysis.overall_coverage {
        writeln!(output, "**Overall Coverage:** {:.2}%", coverage).unwrap();
    }

    output
}
