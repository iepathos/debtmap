//! Centrality Metrics for Call Graphs
//!
//! This module computes centrality measures that identify important functions
//! in the call graph based on their structural position.

use crate::priority::call_graph::{CallGraph, FunctionId};
use petgraph::algo::dijkstra;
use petgraph::graph::{DiGraph, NodeIndex};
use std::collections::HashMap;

/// Compute betweenness centrality for a function
///
/// Betweenness centrality measures how often a function appears on the shortest
/// paths between other functions. High betweenness indicates a bridge function
/// that connects different modules or subsystems.
///
/// # Arguments
///
/// * `call_graph` - The call graph to analyze
/// * `function_id` - The function to compute centrality for
///
/// # Returns
///
/// A value between 0.0 and 1.0 indicating normalized betweenness centrality
pub fn compute_betweenness_centrality(call_graph: &CallGraph, function_id: &FunctionId) -> f64 {
    // Convert CallGraph to petgraph DiGraph for efficient algorithm implementation
    let (graph, node_map) = build_petgraph(call_graph);

    // Find the node index for our target function
    let target_node = match node_map.get(function_id) {
        Some(&node) => node,
        None => return 0.0,
    };

    // Count how many shortest paths pass through this node
    let mut betweenness = 0.0;
    let all_nodes: Vec<NodeIndex> = graph.node_indices().collect();

    // For each pair of nodes, check if shortest path goes through target
    for &source in &all_nodes {
        if source == target_node {
            continue;
        }

        let distances = dijkstra(&graph, source, None, |_| 1);

        for &destination in &all_nodes {
            if destination == source || destination == target_node {
                continue;
            }

            // Check if path from source to destination goes through target
            if let (Some(&source_to_target), Some(&source_to_dest)) =
                (distances.get(&target_node), distances.get(&destination))
            {
                let target_to_dest = dijkstra(&graph, target_node, Some(destination), |_| 1)
                    .get(&destination)
                    .copied()
                    .unwrap_or(usize::MAX);

                // If distances add up, the shortest path goes through target
                if source_to_target + target_to_dest == source_to_dest {
                    betweenness += 1.0;
                }
            }
        }
    }

    // Normalize by the number of possible pairs
    let n = all_nodes.len();
    if n <= 2 {
        return 0.0;
    }

    let max_pairs = (n - 1) * (n - 2);
    betweenness / max_pairs as f64
}

/// Compute depth from entry points
///
/// Depth measures how far a function is from entry points (main, public APIs).
/// Lower depth indicates functions closer to user-facing code.
///
/// # Arguments
///
/// * `call_graph` - The call graph to analyze
/// * `function_id` - The function to compute depth for
///
/// # Returns
///
/// The minimum distance to any entry point, or usize::MAX if unreachable
pub fn compute_depth_from_entry_points(call_graph: &CallGraph, function_id: &FunctionId) -> usize {
    let (graph, node_map) = build_petgraph(call_graph);

    let target_node = match node_map.get(function_id) {
        Some(&node) => node,
        None => return usize::MAX,
    };

    // Find entry points (functions with zero incoming calls or marked as entry points)
    let entry_points = find_entry_points(call_graph, &node_map);

    if entry_points.is_empty() {
        return usize::MAX;
    }

    // Find minimum distance from any entry point
    entry_points
        .iter()
        .filter_map(|&entry| {
            dijkstra(&graph, entry, Some(target_node), |_| 1)
                .get(&target_node)
                .copied()
        })
        .min()
        .unwrap_or(usize::MAX)
}

/// Build a petgraph DiGraph from our CallGraph for efficient algorithms
fn build_petgraph(
    call_graph: &CallGraph,
) -> (DiGraph<FunctionId, ()>, HashMap<FunctionId, NodeIndex>) {
    let mut graph = DiGraph::new();
    let mut node_map = HashMap::new();

    // Add all functions as nodes
    let all_functions = call_graph.find_all_functions();
    for func_id in all_functions {
        let node = graph.add_node(func_id.clone());
        node_map.insert(func_id, node);
    }

    // Add all calls as edges
    for func_id in node_map.keys() {
        let callees = call_graph.get_callees(func_id);
        if let Some(&caller_node) = node_map.get(func_id) {
            for callee in callees {
                if let Some(&callee_node) = node_map.get(&callee) {
                    graph.add_edge(caller_node, callee_node, ());
                }
            }
        }
    }

    (graph, node_map)
}

/// Find entry points in the call graph
fn find_entry_points(
    call_graph: &CallGraph,
    node_map: &HashMap<FunctionId, NodeIndex>,
) -> Vec<NodeIndex> {
    let mut entry_points = Vec::new();

    for (func_id, &node) in node_map {
        // Entry points are:
        // 1. Functions explicitly marked as entry points
        // 2. Functions with zero incoming calls (potential main functions)
        // 3. Public API functions
        if call_graph.is_entry_point(func_id) || call_graph.get_callers(func_id).is_empty() {
            entry_points.push(node);
        }
    }

    // If no entry points found, use functions with lowest indegree
    if entry_points.is_empty() {
        let mut functions_by_indegree: Vec<_> = node_map
            .iter()
            .map(|(func_id, &node)| (call_graph.get_callers(func_id).len(), node))
            .collect();
        functions_by_indegree.sort_by_key(|(indegree, _)| *indegree);

        // Take functions with minimum indegree
        if let Some(&(min_indegree, _)) = functions_by_indegree.first() {
            entry_points = functions_by_indegree
                .iter()
                .take_while(|(indegree, _)| *indegree == min_indegree)
                .map(|(_, node)| *node)
                .collect();
        }
    }

    entry_points
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn create_test_function_id(name: &str, line: usize) -> FunctionId {
        FunctionId::new(PathBuf::from("test.rs"), name.to_string(), line)
    }

    #[test]
    fn test_betweenness_centrality_bridge_function() {
        let mut call_graph = CallGraph::new();

        // Create a graph where func2 is a bridge between two clusters
        // Cluster 1: func1 -> func2
        // Cluster 2: func2 -> func3
        let func1 = create_test_function_id("func1", 1);
        let func2 = create_test_function_id("func2", 10);
        let func3 = create_test_function_id("func3", 20);

        call_graph.add_function(func1.clone(), false, false, 1, 5);
        call_graph.add_function(func2.clone(), false, false, 1, 5);
        call_graph.add_function(func3.clone(), false, false, 1, 5);

        call_graph.add_call(crate::priority::call_graph::FunctionCall {
            caller: func1.clone(),
            callee: func2.clone(),
            call_type: crate::priority::call_graph::CallType::Direct,
        });
        call_graph.add_call(crate::priority::call_graph::FunctionCall {
            caller: func2.clone(),
            callee: func3.clone(),
            call_type: crate::priority::call_graph::CallType::Direct,
        });

        let centrality = compute_betweenness_centrality(&call_graph, &func2);

        // func2 should have non-zero betweenness as it's on the path from func1 to func3
        assert!(
            centrality > 0.0,
            "Bridge function should have non-zero betweenness"
        );
    }

    #[test]
    fn test_depth_from_entry_points() {
        let mut call_graph = CallGraph::new();

        // Create a chain: entry -> func1 -> func2 -> func3
        let entry = create_test_function_id("main", 1);
        let func1 = create_test_function_id("func1", 10);
        let func2 = create_test_function_id("func2", 20);
        let func3 = create_test_function_id("func3", 30);

        call_graph.add_function(entry.clone(), true, false, 1, 5);
        call_graph.add_function(func1.clone(), false, false, 1, 5);
        call_graph.add_function(func2.clone(), false, false, 1, 5);
        call_graph.add_function(func3.clone(), false, false, 1, 5);

        call_graph.add_call(crate::priority::call_graph::FunctionCall {
            caller: entry.clone(),
            callee: func1.clone(),
            call_type: crate::priority::call_graph::CallType::Direct,
        });
        call_graph.add_call(crate::priority::call_graph::FunctionCall {
            caller: func1.clone(),
            callee: func2.clone(),
            call_type: crate::priority::call_graph::CallType::Direct,
        });
        call_graph.add_call(crate::priority::call_graph::FunctionCall {
            caller: func2.clone(),
            callee: func3.clone(),
            call_type: crate::priority::call_graph::CallType::Direct,
        });

        assert_eq!(compute_depth_from_entry_points(&call_graph, &entry), 0);
        assert_eq!(compute_depth_from_entry_points(&call_graph, &func1), 1);
        assert_eq!(compute_depth_from_entry_points(&call_graph, &func2), 2);
        assert_eq!(compute_depth_from_entry_points(&call_graph, &func3), 3);
    }
}
