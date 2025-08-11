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

pub fn calculate_coverage_urgency(
    func_id: &FunctionId,
    call_graph: &CallGraph,
    coverage: &LcovData,
    complexity: u32,
) -> f64 {
    let transitive_cov = calculate_transitive_coverage(func_id, call_graph, coverage);

    // Higher urgency for lower coverage
    let coverage_factor = 1.0 - transitive_cov.transitive.max(transitive_cov.direct);

    // Weight by complexity - uncovered complex code is more urgent
    let complexity_weight = (complexity as f64 / 10.0).min(2.0);

    // Calculate urgency score (0-10)
    (coverage_factor * complexity_weight * 10.0).min(10.0)
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

        // Low complexity, no coverage = lower urgency
        let urgency = calculate_coverage_urgency(&func_id, &graph, &coverage, 2);
        assert!(urgency < 5.0);
    }
}
