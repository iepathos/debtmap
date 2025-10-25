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
        .filter(|item| is_critical(item.unified_score.final_score))
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
        .filter(|item| is_critical(item.unified_score.final_score))
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
                    && is_critical(f.unified_score.final_score)
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

/// Threshold for identifying critical debt items
const CRITICAL_SCORE_THRESHOLD: f64 = 8.0;

/// Check if a score is considered critical (≥ threshold)
fn is_critical(score: f64) -> bool {
    score >= CRITICAL_SCORE_THRESHOLD
}

/// Tolerance for considering scores as "unchanged"
const SCORE_CHANGE_TOLERANCE: f64 = 0.5;

/// Check if two scores are considered unchanged (absolute difference < tolerance)
fn is_score_unchanged(before: f64, after: f64) -> bool {
    (before - after).abs() < SCORE_CHANGE_TOLERANCE
}

/// Build a map of (file, function) -> FunctionMetrics for quick lookup
fn build_function_map(
    items: &[crate::priority::DebtItem],
) -> HashMap<(PathBuf, String), &crate::priority::UnifiedDebtItem> {
    use crate::priority::DebtItem;

    items
        .iter()
        .filter_map(|item| match item {
            DebtItem::Function(f) => Some((
                (f.location.file.clone(), f.location.function.clone()),
                f.as_ref(),
            )),
            DebtItem::File(_) => None,
        })
        .collect()
}

fn identify_unchanged_critical(
    before: &UnifiedJsonOutput,
    after: &UnifiedJsonOutput,
) -> UnchangedCritical {
    use crate::priority::DebtItem;

    let mut unchanged_critical = Vec::new();

    let after_map = build_function_map(&after.items);

    for item in &before.items {
        if let DebtItem::Function(before_item) = item {
            if is_critical(before_item.unified_score.final_score) {
                let key = (
                    before_item.location.file.clone(),
                    before_item.location.function.clone(),
                );
                if let Some(after_item) = after_map.get(&key) {
                    if is_score_unchanged(
                        before_item.unified_score.final_score,
                        after_item.unified_score.final_score,
                    ) && is_critical(after_item.unified_score.final_score)
                    {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::output::json::UnifiedJsonOutput;
    use crate::priority::semantic_classifier::FunctionRole;
    use crate::priority::unified_scorer::{Location, UnifiedScore};
    use crate::priority::{
        ActionableRecommendation, DebtItem, DebtType, ImpactMetrics, UnifiedDebtItem,
    };
    use std::path::PathBuf;

    // Helper function to create test UnifiedDebtItem
    fn create_test_debt_item(
        file: &str,
        function: &str,
        line: usize,
        score: f64,
    ) -> UnifiedDebtItem {
        UnifiedDebtItem {
            location: Location {
                file: PathBuf::from(file),
                function: function.to_string(),
                line,
            },
            debt_type: DebtType::ComplexityHotspot {
                cyclomatic: 5,
                cognitive: 8,
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
            function_role: FunctionRole::PureLogic,
            recommendation: ActionableRecommendation {
                primary_action: "Test".into(),
                rationale: "Test".into(),
                implementation_steps: vec![],
                related_items: vec![],
            },
            expected_impact: ImpactMetrics {
                risk_reduction: 0.0,
                complexity_reduction: 0.0,
                coverage_improvement: 0.0,
                lines_reduction: 0,
            },
            transitive_coverage: None,
            upstream_dependencies: 0,
            downstream_dependencies: 0,
            upstream_callers: vec![],
            downstream_callees: vec![],
            nesting_depth: 1,
            function_length: 10,
            cyclomatic_complexity: 5,
            cognitive_complexity: 8,
            entropy_details: None,
            is_pure: Some(false),
            purity_confidence: Some(0.0),
            god_object_indicators: None,
            tier: None,
        }
    }

    // Helper function to create UnifiedJsonOutput
    fn create_test_output(items: Vec<DebtItem>) -> UnifiedJsonOutput {
        UnifiedJsonOutput {
            items,
            total_impact: ImpactMetrics {
                risk_reduction: 0.0,
                complexity_reduction: 0.0,
                coverage_improvement: 0.0,
                lines_reduction: 0,
            },
            total_debt_score: 0.0,
            debt_density: 0.0,
            total_lines_of_code: 1000,
            overall_coverage: Some(50.0),
        }
    }

    #[test]
    fn test_identify_unchanged_critical_empty_inputs() {
        let before = create_test_output(vec![]);
        let after = create_test_output(vec![]);

        let result = identify_unchanged_critical(&before, &after);

        assert_eq!(result.count, 0);
        assert_eq!(result.items.len(), 0);
    }

    #[test]
    fn test_identify_unchanged_critical_no_critical_items() {
        // All scores below 8.0
        let before_items = vec![
            DebtItem::Function(Box::new(create_test_debt_item(
                "src/foo.rs",
                "low_score",
                10,
                5.0,
            ))),
            DebtItem::Function(Box::new(create_test_debt_item(
                "src/bar.rs",
                "another_low",
                20,
                7.5,
            ))),
        ];
        let after_items = vec![
            DebtItem::Function(Box::new(create_test_debt_item(
                "src/foo.rs",
                "low_score",
                10,
                5.2,
            ))),
            DebtItem::Function(Box::new(create_test_debt_item(
                "src/bar.rs",
                "another_low",
                20,
                7.3,
            ))),
        ];

        let before = create_test_output(before_items);
        let after = create_test_output(after_items);

        let result = identify_unchanged_critical(&before, &after);

        assert_eq!(result.count, 0);
        assert_eq!(result.items.len(), 0);
    }

    #[test]
    fn test_identify_unchanged_critical_items_resolved() {
        // Critical items in before, not in after (function removed)
        let before_items = vec![DebtItem::Function(Box::new(create_test_debt_item(
            "src/foo.rs",
            "critical_fn",
            10,
            9.0,
        )))];
        let after_items = vec![];

        let before = create_test_output(before_items);
        let after = create_test_output(after_items);

        let result = identify_unchanged_critical(&before, &after);

        assert_eq!(result.count, 0);
        assert_eq!(result.items.len(), 0);
    }

    #[test]
    fn test_identify_unchanged_critical_items_unchanged() {
        // Critical items with same score (±0.5)
        let before_items = vec![
            DebtItem::Function(Box::new(create_test_debt_item(
                "src/foo.rs",
                "critical_fn",
                10,
                9.0,
            ))),
            DebtItem::Function(Box::new(create_test_debt_item(
                "src/bar.rs",
                "another_critical",
                20,
                10.5,
            ))),
        ];
        let after_items = vec![
            DebtItem::Function(Box::new(create_test_debt_item(
                "src/foo.rs",
                "critical_fn",
                10,
                9.2,
            ))),
            DebtItem::Function(Box::new(create_test_debt_item(
                "src/bar.rs",
                "another_critical",
                20,
                10.3,
            ))),
        ];

        let before = create_test_output(before_items);
        let after = create_test_output(after_items);

        let result = identify_unchanged_critical(&before, &after);

        assert_eq!(result.count, 2);
        assert_eq!(result.items.len(), 2);
        assert_eq!(result.items[0].function, "critical_fn");
        assert_eq!(result.items[0].score, 9.0);
        assert_eq!(result.items[1].function, "another_critical");
        assert_eq!(result.items[1].score, 10.5);
    }

    #[test]
    fn test_identify_unchanged_critical_items_improved_significantly() {
        // Critical items where score drops > 0.5 (improved)
        let before_items = vec![DebtItem::Function(Box::new(create_test_debt_item(
            "src/foo.rs",
            "improved_fn",
            10,
            10.0,
        )))];
        let after_items = vec![DebtItem::Function(Box::new(create_test_debt_item(
            "src/foo.rs",
            "improved_fn",
            10,
            9.0,
        )))];

        let before = create_test_output(before_items);
        let after = create_test_output(after_items);

        let result = identify_unchanged_critical(&before, &after);

        assert_eq!(result.count, 0);
        assert_eq!(result.items.len(), 0);
    }

    #[test]
    fn test_identify_unchanged_critical_items_worsened_but_stays_critical() {
        // Critical items where score increases but both stay >= 8.0
        let before_items = vec![DebtItem::Function(Box::new(create_test_debt_item(
            "src/foo.rs",
            "worsened_fn",
            10,
            8.0,
        )))];
        let after_items = vec![DebtItem::Function(Box::new(create_test_debt_item(
            "src/foo.rs",
            "worsened_fn",
            10,
            9.0,
        )))];

        let before = create_test_output(before_items);
        let after = create_test_output(after_items);

        let result = identify_unchanged_critical(&before, &after);

        // Score change is 1.0, which is > 0.5, so it should NOT be included
        assert_eq!(result.count, 0);
        assert_eq!(result.items.len(), 0);
    }

    #[test]
    fn test_identify_unchanged_critical_mixed_scenario() {
        // Mix of unchanged, resolved, and improved
        let before_items = vec![
            DebtItem::Function(Box::new(create_test_debt_item(
                "src/a.rs",
                "unchanged",
                10,
                9.0,
            ))),
            DebtItem::Function(Box::new(create_test_debt_item(
                "src/b.rs", "resolved", 20, 8.5,
            ))),
            DebtItem::Function(Box::new(create_test_debt_item(
                "src/c.rs", "improved", 30, 10.0,
            ))),
            DebtItem::Function(Box::new(create_test_debt_item(
                "src/d.rs",
                "not_critical",
                40,
                7.0,
            ))),
        ];
        let after_items = vec![
            DebtItem::Function(Box::new(create_test_debt_item(
                "src/a.rs",
                "unchanged",
                10,
                9.1,
            ))),
            // resolved is missing
            DebtItem::Function(Box::new(create_test_debt_item(
                "src/c.rs", "improved", 30, 8.5,
            ))),
            DebtItem::Function(Box::new(create_test_debt_item(
                "src/d.rs",
                "not_critical",
                40,
                7.2,
            ))),
        ];

        let before = create_test_output(before_items);
        let after = create_test_output(after_items);

        let result = identify_unchanged_critical(&before, &after);

        assert_eq!(result.count, 1);
        assert_eq!(result.items.len(), 1);
        assert_eq!(result.items[0].function, "unchanged");
        assert_eq!(result.items[0].score, 9.0);
    }

    #[test]
    fn test_identify_unchanged_critical_at_boundary() {
        // Test edge case where score change is exactly 0.5
        let before_items = vec![DebtItem::Function(Box::new(create_test_debt_item(
            "src/foo.rs",
            "boundary_fn",
            10,
            9.0,
        )))];
        let after_items = vec![DebtItem::Function(Box::new(create_test_debt_item(
            "src/foo.rs",
            "boundary_fn",
            10,
            8.5,
        )))];

        let before = create_test_output(before_items);
        let after = create_test_output(after_items);

        let result = identify_unchanged_critical(&before, &after);

        // Score change is exactly 0.5, which is NOT < 0.5, so should not be included
        assert_eq!(result.count, 0);
        assert_eq!(result.items.len(), 0);
    }

    #[test]
    fn test_identify_unchanged_critical_after_becomes_non_critical() {
        // Item is critical in before, but drops below 8.0 in after
        let before_items = vec![DebtItem::Function(Box::new(create_test_debt_item(
            "src/foo.rs",
            "fixed_fn",
            10,
            9.0,
        )))];
        let after_items = vec![DebtItem::Function(Box::new(create_test_debt_item(
            "src/foo.rs",
            "fixed_fn",
            10,
            7.5,
        )))];

        let before = create_test_output(before_items);
        let after = create_test_output(after_items);

        let result = identify_unchanged_critical(&before, &after);

        // After score is < 8.0, so should not be included
        assert_eq!(result.count, 0);
        assert_eq!(result.items.len(), 0);
    }

    // Tests for is_critical
    #[test]
    fn test_is_critical_below_threshold() {
        assert!(!is_critical(7.9));
        assert!(!is_critical(0.0));
        assert!(!is_critical(5.5));
    }

    #[test]
    fn test_is_critical_at_threshold() {
        assert!(is_critical(8.0));
    }

    #[test]
    fn test_is_critical_above_threshold() {
        assert!(is_critical(8.1));
        assert!(is_critical(10.0));
        assert!(is_critical(15.5));
    }

    // Tests for is_score_unchanged
    #[test]
    fn test_is_score_unchanged_exactly_equal() {
        assert!(is_score_unchanged(9.0, 9.0));
        assert!(is_score_unchanged(0.0, 0.0));
    }

    #[test]
    fn test_is_score_unchanged_within_tolerance() {
        assert!(is_score_unchanged(9.0, 9.3));
        assert!(is_score_unchanged(9.3, 9.0));
        assert!(is_score_unchanged(10.0, 10.49));
        assert!(is_score_unchanged(10.49, 10.0));
    }

    #[test]
    fn test_is_score_unchanged_at_boundary() {
        // Exactly at tolerance boundary (0.5) should NOT be considered unchanged
        assert!(!is_score_unchanged(9.0, 8.5));
        assert!(!is_score_unchanged(8.5, 9.0));
    }

    #[test]
    fn test_is_score_unchanged_outside_tolerance() {
        assert!(!is_score_unchanged(9.0, 8.4));
        assert!(!is_score_unchanged(8.4, 9.0));
        assert!(!is_score_unchanged(10.0, 11.0));
        assert!(!is_score_unchanged(5.0, 7.0));
    }

    // Tests for build_function_map
    #[test]
    fn test_build_function_map_empty() {
        let items: Vec<DebtItem> = vec![];
        let result = build_function_map(&items);
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_build_function_map_only_functions() {
        let items = vec![
            DebtItem::Function(Box::new(create_test_debt_item(
                "src/foo.rs",
                "func1",
                10,
                9.0,
            ))),
            DebtItem::Function(Box::new(create_test_debt_item(
                "src/bar.rs",
                "func2",
                20,
                8.5,
            ))),
        ];

        let result = build_function_map(&items);

        assert_eq!(result.len(), 2);
        assert!(result.contains_key(&(PathBuf::from("src/foo.rs"), "func1".to_string())));
        assert!(result.contains_key(&(PathBuf::from("src/bar.rs"), "func2".to_string())));
    }

    #[test]
    fn test_build_function_map_filters_file_items() {
        use crate::priority::{
            file_metrics::{FileDebtMetrics, FileImpact, GodObjectIndicators},
            FileDebtItem,
        };

        let metrics = FileDebtMetrics {
            path: PathBuf::from("src/foo.rs"),
            total_lines: 100,
            function_count: 5,
            class_count: 1,
            avg_complexity: 5.0,
            max_complexity: 10,
            total_complexity: 25,
            coverage_percent: 50.0,
            uncovered_lines: 50,
            god_object_indicators: GodObjectIndicators {
                methods_count: 5,
                fields_count: 3,
                responsibilities: 2,
                is_god_object: false,
                god_object_score: 0.5,
                responsibility_names: vec![],
                recommended_splits: vec![],
            },
            function_scores: vec![],
        };

        let items = vec![
            DebtItem::Function(Box::new(create_test_debt_item(
                "src/foo.rs",
                "func1",
                10,
                9.0,
            ))),
            DebtItem::File(Box::new(FileDebtItem {
                metrics,
                score: 10.0,
                priority_rank: 0,
                recommendation: "Test".into(),
                impact: FileImpact {
                    complexity_reduction: 0.0,
                    maintainability_improvement: 0.0,
                    test_effort: 0.0,
                },
            })),
        ];

        let result = build_function_map(&items);

        // Only the function item should be in the map
        assert_eq!(result.len(), 1);
        assert!(result.contains_key(&(PathBuf::from("src/foo.rs"), "func1".to_string())));
    }

    #[test]
    fn test_identify_unchanged_critical_file_items_ignored() {
        // File items should be ignored (only functions are processed)
        use crate::priority::{
            file_metrics::{FileDebtMetrics, FileImpact, GodObjectIndicators},
            FileDebtItem,
        };

        let metrics = FileDebtMetrics {
            path: PathBuf::from("src/foo.rs"),
            total_lines: 100,
            function_count: 5,
            class_count: 1,
            avg_complexity: 5.0,
            max_complexity: 10,
            total_complexity: 25,
            coverage_percent: 50.0,
            uncovered_lines: 50,
            god_object_indicators: GodObjectIndicators {
                methods_count: 5,
                fields_count: 3,
                responsibilities: 2,
                is_god_object: false,
                god_object_score: 0.5,
                responsibility_names: vec![],
                recommended_splits: vec![],
            },
            function_scores: vec![],
        };

        let file_item = DebtItem::File(Box::new(FileDebtItem {
            metrics,
            score: 10.0,
            priority_rank: 0,
            recommendation: "Test".into(),
            impact: FileImpact {
                complexity_reduction: 0.0,
                maintainability_improvement: 0.0,
                test_effort: 0.0,
            },
        }));

        let before_items = vec![file_item.clone()];
        let after_items = vec![file_item];

        let before = create_test_output(before_items);
        let after = create_test_output(after_items);

        let result = identify_unchanged_critical(&before, &after);

        assert_eq!(result.count, 0);
        assert_eq!(result.items.len(), 0);
    }
}
