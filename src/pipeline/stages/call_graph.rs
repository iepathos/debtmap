//! Pure functions for call graph construction and analysis.
//!
//! These functions build and analyze call graphs from function metrics
//! without performing any I/O operations.

use crate::core::FunctionMetrics;
use crate::priority::call_graph::{CallGraph, FunctionId};
use std::collections::HashSet;
use std::path::Path;

/// Build initial call graph from function metrics (pure).
///
/// Creates a call graph with all functions as nodes, setting their basic
/// properties like complexity, length, and role (entry point, test).
///
/// # Arguments
///
/// * `metrics` - Slice of function metrics to build graph from
///
/// # Returns
///
/// Call graph with all functions added as nodes
///
/// # Examples
///
/// ```ignore
/// let metrics = vec![
///     FunctionMetrics { name: "foo".into(), file: "main.rs".into(), line: 10, ... },
///     FunctionMetrics { name: "bar".into(), file: "main.rs".into(), line: 20, ... },
/// ];
/// let graph = build_call_graph(&metrics);
/// assert_eq!(graph.node_count(), 2);
/// ```
pub fn build_call_graph(metrics: &[FunctionMetrics]) -> CallGraph {
    let mut call_graph = CallGraph::new();

    for metric in metrics {
        let func_id = FunctionId::new(metric.file.clone(), metric.name.clone(), metric.line);

        call_graph.add_function(
            func_id,
            is_entry_point(&metric.name),
            is_test_function(&metric.name, &metric.file, metric.is_test),
            metric.cyclomatic,
            metric.length,
        );
    }

    call_graph
}

/// Determine if a function is an entry point (pure).
///
/// Entry points are functions like `main`, handlers, or runners that
/// serve as application entry points.
///
/// # Arguments
///
/// * `function_name` - Name of the function to check
///
/// # Returns
///
/// `true` if function is an entry point, `false` otherwise
fn is_entry_point(function_name: &str) -> bool {
    matches!(function_name, "main")
        || function_name.starts_with("handle_")
        || function_name.starts_with("run_")
}

/// Determine if a function is a test function (pure).
///
/// Tests are identified by the test attribute, naming convention,
/// or location in test modules/directories.
///
/// # Arguments
///
/// * `function_name` - Name of the function
/// * `file_path` - Path to the file containing the function
/// * `is_test_attr` - Whether function has test attribute
///
/// # Returns
///
/// `true` if function is a test, `false` otherwise
fn is_test_function(function_name: &str, file_path: &Path, is_test_attr: bool) -> bool {
    is_test_attr
        || function_name.starts_with("test_")
        || file_path.to_string_lossy().contains("test")
}

/// Find functions that are only reachable from test roots (pure).
///
/// Identifies test functions and test helper functions using the
/// call graph's test analysis capabilities.
///
/// # Arguments
///
/// * `graph` - Call graph to analyze
/// * `_test_roots` - Set of known test function IDs (for future use)
///
/// # Returns
///
/// Set of function IDs that are tests or test helpers
pub fn find_test_only_functions(
    graph: &CallGraph,
    _test_roots: &HashSet<FunctionId>,
) -> HashSet<FunctionId> {
    // Use call graph's built-in test function detection
    graph.find_test_functions().into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn test_metric(name: &str, file: &str, line: usize) -> FunctionMetrics {
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
            purity_reason: None,
            call_dependencies: None,
            detected_patterns: None,
            upstream_callers: None,
            downstream_callees: None,
            mapping_pattern_result: None,
            adjusted_complexity: None,
            composition_metrics: None,
            language_specific: None,
            purity_level: None,
            error_swallowing_count: None,
            error_swallowing_patterns: None,
            entropy_analysis: None,
        }
    }

    #[test]
    fn test_build_call_graph_empty() {
        let graph = build_call_graph(&[]);
        assert_eq!(graph.node_count(), 0);
    }

    #[test]
    fn test_build_call_graph_single_function() {
        let metrics = vec![test_metric("foo", "main.rs", 10)];
        let graph = build_call_graph(&metrics);
        assert_eq!(graph.node_count(), 1);
    }

    #[test]
    fn test_build_call_graph_multiple_functions() {
        let metrics = vec![
            test_metric("foo", "main.rs", 10),
            test_metric("bar", "main.rs", 20),
            test_metric("baz", "lib.rs", 5),
        ];
        let graph = build_call_graph(&metrics);
        assert_eq!(graph.node_count(), 3);
    }

    #[test]
    fn test_is_entry_point_main() {
        assert!(is_entry_point("main"));
    }

    #[test]
    fn test_is_entry_point_handler() {
        assert!(is_entry_point("handle_request"));
        assert!(is_entry_point("handle_event"));
    }

    #[test]
    fn test_is_entry_point_runner() {
        assert!(is_entry_point("run_server"));
        assert!(is_entry_point("run_tests"));
    }

    #[test]
    fn test_is_entry_point_regular_function() {
        assert!(!is_entry_point("calculate"));
        assert!(!is_entry_point("process_data"));
    }

    #[test]
    fn test_is_test_function_with_attr() {
        assert!(is_test_function("foo", Path::new("main.rs"), true));
    }

    #[test]
    fn test_is_test_function_with_prefix() {
        assert!(is_test_function(
            "test_calculation",
            Path::new("main.rs"),
            false
        ));
    }

    #[test]
    fn test_is_test_function_in_test_file() {
        assert!(is_test_function(
            "foo",
            Path::new("tests/integration.rs"),
            false
        ));
        assert!(is_test_function(
            "bar",
            Path::new("src/test_utils.rs"),
            false
        ));
    }

    #[test]
    fn test_is_test_function_regular() {
        assert!(!is_test_function(
            "calculate",
            Path::new("src/main.rs"),
            false
        ));
    }

    #[test]
    fn test_find_test_only_functions_empty() {
        let graph = CallGraph::new();
        let test_roots = HashSet::new();
        let result = find_test_only_functions(&graph, &test_roots);
        assert!(result.is_empty());
    }
}
