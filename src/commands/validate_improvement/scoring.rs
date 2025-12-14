//! Pure scoring logic for validation.
//!
//! This module contains pure functions for calculating validation
//! scores and determining status. No side effects.

use super::types::TrendAnalysis;
use crate::comparison::types::{ComparisonResult, TargetComparison};

use super::types::{ProjectSummary, TargetSummary, ValidationResult};

/// Pure: Calculate composite validation score from components.
///
/// Weights:
/// - Target improvement: 50%
/// - Project health: 30%
/// - No regressions: 20%
pub fn calculate_composite_score(
    target_component: f64,
    project_health_component: f64,
    no_regression_component: f64,
) -> f64 {
    (target_component * 0.5 + project_health_component * 0.3 + no_regression_component * 0.2)
        .clamp(0.0, 100.0)
}

/// Pure: Determine validation status from score.
pub fn determine_status(improvement_score: f64) -> String {
    if improvement_score >= 75.0 {
        "complete"
    } else {
        "incomplete"
    }
    .to_string()
}

/// Pure: Build target summary from comparison data.
pub fn build_target_summary(target_item: &Option<TargetComparison>) -> Option<TargetSummary> {
    target_item.as_ref().map(|target| TargetSummary {
        location: target.location.clone(),
        score_before: target.before.score,
        score_after: target.after.as_ref().map(|a| a.score),
        improvement_percent: target.improvements.score_reduction_pct,
        status: format!("{:?}", target.status).to_lowercase(),
    })
}

/// Pure: Build project summary from comparison data.
pub fn build_project_summary(comparison: &ComparisonResult) -> ProjectSummary {
    ProjectSummary {
        total_debt_before: comparison.project_health.before.total_debt_score,
        total_debt_after: comparison.project_health.after.total_debt_score,
        improvement_percent: comparison.project_health.changes.debt_score_change_pct,
        items_resolved: comparison.summary.resolved_count,
        items_new: comparison.summary.new_critical_count,
    }
}

/// Pure: Calculate trend analysis based on previous validation.
pub fn calculate_trend_analysis(previous: &ValidationResult, current_score: f64) -> TrendAnalysis {
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

// =============================================================================
// Tests
// =============================================================================

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

    #[test]
    fn test_calculate_trend_analysis_regression() {
        let previous = ValidationResult {
            completion_percentage: 80.0,
            status: "complete".to_string(),
            improvements: vec![],
            remaining_issues: vec![],
            gaps: std::collections::HashMap::new(),
            target_summary: None,
            project_summary: ProjectSummary {
                total_debt_before: 100.0,
                total_debt_after: 50.0,
                improvement_percent: 50.0,
                items_resolved: 5,
                items_new: 0,
            },
            trend_analysis: None,
            attempt_number: Some(1),
        };

        let trend = calculate_trend_analysis(&previous, 70.0);
        assert_eq!(trend.direction, "regression");
        assert!(trend.recommendation.contains("CRITICAL"));
    }

    #[test]
    fn test_calculate_trend_analysis_progress() {
        let previous = ValidationResult {
            completion_percentage: 50.0,
            status: "incomplete".to_string(),
            improvements: vec![],
            remaining_issues: vec![],
            gaps: std::collections::HashMap::new(),
            target_summary: None,
            project_summary: ProjectSummary {
                total_debt_before: 100.0,
                total_debt_after: 80.0,
                improvement_percent: 20.0,
                items_resolved: 2,
                items_new: 0,
            },
            trend_analysis: None,
            attempt_number: Some(1),
        };

        let trend = calculate_trend_analysis(&previous, 60.0);
        assert_eq!(trend.direction, "progress");
        assert!(trend.recommendation.contains("Continue"));
    }
}
