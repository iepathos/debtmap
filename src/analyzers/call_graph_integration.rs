//! Call Graph Integration
//!
//! This module provides functionality to populate call graph data into FunctionMetrics.
//! It bridges the gap between call graph analysis and function metrics to ensure that
//! upstream_callers and downstream_callees fields are properly populated.

use crate::core::FunctionMetrics;
use crate::priority::call_graph::{CallGraph, FunctionId};
use std::collections::HashMap;

/// Populate call graph data into function metrics
///
/// This pure function takes a vector of FunctionMetrics and a CallGraph, then returns
/// a new vector of FunctionMetrics with the call graph fields populated.
pub fn populate_call_graph_data(
    mut function_metrics: Vec<FunctionMetrics>,
    call_graph: &CallGraph,
) -> Vec<FunctionMetrics> {
    // Create a mapping from function metrics to their function IDs for efficient lookup
    let metric_to_id_map: HashMap<usize, FunctionId> = function_metrics
        .iter()
        .enumerate()
        .map(|(idx, metric)| {
            let func_id = FunctionId {
                file: metric.file.clone(),
                name: metric.name.clone(),
                line: metric.line,
            };
            (idx, func_id)
        })
        .collect();

    // Populate call graph data for each function metric
    for (idx, metric) in function_metrics.iter_mut().enumerate() {
        if let Some(func_id) = metric_to_id_map.get(&idx) {
            // Get callers (upstream dependencies)
            let upstream_callers: Vec<String> = call_graph
                .get_callers(func_id)
                .into_iter()
                .map(|caller_id| format_function_name(&caller_id))
                .collect();

            // Get callees (downstream dependencies)
            let downstream_callees: Vec<String> = call_graph
                .get_callees(func_id)
                .into_iter()
                .map(|callee_id| format_function_name(&callee_id))
                .collect();

            // Update the function metrics with call graph data
            metric.upstream_callers = if upstream_callers.is_empty() {
                None
            } else {
                Some(upstream_callers)
            };

            metric.downstream_callees = if downstream_callees.is_empty() {
                None
            } else {
                Some(downstream_callees)
            };
        }
    }

    function_metrics
}

/// Format a function ID into a human-readable string
///
/// This pure function creates a consistent string representation of a function
/// that includes the file path (just the filename) and the function name.
fn format_function_name(func_id: &FunctionId) -> String {
    let file_name = func_id
        .file
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("unknown");

    format!("{}:{}", file_name, func_id.name)
}

/// Filter call graph data for Python functions only
///
/// This helper function filters the call graph to include only Python functions,
/// which is useful when we want to focus on Python-specific call relationships.
pub fn filter_python_call_graph(call_graph: &CallGraph) -> CallGraph {
    let mut filtered_graph = CallGraph::new();

    // Get all Python function nodes
    let python_functions: Vec<FunctionId> = call_graph
        .get_all_functions()
        .filter(|func_id| {
            func_id.file.extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext == "py")
                .unwrap_or(false)
        })
        .cloned()
        .collect();

    // Add Python functions to the filtered graph
    for func_id in &python_functions {
        if let Some((is_entry_point, is_test, complexity, length)) = call_graph.get_function_info(func_id) {
            filtered_graph.add_function(
                func_id.clone(),
                is_entry_point,
                is_test,
                complexity,
                length,
            );
        }
    }

    // Add call relationships between Python functions
    for func_id in &python_functions {
        let callees = call_graph.get_callees(func_id);
        for callee in callees {
            // Only add the call if the callee is also a Python function
            if python_functions.contains(&callee) {
                filtered_graph.add_call_parts(func_id.clone(), callee, crate::priority::call_graph::CallType::Direct);
            }
        }
    }

    filtered_graph
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn create_test_function_metric(name: &str, file: &str, line: usize) -> FunctionMetrics {
        FunctionMetrics {
            name: name.to_string(),
            file: PathBuf::from(file),
            line,
            cyclomatic: 1,
            cognitive: 1,
            nesting: 1,
            length: 10,
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
        }
    }

    #[test]
    fn test_populate_call_graph_data_empty() {
        let metrics = vec![];
        let call_graph = CallGraph::new();

        let result = populate_call_graph_data(metrics, &call_graph);
        assert!(result.is_empty());
    }

    #[test]
    fn test_populate_call_graph_data_single_function() {
        let mut metrics = vec![
            create_test_function_metric("test_func", "test.py", 10)
        ];
        let mut call_graph = CallGraph::new();

        let func_id = FunctionId {
            file: PathBuf::from("test.py"),
            name: "test_func".to_string(),
            line: 10,
        };

        call_graph.add_function(func_id.clone(), false, false, 1, 10);

        let result = populate_call_graph_data(metrics, &call_graph);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].upstream_callers, None);
        assert_eq!(result[0].downstream_callees, None);
    }

    #[test]
    fn test_populate_call_graph_data_with_calls() {
        let metrics = vec![
            create_test_function_metric("caller", "test.py", 5),
            create_test_function_metric("callee", "test.py", 15),
        ];

        let mut call_graph = CallGraph::new();

        let caller_id = FunctionId {
            file: PathBuf::from("test.py"),
            name: "caller".to_string(),
            line: 5,
        };

        let callee_id = FunctionId {
            file: PathBuf::from("test.py"),
            name: "callee".to_string(),
            line: 15,
        };

        call_graph.add_function(caller_id.clone(), false, false, 1, 10);
        call_graph.add_function(callee_id.clone(), false, false, 1, 10);
        call_graph.add_call(&caller_id, &callee_id);

        let result = populate_call_graph_data(metrics, &call_graph);

        assert_eq!(result.len(), 2);

        // Check caller function
        let caller_metric = &result[0];
        assert_eq!(caller_metric.name, "caller");
        assert_eq!(caller_metric.upstream_callers, None);
        assert_eq!(caller_metric.downstream_callees, Some(vec!["test.py:callee".to_string()]));

        // Check callee function
        let callee_metric = &result[1];
        assert_eq!(callee_metric.name, "callee");
        assert_eq!(callee_metric.upstream_callers, Some(vec!["test.py:caller".to_string()]));
        assert_eq!(callee_metric.downstream_callees, None);
    }

    #[test]
    fn test_format_function_name() {
        let func_id = FunctionId {
            file: PathBuf::from("/path/to/test.py"),
            name: "my_function".to_string(),
            line: 10,
        };

        let formatted = format_function_name(&func_id);
        assert_eq!(formatted, "test.py:my_function");
    }
}