//! Orchestration functions for formatting workflows
//!
//! This module contains thin wrapper functions that coordinate between
//! the public API and specialized formatting modules.

use crate::formatting::FormattingConfig;
use crate::priority::{UnifiedAnalysis, UnifiedAnalysisQueries};
use colored::*;
use std::fmt::Write;

use super::{recommendations, verbosity};

/// Format with default configuration and specified verbosity
pub(super) fn format_default_with_verbosity(
    analysis: &UnifiedAnalysis,
    limit: usize,
    verbosity: u8,
) -> String {
    format_default_with_config(analysis, limit, verbosity, FormattingConfig::default())
}

/// Format with full configuration options
pub(super) fn format_default_with_config(
    analysis: &UnifiedAnalysis,
    limit: usize,
    verbosity: u8,
    config: FormattingConfig,
) -> String {
    recommendations::format_default_with_config(analysis, limit, verbosity, config)
}

/// Format default output (top N items)
#[allow(dead_code)]
pub(super) fn format_default(analysis: &UnifiedAnalysis, limit: usize) -> String {
    format_default_with_verbosity(analysis, limit, 0)
}

/// Format tail (bottom N items) with verbosity
#[allow(dead_code)]
pub(super) fn format_tail_with_verbosity(
    analysis: &UnifiedAnalysis,
    n: usize,
    verbosity: u8,
) -> String {
    format_tail_with_config(analysis, n, verbosity, FormattingConfig::default())
}

/// Format tail (bottom N items) with full configuration
pub(super) fn format_tail_with_config(
    analysis: &UnifiedAnalysis,
    n: usize,
    verbosity: u8,
    config: FormattingConfig,
) -> String {
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

    let tail_items = analysis.get_bottom_priorities(n);
    let start_rank = (analysis.items.len() - tail_items.len()) + 1;

    for (idx, item) in tail_items.iter().enumerate() {
        verbosity::format_priority_item_with_config(
            &mut output,
            start_rank + idx,
            item,
            verbosity,
            config,
            analysis.has_coverage_data,
        );
        writeln!(output).unwrap();
    }

    output
}
