//! Validation of technical debt improvements
//!
//! This module validates that technical debt improvements have been made
//! by analyzing comparison output from `debtmap compare`.
//!
//! # Scoring Algorithm
//!
//! The validation score is a composite of three components:
//! - Target improvement (50%): Did the specific target item improve?
//! - Project health (30%): Did overall project debt decrease?
//! - No regressions (20%): Were new critical items introduced?
//!
//! # Examples
//!
//! ```no_run
//! use debtmap::commands::validate_improvement::*;
//! use std::path::PathBuf;
//!
//! let config = ValidateImprovementConfig {
//!     comparison_path: PathBuf::from("comparison.json"),
//!     output_path: PathBuf::from("validation.json"),
//!     previous_validation: None,
//!     threshold: 75.0,
//!     format: OutputFormat::Json,
//!     quiet: false,
//! };
//!
//! validate_improvement(config).unwrap();
//! ```

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::comparison::types::{ComparisonResult, RegressionItem, TargetComparison, TargetStatus};

/// Output format for validation results
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Json,
    Terminal,
    Markdown,
}

/// Configuration for validation
#[derive(Debug, Clone)]
pub struct ValidateImprovementConfig {
    pub comparison_path: PathBuf,
    pub output_path: PathBuf,
    pub previous_validation: Option<PathBuf>,
    pub threshold: f64,
    pub format: OutputFormat,
    pub quiet: bool,
}

/// Validation result structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub completion_percentage: f64,
    pub status: String,
    pub improvements: Vec<String>,
    pub remaining_issues: Vec<String>,
    pub gaps: HashMap<String, GapDetail>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_summary: Option<TargetSummary>,
    pub project_summary: ProjectSummary,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trend_analysis: Option<TrendAnalysis>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attempt_number: Option<u32>,
}

/// Gap detail structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GapDetail {
    pub description: String,
    pub location: String,
    pub severity: String,
    pub suggested_fix: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score_before: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score_after: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_score: Option<f64>,
}

/// Target summary structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetSummary {
    pub location: String,
    pub score_before: f64,
    pub score_after: Option<f64>,
    pub improvement_percent: f64,
    pub status: String,
}

/// Project summary structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectSummary {
    pub total_debt_before: f64,
    pub total_debt_after: f64,
    pub improvement_percent: f64,
    pub items_resolved: usize,
    pub items_new: usize,
}

/// Trend analysis structure for tracking progress across attempts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendAnalysis {
    pub direction: String,
    pub previous_completion: Option<f64>,
    pub change: Option<f64>,
    pub recommendation: String,
}

/// Main entry point for validation
pub fn validate_improvement(config: ValidateImprovementConfig) -> Result<()> {
    // I/O: Load inputs
    let comparison = load_comparison(&config.comparison_path)?;
    let previous = config
        .previous_validation
        .as_ref()
        .map(|path| load_previous_validation(path))
        .transpose()?;

    // Pure: Perform calculations
    let result = validate_improvement_internal(&comparison, previous.as_ref())?;

    // I/O: Write outputs
    write_validation_result(&config.output_path, &result, config.format)?;

    // I/O: Print to console (if not quiet)
    if !config.quiet {
        print_validation_summary(&result);
    }

    Ok(())
}

/// Load comparison file
fn load_comparison(path: &Path) -> Result<ComparisonResult> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read comparison file: {}", path.display()))?;

    serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse comparison JSON from: {}", path.display()))
}

/// Load previous validation
fn load_previous_validation(path: &Path) -> Result<ValidationResult> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read validation file: {}", path.display()))?;

    serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse validation JSON from: {}", path.display()))
}

/// Pure validation logic
fn validate_improvement_internal(
    comparison: &ComparisonResult,
    previous: Option<&ValidationResult>,
) -> Result<ValidationResult> {
    let mut improvements = Vec::new();
    let mut remaining_issues = Vec::new();
    let mut gaps = HashMap::new();

    let target_component = process_target_improvements(
        comparison,
        &mut improvements,
        &mut remaining_issues,
        &mut gaps,
    );
    let no_regression_component = process_regressions(comparison, &mut remaining_issues, &mut gaps);
    let project_health_component = process_project_health(comparison, &mut improvements);

    let improvement_score = calculate_composite_score(
        target_component,
        project_health_component,
        no_regression_component,
    );

    let status = determine_status(improvement_score);
    let target_summary = build_target_summary(&comparison.target_item);
    let project_summary = build_project_summary(comparison);

    // Calculate trend analysis if previous validation is provided
    let trend_analysis = previous.map(|prev| calculate_trend_analysis(prev, improvement_score));
    let attempt_number = previous.map(|prev| prev.attempt_number.unwrap_or(1) + 1);

    Ok(ValidationResult {
        completion_percentage: improvement_score,
        status,
        improvements,
        remaining_issues,
        gaps,
        target_summary,
        project_summary,
        trend_analysis,
        attempt_number,
    })
}

/// Process target improvements
fn process_target_improvements(
    comparison: &ComparisonResult,
    improvements: &mut Vec<String>,
    remaining_issues: &mut Vec<String>,
    gaps: &mut HashMap<String, GapDetail>,
) -> f64 {
    comparison
        .target_item
        .as_ref()
        .map(|target| {
            let improvement_pct = target.improvements.score_reduction_pct;

            if improvement_pct > 0.0 {
                improvements.push(format!(
                    "Target item score reduced by {:.1}% ({:.1} → {:.1})",
                    improvement_pct,
                    target.before.score,
                    target.after.as_ref().map(|a| a.score).unwrap_or(0.0)
                ));
            }

            if target.status == TargetStatus::Unchanged {
                add_target_gap(target, remaining_issues, gaps);
            }

            improvement_pct.min(100.0)
        })
        .unwrap_or(0.0)
}

/// Add target gap
fn add_target_gap(
    target: &TargetComparison,
    remaining_issues: &mut Vec<String>,
    gaps: &mut HashMap<String, GapDetail>,
) {
    remaining_issues.push("Target debt item not improved".to_string());
    gaps.insert(
        "insufficient_target_improvement".to_string(),
        GapDetail {
            description: "Target function still above complexity threshold".to_string(),
            location: target.location.clone(),
            severity: "high".to_string(),
            suggested_fix: "Further extract helper functions or simplify logic".to_string(),
            score_before: Some(target.before.score),
            score_after: target.after.as_ref().map(|a| a.score),
            current_score: None,
        },
    );
}

/// Process regressions
fn process_regressions(
    comparison: &ComparisonResult,
    remaining_issues: &mut Vec<String>,
    gaps: &mut HashMap<String, GapDetail>,
) -> f64 {
    let regression_count = comparison.regressions.len();

    if regression_count > 0 {
        remaining_issues.push(format!(
            "{} new critical debt item{} introduced",
            regression_count,
            if regression_count == 1 { "" } else { "s" }
        ));

        add_regression_gaps(&comparison.regressions, gaps);
    }

    let regression_penalty = (regression_count * 20).min(100) as f64;
    (100.0 - regression_penalty).max(0.0)
}

/// Add regression gaps
fn add_regression_gaps(regressions: &[RegressionItem], gaps: &mut HashMap<String, GapDetail>) {
    for (idx, regression) in regressions.iter().take(3).enumerate() {
        gaps.insert(
            format!("regression_{}", idx),
            GapDetail {
                description: regression.description.clone(),
                location: regression.location.clone(),
                severity: "high".to_string(),
                suggested_fix: "Simplify using pure functional patterns".to_string(),
                score_before: None,
                score_after: None,
                current_score: Some(regression.score),
            },
        );
    }
}

/// Process project health
fn process_project_health(comparison: &ComparisonResult, improvements: &mut Vec<String>) -> f64 {
    let debt_improvement_pct = comparison.project_health.changes.debt_score_change_pct;

    if debt_improvement_pct < 0.0 {
        improvements.push(format!(
            "Overall project debt reduced by {:.1}%",
            debt_improvement_pct.abs()
        ));
    }

    (debt_improvement_pct.abs() * 10.0).min(100.0)
}

/// Calculate composite score
fn calculate_composite_score(
    target_component: f64,
    project_health_component: f64,
    no_regression_component: f64,
) -> f64 {
    (target_component * 0.5 + project_health_component * 0.3 + no_regression_component * 0.2)
        .clamp(0.0, 100.0)
}

/// Determine status
fn determine_status(improvement_score: f64) -> String {
    if improvement_score >= 75.0 {
        "complete"
    } else {
        "incomplete"
    }
    .to_string()
}

/// Build target summary
fn build_target_summary(target_item: &Option<TargetComparison>) -> Option<TargetSummary> {
    target_item.as_ref().map(|target| TargetSummary {
        location: target.location.clone(),
        score_before: target.before.score,
        score_after: target.after.as_ref().map(|a| a.score),
        improvement_percent: target.improvements.score_reduction_pct,
        status: format!("{:?}", target.status).to_lowercase(),
    })
}

/// Build project summary
fn build_project_summary(comparison: &ComparisonResult) -> ProjectSummary {
    ProjectSummary {
        total_debt_before: comparison.project_health.before.total_debt_score,
        total_debt_after: comparison.project_health.after.total_debt_score,
        improvement_percent: comparison.project_health.changes.debt_score_change_pct,
        items_resolved: comparison.summary.resolved_count,
        items_new: comparison.summary.new_critical_count,
    }
}

/// Write validation result
fn write_validation_result(
    path: &Path,
    result: &ValidationResult,
    format: OutputFormat,
) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
    }

    let output = match format {
        OutputFormat::Json => format_validation_json(result)?,
        OutputFormat::Terminal => format_validation_terminal(result)?,
        OutputFormat::Markdown => format_validation_markdown(result)?,
    };

    fs::write(path, output)
        .with_context(|| format!("Failed to write validation result to: {}", path.display()))?;

    Ok(())
}

/// Format validation as JSON
fn format_validation_json(result: &ValidationResult) -> Result<String> {
    serde_json::to_string_pretty(result).context("Failed to serialize validation result")
}

/// Format validation for terminal
fn format_validation_terminal(result: &ValidationResult) -> Result<String> {
    let mut output = String::new();

    output.push_str("═══ Validation Results ═══\n");
    output.push_str(&format!(
        "Completion: {:.1}%\n",
        result.completion_percentage
    ));
    output.push_str(&format!("Status: {}\n\n", result.status));

    if !result.improvements.is_empty() {
        output.push_str("✓ Improvements:\n");
        for improvement in &result.improvements {
            output.push_str(&format!("  • {}\n", improvement));
        }
        output.push('\n');
    }

    if !result.remaining_issues.is_empty() {
        output.push_str("⚠ Remaining Issues:\n");
        for issue in &result.remaining_issues {
            output.push_str(&format!("  • {}\n", issue));
        }
    }

    Ok(output)
}

/// Format validation as Markdown
fn format_validation_markdown(result: &ValidationResult) -> Result<String> {
    let mut output = String::new();

    output.push_str("# Validation Results\n\n");
    output.push_str(&format!(
        "**Completion**: {:.1}%\n",
        result.completion_percentage
    ));
    output.push_str(&format!("**Status**: {}\n\n", result.status));

    if !result.improvements.is_empty() {
        output.push_str("## Improvements\n\n");
        for improvement in &result.improvements {
            output.push_str(&format!("- {}\n", improvement));
        }
        output.push('\n');
    }

    if !result.remaining_issues.is_empty() {
        output.push_str("## Remaining Issues\n\n");
        for issue in &result.remaining_issues {
            output.push_str(&format!("- {}\n", issue));
        }
        output.push('\n');
    }

    if !result.gaps.is_empty() {
        output.push_str("## Gaps\n\n");
        for (key, gap) in &result.gaps {
            output.push_str(&format!("### {}\n\n", key));
            output.push_str(&format!("- **Description**: {}\n", gap.description));
            output.push_str(&format!("- **Location**: {}\n", gap.location));
            output.push_str(&format!("- **Severity**: {}\n", gap.severity));
            output.push_str(&format!("- **Suggested Fix**: {}\n\n", gap.suggested_fix));
        }
    }

    Ok(output)
}

/// Print validation summary
fn print_validation_summary(result: &ValidationResult) {
    println!(
        "\nValidation complete: {:.1}% improvement",
        result.completion_percentage
    );
    println!("Status: {}", result.status);
}

/// Pure function to calculate trend analysis based on previous validation
fn calculate_trend_analysis(previous: &ValidationResult, current_score: f64) -> TrendAnalysis {
    let previous_completion = previous.completion_percentage;
    let change = current_score - previous_completion;

    let (direction, recommendation) = if change < -5.0 {
        (
            "regression".to_string(),
            "CRITICAL: Stop refactoring. Return to original plan and complete remaining items."
                .to_string(),
        )
    } else if change > 5.0 {
        (
            "progress".to_string(),
            "Continue completing remaining plan items.".to_string(),
        )
    } else {
        (
            "stable".to_string(),
            "Progress stalled. Focus on completing specific plan items rather than refactoring."
                .to_string(),
        )
    };

    TrendAnalysis {
        direction,
        previous_completion: Some(previous_completion),
        change: Some(change),
        recommendation,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_composite_score() {
        let score = calculate_composite_score(80.0, 50.0, 100.0);
        // 80*0.5 + 50*0.3 + 100*0.2 = 40 + 15 + 20 = 75
        assert_eq!(score, 75.0);
    }

    #[test]
    fn test_determine_status() {
        assert_eq!(determine_status(75.0), "complete");
        assert_eq!(determine_status(80.0), "complete");
        assert_eq!(determine_status(74.9), "incomplete");
        assert_eq!(determine_status(50.0), "incomplete");
    }

    #[test]
    fn test_calculate_composite_score_clamping() {
        let score = calculate_composite_score(100.0, 100.0, 100.0);
        assert_eq!(score, 100.0);

        let score = calculate_composite_score(0.0, 0.0, 0.0);
        assert_eq!(score, 0.0);
    }
}
