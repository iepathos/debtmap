//! Pure functions for classifying risk levels.
//!
//! This module provides stateless functions that classify
//! a risk score into discrete risk classifications.

use crate::priority::semantic_classifier::FunctionRole;
use crate::risk::evidence::RiskClassification;

/// Calculates adjustment factor based on function role.
///
/// Some roles are more tolerant of certain risk scores:
/// - IOWrapper: +1.0 adjustment (more lenient for I/O)
/// - Orchestrator: +0.5 adjustment (slightly more lenient)
/// - PatternMatch: +1.5 adjustment (very lenient)
/// - Debug: +2.0 adjustment (very lenient)
/// - Others: 0.0 (standard thresholds)
pub fn calculate_role_adjustment(role: &FunctionRole) -> f64 {
    match role {
        FunctionRole::IOWrapper => 1.0,
        FunctionRole::Orchestrator => 0.5,
        FunctionRole::PatternMatch => 1.5,
        FunctionRole::Debug => 2.0,
        _ => 0.0,
    }
}

/// Classifies risk based on adjusted score.
///
/// Risk levels:
/// - WellDesigned: score <= 2.0
/// - Acceptable: score <= 4.0
/// - NeedsImprovement: score <= 7.0
/// - Risky: score <= 9.0
/// - Critical: score > 9.0
pub fn classify_by_score(adjusted_score: f64) -> RiskClassification {
    match adjusted_score {
        s if s <= 2.0 => RiskClassification::WellDesigned,
        s if s <= 4.0 => RiskClassification::Acceptable,
        s if s <= 7.0 => RiskClassification::NeedsImprovement,
        s if s <= 9.0 => RiskClassification::Risky,
        _ => RiskClassification::Critical,
    }
}

/// Classifies risk level with role-based adjustment.
///
/// The score is adjusted down based on the function's role,
/// then classified using standard thresholds.
pub fn classify_risk_level(score: f64, role: &FunctionRole) -> RiskClassification {
    let adjustment = calculate_role_adjustment(role);
    let adjusted_score = (score - adjustment).max(0.0);
    classify_by_score(adjusted_score)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_role_adjustment() {
        assert!((calculate_role_adjustment(&FunctionRole::IOWrapper) - 1.0).abs() < 0.001);
        assert!((calculate_role_adjustment(&FunctionRole::Orchestrator) - 0.5).abs() < 0.001);
        assert!((calculate_role_adjustment(&FunctionRole::PatternMatch) - 1.5).abs() < 0.001);
        assert!((calculate_role_adjustment(&FunctionRole::Debug) - 2.0).abs() < 0.001);
        assert!((calculate_role_adjustment(&FunctionRole::PureLogic) - 0.0).abs() < 0.001);
        assert!((calculate_role_adjustment(&FunctionRole::EntryPoint) - 0.0).abs() < 0.001);
        assert!((calculate_role_adjustment(&FunctionRole::Unknown) - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_classify_by_score() {
        assert_eq!(classify_by_score(0.0), RiskClassification::WellDesigned);
        assert_eq!(classify_by_score(2.0), RiskClassification::WellDesigned);
        assert_eq!(classify_by_score(2.1), RiskClassification::Acceptable);
        assert_eq!(classify_by_score(4.0), RiskClassification::Acceptable);
        assert_eq!(classify_by_score(4.1), RiskClassification::NeedsImprovement);
        assert_eq!(classify_by_score(7.0), RiskClassification::NeedsImprovement);
        assert_eq!(classify_by_score(7.1), RiskClassification::Risky);
        assert_eq!(classify_by_score(9.0), RiskClassification::Risky);
        assert_eq!(classify_by_score(9.1), RiskClassification::Critical);
        assert_eq!(classify_by_score(10.0), RiskClassification::Critical);
    }

    #[test]
    fn test_classify_risk_level_with_adjustment() {
        // Score 5.0 with IOWrapper role (adjustment 1.0) -> 4.0 -> Acceptable
        assert_eq!(
            classify_risk_level(5.0, &FunctionRole::IOWrapper),
            RiskClassification::Acceptable
        );

        // Score 5.0 with PureLogic role (no adjustment) -> 5.0 -> NeedsImprovement
        assert_eq!(
            classify_risk_level(5.0, &FunctionRole::PureLogic),
            RiskClassification::NeedsImprovement
        );

        // Score 3.0 with Debug role (adjustment 2.0) -> 1.0 -> WellDesigned
        assert_eq!(
            classify_risk_level(3.0, &FunctionRole::Debug),
            RiskClassification::WellDesigned
        );
    }

    #[test]
    fn test_classify_risk_level_floor_at_zero() {
        // Even with large adjustment, score should floor at 0.0
        assert_eq!(
            classify_risk_level(1.0, &FunctionRole::Debug),
            RiskClassification::WellDesigned
        );
    }
}
