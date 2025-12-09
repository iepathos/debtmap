use crate::core::{CircularDependency, Dependency, ModuleDependency};
use std::collections::HashMap;

/// Dependency graph for analyzing module relationships
#[derive(Debug, Clone)]
pub struct DependencyGraph {
    adjacency: HashMap<String, Vec<String>>,
    modules: Vec<String>,
}

impl Default for DependencyGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl DependencyGraph {
    /// Create a new empty dependency graph
    pub fn new() -> Self {
        Self {
            adjacency: HashMap::new(),
            modules: Vec::new(),
        }
    }

    /// Add a module to the graph
    pub fn add_module(&mut self, module: String) {
        if !self.adjacency.contains_key(&module) {
            self.adjacency.insert(module.clone(), Vec::new());
            if !self.modules.contains(&module) {
                self.modules.push(module);
            }
        }
    }

    /// Add a dependency edge between two modules
    pub fn add_dependency(&mut self, from: String, to: String) {
        self.add_module(from.clone());
        self.add_module(to.clone());
        self.adjacency.entry(from).or_default().push(to);
    }

    /// Detect circular dependencies using iterative DFS
    ///
    /// Uses an iterative approach to avoid stack overflow on large dependency graphs.
    pub fn detect_circular_dependencies(&self) -> Vec<CircularDependency> {
        let mut visited: HashMap<String, bool> =
            self.modules.iter().map(|m| (m.clone(), false)).collect();
        let mut rec_stack: HashMap<String, bool> =
            self.modules.iter().map(|m| (m.clone(), false)).collect();

        let mut circular_deps = Vec::new();

        for module in &self.modules {
            if !visited.get(module).copied().unwrap_or(false) {
                self.dfs_detect_cycles_iterative(
                    module,
                    &mut visited,
                    &mut rec_stack,
                    &mut circular_deps,
                );
            }
        }

        circular_deps
    }

    /// Iterative DFS cycle detection to avoid stack overflow on large graphs.
    ///
    /// Uses an explicit stack with frame state tracking to simulate recursive DFS.
    /// Each frame tracks: module name, dependency index, and entry state.
    fn dfs_detect_cycles_iterative(
        &self,
        start_module: &str,
        visited: &mut HashMap<String, bool>,
        rec_stack: &mut HashMap<String, bool>,
        cycles: &mut Vec<CircularDependency>,
    ) {
        // Stack frame: (module_name, dep_index, is_entering)
        // is_entering=true: first visit to this node
        // is_entering=false: returning from processing children
        let mut stack: Vec<(String, usize, bool)> =
            vec![(start_module.to_string(), 0, true)];
        let mut path: Vec<String> = Vec::new();

        while let Some((module, _dep_idx, is_entering)) = stack.pop() {
            if is_entering {
                // First visit to this module - mark as visited and in recursion stack
                visited.insert(module.clone(), true);
                rec_stack.insert(module.clone(), true);
                path.push(module.clone());

                // Push a "return" frame to clean up when we're done with this module
                stack.push((module.clone(), 0, false));

                // Process dependencies
                if let Some(deps) = self.adjacency.get(&module) {
                    // Push dependencies in reverse order so we process them in forward order
                    for (i, dep) in deps.iter().enumerate().rev() {
                        if !visited.get(dep).copied().unwrap_or(false) {
                            // Unvisited node - push it for processing
                            stack.push((dep.clone(), i, true));
                        } else if rec_stack.get(dep).copied().unwrap_or(false) {
                            // Found a cycle! dep is already in the current path
                            if let Some(cycle_start) = path.iter().position(|m| m == dep) {
                                let cycle = path[cycle_start..].to_vec();
                                cycles.push(CircularDependency { cycle });
                            }
                        }
                    }
                }
            } else {
                // Returning from this module - clean up
                path.pop();
                rec_stack.insert(module, false);
            }
        }
    }

    /// Calculate coupling metrics for all modules
    pub fn calculate_coupling_metrics(&self) -> Vec<ModuleDependency> {
        self.modules
            .iter()
            .map(|module| {
                let dependencies = self.get_dependencies(module);
                let dependents = self.get_dependents(module);

                ModuleDependency {
                    module: module.clone(),
                    dependencies,
                    dependents,
                }
            })
            .collect()
    }

    /// Get the number of modules in the graph
    pub fn module_count(&self) -> usize {
        self.modules.len()
    }

    /// Get the number of dependencies (edges) in the graph
    pub fn dependency_count(&self) -> usize {
        self.adjacency.values().map(|deps| deps.len()).sum()
    }

    /// Check if a module exists in the graph
    pub fn has_module(&self, module: &str) -> bool {
        self.modules.contains(&module.to_string())
    }

    /// Get all modules that a given module depends on
    pub fn get_dependencies(&self, module: &str) -> Vec<String> {
        self.adjacency.get(module).cloned().unwrap_or_default()
    }

    /// Get all modules that depend on a given module
    pub fn get_dependents(&self, module: &str) -> Vec<String> {
        self.adjacency
            .iter()
            .filter_map(|(other_module, deps)| {
                (deps.contains(&module.to_string()) && other_module != module)
                    .then_some(other_module.clone())
            })
            .collect()
    }
}

/// Build a dependency graph from a list of dependencies
pub fn build_dependency_graph(dependencies: &[Dependency]) -> DependencyGraph {
    dependencies
        .iter()
        .filter_map(|dep| extract_module_from_dependency(&dep.name))
        .fold(DependencyGraph::new(), |mut graph, module| {
            graph.add_module(module);
            graph
        })
}

/// Extract module name from a dependency string
fn extract_module_from_dependency(dep_name: &str) -> Option<String> {
    // Simple heuristic: take the first part of a path-like dependency
    dep_name.split("::").next().map(|s| s.to_string())
}

/// Analyze module dependencies from file paths and imports
pub fn analyze_module_dependencies(
    files: &[(std::path::PathBuf, Vec<Dependency>)],
) -> DependencyGraph {
    files.iter().fold(
        DependencyGraph::new(),
        |mut graph, (file_path, file_deps)| {
            let module_name = extract_module_name(file_path);
            graph.add_module(module_name.clone());

            file_deps
                .iter()
                .filter_map(|dep| extract_module_from_dependency(&dep.name))
                .filter(|dep_module| *dep_module != module_name)
                .for_each(|dep_module| {
                    graph.add_dependency(module_name.clone(), dep_module);
                });

            graph
        },
    )
}

/// Extract module name from file path
fn extract_module_name(path: &std::path::Path) -> String {
    path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_circular_dependency_detection() {
        let mut graph = DependencyGraph::new();

        // Create a circular dependency: A -> B -> C -> A
        graph.add_dependency("A".to_string(), "B".to_string());
        graph.add_dependency("B".to_string(), "C".to_string());
        graph.add_dependency("C".to_string(), "A".to_string());

        let circular = graph.detect_circular_dependencies();
        assert_eq!(circular.len(), 1);
        assert_eq!(circular[0].cycle.len(), 3);
    }

    #[test]
    fn test_self_dependency() {
        let mut graph = DependencyGraph::new();

        // Create a self-dependency: A -> A
        graph.add_dependency("A".to_string(), "A".to_string());

        let circular = graph.detect_circular_dependencies();
        assert_eq!(circular.len(), 1);
        assert_eq!(circular[0].cycle.len(), 1);
        assert_eq!(circular[0].cycle[0], "A");
    }

    #[test]
    fn test_coupling_metrics() {
        let mut graph = DependencyGraph::new();

        // A depends on B and C
        // B depends on C
        // D depends on A
        graph.add_dependency("A".to_string(), "B".to_string());
        graph.add_dependency("A".to_string(), "C".to_string());
        graph.add_dependency("B".to_string(), "C".to_string());
        graph.add_dependency("D".to_string(), "A".to_string());

        let metrics = graph.calculate_coupling_metrics();

        // Find module A's metrics
        let a_metrics = metrics.iter().find(|m| m.module == "A").unwrap();
        assert_eq!(a_metrics.dependencies.len(), 2); // Depends on B and C
        assert_eq!(a_metrics.dependents.len(), 1); // D depends on A
    }

    /// Stress test: large dependency chain should not stack overflow
    #[test]
    fn test_large_dependency_chain_no_stack_overflow() {
        let mut graph = DependencyGraph::new();

        // Create a deep chain: mod_0 -> mod_1 -> mod_2 -> ... -> mod_3999
        let num_modules = 4000;
        for i in 0..num_modules - 1 {
            graph.add_dependency(format!("mod_{}", i), format!("mod_{}", i + 1));
        }

        // Should complete without stack overflow
        let circular = graph.detect_circular_dependencies();
        assert!(circular.is_empty(), "Linear chain should have no cycles");
    }

    /// Stress test: large graph with cycle should detect it without stack overflow
    #[test]
    fn test_large_graph_with_cycle_no_stack_overflow() {
        let mut graph = DependencyGraph::new();

        // Create a deep chain with a cycle at the end
        let num_modules = 4000;
        for i in 0..num_modules - 1 {
            graph.add_dependency(format!("mod_{}", i), format!("mod_{}", i + 1));
        }
        // Add cycle: last module points back to first
        graph.add_dependency(format!("mod_{}", num_modules - 1), "mod_0".to_string());

        // Should detect the cycle without stack overflow
        let circular = graph.detect_circular_dependencies();
        assert_eq!(circular.len(), 1, "Should detect exactly one cycle");
        assert_eq!(
            circular[0].cycle.len(),
            num_modules,
            "Cycle should include all modules"
        );
    }

    /// Stress test: wide graph with many independent chains
    #[test]
    fn test_wide_graph_no_stack_overflow() {
        let mut graph = DependencyGraph::new();

        // Create 100 independent chains of length 100 each
        for chain in 0..100 {
            for i in 0..99 {
                graph.add_dependency(
                    format!("chain_{}_mod_{}", chain, i),
                    format!("chain_{}_mod_{}", chain, i + 1),
                );
            }
        }

        // Should complete without stack overflow
        let circular = graph.detect_circular_dependencies();
        assert!(circular.is_empty(), "Independent chains should have no cycles");
    }
}
