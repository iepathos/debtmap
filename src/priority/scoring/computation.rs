//! Computation module - Pure functions for calculating metrics and impacts
//!
//! This module contains all pure computation functions for calculating various metrics,
//! scores, and expected impacts. All functions in this module should be pure (no side effects)
//! and easy to test independently.

use crate::core::FunctionMetrics;
use crate::priority::unified_scorer::EntropyDetails;
use crate::priority::{
    scoring::test_calculation::{calculate_tests_needed, ComplexityTier},
    DebtType, ImpactMetrics, UnifiedScore,
};

/// Helper function to calculate entropy details from FunctionMetrics
/// Pure function - transforms function metrics into entropy details
pub(super) fn calculate_entropy_details(func: &FunctionMetrics) -> Option<EntropyDetails> {
    func.entropy_score.as_ref().map(|entropy_score| {
        // Use the new framework's dampening calculation
        let calculator = crate::complexity::entropy_core::UniversalEntropyCalculator::new(
            crate::complexity::entropy_core::EntropyConfig::default(),
        );
        let dampening_value = calculator.apply_dampening(entropy_score);
        let dampening_factor = (dampening_value / 2.0).clamp(0.5, 1.0); // Normalize to 0.5-1.0 range

        let adjusted_cyclomatic = (func.cyclomatic as f64 * dampening_factor) as u32;
        let _adjusted_cognitive = (func.cognitive as f64 * dampening_factor) as u32;

        EntropyDetails {
            entropy_score: entropy_score.token_entropy,
            pattern_repetition: entropy_score.pattern_repetition,
            original_complexity: func.cyclomatic,
            adjusted_complexity: adjusted_cyclomatic,
            dampening_factor,
        }
    })
}

/// Pure function - calculates risk score from function metrics
/// Returns a score from 0.0 to 10.0 based on complexity and length
pub(super) fn calculate_risk_score(func: &FunctionMetrics) -> f64 {
    // Better scaling for complexity risk (0-1 range)
    // Cyclomatic 10 = 0.33, 20 = 0.67, 30+ = 1.0
    let cyclo_risk = (func.cyclomatic as f64 / 30.0).min(1.0);

    // Cognitive complexity tends to be higher, so scale differently
    // Cognitive 15 = 0.33, 30 = 0.67, 45+ = 1.0
    let cognitive_risk = (func.cognitive as f64 / 45.0).min(1.0);

    // Length risk - functions over 100 lines are definitely risky
    let length_risk = (func.length as f64 / 100.0).min(1.0);

    // Average the three risk factors
    // Complexity is most important, then cognitive, then length
    let weighted_risk = cyclo_risk * 0.4 + cognitive_risk * 0.4 + length_risk * 0.2;

    // Scale to 0-10 range for final risk score
    // Note: Coverage is handled separately in the unified scoring system
    weighted_risk * 10.0
}

/// Pure function - calculates how many functions should be extracted
/// Based on cyclomatic and cognitive complexity
pub(super) fn calculate_functions_to_extract(cyclomatic: u32, cognitive: u32) -> u32 {
    let max_complexity = cyclomatic.max(cognitive);
    // Target complexity per function is 3-5
    // Calculate how many functions needed to achieve this
    match max_complexity {
        0..=10 => 2,                      // Extract 2 functions: 10/2 = 5 complexity each
        11..=15 => 3,                     // Extract 3 functions: 15/3 = 5 complexity each
        16..=20 => 4,                     // Extract 4 functions: 20/4 = 5 complexity each
        21..=25 => 5,                     // Extract 5 functions: 25/5 = 5 complexity each
        26..=30 => 6,                     // Extract 6 functions: 30/6 = 5 complexity each
        _ => (max_complexity / 5).max(6), // For very high complexity, aim for ~5 per function
    }
}

/// Calculate test cases needed based on complexity and current coverage
/// Delegates to unified test_calculation module (Moderate/High tier)
pub(super) fn calculate_needed_test_cases(cyclomatic: u32, coverage_pct: f64) -> u32 {
    // Use appropriate tier based on complexity
    let tier = if cyclomatic > 30 {
        ComplexityTier::High
    } else if cyclomatic > 10 {
        ComplexityTier::Moderate
    } else {
        ComplexityTier::Simple
    };

    calculate_tests_needed(cyclomatic, coverage_pct, Some(tier)).count
}

/// Calculate approximate test cases for simple functions
/// Delegates to unified test_calculation module (Simple tier)
pub(super) fn calculate_simple_test_cases(cyclomatic: u32, coverage_pct: f64) -> u32 {
    calculate_tests_needed(cyclomatic, coverage_pct, Some(ComplexityTier::Simple)).count
}

/// Pure function - determines if a function is considered complex
pub(super) fn is_function_complex(cyclomatic: u32, cognitive: u32) -> bool {
    cyclomatic > 10 || cognitive > 15
}

/// Pure function - calculates the risk reduction factor based on debt type
pub(super) fn calculate_risk_factor(debt_type: &DebtType) -> f64 {
    match debt_type {
        DebtType::TestingGap { .. } => 0.42,
        DebtType::ComplexityHotspot { .. } => 0.35,
        DebtType::ErrorSwallowing { .. } => 0.35, // High risk - can hide critical failures
        DebtType::DeadCode { .. } => 0.3,
        DebtType::Duplication { .. } => 0.25,
        DebtType::Risk { .. } => 0.2,
        DebtType::TestComplexityHotspot { .. } => 0.15,
        DebtType::TestTodo { .. } | DebtType::TestDuplication { .. } => 0.1,
        // Resource Management debt types (medium risk)
        DebtType::BlockingIO { .. } => 0.45,
        DebtType::NestedLoops { .. } => 0.4,
        DebtType::AllocationInefficiency { .. } => 0.3,
        DebtType::StringConcatenation { .. } => 0.25,
        DebtType::SuboptimalDataStructure { .. } => 0.2,
        // Organization debt types (maintenance risk)
        DebtType::GodObject { .. } => 0.4,
        DebtType::GodModule { .. } => 0.4,
        DebtType::FeatureEnvy { .. } => 0.25,
        DebtType::PrimitiveObsession { .. } => 0.2,
        DebtType::MagicValues { .. } => 0.15,
        // Testing quality debt types (low risk)
        DebtType::FlakyTestPattern { .. } => 0.3,
        DebtType::AssertionComplexity { .. } => 0.15,
        // Resource management debt types (medium risk)
        DebtType::ResourceLeak { .. } => 0.5,
        DebtType::AsyncMisuse { .. } => 0.4,
        DebtType::CollectionInefficiency { .. } => 0.2,
    }
}

/// Pure function - calculates coverage improvement potential for testing gaps
pub(super) fn calculate_coverage_improvement(coverage: f64, is_complex: bool) -> f64 {
    let potential = 1.0 - coverage;
    if is_complex {
        potential * 50.0 // 50% of potential due to complexity
    } else {
        potential * 100.0 // Full coverage potential for simple functions
    }
}

/// Pure function - calculates lines that could be reduced through refactoring
pub(super) fn calculate_lines_reduction(debt_type: &DebtType) -> u32 {
    match debt_type {
        DebtType::DeadCode {
            cyclomatic,
            cognitive,
            ..
        } => *cyclomatic + *cognitive,
        DebtType::Duplication {
            instances,
            total_lines,
        }
        | DebtType::TestDuplication {
            instances,
            total_lines,
            ..
        } => *total_lines - (*total_lines / instances),
        _ => 0,
    }
}

/// Pure function - calculates complexity reduction potential based on debt type
pub(super) fn calculate_complexity_reduction(debt_type: &DebtType, is_complex: bool) -> f64 {
    match debt_type {
        DebtType::DeadCode {
            cyclomatic,
            cognitive,
            ..
        } => (*cyclomatic + *cognitive) as f64 * 0.5,
        DebtType::TestingGap { cyclomatic, .. } if is_complex => *cyclomatic as f64 * 0.3,
        DebtType::ComplexityHotspot { cyclomatic, .. } => *cyclomatic as f64 * 0.5,
        DebtType::TestComplexityHotspot { cyclomatic, .. } => *cyclomatic as f64 * 0.3,
        // Organization debt types - significant complexity reduction potential
        DebtType::GodObject {
            god_object_score, ..
        } => *god_object_score * 0.4,
        DebtType::NestedLoops { depth, .. } => (*depth as f64).powf(2.0) * 0.3, // Quadratic impact
        DebtType::FeatureEnvy { .. } => 2.0, // Modest improvement
        _ => 0.0,
    }
}

/// Pure function - calculates expected impact metrics for addressing technical debt
/// Composes other pure calculation functions to determine overall impact
pub(super) fn calculate_expected_impact(
    _func: &FunctionMetrics,
    debt_type: &DebtType,
    score: &UnifiedScore,
) -> ImpactMetrics {
    let risk_factor = calculate_risk_factor(debt_type);
    let risk_reduction = score.final_score * risk_factor;

    let (coverage_improvement, lines_reduction, complexity_reduction) = match debt_type {
        DebtType::TestingGap {
            coverage,
            cyclomatic,
            cognitive,
        } => {
            let is_complex = is_function_complex(*cyclomatic, *cognitive);
            (
                calculate_coverage_improvement(*coverage, is_complex),
                0,
                calculate_complexity_reduction(debt_type, is_complex),
            )
        }
        _ => (
            0.0,
            calculate_lines_reduction(debt_type),
            calculate_complexity_reduction(debt_type, false),
        ),
    };

    ImpactMetrics {
        coverage_improvement,
        lines_reduction,
        complexity_reduction,
        risk_reduction,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_risk_score_low_complexity() {
        let func = FunctionMetrics {
            name: "test".to_string(),
            file: "test.rs".into(),
            line: 1,
            cyclomatic: 5,
            cognitive: 3,
            nesting: 1,
            length: 20,
            is_test: false,
            visibility: None,
            is_trait_method: false,
            in_test_module: false,
            entropy_score: None,
            is_pure: None,
            purity_confidence: None,
            detected_patterns: None,
            upstream_callers: None,
            downstream_callees: None,
        };
        let risk = calculate_risk_score(&func);
        assert!(risk < 3.0, "Low complexity should have low risk score");
    }

    #[test]
    fn test_calculate_risk_score_high_complexity() {
        let func = FunctionMetrics {
            name: "test".to_string(),
            file: "test.rs".into(),
            line: 1,
            cyclomatic: 30,
            cognitive: 45,
            nesting: 3,
            length: 100,
            is_test: false,
            visibility: None,
            is_trait_method: false,
            in_test_module: false,
            entropy_score: None,
            is_pure: None,
            purity_confidence: None,
            detected_patterns: None,
            upstream_callers: None,
            downstream_callees: None,
        };
        let risk = calculate_risk_score(&func);
        assert!(risk >= 8.0, "High complexity should have high risk score");
    }

    #[test]
    fn test_calculate_functions_to_extract() {
        assert_eq!(calculate_functions_to_extract(8, 6), 2);
        assert_eq!(calculate_functions_to_extract(15, 12), 3);
        assert_eq!(calculate_functions_to_extract(20, 18), 4);
        assert_eq!(calculate_functions_to_extract(50, 40), 10);
    }

    #[test]
    fn test_is_function_complex() {
        assert!(!is_function_complex(5, 8));
        assert!(is_function_complex(15, 8));
        assert!(is_function_complex(8, 20));
        assert!(is_function_complex(15, 20));
    }

    #[test]
    fn test_calculate_coverage_improvement_simple() {
        let improvement = calculate_coverage_improvement(0.5, false);
        assert!((improvement - 50.0).abs() < 0.01);
    }

    #[test]
    fn test_calculate_coverage_improvement_complex() {
        let improvement = calculate_coverage_improvement(0.5, true);
        assert!((improvement - 25.0).abs() < 0.01);
    }

    #[test]
    fn test_calculate_risk_factor() {
        assert_eq!(
            calculate_risk_factor(&DebtType::TestingGap {
                coverage: 0.5,
                cyclomatic: 10,
                cognitive: 8
            }),
            0.42
        );
        assert_eq!(
            calculate_risk_factor(&DebtType::ComplexityHotspot {
                cyclomatic: 20,
                cognitive: 15
            }),
            0.35
        );
        assert_eq!(
            calculate_risk_factor(&DebtType::DeadCode {
                cyclomatic: 5,
                cognitive: 3,
                visibility: crate::priority::FunctionVisibility::Private,
                usage_hints: vec![]
            }),
            0.3
        );
    }
}
