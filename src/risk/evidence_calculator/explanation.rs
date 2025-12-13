//! Pure functions for generating risk explanations and recommendations.
//!
//! This module contains stateless functions that generate human-readable
//! explanations of risk assessments and prioritized recommendations.

use crate::priority::semantic_classifier::FunctionRole;
use crate::risk::evidence::{RemediationAction, RiskFactor, RiskSeverity, RiskType};

use super::role_utils::role_to_display_string;

/// Formats a RiskType into a human-readable description.
pub fn format_risk_type(risk_type: &RiskType) -> String {
    match risk_type {
        RiskType::Complexity {
            cyclomatic,
            cognitive,
            ..
        } => {
            format!("High complexity (cyclomatic: {cyclomatic}, cognitive: {cognitive})")
        }
        RiskType::Coverage {
            coverage_percentage,
            ..
        } => {
            format!("Low test coverage ({coverage_percentage:.0}%)")
        }
        RiskType::Coupling {
            afferent_coupling,
            efferent_coupling,
            ..
        } => {
            format!("High coupling (incoming: {afferent_coupling}, outgoing: {efferent_coupling})")
        }
        RiskType::ChangeFrequency {
            commits_last_month, ..
        } => {
            format!("Frequent changes ({commits_last_month} commits last month)")
        }
        RiskType::Architecture { .. } => "Architectural issues detected".to_string(),
    }
}

/// Formats a RiskSeverity into a human-readable description.
pub fn format_risk_severity(severity: RiskSeverity) -> &'static str {
    match severity {
        RiskSeverity::None => "no significant issues",
        RiskSeverity::Low => "minor issues",
        RiskSeverity::Moderate => "moderate issues requiring attention",
        RiskSeverity::High => "significant issues requiring prompt action",
        RiskSeverity::Critical => "critical issues requiring immediate attention",
    }
}

/// Finds the highest-impact risk factor from a list.
///
/// Impact is calculated as score × weight. Factors with zero weight are excluded.
pub fn find_highest_risk_factor(factors: &[RiskFactor]) -> Option<&RiskFactor> {
    factors.iter().filter(|f| f.weight > 0.0).max_by(|a, b| {
        (a.score * a.weight)
            .partial_cmp(&(b.score * b.weight))
            .unwrap_or(std::cmp::Ordering::Equal)
    })
}

/// Generates a human-readable explanation of the risk assessment.
pub fn generate_explanation(factors: &[RiskFactor], role: &FunctionRole, score: f64) -> String {
    let role_str = role_to_display_string(role);
    let mut explanation = format!("Risk score {score:.1}/10 for {role_str} function. ");

    if let Some(highest) = find_highest_risk_factor(factors) {
        let factor_desc = format_risk_type(&highest.risk_type);
        let severity_desc = format_risk_severity(highest.severity);

        explanation.push_str(&format!(
            "Primary factor: {factor_desc} with {severity_desc}."
        ));
    }

    explanation
}

/// Gets the effort estimate in hours for a remediation action.
pub fn get_effort_estimate(action: &RemediationAction) -> u32 {
    match action {
        RemediationAction::RefactorComplexity {
            estimated_effort_hours,
            ..
        } => *estimated_effort_hours,
        RemediationAction::AddTestCoverage {
            estimated_effort_hours,
            ..
        } => *estimated_effort_hours,
        RemediationAction::ReduceCoupling {
            estimated_effort_hours,
            ..
        } => *estimated_effort_hours,
        RemediationAction::ExtractLogic { .. } => 2, // Default low effort for extraction
    }
}

/// Generates prioritized recommendations from risk factors.
///
/// Returns the top 3 recommendations sorted by lowest estimated effort first.
pub fn generate_recommendations(factors: &[RiskFactor]) -> Vec<RemediationAction> {
    let mut all_actions: Vec<RemediationAction> = factors
        .iter()
        .flat_map(|f| f.remediation_actions.clone())
        .collect();

    // Sort by expected effort (lowest first) and take top 3
    all_actions.sort_by(|a, b| {
        let effort_a = get_effort_estimate(a);
        let effort_b = get_effort_estimate(b);
        effort_a.cmp(&effort_b)
    });

    all_actions.into_iter().take(3).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::risk::evidence::{
        ComparisonResult, ComplexityEvidence, ComplexityThreshold, RiskEvidence, TestQuality,
    };

    #[test]
    fn test_format_risk_type_complexity() {
        let risk_type = RiskType::Complexity {
            cyclomatic: 15,
            cognitive: 20,
            lines: 100,
            threshold_type: ComplexityThreshold::High,
        };
        assert_eq!(
            format_risk_type(&risk_type),
            "High complexity (cyclomatic: 15, cognitive: 20)"
        );
    }

    #[test]
    fn test_format_risk_type_coverage() {
        let risk_type = RiskType::Coverage {
            coverage_percentage: 45.5,
            critical_paths_uncovered: 3,
            test_quality: TestQuality::Poor,
        };
        assert_eq!(format_risk_type(&risk_type), "Low test coverage (46%)");
    }

    #[test]
    fn test_format_risk_type_coupling() {
        let risk_type = RiskType::Coupling {
            afferent_coupling: 10,
            efferent_coupling: 5,
            instability: 0.33,
            circular_dependencies: 0,
        };
        assert_eq!(
            format_risk_type(&risk_type),
            "High coupling (incoming: 10, outgoing: 5)"
        );
    }

    #[test]
    fn test_format_risk_severity() {
        assert_eq!(
            format_risk_severity(RiskSeverity::None),
            "no significant issues"
        );
        assert_eq!(format_risk_severity(RiskSeverity::Low), "minor issues");
        assert_eq!(
            format_risk_severity(RiskSeverity::Moderate),
            "moderate issues requiring attention"
        );
        assert_eq!(
            format_risk_severity(RiskSeverity::High),
            "significant issues requiring prompt action"
        );
        assert_eq!(
            format_risk_severity(RiskSeverity::Critical),
            "critical issues requiring immediate attention"
        );
    }

    #[test]
    fn test_find_highest_risk_factor_empty() {
        let factors: Vec<RiskFactor> = vec![];
        assert!(find_highest_risk_factor(&factors).is_none());
    }

    #[test]
    fn test_find_highest_risk_factor() {
        let factors = vec![
            create_test_factor(5.0, 0.5), // impact: 2.5
            create_test_factor(8.0, 0.3), // impact: 2.4
            create_test_factor(3.0, 1.0), // impact: 3.0 (highest)
        ];
        let highest = find_highest_risk_factor(&factors).unwrap();
        assert!((highest.score - 3.0).abs() < 0.001);
    }

    #[test]
    fn test_find_highest_risk_factor_skips_zero_weight() {
        let factors = vec![
            create_test_factor(10.0, 0.0), // zero weight, excluded
            create_test_factor(5.0, 0.5),  // impact: 2.5
        ];
        let highest = find_highest_risk_factor(&factors).unwrap();
        assert!((highest.score - 5.0).abs() < 0.001);
    }

    #[test]
    fn test_generate_explanation() {
        let factors = vec![create_test_factor(8.0, 0.5)];
        let explanation = generate_explanation(&factors, &FunctionRole::PureLogic, 7.5);
        assert!(explanation.contains("Risk score 7.5/10"));
        assert!(explanation.contains("pure logic"));
        assert!(explanation.contains("High complexity"));
    }

    #[test]
    fn test_get_effort_estimate() {
        use crate::risk::evidence::RefactoringTechnique;

        let action = RemediationAction::RefactorComplexity {
            current_complexity: 20,
            target_complexity: 10,
            estimated_effort_hours: 4,
            suggested_techniques: vec![RefactoringTechnique::ExtractMethod],
            expected_risk_reduction: 0.3,
        };
        assert_eq!(get_effort_estimate(&action), 4);

        let extract = RemediationAction::ExtractLogic {
            extraction_candidates: vec![],
            pure_function_opportunities: 2,
            testability_improvement: 0.5,
        };
        assert_eq!(get_effort_estimate(&extract), 2);
    }

    fn create_test_factor(score: f64, weight: f64) -> RiskFactor {
        RiskFactor {
            risk_type: RiskType::Complexity {
                cyclomatic: 15,
                cognitive: 20,
                lines: 100,
                threshold_type: ComplexityThreshold::High,
            },
            score,
            severity: RiskSeverity::High,
            evidence: RiskEvidence::Complexity(ComplexityEvidence {
                cyclomatic_complexity: 15,
                cognitive_complexity: 20,
                lines_of_code: 100,
                nesting_depth: 3,
                threshold_exceeded: true,
                role_adjusted: false,
                comparison_to_baseline: ComparisonResult::AboveP75,
            }),
            remediation_actions: vec![],
            weight,
            confidence: 0.8,
        }
    }
}
