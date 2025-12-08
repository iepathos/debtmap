use crate::priority::{DebtItem, ImpactMetrics};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

/// Internal type for parsing debtmap JSON files during comparison.
/// This supports parsing the unified JSON format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebtmapJsonInput {
    pub items: Vec<DebtItem>,
    #[serde(default = "default_impact_metrics")]
    pub total_impact: ImpactMetrics,
    #[serde(default)]
    pub total_debt_score: f64,
    #[serde(default)]
    pub debt_density: f64,
    #[serde(default)]
    pub total_lines_of_code: usize,
    #[serde(default)]
    pub overall_coverage: Option<f64>,
}

fn default_impact_metrics() -> ImpactMetrics {
    ImpactMetrics {
        complexity_reduction: 0.0,
        coverage_improvement: 0.0,
        risk_reduction: 0.0,
        lines_reduction: 0,
    }
}

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

// =============================================================================
// I/O Shell Functions (Side Effects at Boundaries)
// =============================================================================

/// I/O: Read automation mode from environment variables
fn read_automation_mode() -> bool {
    std::env::var("PRODIGY_AUTOMATION")
        .unwrap_or_default()
        .eq_ignore_ascii_case("true")
        || std::env::var("PRODIGY_VALIDATION")
            .unwrap_or_default()
            .eq_ignore_ascii_case("true")
}

/// I/O: Load both debtmap files
fn load_both_debtmaps(config: &CompareConfig) -> Result<(DebtmapJsonInput, DebtmapJsonInput)> {
    let before = load_debtmap(&config.before_path)?;
    let after = load_debtmap(&config.after_path)?;
    Ok((before, after))
}

/// I/O: Load single debtmap file
fn load_debtmap(path: &Path) -> Result<DebtmapJsonInput> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read debtmap file: {}", path.display()))?;

    serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse debtmap JSON from: {}", path.display()))
}

/// I/O Shell: Main entry point - orchestrates I/O and delegates to pure validation
pub fn compare_debtmaps(config: CompareConfig) -> Result<()> {
    let is_automation = read_automation_mode();

    if !is_automation {
        println!("Loading debtmap data from before and after states...");
    }

    let (before, after) = load_both_debtmaps(&config)?;
    let result = perform_validation(&before, &after)?; // Pure!

    write_validation_result(&config.output_path, &result)?;

    if !is_automation {
        print_summary(&result);
    }

    Ok(())
}

// =============================================================================
// Pure Message Builder Functions
// =============================================================================

/// Pure: Build resolved items improvement message
fn build_resolved_message(resolved: &ResolvedItems) -> Option<String> {
    if resolved.high_priority_count > 0 {
        Some(format!(
            "Resolved {} high-priority debt items",
            resolved.high_priority_count
        ))
    } else {
        None
    }
}

/// Pure: Build complexity reduction improvement message
fn build_complexity_message(improved: &ImprovedItems) -> Option<String> {
    if improved.complexity_reduction > 0.0 {
        Some(format!(
            "Reduced average cyclomatic complexity by {:.0}%",
            improved.complexity_reduction * 100.0
        ))
    } else {
        None
    }
}

/// Pure: Build coverage improvement message
fn build_coverage_message(improved: &ImprovedItems) -> Option<String> {
    if improved.coverage_improvement > 0.0 {
        Some(format!(
            "Added test coverage for {} critical functions",
            improved.coverage_improvement_count
        ))
    } else {
        None
    }
}

/// Pure: Build all improvement messages
fn build_all_improvement_messages(
    resolved: &ResolvedItems,
    improved: &ImprovedItems,
) -> Vec<String> {
    [
        build_resolved_message(resolved),
        build_complexity_message(improved),
        build_coverage_message(improved),
    ]
    .into_iter()
    .flatten()
    .collect()
}

/// Pure: Build unchanged critical items issue message
fn build_unchanged_critical_message(unchanged_critical: &UnchangedCritical) -> Option<String> {
    if unchanged_critical.count > 0 {
        Some(format!(
            "{} critical debt item{} still present",
            unchanged_critical.count,
            if unchanged_critical.count == 1 {
                ""
            } else {
                "s"
            }
        ))
    } else {
        None
    }
}

/// Pure: Build regression issue message
fn build_regression_message(new_items: &NewItems) -> Option<String> {
    if new_items.critical_count > 0 {
        Some(format!(
            "{} new critical debt items introduced",
            new_items.critical_count
        ))
    } else {
        None
    }
}

/// Pure: Build all issue messages
fn build_all_issue_messages(
    unchanged_critical: &UnchangedCritical,
    new_items: &NewItems,
) -> Vec<String> {
    [
        build_unchanged_critical_message(unchanged_critical),
        build_regression_message(new_items),
    ]
    .into_iter()
    .flatten()
    .collect()
}

// =============================================================================
// Pure Gap Builder Functions
// =============================================================================

/// Pure: Build a single critical debt gap detail
fn build_critical_debt_gap(item: &ItemInfo, idx: usize) -> (String, GapDetail) {
    let key = format!("critical_debt_remaining_{}", idx);
    let detail = GapDetail {
        description: format!("High-priority debt item still present in {}", item.function),
        location: format!("{}:{}:{}", item.file.display(), item.function, item.line),
        severity: "high".to_string(),
        suggested_fix: "Apply functional programming patterns to reduce complexity".to_string(),
        original_score: Some(item.score),
        current_score: Some(item.score),
        original_complexity: None,
        current_complexity: None,
        target_complexity: None,
    };
    (key, detail)
}

/// Pure: Build regression gap detail
fn build_regression_gap(new_items: &NewItems) -> Option<(String, GapDetail)> {
    if new_items.critical_count == 0 {
        return None;
    }

    let detail = GapDetail {
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
    };
    Some(("regression_detected".to_string(), detail))
}

/// Pure: Build all gaps from analysis results
fn build_all_gaps(
    unchanged_critical: &UnchangedCritical,
    new_items: &NewItems,
) -> HashMap<String, GapDetail> {
    let mut gaps = HashMap::new();

    // Add critical debt gaps (max 2)
    for (idx, item) in unchanged_critical.items.iter().take(2).enumerate() {
        let (key, detail) = build_critical_debt_gap(item, idx);
        gaps.insert(key, detail);
    }

    // Add regression gap if applicable
    if let Some((key, detail)) = build_regression_gap(new_items) {
        gaps.insert(key, detail);
    }

    gaps
}

// =============================================================================
// Pure Status Determination
// =============================================================================

/// Pure: Determine validation status based on score and metrics
fn determine_status(
    improvement_score: f64,
    new_items: &NewItems,
    before_summary: &AnalysisSummary,
    after_summary: &AnalysisSummary,
) -> String {
    let has_regressions = new_items.critical_count > 0;
    let all_high_priority_addressed =
        before_summary.high_priority_items > 0 && after_summary.high_priority_items == 0;
    let meets_score_threshold = improvement_score >= 75.0;

    if has_regressions {
        "failed"
    } else if all_high_priority_addressed || meets_score_threshold {
        "complete"
    } else if improvement_score >= 40.0 {
        "incomplete"
    } else {
        "failed"
    }
    .to_string()
}

// =============================================================================
// Validation Core
// =============================================================================

fn perform_validation(
    before: &DebtmapJsonInput,
    after: &DebtmapJsonInput,
) -> Result<ValidationResult> {
    // Create summaries (pure)
    let before_summary = create_summary(before);
    let after_summary = create_summary(after);

    // Identify all changes (pure)
    let resolved = identify_resolved_items(before, after);
    let improved = identify_improved_items(before, after);
    let new_items = identify_new_items(before, after);
    let unchanged_critical = identify_unchanged_critical(before, after);

    // Build messages (pure)
    let improvements = build_all_improvement_messages(&resolved, &improved);
    let remaining_issues = build_all_issue_messages(&unchanged_critical, &new_items);
    let gaps = build_all_gaps(&unchanged_critical, &new_items);

    // Calculate score (pure)
    let completion = calculate_improvement_score(
        &resolved,
        &improved,
        &new_items,
        &unchanged_critical,
        &before_summary,
        &after_summary,
    );

    // Determine status (pure)
    let status = determine_status(completion, &new_items, &before_summary, &after_summary);

    Ok(ValidationResult {
        completion_percentage: completion,
        status,
        improvements,
        remaining_issues,
        gaps,
        before_summary,
        after_summary,
    })
}

fn create_summary(analysis: &DebtmapJsonInput) -> AnalysisSummary {
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
        .filter(|item| is_critical(item.unified_score.final_score.value()))
        .count();

    let average_score = if function_items.is_empty() {
        0.0
    } else {
        function_items
            .iter()
            .map(|i| i.unified_score.final_score.value())
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

fn identify_resolved_items(before: &DebtmapJsonInput, after: &DebtmapJsonInput) -> ResolvedItems {
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
        .filter(|item| is_critical(item.unified_score.final_score.value()))
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

fn identify_improved_items(before: &DebtmapJsonInput, after: &DebtmapJsonInput) -> ImprovedItems {
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
                let score_improvement = before_item.unified_score.final_score.value()
                    - after_item.unified_score.final_score.value();
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

fn identify_new_items(before: &DebtmapJsonInput, after: &DebtmapJsonInput) -> NewItems {
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
                    && is_critical(f.unified_score.final_score.value())
                {
                    Some(ItemInfo {
                        file: f.location.file.clone(),
                        function: f.location.function.clone(),
                        line: f.location.line,
                        score: f.unified_score.final_score.value(),
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

/// Filter items to find critical items that remain unchanged between before and after
fn filter_unchanged_critical_items(
    before_items: &[crate::priority::DebtItem],
    after_map: &HashMap<(PathBuf, String), &crate::priority::UnifiedDebtItem>,
) -> Vec<ItemInfo> {
    use crate::priority::DebtItem;

    before_items
        .iter()
        .filter_map(|item| {
            if let DebtItem::Function(before_item) = item {
                if is_critical(before_item.unified_score.final_score.value()) {
                    let key = (
                        before_item.location.file.clone(),
                        before_item.location.function.clone(),
                    );
                    if let Some(after_item) = after_map.get(&key) {
                        if is_score_unchanged(
                            before_item.unified_score.final_score.value(),
                            after_item.unified_score.final_score.value(),
                        ) && is_critical(after_item.unified_score.final_score.value())
                        {
                            return Some(ItemInfo {
                                file: before_item.location.file.clone(),
                                function: before_item.location.function.clone(),
                                line: before_item.location.line,
                                score: before_item.unified_score.final_score.value(),
                            });
                        }
                    }
                }
            }
            None
        })
        .collect()
}

fn identify_unchanged_critical(
    before: &DebtmapJsonInput,
    after: &DebtmapJsonInput,
) -> UnchangedCritical {
    let after_map = build_function_map(&after.items);
    let items = filter_unchanged_critical_items(&before.items, &after_map);

    UnchangedCritical {
        count: items.len(),
        items,
    }
}

// =============================================================================
// Pure Scoring Component Functions
// =============================================================================

/// Pure: Calculate high priority resolution progress percentage
fn score_high_priority_progress(
    before_summary: &AnalysisSummary,
    after_summary: &AnalysisSummary,
    resolved: &ResolvedItems,
) -> f64 {
    if before_summary.high_priority_items == 0 {
        return 100.0;
    }

    let resolved_count = resolved.high_priority_count as f64;
    let addressed_count = before_summary
        .high_priority_items
        .saturating_sub(after_summary.high_priority_items) as f64;

    (addressed_count.max(resolved_count) / before_summary.high_priority_items as f64) * 100.0
}

/// Pure: Calculate overall score improvement percentage
fn score_overall_improvement(
    before_summary: &AnalysisSummary,
    after_summary: &AnalysisSummary,
) -> f64 {
    if before_summary.average_score <= 0.0 {
        return 0.0;
    }
    ((before_summary.average_score - after_summary.average_score) / before_summary.average_score)
        * 100.0
}

/// Pure: Calculate complexity reduction score (0-100)
fn score_complexity_reduction(improved: &ImprovedItems) -> f64 {
    improved.complexity_reduction * 100.0
}

/// Pure: Calculate regression penalty score (100 if no regressions, 0 otherwise)
fn score_regression_penalty(new_items: &NewItems) -> f64 {
    if new_items.critical_count == 0 {
        100.0
    } else {
        0.0
    }
}

/// Pure: Apply penalty for unchanged critical items
fn apply_unchanged_penalty(
    score: f64,
    unchanged_critical: &UnchangedCritical,
    has_improvements: bool,
) -> f64 {
    if unchanged_critical.count == 0 {
        return score;
    }

    let (penalty_rate, max_penalty) = if has_improvements {
        (0.05, 0.25) // Lighter penalty when there are improvements
    } else {
        (0.1, 0.5)
    };

    let penalty_factor = 1.0 - (unchanged_critical.count as f64 * penalty_rate).min(max_penalty);
    score * penalty_factor
}

/// Pure: Apply minimum threshold for significant improvements
fn apply_minimum_threshold(score: f64, has_improvements: bool, score_improvement: f64) -> f64 {
    if has_improvements && score < 40.0 && score_improvement > 5.0 {
        40.0
    } else {
        score.clamp(0.0, 100.0)
    }
}

// =============================================================================
// Main Scoring Orchestration
// =============================================================================

fn calculate_improvement_score(
    resolved: &ResolvedItems,
    improved: &ImprovedItems,
    new_items: &NewItems,
    unchanged_critical: &UnchangedCritical,
    before_summary: &AnalysisSummary,
    after_summary: &AnalysisSummary,
) -> f64 {
    // Early return for empty case
    if before_summary.total_items == 0 && after_summary.total_items == 0 {
        return 100.0;
    }

    // Compose scoring components (all pure)
    let high_priority = score_high_priority_progress(before_summary, after_summary, resolved);
    let improvement = score_overall_improvement(before_summary, after_summary);
    let complexity = score_complexity_reduction(improved);
    let regression = score_regression_penalty(new_items);

    // Calculate weighted score
    let weighted_score =
        high_priority * 0.4 + improvement.max(0.0) * 0.3 + complexity * 0.2 + regression * 0.1;

    // Apply adjustments (pure)
    let has_improvements = complexity > 0.0 || improvement > 0.0;
    let penalized = apply_unchanged_penalty(weighted_score, unchanged_critical, has_improvements);

    apply_minimum_threshold(penalized, has_improvements, improvement)
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
    use crate::priority::coverage_propagation::TransitiveCoverage;
    use crate::priority::score_types::Score0To100;
    use crate::priority::semantic_classifier::FunctionRole;
    use crate::priority::unified_scorer::{Location, UnifiedDebtItem, UnifiedScore};
    use crate::priority::{
        ActionableRecommendation, DebtItem, DebtType, FileDebtItem, ImpactMetrics,
    };
    use std::path::PathBuf;

    // Helper function to create empty output for tests
    fn create_empty_output() -> DebtmapJsonInput {
        DebtmapJsonInput {
            items: vec![],
            total_impact: ImpactMetrics {
                complexity_reduction: 0.0,
                coverage_improvement: 0.0,
                risk_reduction: 0.0,
                lines_reduction: 0,
            },
            total_debt_score: 0.0,
            debt_density: 0.0,
            total_lines_of_code: 0,
            overall_coverage: None,
        }
    }

    // Helper function to create output with items
    fn create_output_with_items(items: Vec<DebtItem>) -> DebtmapJsonInput {
        DebtmapJsonInput {
            items,
            total_impact: ImpactMetrics {
                complexity_reduction: 0.0,
                coverage_improvement: 0.0,
                risk_reduction: 0.0,
                lines_reduction: 0,
            },
            total_debt_score: 0.0,
            debt_density: 0.0,
            total_lines_of_code: 1000,
            overall_coverage: None,
        }
    }

    // Helper for tests that need specific line numbers and coverage as f64
    fn create_function_item(
        file: &str,
        function: &str,
        line: usize,
        score: f64,
        complexity: u32,
        coverage: Option<f64>,
    ) -> DebtItem {
        let transitive_coverage = coverage.map(|c| TransitiveCoverage {
            direct: c,
            transitive: c,
            propagated_from: vec![],
            uncovered_lines: vec![],
        });

        DebtItem::Function(Box::new(UnifiedDebtItem {
            location: Location {
                file: PathBuf::from(file),
                function: function.to_string(),
                line,
            },
            debt_type: DebtType::ComplexityHotspot {
                cyclomatic: complexity,
                cognitive: 0,
                adjusted_cyclomatic: None,
            },
            unified_score: UnifiedScore {
                complexity_factor: 0.0,
                coverage_factor: 0.0,
                dependency_factor: 0.0,
                role_multiplier: 1.0,
                final_score: Score0To100::new(score),
                base_score: None,
                exponential_factor: None,
                risk_boost: None,
                pre_adjustment_score: None,
                adjustment_applied: None,
                purity_factor: None,
                refactorability_factor: None,
                pattern_factor: None,
            },
            function_role: FunctionRole::Unknown,
            recommendation: ActionableRecommendation {
                primary_action: String::new(),
                rationale: String::new(),
                implementation_steps: vec![],
                related_items: vec![],
                steps: None,
                estimated_effort_hours: None,
            },
            expected_impact: ImpactMetrics {
                coverage_improvement: 0.0,
                lines_reduction: 0,
                complexity_reduction: 0.0,
                risk_reduction: 0.0,
            },
            transitive_coverage,
            upstream_dependencies: 0,
            downstream_dependencies: 0,
            upstream_callers: vec![],
            downstream_callees: vec![],
            nesting_depth: 0,
            function_length: 50,
            cyclomatic_complexity: complexity,
            cognitive_complexity: 0,
            entropy_details: None,
            entropy_adjusted_cyclomatic: None,
            entropy_adjusted_cognitive: None,
            entropy_dampening_factor: None,
            is_pure: None,
            purity_confidence: None,
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
            language_specific: None, // spec 190
            detected_pattern: None,
            contextual_risk: None, // spec 203
            file_line_count: None,
            responsibility_category: None,
        }))
    }

    // Helper for tests that use TransitiveCoverage directly
    fn create_test_function_item(
        file: &str,
        function: &str,
        score: f64,
        complexity: u32,
        coverage: Option<TransitiveCoverage>,
    ) -> DebtItem {
        DebtItem::Function(Box::new(UnifiedDebtItem {
            location: Location {
                file: PathBuf::from(file),
                function: function.to_string(),
                line: 1,
            },
            debt_type: DebtType::ComplexityHotspot {
                cyclomatic: complexity,
                cognitive: 5,
                adjusted_cyclomatic: None,
            },
            unified_score: UnifiedScore {
                complexity_factor: 0.0,
                coverage_factor: 0.0,
                dependency_factor: 0.0,
                role_multiplier: 1.0,
                final_score: Score0To100::new(score),
                base_score: None,
                exponential_factor: None,
                risk_boost: None,
                pre_adjustment_score: None,
                adjustment_applied: None,
                purity_factor: None,
                refactorability_factor: None,
                pattern_factor: None,
            },
            function_role: FunctionRole::PureLogic,
            recommendation: ActionableRecommendation {
                primary_action: "refactor".to_string(),
                rationale: "test".to_string(),
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
            transitive_coverage: coverage,
            file_context: None,
            upstream_dependencies: 0,
            downstream_dependencies: 0,
            upstream_callers: vec![],
            downstream_callees: vec![],
            nesting_depth: 1,
            function_length: 10,
            cyclomatic_complexity: complexity,
            cognitive_complexity: 5,
            entropy_details: None,
            entropy_adjusted_cyclomatic: None,
            entropy_adjusted_cognitive: None,
            entropy_dampening_factor: None,
            is_pure: None,
            purity_confidence: None,
            purity_level: None,
            god_object_indicators: None,
            tier: None,
            function_context: None,
            context_confidence: None,
            contextual_recommendation: None,
            pattern_analysis: None,
            context_multiplier: None,
            context_type: None,
            language_specific: None, // spec 190
            detected_pattern: None,
            contextual_risk: None, // spec 203
            file_line_count: None,
            responsibility_category: None,
        }))
    }

    // Helper function to create test UnifiedDebtItem (for property tests and simpler tests)
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
                adjusted_cyclomatic: None,
            },
            unified_score: UnifiedScore {
                complexity_factor: 0.0,
                coverage_factor: 0.0,
                dependency_factor: 0.0,
                role_multiplier: 1.0,
                final_score: Score0To100::new(score),
                base_score: None,
                exponential_factor: None,
                risk_boost: None,
                pre_adjustment_score: None,
                adjustment_applied: None,
                purity_factor: None,
                refactorability_factor: None,
                pattern_factor: None,
            },
            function_role: FunctionRole::PureLogic,
            recommendation: ActionableRecommendation {
                primary_action: "Test".into(),
                rationale: "Test".into(),
                implementation_steps: vec![],
                related_items: vec![],
                steps: None,
                estimated_effort_hours: None,
            },
            expected_impact: ImpactMetrics {
                risk_reduction: 0.0,
                complexity_reduction: 0.0,
                coverage_improvement: 0.0,
                lines_reduction: 0,
            },
            transitive_coverage: None,
            file_context: None,
            upstream_dependencies: 0,
            downstream_dependencies: 0,
            upstream_callers: vec![],
            downstream_callees: vec![],
            nesting_depth: 1,
            function_length: 10,
            cyclomatic_complexity: 5,
            cognitive_complexity: 8,
            entropy_details: None,
            entropy_adjusted_cyclomatic: None,
            entropy_adjusted_cognitive: None,
            entropy_dampening_factor: None,
            is_pure: Some(false),
            purity_confidence: Some(0.0),
            purity_level: None,
            god_object_indicators: None,
            tier: None,
            function_context: None,
            context_confidence: None,
            contextual_recommendation: None,
            pattern_analysis: None,
            context_multiplier: None,
            context_type: None,
            language_specific: None, // spec 190
            detected_pattern: None,
            contextual_risk: None, // spec 203
            file_line_count: None,
            responsibility_category: None,
        }
    }

    // Helper function to create DebtmapJsonInput
    fn create_test_output(items: Vec<DebtItem>) -> DebtmapJsonInput {
        DebtmapJsonInput {
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

    // Tests for perform_validation function
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
        let before = create_test_output(vec![create_function_item(
            "src/test.rs",
            "complex_fn",
            10,
            10.0,
            15,
            Some(0.0),
        )]);
        let after = create_test_output(vec![]);

        let result = perform_validation(&before, &after).unwrap();

        assert_eq!(result.status, "complete");
        assert!(result
            .improvements
            .iter()
            .any(|i| i.contains("Resolved 1 high-priority")));
        assert_eq!(result.remaining_issues.len(), 0);
        assert!(result.completion_percentage >= 75.0);
    }

    #[test]
    fn test_perform_validation_complexity_reduction() {
        let before = create_test_output(vec![create_function_item(
            "src/test.rs",
            "fn1",
            10,
            10.0,
            20,
            Some(0.5),
        )]);
        let after = create_test_output(vec![create_function_item(
            "src/test.rs",
            "fn1",
            10,
            8.0,
            10,
            Some(0.5),
        )]);

        let result = perform_validation(&before, &after).unwrap();

        assert!(result
            .improvements
            .iter()
            .any(|i| i.contains("Reduced average cyclomatic complexity")));
    }

    #[test]
    fn test_perform_validation_coverage_improvement() {
        let before = create_test_output(vec![create_function_item(
            "src/test.rs",
            "fn1",
            10,
            10.0,
            10,
            Some(0.0),
        )]);
        let after = create_test_output(vec![create_function_item(
            "src/test.rs",
            "fn1",
            10,
            8.0,
            10,
            Some(0.8),
        )]);

        let result = perform_validation(&before, &after).unwrap();

        assert!(result
            .improvements
            .iter()
            .any(|i| i.contains("Added test coverage")));
    }

    #[test]
    fn test_perform_validation_unchanged_critical() {
        let before = create_test_output(vec![create_function_item(
            "src/test.rs",
            "complex_fn",
            10,
            10.0,
            15,
            Some(0.0),
        )]);
        let after = create_test_output(vec![create_function_item(
            "src/test.rs",
            "complex_fn",
            10,
            10.0,
            15,
            Some(0.0),
        )]);

        let result = perform_validation(&before, &after).unwrap();

        assert!(result
            .remaining_issues
            .iter()
            .any(|i| i.contains("critical debt item")));
        assert!(result.gaps.contains_key("critical_debt_remaining_0"));
    }

    #[test]
    fn test_perform_validation_new_critical_regression() {
        let before = create_test_output(vec![]);
        let after = create_test_output(vec![create_function_item(
            "src/new.rs",
            "bad_fn",
            20,
            12.0,
            20,
            Some(0.0),
        )]);

        let result = perform_validation(&before, &after).unwrap();

        assert!(result
            .remaining_issues
            .iter()
            .any(|i| i.contains("new critical debt items")));
        assert!(result.gaps.contains_key("regression_detected"));
        assert_eq!(result.status, "failed");
    }

    #[test]
    fn test_perform_validation_combined_improvements() {
        let before = create_test_output(vec![
            create_function_item("src/test.rs", "fn1", 10, 10.0, 20, Some(0.0)),
            create_function_item("src/test.rs", "fn2", 30, 9.0, 15, Some(0.2)),
        ]);
        let after = create_test_output(vec![create_function_item(
            "src/test.rs",
            "fn2",
            30,
            7.0,
            10,
            Some(0.8),
        )]);

        let result = perform_validation(&before, &after).unwrap();

        assert!(result.improvements.len() >= 2);
        assert!(result.improvements.iter().any(|i| i.contains("Resolved")));
        assert!(result
            .improvements
            .iter()
            .any(|i| i.contains("complexity") || i.contains("coverage")));
        assert_eq!(result.status, "complete");
    }

    #[test]
    fn test_perform_validation_status_complete() {
        let before = create_test_output(vec![create_function_item(
            "src/test.rs",
            "fn1",
            10,
            10.0,
            15,
            Some(0.0),
        )]);
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
        let before = create_test_output(vec![create_function_item(
            "src/test.rs",
            "fn1",
            10,
            10.0,
            15,
            Some(0.0),
        )]);
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
        let before = create_test_output(vec![create_function_item(
            "src/test.rs",
            "critical_fn",
            10,
            10.0,
            15,
            Some(0.0),
        )]);
        let after = create_test_output(vec![create_function_item(
            "src/test.rs",
            "critical_fn",
            10,
            10.0,
            15,
            Some(0.0),
        )]);

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

        assert!(result
            .remaining_issues
            .iter()
            .any(|i| i.contains("3 critical debt items")));
        assert_eq!(result.gaps.len(), 2); // Only first 2 are added
        assert!(result.gaps.contains_key("critical_debt_remaining_0"));
        assert!(result.gaps.contains_key("critical_debt_remaining_1"));
    }

    // Tests for identify_improved_items function
    #[test]
    fn test_empty_before_and_after() {
        let before = create_empty_output();
        let after = create_empty_output();

        let result = identify_improved_items(&before, &after);

        assert_eq!(result.complexity_reduction, 0.0);
        assert_eq!(result.coverage_improvement, 0.0);
        assert_eq!(result.coverage_improvement_count, 0);
    }

    #[test]
    fn test_no_improvements_below_threshold() {
        let before = create_output_with_items(vec![create_test_function_item(
            "test.rs", "func1", 5.0, 10, None,
        )]);
        let after = create_output_with_items(vec![create_test_function_item(
            "test.rs", "func1", 4.6, 10, None,
        )]);

        let result = identify_improved_items(&before, &after);

        assert_eq!(result.complexity_reduction, 0.0);
        assert_eq!(result.coverage_improvement, 0.0);
        assert_eq!(result.coverage_improvement_count, 0);
    }

    #[test]
    fn test_score_improvement_above_threshold_with_complexity_reduction() {
        let before = create_output_with_items(vec![create_test_function_item(
            "test.rs", "func1", 10.0, 20, None,
        )]);
        let after = create_output_with_items(vec![create_test_function_item(
            "test.rs", "func1", 9.0, 10, None,
        )]);

        let result = identify_improved_items(&before, &after);

        assert!(result.complexity_reduction > 0.0);
        assert_eq!((result.complexity_reduction * 100.0).round() / 100.0, 0.5);
        assert_eq!(result.coverage_improvement_count, 0);
    }

    #[test]
    fn test_score_improvement_with_coverage_increase() {
        let before_coverage = Some(TransitiveCoverage {
            direct: 0.3,
            transitive: 0.2,
            propagated_from: vec![],
            uncovered_lines: vec![],
        });
        let after_coverage = Some(TransitiveCoverage {
            direct: 0.8,
            transitive: 0.7,
            propagated_from: vec![],
            uncovered_lines: vec![],
        });

        let before = create_output_with_items(vec![create_test_function_item(
            "test.rs",
            "func1",
            10.0,
            10,
            before_coverage,
        )]);
        let after = create_output_with_items(vec![create_test_function_item(
            "test.rs",
            "func1",
            9.0,
            10,
            after_coverage,
        )]);

        let result = identify_improved_items(&before, &after);

        assert_eq!(result.coverage_improvement_count, 1);
        assert_eq!(result.coverage_improvement, 1.0);
    }

    #[test]
    fn test_score_improvement_with_both_metrics_improved() {
        let before_coverage = Some(TransitiveCoverage {
            direct: 0.3,
            transitive: 0.2,
            propagated_from: vec![],
            uncovered_lines: vec![],
        });
        let after_coverage = Some(TransitiveCoverage {
            direct: 0.8,
            transitive: 0.7,
            propagated_from: vec![],
            uncovered_lines: vec![],
        });

        let before = create_output_with_items(vec![create_test_function_item(
            "test.rs",
            "func1",
            10.0,
            20,
            before_coverage,
        )]);
        let after = create_output_with_items(vec![create_test_function_item(
            "test.rs",
            "func1",
            9.0,
            10,
            after_coverage,
        )]);

        let result = identify_improved_items(&before, &after);

        assert!(result.complexity_reduction > 0.0);
        assert_eq!((result.complexity_reduction * 100.0).round() / 100.0, 0.5);
        assert_eq!(result.coverage_improvement_count, 1);
        assert_eq!(result.coverage_improvement, 1.0);
    }

    #[test]
    fn test_item_only_in_after_not_in_before() {
        let before = create_empty_output();
        let after = create_output_with_items(vec![create_test_function_item(
            "test.rs", "new_func", 5.0, 10, None,
        )]);

        let result = identify_improved_items(&before, &after);

        assert_eq!(result.complexity_reduction, 0.0);
        assert_eq!(result.coverage_improvement_count, 0);
    }

    #[test]
    fn test_item_only_in_before_not_in_after() {
        let before = create_output_with_items(vec![create_test_function_item(
            "test.rs", "old_func", 5.0, 10, None,
        )]);
        let after = create_empty_output();

        let result = identify_improved_items(&before, &after);

        assert_eq!(result.complexity_reduction, 0.0);
        assert_eq!(result.coverage_improvement_count, 0);
    }

    #[test]
    fn test_file_level_items_are_filtered() {
        use crate::priority::file_metrics::{FileDebtMetrics, FileImpact};

        let before = create_output_with_items(vec![
            DebtItem::File(Box::new(FileDebtItem {
                metrics: FileDebtMetrics {
                    path: PathBuf::from("test.rs"),
                    total_lines: 100,
                    function_count: 5,
                    class_count: 0,
                    avg_complexity: 15.0,
                    max_complexity: 20,
                    total_complexity: 75,
                    coverage_percent: 0.0,
                    uncovered_lines: 100,
                    god_object_analysis: None,
                    god_object_type: None,
                    file_type: None,
                    function_scores: vec![],
                },
                score: 10.0,
                priority_rank: 1,
                recommendation: "refactor".to_string(),
                impact: FileImpact {
                    complexity_reduction: 0.0,
                    maintainability_improvement: 0.0,
                    test_effort: 0.0,
                },
            })),
            create_test_function_item("test.rs", "func1", 10.0, 20, None),
        ]);
        let after = create_output_with_items(vec![create_test_function_item(
            "test.rs", "func1", 9.0, 10, None,
        )]);

        let result = identify_improved_items(&before, &after);

        assert!(result.complexity_reduction > 0.0);
    }

    #[test]
    fn test_multiple_improvements() {
        let before = create_output_with_items(vec![
            create_test_function_item("test.rs", "func1", 10.0, 20, None),
            create_test_function_item("test.rs", "func2", 8.0, 15, None),
            create_test_function_item("test.rs", "func3", 6.0, 10, None),
        ]);
        let after = create_output_with_items(vec![
            create_test_function_item("test.rs", "func1", 9.0, 10, None),
            create_test_function_item("test.rs", "func2", 7.0, 8, None),
            create_test_function_item("test.rs", "func3", 5.0, 5, None),
        ]);

        let result = identify_improved_items(&before, &after);

        assert!(result.complexity_reduction > 0.0);
    }

    #[test]
    fn test_missing_transitive_coverage_uses_default() {
        let before = create_output_with_items(vec![create_test_function_item(
            "test.rs", "func1", 10.0, 20, None,
        )]);
        let after = create_output_with_items(vec![create_test_function_item(
            "test.rs", "func1", 9.0, 10, None,
        )]);

        let result = identify_improved_items(&before, &after);

        assert_eq!(result.coverage_improvement_count, 0);
    }

    #[test]
    fn test_coverage_uses_max_of_direct_and_transitive() {
        let before_coverage = Some(TransitiveCoverage {
            direct: 0.3,
            transitive: 0.5,
            propagated_from: vec![],
            uncovered_lines: vec![],
        });
        let after_coverage = Some(TransitiveCoverage {
            direct: 0.8,
            transitive: 0.6,
            propagated_from: vec![],
            uncovered_lines: vec![],
        });

        let before = create_output_with_items(vec![create_test_function_item(
            "test.rs",
            "func1",
            10.0,
            10,
            before_coverage,
        )]);
        let after = create_output_with_items(vec![create_test_function_item(
            "test.rs",
            "func1",
            9.0,
            10,
            after_coverage,
        )]);

        let result = identify_improved_items(&before, &after);

        assert_eq!(result.coverage_improvement_count, 1);
    }

    #[test]
    fn test_average_complexity_reduction_calculation() {
        let before = create_output_with_items(vec![
            create_test_function_item("test.rs", "func1", 10.0, 20, None),
            create_test_function_item("test.rs", "func2", 8.0, 10, None),
        ]);
        let after = create_output_with_items(vec![
            create_test_function_item("test.rs", "func1", 9.0, 10, None),
            create_test_function_item("test.rs", "func2", 7.0, 5, None),
        ]);

        let result = identify_improved_items(&before, &after);

        assert_eq!((result.complexity_reduction * 100.0).round() / 100.0, 0.5);
    }

    // Tests for identify_unchanged_critical function
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
            file_metrics::{FileDebtMetrics, FileImpact},
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
            god_object_analysis: None,
            function_scores: vec![],
            god_object_type: None,
            file_type: None,
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

    // Tests for filter_unchanged_critical_items
    #[test]
    fn test_filter_unchanged_critical_items_empty() {
        let before_items: Vec<DebtItem> = vec![];
        let after_map = HashMap::new();

        let result = filter_unchanged_critical_items(&before_items, &after_map);

        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_filter_unchanged_critical_items_all_resolved() {
        let before_items = vec![DebtItem::Function(Box::new(create_test_debt_item(
            "src/foo.rs",
            "resolved_fn",
            10,
            9.0,
        )))];
        let after_map = HashMap::new(); // Empty map means all resolved

        let result = filter_unchanged_critical_items(&before_items, &after_map);

        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_filter_unchanged_critical_items_all_improved() {
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
            8.0,
        )))];
        let after_map = build_function_map(&after_items);

        let result = filter_unchanged_critical_items(&before_items, &after_map);

        // Score changed by 2.0 which is > 0.5, so not included
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_filter_unchanged_critical_items_mixed() {
        let before_items = vec![
            DebtItem::Function(Box::new(create_test_debt_item(
                "src/a.rs",
                "unchanged",
                10,
                9.0,
            ))),
            DebtItem::Function(Box::new(create_test_debt_item(
                "src/b.rs", "improved", 20, 10.0,
            ))),
        ];

        let after_items = vec![
            DebtItem::Function(Box::new(create_test_debt_item(
                "src/a.rs",
                "unchanged",
                10,
                9.1,
            ))),
            DebtItem::Function(Box::new(create_test_debt_item(
                "src/b.rs", "improved", 20, 8.0,
            ))),
        ];
        let after_map = build_function_map(&after_items);

        let result = filter_unchanged_critical_items(&before_items, &after_map);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].function, "unchanged");
    }

    #[test]
    fn test_filter_unchanged_critical_items_stays_critical_within_tolerance() {
        let before_items = vec![DebtItem::Function(Box::new(create_test_debt_item(
            "src/foo.rs",
            "critical_fn",
            10,
            9.0,
        )))];

        let after_items = vec![DebtItem::Function(Box::new(create_test_debt_item(
            "src/foo.rs",
            "critical_fn",
            10,
            9.3,
        )))];
        let after_map = build_function_map(&after_items);

        let result = filter_unchanged_critical_items(&before_items, &after_map);

        // Change is 0.3 which is < 0.5, and both are critical
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].function, "critical_fn");
        assert_eq!(result[0].score, 9.0);
    }

    #[test]
    fn test_identify_unchanged_critical_file_items_ignored() {
        // File items should be ignored (only functions are processed)
        use crate::priority::{
            file_metrics::{FileDebtMetrics, FileImpact},
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
            god_object_analysis: None,
            function_scores: vec![],
            god_object_type: None,
            file_type: None,
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

    // =======================================================================
    // Tests for extracted pure scoring functions
    // =======================================================================

    #[test]
    fn test_score_high_priority_progress_all_resolved() {
        let before = AnalysisSummary {
            total_items: 5,
            high_priority_items: 5,
            average_score: 10.0,
        };
        let after = AnalysisSummary {
            total_items: 0,
            high_priority_items: 0,
            average_score: 0.0,
        };
        let resolved = ResolvedItems {
            high_priority_count: 5,
            total_count: 5,
        };

        let score = score_high_priority_progress(&before, &after, &resolved);
        assert_eq!(score, 100.0);
    }

    #[test]
    fn test_score_high_priority_progress_partial() {
        let before = AnalysisSummary {
            total_items: 10,
            high_priority_items: 10,
            average_score: 10.0,
        };
        let after = AnalysisSummary {
            total_items: 5,
            high_priority_items: 5,
            average_score: 8.0,
        };
        let resolved = ResolvedItems {
            high_priority_count: 3,
            total_count: 5,
        };

        let score = score_high_priority_progress(&before, &after, &resolved);
        assert_eq!(score, 50.0); // 5 addressed out of 10
    }

    #[test]
    fn test_score_high_priority_progress_no_high_priority() {
        let before = AnalysisSummary {
            total_items: 10,
            high_priority_items: 0,
            average_score: 5.0,
        };
        let after = AnalysisSummary {
            total_items: 10,
            high_priority_items: 0,
            average_score: 5.0,
        };
        let resolved = ResolvedItems {
            high_priority_count: 0,
            total_count: 0,
        };

        let score = score_high_priority_progress(&before, &after, &resolved);
        assert_eq!(score, 100.0);
    }

    #[test]
    fn test_score_overall_improvement_significant() {
        let before = AnalysisSummary {
            total_items: 10,
            high_priority_items: 5,
            average_score: 10.0,
        };
        let after = AnalysisSummary {
            total_items: 10,
            high_priority_items: 3,
            average_score: 8.0,
        };

        let score = score_overall_improvement(&before, &after);
        assert_eq!(score, 20.0); // (10-8)/10 * 100
    }

    #[test]
    fn test_score_overall_improvement_zero_before() {
        let before = AnalysisSummary {
            total_items: 0,
            high_priority_items: 0,
            average_score: 0.0,
        };
        let after = AnalysisSummary {
            total_items: 5,
            high_priority_items: 2,
            average_score: 5.0,
        };

        let score = score_overall_improvement(&before, &after);
        assert_eq!(score, 0.0);
    }

    #[test]
    fn test_score_complexity_reduction() {
        let improved = ImprovedItems {
            complexity_reduction: 0.5,
            coverage_improvement: 0.0,
            coverage_improvement_count: 0,
        };

        let score = score_complexity_reduction(&improved);
        assert_eq!(score, 50.0);
    }

    #[test]
    fn test_score_regression_penalty_no_regressions() {
        let new_items = NewItems {
            critical_count: 0,
            items: vec![],
        };
        assert_eq!(score_regression_penalty(&new_items), 100.0);
    }

    #[test]
    fn test_score_regression_penalty_with_regressions() {
        let new_items = NewItems {
            critical_count: 3,
            items: vec![],
        };
        assert_eq!(score_regression_penalty(&new_items), 0.0);
    }

    #[test]
    fn test_apply_unchanged_penalty_no_unchanged() {
        let unchanged = UnchangedCritical {
            count: 0,
            items: vec![],
        };
        let score = apply_unchanged_penalty(80.0, &unchanged, true);
        assert_eq!(score, 80.0);
    }

    #[test]
    fn test_apply_unchanged_penalty_with_improvements() {
        let unchanged = UnchangedCritical {
            count: 2,
            items: vec![],
        };
        let score = apply_unchanged_penalty(80.0, &unchanged, true);
        // With improvements: penalty_rate=0.05, so 1.0 - (2*0.05) = 0.9
        assert_eq!(score, 72.0); // 80.0 * 0.9
    }

    #[test]
    fn test_apply_unchanged_penalty_without_improvements() {
        let unchanged = UnchangedCritical {
            count: 2,
            items: vec![],
        };
        let score = apply_unchanged_penalty(80.0, &unchanged, false);
        // Without improvements: penalty_rate=0.1, so 1.0 - (2*0.1) = 0.8
        assert_eq!(score, 64.0); // 80.0 * 0.8
    }

    #[test]
    fn test_apply_minimum_threshold_boosts_low_score() {
        let score = apply_minimum_threshold(30.0, true, 10.0);
        assert_eq!(score, 40.0);
    }

    #[test]
    fn test_apply_minimum_threshold_no_boost_when_no_improvements() {
        let score = apply_minimum_threshold(30.0, false, 10.0);
        assert_eq!(score, 30.0);
    }

    #[test]
    fn test_apply_minimum_threshold_clamps_to_100() {
        let score = apply_minimum_threshold(150.0, true, 10.0);
        assert_eq!(score, 100.0);
    }

    #[test]
    fn test_determine_status_complete_all_high_priority_addressed() {
        let new_items = NewItems {
            critical_count: 0,
            items: vec![],
        };
        let before = AnalysisSummary {
            total_items: 10,
            high_priority_items: 5,
            average_score: 10.0,
        };
        let after = AnalysisSummary {
            total_items: 5,
            high_priority_items: 0,
            average_score: 5.0,
        };

        let status = determine_status(60.0, &new_items, &before, &after);
        assert_eq!(status, "complete");
    }

    #[test]
    fn test_determine_status_failed_with_regressions() {
        let new_items = NewItems {
            critical_count: 2,
            items: vec![],
        };
        let before = AnalysisSummary {
            total_items: 10,
            high_priority_items: 5,
            average_score: 10.0,
        };
        let after = AnalysisSummary {
            total_items: 10,
            high_priority_items: 3,
            average_score: 8.0,
        };

        let status = determine_status(80.0, &new_items, &before, &after);
        assert_eq!(status, "failed");
    }

    #[test]
    fn test_determine_status_incomplete() {
        let new_items = NewItems {
            critical_count: 0,
            items: vec![],
        };
        let before = AnalysisSummary {
            total_items: 10,
            high_priority_items: 5,
            average_score: 10.0,
        };
        let after = AnalysisSummary {
            total_items: 8,
            high_priority_items: 3,
            average_score: 8.0,
        };

        let status = determine_status(55.0, &new_items, &before, &after);
        assert_eq!(status, "incomplete");
    }

    // =======================================================================
    // Tests for pure message builders
    // =======================================================================

    #[test]
    fn test_build_resolved_message_with_count() {
        let resolved = ResolvedItems {
            high_priority_count: 3,
            total_count: 5,
        };
        let msg = build_resolved_message(&resolved);
        assert_eq!(msg, Some("Resolved 3 high-priority debt items".to_string()));
    }

    #[test]
    fn test_build_resolved_message_zero() {
        let resolved = ResolvedItems {
            high_priority_count: 0,
            total_count: 0,
        };
        let msg = build_resolved_message(&resolved);
        assert_eq!(msg, None);
    }

    #[test]
    fn test_build_complexity_message_with_reduction() {
        let improved = ImprovedItems {
            complexity_reduction: 0.25,
            coverage_improvement: 0.0,
            coverage_improvement_count: 0,
        };
        let msg = build_complexity_message(&improved);
        assert_eq!(
            msg,
            Some("Reduced average cyclomatic complexity by 25%".to_string())
        );
    }

    #[test]
    fn test_build_coverage_message_with_improvement() {
        let improved = ImprovedItems {
            complexity_reduction: 0.0,
            coverage_improvement: 2.0,
            coverage_improvement_count: 2,
        };
        let msg = build_coverage_message(&improved);
        assert_eq!(
            msg,
            Some("Added test coverage for 2 critical functions".to_string())
        );
    }

    #[test]
    fn test_build_all_improvement_messages() {
        let resolved = ResolvedItems {
            high_priority_count: 2,
            total_count: 3,
        };
        let improved = ImprovedItems {
            complexity_reduction: 0.3,
            coverage_improvement: 1.0,
            coverage_improvement_count: 1,
        };

        let messages = build_all_improvement_messages(&resolved, &improved);
        assert_eq!(messages.len(), 3);
    }

    #[test]
    fn test_build_unchanged_critical_message_singular() {
        let unchanged = UnchangedCritical {
            count: 1,
            items: vec![],
        };
        let msg = build_unchanged_critical_message(&unchanged);
        assert_eq!(msg, Some("1 critical debt item still present".to_string()));
    }

    #[test]
    fn test_build_unchanged_critical_message_plural() {
        let unchanged = UnchangedCritical {
            count: 3,
            items: vec![],
        };
        let msg = build_unchanged_critical_message(&unchanged);
        assert_eq!(msg, Some("3 critical debt items still present".to_string()));
    }

    #[test]
    fn test_build_regression_message_with_regressions() {
        let new_items = NewItems {
            critical_count: 2,
            items: vec![],
        };
        let msg = build_regression_message(&new_items);
        assert_eq!(
            msg,
            Some("2 new critical debt items introduced".to_string())
        );
    }

    // =======================================================================
    // Tests for pure gap builders
    // =======================================================================

    #[test]
    fn test_build_critical_debt_gap() {
        let item = ItemInfo {
            file: PathBuf::from("src/test.rs"),
            function: "complex_fn".to_string(),
            line: 42,
            score: 9.5,
        };

        let (key, detail) = build_critical_debt_gap(&item, 0);
        assert_eq!(key, "critical_debt_remaining_0");
        assert_eq!(detail.severity, "high");
        assert!(detail.location.contains("src/test.rs"));
        assert!(detail.location.contains("complex_fn"));
        assert_eq!(detail.original_score, Some(9.5));
    }

    #[test]
    fn test_build_regression_gap_none_when_no_regressions() {
        let new_items = NewItems {
            critical_count: 0,
            items: vec![],
        };
        let gap = build_regression_gap(&new_items);
        assert!(gap.is_none());
    }

    #[test]
    fn test_build_regression_gap_with_regressions() {
        let new_items = NewItems {
            critical_count: 1,
            items: vec![ItemInfo {
                file: PathBuf::from("src/new.rs"),
                function: "bad_fn".to_string(),
                line: 10,
                score: 12.0,
            }],
        };
        let gap = build_regression_gap(&new_items);
        assert!(gap.is_some());
        let (key, detail) = gap.unwrap();
        assert_eq!(key, "regression_detected");
        assert_eq!(detail.current_score, Some(12.0));
    }

    #[test]
    fn test_build_all_gaps_combined() {
        let unchanged = UnchangedCritical {
            count: 2,
            items: vec![
                ItemInfo {
                    file: PathBuf::from("src/a.rs"),
                    function: "fn1".to_string(),
                    line: 10,
                    score: 9.0,
                },
                ItemInfo {
                    file: PathBuf::from("src/b.rs"),
                    function: "fn2".to_string(),
                    line: 20,
                    score: 10.0,
                },
            ],
        };
        let new_items = NewItems {
            critical_count: 1,
            items: vec![ItemInfo {
                file: PathBuf::from("src/c.rs"),
                function: "fn3".to_string(),
                line: 30,
                score: 11.0,
            }],
        };

        let gaps = build_all_gaps(&unchanged, &new_items);
        assert_eq!(gaps.len(), 3); // 2 critical + 1 regression
        assert!(gaps.contains_key("critical_debt_remaining_0"));
        assert!(gaps.contains_key("critical_debt_remaining_1"));
        assert!(gaps.contains_key("regression_detected"));
    }

    // Property-based tests
    #[cfg(test)]
    mod property_tests {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            #[test]
            fn prop_is_critical_threshold_consistent(score in 0.0f64..20.0f64) {
                let result = is_critical(score);
                if score >= CRITICAL_SCORE_THRESHOLD {
                    prop_assert!(result);
                } else {
                    prop_assert!(!result);
                }
            }

            #[test]
            fn prop_is_score_unchanged_symmetric(before in 0.0f64..20.0f64, after in 0.0f64..20.0f64) {
                let result1 = is_score_unchanged(before, after);
                let result2 = is_score_unchanged(after, before);
                prop_assert_eq!(result1, result2, "is_score_unchanged should be symmetric");
            }

            #[test]
            fn prop_is_score_unchanged_reflexive(score in 0.0f64..20.0f64) {
                prop_assert!(is_score_unchanged(score, score), "score should be unchanged from itself");
            }

            #[test]
            fn prop_filter_returns_subset(count in 0usize..20) {
                // Generate test items
                let before_items: Vec<DebtItem> = (0..count)
                    .map(|i| {
                        DebtItem::Function(Box::new(create_test_debt_item(
                            "src/test.rs",
                            &format!("fn_{}", i),
                            i * 10,
                            8.5 + (i as f64 % 5.0),
                        )))
                    })
                    .collect();

                let after_map = build_function_map(&before_items);
                let result = filter_unchanged_critical_items(&before_items, &after_map);

                // Result should never be larger than input
                prop_assert!(result.len() <= before_items.len());
            }

            #[test]
            fn prop_all_returned_items_are_critical(count in 1usize..20) {
                // Generate test items with varying scores
                let before_items: Vec<DebtItem> = (0..count)
                    .map(|i| {
                        DebtItem::Function(Box::new(create_test_debt_item(
                            "src/test.rs",
                            &format!("fn_{}", i),
                            i * 10,
                            6.0 + (i as f64 % 8.0), // Scores from 6.0 to 14.0
                        )))
                    })
                    .collect();

                let after_map = build_function_map(&before_items);
                let result = filter_unchanged_critical_items(&before_items, &after_map);

                // All returned items should have critical scores
                for item in &result {
                    prop_assert!(is_critical(item.score), "Item score {} should be critical", item.score);
                }
            }

            #[test]
            fn prop_count_equals_length(count in 0usize..50) {
                let before_items: Vec<DebtItem> = (0..count)
                    .map(|i| {
                        DebtItem::Function(Box::new(create_test_debt_item(
                            "src/test.rs",
                            &format!("fn_{}", i),
                            i * 10,
                            8.5 + (i as f64 % 5.0),
                        )))
                    })
                    .collect();

                let after_output = create_test_output(before_items.clone());
                let before_output = create_test_output(before_items);

                let result = identify_unchanged_critical(&before_output, &after_output);

                // Invariant: count should always equal items length
                prop_assert_eq!(result.count, result.items.len());
            }
        }
    }
}
