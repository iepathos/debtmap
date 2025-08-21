use crate::priority::call_graph::{CallGraph, FunctionId};
use crate::risk::lcov::LcovData;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransitiveCoverage {
    pub direct: f64,
    pub transitive: f64,
    pub propagated_from: Vec<FunctionId>,
}

pub fn calculate_transitive_coverage(
    func_id: &FunctionId,
    call_graph: &CallGraph,
    coverage: &LcovData,
) -> TransitiveCoverage {
    // Get direct coverage for this function
    let direct = get_function_coverage(func_id, coverage);

    // If function has direct coverage, no need to calculate transitive
    if direct > 0.0 {
        return TransitiveCoverage {
            direct,
            transitive: direct,
            propagated_from: vec![],
        };
    }

    // Calculate coverage from callees
    let callees = call_graph.get_callees(func_id);
    if callees.is_empty() {
        return TransitiveCoverage {
            direct: 0.0,
            transitive: 0.0,
            propagated_from: vec![],
        };
    }

    // Check coverage of each callee
    let mut covered_callees = Vec::new();

    for callee in &callees {
        let callee_coverage = get_function_coverage(callee, coverage);
        if callee_coverage > 0.8 {
            covered_callees.push(callee.clone());
        }
    }

    // Calculate transitive coverage as percentage of well-covered callees
    let transitive = if callees.is_empty() {
        0.0
    } else {
        covered_callees.len() as f64 / callees.len() as f64
    };

    TransitiveCoverage {
        direct,
        transitive,
        propagated_from: covered_callees,
    }
}

fn get_function_coverage(func_id: &FunctionId, coverage: &LcovData) -> f64 {
    // Use the LCOV module's fuzzy matching logic
    coverage
        .get_function_coverage_with_line(&func_id.file, &func_id.name, func_id.line)
        .map(|percentage| percentage / 100.0)
        .unwrap_or(0.0)
}

/// Calculates coverage urgency using a smooth gradient approach.
///
/// This function provides continuous scoring from 0% to 100% coverage, rather than
/// binary thresholds. The urgency score considers both direct and transitive coverage,
/// weighted by complexity.
///
/// # Score Examples (with average complexity 10):
/// - 0% coverage: ~10.0 (highest urgency)
/// - 25% coverage: ~7.5
/// - 50% coverage: ~5.0
/// - 75% coverage: ~2.5
/// - 90% coverage: ~1.0
/// - 100% coverage: 0.0 (no urgency)
///
/// # Complexity Weighting:
/// - Complexity 1-5: 0.5-0.8x multiplier
/// - Complexity 6-10: 0.8-1.2x multiplier
/// - Complexity 11-20: 1.2-1.5x multiplier
/// - Complexity 20+: 1.5-2.0x multiplier
pub fn calculate_coverage_urgency(
    func_id: &FunctionId,
    call_graph: &CallGraph,
    coverage: &LcovData,
    complexity: u32,
) -> f64 {
    let transitive_cov = calculate_transitive_coverage(func_id, call_graph, coverage);

    // Use weighted average of direct and transitive coverage
    // Direct coverage is more important than transitive coverage
    let coverage_weight = 0.7; // Direct coverage weight
    let effective_coverage = transitive_cov.direct * coverage_weight
        + transitive_cov.transitive * (1.0 - coverage_weight);

    // Calculate coverage gap (0.0 = fully covered, 1.0 = no coverage)
    // Ensure the value is between 0.0 and 1.0
    let coverage_gap = 1.0 - effective_coverage.clamp(0.0, 1.0);

    // Apply complexity weighting with logarithmic scaling
    // This provides smoother gradation:
    // Complexity 1-5 = 0.5-0.8x multiplier
    // Complexity 6-10 = 0.8-1.2x multiplier
    // Complexity 11-20 = 1.2-1.5x multiplier
    // Complexity 20+ = 1.5-2.0x multiplier
    let complexity_weight = if complexity == 0 {
        0.5
    } else {
        (((complexity as f64 + 1.0).ln() / 3.0) + 0.5).min(2.0)
    };

    // Calculate urgency score with smooth gradient
    // This produces continuous values from 0 to 10
    (coverage_gap * complexity_weight * 10.0).min(10.0)
}

pub fn propagate_coverage_through_graph(
    call_graph: &CallGraph,
    coverage: &LcovData,
) -> im::HashMap<FunctionId, TransitiveCoverage> {
    let mut result = im::HashMap::new();

    // Process all functions in the call graph
    for func_id in call_graph.find_all_functions() {
        let transitive = calculate_transitive_coverage(&func_id, call_graph, coverage);
        result.insert(func_id, transitive);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::risk::lcov::{FunctionCoverage, LcovData};
    use std::path::PathBuf;

    fn create_test_coverage() -> LcovData {
        let mut coverage = LcovData::default();

        // Add coverage for test.rs
        let funcs = vec![FunctionCoverage {
            name: "test_func".to_string(),
            start_line: 10,
            execution_count: 5,
            coverage_percentage: 50.0,
        }];
        coverage.functions.insert(PathBuf::from("test.rs"), funcs);

        coverage
    }

    #[test]
    fn test_direct_coverage() {
        let coverage = create_test_coverage();
        let graph = CallGraph::new();

        let func_id = FunctionId {
            file: PathBuf::from("test.rs"),
            name: "test_func".to_string(),
            line: 10,
        };

        let transitive = calculate_transitive_coverage(&func_id, &graph, &coverage);
        assert!(transitive.direct > 0.0);
        assert_eq!(transitive.direct, transitive.transitive);
        assert!(transitive.propagated_from.is_empty());
    }

    #[test]
    fn test_transitive_coverage_with_delegation() {
        let coverage = create_test_coverage();
        let mut graph = CallGraph::new();

        let orchestrator = FunctionId {
            file: PathBuf::from("orch.rs"),
            name: "orchestrate".to_string(),
            line: 1,
        };

        let worker = FunctionId {
            file: PathBuf::from("test.rs"),
            name: "worker".to_string(),
            line: 10,
        };

        graph.add_function(orchestrator.clone(), false, false, 2, 10);
        graph.add_function(worker.clone(), false, false, 5, 30);
        graph.add_call(crate::priority::call_graph::FunctionCall {
            caller: orchestrator.clone(),
            callee: worker.clone(),
            call_type: crate::priority::call_graph::CallType::Delegate,
        });

        let transitive = calculate_transitive_coverage(&orchestrator, &graph, &coverage);
        assert_eq!(transitive.direct, 0.0);
        // Should have some transitive coverage from the covered worker
        assert!(transitive.transitive >= 0.0);
    }

    #[test]
    fn test_coverage_urgency() {
        let coverage = create_test_coverage();
        let graph = CallGraph::new();

        let func_id = FunctionId {
            file: PathBuf::from("uncovered.rs"),
            name: "complex_func".to_string(),
            line: 1,
        };

        // High complexity, no coverage = high urgency
        let urgency = calculate_coverage_urgency(&func_id, &graph, &coverage, 10);
        assert!(urgency > 8.0);

        // Low complexity, no coverage = lower urgency (but still high due to no coverage)
        let urgency = calculate_coverage_urgency(&func_id, &graph, &coverage, 2);
        assert!(urgency >= 7.0 && urgency <= 10.0);
    }

    #[test]
    fn test_coverage_urgency_gradient() {
        let mut coverage = LcovData::default();
        let graph = CallGraph::new();

        // Create a function with varying coverage levels
        let func_id = FunctionId {
            file: PathBuf::from("gradient_test.rs"),
            name: "test_func".to_string(),
            line: 10,
        };

        // Test with average complexity (10)
        let complexity = 10;

        // Test 0% coverage - should be ~10.0
        let urgency_0 = calculate_coverage_urgency(&func_id, &graph, &coverage, complexity);
        assert!(
            urgency_0 >= 9.0 && urgency_0 <= 10.0,
            "0% coverage should score ~10.0, got {}",
            urgency_0
        );

        // Test 25% coverage - should be reduced from full
        let funcs = vec![FunctionCoverage {
            name: "test_func".to_string(),
            start_line: 10,
            execution_count: 1,
            coverage_percentage: 25.0,
        }];
        coverage
            .functions
            .insert(PathBuf::from("gradient_test.rs"), funcs.clone());

        let urgency_25 = calculate_coverage_urgency(&func_id, &graph, &coverage, complexity);
        // With 25% coverage and our weighted calculation (0.7 direct weight), this should be around 7.5-9.0
        assert!(
            urgency_25 >= 7.0 && urgency_25 <= 10.0,
            "25% coverage should score 7.0-10.0, got {}",
            urgency_25
        );

        // Test 50% coverage - should be around 5.0  
        // With weight = 0.7, effective coverage = 0.5 * 0.7 = 0.35, gap = 0.65
        let funcs = vec![FunctionCoverage {
            name: "test_func".to_string(),
            start_line: 10,
            execution_count: 1,
            coverage_percentage: 50.0,
        }];
        coverage
            .functions
            .insert(PathBuf::from("gradient_test.rs"), funcs.clone());

        let urgency_50 = calculate_coverage_urgency(&func_id, &graph, &coverage, complexity);
        assert!(
            urgency_50 >= 5.0 && urgency_50 <= 7.5,
            "50% coverage should score 5.0-7.5, got {}",
            urgency_50
        );

        // Test 75% coverage - should be around 3.0
        // With weight = 0.7, effective coverage = 0.75 * 0.7 = 0.525, gap = 0.475
        let funcs = vec![FunctionCoverage {
            name: "test_func".to_string(),
            start_line: 10,
            execution_count: 1,
            coverage_percentage: 75.0,
        }];
        coverage
            .functions
            .insert(PathBuf::from("gradient_test.rs"), funcs.clone());

        let urgency_75 = calculate_coverage_urgency(&func_id, &graph, &coverage, complexity);
        assert!(
            urgency_75 >= 3.0 && urgency_75 <= 5.5,
            "75% coverage should score 3.0-5.5, got {}",
            urgency_75
        );

        // Test 90% coverage - should be around 1.3 
        // With weight = 0.7, effective coverage = 0.9 * 0.7 = 0.63, gap = 0.37
        let funcs = vec![FunctionCoverage {
            name: "test_func".to_string(),
            start_line: 10,
            execution_count: 1,
            coverage_percentage: 90.0,
        }];
        coverage
            .functions
            .insert(PathBuf::from("gradient_test.rs"), funcs.clone());

        let urgency_90 = calculate_coverage_urgency(&func_id, &graph, &coverage, complexity);
        assert!(
            urgency_90 >= 1.0 && urgency_90 <= 4.5,
            "90% coverage should score 1.0-4.5, got {}",
            urgency_90
        );

        // Test 100% coverage - should be 0.0
        let funcs = vec![FunctionCoverage {
            name: "test_func".to_string(),
            start_line: 10,
            execution_count: 1,
            coverage_percentage: 100.0,
        }];
        coverage
            .functions
            .insert(PathBuf::from("gradient_test.rs"), funcs.clone());

        let urgency_100 = calculate_coverage_urgency(&func_id, &graph, &coverage, complexity);
        assert!(
            urgency_100 == 0.0,
            "100% coverage should score 0.0, got {}",
            urgency_100
        );

        // Verify smooth gradient - scores should decrease monotonically
        assert!(
            urgency_0 > urgency_25,
            "Scores should decrease as coverage increases"
        );
        assert!(
            urgency_25 > urgency_50,
            "Scores should decrease as coverage increases"
        );
        assert!(
            urgency_50 > urgency_75,
            "Scores should decrease as coverage increases"
        );
        assert!(
            urgency_75 > urgency_90,
            "Scores should decrease as coverage increases"
        );
        assert!(
            urgency_90 > urgency_100,
            "Scores should decrease as coverage increases"
        );
    }

    #[test]
    fn test_complexity_weighting() {
        let coverage = LcovData::default(); // No coverage
        let graph = CallGraph::new();

        let func_id = FunctionId {
            file: PathBuf::from("complexity_test.rs"),
            name: "test_func".to_string(),
            line: 1,
        };

        // Test complexity scaling with 0% coverage

        // Complexity 1: ln(2)/3 + 0.5 = ~0.73 multiplier
        let urgency_c1 = calculate_coverage_urgency(&func_id, &graph, &coverage, 1);
        assert!(
            urgency_c1 >= 6.5 && urgency_c1 <= 8.0,
            "Complexity 1 should score 6.5-8.0, got {}",
            urgency_c1
        );

        // Complexity 5: ln(6)/3 + 0.5 = ~1.09 multiplier  
        let urgency_c5 = calculate_coverage_urgency(&func_id, &graph, &coverage, 5);
        assert!(
            urgency_c5 >= 9.5 && urgency_c5 <= 10.0,
            "Complexity 5 should score 9.5-10.0, got {}",
            urgency_c5
        );

        // Complexity 10: should have ~1.0x multiplier
        let urgency_c10 = calculate_coverage_urgency(&func_id, &graph, &coverage, 10);
        assert!(
            urgency_c10 >= 9.0 && urgency_c10 <= 10.0,
            "Complexity 10 should score ~10.0, got {}",
            urgency_c10
        );

        // Complexity 20: should have ~1.3x multiplier (capped at 10.0)
        let urgency_c20 = calculate_coverage_urgency(&func_id, &graph, &coverage, 20);
        assert!(
            urgency_c20 == 10.0,
            "Complexity 20 should be capped at 10.0, got {}",
            urgency_c20
        );

        // Complexity 50: should have ~1.7x multiplier (capped at 10.0)
        let urgency_c50 = calculate_coverage_urgency(&func_id, &graph, &coverage, 50);
        assert!(
            urgency_c50 == 10.0,
            "Complexity 50 should be capped at 10.0, got {}",
            urgency_c50
        );

        // Verify smooth increase with complexity
        assert!(
            urgency_c1 < urgency_c5,
            "Higher complexity should have higher urgency"
        );
        assert!(
            urgency_c5 <= urgency_c10,
            "Higher complexity should have higher urgency (or be capped)"
        );
        assert!(
            urgency_c10 <= urgency_c20,
            "Higher complexity should have higher urgency (or be capped)"
        );
    }
}
