//! Pure functions for aggregating risk factors into a composite score.
//!
//! This module contains stateless functions that combine multiple
//! risk factors using weighted averaging and role-based multipliers.

use crate::priority::semantic_classifier::FunctionRole;
use crate::risk::evidence::RiskFactor;

/// Calculates weighted average of risk factors.
///
/// Factors with zero weight are excluded from the calculation.
/// Returns 0.0 if all factors have zero weight or the list is empty.
pub fn calculate_weighted_average(factors: &[RiskFactor]) -> f64 {
    let (total_score, total_weight) = factors
        .iter()
        .filter(|f| f.weight > 0.0)
        .fold((0.0, 0.0), |(score, weight), factor| {
            (score + factor.score * factor.weight, weight + factor.weight)
        });

    if total_weight == 0.0 {
        0.0
    } else {
        total_score / total_weight
    }
}

/// Returns the risk multiplier based on function role.
///
/// Different roles have different risk implications:
/// - PureLogic (1.2): Business logic is more critical
/// - EntryPoint (1.1): Entry points are important
/// - Orchestrator (0.9): Orchestration is less risky
/// - IOWrapper (0.7): I/O wrappers are expected to be simple
/// - PatternMatch (0.5): Pattern matching is very low risk
/// - Debug (0.4): Debug functions have very low test priority
/// - Unknown (1.0): Default multiplier
pub fn get_role_multiplier(role: &FunctionRole) -> f64 {
    match role {
        FunctionRole::PureLogic => 1.2,
        FunctionRole::EntryPoint => 1.1,
        FunctionRole::Orchestrator => 0.9,
        FunctionRole::IOWrapper => 0.7,
        FunctionRole::PatternMatch => 0.5,
        FunctionRole::Debug => 0.4,
        FunctionRole::Unknown => 1.0,
    }
}

/// Aggregates risk factors into a single score adjusted by role.
pub fn aggregate_risk_factors(factors: &[RiskFactor], role: &FunctionRole) -> f64 {
    let base_score = calculate_weighted_average(factors);
    let role_multiplier = get_role_multiplier(role);
    base_score * role_multiplier
}

/// Calculates weighted confidence from risk factors.
///
/// Returns 0.5 as default confidence if no factors have positive weight.
pub fn calculate_confidence(factors: &[RiskFactor]) -> f64 {
    if factors.is_empty() {
        return 0.0;
    }

    let (total_confidence, total_weight) =
        factors
            .iter()
            .filter(|f| f.weight > 0.0)
            .fold((0.0, 0.0), |(conf, weight), factor| {
                (
                    conf + factor.confidence * factor.weight,
                    weight + factor.weight,
                )
            });

    if total_weight == 0.0 {
        0.5
    } else {
        total_confidence / total_weight
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::risk::evidence::{
        ComparisonResult, ComplexityEvidence, ComplexityThreshold, CouplingEvidence,
        CoverageEvidence, RiskEvidence, RiskSeverity, RiskType, TestQuality,
    };

    #[test]
    fn test_calculate_weighted_average_normal_factors() {
        let factors = create_test_factors_normal();
        // (8.0 * 0.5 + 6.0 * 0.3 + 4.0 * 0.2) / (0.5 + 0.3 + 0.2)
        // = (4.0 + 1.8 + 0.8) / 1.0 = 6.6
        let result = calculate_weighted_average(&factors);
        assert!((result - 6.6).abs() < 0.001);
    }

    #[test]
    fn test_calculate_weighted_average_empty_factors() {
        let factors = vec![];
        let result = calculate_weighted_average(&factors);
        assert_eq!(result, 0.0);
    }

    #[test]
    fn test_calculate_weighted_average_all_zero_weights() {
        let factors = create_test_factors_zero_weights();
        let result = calculate_weighted_average(&factors);
        assert_eq!(result, 0.0);
    }

    #[test]
    fn test_calculate_weighted_average_mixed_zero_weights() {
        let factors = create_test_factors_mixed_weights();
        // Only non-zero weights: (5.0 * 0.6 + 10.0 * 0.4) / (0.6 + 0.4)
        // = (3.0 + 4.0) / 1.0 = 7.0
        let result = calculate_weighted_average(&factors);
        assert!((result - 7.0).abs() < 0.001);
    }

    #[test]
    fn test_calculate_weighted_average_single_factor() {
        let factors = create_test_factors_single();
        // Single factor: 9.5 * 1.0 / 1.0 = 9.5
        let result = calculate_weighted_average(&factors);
        assert!((result - 9.5).abs() < 0.001);
    }

    #[test]
    fn test_calculate_weighted_average_high_scores() {
        let factors = create_test_factors_high_scores();
        // Maximum scores: (10.0 * 0.7 + 10.0 * 0.3) / (0.7 + 0.3)
        // = (7.0 + 3.0) / 1.0 = 10.0
        let result = calculate_weighted_average(&factors);
        assert!((result - 10.0).abs() < 0.001);
    }

    #[test]
    fn test_calculate_weighted_average_low_scores() {
        let factors = create_test_factors_low_scores();
        // Low scores: (0.5 * 0.5 + 1.0 * 0.5) / (0.5 + 0.5)
        // = (0.25 + 0.5) / 1.0 = 0.75
        let result = calculate_weighted_average(&factors);
        assert!((result - 0.75).abs() < 0.001);
    }

    #[test]
    fn test_calculate_weighted_average_unequal_weights() {
        let factors = create_test_factors_unequal_weights();
        // (2.0 * 0.9 + 10.0 * 0.1) / (0.9 + 0.1)
        // = (1.8 + 1.0) / 1.0 = 2.8
        let result = calculate_weighted_average(&factors);
        assert!((result - 2.8).abs() < 0.001);
    }

    #[test]
    fn test_get_role_multiplier() {
        assert!((get_role_multiplier(&FunctionRole::PureLogic) - 1.2).abs() < 0.001);
        assert!((get_role_multiplier(&FunctionRole::EntryPoint) - 1.1).abs() < 0.001);
        assert!((get_role_multiplier(&FunctionRole::Orchestrator) - 0.9).abs() < 0.001);
        assert!((get_role_multiplier(&FunctionRole::IOWrapper) - 0.7).abs() < 0.001);
        assert!((get_role_multiplier(&FunctionRole::PatternMatch) - 0.5).abs() < 0.001);
        assert!((get_role_multiplier(&FunctionRole::Debug) - 0.4).abs() < 0.001);
        assert!((get_role_multiplier(&FunctionRole::Unknown) - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_aggregate_risk_factors() {
        let factors = create_test_factors_normal();
        // Base score: 6.6, role multiplier for PureLogic: 1.2
        // Result: 6.6 * 1.2 = 7.92
        let result = aggregate_risk_factors(&factors, &FunctionRole::PureLogic);
        assert!((result - 7.92).abs() < 0.001);
    }

    // Helper functions to create test data
    fn create_test_factors_normal() -> Vec<RiskFactor> {
        vec![
            create_complexity_factor(8.0, 0.5, 20, 15),
            create_coverage_factor(6.0, 0.3, 30.0),
            create_coupling_factor(4.0, 0.2, 10, 8),
        ]
    }

    fn create_test_factors_zero_weights() -> Vec<RiskFactor> {
        vec![
            create_complexity_factor(8.0, 0.0, 15, 10),
            create_coverage_factor(6.0, 0.0, 40.0),
        ]
    }

    fn create_test_factors_mixed_weights() -> Vec<RiskFactor> {
        vec![
            create_complexity_factor(8.0, 0.0, 15, 10),
            create_coverage_factor(5.0, 0.6, 50.0),
            create_coupling_factor(10.0, 0.4, 20, 15),
        ]
    }

    fn create_test_factors_single() -> Vec<RiskFactor> {
        vec![create_complexity_factor(9.5, 1.0, 30, 25)]
    }

    fn create_test_factors_high_scores() -> Vec<RiskFactor> {
        vec![
            create_complexity_factor(10.0, 0.7, 40, 35),
            create_coverage_factor(10.0, 0.3, 0.0),
        ]
    }

    fn create_test_factors_low_scores() -> Vec<RiskFactor> {
        vec![
            create_complexity_factor(0.5, 0.5, 3, 2),
            create_coverage_factor(1.0, 0.5, 95.0),
        ]
    }

    fn create_test_factors_unequal_weights() -> Vec<RiskFactor> {
        vec![
            create_complexity_factor(2.0, 0.9, 5, 3),
            create_coverage_factor(10.0, 0.1, 10.0),
        ]
    }

    fn classify_complexity_threshold(cyclomatic: u32) -> ComplexityThreshold {
        match () {
            _ if cyclomatic > 20 => ComplexityThreshold::Critical,
            _ if cyclomatic > 10 => ComplexityThreshold::High,
            _ if cyclomatic > 5 => ComplexityThreshold::Moderate,
            _ => ComplexityThreshold::Low,
        }
    }

    fn create_complexity_factor(
        score: f64,
        weight: f64,
        cyclomatic: u32,
        cognitive: u32,
    ) -> RiskFactor {
        RiskFactor {
            risk_type: RiskType::Complexity {
                cyclomatic,
                cognitive,
                lines: 100,
                threshold_type: classify_complexity_threshold(cyclomatic),
            },
            score,
            severity: score_to_severity(score),
            evidence: RiskEvidence::Complexity(ComplexityEvidence {
                cyclomatic_complexity: cyclomatic,
                cognitive_complexity: cognitive,
                lines_of_code: 100,
                nesting_depth: 3,
                threshold_exceeded: cyclomatic > 10,
                role_adjusted: false,
                comparison_to_baseline: score_to_comparison(score),
            }),
            remediation_actions: vec![],
            weight,
            confidence: 0.8,
        }
    }

    fn create_coverage_factor(score: f64, weight: f64, coverage: f64) -> RiskFactor {
        RiskFactor {
            risk_type: RiskType::Coverage {
                coverage_percentage: coverage,
                critical_paths_uncovered: ((100.0 - coverage) / 5.0) as u32,
                test_quality: coverage_to_quality(coverage),
            },
            score,
            severity: score_to_severity(score),
            evidence: RiskEvidence::Coverage(CoverageEvidence {
                coverage_percentage: coverage,
                critical_paths_uncovered: ((100.0 - coverage) / 5.0) as u32,
                test_count: (coverage / 5.0) as u32,
                test_quality: coverage_to_quality(coverage),
                comparison_to_baseline: score_to_comparison(score),
            }),
            remediation_actions: vec![],
            weight,
            confidence: 0.7,
        }
    }

    fn create_coupling_factor(score: f64, weight: f64, afferent: u32, efferent: u32) -> RiskFactor {
        let instability = efferent as f64 / (afferent + efferent) as f64;
        RiskFactor {
            risk_type: RiskType::Coupling {
                afferent_coupling: afferent,
                efferent_coupling: efferent,
                instability,
                circular_dependencies: 0,
            },
            score,
            severity: score_to_severity(score),
            evidence: RiskEvidence::Coupling(CouplingEvidence {
                afferent_coupling: afferent,
                efferent_coupling: efferent,
                instability,
                circular_dependencies: 0,
                comparison_to_baseline: score_to_comparison(score),
            }),
            remediation_actions: vec![],
            weight,
            confidence: 0.9,
        }
    }

    fn score_to_severity(score: f64) -> RiskSeverity {
        match score {
            s if s >= 9.0 => RiskSeverity::Critical,
            s if s >= 7.0 => RiskSeverity::High,
            s if s >= 4.0 => RiskSeverity::Moderate,
            s if s >= 2.0 => RiskSeverity::Low,
            _ => RiskSeverity::None,
        }
    }

    fn score_to_comparison(score: f64) -> ComparisonResult {
        match score {
            s if s >= 9.5 => ComparisonResult::AboveP95,
            s if s >= 9.0 => ComparisonResult::AboveP90,
            s if s >= 7.5 => ComparisonResult::AboveP75,
            s if s >= 5.0 => ComparisonResult::AboveMedian,
            _ => ComparisonResult::BelowMedian,
        }
    }

    fn coverage_to_quality(coverage: f64) -> TestQuality {
        match coverage {
            c if c >= 90.0 => TestQuality::Excellent,
            c if c >= 70.0 => TestQuality::Good,
            c if c >= 50.0 => TestQuality::Adequate,
            c if c > 0.0 => TestQuality::Poor,
            _ => TestQuality::Missing,
        }
    }
}
