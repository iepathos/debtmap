//! Pure call graph construction and enrichment.
//!
//! This module provides pure functions for building and enriching call graphs
//! without any I/O or progress reporting side effects.

use crate::analyzers::call_graph_integration;
use crate::core::FunctionMetrics;
use crate::priority::call_graph::{CallGraph, FunctionId};
use std::collections::HashSet;

/// Configuration for call graph computation.
#[derive(Debug, Clone, Default)]
pub struct CallGraphConfig {
    /// Whether to compute transitive closure.
    pub compute_transitive: bool,
    /// Whether to detect trait patterns.
    pub detect_trait_patterns: bool,
}

/// Result of call graph enrichment containing exclusions and function pointer info.
#[derive(Debug, Clone, Default)]
pub struct CallGraphEnrichmentResult {
    /// Functions excluded from priority scoring due to framework patterns.
    pub framework_exclusions: HashSet<FunctionId>,
    /// Functions used via function pointers.
    pub function_pointer_used_functions: HashSet<FunctionId>,
}

/// Build initial call graph from function metrics (pure).
///
/// This is a pure transformation that creates a call graph structure
/// from the complexity metrics without any I/O operations.
///
/// # Arguments
///
/// * `metrics` - The function metrics to build the call graph from
///
/// # Returns
///
/// A `CallGraph` populated with nodes from the metrics.
pub fn build_initial_call_graph(metrics: &[FunctionMetrics]) -> CallGraph {
    crate::builders::call_graph::build_initial_call_graph(metrics)
}

/// Enrich metrics with call graph data (pure).
///
/// Populates upstream_callers and downstream_callees fields in function metrics
/// based on call graph relationships.
///
/// # Arguments
///
/// * `metrics` - The function metrics to enrich
/// * `call_graph` - The call graph containing relationship data
///
/// # Returns
///
/// Enriched metrics with call graph data populated.
pub fn enrich_metrics_with_call_graph(
    metrics: Vec<FunctionMetrics>,
    call_graph: &CallGraph,
) -> Vec<FunctionMetrics> {
    call_graph_integration::populate_call_graph_data(metrics, call_graph)
}

/// Apply trait pattern detection to call graph (mutates graph).
///
/// This ensures trait methods (Default, Clone, constructors, etc.) are marked
/// as entry points after the enhanced graph has been merged in.
///
/// # Arguments
///
/// * `call_graph` - The call graph to update
pub fn apply_trait_patterns(call_graph: &mut CallGraph) {
    use crate::analysis::call_graph::TraitRegistry;
    let trait_registry = TraitRegistry::new();
    trait_registry.detect_common_trait_patterns(call_graph);
}

/// Find test-only functions in the call graph (pure).
///
/// Identifies functions that are only reachable from test code.
///
/// # Arguments
///
/// * `call_graph` - The call graph to analyze
///
/// # Returns
///
/// A set of function IDs that are only used by tests.
pub fn find_test_only_functions(call_graph: &CallGraph) -> HashSet<FunctionId> {
    call_graph.find_test_only_functions().into_iter().collect()
}

/// Pure predicate: should skip test functions.
pub fn is_test_function(metric: &FunctionMetrics) -> bool {
    metric.is_test || metric.in_test_module
}

/// Pure predicate: is closure function.
pub fn is_closure(metric: &FunctionMetrics) -> bool {
    metric.name.contains("<closure@")
}

/// Pure predicate: is trivial function.
pub fn is_trivial_function(metric: &FunctionMetrics, callee_count: usize) -> bool {
    metric.cyclomatic == 1 && metric.cognitive == 0 && metric.length <= 3 && callee_count == 1
}

/// Pure predicate: should process metric for debt analysis.
///
/// Determines whether a function metric should be included in debt analysis
/// based on various filtering criteria.
pub fn should_process_metric(
    metric: &FunctionMetrics,
    call_graph: &CallGraph,
    test_only_functions: &HashSet<FunctionId>,
) -> bool {
    // Early returns for test functions and closures
    if is_test_function(metric) || is_closure(metric) {
        return false;
    }

    let func_id = FunctionId::new(metric.file.clone(), metric.name.clone(), metric.line);

    // Skip if in test-only functions set
    if test_only_functions.contains(&func_id) {
        return false;
    }

    // Get callee count for triviality check
    let callee_count = call_graph.get_callees(&func_id).len();

    // Skip trivial functions
    !is_trivial_function(metric, callee_count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn create_test_metric(name: &str, is_test: bool, cyclomatic: u32) -> FunctionMetrics {
        FunctionMetrics {
            name: name.to_string(),
            file: PathBuf::from("test.rs"),
            line: 1,
            length: 10,
            cyclomatic,
            cognitive: 0,
            nesting: 0,
            is_test,
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
        }
    }

    fn create_trivial_metric() -> FunctionMetrics {
        let mut metric = create_test_metric("trivial", false, 1);
        metric.length = 3;
        metric
    }

    fn create_complex_metric() -> FunctionMetrics {
        let mut metric = create_test_metric("complex", false, 10);
        metric.length = 50;
        metric.cognitive = 15;
        metric
    }

    #[test]
    fn test_is_test_function() {
        let test_fn = create_test_metric("test_foo", true, 1);
        let normal_fn = create_test_metric("foo", false, 1);

        assert!(is_test_function(&test_fn));
        assert!(!is_test_function(&normal_fn));
    }

    #[test]
    fn test_is_closure() {
        let closure = create_test_metric("<closure@1:5>", false, 1);
        let normal_fn = create_test_metric("foo", false, 1);

        assert!(is_closure(&closure));
        assert!(!is_closure(&normal_fn));
    }

    #[test]
    fn test_is_trivial_function() {
        let trivial = create_trivial_metric();
        let complex = create_complex_metric();

        assert!(is_trivial_function(&trivial, 1));
        assert!(!is_trivial_function(&complex, 5));
    }

    #[test]
    fn test_build_initial_call_graph_empty() {
        let metrics: Vec<FunctionMetrics> = vec![];
        let graph = build_initial_call_graph(&metrics);
        assert_eq!(graph.node_count(), 0);
    }
}
