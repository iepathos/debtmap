//! Pure processors for validation components.
//!
//! This module contains pure functions that process comparison data
//! and produce validation components. Functions return data rather
//! than mutating state.

use std::collections::HashMap;

use crate::comparison::types::{ComparisonResult, RegressionItem, TargetComparison, TargetStatus};

use super::types::GapDetail;

/// Result of processing target improvements.
pub struct TargetProcessResult {
    pub component_score: f64,
    pub improvements: Vec<String>,
    pub remaining_issues: Vec<String>,
    pub gaps: HashMap<String, GapDetail>,
}

/// Result of processing regressions.
pub struct RegressionProcessResult {
    pub component_score: f64,
    pub remaining_issues: Vec<String>,
    pub gaps: HashMap<String, GapDetail>,
}

/// Result of processing project health.
pub struct ProjectHealthResult {
    pub component_score: f64,
    pub improvements: Vec<String>,
}

/// Pure: Process target improvements from comparison.
///
/// Returns the target component score and accumulated findings.
pub fn process_target_improvements(comparison: &ComparisonResult) -> TargetProcessResult {
    let mut improvements = Vec::new();
    let mut remaining_issues = Vec::new();
    let mut gaps = HashMap::new();

    let component_score = comparison
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
                let (issue, gap_key, gap) = build_target_gap(target);
                remaining_issues.push(issue);
                gaps.insert(gap_key, gap);
            }

            improvement_pct.min(100.0)
        })
        .unwrap_or(0.0);

    TargetProcessResult {
        component_score,
        improvements,
        remaining_issues,
        gaps,
    }
}

/// Pure: Build gap detail for unchanged target.
fn build_target_gap(target: &TargetComparison) -> (String, String, GapDetail) {
    let issue = "Target debt item not improved".to_string();
    let gap_key = "insufficient_target_improvement".to_string();
    let gap = GapDetail {
        description: "Target function still above complexity threshold".to_string(),
        location: target.location.clone(),
        severity: "high".to_string(),
        suggested_fix: "Further extract helper functions or simplify logic".to_string(),
        score_before: Some(target.before.score),
        score_after: target.after.as_ref().map(|a| a.score),
        current_score: None,
    };
    (issue, gap_key, gap)
}

/// Pure: Process regressions from comparison.
///
/// Returns the regression component score and accumulated findings.
pub fn process_regressions(comparison: &ComparisonResult) -> RegressionProcessResult {
    let mut remaining_issues = Vec::new();
    let mut gaps = HashMap::new();

    let regression_count = comparison.regressions.len();

    if regression_count > 0 {
        remaining_issues.push(format!(
            "{} new critical debt item{} introduced",
            regression_count,
            if regression_count == 1 { "" } else { "s" }
        ));

        gaps.extend(build_regression_gaps(&comparison.regressions));
    }

    let regression_penalty = (regression_count * 20).min(100) as f64;
    let component_score = (100.0 - regression_penalty).max(0.0);

    RegressionProcessResult {
        component_score,
        remaining_issues,
        gaps,
    }
}

/// Pure: Build gap details for regressions.
fn build_regression_gaps(regressions: &[RegressionItem]) -> HashMap<String, GapDetail> {
    regressions
        .iter()
        .take(3)
        .enumerate()
        .map(|(idx, regression)| {
            let key = format!("regression_{}", idx);
            let gap = GapDetail {
                description: regression.description.clone(),
                location: regression.location.clone(),
                severity: "high".to_string(),
                suggested_fix: "Simplify using pure functional patterns".to_string(),
                score_before: None,
                score_after: None,
                current_score: Some(regression.score),
            };
            (key, gap)
        })
        .collect()
}

/// Pure: Process project health from comparison.
///
/// Returns the health component score and improvements found.
pub fn process_project_health(comparison: &ComparisonResult) -> ProjectHealthResult {
    let mut improvements = Vec::new();

    let debt_improvement_pct = comparison.project_health.changes.debt_score_change_pct;

    if debt_improvement_pct < 0.0 {
        improvements.push(format!(
            "Overall project debt reduced by {:.1}%",
            debt_improvement_pct.abs()
        ));
    }

    let component_score = (debt_improvement_pct.abs() * 10.0).min(100.0);

    ProjectHealthResult {
        component_score,
        improvements,
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_regression_gaps_limits_to_three() {
        let regressions: Vec<RegressionItem> = (0..5)
            .map(|i| RegressionItem {
                location: format!("file_{}.rs", i),
                description: format!("Regression {}", i),
                score: 50.0 + i as f64,
                debt_type: "complexity".to_string(),
            })
            .collect();

        let gaps = build_regression_gaps(&regressions);

        assert_eq!(gaps.len(), 3);
        assert!(gaps.contains_key("regression_0"));
        assert!(gaps.contains_key("regression_1"));
        assert!(gaps.contains_key("regression_2"));
        assert!(!gaps.contains_key("regression_3"));
    }

    #[test]
    fn test_build_target_gap() {
        use crate::comparison::types::{ImprovementMetrics, TargetMetrics};

        let target = TargetComparison {
            location: "src/lib.rs:foo".to_string(),
            match_strategy: None,
            match_confidence: None,
            matched_items_count: None,
            before: TargetMetrics {
                score: 80.0,
                cyclomatic_complexity: 15,
                cognitive_complexity: 20,
                coverage: 50.0,
                function_length: 100,
                nesting_depth: 3,
            },
            after: Some(TargetMetrics {
                score: 75.0,
                cyclomatic_complexity: 12,
                cognitive_complexity: 15,
                coverage: 55.0,
                function_length: 80,
                nesting_depth: 2,
            }),
            status: TargetStatus::Unchanged,
            improvements: ImprovementMetrics {
                score_reduction_pct: 6.25,
                complexity_reduction_pct: 20.0,
                coverage_improvement_pct: 10.0,
            },
        };

        let (issue, key, gap) = build_target_gap(&target);

        assert_eq!(issue, "Target debt item not improved");
        assert_eq!(key, "insufficient_target_improvement");
        assert_eq!(gap.severity, "high");
        assert_eq!(gap.score_before, Some(80.0));
        assert_eq!(gap.score_after, Some(75.0));
    }
}
