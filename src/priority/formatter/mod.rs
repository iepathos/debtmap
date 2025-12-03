//! Terminal formatting for priority analysis results
//!
//! This module provides formatted output for technical debt priorities,
//! including detailed recommendations and summary tables.

use crate::formatting::{ColoredFormatter, FormattingConfig};
use crate::output::evidence_formatter::EvidenceFormatter;
use crate::priority::classification::Severity;
use crate::priority::{
    self, score_formatter, DebtType, DisplayGroup, FunctionRole, Tier, UnifiedAnalysis,
    UnifiedAnalysisQueries, UnifiedDebtItem,
};
use colored::*;
use std::fmt::Write;

#[path = "../formatter_verbosity.rs"]
mod verbosity;

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

use context::create_format_context;
#[allow(deprecated)]
use sections::{apply_formatted_sections, generate_formatted_sections};

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
        OutputFormat::Default => orchestrators::format_default_with_config(analysis, 10, verbosity, config),
        OutputFormat::Top(n) => orchestrators::format_default_with_config(analysis, n, verbosity, config),
        OutputFormat::Tail(n) => orchestrators::format_tail_with_config(analysis, n, verbosity, config),
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
    // Use functional composition to format different sections
    let format_context = create_format_context(rank, item, has_coverage_data);

    // Format each section using pure functions composed together
    let formatted_sections = generate_formatted_sections(&format_context);

    // Apply formatting to output (I/O at edges)
    #[allow(deprecated)]
    apply_formatted_sections(output, formatted_sections);
}

/// Legacy API for backward compatibility. Use the new pure functions instead.
///
/// # Deprecated
/// This function mixes formatting logic with I/O operations. For new code, use:
/// ```ignore
/// use crate::priority::formatter::pure::format_priority_item;
/// use crate::priority::formatter::writer::write_priority_item;
/// use crate::formatting::FormattingConfig;
/// use std::io::Write;
///
/// let formatted = pure::format_priority_item(rank, item, 0, FormattingConfig::default(), has_coverage_data);
/// let mut buffer = Vec::new();
/// write_priority_item(&mut buffer, &formatted)?;
/// let output = String::from_utf8(buffer)?;
/// ```
#[deprecated(
    since = "0.1.0",
    note = "Use pure::format_priority_item + writer::write_priority_item for testable, functional code"
)]
pub fn format_priority_item_legacy(
    output: &mut String,
    rank: usize,
    item: &UnifiedDebtItem,
    has_coverage_data: bool,
) {
    // Wrap pure format + write operations for backward compatibility
    let formatted = pure::format_priority_item(
        rank,
        item,
        0, // default verbosity
        FormattingConfig::default(),
        has_coverage_data,
    );

    // Convert String to Write-compatible buffer
    let mut buffer = Vec::new();
    let _ = writer::write_priority_item(&mut buffer, &formatted);
    if let Ok(result) = String::from_utf8(buffer) {
        output.push_str(&result);
    }
}

// Re-export helper functions from helpers module (spec 205)
pub use helpers::{
    extract_complexity_info, extract_dependency_info, format_debt_type, format_impact,
    format_role, get_severity_color, get_severity_label,
};

// Stub function needed by recommendations.rs (implementation delegated to sections module)
pub fn format_file_priority_item_with_verbosity(
    output: &mut String,
    rank: usize,
    item: &crate::priority::FileDebtItem,
    config: FormattingConfig,
    verbosity: u8,
) {
    use crate::priority::score_formatter;
    use std::fmt::Write;

    writeln!(
        output,
        "#{} {} - Score: {:.1}",
        rank,
        item.metrics.path.display(),
        item.score
    ).unwrap();

    if verbosity >= 1 {
        writeln!(output, "  {} lines, {} functions", item.metrics.total_lines, item.metrics.function_count).unwrap();
    }
}

