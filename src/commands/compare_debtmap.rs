use crate::priority::UnifiedAnalysis;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub completion_percentage: f64,
    pub status: String,
    pub improvements: Vec<String>,
    pub remaining_issues: Vec<String>,
    pub gaps: HashMap<String, GapDetail>,
    pub before_summary: AnalysisSummary,
    pub after_summary: AnalysisSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GapDetail {
    pub description: String,
    pub location: String,
    pub severity: String,
    pub suggested_fix: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_score: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_score: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_complexity: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_complexity: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_complexity: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisSummary {
    pub total_items: usize,
    pub high_priority_items: usize,
    pub average_score: f64,
}

pub struct CompareConfig {
    pub before_path: PathBuf,
    pub after_path: PathBuf,
    pub output_path: PathBuf,
}

pub fn compare_debtmaps(config: CompareConfig) -> Result<()> {
    let is_automation = std::env::var("PRODIGY_AUTOMATION")
        .unwrap_or_default()
        .eq_ignore_ascii_case("true")
        || std::env::var("PRODIGY_VALIDATION")
            .unwrap_or_default()
            .eq_ignore_ascii_case("true");

    if !is_automation {
        println!("Loading debtmap data from before and after states...");
    }

    let before = load_debtmap(&config.before_path)?;
    let after = load_debtmap(&config.after_path)?;

    let validation_result = perform_validation(&before, &after)?;

    write_validation_result(&config.output_path, &validation_result)?;

    if !is_automation {
        print_summary(&validation_result);
    }

    Ok(())
}

fn load_debtmap(path: &Path) -> Result<UnifiedAnalysis> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read debtmap file: {}", path.display()))?;

    serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse debtmap JSON from: {}", path.display()))
}

fn perform_validation(
    before: &UnifiedAnalysis,
    after: &UnifiedAnalysis,
) -> Result<ValidationResult> {
    let before_summary = create_summary(before);
    let after_summary = create_summary(after);

    let mut improvements = Vec::new();
    let mut remaining_issues = Vec::new();
    let mut gaps = HashMap::new();

    let resolved_items = identify_resolved_items(before, after);
    let improved_items = identify_improved_items(before, after);
    let new_items = identify_new_items(before, after);
    let unchanged_critical = identify_unchanged_critical(before, after);

    if resolved_items.high_priority_count > 0 {
        improvements.push(format!(
            "Resolved {} high-priority debt items",
            resolved_items.high_priority_count
        ));
    }

    if improved_items.complexity_reduction > 0.0 {
        improvements.push(format!(
            "Reduced average cyclomatic complexity by {:.0}%",
            improved_items.complexity_reduction * 100.0
        ));
    }

    if improved_items.coverage_improvement > 0.0 {
        improvements.push(format!(
            "Added test coverage for {} critical functions",
            improved_items.coverage_improvement_count
        ));
    }

    if unchanged_critical.count > 0 {
        remaining_issues.push(format!(
            "{} critical debt item{} still present",
            unchanged_critical.count,
            if unchanged_critical.count == 1 {
                ""
            } else {
                "s"
            }
        ));

        for (idx, item) in unchanged_critical.items.iter().take(2).enumerate() {
            gaps.insert(
                format!("critical_debt_remaining_{}", idx),
                GapDetail {
                    description: format!(
                        "High-priority debt item still present in {}",
                        item.function
                    ),
                    location: format!("{}:{}:{}", item.file.display(), item.function, item.line),
                    severity: "high".to_string(),
                    suggested_fix: "Apply functional programming patterns to reduce complexity"
                        .to_string(),
                    original_score: Some(item.score),
                    current_score: Some(item.score),
                    original_complexity: None,
                    current_complexity: None,
                    target_complexity: None,
                },
            );
        }
    }

    if new_items.critical_count > 0 {
        remaining_issues.push(format!(
            "{} new critical debt items introduced",
            new_items.critical_count
        ));

        gaps.insert(
            "regression_detected".to_string(),
            GapDetail {
                description: "New complexity introduced during refactoring".to_string(),
                location: new_items
                    .items
                    .first()
                    .map(|i| format!("{}:{}:{}", i.file.display(), i.function, i.line))
                    .unwrap_or_default(),
                severity: "high".to_string(),
                suggested_fix: "Simplify the newly added conditional logic".to_string(),
                original_score: None,
                current_score: new_items.items.first().map(|i| i.score),
                original_complexity: None,
                current_complexity: None,
                target_complexity: None,
            },
        );
    }

    let improvement_score = calculate_improvement_score(
        &resolved_items,
        &improved_items,
        &new_items,
        &unchanged_critical,
        &before_summary,
        &after_summary,
    );

    let status = if improvement_score >= 75.0 {
        "complete"
    } else if improvement_score >= 40.0 {
        "incomplete"
    } else {
        "failed"
    }
    .to_string();

    Ok(ValidationResult {
        completion_percentage: improvement_score,
        status,
        improvements,
        remaining_issues,
        gaps,
        before_summary,
        after_summary,
    })
}

fn create_summary(analysis: &UnifiedAnalysis) -> AnalysisSummary {
    let high_priority_items = analysis
        .items
        .iter()
        .filter(|item| item.unified_score.final_score >= 8.0)
        .count();

    let average_score = if analysis.items.is_empty() {
        0.0
    } else {
        analysis
            .items
            .iter()
            .map(|i| i.unified_score.final_score)
            .sum::<f64>()
            / analysis.items.len() as f64
    };

    AnalysisSummary {
        total_items: analysis.items.len(),
        high_priority_items,
        average_score,
    }
}

struct ResolvedItems {
    high_priority_count: usize,
    total_count: usize,
}

fn identify_resolved_items(before: &UnifiedAnalysis, after: &UnifiedAnalysis) -> ResolvedItems {
    let after_keys: HashSet<_> = after
        .items
        .iter()
        .map(|i| (i.location.file.clone(), i.location.function.clone()))
        .collect();

    let resolved: Vec<_> = before
        .items
        .iter()
        .filter(|item| {
            !after_keys.contains(&(item.location.file.clone(), item.location.function.clone()))
        })
        .collect();

    let high_priority_count = resolved
        .iter()
        .filter(|item| item.unified_score.final_score >= 8.0)
        .count();

    ResolvedItems {
        high_priority_count,
        total_count: resolved.len(),
    }
}

struct ImprovedItems {
    complexity_reduction: f64,
    coverage_improvement: f64,
    coverage_improvement_count: usize,
}

fn identify_improved_items(before: &UnifiedAnalysis, after: &UnifiedAnalysis) -> ImprovedItems {
    let before_map: HashMap<_, _> = before
        .items
        .iter()
        .map(|i| ((i.location.file.clone(), i.location.function.clone()), i))
        .collect();

    let mut total_complexity_reduction = 0.0;
    let mut coverage_improvement_count = 0;
    let mut improved_count = 0;

    for after_item in &after.items {
        let key = (
            after_item.location.file.clone(),
            after_item.location.function.clone(),
        );
        if let Some(before_item) = before_map.get(&key) {
            let score_improvement =
                before_item.unified_score.final_score - after_item.unified_score.final_score;
            if score_improvement > 0.5 {
                improved_count += 1;

                if after_item.cyclomatic_complexity < before_item.cyclomatic_complexity {
                    let reduction = (before_item.cyclomatic_complexity
                        - after_item.cyclomatic_complexity)
                        as f64
                        / before_item.cyclomatic_complexity as f64;
                    total_complexity_reduction += reduction;
                }

                let after_coverage = after_item
                    .transitive_coverage
                    .as_ref()
                    .map(|tc| tc.direct.max(tc.transitive))
                    .unwrap_or(0.0);
                let before_coverage = before_item
                    .transitive_coverage
                    .as_ref()
                    .map(|tc| tc.direct.max(tc.transitive))
                    .unwrap_or(0.0);

                if after_coverage > before_coverage {
                    coverage_improvement_count += 1;
                }
            }
        }
    }

    ImprovedItems {
        complexity_reduction: if improved_count > 0 {
            total_complexity_reduction / improved_count as f64
        } else {
            0.0
        },
        coverage_improvement: coverage_improvement_count as f64,
        coverage_improvement_count,
    }
}

struct NewItems {
    critical_count: usize,
    items: Vec<ItemInfo>,
}

struct ItemInfo {
    file: PathBuf,
    function: String,
    line: usize,
    score: f64,
}

fn identify_new_items(before: &UnifiedAnalysis, after: &UnifiedAnalysis) -> NewItems {
    let before_keys: HashSet<_> = before
        .items
        .iter()
        .map(|i| (i.location.file.clone(), i.location.function.clone()))
        .collect();

    let new_items: Vec<_> = after
        .items
        .iter()
        .filter(|item| {
            !before_keys.contains(&(item.location.file.clone(), item.location.function.clone()))
        })
        .filter(|item| item.unified_score.final_score >= 8.0)
        .map(|item| ItemInfo {
            file: item.location.file.clone(),
            function: item.location.function.clone(),
            line: item.location.line,
            score: item.unified_score.final_score,
        })
        .collect();

    NewItems {
        critical_count: new_items.len(),
        items: new_items,
    }
}

struct UnchangedCritical {
    count: usize,
    items: Vec<ItemInfo>,
}

fn identify_unchanged_critical(
    before: &UnifiedAnalysis,
    after: &UnifiedAnalysis,
) -> UnchangedCritical {
    let mut unchanged_critical = Vec::new();

    let after_map: HashMap<_, _> = after
        .items
        .iter()
        .map(|i| ((i.location.file.clone(), i.location.function.clone()), i))
        .collect();

    for before_item in &before.items {
        if before_item.unified_score.final_score >= 8.0 {
            let key = (
                before_item.location.file.clone(),
                before_item.location.function.clone(),
            );
            if let Some(after_item) = after_map.get(&key) {
                let score_change = (before_item.unified_score.final_score
                    - after_item.unified_score.final_score)
                    .abs();
                if score_change < 0.5 && after_item.unified_score.final_score >= 8.0 {
                    unchanged_critical.push(ItemInfo {
                        file: before_item.location.file.clone(),
                        function: before_item.location.function.clone(),
                        line: before_item.location.line,
                        score: before_item.unified_score.final_score,
                    });
                }
            }
        }
    }

    UnchangedCritical {
        count: unchanged_critical.len(),
        items: unchanged_critical,
    }
}

fn calculate_improvement_score(
    resolved: &ResolvedItems,
    improved: &ImprovedItems,
    new_items: &NewItems,
    unchanged_critical: &UnchangedCritical,
    before_summary: &AnalysisSummary,
    after_summary: &AnalysisSummary,
) -> f64 {
    let resolved_high_priority_score = if before_summary.high_priority_items > 0 {
        (resolved.high_priority_count as f64 / before_summary.high_priority_items as f64) * 100.0
    } else {
        100.0
    };

    let overall_score_improvement = if before_summary.average_score > 0.0 {
        ((before_summary.average_score - after_summary.average_score)
            / before_summary.average_score)
            * 100.0
    } else {
        0.0
    };

    let complexity_reduction_score = improved.complexity_reduction * 100.0;

    let no_new_critical_score = if new_items.critical_count == 0 {
        100.0
    } else {
        0.0
    };

    let weighted_score = resolved_high_priority_score * 0.4
        + overall_score_improvement.max(0.0) * 0.3
        + complexity_reduction_score * 0.2
        + no_new_critical_score * 0.1;

    let penalty = unchanged_critical.count as f64 * 5.0;

    (weighted_score - penalty).clamp(0.0, 100.0)
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

fn print_summary(result: &ValidationResult) {
    println!("\n=== Debtmap Validation Results ===");
    println!("Completion: {:.1}%", result.completion_percentage);
    println!("Status: {}", result.status);

    if !result.improvements.is_empty() {
        println!("\nImprovements:");
        for improvement in &result.improvements {
            println!("  ✓ {}", improvement);
        }
    }

    if !result.remaining_issues.is_empty() {
        println!("\nRemaining Issues:");
        for issue in &result.remaining_issues {
            println!("  ✗ {}", issue);
        }
    }

    println!(
        "\nBefore: {} items (avg score: {:.1})",
        result.before_summary.total_items, result.before_summary.average_score
    );
    println!(
        "After: {} items (avg score: {:.1})",
        result.after_summary.total_items, result.after_summary.average_score
    );
}
