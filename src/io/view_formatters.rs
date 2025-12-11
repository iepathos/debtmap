//! Output formatters that consume PreparedDebtView.
//!
//! This module implements spec 252: Output Format Unification.
//! All formatters accept a `PreparedDebtView` and produce formatted output.
//!
//! # Architecture
//!
//! ```text
//! UnifiedAnalysis → prepare_view() → PreparedDebtView → formatter → output
//!                                          │
//!                                          ├→ format_terminal()
//!                                          ├→ format_json()
//!                                          └→ format_markdown()
//! ```
//!
//! # Benefits
//!
//! - Single view preparation, multiple output formats
//! - Consistent data across all outputs
//! - No duplicate filtering logic
//! - Pure transformation from view to string/struct

use crate::priority::view::{PreparedDebtView, ViewItem, ViewSummary};
use serde::{Deserialize, Serialize};

// ============================================================================
// JSON OUTPUT FORMAT
// ============================================================================

/// JSON output structure for PreparedDebtView.
///
/// Provides structured JSON output with metadata, summary, and items.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonOutput {
    pub format_version: String,
    pub metadata: JsonMetadata,
    pub summary: JsonSummary,
    pub items: Vec<JsonItem>,
}

/// Metadata about the analysis run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonMetadata {
    pub debtmap_version: String,
    pub generated_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_root: Option<String>,
}

/// Summary statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonSummary {
    pub total_items: usize,
    pub total_items_before_filter: usize,
    pub total_debt_score: f64,
    pub debt_density: f64,
    pub total_lines_of_code: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub overall_coverage: Option<f64>,
    pub score_distribution: JsonScoreDistribution,
    pub category_counts: JsonCategoryCounts,
}

/// Score distribution by severity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonScoreDistribution {
    pub critical: usize,
    pub high: usize,
    pub medium: usize,
    pub low: usize,
}

/// Item counts by category.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonCategoryCounts {
    pub architecture: usize,
    pub testing: usize,
    pub performance: usize,
    pub code_quality: usize,
}

/// JSON representation of a debt item.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum JsonItem {
    Function(Box<JsonFunctionItem>),
    File(Box<JsonFileItem>),
}

/// Function-level debt item in JSON format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonFunctionItem {
    pub score: f64,
    pub severity: String,
    pub category: String,
    pub location: JsonLocation,
    pub metrics: JsonFunctionMetrics,
    pub recommendation: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tier: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scoring_details: Option<JsonScoringDetails>,
}

/// File-level debt item in JSON format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonFileItem {
    pub score: f64,
    pub severity: String,
    pub category: String,
    pub location: JsonLocation,
    pub metrics: JsonFileMetrics,
    pub recommendation: String,
}

/// Unified location structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonLocation {
    pub file: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function: Option<String>,
}

/// Function metrics in JSON format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonFunctionMetrics {
    pub cyclomatic_complexity: u32,
    pub cognitive_complexity: u32,
    pub function_length: usize,
    pub nesting_depth: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub coverage: Option<f64>,
}

/// File metrics in JSON format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonFileMetrics {
    pub total_lines: usize,
    pub function_count: usize,
    pub avg_complexity: f64,
    pub max_complexity: u32,
    pub coverage_percent: f64,
}

/// Scoring details for verbose output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonScoringDetails {
    pub complexity_factor: f64,
    pub coverage_factor: f64,
    pub dependency_factor: f64,
    pub role_multiplier: f64,
    pub final_score: f64,
}

// ============================================================================
// FORMAT FUNCTIONS
// ============================================================================

/// Formats a PreparedDebtView as JSON.
///
/// # Arguments
///
/// * `view` - The prepared view to format
/// * `include_scoring_details` - Whether to include detailed scoring breakdown
///
/// # Returns
///
/// JSON string representation of the view.
pub fn format_json(view: &PreparedDebtView, include_scoring_details: bool) -> String {
    let output = to_json_output(view, include_scoring_details);
    serde_json::to_string_pretty(&output).unwrap_or_else(|_| "{}".to_string())
}

/// Converts PreparedDebtView to JsonOutput structure.
pub fn to_json_output(view: &PreparedDebtView, include_scoring_details: bool) -> JsonOutput {
    JsonOutput {
        format_version: "3.0".to_string(),
        metadata: JsonMetadata {
            debtmap_version: env!("CARGO_PKG_VERSION").to_string(),
            generated_at: chrono::Utc::now().to_rfc3339(),
            project_root: None,
        },
        summary: convert_summary(&view.summary),
        items: view
            .items
            .iter()
            .map(|item| convert_item(item, include_scoring_details))
            .collect(),
    }
}

fn convert_summary(summary: &ViewSummary) -> JsonSummary {
    JsonSummary {
        total_items: summary.total_items_after_filter,
        total_items_before_filter: summary.total_items_before_filter,
        total_debt_score: summary.total_debt_score,
        debt_density: summary.debt_density,
        total_lines_of_code: summary.total_lines_of_code,
        overall_coverage: summary.overall_coverage,
        score_distribution: JsonScoreDistribution {
            critical: summary.score_distribution.critical,
            high: summary.score_distribution.high,
            medium: summary.score_distribution.medium,
            low: summary.score_distribution.low,
        },
        category_counts: JsonCategoryCounts {
            architecture: summary.category_counts.architecture,
            testing: summary.category_counts.testing,
            performance: summary.category_counts.performance,
            code_quality: summary.category_counts.code_quality,
        },
    }
}

fn convert_item(item: &ViewItem, include_scoring_details: bool) -> JsonItem {
    match item {
        ViewItem::Function(func) => {
            let loc = item.location();
            JsonItem::Function(Box::new(JsonFunctionItem {
                score: func.unified_score.final_score.value(),
                severity: item.severity().as_str().to_lowercase(),
                category: item.category().to_string(),
                location: JsonLocation {
                    file: loc.file.to_string_lossy().to_string(),
                    line: Some(loc.line.unwrap_or(0)),
                    function: loc.function.clone(),
                },
                metrics: JsonFunctionMetrics {
                    cyclomatic_complexity: func.cyclomatic_complexity,
                    cognitive_complexity: func.cognitive_complexity,
                    function_length: func.function_length,
                    nesting_depth: func.nesting_depth,
                    coverage: func.transitive_coverage.as_ref().map(|c| c.direct),
                },
                recommendation: func.recommendation.primary_action.clone(),
                tier: func
                    .tier
                    .as_ref()
                    .map(|t| format!("{:?}", t).to_lowercase()),
                scoring_details: if include_scoring_details {
                    Some(JsonScoringDetails {
                        complexity_factor: func.unified_score.complexity_factor,
                        coverage_factor: func.unified_score.coverage_factor,
                        dependency_factor: func.unified_score.dependency_factor,
                        role_multiplier: func.unified_score.role_multiplier,
                        final_score: func.unified_score.final_score.value(),
                    })
                } else {
                    None
                },
            }))
        }
        ViewItem::File(file) => {
            let loc = item.location();
            JsonItem::File(Box::new(JsonFileItem {
                score: file.score,
                severity: item.severity().as_str().to_lowercase(),
                category: item.category().to_string(),
                location: JsonLocation {
                    file: loc.file.to_string_lossy().to_string(),
                    line: None,
                    function: None,
                },
                metrics: JsonFileMetrics {
                    total_lines: file.metrics.total_lines,
                    function_count: file.metrics.function_count,
                    avg_complexity: file.metrics.avg_complexity,
                    max_complexity: file.metrics.max_complexity,
                    coverage_percent: file.metrics.coverage_percent,
                },
                recommendation: file.recommendation.clone(),
            }))
        }
    }
}

// ============================================================================
// TERMINAL FORMAT
// ============================================================================

/// Configuration for terminal output.
pub struct TerminalConfig {
    pub verbosity: u8,
    pub use_color: bool,
    pub summary_mode: bool,
}

impl Default for TerminalConfig {
    fn default() -> Self {
        Self {
            verbosity: 0,
            use_color: true,
            summary_mode: false,
        }
    }
}

/// Formats a PreparedDebtView for terminal output.
///
/// # Arguments
///
/// * `view` - The prepared view to format
/// * `config` - Terminal formatting configuration
///
/// # Returns
///
/// Formatted string for terminal display.
pub fn format_terminal(view: &PreparedDebtView, config: &TerminalConfig) -> String {
    use std::fmt::Write;
    let mut output = String::new();

    // Header
    writeln!(
        output,
        "\n═══════════════════════════════════════════════════════════════════════════════"
    )
    .ok();
    writeln!(output, "                           TECHNICAL DEBT ANALYSIS").ok();
    writeln!(
        output,
        "═══════════════════════════════════════════════════════════════════════════════\n"
    )
    .ok();

    // Summary
    format_terminal_summary(&mut output, &view.summary);

    if view.is_empty() {
        writeln!(
            output,
            "\nNo technical debt items found matching current thresholds."
        )
        .ok();
        return output;
    }

    // Items
    if config.summary_mode {
        format_terminal_summary_mode(&mut output, view);
    } else {
        format_terminal_items(&mut output, view, config.verbosity);
    }

    output
}

fn format_terminal_summary(output: &mut String, summary: &ViewSummary) {
    use std::fmt::Write;

    writeln!(output, "Summary:").ok();
    writeln!(
        output,
        "  Total items: {} (of {} analyzed)",
        summary.total_items_after_filter, summary.total_items_before_filter
    )
    .ok();
    writeln!(
        output,
        "  Total debt score: {:.1}",
        summary.total_debt_score
    )
    .ok();
    writeln!(
        output,
        "  Debt density: {:.2} per 1k LOC",
        summary.debt_density
    )
    .ok();
    if let Some(coverage) = summary.overall_coverage {
        writeln!(output, "  Overall coverage: {:.1}%", coverage * 100.0).ok();
    }
    writeln!(output).ok();

    writeln!(
        output,
        "  By severity: {} critical, {} high, {} medium, {} low",
        summary.score_distribution.critical,
        summary.score_distribution.high,
        summary.score_distribution.medium,
        summary.score_distribution.low
    )
    .ok();
    writeln!(output).ok();
}

fn format_terminal_summary_mode(output: &mut String, view: &PreparedDebtView) {
    use std::fmt::Write;

    // Group by tier and show counts
    let mut tier_counts: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();

    for item in &view.items {
        let tier = item
            .tier()
            .map(|t| format!("{:?}", t))
            .unwrap_or_else(|| "Unclassified".to_string());
        *tier_counts.entry(tier).or_insert(0) += 1;
    }

    writeln!(output, "Items by Recommendation Tier:").ok();
    for (tier, count) in &tier_counts {
        writeln!(output, "  {}: {}", tier, count).ok();
    }
}

fn format_terminal_items(output: &mut String, view: &PreparedDebtView, verbosity: u8) {
    use std::fmt::Write;

    writeln!(
        output,
        "───────────────────────────────────────────────────────────────────────────────"
    )
    .ok();
    writeln!(output, "Top Priority Items:").ok();
    writeln!(
        output,
        "───────────────────────────────────────────────────────────────────────────────\n"
    )
    .ok();

    for (i, item) in view.items.iter().enumerate() {
        format_terminal_item(output, item, i + 1, verbosity);
    }
}

fn format_terminal_item(output: &mut String, item: &ViewItem, rank: usize, verbosity: u8) {
    use std::fmt::Write;

    let loc = item.location();
    let severity = item.severity().as_str().to_uppercase();

    writeln!(
        output,
        "{}. [{}] {:.1} - {}",
        rank,
        severity,
        item.score(),
        loc.file.display()
    )
    .ok();

    if let Some(func) = &loc.function {
        writeln!(
            output,
            "   Function: {} (line {})",
            func,
            loc.line.unwrap_or(0)
        )
        .ok();
    }

    match item {
        ViewItem::Function(f) => {
            writeln!(
                output,
                "   Complexity: cyclomatic={}, cognitive={}, nesting={}",
                f.cyclomatic_complexity, f.cognitive_complexity, f.nesting_depth
            )
            .ok();
            if verbosity >= 1 {
                writeln!(
                    output,
                    "   Recommendation: {}",
                    f.recommendation.primary_action
                )
                .ok();
            }
        }
        ViewItem::File(f) => {
            writeln!(
                output,
                "   File metrics: {} lines, {} functions, avg complexity={:.1}",
                f.metrics.total_lines, f.metrics.function_count, f.metrics.avg_complexity
            )
            .ok();
            if verbosity >= 1 {
                writeln!(output, "   Recommendation: {}", f.recommendation).ok();
            }
        }
    }

    writeln!(output).ok();
}

// ============================================================================
// MARKDOWN FORMAT
// ============================================================================

/// Configuration for markdown output.
#[derive(Default)]
pub struct MarkdownConfig {
    pub verbosity: u8,
    pub show_filter_stats: bool,
}

/// Formats a PreparedDebtView as Markdown.
///
/// # Arguments
///
/// * `view` - The prepared view to format
/// * `config` - Markdown formatting configuration
///
/// # Returns
///
/// Formatted markdown string.
pub fn format_markdown(view: &PreparedDebtView, config: &MarkdownConfig) -> String {
    use std::fmt::Write;
    let mut output = String::new();

    // Header
    writeln!(output, "# Technical Debt Analysis Report\n").ok();

    // Summary section
    format_markdown_summary(&mut output, &view.summary);

    if view.is_empty() {
        writeln!(
            output,
            "\n*No technical debt items found matching current thresholds.*"
        )
        .ok();
        return output;
    }

    // Items section
    writeln!(output, "## Debt Items\n").ok();
    for (i, item) in view.items.iter().enumerate() {
        format_markdown_item(&mut output, item, i + 1, config.verbosity);
    }

    // Filter stats if requested
    if config.show_filter_stats {
        format_markdown_filter_stats(&mut output, &view.summary);
    }

    output
}

fn format_markdown_summary(output: &mut String, summary: &ViewSummary) {
    use std::fmt::Write;

    writeln!(output, "## Summary\n").ok();
    writeln!(
        output,
        "**Total Debt Items:** {}\n",
        summary.total_items_after_filter
    )
    .ok();
    writeln!(output, "| Metric | Value |").ok();
    writeln!(output, "|--------|-------|").ok();
    writeln!(
        output,
        "| Total Debt Score | {:.1} |",
        summary.total_debt_score
    )
    .ok();
    writeln!(
        output,
        "| Debt Density | {:.2} per 1k LOC |",
        summary.debt_density
    )
    .ok();
    writeln!(
        output,
        "| Lines of Code | {} |",
        summary.total_lines_of_code
    )
    .ok();
    if let Some(coverage) = summary.overall_coverage {
        writeln!(output, "| Overall Coverage | {:.1}% |", coverage * 100.0).ok();
    }
    writeln!(output).ok();

    // Score distribution
    writeln!(output, "### Score Distribution\n").ok();
    writeln!(output, "| Severity | Count |").ok();
    writeln!(output, "|----------|-------|").ok();
    writeln!(
        output,
        "| Critical | {} |",
        summary.score_distribution.critical
    )
    .ok();
    writeln!(output, "| High | {} |", summary.score_distribution.high).ok();
    writeln!(output, "| Medium | {} |", summary.score_distribution.medium).ok();
    writeln!(output, "| Low | {} |", summary.score_distribution.low).ok();
    writeln!(output).ok();
}

fn format_markdown_item(output: &mut String, item: &ViewItem, rank: usize, verbosity: u8) {
    use std::fmt::Write;

    let loc = item.location();
    let severity = item.severity().as_str();

    writeln!(
        output,
        "### {}. {} (Score: {:.1})\n",
        rank,
        severity,
        item.score()
    )
    .ok();
    writeln!(output, "**File:** `{}`", loc.file.display()).ok();
    if let Some(func) = &loc.function {
        writeln!(
            output,
            "**Function:** `{}` (line {})",
            func,
            loc.line.unwrap_or(0)
        )
        .ok();
    }
    writeln!(output).ok();

    match item {
        ViewItem::Function(f) => {
            writeln!(output, "| Metric | Value |").ok();
            writeln!(output, "|--------|-------|").ok();
            writeln!(
                output,
                "| Cyclomatic Complexity | {} |",
                f.cyclomatic_complexity
            )
            .ok();
            writeln!(
                output,
                "| Cognitive Complexity | {} |",
                f.cognitive_complexity
            )
            .ok();
            writeln!(output, "| Nesting Depth | {} |", f.nesting_depth).ok();
            writeln!(output, "| Function Length | {} lines |", f.function_length).ok();
            if let Some(cov) = f.transitive_coverage.as_ref() {
                writeln!(output, "| Coverage | {:.1}% |", cov.direct * 100.0).ok();
            }
            writeln!(output).ok();

            if verbosity >= 1 {
                writeln!(
                    output,
                    "**Recommendation:** {}\n",
                    f.recommendation.primary_action
                )
                .ok();
            }
        }
        ViewItem::File(f) => {
            writeln!(output, "| Metric | Value |").ok();
            writeln!(output, "|--------|-------|").ok();
            writeln!(output, "| Total Lines | {} |", f.metrics.total_lines).ok();
            writeln!(output, "| Function Count | {} |", f.metrics.function_count).ok();
            writeln!(
                output,
                "| Avg Complexity | {:.1} |",
                f.metrics.avg_complexity
            )
            .ok();
            writeln!(output, "| Max Complexity | {} |", f.metrics.max_complexity).ok();
            writeln!(
                output,
                "| Coverage | {:.1}% |",
                f.metrics.coverage_percent * 100.0
            )
            .ok();
            writeln!(output).ok();

            if verbosity >= 1 {
                writeln!(output, "**Recommendation:** {}\n", f.recommendation).ok();
            }
        }
    }
}

fn format_markdown_filter_stats(output: &mut String, summary: &ViewSummary) {
    use std::fmt::Write;

    writeln!(output, "## Filtering Summary\n").ok();
    writeln!(
        output,
        "- Total items analyzed: {}",
        summary.total_items_before_filter
    )
    .ok();
    writeln!(
        output,
        "- Items included: {}",
        summary.total_items_after_filter
    )
    .ok();
    writeln!(output, "- Filtered by score: {}", summary.filtered_by_score).ok();
    writeln!(output, "- Filtered by tier: {}", summary.filtered_by_tier).ok();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::priority::call_graph::CallGraph;
    use crate::priority::tiers::TierConfig;
    use crate::priority::view::ViewConfig;
    use crate::priority::view_pipeline::prepare_view;
    use crate::priority::UnifiedAnalysis;

    fn create_empty_view() -> PreparedDebtView {
        let call_graph = CallGraph::new();
        let analysis = UnifiedAnalysis::new(call_graph);
        prepare_view(&analysis, &ViewConfig::default(), &TierConfig::default())
    }

    #[test]
    fn test_format_json_empty_view() {
        let view = create_empty_view();
        let json = format_json(&view, false);
        // Pretty-printed JSON has spaces after colons
        assert!(json.contains("\"format_version\": \"3.0\""));
        assert!(json.contains("\"total_items\": 0"));
    }

    #[test]
    fn test_format_terminal_empty_view() {
        let view = create_empty_view();
        let config = TerminalConfig::default();
        let output = format_terminal(&view, &config);
        assert!(output.contains("TECHNICAL DEBT ANALYSIS"));
        assert!(output.contains("No technical debt items found"));
    }

    #[test]
    fn test_format_markdown_empty_view() {
        let view = create_empty_view();
        let config = MarkdownConfig::default();
        let output = format_markdown(&view, &config);
        assert!(output.contains("# Technical Debt Analysis Report"));
        assert!(output.contains("No technical debt items found"));
    }

    #[test]
    fn test_to_json_output_structure() {
        let view = create_empty_view();
        let output = to_json_output(&view, false);
        assert_eq!(output.format_version, "3.0");
        assert!(output.items.is_empty());
    }

    #[test]
    fn test_format_json_with_scoring_details() {
        let view = create_empty_view();
        let json_without = format_json(&view, false);
        let json_with = format_json(&view, true);
        // Both should be valid JSON, scoring details only visible with items
        assert!(json_without.contains("format_version"));
        assert!(json_with.contains("format_version"));
    }

    #[test]
    fn test_terminal_config_default() {
        let config = TerminalConfig::default();
        assert_eq!(config.verbosity, 0);
        assert!(config.use_color);
        assert!(!config.summary_mode);
    }

    #[test]
    fn test_markdown_config_default() {
        let config = MarkdownConfig::default();
        assert_eq!(config.verbosity, 0);
        assert!(!config.show_filter_stats);
    }
}
