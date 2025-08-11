use crate::core::FunctionMetrics;
use crate::priority::{
    call_graph::{CallGraph, FunctionId},
    coverage_propagation::{
        calculate_coverage_urgency, calculate_transitive_coverage, TransitiveCoverage,
    },
    semantic_classifier::{
        calculate_semantic_priority, classify_function_role, get_role_multiplier, FunctionRole,
    },
    ActionableRecommendation, DebtType, ImpactMetrics,
};
use crate::risk::lcov::LcovData;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedScore {
    pub complexity_factor: f64, // 0-10, weighted 25%
    pub coverage_factor: f64,   // 0-10, weighted 35%
    pub roi_factor: f64,        // 0-10, weighted 25%
    pub semantic_factor: f64,   // 0-10, weighted 15%
    pub role_multiplier: f64,   // 0.1-1.5x based on function role
    pub final_score: f64,       // Computed composite score
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedDebtItem {
    pub location: Location,
    pub debt_type: DebtType,
    pub unified_score: UnifiedScore,
    pub function_role: FunctionRole,
    pub recommendation: ActionableRecommendation,
    pub expected_impact: ImpactMetrics,
    pub transitive_coverage: Option<TransitiveCoverage>,
    pub upstream_dependencies: usize,
    pub downstream_dependencies: usize,
    pub nesting_depth: u32,
    pub function_length: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Location {
    pub file: PathBuf,
    pub function: String,
    pub line: usize,
}

pub fn calculate_unified_priority(
    func: &FunctionMetrics,
    call_graph: &CallGraph,
    coverage: Option<&LcovData>,
    roi_score: f64,
) -> UnifiedScore {
    let func_id = FunctionId {
        file: func.file.clone(),
        name: func.name.clone(),
        line: func.line,
    };

    // Calculate complexity factor (normalized to 0-10)
    let complexity_factor = normalize_complexity(func.cyclomatic, func.cognitive);

    // Calculate coverage factor (0-10, higher means more urgent to cover)
    let coverage_factor = if func.is_test {
        // Test functions don't need coverage - they are the coverage mechanism
        0.0
    } else if let Some(cov) = coverage {
        calculate_coverage_urgency(&func_id, call_graph, cov, func.cyclomatic)
    } else {
        // No coverage data - assume worst case
        10.0
    };

    // Calculate ROI factor (normalized to 0-10)
    let roi_factor = normalize_roi(roi_score);

    // Classify function role and calculate semantic priority
    let role = classify_function_role(func, &func_id, call_graph);
    let semantic_factor = calculate_semantic_priority(func, role, &func_id, call_graph);
    let role_multiplier = get_role_multiplier(role);

    // Calculate weighted composite score
    let base_score = complexity_factor * 0.25
        + coverage_factor * 0.35
        + roi_factor * 0.25
        + semantic_factor * 0.15;

    // Apply role multiplier
    let final_score = (base_score * role_multiplier).min(10.0);

    UnifiedScore {
        complexity_factor,
        coverage_factor,
        roi_factor,
        semantic_factor,
        role_multiplier,
        final_score,
    }
}

fn normalize_complexity(cyclomatic: u32, cognitive: u32) -> f64 {
    // Normalize complexity to 0-10 scale
    let combined = (cyclomatic + cognitive) as f64 / 2.0;

    // Use logarithmic scale for better distribution
    // Complexity of 1-5 = low (0-3), 6-10 = medium (3-6), 11+ = high (6-10)
    if combined <= 5.0 {
        combined * 0.6
    } else if combined <= 10.0 {
        3.0 + (combined - 5.0) * 0.6
    } else {
        6.0 + ((combined - 10.0) * 0.2).min(4.0)
    }
}

fn normalize_roi(roi: f64) -> f64 {
    // ROI typically ranges from 0.1 to 10.0
    // Normalize to 0-10 scale with logarithmic transformation
    if roi <= 0.0 {
        0.0
    } else if roi <= 1.0 {
        roi * 3.0
    } else if roi <= 5.0 {
        3.0 + (roi - 1.0) * 1.0
    } else {
        7.0 + ((roi - 5.0) * 0.6).min(3.0)
    }
}

pub fn create_unified_debt_item(
    func: &FunctionMetrics,
    call_graph: &CallGraph,
    coverage: Option<&LcovData>,
    roi_score: f64,
) -> UnifiedDebtItem {
    let func_id = FunctionId {
        file: func.file.clone(),
        name: func.name.clone(),
        line: func.line,
    };

    let unified_score = calculate_unified_priority(func, call_graph, coverage, roi_score);
    let role = classify_function_role(func, &func_id, call_graph);

    let transitive_coverage =
        coverage.map(|cov| calculate_transitive_coverage(&func_id, call_graph, cov));

    let debt_type = determine_debt_type(func, &transitive_coverage);
    let recommendation = generate_recommendation(func, &debt_type, role, &unified_score);
    let expected_impact = calculate_expected_impact(func, &debt_type, &unified_score);

    // Get dependency counts from call graph
    let upstream_dependencies = call_graph.get_callers(&func_id).len();
    let downstream_dependencies = call_graph.get_callees(&func_id).len();

    UnifiedDebtItem {
        location: Location {
            file: func.file.clone(),
            function: func.name.clone(),
            line: func.line,
        },
        debt_type,
        unified_score,
        function_role: role,
        recommendation,
        expected_impact,
        transitive_coverage,
        upstream_dependencies,
        downstream_dependencies,
        nesting_depth: func.nesting,
        function_length: func.length,
    }
}

fn determine_debt_type(func: &FunctionMetrics, coverage: &Option<TransitiveCoverage>) -> DebtType {
    // Determine primary debt type based on metrics
    if let Some(cov) = coverage {
        if cov.direct < 0.2 && func.cyclomatic > 3 {
            return DebtType::TestingGap {
                coverage: cov.direct,
                cyclomatic: func.cyclomatic,
                cognitive: func.cognitive,
            };
        }
    }

    if func.cyclomatic > 10 || func.cognitive > 15 {
        return DebtType::ComplexityHotspot {
            cyclomatic: func.cyclomatic,
            cognitive: func.cognitive,
        };
    }

    // Default to risk-based debt
    DebtType::Risk {
        risk_score: 5.0,
        factors: vec!["General technical debt".to_string()],
    }
}

fn generate_recommendation(
    _func: &FunctionMetrics,
    debt_type: &DebtType,
    role: FunctionRole,
    _score: &UnifiedScore,
) -> ActionableRecommendation {
    let (primary_action, rationale, steps) = match debt_type {
        DebtType::TestingGap {
            coverage,
            cyclomatic,
            cognitive,
        } => {
            // Consider both cyclomatic and cognitive complexity
            // A function is complex if either metric exceeds its threshold
            let is_complex = *cyclomatic > 10 || *cognitive > 15;
            if is_complex {
                // High complexity: recommend functional refactoring approach
                (
                    format!(
                        "Extract pure functions, add property tests, then refactor (cyclo={cyclomatic} to <10, cog={cognitive} to <15)"
                    ),
                    {
                        let role_str = match role {
                            FunctionRole::PureLogic => "business logic",
                            FunctionRole::Orchestrator => "orchestration",
                            FunctionRole::IOWrapper => "I/O wrapper",
                            FunctionRole::EntryPoint => "entry point",
                            FunctionRole::Unknown => "function",
                        };
                        let coverage_pct = (coverage * 100.0) as i32;
                        format!(
                            "Complex {role_str} (cyclo={cyclomatic}, cog={cognitive}) with {coverage_pct}% coverage - extract pure logic first"
                        )
                    },
                    vec![
                        "Identify and extract pure functions (no side effects)".to_string(),
                        "Add property-based tests for pure logic".to_string(),
                        "Replace conditionals with pattern matching where possible".to_string(),
                        "Convert loops to map/filter/fold operations".to_string(),
                        "Push I/O to the boundaries".to_string(),
                    ],
                )
            } else {
                // Simple function: just add tests
                (
                    format!("Add {} unit tests for full coverage", cyclomatic.max(&2)),
                    format!(
                        "{} with {}% coverage, manageable complexity (cyclo={}, cog={})",
                        match role {
                            FunctionRole::PureLogic => "Business logic",
                            FunctionRole::Orchestrator => "Orchestration",
                            FunctionRole::IOWrapper => "I/O wrapper",
                            FunctionRole::EntryPoint => "Entry point",
                            FunctionRole::Unknown => "Function",
                        },
                        (coverage * 100.0) as i32,
                        cyclomatic,
                        cognitive
                    ),
                    vec![
                        "Test happy path scenarios".to_string(),
                        "Add edge case tests".to_string(),
                        "Cover error conditions".to_string(),
                    ],
                )
            }
        }
        DebtType::ComplexityHotspot {
            cyclomatic,
            cognitive,
        } => (
            format!(
                "Extract {} sub-functions to reduce complexity",
                cyclomatic / 5 + 1
            ),
            format!(
                "Highest complexity function (CC:{cyclomatic}, Cog:{cognitive}), affects all dependent calculations"
            ),
            vec![
                "Identify logical groups in the function".to_string(),
                "Extract each group into a named function".to_string(),
                "Add unit tests for extracted functions".to_string(),
            ],
        ),
        DebtType::Orchestration { delegates_to } => (
            "Consider integration test instead of unit tests".to_string(),
            format!(
                "Orchestration function delegating to {} tested functions",
                delegates_to.len()
            ),
            vec![
                "Write integration test covering the flow".to_string(),
                "Verify delegation behavior".to_string(),
            ],
        ),
        DebtType::Duplication {
            instances,
            total_lines,
        } => (
            "Extract common logic into shared module".to_string(),
            format!(
                "Duplicated across {instances} locations ({total_lines} lines total)"
            ),
            vec![
                "Create shared utility module".to_string(),
                "Replace duplicated code with calls to shared module".to_string(),
                "Add comprehensive tests to shared module".to_string(),
            ],
        ),
        DebtType::Risk {
            risk_score,
            factors,
        } => (
            "Address technical debt".to_string(),
            format!("Risk score {:.1}: {}", risk_score, factors.join(", ")),
            vec![
                "Review and refactor problematic areas".to_string(),
                "Add missing tests".to_string(),
                "Update documentation".to_string(),
            ],
        ),
        // Test-specific debt types
        DebtType::TestComplexityHotspot {
            cyclomatic,
            cognitive,
            threshold
        } => (
            format!("Simplify test - complexity {} exceeds test threshold {}", cyclomatic.max(cognitive), threshold),
            format!("Test has high complexity (cyclo={cyclomatic}, cognitive={cognitive}) - consider splitting into smaller tests"),
            vec![
                "Break complex test into multiple smaller tests".to_string(),
                "Extract test setup into helper functions".to_string(),
                "Use parameterized tests for similar test cases".to_string(),
            ],
        ),
        DebtType::TestTodo { priority: _, reason } => (
            "Complete test TODO".to_string(),
            format!("Test contains TODO: {}", reason.as_ref().unwrap_or(&"No reason specified".to_string())),
            vec![
                "Address the TODO comment".to_string(),
                "Implement missing test logic".to_string(),
                "Remove TODO once completed".to_string(),
            ],
        ),
        DebtType::TestDuplication { instances, total_lines, similarity: _ } => (
            format!("Remove test duplication - {instances} similar test blocks"),
            format!("{instances} duplicated test blocks found across {total_lines} lines"),
            vec![
                "Extract common test logic into helper functions".to_string(),
                "Create parameterized tests for similar test cases".to_string(),
                "Use test fixtures for shared setup".to_string(),
            ],
        ),
    };

    ActionableRecommendation {
        primary_action,
        rationale,
        implementation_steps: steps,
        related_items: vec![],
    }
}

fn calculate_expected_impact(
    _func: &FunctionMetrics,
    debt_type: &DebtType,
    score: &UnifiedScore,
) -> ImpactMetrics {
    match debt_type {
        DebtType::TestingGap {
            coverage,
            cyclomatic,
            cognitive,
        } => {
            // For high complexity functions, the impact includes both testing and refactoring benefits
            // Consider both cyclomatic and cognitive complexity
            // A function is complex if either metric exceeds its threshold
            let is_complex = *cyclomatic > 10 || *cognitive > 15;

            ImpactMetrics {
                // Show the actual coverage gain for this function/module
                // High complexity functions get less coverage benefit (need refactoring first)
                coverage_improvement: if is_complex {
                    (1.0 - coverage) * 50.0 // 50% of potential due to complexity
                } else {
                    (1.0 - coverage) * 100.0 // Full coverage potential for simple functions
                },
                lines_reduction: 0,
                complexity_reduction: if is_complex {
                    *cyclomatic as f64 * 0.3
                } else {
                    0.0
                },
                risk_reduction: score.final_score * 0.42,
            }
        }
        DebtType::ComplexityHotspot {
            cyclomatic,
            cognitive: _,
        } => ImpactMetrics {
            coverage_improvement: 0.0,
            lines_reduction: 0,
            complexity_reduction: (*cyclomatic as f64 * 0.5),
            risk_reduction: score.final_score * 0.35,
        },
        DebtType::Duplication {
            instances,
            total_lines,
        } => ImpactMetrics {
            coverage_improvement: 0.0,
            lines_reduction: *total_lines - (*total_lines / instances),
            complexity_reduction: 0.0,
            risk_reduction: score.final_score * 0.25,
        },
        DebtType::Orchestration { .. } => ImpactMetrics {
            coverage_improvement: 0.0,
            lines_reduction: 0,
            complexity_reduction: 0.0,
            risk_reduction: score.final_score * 0.1, // Low priority for orchestration
        },
        DebtType::Risk { .. } => ImpactMetrics {
            coverage_improvement: 0.0,
            lines_reduction: 0,
            complexity_reduction: 0.0,
            risk_reduction: score.final_score * 0.2,
        },
        // Test-specific debt types have lower impact on overall metrics
        DebtType::TestComplexityHotspot {
            cyclomatic,
            cognitive: _,
            threshold: _,
        } => ImpactMetrics {
            coverage_improvement: 0.0, // Tests don't improve coverage
            lines_reduction: 0,
            complexity_reduction: (*cyclomatic as f64 * 0.3),
            risk_reduction: score.final_score * 0.15, // Lower risk impact for tests
        },
        DebtType::TestTodo { .. } => ImpactMetrics {
            coverage_improvement: 0.0,
            lines_reduction: 0,
            complexity_reduction: 0.0,
            risk_reduction: score.final_score * 0.1,
        },
        DebtType::TestDuplication {
            instances,
            total_lines,
            similarity: _,
        } => ImpactMetrics {
            coverage_improvement: 0.0,
            lines_reduction: *total_lines - (*total_lines / instances),
            complexity_reduction: 0.0,
            risk_reduction: score.final_score * 0.1,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_metrics() -> FunctionMetrics {
        FunctionMetrics {
            file: PathBuf::from("test.rs"),
            name: "test_function".to_string(),
            line: 10,
            length: 50,
            cyclomatic: 5,
            cognitive: 8,
            nesting: 0,
            is_test: false,
        }
    }

    #[test]
    fn test_normalize_complexity() {
        assert!(normalize_complexity(1, 1) < 2.0);
        assert!(normalize_complexity(5, 5) > 2.0);
        assert!(normalize_complexity(5, 5) < 6.0);
        assert!(normalize_complexity(10, 10) > 5.0);
        assert!(normalize_complexity(20, 20) <= 10.0);
    }

    #[test]
    fn test_normalize_roi() {
        assert_eq!(normalize_roi(0.0), 0.0);
        assert!(normalize_roi(0.5) < 3.0);
        assert!(normalize_roi(1.0) <= 3.0);
        assert!(normalize_roi(3.0) > 3.0);
        assert!(normalize_roi(3.0) < 7.0);
        assert!(normalize_roi(10.0) <= 10.0);
    }

    #[test]
    fn test_unified_scoring() {
        let func = create_test_metrics();
        let graph = CallGraph::new();
        let score = calculate_unified_priority(&func, &graph, None, 5.0);

        assert!(score.complexity_factor > 0.0);
        assert!(score.coverage_factor > 0.0);
        assert!(score.roi_factor > 0.0);
        assert!(score.semantic_factor > 0.0);
        assert!(score.final_score > 0.0);
        assert!(score.final_score <= 10.0);
    }

    #[test]
    fn test_debt_type_determination() {
        let func = create_test_metrics();
        let coverage = Some(TransitiveCoverage {
            direct: 0.1,
            transitive: 0.1,
            propagated_from: vec![],
        });

        let debt_type = determine_debt_type(&func, &coverage);
        match debt_type {
            DebtType::TestingGap { .. } => (),
            _ => panic!("Expected TestingGap debt type"),
        }
    }

    #[test]
    fn test_recommendation_generation() {
        let func = create_test_metrics();
        let debt_type = DebtType::ComplexityHotspot {
            cyclomatic: 15,
            cognitive: 20,
        };
        let score = UnifiedScore {
            complexity_factor: 8.0,
            coverage_factor: 7.0,
            roi_factor: 6.0,
            semantic_factor: 5.0,
            role_multiplier: 1.0,
            final_score: 6.5,
        };

        let rec = generate_recommendation(&func, &debt_type, FunctionRole::PureLogic, &score);
        assert!(rec.primary_action.contains("Extract"));
        assert!(rec.rationale.contains("complexity"));
        assert!(!rec.implementation_steps.is_empty());
    }
}
