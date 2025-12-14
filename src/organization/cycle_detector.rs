/// Circular dependency detection for module splits
///
/// This module detects cycles in the dependency graph between proposed module splits
/// using depth-first search.
use crate::organization::god_object::types::ModuleSplit;
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

/// Check if a cycle (as sorted nodes) already exists in the cycles collection
fn is_duplicate_cycle(cycles: &[Vec<String>], new_cycle_sorted: &[String]) -> bool {
    cycles.iter().any(|existing| {
        let mut sorted = existing.clone();
        sorted.sort();
        sorted == new_cycle_sorted
    })
}

/// Extract a cycle from the current path starting at the given index
fn extract_cycle(path: &[String], cycle_start_idx: usize) -> Vec<String> {
    path[cycle_start_idx..].to_vec()
}

/// Create a sorted copy of a cycle for duplicate comparison
fn normalize_cycle(cycle: &[String]) -> Vec<String> {
    let mut sorted = cycle.to_vec();
    sorted.sort();
    sorted
}

/// Try to record a cycle if it's not a duplicate
fn try_record_cycle(cycles: &mut Vec<Vec<String>>, path: &[String], cycle_start_idx: usize) {
    let cycle = extract_cycle(path, cycle_start_idx);
    let normalized = normalize_cycle(&cycle);
    if !is_duplicate_cycle(cycles, &normalized) {
        cycles.push(cycle);
    }
}

/// Process entering a node: mark as visited and on recursion stack
fn enter_node(
    node: &str,
    visited: &mut HashSet<String>,
    rec_stack: &mut HashSet<String>,
    path: &mut Vec<String>,
) {
    visited.insert(node.to_string());
    rec_stack.insert(node.to_string());
    path.push(node.to_string());
}

/// Process leaving a node: remove from recursion stack and path
fn leave_node(node: &str, rec_stack: &mut HashSet<String>, path: &mut Vec<String>) {
    path.pop();
    rec_stack.remove(node);
}

/// Collect neighbor actions: returns nodes to visit and cycles found
fn process_neighbors(
    neighbors: &[String],
    visited: &HashSet<String>,
    rec_stack: &HashSet<String>,
    path: &[String],
) -> (Vec<String>, Vec<usize>) {
    let mut to_visit = Vec::new();
    let mut cycle_starts = Vec::new();

    for neighbor in neighbors.iter().rev() {
        if !visited.contains(neighbor) {
            to_visit.push(neighbor.clone());
        } else if rec_stack.contains(neighbor) {
            if let Some(idx) = path.iter().position(|n| n == neighbor) {
                cycle_starts.push(idx);
            }
        }
    }

    (to_visit, cycle_starts)
}

/// Iterative DFS cycle detection to avoid stack overflow on large graphs.
///
/// Uses an explicit stack with frame state tracking to simulate recursive DFS.
fn dfs_find_cycles(
    start_node: &str,
    graph: &HashMap<String, Vec<String>>,
    visited: &mut HashSet<String>,
    rec_stack: &mut HashSet<String>,
    _current_path: &mut [String],
    cycles: &mut Vec<Vec<String>>,
) {
    let mut stack: Vec<(String, bool)> = vec![(start_node.to_string(), true)];
    let mut path: Vec<String> = Vec::new();

    while let Some((node, is_entering)) = stack.pop() {
        if !is_entering {
            leave_node(&node, rec_stack, &mut path);
            continue;
        }

        enter_node(&node, visited, rec_stack, &mut path);
        stack.push((node.clone(), false));

        let neighbors = graph.get(&node).map(|v| v.as_slice()).unwrap_or(&[]);
        let (to_visit, cycle_starts) = process_neighbors(neighbors, visited, rec_stack, &path);

        for idx in cycle_starts {
            try_record_cycle(cycles, &path, idx);
        }

        for neighbor in to_visit {
            stack.push((neighbor, true));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::organization::god_object::types::{ModuleSplit, Priority};

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
            representative_methods: vec![],
            fields_needed: vec![],
            trait_suggestion: None,
            behavior_category: None,
            ..Default::default()
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

    // ========================================
    // Pure function unit tests (Stillwater style)
    // ========================================

    #[test]
    fn test_is_duplicate_cycle_finds_match() {
        let existing = vec![vec!["A".to_string(), "B".to_string(), "C".to_string()]];
        let new_cycle = vec!["A".to_string(), "B".to_string(), "C".to_string()];
        assert!(is_duplicate_cycle(&existing, &new_cycle));
    }

    #[test]
    fn test_is_duplicate_cycle_different_order_still_matches() {
        let existing = vec![vec!["C".to_string(), "A".to_string(), "B".to_string()]];
        // Same nodes, different order - normalized should match
        let new_cycle = vec!["A".to_string(), "B".to_string(), "C".to_string()];
        assert!(is_duplicate_cycle(&existing, &new_cycle));
    }

    #[test]
    fn test_is_duplicate_cycle_no_match() {
        let existing = vec![vec!["A".to_string(), "B".to_string()]];
        let new_cycle = vec!["A".to_string(), "C".to_string()];
        assert!(!is_duplicate_cycle(&existing, &new_cycle));
    }

    #[test]
    fn test_extract_cycle_from_middle() {
        let path = vec![
            "A".to_string(),
            "B".to_string(),
            "C".to_string(),
            "D".to_string(),
        ];
        let cycle = extract_cycle(&path, 1);
        assert_eq!(cycle, vec!["B", "C", "D"]);
    }

    #[test]
    fn test_normalize_cycle_sorts_alphabetically() {
        let cycle = vec!["C".to_string(), "A".to_string(), "B".to_string()];
        let normalized = normalize_cycle(&cycle);
        assert_eq!(normalized, vec!["A", "B", "C"]);
    }

    #[test]
    fn test_process_neighbors_categorizes_correctly() {
        let neighbors = vec![
            "unvisited".to_string(),
            "in_stack".to_string(),
            "visited_done".to_string(),
        ];
        let mut visited = HashSet::new();
        visited.insert("in_stack".to_string());
        visited.insert("visited_done".to_string());

        let mut rec_stack = HashSet::new();
        rec_stack.insert("in_stack".to_string());

        let path = vec!["start".to_string(), "in_stack".to_string()];

        let (to_visit, cycle_starts) = process_neighbors(&neighbors, &visited, &rec_stack, &path);

        // "unvisited" should be in to_visit
        assert!(to_visit.contains(&"unvisited".to_string()));
        // "in_stack" forms a cycle at index 1
        assert_eq!(cycle_starts.len(), 1);
        assert_eq!(cycle_starts[0], 1);
        // "visited_done" is visited but not in rec_stack - no cycle, no visit
    }

    #[test]
    fn test_process_neighbors_empty() {
        let neighbors: Vec<String> = vec![];
        let visited = HashSet::new();
        let rec_stack = HashSet::new();
        let path = vec!["start".to_string()];

        let (to_visit, cycle_starts) = process_neighbors(&neighbors, &visited, &rec_stack, &path);

        assert!(to_visit.is_empty());
        assert!(cycle_starts.is_empty());
    }

    #[test]
    fn test_enter_and_leave_node_state_management() {
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();
        let mut path = Vec::new();

        enter_node("A", &mut visited, &mut rec_stack, &mut path);

        assert!(visited.contains("A"));
        assert!(rec_stack.contains("A"));
        assert_eq!(path, vec!["A"]);

        leave_node("A", &mut rec_stack, &mut path);

        assert!(visited.contains("A")); // Still visited
        assert!(!rec_stack.contains("A")); // Removed from rec_stack
        assert!(path.is_empty()); // Removed from path
    }

    #[test]
    fn test_try_record_cycle_prevents_duplicates() {
        let mut cycles = vec![vec!["A".to_string(), "B".to_string()]];
        let path = vec!["X".to_string(), "A".to_string(), "B".to_string()];

        // Try to add same cycle (different order in path)
        try_record_cycle(&mut cycles, &path, 1);

        assert_eq!(cycles.len(), 1, "Duplicate should not be added");
    }

    #[test]
    fn test_try_record_cycle_adds_new() {
        let mut cycles = vec![vec!["A".to_string(), "B".to_string()]];
        let path = vec!["X".to_string(), "C".to_string(), "D".to_string()];

        try_record_cycle(&mut cycles, &path, 1);

        assert_eq!(cycles.len(), 2, "New cycle should be added");
        assert!(cycles.iter().any(|c| c.contains(&"C".to_string())));
    }

    #[test]
    fn test_empty_graph() {
        let splits: Vec<ModuleSplit> = vec![];
        let cycles = detect_circular_dependencies(&splits);
        assert!(cycles.is_empty());
    }
}
