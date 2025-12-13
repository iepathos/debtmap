//! Pure threshold validation logic.
//!
//! This module contains pure functions for validating metrics against
//! thresholds. Following the Stillwater philosophy, these functions
//! have no side effects - they take inputs and return outputs.
//!
//! # Design
//!
//! The validation is split into:
//! - Primary quality metrics (scale-independent, always checked)
//! - Safety net metrics (high ceiling to catch extreme cases)
//! - Deprecated metrics (warn but allow, for backwards compatibility)

use super::types::{CheckResult, ThresholdCheckResult, ValidationDetails};
use crate::config::ValidationThresholds;
use crate::core::AnalysisResults;
use crate::risk;

/// Pure: Check if complexity meets threshold
fn check_average_complexity(actual: f64, threshold: f64) -> CheckResult {
    CheckResult {
        name: "average_complexity",
        passed: actual <= threshold,
        actual,
        threshold,
        is_deprecated: false,
    }
}

/// Pure: Check if debt density meets threshold
fn check_debt_density(actual: f64, threshold: f64) -> CheckResult {
    CheckResult {
        name: "debt_density",
        passed: actual <= threshold,
        actual,
        threshold,
        is_deprecated: false,
    }
}

/// Pure: Check if codebase risk score meets threshold
fn check_codebase_risk(actual: f64, threshold: f64) -> CheckResult {
    CheckResult {
        name: "codebase_risk_score",
        passed: actual <= threshold,
        actual,
        threshold,
        is_deprecated: false,
    }
}

/// Pure: Check if total debt score meets threshold (safety net)
fn check_debt_score(actual: u32, threshold: u32) -> CheckResult {
    CheckResult {
        name: "total_debt_score",
        passed: actual <= threshold,
        actual: actual as f64,
        threshold: threshold as f64,
        is_deprecated: false,
    }
}

/// Pure: Check if coverage meets minimum threshold
fn check_coverage(actual: f64, threshold: f64) -> CheckResult {
    CheckResult {
        name: "coverage_percentage",
        passed: actual >= threshold,
        actual,
        threshold,
        is_deprecated: false,
    }
}

/// Pure: Check deprecated high complexity count threshold
#[allow(deprecated)]
fn check_high_complexity_count(actual: usize, threshold: Option<usize>) -> Option<CheckResult> {
    threshold.map(|t| CheckResult {
        name: "high_complexity_count",
        passed: actual <= t,
        actual: actual as f64,
        threshold: t as f64,
        is_deprecated: true,
    })
}

/// Pure: Check deprecated debt items threshold
#[allow(deprecated)]
fn check_debt_items(actual: usize, threshold: Option<usize>) -> Option<CheckResult> {
    threshold.map(|t| CheckResult {
        name: "debt_items",
        passed: actual <= t,
        actual: actual as f64,
        threshold: t as f64,
        is_deprecated: true,
    })
}

/// Pure: Check deprecated high risk functions threshold
#[allow(deprecated)]
fn check_high_risk_functions(actual: usize, threshold: Option<usize>) -> Option<CheckResult> {
    threshold.map(|t| CheckResult {
        name: "high_risk_functions",
        passed: actual <= t,
        actual: actual as f64,
        threshold: t as f64,
        is_deprecated: true,
    })
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

    // Collect all check results
    let mut checks = vec![
        check_average_complexity(
            results.complexity.summary.average_complexity,
            thresholds.max_average_complexity,
        ),
        check_debt_density(debt_density, max_debt_density),
        check_codebase_risk(
            insights.codebase_risk_score,
            thresholds.max_codebase_risk_score,
        ),
        check_debt_score(total_debt_score, thresholds.max_total_debt_score),
        check_coverage(coverage_percentage, thresholds.min_coverage_percentage),
    ];

    // Add deprecated checks if thresholds are set
    if let Some(check) = check_high_complexity_count(
        results.complexity.summary.high_complexity_count,
        thresholds.max_high_complexity_count,
    ) {
        checks.push(check);
    }

    if let Some(check) = check_debt_items(
        results.technical_debt.items.len(),
        thresholds.max_debt_items,
    ) {
        checks.push(check);
    }

    if let Some(check) =
        check_high_risk_functions(high_risk_count, thresholds.max_high_risk_functions)
    {
        checks.push(check);
    }

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

    // Collect all check results
    let mut checks = vec![
        check_average_complexity(
            results.complexity.summary.average_complexity,
            thresholds.max_average_complexity,
        ),
        check_debt_density(debt_density, max_debt_density),
        check_debt_score(total_debt_score, thresholds.max_total_debt_score),
    ];

    // Add deprecated checks if thresholds are set
    if let Some(check) = check_high_complexity_count(
        results.complexity.summary.high_complexity_count,
        thresholds.max_high_complexity_count,
    ) {
        checks.push(check);
    }

    if let Some(check) = check_debt_items(
        results.technical_debt.items.len(),
        thresholds.max_debt_items,
    ) {
        checks.push(check);
    }

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
    fn test_check_average_complexity_pass() {
        let result = check_average_complexity(5.0, 10.0);
        assert!(result.passed);
        assert_eq!(result.name, "average_complexity");
        assert!(!result.is_deprecated);
    }

    #[test]
    fn test_check_average_complexity_fail() {
        let result = check_average_complexity(15.0, 10.0);
        assert!(!result.passed);
    }

    #[test]
    fn test_check_debt_density_pass() {
        let result = check_debt_density(0.15, 0.20);
        assert!(result.passed);
    }

    #[test]
    fn test_check_debt_density_fail() {
        let result = check_debt_density(0.25, 0.20);
        assert!(!result.passed);
    }

    #[test]
    fn test_check_coverage_pass() {
        let result = check_coverage(75.0, 60.0);
        assert!(result.passed);
    }

    #[test]
    fn test_check_coverage_fail() {
        let result = check_coverage(50.0, 60.0);
        assert!(!result.passed);
    }

    #[test]
    fn test_check_deprecated_returns_none_when_unset() {
        let result = check_high_complexity_count(10, None);
        assert!(result.is_none());
    }

    #[test]
    fn test_check_deprecated_returns_some_when_set() {
        let result = check_high_complexity_count(10, Some(20));
        assert!(result.is_some());
        let check = result.unwrap();
        assert!(check.passed);
        assert!(check.is_deprecated);
    }

    #[test]
    fn test_threshold_check_result_all_pass() {
        let checks = vec![
            check_average_complexity(5.0, 10.0),
            check_debt_density(0.1, 0.2),
        ];
        let result = ThresholdCheckResult::from_checks(checks);
        assert!(result.passed);
    }

    #[test]
    fn test_threshold_check_result_one_fail() {
        let checks = vec![
            check_average_complexity(15.0, 10.0), // fails
            check_debt_density(0.1, 0.2),         // passes
        ];
        let result = ThresholdCheckResult::from_checks(checks);
        assert!(!result.passed);
    }

    #[test]
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
