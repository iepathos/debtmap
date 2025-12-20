//! Compare command handler
//!
//! This module contains the handler for the `compare` subcommand,
//! which compares two analysis results and generates a diff report.

use crate::cli::args::OutputFormat;
use crate::commands::compare_debtmap::DebtmapJsonInput;
use crate::comparison::{Comparator, ComparisonResult, DebtTrend, PlanParser, TargetStatus};
use crate::priority::UnifiedAnalysis;
use anyhow::Result;
use std::path::Path;

/// Handle the compare command
pub fn handle_compare_command(
    before: &Path,
    after: &Path,
    plan: Option<&Path>,
    target_location: Option<String>,
    format: OutputFormat,
    output: Option<&Path>,
) -> Result<()> {
    use std::fs;

    // Extract target location from plan or use explicit location
    let target = if let Some(plan_path) = plan {
        Some(PlanParser::extract_target_location(plan_path)?)
    } else {
        target_location
    };

    // Load JSON output and convert to UnifiedAnalysis
    let before_content = fs::read_to_string(before)?;
    let before_json: DebtmapJsonInput = serde_json::from_str(&before_content)?;
    let before_results = json_to_analysis(before_json);

    let after_content = fs::read_to_string(after)?;
    let after_json: DebtmapJsonInput = serde_json::from_str(&after_content)?;
    let after_results = json_to_analysis(after_json);

    // Perform comparison
    let comparator = Comparator::new(before_results, after_results, target);
    let comparison = comparator.compare()?;

    // Output results
    let output_str = match format {
        OutputFormat::Json => serde_json::to_string_pretty(&comparison)?,
        OutputFormat::Markdown => format_comparison_markdown(&comparison),
        // LLM markdown uses same format as regular markdown for comparison
        OutputFormat::LlmMarkdown => format_comparison_markdown(&comparison),
        OutputFormat::Html => format_comparison_markdown(&comparison),
        OutputFormat::Dot => {
            // DOT format not applicable for comparison, use terminal
            print_comparison_terminal(&comparison);
            return Ok(());
        }
        OutputFormat::Terminal => {
            print_comparison_terminal(&comparison);
            return Ok(());
        }
    };

    // Write to file or stdout
    if let Some(output_path) = output {
        std::fs::write(output_path, output_str)?;
    } else {
        println!("{}", output_str);
    }

    Ok(())
}

/// Pure function to convert DebtmapJsonInput to UnifiedAnalysis
/// Splits merged DebtItem enum into separate function and file vectors
fn json_to_analysis(json: DebtmapJsonInput) -> UnifiedAnalysis {
    use crate::priority::{call_graph::CallGraph, DebtItem};
    use im::Vector;

    let mut items = Vector::new();
    let mut file_items = Vector::new();

    // Split DebtItems into function and file items
    for item in json.items {
        match item {
            DebtItem::Function(func) => items.push_back(*func),
            DebtItem::File(file) => file_items.push_back(*file),
        }
    }

    // Create UnifiedAnalysis with empty call graph and data flow graph
    // These aren't serialized in JSON output anyway
    let call_graph = CallGraph::new();

    UnifiedAnalysis {
        items,
        file_items,
        total_impact: json.total_impact,
        total_debt_score: json.total_debt_score,
        debt_density: json.debt_density,
        total_lines_of_code: json.total_lines_of_code,
        call_graph: call_graph.clone(),
        data_flow_graph: crate::data_flow::DataFlowGraph::from_call_graph(call_graph),
        overall_coverage: json.overall_coverage,
        has_coverage_data: json.overall_coverage.is_some(),
        timings: None,
        stats: crate::priority::FilterStatistics::new(),
        analyzed_files: std::collections::HashMap::new(),
    }
}

/// Format comparison as markdown
fn format_comparison_markdown(comparison: &ComparisonResult) -> String {
    let mut md = String::new();

    md.push_str("# Debtmap Comparison Report\n\n");
    md.push_str(&format!(
        "**Date**: {}\n\n",
        comparison.metadata.comparison_date
    ));

    if let Some(target) = &comparison.target_item {
        md.push_str("## Target Item Analysis\n\n");

        let status_icon = match target.status {
            TargetStatus::Resolved => "[OK]",
            TargetStatus::Improved => "[OK]",
            TargetStatus::Unchanged => "[WARNING]",
            TargetStatus::Regressed => "[ERROR]",
            TargetStatus::NotFoundBefore | TargetStatus::NotFound => "[UNKNOWN]",
        };

        md.push_str(&format!(
            "{} **Status**: {:?}\n\n",
            status_icon, target.status
        ));
        md.push_str(&format!("**Location**: `{}`\n\n", target.location));

        md.push_str("### Before\n");
        md.push_str(&format!("- **Score**: {:.1}\n", target.before.score));
        md.push_str(&format!(
            "- **Complexity**: Cyclomatic {}, Cognitive {}\n",
            target.before.cyclomatic_complexity, target.before.cognitive_complexity
        ));
        md.push_str(&format!("- **Coverage**: {:.1}%\n", target.before.coverage));
        md.push_str(&format!(
            "- **Function Length**: {} lines\n\n",
            target.before.function_length
        ));

        if let Some(after_metrics) = &target.after {
            md.push_str("### After\n");
            md.push_str(&format!("- **Score**: {:.1}\n", after_metrics.score));
            md.push_str(&format!(
                "- **Complexity**: Cyclomatic {}, Cognitive {}\n",
                after_metrics.cyclomatic_complexity, after_metrics.cognitive_complexity
            ));
            md.push_str(&format!("- **Coverage**: {:.1}%\n", after_metrics.coverage));
            md.push_str(&format!(
                "- **Function Length**: {} lines\n\n",
                after_metrics.function_length
            ));
        }

        md.push_str("### Improvements\n");
        md.push_str(&format!(
            "- Score reduced by **{:.1}%**\n",
            target.improvements.score_reduction_pct
        ));
        md.push_str(&format!(
            "- Complexity reduced by **{:.1}%**\n",
            target.improvements.complexity_reduction_pct
        ));
        md.push_str(&format!(
            "- Coverage improved by **{:.1}%**\n\n",
            target.improvements.coverage_improvement_pct
        ));
    }

    md.push_str("## Project Health\n\n");

    let trend_icon = match comparison.summary.overall_debt_trend {
        DebtTrend::Improving => "[IMPROVING]",
        DebtTrend::Stable => "[STABLE]",
        DebtTrend::Regressing => "[REGRESSING]",
    };

    md.push_str(&format!(
        "### Overall Trend: {} {:?}\n\n",
        trend_icon, comparison.summary.overall_debt_trend
    ));
    md.push_str(&format!(
        "- Total debt: {:.1} -> {:.1} ({:+.1}%)\n",
        comparison.project_health.before.total_debt_score,
        comparison.project_health.after.total_debt_score,
        comparison.project_health.changes.debt_score_change_pct
    ));
    md.push_str(&format!(
        "- Critical items: {} -> {} ({:+})\n",
        comparison.project_health.before.critical_items,
        comparison.project_health.after.critical_items,
        comparison.project_health.changes.critical_items_change
    ));

    if !comparison.regressions.is_empty() {
        md.push_str(&format!(
            "\n[WARNING] {} new critical item(s) detected\n\n",
            comparison.regressions.len()
        ));

        md.push_str("### Regressions\n\n");
        for reg in &comparison.regressions {
            md.push_str(&format!("- `{}` (score: {:.1})\n", reg.location, reg.score));
        }
    } else {
        md.push_str("\n[OK] No new critical items introduced\n");
    }

    md.push_str("\n## Summary\n\n");
    if comparison.summary.target_improved {
        md.push_str("[OK] Target item significantly improved\n");
    }
    if comparison.summary.new_critical_count == 0 {
        md.push_str("[OK] No regressions detected\n");
    }
    match comparison.summary.overall_debt_trend {
        DebtTrend::Improving => md.push_str("[OK] Overall project health improved\n"),
        DebtTrend::Stable => md.push_str("[STABLE] Overall project health stable\n"),
        DebtTrend::Regressing => md.push_str("[WARNING] Overall project health declined\n"),
    }

    md
}

/// Print comparison to terminal
fn print_comparison_terminal(comparison: &ComparisonResult) {
    println!("{}", format_comparison_markdown(comparison));
}

#[cfg(test)]
mod tests {
    // Tests for compare command would go here
    // Integration tests are more appropriate since this mainly does I/O
}
