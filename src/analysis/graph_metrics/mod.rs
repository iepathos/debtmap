//! Graph Metrics Module
//!
//! This module provides graph-based metrics computation for call graphs,
//! including centrality measures, clustering coefficients, and structural
//! pattern detection for responsibility classification.
//!
//! # Features
//!
//! - Betweenness centrality (identifies bridge functions between modules)
//! - Clustering coefficient (identifies tightly-coupled function groups)
//! - Pattern detection (orchestrators, hubs, leaf nodes, bridges)
//! - Integration with I/O detection for responsibility classification

pub mod centrality;
pub mod clustering;
pub mod patterns;

pub use centrality::{compute_betweenness_centrality, compute_depth_from_entry_points};
pub use clustering::compute_clustering_coefficient;
pub use patterns::{CallGraphPattern, PatternDetector, ResponsibilityClassification};

use crate::priority::call_graph::{CallGraph, FunctionId};
use serde::{Deserialize, Serialize};

/// Graph metrics for a function in the call graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphMetrics {
    /// Number of functions this function calls (outgoing edges)
    pub outdegree: usize,
    /// Number of functions that call this function (incoming edges)
    pub indegree: usize,
    /// Distance from entry points (main, public APIs)
    pub depth: usize,
    /// Betweenness centrality (how often this function appears on shortest paths)
    pub betweenness: f64,
    /// Clustering coefficient (how connected a function's neighbors are)
    pub clustering: f64,
}

impl GraphMetrics {
    /// Create empty metrics
    pub fn empty() -> Self {
        Self {
            outdegree: 0,
            indegree: 0,
            depth: usize::MAX,
            betweenness: 0.0,
            clustering: 0.0,
        }
    }

    /// Compute metrics for a function in the call graph
    pub fn compute(call_graph: &CallGraph, function_id: &FunctionId) -> Self {
        let outdegree = call_graph.get_callees(function_id).len();
        let indegree = call_graph.get_callers(function_id).len();
        let depth = compute_depth_from_entry_points(call_graph, function_id);
        let betweenness = compute_betweenness_centrality(call_graph, function_id);
        let clustering = compute_clustering_coefficient(call_graph, function_id);

        Self {
            outdegree,
            indegree,
            depth,
            betweenness,
            clustering,
        }
    }

    /// Check if function is an orchestrator (high outdegree, coordinates operations)
    pub fn is_orchestrator(&self) -> bool {
        self.outdegree >= 5 && self.indegree <= 3
    }

    /// Check if function is a leaf node (no outgoing calls)
    pub fn is_leaf(&self) -> bool {
        self.outdegree == 0
    }

    /// Check if function is a hub (frequently called)
    pub fn is_hub(&self) -> bool {
        self.indegree >= 10
    }

    /// Check if function is a bridge (high betweenness, connects modules)
    pub fn is_bridge(&self) -> bool {
        self.betweenness > 0.5
    }

    /// Check if function is part of utility cluster (tight coupling)
    pub fn is_utility_cluster(&self) -> bool {
        self.clustering > 0.6 && self.indegree >= 3
    }
}
