//! Stress tests for call graph operations to detect stack overflow issues.
//!
//! These tests exercise graph traversal algorithms (`is_recursive`, `topological_sort`)
//! at scale to ensure the iterative implementations (Spec 206) work correctly and
//! don't regress to recursive implementations that could cause stack overflow.
//!
//! ## Test Categories
//!
//! - **CI Tests**: Run on every CI build (not `#[ignore]`)
//! - **Stress Tests**: Marked `#[ignore]`, run with `cargo test --ignored`
//!
//! ## Stack Size Rationale
//!
//! Default stack: 8MB
//! Typical recursive frame: ~1KB
//! Safe depth: ~8,000 nodes
//! Tests use 5,000 for safety margin.

#[cfg(test)]
mod call_graph_stress_tests {
    use debtmap::priority::call_graph::{CallGraph, CallType, FunctionId};
    use std::path::PathBuf;
    use std::time::Instant;

    /// Generate a linear chain of N nodes: A -> B -> C -> ... (no cycle)
    fn create_linear_chain(n: usize) -> (CallGraph, Vec<FunctionId>) {
        let mut graph = CallGraph::new();

        let nodes: Vec<FunctionId> = (0..n)
            .map(|i| FunctionId::new(PathBuf::from("chain.rs"), format!("func_{}", i), i * 10))
            .collect();

        for node in &nodes {
            graph.add_function(node.clone(), false, false, 1, 10);
        }

        for i in 0..n.saturating_sub(1) {
            graph.add_call_parts(nodes[i].clone(), nodes[i + 1].clone(), CallType::Direct);
        }

        (graph, nodes)
    }

    /// Generate a ring (cycle) of N nodes: A -> B -> C -> ... -> A
    fn create_ring(n: usize) -> (CallGraph, Vec<FunctionId>) {
        let mut graph = CallGraph::new();

        let nodes: Vec<FunctionId> = (0..n)
            .map(|i| FunctionId::new(PathBuf::from("ring.rs"), format!("scc_func_{}", i), i * 10))
            .collect();

        for node in &nodes {
            graph.add_function(node.clone(), false, false, 1, 10);
        }

        // Create ring edges
        for i in 0..n {
            graph.add_call_parts(
                nodes[i].clone(),
                nodes[(i + 1) % n].clone(),
                CallType::Direct,
            );
        }

        (graph, nodes)
    }

    /// Generate a DAG with N nodes and fan-out F
    fn create_dag(n: usize, fan_out: usize) -> (CallGraph, Vec<FunctionId>) {
        let mut graph = CallGraph::new();

        let nodes: Vec<FunctionId> = (0..n)
            .map(|i| {
                FunctionId::new(
                    PathBuf::from(format!("module_{}/file.rs", i / 100)),
                    format!("func_{}", i),
                    (i % 100) * 10,
                )
            })
            .collect();

        for (i, node) in nodes.iter().enumerate() {
            graph.add_function(node.clone(), i % 10 == 0, false, 1, 10);
        }

        // Create DAG edges: each node calls next fan_out nodes (forward only)
        for i in 0..n {
            for j in 1..=fan_out {
                if i + j < n {
                    graph.add_call_parts(nodes[i].clone(), nodes[i + j].clone(), CallType::Direct);
                }
            }
        }

        (graph, nodes)
    }

    // =========================================================================
    // CI Tests (run on every build)
    // =========================================================================

    /// Quick test that runs in CI to catch regressions
    /// Smaller than stress tests but large enough to detect naive recursion
    #[test]
    fn test_graph_operations_ci_validation() {
        let mut graph = CallGraph::new();
        let size = 500; // Large enough to stress, small enough for CI

        // Create linear chain with cycle at end
        let nodes: Vec<FunctionId> = (0..size)
            .map(|i| FunctionId::new(PathBuf::from("ci_test.rs"), format!("f{}", i), i))
            .collect();

        for node in &nodes {
            graph.add_function(node.clone(), false, false, 1, 5);
        }

        // Linear chain
        for i in 0..size - 1 {
            graph.add_call_parts(nodes[i].clone(), nodes[i + 1].clone(), CallType::Direct);
        }

        // Add cycle at the end (nodes 490-499 form a cycle)
        graph.add_call_parts(
            nodes[size - 1].clone(),
            nodes[size - 10].clone(),
            CallType::Direct,
        );

        // Add an isolated node that cannot reach the cycle
        let isolated_node =
            FunctionId::new(PathBuf::from("isolated.rs"), "isolated_func".to_string(), 0);
        graph.add_function(isolated_node.clone(), false, false, 1, 5);

        // Test operations complete without overflow
        assert!(graph.is_recursive(&nodes[size - 5])); // In cycle
        assert!(!graph.is_recursive(&isolated_node)); // Isolated node has no cycle

        // Topo sort should handle cycle gracefully
        let _ = graph.topological_sort();
    }

    /// CI test for is_recursive on medium-sized linear chain
    #[test]
    fn test_is_recursive_ci_linear_chain() {
        let chain_length = 1_000;
        let (graph, nodes) = create_linear_chain(chain_length);

        // No cycles, so none should be recursive
        assert!(!graph.is_recursive(&nodes[0]));
        assert!(!graph.is_recursive(&nodes[chain_length / 2]));
        assert!(!graph.is_recursive(&nodes[chain_length - 1]));
    }

    /// CI test for topological_sort on medium-sized DAG
    #[test]
    fn test_topological_sort_ci_dag() {
        let num_nodes = 1_000;
        let (graph, _nodes) = create_dag(num_nodes, 3);

        let start = Instant::now();
        let result = graph.topological_sort();
        let elapsed = start.elapsed();

        assert!(result.is_ok());
        let sorted = result.unwrap();
        assert_eq!(sorted.len(), num_nodes);

        // Should complete quickly (< 1 second)
        assert!(
            elapsed.as_secs() < 1,
            "Topo sort took too long: {:?}",
            elapsed
        );
    }

    // =========================================================================
    // Stress Tests (run with --ignored)
    // =========================================================================

    /// Test is_recursive on a 5,000 node linear chain
    /// This would cause stack overflow with recursive implementation
    #[test]
    #[ignore = "stress test - run with --ignored"]
    fn test_is_recursive_large_linear_chain() {
        let chain_length = 5_000;
        let (graph, nodes) = create_linear_chain(chain_length);

        // No cycles, so none should be recursive
        // This traverses entire graph depth - would overflow with recursion
        let start = Instant::now();
        assert!(!graph.is_recursive(&nodes[0]));
        assert!(!graph.is_recursive(&nodes[chain_length / 2]));
        let elapsed = start.elapsed();

        // Ensure reasonable performance
        assert!(
            elapsed.as_secs() < 5,
            "is_recursive on linear chain took too long: {:?}",
            elapsed
        );
    }

    /// Test is_recursive on a 1,000 node fully connected component (ring)
    /// Every node can reach every other node
    #[test]
    #[ignore = "stress test - run with --ignored"]
    fn test_is_recursive_large_scc() {
        let scc_size = 1_000;
        let (graph, nodes) = create_ring(scc_size);

        // All nodes are in cycle
        let start = Instant::now();
        assert!(graph.is_recursive(&nodes[0]));
        assert!(graph.is_recursive(&nodes[scc_size / 2]));
        assert!(graph.is_recursive(&nodes[scc_size - 1]));
        let elapsed = start.elapsed();

        // Ensure reasonable performance
        assert!(
            elapsed.as_secs() < 5,
            "is_recursive on SCC took too long: {:?}",
            elapsed
        );
    }

    /// Test topological_sort on a 10,000 node DAG
    /// Validates O(N) performance without stack overflow
    #[test]
    #[ignore = "stress test - run with --ignored"]
    fn test_topological_sort_large_dag() {
        let num_nodes = 10_000;
        let (graph, _nodes) = create_dag(num_nodes, 3);

        let start = Instant::now();
        let result = graph.topological_sort();
        let elapsed = start.elapsed();

        assert!(result.is_ok());
        let sorted = result.unwrap();
        assert_eq!(sorted.len(), num_nodes);

        // Should complete quickly (< 1 second)
        assert!(
            elapsed.as_secs() < 1,
            "Topo sort took too long: {:?}",
            elapsed
        );
    }

    /// Test topological_sort on a 5,000 depth chain
    /// Worst case for stack depth
    #[test]
    #[ignore = "stress test - run with --ignored"]
    fn test_topological_sort_deep_chain() {
        let depth = 5_000;
        let (graph, _nodes) = create_linear_chain(depth);

        let start = Instant::now();
        let result = graph.topological_sort();
        let elapsed = start.elapsed();

        assert!(result.is_ok());
        let sorted = result.unwrap();

        // Verify order: leaves first (depth-1), then depth-2, ..., then 0
        // Last node in chain should come first in topo order
        assert_eq!(sorted.first().unwrap().name, format!("func_{}", depth - 1));
        assert_eq!(sorted.last().unwrap().name, "func_0");

        // Should complete quickly
        assert!(
            elapsed.as_secs() < 1,
            "Topo sort on deep chain took too long: {:?}",
            elapsed
        );
    }

    /// Combined stress test: large graph with both cycles and DAG portions
    #[test]
    #[ignore = "stress test - run with --ignored"]
    fn test_mixed_graph_large_scale() {
        let mut graph = CallGraph::new();
        let dag_size = 3_000;
        let cycle_size = 1_000;

        // Create DAG portion
        let dag_nodes: Vec<FunctionId> = (0..dag_size)
            .map(|i| FunctionId::new(PathBuf::from("dag.rs"), format!("dag_func_{}", i), i * 10))
            .collect();

        for node in &dag_nodes {
            graph.add_function(node.clone(), false, false, 1, 10);
        }

        for i in 0..dag_size {
            for j in 1..=2 {
                if i + j < dag_size {
                    graph.add_call_parts(
                        dag_nodes[i].clone(),
                        dag_nodes[i + j].clone(),
                        CallType::Direct,
                    );
                }
            }
        }

        // Create cycle portion
        let cycle_nodes: Vec<FunctionId> = (0..cycle_size)
            .map(|i| {
                FunctionId::new(
                    PathBuf::from("cycle.rs"),
                    format!("cycle_func_{}", i),
                    i * 10,
                )
            })
            .collect();

        for node in &cycle_nodes {
            graph.add_function(node.clone(), false, false, 1, 10);
        }

        // Ring in cycle portion
        for i in 0..cycle_size {
            graph.add_call_parts(
                cycle_nodes[i].clone(),
                cycle_nodes[(i + 1) % cycle_size].clone(),
                CallType::Direct,
            );
        }

        // Connect DAG to cycle
        graph.add_call_parts(
            dag_nodes[dag_size / 2].clone(),
            cycle_nodes[0].clone(),
            CallType::Direct,
        );

        let start = Instant::now();

        // DAG nodes before connection should not be recursive
        assert!(!graph.is_recursive(&dag_nodes[0]));

        // Cycle nodes should be recursive
        assert!(graph.is_recursive(&cycle_nodes[0]));
        assert!(graph.is_recursive(&cycle_nodes[cycle_size / 2]));

        // Topological sort should still work
        let result = graph.topological_sort();
        assert!(result.is_ok());

        let elapsed = start.elapsed();
        assert!(
            elapsed.as_secs() < 5,
            "Mixed graph operations took too long: {:?}",
            elapsed
        );
    }

    /// Test performance scaling - should be roughly O(N)
    #[test]
    #[ignore = "stress test - run with --ignored"]
    fn test_performance_scaling() {
        let sizes = [1_000, 2_000, 4_000, 8_000];
        let mut times = Vec::new();

        for &size in &sizes {
            let (graph, nodes) = create_linear_chain(size);

            let start = Instant::now();
            let _ = graph.is_recursive(&nodes[0]);
            let _ = graph.topological_sort();
            let elapsed = start.elapsed();

            times.push((size, elapsed.as_millis()));
        }

        // Verify roughly linear scaling (2x size should be < 4x time)
        // This is a loose check to catch O(N^2) or worse regressions
        for i in 1..times.len() {
            let (prev_size, prev_time) = times[i - 1];
            let (curr_size, curr_time) = times[i];

            let size_ratio = curr_size as f64 / prev_size as f64;
            let time_ratio = if prev_time > 0 {
                curr_time as f64 / prev_time as f64
            } else {
                1.0 // Avoid division by zero for very fast operations
            };

            // Allow some overhead but catch exponential blowup
            // Linear should be ~2x time for 2x size
            // We allow 4x as acceptable (accounts for cache effects, etc)
            assert!(
                time_ratio < size_ratio * 4.0,
                "Performance scaling regression: {}x size increase caused {}x time increase (sizes: {} -> {}, times: {}ms -> {}ms)",
                size_ratio,
                time_ratio,
                prev_size,
                curr_size,
                prev_time,
                curr_time
            );
        }
    }
}
