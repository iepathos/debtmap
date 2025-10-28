//! Clustering Coefficient for Call Graphs
//!
//! This module computes clustering coefficients that identify how tightly
//! connected a function's neighbors are, indicating utility clusters and
//! cohesive functional groups.

use crate::priority::call_graph::{CallGraph, FunctionId};

/// Compute clustering coefficient for a function
///
/// The clustering coefficient measures how connected a function's neighbors
/// (callees) are to each other. A high clustering coefficient indicates that
/// the function is part of a tightly-coupled group of functions that call
/// each other frequently.
///
/// # Formula
///
/// For a function with neighbors N, the clustering coefficient is:
/// C = (actual edges between neighbors) / (possible edges between neighbors)
///
/// Where possible edges = N * (N - 1) for a directed graph
///
/// # Arguments
///
/// * `call_graph` - The call graph to analyze
/// * `function_id` - The function to compute clustering for
///
/// # Returns
///
/// A value between 0.0 and 1.0 indicating the clustering coefficient
pub fn compute_clustering_coefficient(call_graph: &CallGraph, function_id: &FunctionId) -> f64 {
    // Get the neighbors (functions this function calls)
    let neighbors = call_graph.get_callees(function_id);

    // Need at least 2 neighbors for clustering to be meaningful
    if neighbors.len() < 2 {
        return 0.0;
    }

    // Count how many edges exist between neighbors
    let mut actual_edges = 0;
    for i in 0..neighbors.len() {
        for j in 0..neighbors.len() {
            if i != j {
                // Check if neighbor[i] calls neighbor[j]
                let neighbor_i_callees = call_graph.get_callees(&neighbors[i]);
                if neighbor_i_callees.contains(&neighbors[j]) {
                    actual_edges += 1;
                }
            }
        }
    }

    // Calculate possible edges (N * (N-1) for directed graph)
    let n = neighbors.len();
    let possible_edges = n * (n - 1);

    if possible_edges == 0 {
        return 0.0;
    }

    actual_edges as f64 / possible_edges as f64
}

/// Compute local clustering for bidirectional connections
///
/// This variant considers only bidirectional edges (mutual calls) between
/// neighbors, which can indicate stronger coupling.
///
/// # Arguments
///
/// * `call_graph` - The call graph to analyze
/// * `function_id` - The function to compute clustering for
///
/// # Returns
///
/// A value between 0.0 and 1.0 indicating bidirectional clustering
pub fn compute_bidirectional_clustering(call_graph: &CallGraph, function_id: &FunctionId) -> f64 {
    let neighbors = call_graph.get_callees(function_id);

    if neighbors.len() < 2 {
        return 0.0;
    }

    let mut bidirectional_edges = 0;
    for i in 0..neighbors.len() {
        for j in (i + 1)..neighbors.len() {
            // Check if neighbors[i] and neighbors[j] call each other
            let i_callees = call_graph.get_callees(&neighbors[i]);
            let j_callees = call_graph.get_callees(&neighbors[j]);

            if i_callees.contains(&neighbors[j]) && j_callees.contains(&neighbors[i]) {
                bidirectional_edges += 1;
            }
        }
    }

    let n = neighbors.len();
    let possible_pairs = n * (n - 1) / 2;

    if possible_pairs == 0 {
        return 0.0;
    }

    bidirectional_edges as f64 / possible_pairs as f64
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn create_test_function_id(name: &str, line: usize) -> FunctionId {
        FunctionId::new(PathBuf::from("test.rs"), name.to_string(), line)
    }

    #[test]
    fn test_clustering_coefficient_tight_cluster() {
        let mut call_graph = CallGraph::new();

        // Create a tight cluster where all neighbors call each other
        // main -> func1, func2, func3
        // func1 -> func2, func3
        // func2 -> func1, func3
        // func3 -> func1, func2
        let main = create_test_function_id("main", 1);
        let func1 = create_test_function_id("func1", 10);
        let func2 = create_test_function_id("func2", 20);
        let func3 = create_test_function_id("func3", 30);

        call_graph.add_function(main.clone(), true, false, 1, 5);
        call_graph.add_function(func1.clone(), false, false, 1, 5);
        call_graph.add_function(func2.clone(), false, false, 1, 5);
        call_graph.add_function(func3.clone(), false, false, 1, 5);

        // Main calls all three
        call_graph.add_call(crate::priority::call_graph::FunctionCall {
            caller: main.clone(),
            callee: func1.clone(),
            call_type: crate::priority::call_graph::CallType::Direct,
        });
        call_graph.add_call(crate::priority::call_graph::FunctionCall {
            caller: main.clone(),
            callee: func2.clone(),
            call_type: crate::priority::call_graph::CallType::Direct,
        });
        call_graph.add_call(crate::priority::call_graph::FunctionCall {
            caller: main.clone(),
            callee: func3.clone(),
            call_type: crate::priority::call_graph::CallType::Direct,
        });

        // func1 calls func2 and func3
        call_graph.add_call(crate::priority::call_graph::FunctionCall {
            caller: func1.clone(),
            callee: func2.clone(),
            call_type: crate::priority::call_graph::CallType::Direct,
        });
        call_graph.add_call(crate::priority::call_graph::FunctionCall {
            caller: func1.clone(),
            callee: func3.clone(),
            call_type: crate::priority::call_graph::CallType::Direct,
        });

        // func2 calls func1 and func3
        call_graph.add_call(crate::priority::call_graph::FunctionCall {
            caller: func2.clone(),
            callee: func1.clone(),
            call_type: crate::priority::call_graph::CallType::Direct,
        });
        call_graph.add_call(crate::priority::call_graph::FunctionCall {
            caller: func2.clone(),
            callee: func3.clone(),
            call_type: crate::priority::call_graph::CallType::Direct,
        });

        // func3 calls func1 and func2
        call_graph.add_call(crate::priority::call_graph::FunctionCall {
            caller: func3.clone(),
            callee: func1.clone(),
            call_type: crate::priority::call_graph::CallType::Direct,
        });
        call_graph.add_call(crate::priority::call_graph::FunctionCall {
            caller: func3.clone(),
            callee: func2.clone(),
            call_type: crate::priority::call_graph::CallType::Direct,
        });

        let clustering = compute_clustering_coefficient(&call_graph, &main);

        // With 3 neighbors and all 6 possible edges present, clustering should be 1.0
        assert!(
            clustering > 0.9,
            "Tight cluster should have high clustering coefficient, got {}",
            clustering
        );
    }

    #[test]
    fn test_clustering_coefficient_no_cluster() {
        let mut call_graph = CallGraph::new();

        // Create a star pattern where neighbors don't call each other
        // main -> func1, func2, func3 (but no edges between func1, func2, func3)
        let main = create_test_function_id("main", 1);
        let func1 = create_test_function_id("func1", 10);
        let func2 = create_test_function_id("func2", 20);
        let func3 = create_test_function_id("func3", 30);

        call_graph.add_function(main.clone(), true, false, 1, 5);
        call_graph.add_function(func1.clone(), false, false, 1, 5);
        call_graph.add_function(func2.clone(), false, false, 1, 5);
        call_graph.add_function(func3.clone(), false, false, 1, 5);

        call_graph.add_call(crate::priority::call_graph::FunctionCall {
            caller: main.clone(),
            callee: func1.clone(),
            call_type: crate::priority::call_graph::CallType::Direct,
        });
        call_graph.add_call(crate::priority::call_graph::FunctionCall {
            caller: main.clone(),
            callee: func2.clone(),
            call_type: crate::priority::call_graph::CallType::Direct,
        });
        call_graph.add_call(crate::priority::call_graph::FunctionCall {
            caller: main.clone(),
            callee: func3.clone(),
            call_type: crate::priority::call_graph::CallType::Direct,
        });

        let clustering = compute_clustering_coefficient(&call_graph, &main);

        // No edges between neighbors, so clustering should be 0.0
        assert_eq!(
            clustering, 0.0,
            "No clustering should result in 0.0 coefficient"
        );
    }

    #[test]
    fn test_clustering_coefficient_single_neighbor() {
        let mut call_graph = CallGraph::new();

        let main = create_test_function_id("main", 1);
        let func1 = create_test_function_id("func1", 10);

        call_graph.add_function(main.clone(), true, false, 1, 5);
        call_graph.add_function(func1.clone(), false, false, 1, 5);

        call_graph.add_call(crate::priority::call_graph::FunctionCall {
            caller: main.clone(),
            callee: func1.clone(),
            call_type: crate::priority::call_graph::CallType::Direct,
        });

        let clustering = compute_clustering_coefficient(&call_graph, &main);

        // Single neighbor should result in 0.0 clustering
        assert_eq!(
            clustering, 0.0,
            "Single neighbor should have 0.0 clustering"
        );
    }
}
