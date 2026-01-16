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
/// Converts output format types back to internal types for comparison
fn json_to_analysis(json: DebtmapJsonInput) -> UnifiedAnalysis {
    use crate::output::unified::UnifiedDebtItemOutput;
    use crate::priority::call_graph::CallGraph;
    use im::Vector;

    let mut items = Vector::new();
    let mut file_items = Vector::new();

    // Convert output format items to internal types
    for item in json.items {
        match item {
            UnifiedDebtItemOutput::Function(func) => {
                items.push_back(output_to_internal_function(&func));
            }
            UnifiedDebtItemOutput::File(file) => {
                file_items.push_back(output_to_internal_file(&file));
            }
        }
    }

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

/// Convert FunctionDebtItemOutput to UnifiedDebtItem with minimal fields for comparison
fn output_to_internal_function(
    output: &crate::output::unified::FunctionDebtItemOutput,
) -> crate::priority::unified_scorer::UnifiedDebtItem {
    use crate::priority::unified_scorer::{Location, UnifiedDebtItem, UnifiedScore};
    use crate::priority::{ActionableRecommendation, ImpactMetrics};
    use std::path::PathBuf;

    UnifiedDebtItem {
        location: Location {
            file: PathBuf::from(&output.location.file),
            function: output.location.function.clone().unwrap_or_default(),
            line: output.location.line.unwrap_or(0),
        },
        debt_type: output.debt_type.clone(),
        unified_score: UnifiedScore {
            complexity_factor: 0.0,
            coverage_factor: 0.0,
            dependency_factor: 0.0,
            role_multiplier: 1.0,
            final_score: output.score,
            base_score: None,
            exponential_factor: None,
            risk_boost: None,
            pre_adjustment_score: None,
            adjustment_applied: None,
            purity_factor: None,
            refactorability_factor: None,
            pattern_factor: None,
            debt_adjustment: None,
            pre_normalization_score: None,
            structural_multiplier: None,
            has_coverage_data: output.metrics.coverage.is_some(),
            contextual_risk_multiplier: None,
            pre_contextual_score: None,
        },
        function_role: output.function_role,
        recommendation: ActionableRecommendation {
            primary_action: String::new(),
            rationale: String::new(),
            implementation_steps: vec![],
            related_items: vec![],
            steps: None,
            estimated_effort_hours: None,
        },
        expected_impact: ImpactMetrics {
            complexity_reduction: 0.0,
            coverage_improvement: 0.0,
            risk_reduction: 0.0,
            lines_reduction: 0,
        },
        transitive_coverage: output.metrics.coverage.map(|cov| {
            crate::priority::coverage_propagation::TransitiveCoverage {
                direct: cov,
                transitive: cov,
                propagated_from: vec![],
                uncovered_lines: output.metrics.uncovered_lines.clone().unwrap_or_default(),
            }
        }),
        upstream_dependencies: output.dependencies.upstream_count,
        downstream_dependencies: output.dependencies.downstream_count,
        upstream_callers: vec![],
        downstream_callees: vec![],
        upstream_production_callers: vec![],
        upstream_test_callers: vec![],
        production_blast_radius: 0,
        nesting_depth: output.metrics.nesting_depth,
        function_length: output.metrics.length,
        cyclomatic_complexity: output.metrics.cyclomatic_complexity,
        cognitive_complexity: output.metrics.cognitive_complexity,
        is_pure: output.purity_analysis.as_ref().map(|p| p.is_pure),
        purity_confidence: output.purity_analysis.as_ref().map(|p| p.confidence),
        purity_level: None,
        god_object_indicators: None,
        tier: None,
        function_context: None,
        context_confidence: None,
        contextual_recommendation: None,
        pattern_analysis: None,
        file_context: None,
        context_multiplier: None,
        context_type: None,
        language_specific: None,
        detected_pattern: None,
        contextual_risk: None,
        file_line_count: None,
        responsibility_category: None,
        error_swallowing_count: None,
        error_swallowing_patterns: None,
        entropy_analysis: None,
        context_suggestion: None,
    }
}

/// Convert FileDebtItemOutput to FileDebtItem with minimal fields for comparison
fn output_to_internal_file(
    output: &crate::output::unified::FileDebtItemOutput,
) -> crate::priority::file_metrics::FileDebtItem {
    use crate::priority::file_metrics::{FileDebtItem, FileDebtMetrics, FileImpact};
    use std::path::PathBuf;

    FileDebtItem {
        metrics: FileDebtMetrics {
            path: PathBuf::from(&output.location.file),
            total_lines: output.metrics.lines,
            function_count: output.metrics.functions,
            class_count: output.metrics.classes,
            avg_complexity: output.metrics.avg_complexity,
            max_complexity: output.metrics.max_complexity,
            total_complexity: output.metrics.total_complexity,
            coverage_percent: output.metrics.coverage,
            uncovered_lines: output.metrics.uncovered_lines,
            god_object_analysis: None,
            function_scores: vec![],
            god_object_type: None,
            file_type: None,
            afferent_coupling: 0,
            efferent_coupling: 0,
            instability: 0.0,
            dependents: vec![],
            dependencies_list: vec![],
        },
        score: output.score,
        priority_rank: match output.priority {
            crate::output::unified::Priority::Critical => 1,
            crate::output::unified::Priority::High => 2,
            crate::output::unified::Priority::Medium => 3,
            crate::output::unified::Priority::Low => 4,
        },
        recommendation: String::new(),
        impact: FileImpact {
            complexity_reduction: 0.0,
            maintainability_improvement: 0.0,
            test_effort: 0.0,
        },
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
