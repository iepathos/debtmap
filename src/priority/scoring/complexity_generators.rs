//! Complexity-level based recommendation generators
//!
//! Pure functions that generate refactoring recommendations based on
//! cyclomatic complexity thresholds. Each generator is focused on a
//! specific complexity tier.
//! Following Stillwater philosophy: composition of small, pure functions.

use super::complexity_classification::{classify_complexity_level, ComplexityLevel};
use crate::priority::{DebtType, TransitiveCoverage};

/// Recommendation tuple: (action, rationale, steps)
pub type RecommendationOutput = (String, String, Vec<String>);

/// Generate complexity-based recommendation for risk debt
///
/// Dispatches to appropriate generator based on complexity level.
pub fn generate_complexity_risk_recommendation(
    cyclo: u32,
    coverage: &Option<TransitiveCoverage>,
    factors: &[String],
) -> RecommendationOutput {
    let complexity_level = classify_complexity_level(cyclo);
    let has_good_coverage = coverage.as_ref().map(|c| c.direct >= 0.8).unwrap_or(false);
    let has_coverage_issue = factors
        .iter()
        .any(|f| f.contains("coverage") || f.contains("Coverage") || f.contains("uncovered"));

    match complexity_level {
        ComplexityLevel::Low => generate_low_complexity_recommendation(cyclo, has_coverage_issue),
        ComplexityLevel::LowModerate => {
            generate_low_moderate_complexity_recommendation(cyclo, has_good_coverage)
        }
        ComplexityLevel::Moderate => {
            generate_moderate_complexity_recommendation(cyclo, has_good_coverage)
        }
        ComplexityLevel::High => {
            generate_high_complexity_recommendation(cyclo, has_good_coverage, has_coverage_issue)
        }
    }
}

/// Generate recommendation for low complexity functions (1-4)
pub fn generate_low_complexity_recommendation(
    cyclo: u32,
    has_coverage_issue: bool,
) -> RecommendationOutput {
    let action = if has_coverage_issue || cyclo > 3 {
        format!(
            "Extract helper functions for clarity, then add {} unit tests",
            cyclo.max(3)
        )
    } else {
        "Simplify function structure and improve testability".to_string()
    };

    (
        action,
        "Low complexity but flagged for improvement".to_string(),
        vec![
            "Extract helper functions for clarity".to_string(),
            "Remove unnecessary branching".to_string(),
            "Consolidate similar code paths".to_string(),
            format!(
                "Add {} unit tests for edge cases and main paths",
                cyclo.max(3)
            ),
        ],
    )
}

/// Generate recommendation for low-moderate complexity functions (5-6)
pub fn generate_low_moderate_complexity_recommendation(
    cyclo: u32,
    has_good_coverage: bool,
) -> RecommendationOutput {
    let functions_to_extract = 2;
    let target_complexity = 3;

    let action = if has_good_coverage {
        format!(
            "Extract {} pure functions (complexity {} -> {})",
            functions_to_extract, cyclo, target_complexity
        )
    } else {
        format!(
            "Extract {} pure functions (complexity {} -> {}) and add comprehensive tests",
            functions_to_extract, cyclo, target_complexity
        )
    };

    let mut steps = vec![
        format!(
            "Identify {} logical sections from {} branches:",
            functions_to_extract, cyclo
        ),
        format!(
            "  - Look for groups of ~{} related conditions",
            cyclo / functions_to_extract.max(1)
        ),
        format!(
            "  - Each extracted function should have complexity <={}",
            target_complexity
        ),
        "Extraction candidates:".to_string(),
        "  - Validation logic -> validate_preconditions()".to_string(),
        "  - Main logic -> process_core()".to_string(),
        "Move all I/O operations to a single orchestrator function".to_string(),
    ];

    if !has_good_coverage {
        steps.push(format!(
            "Write {} unit tests for the extracted pure functions",
            functions_to_extract * 3
        ));
        steps.push("Achieve 80%+ test coverage for all functions".to_string());
    } else {
        steps.push(format!(
            "Goal: Reduce cyclomatic complexity from {} to <={}",
            cyclo, target_complexity
        ));
    }

    (
        action,
        "Low-moderate complexity requiring refactoring".to_string(),
        steps,
    )
}

/// Generate recommendation for moderate complexity functions (7-10)
pub fn generate_moderate_complexity_recommendation(
    cyclo: u32,
    has_good_coverage: bool,
) -> RecommendationOutput {
    let functions_to_extract = (cyclo / 3).max(2);
    let target_complexity = 3;

    let action = if has_good_coverage {
        format!(
            "Extract {} pure functions (complexity {} -> {})",
            functions_to_extract, cyclo, target_complexity
        )
    } else {
        format!(
            "Extract {} pure functions (complexity {} -> {}) and add comprehensive tests",
            functions_to_extract, cyclo, target_complexity
        )
    };

    let mut steps = vec![
        format!(
            "Identify {} logical sections from {} branches:",
            functions_to_extract, cyclo
        ),
        format!(
            "  - Look for groups of ~{} related conditions",
            cyclo / functions_to_extract.max(1)
        ),
        format!(
            "  - Each extracted function should have complexity <={}",
            target_complexity
        ),
        "Extraction candidates:".to_string(),
        "  - Validation logic -> validate_preconditions()".to_string(),
        "  - Complex calculations -> calculate_specific()".to_string(),
        "  - Loop processing -> process_items()".to_string(),
        "Move all I/O operations to a single orchestrator function".to_string(),
    ];

    if !has_good_coverage {
        steps.push(format!(
            "Write {} unit tests for the extracted pure functions",
            functions_to_extract * 3
        ));
        steps.push("Achieve 80%+ test coverage for all functions".to_string());
    } else {
        steps.push(format!(
            "Goal: Reduce cyclomatic complexity from {} to <={}",
            cyclo, target_complexity
        ));
    }

    (
        action,
        "Moderate complexity requiring refactoring".to_string(),
        steps,
    )
}

/// Generate recommendation for high complexity functions (11+)
pub fn generate_high_complexity_recommendation(
    cyclo: u32,
    has_good_coverage: bool,
    has_coverage_issue: bool,
) -> RecommendationOutput {
    let functions_to_extract = (cyclo / 4).max(3);
    let target_complexity = 5;

    let action = if has_good_coverage {
        format!(
            "Decompose into {} pure functions (complexity {} -> {})",
            functions_to_extract, cyclo, target_complexity
        )
    } else if has_coverage_issue {
        format!(
            "Add {} tests, then decompose into {} pure functions (complexity {} -> {})",
            cyclo, functions_to_extract, cyclo, target_complexity
        )
    } else {
        format!(
            "Decompose into {} pure functions (complexity {} -> {}) with comprehensive tests",
            functions_to_extract, cyclo, target_complexity
        )
    };

    let mut steps = vec![
        format!(
            "This high-complexity function needs to be broken down into {} logical units:",
            functions_to_extract
        ),
        format!("1. Map {} execution paths into logical groupings:", cyclo),
        format!("  - ~{} paths for input validation", cyclo / 4),
        format!("  - ~{} paths for core logic", cyclo / 2),
        format!("  - ~{} paths for output/error handling", cyclo / 4),
    ];

    if has_coverage_issue && !has_good_coverage {
        steps.extend(vec![
            format!(
                "2. Add {} unit tests before refactoring to prevent regressions:",
                cyclo
            ),
            "  - Focus on happy path and major error conditions first".to_string(),
            "  - Cover all branches for confidence in refactoring".to_string(),
        ]);
    }

    let step_num = if has_coverage_issue && !has_good_coverage {
        3
    } else {
        2
    };

    steps.extend(vec![
        format!(
            "{}. Extract functions with single responsibilities:",
            step_num
        ),
        "  - validate_inputs() for precondition checks".to_string(),
        "  - process_core_logic() for main algorithm".to_string(),
        "  - handle_results() for output formatting".to_string(),
        "  - handle_errors() for error cases".to_string(),
        format!(
            "{}. Each function should have cyclomatic complexity <={}",
            step_num + 1,
            target_complexity
        ),
        format!(
            "{}. Add unit tests for each extracted function (~3-5 tests per function)",
            step_num + 2
        ),
    ]);

    (
        action,
        "High complexity requiring decomposition".to_string(),
        steps,
    )
}

/// Generate recommendation for complexity hotspots
pub fn generate_complexity_hotspot_recommendation(
    cyclomatic: u32,
    cognitive: u32,
) -> RecommendationOutput {
    use crate::priority::scoring::recommendation::calculate_functions_to_extract;

    let functions_to_extract = calculate_functions_to_extract(cyclomatic, cognitive);
    let target_per_function = (cyclomatic / functions_to_extract).max(3);

    (
        format!(
            "Extract {} pure functions, each handling ~{} branches (complexity {} -> ~{})",
            functions_to_extract,
            cyclomatic / functions_to_extract.max(1),
            cyclomatic,
            target_per_function
        ),
        format!(
            "High complexity function (cyclo={}, cog={}) likely with low coverage - needs testing and refactoring",
            cyclomatic, cognitive
        ),
        vec![
            format!(
                "Identify {} branch clusters from {} total branches:",
                functions_to_extract, cyclomatic
            ),
            format!(
                "  - Each cluster should handle ~{} related conditions",
                cyclomatic / functions_to_extract.max(1)
            ),
            "Common extraction patterns:".to_string(),
            "  - Early validation checks -> validate_preconditions()".to_string(),
            "  - Complex calculations in branches -> calculate_[specific]()".to_string(),
            "  - Data processing in loops -> process_[item_type]()".to_string(),
            "  - Error handling branches -> handle_[error_case]()".to_string(),
            format!(
                "Each extracted function should have cyclomatic complexity <={}",
                target_per_function
            ),
            format!(
                "Write ~{} tests per extracted function for full branch coverage",
                target_per_function
            ),
            "Use property-based testing for complex logic validation".to_string(),
        ],
    )
}

/// Generate recommendation for infrastructure debt types
pub fn generate_infrastructure_recommendation_with_coverage(
    debt_type: &DebtType,
    coverage: &Option<TransitiveCoverage>,
) -> RecommendationOutput {
    match debt_type {
        DebtType::Duplication {
            instances,
            total_lines,
        } => (
            "Extract common logic into shared module".to_string(),
            format!("Duplicated across {instances} locations ({total_lines} lines total)"),
            vec![
                "Create shared utility module".to_string(),
                "Replace duplicated code with calls to shared module".to_string(),
                "Add comprehensive tests to shared module".to_string(),
            ],
        ),
        DebtType::Risk {
            risk_score,
            factors,
        } => {
            let has_complexity_issue = factors.iter().any(|f| {
                f.contains("complexity") || f.contains("cyclomatic") || f.contains("cognitive")
            });

            if has_complexity_issue {
                let cyclo = extract_cyclomatic_from_factors(factors).unwrap_or(0);
                let (action, _, steps) =
                    generate_complexity_risk_recommendation(cyclo, coverage, factors);
                (
                    action,
                    format!("Risk score {:.1}: {}", risk_score, factors.join(", ")),
                    steps,
                )
            } else {
                (
                    "Address identified risk factors".to_string(),
                    format!("Risk score {:.1}: {}", risk_score, factors.join(", ")),
                    vec![
                        "Review and refactor problematic areas".to_string(),
                        "Add missing tests if coverage is low".to_string(),
                        "Update documentation".to_string(),
                    ],
                )
            }
        }
        DebtType::ComplexityHotspot {
            cyclomatic,
            cognitive,
        } => generate_complexity_hotspot_recommendation(*cyclomatic, *cognitive),
        _ => unreachable!("Not an infrastructure debt type"),
    }
}

/// Extract cyclomatic complexity value from factors strings
fn extract_cyclomatic_from_factors(factors: &[String]) -> Option<u32> {
    factors
        .iter()
        .find(|f| f.contains("cyclomatic"))
        .and_then(|f| {
            f.split(':')
                .nth(1)?
                .trim()
                .strip_suffix(')')?
                .parse::<u32>()
                .ok()
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_low_complexity_recommendation() {
        let (action, rationale, steps) = generate_low_complexity_recommendation(3, true);

        assert!(action.contains("unit tests"));
        assert!(rationale.contains("Low complexity"));
        assert!(!steps.is_empty());
        assert!(steps.iter().any(|s| s.contains("unit tests")));

        let (action2, rationale2, steps2) = generate_low_complexity_recommendation(3, false);

        assert!(action2.contains("Simplify function structure"));
        assert!(rationale2.contains("Low complexity"));
        assert!(!steps2.is_empty());
    }

    #[test]
    fn test_generate_low_moderate_complexity_recommendation() {
        let (action, rationale, steps) = generate_low_moderate_complexity_recommendation(5, true);

        assert!(action.contains("Extract"));
        assert!(action.contains("pure functions"));
        assert!(rationale.contains("Low-moderate complexity"));
        assert!(!steps.is_empty());

        let (action2, rationale2, steps2) =
            generate_low_moderate_complexity_recommendation(5, false);

        assert!(action2.contains("Extract"));
        assert!(action2.contains("comprehensive tests"));
        assert!(rationale2.contains("Low-moderate complexity"));
        assert!(steps2.iter().any(|s| s.contains("test")));
    }

    #[test]
    fn test_generate_moderate_complexity_recommendation() {
        let (action, rationale, steps) = generate_moderate_complexity_recommendation(9, true);

        assert!(action.contains("Extract"));
        assert!(action.contains("pure functions"));
        assert!(rationale.contains("Moderate complexity"));
        assert!(!steps.is_empty());
        assert!(steps.iter().any(|s| s.contains("logical sections")));

        let (action2, rationale2, steps2) = generate_moderate_complexity_recommendation(9, false);

        assert!(action2.contains("Extract"));
        assert!(action2.contains("comprehensive tests"));
        assert!(rationale2.contains("Moderate complexity"));
        assert!(steps2.iter().any(|s| s.contains("unit tests")));
    }

    #[test]
    fn test_generate_high_complexity_recommendation() {
        let (action, rationale, steps) = generate_high_complexity_recommendation(15, true, false);

        assert!(action.contains("Decompose"));
        assert!(action.contains("pure functions"));
        assert!(rationale.contains("High complexity"));
        assert!(!steps.is_empty());

        let (action2, rationale2, steps2) =
            generate_high_complexity_recommendation(15, false, true);

        assert!(action2.contains("Add"));
        assert!(action2.contains("tests"));
        assert!(action2.contains("decompose"));
        assert!(rationale2.contains("High complexity"));
        assert!(!steps2.is_empty());

        let (action3, rationale3, steps3) =
            generate_high_complexity_recommendation(15, false, false);

        assert!(action3.contains("Decompose"));
        assert!(action3.contains("comprehensive tests"));
        assert!(rationale3.contains("High complexity"));
        assert!(!steps3.is_empty());
    }

    #[test]
    fn test_generate_complexity_hotspot_recommendation() {
        let (action, rationale, steps) = generate_complexity_hotspot_recommendation(20, 30);

        assert!(action.contains("Extract"));
        assert!(action.contains("pure functions"));
        assert!(rationale.contains("High complexity"));
        assert!(rationale.contains("cyclo=20"));
        assert!(rationale.contains("cog=30"));
        assert!(!steps.is_empty());
        assert!(steps.iter().any(|s| s.contains("branch clusters")));
        assert!(steps.iter().any(|s| s.contains("property-based testing")));
    }

    #[test]
    fn test_extract_cyclomatic_from_factors() {
        let factors = vec![
            "High cyclomatic complexity: 15)".to_string(),
            "Low coverage".to_string(),
        ];
        assert_eq!(extract_cyclomatic_from_factors(&factors), Some(15));

        let no_cyclo = vec!["Low coverage".to_string()];
        assert_eq!(extract_cyclomatic_from_factors(&no_cyclo), None);
    }
}
