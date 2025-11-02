//! Call Graph Adapter for Purity Propagation
//!
//! This adapter wraps the existing RustCallGraph to provide a clean interface
//! for purity propagation analysis.

use crate::analysis::call_graph::RustCallGraph;
use crate::priority::call_graph::FunctionId;
use anyhow::Result;

/// Adapter to use existing RustCallGraph for purity propagation
pub struct PurityCallGraphAdapter {
    rust_graph: RustCallGraph,
}

impl PurityCallGraphAdapter {
    /// Create adapter from existing call graph
    pub fn from_rust_graph(rust_graph: RustCallGraph) -> Self {
        Self { rust_graph }
    }

    /// Get dependencies for a function (functions this function calls)
    pub fn get_dependencies(&self, func_id: &FunctionId) -> Vec<FunctionId> {
        self.rust_graph.base_graph.get_callees(func_id)
    }

    /// Get dependents (callers) for a function
    pub fn get_dependents(&self, func_id: &FunctionId) -> Vec<FunctionId> {
        self.rust_graph.base_graph.get_callers(func_id)
    }

    /// Check if function is in a cycle (recursive)
    pub fn is_in_cycle(&self, func_id: &FunctionId) -> bool {
        self.rust_graph.base_graph.is_recursive(func_id)
    }

    /// Topological sort for bottom-up analysis
    pub fn topological_sort(&self) -> Result<Vec<FunctionId>> {
        // Delegate to existing call graph implementation
        self.rust_graph
            .base_graph
            .topological_sort()
            .map_err(|e| anyhow::anyhow!(e))
    }
}
