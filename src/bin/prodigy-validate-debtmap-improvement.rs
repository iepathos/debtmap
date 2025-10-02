use anyhow::{Context, Result};
use debtmap::comparison::types::{ComparisonResult, TargetStatus};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ValidationResult {
    completion_percentage: f64,
    status: String,
    improvements: Vec<String>,
    remaining_issues: Vec<String>,
    gaps: HashMap<String, GapDetail>,
    #[serde(skip_serializing_if = "Option::is_none")]
    target_summary: Option<TargetSummary>,
    project_summary: ProjectSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GapDetail {
    description: String,
    location: String,
    severity: String,
    suggested_fix: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    score_before: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    score_after: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    current_score: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TargetSummary {
    location: String,
    score_before: f64,
    score_after: Option<f64>,
    improvement_percent: f64,
    status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProjectSummary {
    total_debt_before: f64,
    total_debt_after: f64,
    improvement_percent: f64,
    items_resolved: usize,
    items_new: usize,
}

fn main() -> Result<()> {
    let args = env::var("ARGUMENTS").unwrap_or_default();

    let is_automation = env::var("PRODIGY_AUTOMATION")
        .unwrap_or_default()
        .eq_ignore_ascii_case("true")
        || env::var("PRODIGY_VALIDATION")
            .unwrap_or_default()
            .eq_ignore_ascii_case("true");

    if !is_automation {
        println!("Validating debtmap improvement...");
    }

    let (comparison_path, output_path) = parse_arguments(&args)?;

    if !is_automation {
        println!("Loading comparison from: {}", comparison_path.display());
        println!("Output will be written to: {}", output_path.display());
    }

    let comparison = load_comparison(&comparison_path)?;
    let validation = validate_improvement(&comparison)?;

    write_validation_result(&output_path, &validation)?;

    if !is_automation {
        println!(
            "\nValidation complete: {:.1}% improvement",
            validation.completion_percentage
        );
        println!("Status: {}", validation.status);
    }

    Ok(())
}

fn parse_arguments(args: &str) -> Result<(PathBuf, PathBuf)> {
    let mut comparison_path = None;
    let mut output_path = None;

    let parts: Vec<&str> = args.split_whitespace().collect();
    let mut i = 0;

    while i < parts.len() {
        match parts[i] {
            "--comparison" => {
                if i + 1 < parts.len() {
                    comparison_path = Some(PathBuf::from(parts[i + 1]));
                    i += 2;
                } else {
                    anyhow::bail!("--comparison requires a path argument");
                }
            }
            "--output" => {
                if i + 1 < parts.len() {
                    output_path = Some(PathBuf::from(parts[i + 1]));
                    i += 2;
                } else {
                    anyhow::bail!("--output requires a path argument");
                }
            }
            _ => i += 1,
        }
    }

    let comparison_path = comparison_path.context("Missing required --comparison argument")?;
    let output_path =
        output_path.unwrap_or_else(|| PathBuf::from(".prodigy/debtmap-validation.json"));

    if !comparison_path.exists() {
        anyhow::bail!(
            "Comparison file does not exist: {}",
            comparison_path.display()
        );
    }

    Ok((comparison_path, output_path))
}

fn load_comparison(path: &Path) -> Result<ComparisonResult> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read comparison file: {}", path.display()))?;

    serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse comparison JSON from: {}", path.display()))
}

fn validate_improvement(comparison: &ComparisonResult) -> Result<ValidationResult> {
    let mut improvements = Vec::new();
    let mut remaining_issues = Vec::new();
    let mut gaps = HashMap::new();

    // Extract target improvement metrics
    let target_component = if let Some(target) = &comparison.target_item {
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

        improvement_pct.min(100.0)
    } else {
        0.0
    };

    // Calculate regression penalty
    let regression_count = comparison.regressions.len();
    let regression_penalty = (regression_count * 20).min(100) as f64;
    let no_regression_component = (100.0 - regression_penalty).max(0.0);

    if regression_count > 0 {
        remaining_issues.push(format!(
            "{} new critical debt item{} introduced",
            regression_count,
            if regression_count == 1 { "" } else { "s" }
        ));

        for (idx, regression) in comparison.regressions.iter().take(3).enumerate() {
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

    // Extract project health metrics
    let project_health = &comparison.project_health;
    let debt_improvement_pct = project_health.changes.debt_score_change_pct;
    let project_health_component = (debt_improvement_pct.abs() * 10.0).min(100.0);

    if debt_improvement_pct < 0.0 {
        improvements.push(format!(
            "Overall project debt reduced by {:.1}%",
            debt_improvement_pct.abs()
        ));
    }

    // Calculate composite score
    let improvement_score =
        (target_component * 0.5 + project_health_component * 0.3 + no_regression_component * 0.2)
            .clamp(0.0, 100.0);

    let status = if improvement_score >= 75.0 {
        "complete"
    } else {
        "incomplete"
    }
    .to_string();

    let target_summary = comparison.target_item.as_ref().map(|target| TargetSummary {
        location: target.location.clone(),
        score_before: target.before.score,
        score_after: target.after.as_ref().map(|a| a.score),
        improvement_percent: target.improvements.score_reduction_pct,
        status: format!("{:?}", target.status).to_lowercase(),
    });

    let project_summary = ProjectSummary {
        total_debt_before: project_health.before.total_debt_score,
        total_debt_after: project_health.after.total_debt_score,
        improvement_percent: debt_improvement_pct,
        items_resolved: comparison.summary.resolved_count,
        items_new: comparison.summary.new_critical_count,
    };

    Ok(ValidationResult {
        completion_percentage: improvement_score,
        status,
        improvements,
        remaining_issues,
        gaps,
        target_summary,
        project_summary,
    })
}

fn write_validation_result(path: &Path, result: &ValidationResult) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
    }

    let json = serde_json::to_string_pretty(result)?;
    fs::write(path, json)
        .with_context(|| format!("Failed to write validation result to: {}", path.display()))?;

    Ok(())
}
