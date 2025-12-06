//! Terminal formatting for priority analysis results
//!
//! This module provides formatted output for technical debt priorities,
//! including detailed recommendations and summary tables.

use crate::formatting::FormattingConfig;
use crate::priority::{UnifiedAnalysis, UnifiedDebtItem};

use crate::priority::formatter_verbosity as verbosity;

// Submodules (spec 205: organized by responsibility)
mod context;
mod dependencies;
mod helpers;
mod orchestrators;
pub mod pure;
mod recommendations;
mod sections;
pub mod summary;
pub mod writer;

#[derive(Debug, Clone, Copy)]
pub enum OutputFormat {
    Default,     // Top 10 with clean formatting
    Top(usize),  // Top N items
    Tail(usize), // Bottom N items (lowest priority)
}

pub fn format_priorities(analysis: &UnifiedAnalysis, format: OutputFormat) -> String {
    format_priorities_with_verbosity(analysis, format, 0)
}

pub fn format_priorities_with_verbosity(
    analysis: &UnifiedAnalysis,
    format: OutputFormat,
    verbosity: u8,
) -> String {
    format_priorities_with_config(analysis, format, verbosity, FormattingConfig::default())
}

pub fn format_priorities_with_config(
    analysis: &UnifiedAnalysis,
    format: OutputFormat,
    verbosity: u8,
    config: FormattingConfig,
) -> String {
    match format {
        OutputFormat::Default => {
            orchestrators::format_default_with_config(analysis, 10, verbosity, config)
        }
        OutputFormat::Top(n) => {
            orchestrators::format_default_with_config(analysis, n, verbosity, config)
        }
        OutputFormat::Tail(n) => {
            orchestrators::format_tail_with_config(analysis, n, verbosity, config)
        }
    }
}

/// Format priorities with tiered display for terminal output (summary mode)
pub fn format_summary_terminal(analysis: &UnifiedAnalysis, limit: usize, verbosity: u8) -> String {
    summary::format_summary_terminal(analysis, limit, verbosity)
}

// Terminal formatting functions moved to summary.rs

// Unused formatting functions removed (format_tail, format_detailed)

pub fn format_priority_item(
    output: &mut String,
    rank: usize,
    item: &UnifiedDebtItem,
    has_coverage_data: bool,
) {
    // Use pure functional formatting with writer pattern
    let formatted = pure::format_priority_item(
        rank,
        item,
        0, // default verbosity
        FormattingConfig::default(),
        has_coverage_data,
    );

    // Write to output buffer (I/O at edges)
    let mut buffer = Vec::new();
    let _ = writer::write_priority_item(&mut buffer, &formatted);
    if let Ok(result) = String::from_utf8(buffer) {
        output.push_str(&result);
    }
}

// Re-export helper functions from helpers module (spec 205)
pub use helpers::{
    extract_complexity_info, extract_dependency_info, format_debt_type, format_impact, format_role,
};

// Format file-level priority items with detailed information
pub fn format_file_priority_item_with_verbosity(
    output: &mut String,
    rank: usize,
    item: &crate::priority::FileDebtItem,
    _config: FormattingConfig,
    _verbosity: u8,
) {
    use colored::*;
    use std::fmt::Write;

    // Determine severity based on score
    let severity = crate::priority::classification::Severity::from_score(item.score);
    let (severity_label, severity_color) = (severity.as_str(), severity.color());

    // Header section
    writeln!(
        output,
        "#{} {} [{}]",
        rank,
        format!("SCORE: {:.1}", item.score).bright_yellow(),
        severity_label.color(severity_color).bold()
    )
    .unwrap();

    // Location section (file-level)
    writeln!(
        output,
        "{} {}",
        "├─ LOCATION:".bright_blue(),
        item.metrics.path.display()
    )
    .unwrap();

    // Impact section
    writeln!(
        output,
        "{} {}",
        "├─ IMPACT:".bright_blue(),
        format!(
            "-{:.0} complexity, -{:.1} maintainability improvement",
            item.impact.complexity_reduction, item.impact.maintainability_improvement
        )
        .bright_cyan()
    )
    .unwrap();

    // File metrics section
    writeln!(
        output,
        "{} {} lines, {} functions, avg complexity: {:.1}",
        "├─ METRICS:".bright_blue(),
        item.metrics.total_lines,
        item.metrics.function_count,
        item.metrics.avg_complexity
    )
    .unwrap();

    // God object details (if applicable)
    if let Some(ref god_analysis) = item.metrics.god_object_analysis {
        if god_analysis.is_god_object {
            writeln!(
                output,
                "{} {} methods, {} fields, {} responsibilities (score: {:.1})",
                "├─ GOD OBJECT:".bright_blue(),
                god_analysis.method_count,
                god_analysis.field_count,
                god_analysis.responsibility_count,
                god_analysis.god_object_score
            )
            .unwrap();

            // Show recommended splits if available
            if !god_analysis.recommended_splits.is_empty() {
                writeln!(
                    output,
                    "   {} {} recommended module splits",
                    "Suggested:".dimmed(),
                    god_analysis.recommended_splits.len()
                )
                .unwrap();
            }
        }
    }

    // Action section
    writeln!(
        output,
        "{} {}",
        "├─ ACTION:".bright_blue(),
        item.recommendation.bright_yellow()
    )
    .unwrap();

    // Rationale section
    let rationale = format_file_rationale(item);
    writeln!(
        output,
        "{} {}",
        "└─ WHY THIS MATTERS:".bright_blue(),
        rationale
    )
    .unwrap();
}

/// Generate rationale explaining why this file-level debt matters
fn format_file_rationale(item: &crate::priority::FileDebtItem) -> String {
    if let Some(ref god_analysis) = item.metrics.god_object_analysis {
        if god_analysis.is_god_object {
            let responsibilities = god_analysis.responsibility_count;
            let methods = god_analysis.method_count;

            if responsibilities > 5 {
                return format!(
                    "File has {} distinct responsibilities across {} methods. High coupling makes changes risky and testing difficult. Splitting by responsibility will improve maintainability and reduce change impact.",
                    responsibilities, methods
                );
            } else if methods > 50 {
                return format!(
                    "File contains {} methods with {} responsibilities. Large interface makes it difficult to understand and maintain. Extracting cohesive modules will improve clarity.",
                    methods, responsibilities
                );
            } else {
                return format!(
                    "File exhibits god object characteristics (score: {:.1}). Refactoring will improve separation of concerns and testability.",
                    god_analysis.god_object_score
                );
            }
        }
    }

    if item.metrics.total_complexity > 500 {
        format!(
            "High total complexity ({}) across {} functions (avg: {:.1}). Breaking into smaller modules will reduce cognitive load and improve maintainability.",
            item.metrics.total_complexity,
            item.metrics.function_count,
            item.metrics.avg_complexity
        )
    } else if item.metrics.total_lines > 1000 {
        format!(
            "Large file ({} lines) with {} functions. Size alone increases maintenance burden and makes navigation difficult.",
            item.metrics.total_lines,
            item.metrics.function_count
        )
    } else {
        "File-level refactoring will improve overall code organization and maintainability."
            .to_string()
    }
}
