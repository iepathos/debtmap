use crate::output::json::UnifiedJsonOutput;
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

fn load_debtmap(path: &Path) -> Result<UnifiedJsonOutput> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read debtmap file: {}", path.display()))?;

    serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse debtmap JSON from: {}", path.display()))
}

fn perform_validation(
    before: &UnifiedJsonOutput,
    after: &UnifiedJsonOutput,
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

    // Determine status based on functional composition of conditions
    let has_regressions = new_items.critical_count > 0;
    let all_high_priority_addressed = before_summary.high_priority_items > 0
        && after_summary.high_priority_items == 0;
    let meets_score_threshold = improvement_score >= 75.0;

    let status = if has_regressions {
        "failed"
    } else if all_high_priority_addressed || meets_score_threshold {
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

fn create_summary(analysis: &UnifiedJsonOutput) -> AnalysisSummary {
    use crate::priority::DebtItem;

    // Only count Function items for summary
    let function_items: Vec<_> = analysis
        .items
        .iter()
        .filter_map(|item| match item {
            DebtItem::Function(f) => Some(f.as_ref()),
            DebtItem::File(_) => None,
        })
        .collect();

    let high_priority_items = function_items
        .iter()
        .filter(|item| item.unified_score.final_score >= 8.0)
        .count();

    let average_score = if function_items.is_empty() {
        0.0
    } else {
        function_items
            .iter()
            .map(|i| i.unified_score.final_score)
            .sum::<f64>()
            / function_items.len() as f64
    };

    AnalysisSummary {
        total_items: function_items.len(),
        high_priority_items,
        average_score,
    }
}

struct ResolvedItems {
    high_priority_count: usize,
    #[allow(dead_code)]
    total_count: usize,
}

fn identify_resolved_items(before: &UnifiedJsonOutput, after: &UnifiedJsonOutput) -> ResolvedItems {
    use crate::priority::DebtItem;

    // Extract Function items only
    let after_keys: HashSet<_> = after
        .items
        .iter()
        .filter_map(|item| match item {
            DebtItem::Function(f) => Some((f.location.file.clone(), f.location.function.clone())),
            DebtItem::File(_) => None,
        })
        .collect();

    let resolved: Vec<_> = before
        .items
        .iter()
        .filter_map(|item| match item {
            DebtItem::Function(f) => {
                if !after_keys.contains(&(f.location.file.clone(), f.location.function.clone())) {
                    Some(f.as_ref())
                } else {
                    None
                }
            }
            DebtItem::File(_) => None,
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

fn identify_improved_items(before: &UnifiedJsonOutput, after: &UnifiedJsonOutput) -> ImprovedItems {
    use crate::priority::DebtItem;

    let before_map: HashMap<_, _> = before
        .items
        .iter()
        .filter_map(|item| match item {
            DebtItem::Function(f) => {
                Some(((f.location.file.clone(), f.location.function.clone()), f))
            }
            DebtItem::File(_) => None,
        })
        .collect();

    let mut total_complexity_reduction = 0.0;
    let mut coverage_improvement_count = 0;
    let mut improved_count = 0;

    for item in &after.items {
        if let DebtItem::Function(after_item) = item {
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

fn identify_new_items(before: &UnifiedJsonOutput, after: &UnifiedJsonOutput) -> NewItems {
    use crate::priority::DebtItem;

    let before_keys: HashSet<_> = before
        .items
        .iter()
        .filter_map(|item| match item {
            DebtItem::Function(f) => Some((f.location.file.clone(), f.location.function.clone())),
            DebtItem::File(_) => None,
        })
        .collect();

    let new_items: Vec<_> = after
        .items
        .iter()
        .filter_map(|item| match item {
            DebtItem::Function(f) => {
                if !before_keys.contains(&(f.location.file.clone(), f.location.function.clone()))
                    && f.unified_score.final_score >= 8.0
                {
                    Some(ItemInfo {
                        file: f.location.file.clone(),
                        function: f.location.function.clone(),
                        line: f.location.line,
                        score: f.unified_score.final_score,
                    })
                } else {
                    None
                }
            }
            DebtItem::File(_) => None,
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
    before: &UnifiedJsonOutput,
    after: &UnifiedJsonOutput,
) -> UnchangedCritical {
    use crate::priority::DebtItem;

    let mut unchanged_critical = Vec::new();

    let after_map: HashMap<_, _> = after
        .items
        .iter()
        .filter_map(|item| match item {
            DebtItem::Function(f) => {
                Some(((f.location.file.clone(), f.location.function.clone()), f))
            }
            DebtItem::File(_) => None,
        })
        .collect();

    for item in &before.items {
        if let DebtItem::Function(before_item) = item {
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
    // If both before and after have no items, it's 100% complete (nothing to do)
    if before_summary.total_items == 0 && after_summary.total_items == 0 {
        return 100.0;
    }

    // Calculate high-priority resolution score
    let high_priority_progress = if before_summary.high_priority_items > 0 {
        let resolved_count = resolved.high_priority_count as f64;
        // Use saturating subtraction to handle cases where after > before (regressions)
        let addressed_count = before_summary.high_priority_items.saturating_sub(after_summary.high_priority_items) as f64;
        // Use the better of resolved or addressed (items may improve below threshold without being removed)
        (addressed_count.max(resolved_count) / before_summary.high_priority_items as f64) * 100.0
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

    let weighted_score = high_priority_progress * 0.4
        + overall_score_improvement.max(0.0) * 0.3
        + complexity_reduction_score * 0.2
        + no_new_critical_score * 0.1;

    // Apply penalty for unchanged critical items, but ensure progress is still reflected
    // If there are improvements (complexity reduction or coverage), reduce the penalty impact
    let has_improvements = complexity_reduction_score > 0.0 || overall_score_improvement > 0.0;
    let penalty_factor = if unchanged_critical.count > 0 && !has_improvements {
        1.0 - (unchanged_critical.count as f64 * 0.1).min(0.5)
    } else if unchanged_critical.count > 0 {
        // Lighter penalty when there are improvements
        1.0 - (unchanged_critical.count as f64 * 0.05).min(0.25)
    } else {
        1.0
    };

    // Ensure minimum score of 40% when there are significant improvements
    let final_score = weighted_score * penalty_factor;
    if has_improvements && final_score < 40.0 && overall_score_improvement > 5.0 {
        40.0
    } else {
        final_score.clamp(0.0, 100.0)
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::output::json::UnifiedJsonOutput;
    use crate::priority::{DebtItem, unified_scorer::{UnifiedDebtItem, Location, UnifiedScore}, coverage_propagation::TransitiveCoverage};
    use std::path::PathBuf;

    fn create_function_item(
        file: &str,
        function: &str,
        line: usize,
        score: f64,
        complexity: u32,
        coverage: Option<f64>,
    ) -> DebtItem {
        DebtItem::Function(Box::new(UnifiedDebtItem {
            location: Location {
                file: PathBuf::from(file),
                function: function.to_string(),
                line,
            },
            debt_type: crate::priority::DebtType::ComplexityHotspot {
                cyclomatic: complexity,
                cognitive: 0,
            },
            unified_score: UnifiedScore {
                complexity_factor: 0.0,
                coverage_factor: 0.0,
                dependency_factor: 0.0,
                role_multiplier: 1.0,
                final_score: score,
                pre_adjustment_score: None,
                adjustment_applied: None,
            },
            function_role: crate::priority::semantic_classifier::FunctionRole::Unknown,
            recommendation: crate::priority::ActionableRecommendation {
                primary_action: String::new(),
                rationale: String::new(),
                implementation_steps: vec![],
                related_items: vec![],
            },
            expected_impact: crate::priority::ImpactMetrics {
                coverage_improvement: 0.0,
                lines_reduction: 0,
                complexity_reduction: 0.0,
                risk_reduction: 0.0,
            },
            transitive_coverage: coverage.map(|c| TransitiveCoverage {
                direct: c,
                transitive: c,
                propagated_from: vec![],
                uncovered_lines: vec![],
            }),
            upstream_dependencies: 0,
            downstream_dependencies: 0,
            upstream_callers: vec![],
            downstream_callees: vec![],
            nesting_depth: 0,
            function_length: 50,
            cyclomatic_complexity: complexity,
            cognitive_complexity: 0,
            entropy_details: None,
            is_pure: None,
            purity_confidence: None,
            god_object_indicators: None,
            tier: None,
        }))
    }

    fn create_test_output(items: Vec<DebtItem>) -> UnifiedJsonOutput {
        UnifiedJsonOutput {
            items,
            total_impact: crate::priority::ImpactMetrics {
                coverage_improvement: 0.0,
                lines_reduction: 0,
                complexity_reduction: 0.0,
                risk_reduction: 0.0,
            },
            total_debt_score: 0.0,
            debt_density: 0.0,
            total_lines_of_code: 0,
            overall_coverage: None,
        }
    }

    #[test]
    fn test_perform_validation_no_improvements_or_issues() {
        let before = create_test_output(vec![]);
        let after = create_test_output(vec![]);

        let result = perform_validation(&before, &after).unwrap();

        assert_eq!(result.status, "complete");
        assert_eq!(result.improvements.len(), 0);
        assert_eq!(result.remaining_issues.len(), 0);
        assert_eq!(result.gaps.len(), 0);
        assert!(result.completion_percentage >= 75.0);
    }

    #[test]
    fn test_perform_validation_resolved_high_priority() {
        let before = create_test_output(vec![
            create_function_item("src/test.rs", "complex_fn", 10, 10.0, 15, Some(0.0)),
        ]);
        let after = create_test_output(vec![]);

        let result = perform_validation(&before, &after).unwrap();

        assert_eq!(result.status, "complete");
        assert!(result.improvements.iter().any(|i| i.contains("Resolved 1 high-priority")));
        assert_eq!(result.remaining_issues.len(), 0);
        assert!(result.completion_percentage >= 75.0);
    }

    #[test]
    fn test_perform_validation_complexity_reduction() {
        let before = create_test_output(vec![
            create_function_item("src/test.rs", "fn1", 10, 10.0, 20, Some(0.5)),
        ]);
        let after = create_test_output(vec![
            create_function_item("src/test.rs", "fn1", 10, 8.0, 10, Some(0.5)),
        ]);

        let result = perform_validation(&before, &after).unwrap();

        assert!(result.improvements.iter().any(|i| i.contains("Reduced average cyclomatic complexity")));
    }

    #[test]
    fn test_perform_validation_coverage_improvement() {
        let before = create_test_output(vec![
            create_function_item("src/test.rs", "fn1", 10, 10.0, 10, Some(0.0)),
        ]);
        let after = create_test_output(vec![
            create_function_item("src/test.rs", "fn1", 10, 8.0, 10, Some(0.8)),
        ]);

        let result = perform_validation(&before, &after).unwrap();

        assert!(result.improvements.iter().any(|i| i.contains("Added test coverage")));
    }

    #[test]
    fn test_perform_validation_unchanged_critical() {
        let before = create_test_output(vec![
            create_function_item("src/test.rs", "complex_fn", 10, 10.0, 15, Some(0.0)),
        ]);
        let after = create_test_output(vec![
            create_function_item("src/test.rs", "complex_fn", 10, 10.0, 15, Some(0.0)),
        ]);

        let result = perform_validation(&before, &after).unwrap();

        assert!(result.remaining_issues.iter().any(|i| i.contains("critical debt item")));
        assert!(result.gaps.contains_key("critical_debt_remaining_0"));
    }

    #[test]
    fn test_perform_validation_new_critical_regression() {
        let before = create_test_output(vec![]);
        let after = create_test_output(vec![
            create_function_item("src/new.rs", "bad_fn", 20, 12.0, 20, Some(0.0)),
        ]);

        let result = perform_validation(&before, &after).unwrap();

        assert!(result.remaining_issues.iter().any(|i| i.contains("new critical debt items")));
        assert!(result.gaps.contains_key("regression_detected"));
        assert_eq!(result.status, "failed");
    }

    #[test]
    fn test_perform_validation_combined_improvements() {
        let before = create_test_output(vec![
            create_function_item("src/test.rs", "fn1", 10, 10.0, 20, Some(0.0)),
            create_function_item("src/test.rs", "fn2", 30, 9.0, 15, Some(0.2)),
        ]);
        let after = create_test_output(vec![
            create_function_item("src/test.rs", "fn2", 30, 7.0, 10, Some(0.8)),
        ]);

        let result = perform_validation(&before, &after).unwrap();

        assert!(result.improvements.len() >= 2);
        assert!(result.improvements.iter().any(|i| i.contains("Resolved")));
        assert!(result.improvements.iter().any(|i| i.contains("complexity") || i.contains("coverage")));
        assert_eq!(result.status, "complete");
    }

    #[test]
    fn test_perform_validation_status_complete() {
        let before = create_test_output(vec![
            create_function_item("src/test.rs", "fn1", 10, 10.0, 15, Some(0.0)),
        ]);
        let after = create_test_output(vec![]);

        let result = perform_validation(&before, &after).unwrap();

        assert_eq!(result.status, "complete");
        assert!(result.completion_percentage >= 75.0);
    }

    #[test]
    fn test_perform_validation_status_incomplete() {
        let before = create_test_output(vec![
            create_function_item("src/test.rs", "fn1", 10, 10.0, 15, Some(0.0)),
            create_function_item("src/test.rs", "fn2", 20, 11.0, 20, Some(0.0)),
        ]);
        let after = create_test_output(vec![
            create_function_item("src/test.rs", "fn1", 10, 8.0, 10, Some(0.5)),
            create_function_item("src/test.rs", "fn2", 20, 11.0, 20, Some(0.0)),
        ]);

        let result = perform_validation(&before, &after).unwrap();

        assert!(result.completion_percentage >= 40.0 && result.completion_percentage < 75.0);
        assert_eq!(result.status, "incomplete");
    }

    #[test]
    fn test_perform_validation_status_failed() {
        let before = create_test_output(vec![
            create_function_item("src/test.rs", "fn1", 10, 10.0, 15, Some(0.0)),
        ]);
        let after = create_test_output(vec![
            create_function_item("src/test.rs", "fn1", 10, 10.0, 15, Some(0.0)),
            create_function_item("src/test.rs", "fn2", 20, 12.0, 20, Some(0.0)),
        ]);

        let result = perform_validation(&before, &after).unwrap();

        assert!(result.completion_percentage < 40.0);
        assert_eq!(result.status, "failed");
    }

    #[test]
    fn test_perform_validation_gap_detail_generation() {
        let before = create_test_output(vec![
            create_function_item("src/test.rs", "critical_fn", 10, 10.0, 15, Some(0.0)),
        ]);
        let after = create_test_output(vec![
            create_function_item("src/test.rs", "critical_fn", 10, 10.0, 15, Some(0.0)),
        ]);

        let result = perform_validation(&before, &after).unwrap();

        assert!(result.gaps.contains_key("critical_debt_remaining_0"));
        let gap = result.gaps.get("critical_debt_remaining_0").unwrap();
        assert_eq!(gap.severity, "high");
        assert!(gap.location.contains("src/test.rs"));
        assert!(gap.location.contains("critical_fn"));
        assert_eq!(gap.original_score, Some(10.0));
        assert_eq!(gap.current_score, Some(10.0));
    }

    #[test]
    fn test_perform_validation_multiple_unchanged_critical() {
        let before = create_test_output(vec![
            create_function_item("src/test.rs", "fn1", 10, 10.0, 15, Some(0.0)),
            create_function_item("src/test.rs", "fn2", 20, 11.0, 20, Some(0.0)),
            create_function_item("src/test.rs", "fn3", 30, 12.0, 25, Some(0.0)),
        ]);
        let after = create_test_output(vec![
            create_function_item("src/test.rs", "fn1", 10, 10.0, 15, Some(0.0)),
            create_function_item("src/test.rs", "fn2", 20, 11.0, 20, Some(0.0)),
            create_function_item("src/test.rs", "fn3", 30, 12.0, 25, Some(0.0)),
        ]);

        let result = perform_validation(&before, &after).unwrap();

        assert!(result.remaining_issues.iter().any(|i| i.contains("3 critical debt items")));
        assert_eq!(result.gaps.len(), 2); // Only first 2 are added
        assert!(result.gaps.contains_key("critical_debt_remaining_0"));
        assert!(result.gaps.contains_key("critical_debt_remaining_1"));
    }
}
