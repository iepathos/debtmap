// Concise recommendation generation (spec 138a)
//
// This module generates actionable recommendations with:
// - Maximum 5 high-level steps per recommendation
// - Clear impact estimates for each step
// - Difficulty indicators (Easy/Medium/Hard)
// - Executable commands for each step
// - Estimated total effort in hours

use crate::core::FunctionMetrics;
use crate::priority::semantic_classifier::FunctionRole;
use crate::priority::{
    ActionStep, ActionableRecommendation, DebtType, Difficulty, FunctionVisibility,
    TransitiveCoverage,
};

use super::test_calculation::{calculate_tests_needed, ComplexityTier};

/// Generate concise recommendation from debt type and metrics
pub fn generate_concise_recommendation(
    debt_type: &DebtType,
    metrics: &FunctionMetrics,
    role: FunctionRole,
    coverage: &Option<TransitiveCoverage>,
) -> ActionableRecommendation {
    match debt_type {
        DebtType::TestingGap {
            coverage: cov,
            cyclomatic,
            cognitive,
        } => generate_testing_gap_steps(*cov, *cyclomatic, *cognitive, metrics, role, coverage),
        DebtType::ComplexityHotspot {
            cyclomatic,
            cognitive,
        } => generate_complexity_steps(*cyclomatic, *cognitive, metrics),
        DebtType::DeadCode {
            visibility,
            cyclomatic,
            cognitive,
            ..
        } => generate_dead_code_steps(visibility, *cyclomatic, *cognitive, metrics),
        _ => {
            // Fallback for other debt types - use legacy format
            ActionableRecommendation {
                primary_action: "Address technical debt".to_string(),
                rationale: "This item requires attention".to_string(),
                implementation_steps: vec!["Review and address the issue".to_string()],
                related_items: vec![],
                steps: None,
                estimated_effort_hours: None,
            }
        }
    }
}

/// Generate testing gap recommendation with max 5 steps
fn generate_testing_gap_steps(
    coverage_pct: f64,
    cyclomatic: u32,
    cognitive: u32,
    metrics: &FunctionMetrics,
    _role: FunctionRole,
    _transitive_cov: &Option<TransitiveCoverage>,
) -> ActionableRecommendation {
    let tier = if cyclomatic > 30 {
        ComplexityTier::High
    } else if cyclomatic > 10 {
        ComplexityTier::Moderate
    } else {
        ComplexityTier::Simple
    };

    let test_result = calculate_tests_needed(cyclomatic, coverage_pct, Some(tier));
    let tests_needed = test_result.count;
    let coverage_gap = (100.0 - coverage_pct * 100.0) as u32;

    let mut steps = vec![];

    // Step 1: Add tests (highest impact)
    if tests_needed > 0 {
        steps.push(ActionStep {
            description: format!(
                "Add {} tests for {}% coverage gap",
                tests_needed, coverage_gap
            ),
            impact: format!("+{} tests, reduce risk", tests_needed),
            difficulty: Difficulty::for_testing(tests_needed, cyclomatic),
            commands: vec![
                format!("cargo test {}::", metrics.name),
                "# Write focused tests covering critical paths".to_string(),
            ],
        });
    }

    // Step 2: Refactoring (only if complex)
    if cyclomatic > 15 || cognitive > 20 {
        let target_complexity = 10;
        let complexity_reduction = (cyclomatic.saturating_sub(target_complexity)).max(5);

        steps.push(ActionStep {
            description: "Extract complex branches into focused functions".to_string(),
            impact: format!("-{} complexity", complexity_reduction),
            difficulty: Difficulty::for_refactoring(cyclomatic, cognitive),
            commands: vec!["cargo clippy -- -W clippy::cognitive_complexity".to_string()],
        });
    }

    // Step 3: Verify (always include if we have steps)
    if !steps.is_empty() {
        steps.push(ActionStep {
            description: "Verify tests pass and coverage improved".to_string(),
            impact: if tests_needed > 0 {
                format!("Confirmed +{}% coverage", coverage_gap)
            } else {
                "Confirmed refactoring safe".to_string()
            },
            difficulty: Difficulty::Easy,
            commands: vec![
                "cargo test --all".to_string(),
                "# Run coverage tool to verify improvement".to_string(),
            ],
        });
    }

    let estimated_effort = estimate_effort(cyclomatic, tests_needed);

    ActionableRecommendation {
        primary_action: if tests_needed > 0 {
            format!("Add {} tests for untested branches", tests_needed)
        } else {
            "Maintain test coverage".to_string()
        },
        rationale: format!(
            "Function has {}% coverage with complexity {}/{}. Needs {} tests minimum.",
            (coverage_pct * 100.0) as u32,
            cyclomatic,
            cognitive,
            tests_needed
        ),
        implementation_steps: vec![], // Legacy field - empty for new recommendations
        related_items: vec![],
        steps: Some(steps),
        estimated_effort_hours: Some(estimated_effort),
    }
}

/// Generate complexity hotspot recommendation
fn generate_complexity_steps(
    cyclomatic: u32,
    cognitive: u32,
    metrics: &FunctionMetrics,
) -> ActionableRecommendation {
    let functions_to_extract = calculate_functions_to_extract(cyclomatic, cognitive);
    let target_complexity = 10;
    let complexity_reduction = cyclomatic.saturating_sub(target_complexity);

    let steps = vec![
        ActionStep {
            description: "Add tests before refactoring (if coverage < 80%)".to_string(),
            impact: "+safety net for refactoring".to_string(),
            difficulty: Difficulty::Medium,
            commands: vec![format!("cargo test {}::", metrics.name)],
        },
        ActionStep {
            description: format!("Extract {} focused functions", functions_to_extract),
            impact: format!("-{} complexity", complexity_reduction),
            difficulty: Difficulty::for_refactoring(cyclomatic, cognitive),
            commands: vec!["cargo clippy".to_string()],
        },
        ActionStep {
            description: "Verify tests still pass".to_string(),
            impact: "Confirmed refactoring safe".to_string(),
            difficulty: Difficulty::Easy,
            commands: vec!["cargo test --all".to_string()],
        },
    ];

    let estimated_effort = (cyclomatic as f32 / 10.0) * 1.5; // ~1.5hr per 10 complexity

    ActionableRecommendation {
        primary_action: format!(
            "Reduce complexity from {} to ~{}",
            cyclomatic, target_complexity
        ),
        rationale: format!(
            "High complexity {}/{} makes function hard to test and maintain",
            cyclomatic, cognitive
        ),
        implementation_steps: vec![], // Legacy field
        related_items: vec![],
        steps: Some(steps),
        estimated_effort_hours: Some(estimated_effort),
    }
}

/// Generate dead code recommendation
fn generate_dead_code_steps(
    visibility: &FunctionVisibility,
    cyclomatic: u32,
    cognitive: u32,
    _metrics: &FunctionMetrics,
) -> ActionableRecommendation {
    let steps = match visibility {
        FunctionVisibility::Public => vec![
            ActionStep {
                description: "Verify function is not used by external crates".to_string(),
                impact: "Reduced public API surface".to_string(),
                difficulty: Difficulty::Medium,
                commands: vec!["cargo tree --all-features".to_string()],
            },
            ActionStep {
                description: "Remove the function if truly unused".to_string(),
                impact: format!("-{} lines, -{} complexity", cyclomatic * 3, cyclomatic),
                difficulty: Difficulty::Easy,
                commands: vec![],
            },
        ],
        FunctionVisibility::Private | FunctionVisibility::Crate => vec![
            ActionStep {
                description: "Confirm no callers in codebase".to_string(),
                impact: "Safe to remove".to_string(),
                difficulty: Difficulty::Easy,
                commands: vec!["rg \"function_name\"".to_string()],
            },
            ActionStep {
                description: "Remove the function".to_string(),
                impact: format!("-{} lines, -{} complexity", cyclomatic * 3, cyclomatic),
                difficulty: Difficulty::Easy,
                commands: vec![],
            },
        ],
    };

    let estimated_effort = 0.5; // 30 minutes for dead code removal

    ActionableRecommendation {
        primary_action: "Remove unused function".to_string(),
        rationale: format!(
            "Unused {:?} function with complexity {}/{}",
            visibility, cyclomatic, cognitive
        ),
        implementation_steps: vec![],
        related_items: vec![],
        steps: Some(steps),
        estimated_effort_hours: Some(estimated_effort),
    }
}

/// Estimate effort in hours based on metrics
fn estimate_effort(cyclomatic: u32, tests_needed: u32) -> f32 {
    // Base: 10-15 min per test
    let test_effort = tests_needed as f32 * 0.2;

    // Refactoring effort (if needed)
    let refactor_effort = if cyclomatic > 15 {
        (cyclomatic as f32 - 10.0) / 10.0 * 1.5 // ~1.5hr per 10 complexity reduction
    } else {
        0.0
    };

    // Round to nearest 0.5 hours
    ((test_effort + refactor_effort) * 2.0).round() / 2.0
}

/// Calculate number of functions to extract based on complexity
fn calculate_functions_to_extract(cyclomatic: u32, cognitive: u32) -> u32 {
    if cyclomatic > 30 || cognitive > 40 {
        4
    } else if cyclomatic > 20 || cognitive > 30 {
        3
    } else if cyclomatic > 15 || cognitive > 20 {
        2
    } else {
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn create_test_metrics(cyclomatic: u32, cognitive: u32) -> FunctionMetrics {
        FunctionMetrics {
            name: "test_func".to_string(),
            file: PathBuf::from("test.rs"),
            line: 10,
            cyclomatic,
            cognitive,
            nesting: 2,
            length: 50,
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
            mapping_pattern_result: None,
            adjusted_complexity: None,
            composition_metrics: None,
        }
    }

    #[test]
    fn test_max_5_steps_per_recommendation() {
        let metrics = create_test_metrics(20, 25);
        let rec = generate_concise_recommendation(
            &DebtType::ComplexityHotspot {
                cyclomatic: 20,
                cognitive: 25,
            },
            &metrics,
            FunctionRole::PureLogic,
            &None,
        );

        if let Some(steps) = &rec.steps {
            assert!(
                steps.len() <= 5,
                "Should have at most 5 steps, got {}",
                steps.len()
            );
        }
    }

    #[test]
    fn test_all_steps_have_impact() {
        let metrics = create_test_metrics(15, 18);
        let rec = generate_testing_gap_steps(0.5, 15, 18, &metrics, FunctionRole::PureLogic, &None);

        if let Some(steps) = &rec.steps {
            for step in steps {
                assert!(
                    !step.impact.is_empty(),
                    "Step '{}' missing impact",
                    step.description
                );
            }
        }
    }

    #[test]
    fn test_steps_ordered_by_impact() {
        let metrics = create_test_metrics(25, 30);
        let rec = generate_testing_gap_steps(0.3, 25, 30, &metrics, FunctionRole::PureLogic, &None);

        if let Some(steps) = &rec.steps {
            // First step should be testing (highest impact for testing gap)
            assert!(
                steps[0].description.contains("test"),
                "First step should address testing: {}",
                steps[0].description
            );
        }
    }

    #[test]
    fn test_effort_estimation_reasonable() {
        let metrics = create_test_metrics(15, 20);
        let rec = generate_testing_gap_steps(0.5, 15, 20, &metrics, FunctionRole::PureLogic, &None);

        if let Some(effort) = rec.estimated_effort_hours {
            assert!(effort > 0.0);
            assert!(effort < 10.0, "Effort seems too high: {}", effort);
        }
    }

    #[test]
    fn test_difficulty_matches_complexity() {
        // Simple case: Easy difficulty
        let simple_difficulty = Difficulty::for_testing(2, 5);
        assert_eq!(simple_difficulty, Difficulty::Easy);

        // Complex case: Hard difficulty
        let hard_difficulty = Difficulty::for_testing(15, 40);
        assert_eq!(hard_difficulty, Difficulty::Hard);
    }

    #[test]
    fn test_estimate_effort() {
        // Simple case: few tests, low complexity
        let effort1 = estimate_effort(10, 3);
        assert!(effort1 >= 0.5 && effort1 <= 2.0);

        // Complex case: many tests, high complexity
        let effort2 = estimate_effort(30, 10);
        assert!(effort2 > 2.0);
    }

    #[test]
    fn test_calculate_functions_to_extract() {
        assert_eq!(calculate_functions_to_extract(12, 15), 1);
        assert_eq!(calculate_functions_to_extract(18, 25), 2);
        assert_eq!(calculate_functions_to_extract(25, 35), 3);
        assert_eq!(calculate_functions_to_extract(35, 45), 4);
    }
}
