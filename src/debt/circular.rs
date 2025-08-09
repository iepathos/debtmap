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

    /// Detect circular dependencies using DFS
    pub fn detect_circular_dependencies(&self) -> Vec<CircularDependency> {
        let mut visited: HashMap<String, bool> =
            self.modules.iter().map(|m| (m.clone(), false)).collect();
        let mut rec_stack: HashMap<String, bool> =
            self.modules.iter().map(|m| (m.clone(), false)).collect();
        let mut path = Vec::new();

        let mut circular_deps = Vec::new();

        for module in &self.modules {
            if !visited[module] {
                self.dfs_detect_cycles(
                    module,
                    &mut visited,
                    &mut rec_stack,
                    &mut path,
                    &mut circular_deps,
                );
            }
        }

        circular_deps
    }

    fn dfs_detect_cycles(
        &self,
        module: &str,
        visited: &mut HashMap<String, bool>,
        rec_stack: &mut HashMap<String, bool>,
        path: &mut Vec<String>,
        cycles: &mut Vec<CircularDependency>,
    ) {
        visited.insert(module.to_string(), true);
        rec_stack.insert(module.to_string(), true);
        path.push(module.to_string());

        if let Some(deps) = self.adjacency.get(module) {
            for dep in deps {
                if !visited[dep] {
                    self.dfs_detect_cycles(dep, visited, rec_stack, path, cycles);
                } else if rec_stack[dep] {
                    // Found a cycle
                    let cycle_start = path.iter().position(|m| m == dep).unwrap();
                    let cycle = path[cycle_start..].to_vec();
                    cycles.push(CircularDependency { cycle });
                }
            }
        }

        path.pop();
        rec_stack.insert(module.to_string(), false);
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
}
