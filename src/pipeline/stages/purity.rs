//! Pure functions for purity analysis.
//!
//! These functions analyze function purity based on local inspection
//! and call graph analysis.

use crate::core::FunctionMetrics;
use crate::priority::call_graph::{CallGraph, FunctionId};
use std::collections::HashMap;

/// Purity category for a function.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PurityCategory {
    /// Function is pure (no side effects, deterministic)
    Pure,
    /// Function has local side effects only
    ImpureLocal,
    /// Function performs I/O or has global side effects
    Impure,
}

/// Analyze local purity of a function (pure).
///
/// Performs local inspection to determine if function has obvious
/// side effects like I/O operations.
///
/// # Arguments
///
/// * `metric` - Function metrics to analyze
///
/// # Returns
///
/// Purity category based on local analysis
pub fn analyze_local_purity(_metric: &FunctionMetrics) -> PurityCategory {
    // Simplified implementation - in reality would check for:
    // - I/O operations
    // - Mutation of external state
    // - Calls to known impure functions
    PurityCategory::Pure
}

/// Propagate purity through call graph (pure).
///
/// Iteratively refines purity classifications based on what
/// functions call. A function is only pure if all its callees are pure.
///
/// # Arguments
///
/// * `initial` - Initial purity classifications
/// * `graph` - Call graph for propagation
/// * `max_iterations` - Maximum propagation iterations
///
/// # Returns
///
/// Refined purity classifications after propagation
pub fn propagate_purity(
    initial: HashMap<FunctionId, PurityCategory>,
    graph: &CallGraph,
    max_iterations: usize,
) -> HashMap<FunctionId, PurityCategory> {
    let mut purity = initial;

    for _ in 0..max_iterations {
        let updated = propagate_one_step(&purity, graph);
        if updated == purity {
            break; // Converged
        }
        purity = updated;
    }

    purity
}

/// Single propagation step (pure).
fn propagate_one_step(
    current: &HashMap<FunctionId, PurityCategory>,
    graph: &CallGraph,
) -> HashMap<FunctionId, PurityCategory> {
    current
        .iter()
        .map(|(id, category)| {
            let callees = graph.get_callees(id);
            let updated = refine_purity(*category, &callees, current);
            (id.clone(), updated)
        })
        .collect()
}

/// Refine purity based on callees (pure).
fn refine_purity(
    current: PurityCategory,
    callees: &[FunctionId],
    purity_map: &HashMap<FunctionId, PurityCategory>,
) -> PurityCategory {
    if current == PurityCategory::Impure {
        return PurityCategory::Impure;
    }

    let has_impure_callee = callees
        .iter()
        .filter_map(|id| purity_map.get(id))
        .any(|p| *p == PurityCategory::Impure);

    if has_impure_callee {
        PurityCategory::Impure
    } else {
        current
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn test_func_id(name: &str) -> FunctionId {
        FunctionId::new(PathBuf::from("test.rs"), name.to_string(), 1)
    }

    #[test]
    fn test_propagate_purity_empty() {
        let initial = HashMap::new();
        let graph = CallGraph::new();
        let result = propagate_purity(initial, &graph, 10);
        assert!(result.is_empty());
    }

    #[test]
    fn test_propagate_purity_converges() {
        let mut initial = HashMap::new();
        initial.insert(test_func_id("foo"), PurityCategory::Pure);
        initial.insert(test_func_id("bar"), PurityCategory::Pure);

        let graph = CallGraph::new();
        let result = propagate_purity(initial.clone(), &graph, 10);

        // Without call edges, should remain pure
        assert_eq!(
            result.get(&test_func_id("foo")),
            Some(&PurityCategory::Pure)
        );
        assert_eq!(
            result.get(&test_func_id("bar")),
            Some(&PurityCategory::Pure)
        );
    }
}
