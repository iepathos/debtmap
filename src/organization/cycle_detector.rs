/// Circular dependency detection for module splits
///
/// This module detects cycles in the dependency graph between proposed module splits
/// using depth-first search.
use crate::organization::god_object_analysis::ModuleSplit;
use std::collections::{HashMap, HashSet};

/// Detect circular dependencies in module splits
///
/// Uses depth-first search to detect cycles in the module dependency graph.
///
/// # Returns
/// Vector of cycles, where each cycle is a vector of module names
pub fn detect_circular_dependencies(splits: &[ModuleSplit]) -> Vec<Vec<String>> {
    // Build dependency graph
    let mut graph: HashMap<String, Vec<String>> = HashMap::new();
    for split in splits {
        graph.insert(split.suggested_name.clone(), split.dependencies_in.clone());
    }

    // DFS-based cycle detection
    let mut cycles = Vec::new();
    let mut visited = HashSet::new();
    let mut rec_stack = HashSet::new();
    let mut current_path = Vec::new();

    for node in graph.keys() {
        if !visited.contains(node) {
            dfs_find_cycles(
                node,
                &graph,
                &mut visited,
                &mut rec_stack,
                &mut current_path,
                &mut cycles,
            );
        }
    }

    cycles
}

fn dfs_find_cycles(
    node: &str,
    graph: &HashMap<String, Vec<String>>,
    visited: &mut HashSet<String>,
    rec_stack: &mut HashSet<String>,
    current_path: &mut Vec<String>,
    cycles: &mut Vec<Vec<String>>,
) {
    visited.insert(node.to_string());
    rec_stack.insert(node.to_string());
    current_path.push(node.to_string());

    if let Some(neighbors) = graph.get(node) {
        for neighbor in neighbors {
            if !visited.contains(neighbor) {
                dfs_find_cycles(neighbor, graph, visited, rec_stack, current_path, cycles);
            } else if rec_stack.contains(neighbor) {
                // Found a cycle
                if let Some(cycle_start) = current_path.iter().position(|n| n == neighbor) {
                    let mut cycle = current_path[cycle_start..].to_vec();
                    cycle.sort();
                    // Only add if not already present (avoid duplicates)
                    if !cycles.iter().any(|c| {
                        let mut sorted_c = c.clone();
                        sorted_c.sort();
                        sorted_c == cycle
                    }) {
                        cycles.push(current_path[cycle_start..].to_vec());
                    }
                }
            }
        }
    }

    current_path.pop();
    rec_stack.remove(node);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::organization::god_object_analysis::{ModuleSplit, Priority};

    fn create_split_with_deps(name: &str, deps_in: Vec<&str>, _deps_out: Vec<&str>) -> ModuleSplit {
        ModuleSplit {
            suggested_name: name.to_string(),
            methods_to_move: vec![],
            structs_to_move: vec![],
            responsibility: "test".to_string(),
            estimated_lines: 100,
            method_count: 5,
            warning: None,
            priority: Priority::Medium,
            cohesion_score: None,
            dependencies_in: deps_in.iter().map(|s| s.to_string()).collect(),
            dependencies_out: _deps_out.iter().map(|s| s.to_string()).collect(),
            domain: String::new(),
            rationale: None,
            method: crate::organization::SplitAnalysisMethod::None,
            severity: None,
            interface_estimate: None,
            classification_evidence: None,
        }
    }

    #[test]
    fn test_simple_cycle_detection() {
        let splits = vec![
            create_split_with_deps("ModuleA", vec!["ModuleB"], vec![]),
            create_split_with_deps("ModuleB", vec!["ModuleC"], vec![]),
            create_split_with_deps("ModuleC", vec!["ModuleA"], vec![]),
        ];

        let cycles = detect_circular_dependencies(&splits);

        assert_eq!(cycles.len(), 1, "Should detect one cycle");
        assert!(cycles[0].contains(&"ModuleA".to_string()));
        assert!(cycles[0].contains(&"ModuleB".to_string()));
        assert!(cycles[0].contains(&"ModuleC".to_string()));
    }

    #[test]
    fn test_no_cycles() {
        let splits = vec![
            create_split_with_deps("ModuleA", vec!["ModuleB"], vec![]),
            create_split_with_deps("ModuleB", vec!["ModuleC"], vec![]),
            create_split_with_deps("ModuleC", vec![], vec![]),
        ];

        let cycles = detect_circular_dependencies(&splits);
        assert_eq!(cycles.len(), 0, "Should detect no cycles");
    }

    #[test]
    fn test_self_cycle() {
        let splits = vec![create_split_with_deps("ModuleA", vec!["ModuleA"], vec![])];

        let cycles = detect_circular_dependencies(&splits);
        assert_eq!(cycles.len(), 1, "Should detect self-cycle");
    }

    #[test]
    fn test_two_node_cycle() {
        let splits = vec![
            create_split_with_deps("ModuleA", vec!["ModuleB"], vec![]),
            create_split_with_deps("ModuleB", vec!["ModuleA"], vec![]),
        ];

        let cycles = detect_circular_dependencies(&splits);
        assert_eq!(cycles.len(), 1, "Should detect two-node cycle");
    }

    #[test]
    fn test_multiple_independent_cycles() {
        let splits = vec![
            // Cycle 1: A -> B -> A
            create_split_with_deps("ModuleA", vec!["ModuleB"], vec![]),
            create_split_with_deps("ModuleB", vec!["ModuleA"], vec![]),
            // Cycle 2: C -> D -> C
            create_split_with_deps("ModuleC", vec!["ModuleD"], vec![]),
            create_split_with_deps("ModuleD", vec!["ModuleC"], vec![]),
        ];

        let cycles = detect_circular_dependencies(&splits);
        assert!(cycles.len() >= 2, "Should detect both cycles");
    }
}
