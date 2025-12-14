//! Pure threshold validation logic.
//!
//! This module contains pure functions for validating metrics against
//! thresholds. Following the Stillwater philosophy, these functions
//! have no side effects - they take inputs and return outputs.
//!
//! # Design
//!
//! Validation focuses on debt density as the primary metric:
//! - Debt density is scale-independent and comparable across projects
//! - Other metrics (complexity, risk, coverage) are informational only
//! - This simplifies CI integration while still providing visibility

use super::types::{CheckResult, ThresholdCheckResult, ValidationDetails};
use crate::config::ValidationThresholds;
use crate::core::AnalysisResults;
use crate::risk;

/// Pure: Check if debt density meets threshold
///
/// This is the primary validation metric - debt density is scale-independent
/// and directly measures the quality of the codebase.
fn check_debt_density(actual: f64, threshold: f64) -> CheckResult {
    CheckResult {
        name: "debt_density",
        passed: actual <= threshold,
        actual,
        threshold,
        is_deprecated: false,
    }
}

/// Pure: Count high-risk functions above threshold
pub fn count_high_risk_functions(insights: &risk::RiskInsight, risk_threshold: f64) -> usize {
    insights
        .top_risks
        .iter()
        .filter(|f| f.risk_score > risk_threshold)
        .count()
}

/// Pure: Validate metrics with full risk insights available.
///
/// This performs comprehensive validation including coverage and risk metrics.
/// Returns (pass_status, validation_details).
#[allow(deprecated)]
pub fn validate_with_risk(
    results: &AnalysisResults,
    insights: &risk::RiskInsight,
    coverage_percentage: f64,
    total_debt_score: u32,
    debt_density: f64,
    thresholds: &ValidationThresholds,
    max_debt_density_override: Option<f64>,
) -> (bool, ValidationDetails) {
    let risk_threshold = 7.0;
    let high_risk_count = count_high_risk_functions(insights, risk_threshold);
    let max_debt_density = max_debt_density_override.unwrap_or(thresholds.max_debt_density);

    // Only debt density is a blocking check - other metrics are informational
    let checks = vec![check_debt_density(debt_density, max_debt_density)];

    let result = ThresholdCheckResult::from_checks(checks);

    let details = ValidationDetails {
        average_complexity: results.complexity.summary.average_complexity,
        max_average_complexity: thresholds.max_average_complexity,
        high_complexity_count: results.complexity.summary.high_complexity_count,
        max_high_complexity_count: thresholds.max_high_complexity_count.unwrap_or(0),
        debt_items: results.technical_debt.items.len(),
        max_debt_items: thresholds.max_debt_items.unwrap_or(0),
        total_debt_score,
        max_total_debt_score: thresholds.max_total_debt_score,
        debt_density,
        max_debt_density,
        codebase_risk_score: insights.codebase_risk_score,
        max_codebase_risk_score: thresholds.max_codebase_risk_score,
        high_risk_functions: high_risk_count,
        max_high_risk_functions: thresholds.max_high_risk_functions.unwrap_or(0),
        coverage_percentage,
        min_coverage_percentage: thresholds.min_coverage_percentage,
    };

    (result.passed, details)
}

/// Pure: Validate metrics without risk insights (basic validation).
///
/// This performs validation without coverage or risk analysis.
/// Returns (pass_status, validation_details).
#[allow(deprecated)]
pub fn validate_basic(
    results: &AnalysisResults,
    total_debt_score: u32,
    debt_density: f64,
    thresholds: &ValidationThresholds,
    max_debt_density_override: Option<f64>,
) -> (bool, ValidationDetails) {
    let max_debt_density = max_debt_density_override.unwrap_or(thresholds.max_debt_density);

    // Only debt density is a blocking check - other metrics are informational
    let checks = vec![check_debt_density(debt_density, max_debt_density)];

    let result = ThresholdCheckResult::from_checks(checks);

    let details = ValidationDetails {
        average_complexity: results.complexity.summary.average_complexity,
        max_average_complexity: thresholds.max_average_complexity,
        high_complexity_count: results.complexity.summary.high_complexity_count,
        max_high_complexity_count: thresholds.max_high_complexity_count.unwrap_or(0),
        debt_items: results.technical_debt.items.len(),
        max_debt_items: thresholds.max_debt_items.unwrap_or(0),
        total_debt_score,
        max_total_debt_score: thresholds.max_total_debt_score,
        debt_density,
        max_debt_density,
        codebase_risk_score: 0.0,
        max_codebase_risk_score: thresholds.max_codebase_risk_score,
        high_risk_functions: 0,
        max_high_risk_functions: thresholds.max_high_risk_functions.unwrap_or(0),
        coverage_percentage: 0.0,
        min_coverage_percentage: thresholds.min_coverage_percentage,
    };

    (result.passed, details)
}

/// Pure: Identify deprecated thresholds that are set.
///
/// Returns a list of deprecated threshold names that the user has configured.
#[allow(deprecated)]
pub fn find_deprecated_thresholds(thresholds: &ValidationThresholds) -> Vec<&'static str> {
    let mut deprecated = Vec::new();

    if thresholds.max_high_complexity_count.is_some() {
        deprecated.push("max_high_complexity_count");
    }
    if thresholds.max_debt_items.is_some() {
        deprecated.push("max_debt_items");
    }
    if thresholds.max_high_risk_functions.is_some() {
        deprecated.push("max_high_risk_functions");
    }

    deprecated
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_debt_density_pass() {
        let result = check_debt_density(0.15, 0.20);
        assert!(result.passed);
        assert_eq!(result.name, "debt_density");
        assert!(!result.is_deprecated);
    }

    #[test]
    fn test_check_debt_density_fail() {
        let result = check_debt_density(0.25, 0.20);
        assert!(!result.passed);
    }

    #[test]
    fn test_check_debt_density_at_threshold() {
        let result = check_debt_density(0.20, 0.20);
        assert!(result.passed); // Equal is passing
    }

    #[test]
    fn test_threshold_check_result_passes_with_debt_density_only() {
        let checks = vec![check_debt_density(15.0, 50.0)];
        let result = ThresholdCheckResult::from_checks(checks);
        assert!(result.passed);
    }

    #[test]
    fn test_threshold_check_result_fails_with_high_debt_density() {
        let checks = vec![check_debt_density(60.0, 50.0)];
        let result = ThresholdCheckResult::from_checks(checks);
        assert!(!result.passed);
    }

    #[test]
    #[allow(deprecated)]
    fn test_find_deprecated_thresholds_none() {
        let thresholds = ValidationThresholds {
            max_average_complexity: 10.0,
            max_high_complexity_count: None,
            max_debt_items: None,
            max_total_debt_score: 1000,
            max_debt_density: 0.2,
            max_codebase_risk_score: 50.0,
            max_high_risk_functions: None,
            min_coverage_percentage: 0.0,
        };
        let deprecated = find_deprecated_thresholds(&thresholds);
        assert!(deprecated.is_empty());
    }

    #[test]
    #[allow(deprecated)]
    fn test_find_deprecated_thresholds_some() {
        let thresholds = ValidationThresholds {
            max_average_complexity: 10.0,
            max_high_complexity_count: Some(5),
            max_debt_items: Some(10),
            max_total_debt_score: 1000,
            max_debt_density: 0.2,
            max_codebase_risk_score: 50.0,
            max_high_risk_functions: None,
            min_coverage_percentage: 0.0,
        };
        let deprecated = find_deprecated_thresholds(&thresholds);
        assert_eq!(deprecated.len(), 2);
        assert!(deprecated.contains(&"max_high_complexity_count"));
        assert!(deprecated.contains(&"max_debt_items"));
    }
}
