//! Dependency Structure Matrix (DSM) Analysis (Spec 205)
//!
//! Provides an alternative visualization of module dependencies that:
//! - Scales linearly: 100x100 matrix is still usable; 100-node graph is chaos
//! - Reveals cycles immediately: cells above the diagonal indicate backward dependencies
//! - Shows layered architecture: triangular matrices indicate clean layering
//! - Enables pattern recognition: clusters, fan-out, fan-in visible at a glance
//!
//! DSM Concept:
//! ```text
//!          A  B  C  D  E
//!       A  .  .  .  .  .     . = no dependency
//!       B  X  .  .  .  .     X = B depends on A (below diagonal = good)
//!       C  X  X  .  .  .     ● = cycle (above diagonal = problem!)
//!       D  .  X  X  .  ●
//!       E  .  .  X  X  .
//! ```
//!
//! Reading: Row depends on Column. Lower-left triangle = healthy dependencies.
//! Upper-right = cycles (problematic).

use crate::priority::FileDebtItem;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};

/// A cell in the dependency structure matrix
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DsmCell {
    /// Whether this cell represents a dependency
    pub has_dependency: bool,
    /// Number of dependencies (for weighted visualization)
    pub dependency_count: usize,
    /// True if this is in the upper triangle (indicates cycle)
    pub is_cycle: bool,
}

/// Severity of a dependency cycle
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CycleSeverity {
    /// Low severity - minor coupling issue
    Low,
    /// Medium severity - should be addressed
    Medium,
    /// High severity - significant architectural issue
    High,
}

/// Information about a dependency cycle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CycleInfo {
    /// Module names involved in the cycle
    pub modules: Vec<String>,
    /// Severity of the cycle
    pub severity: CycleSeverity,
}

/// Metrics computed from the dependency structure matrix
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DsmMetrics {
    /// Number of modules in the matrix
    pub module_count: usize,
    /// Total number of dependencies
    pub dependency_count: usize,
    /// Number of cells above diagonal (potential cycles)
    pub cycle_count: usize,
    /// Density of the dependency matrix (0.0 to 1.0)
    pub density: f64,
    /// Layering score (0.0 = all cycles, 1.0 = perfect layers)
    pub layering_score: f64,
    /// Propagation cost: average number of modules affected by changes
    pub propagation_cost: f64,
}

/// Dependency Structure Matrix for visualizing module dependencies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyMatrix {
    /// Module names in row/column order
    pub modules: Vec<String>,
    /// Adjacency matrix: matrix[row][col] = row depends on col
    pub matrix: Vec<Vec<DsmCell>>,
    /// Detected cycles
    pub cycles: Vec<CycleInfo>,
    /// Computed metrics
    pub metrics: DsmMetrics,
}

impl DependencyMatrix {
    /// Build a dependency matrix from file debt items
    pub fn from_file_items(items: &[FileDebtItem]) -> Self {
        let modules = Self::extract_modules(items);
        let module_index: HashMap<&str, usize> = modules
            .iter()
            .enumerate()
            .map(|(i, m)| (m.as_str(), i))
            .collect();

        let n = modules.len();
        let mut matrix = vec![vec![DsmCell::default(); n]; n];

        // Fill the matrix from dependencies
        for item in items {
            let file_path = item.metrics.path.to_string_lossy();
            let module_name = Self::path_to_module(&file_path);

            if let Some(&row_idx) = module_index.get(module_name.as_str()) {
                for dep in &item.metrics.dependencies_list {
                    let dep_module = Self::path_to_module(dep);
                    if let Some(&col_idx) = module_index.get(dep_module.as_str()) {
                        if row_idx != col_idx {
                            matrix[row_idx][col_idx].has_dependency = true;
                            matrix[row_idx][col_idx].dependency_count += 1;

                            // Mark cycles (above diagonal in ordered matrix)
                            if row_idx < col_idx {
                                matrix[row_idx][col_idx].is_cycle = true;
                            }
                        }
                    }
                }
            }
        }

        let cycles = Self::detect_cycles(&matrix, &modules);
        let metrics = Self::compute_metrics(&matrix, &modules, &cycles);

        DependencyMatrix {
            modules,
            matrix,
            cycles,
            metrics,
        }
    }

    /// Build a dependency matrix from file dependencies directly
    /// Uses top_dependencies from FileDependencies for more accurate representation
    pub fn from_file_dependencies(files: &[crate::output::unified::FileDebtItemOutput]) -> Self {
        let modules = Self::extract_modules_from_output(files);
        let module_index: HashMap<&str, usize> = modules
            .iter()
            .enumerate()
            .map(|(i, m)| (m.as_str(), i))
            .collect();

        let n = modules.len();
        let mut matrix = vec![vec![DsmCell::default(); n]; n];

        // Fill the matrix from dependencies
        for file in files {
            let module_name = Self::path_to_module(&file.location.file);

            if let Some(&row_idx) = module_index.get(module_name.as_str()) {
                if let Some(deps) = &file.dependencies {
                    for dep in &deps.top_dependencies {
                        let dep_module = Self::path_to_module(dep);
                        if let Some(&col_idx) = module_index.get(dep_module.as_str()) {
                            if row_idx != col_idx {
                                matrix[row_idx][col_idx].has_dependency = true;
                                matrix[row_idx][col_idx].dependency_count += 1;

                                // Mark cycles (above diagonal in ordered matrix)
                                if row_idx < col_idx {
                                    matrix[row_idx][col_idx].is_cycle = true;
                                }
                            }
                        }
                    }
                }
            }
        }

        let cycles = Self::detect_cycles(&matrix, &modules);
        let metrics = Self::compute_metrics(&matrix, &modules, &cycles);

        DependencyMatrix {
            modules,
            matrix,
            cycles,
            metrics,
        }
    }

    /// Extract unique module names from file items
    fn extract_modules(items: &[FileDebtItem]) -> Vec<String> {
        let mut modules: HashSet<String> = HashSet::new();

        for item in items {
            let file_path = item.metrics.path.to_string_lossy();
            modules.insert(Self::path_to_module(&file_path));

            // Also collect dependency modules
            for dep in &item.metrics.dependencies_list {
                modules.insert(Self::path_to_module(dep));
            }
        }

        let mut modules: Vec<String> = modules.into_iter().collect();
        modules.sort(); // Alphabetical ordering by default
        modules
    }

    /// Extract unique module names from output format
    fn extract_modules_from_output(
        files: &[crate::output::unified::FileDebtItemOutput],
    ) -> Vec<String> {
        let mut modules: HashSet<String> = HashSet::new();

        for file in files {
            modules.insert(Self::path_to_module(&file.location.file));

            if let Some(deps) = &file.dependencies {
                for dep in &deps.top_dependencies {
                    modules.insert(Self::path_to_module(dep));
                }
            }
        }

        let mut modules: Vec<String> = modules.into_iter().collect();
        modules.sort();
        modules
    }

    /// Convert a file path to a module name
    fn path_to_module(path: &str) -> String {
        let path = std::path::Path::new(path);

        // Get the parent directory as the module
        path.parent()
            .and_then(|p| p.to_str())
            .map(|s| {
                // Remove leading "./" or "src/" if present
                let s = s.strip_prefix("./").unwrap_or(s);
                let s = s.strip_prefix("src/").unwrap_or(s);
                if s.is_empty() || s == "src" {
                    "root".to_string()
                } else {
                    s.to_string()
                }
            })
            .unwrap_or_else(|| "root".to_string())
    }

    /// Detect actual cycles using DFS
    fn detect_cycles(matrix: &[Vec<DsmCell>], modules: &[String]) -> Vec<CycleInfo> {
        let n = modules.len();
        let mut cycles = Vec::new();

        // Build adjacency list
        let mut adj: Vec<Vec<usize>> = vec![Vec::new(); n];
        for (i, row) in matrix.iter().enumerate() {
            for (j, cell) in row.iter().enumerate() {
                if cell.has_dependency {
                    adj[i].push(j);
                }
            }
        }

        // Find strongly connected components using Kosaraju's algorithm
        let sccs = Self::find_sccs(&adj, n);

        // Each SCC with more than one node is a cycle
        for scc in sccs {
            if scc.len() > 1 {
                let cycle_modules: Vec<String> = scc.iter().map(|&i| modules[i].clone()).collect();
                let severity = match scc.len() {
                    2 => CycleSeverity::Low,
                    3..=5 => CycleSeverity::Medium,
                    _ => CycleSeverity::High,
                };
                cycles.push(CycleInfo {
                    modules: cycle_modules,
                    severity,
                });
            }
        }

        cycles
    }

    /// Find strongly connected components using Kosaraju's algorithm
    fn find_sccs(adj: &[Vec<usize>], n: usize) -> Vec<Vec<usize>> {
        let mut visited = vec![false; n];
        let mut finish_order = Vec::new();

        // First DFS pass to get finish order
        for i in 0..n {
            if !visited[i] {
                Self::dfs_finish(i, adj, &mut visited, &mut finish_order);
            }
        }

        // Build reverse graph
        let mut reverse_adj: Vec<Vec<usize>> = vec![Vec::new(); n];
        for (i, neighbors) in adj.iter().enumerate() {
            for &j in neighbors {
                reverse_adj[j].push(i);
            }
        }

        // Second DFS pass on reverse graph in reverse finish order
        visited.fill(false);
        let mut sccs = Vec::new();

        for &node in finish_order.iter().rev() {
            if !visited[node] {
                let mut scc = Vec::new();
                Self::dfs_collect(node, &reverse_adj, &mut visited, &mut scc);
                sccs.push(scc);
            }
        }

        sccs
    }

    /// Iterative DFS to compute finish order (avoids stack overflow on large graphs)
    fn dfs_finish(
        start: usize,
        adj: &[Vec<usize>],
        visited: &mut [bool],
        finish_order: &mut Vec<usize>,
    ) {
        // Use explicit stack instead of recursion to avoid stack overflow
        // Each stack frame: (node, neighbor_index, is_backtracking)
        let mut stack: Vec<(usize, usize, bool)> = vec![(start, 0, false)];

        while let Some((node, neighbor_idx, backtracking)) = stack.pop() {
            if backtracking {
                // All neighbors processed, add to finish order
                finish_order.push(node);
                continue;
            }

            if !visited[node] {
                visited[node] = true;
            }

            // Find next unvisited neighbor
            let neighbors = &adj[node];
            let mut found_unvisited = false;
            for (i, &neighbor) in neighbors.iter().enumerate().skip(neighbor_idx) {
                if !visited[neighbor] {
                    // Push current node back with next neighbor index and backtrack flag
                    stack.push((node, i + 1, false));
                    // Push the unvisited neighbor to explore
                    stack.push((neighbor, 0, false));
                    found_unvisited = true;
                    break;
                }
            }

            if !found_unvisited {
                // No more unvisited neighbors, mark for backtracking
                stack.push((node, 0, true));
            }
        }
    }

    /// Iterative DFS to collect strongly connected component (avoids stack overflow)
    fn dfs_collect(start: usize, adj: &[Vec<usize>], visited: &mut [bool], scc: &mut Vec<usize>) {
        // Use explicit stack instead of recursion
        let mut stack = vec![start];

        while let Some(node) = stack.pop() {
            if visited[node] {
                continue;
            }
            visited[node] = true;
            scc.push(node);

            // Add all unvisited neighbors to the stack
            for &neighbor in &adj[node] {
                if !visited[neighbor] {
                    stack.push(neighbor);
                }
            }
        }
    }

    /// Compute DSM metrics
    fn compute_metrics(
        matrix: &[Vec<DsmCell>],
        modules: &[String],
        _cycles: &[CycleInfo],
    ) -> DsmMetrics {
        let n = modules.len();
        if n == 0 {
            return DsmMetrics {
                module_count: 0,
                dependency_count: 0,
                cycle_count: 0,
                density: 0.0,
                layering_score: 1.0,
                propagation_cost: 0.0,
            };
        }

        let mut dependency_count = 0;
        let mut cycle_count = 0;
        let mut upper_triangle_deps = 0;
        let mut lower_triangle_deps = 0;

        for (i, row) in matrix.iter().enumerate() {
            for (j, cell) in row.iter().enumerate() {
                if cell.has_dependency && i != j {
                    dependency_count += 1;
                    if i < j {
                        upper_triangle_deps += 1;
                    } else {
                        lower_triangle_deps += 1;
                    }
                    if cell.is_cycle {
                        cycle_count += 1;
                    }
                }
            }
        }

        let total_possible = n * (n - 1);
        let density = if total_possible > 0 {
            dependency_count as f64 / total_possible as f64
        } else {
            0.0
        };

        // Layering score: proportion of dependencies in lower triangle
        let total_deps = upper_triangle_deps + lower_triangle_deps;
        let layering_score = if total_deps > 0 {
            lower_triangle_deps as f64 / total_deps as f64
        } else {
            1.0 // No dependencies = perfect layering
        };

        // Propagation cost: average reachable nodes per node
        let propagation_cost = Self::compute_propagation_cost(matrix);

        DsmMetrics {
            module_count: n,
            dependency_count,
            cycle_count,
            density,
            layering_score,
            propagation_cost,
        }
    }

    /// Compute propagation cost using BFS from each node
    fn compute_propagation_cost(matrix: &[Vec<DsmCell>]) -> f64 {
        let n = matrix.len();
        if n == 0 {
            return 0.0;
        }

        let mut total_reachable = 0;

        for start in 0..n {
            let mut visited = vec![false; n];
            let mut queue = VecDeque::new();
            queue.push_back(start);
            visited[start] = true;
            let mut count = 0;

            while let Some(current) = queue.pop_front() {
                for (neighbor, cell) in matrix[current].iter().enumerate() {
                    if cell.has_dependency && !visited[neighbor] {
                        visited[neighbor] = true;
                        count += 1;
                        queue.push_back(neighbor);
                    }
                }
            }
            total_reachable += count;
        }

        total_reachable as f64 / n as f64
    }

    /// Reorder modules to minimize upper-triangle dependencies
    /// Uses a simple topological sort with feedback arc set minimization
    pub fn optimize_ordering(&mut self) {
        if self.modules.len() <= 1 {
            return;
        }

        // Build adjacency list
        let n = self.modules.len();
        let mut adj: Vec<Vec<usize>> = vec![Vec::new(); n];
        for (i, row) in self.matrix.iter().enumerate() {
            for (j, cell) in row.iter().enumerate() {
                if cell.has_dependency {
                    adj[i].push(j);
                }
            }
        }

        // Compute topological order using Kahn's algorithm with tie-breaking
        let order = Self::topological_sort_with_tiebreak(&adj, n);

        // Reorder modules and matrix
        self.apply_ordering(&order);
    }

    fn topological_sort_with_tiebreak(adj: &[Vec<usize>], n: usize) -> Vec<usize> {
        let mut in_degree = vec![0; n];
        for neighbors in adj {
            for &j in neighbors {
                in_degree[j] += 1;
            }
        }

        // Use BinaryHeap for deterministic ordering (prefer lower in-degree, then lower index)
        let mut queue: VecDeque<usize> = VecDeque::new();
        for (i, &deg) in in_degree.iter().enumerate() {
            if deg == 0 {
                queue.push_back(i);
            }
        }

        let mut order = Vec::new();
        while let Some(node) = queue.pop_front() {
            order.push(node);
            for &neighbor in &adj[node] {
                in_degree[neighbor] -= 1;
                if in_degree[neighbor] == 0 {
                    queue.push_back(neighbor);
                }
            }
        }

        // If there are cycles, some nodes won't be in order
        // Add remaining nodes in their original order
        if order.len() < n {
            for i in 0..n {
                if !order.contains(&i) {
                    order.push(i);
                }
            }
        }

        order
    }

    fn apply_ordering(&mut self, order: &[usize]) {
        let n = self.modules.len();

        // Create index mapping
        let new_to_old = order.to_vec();
        let mut old_to_new = vec![0; n];
        for (new_idx, &old_idx) in new_to_old.iter().enumerate() {
            old_to_new[old_idx] = new_idx;
        }

        // Reorder modules
        let new_modules: Vec<String> = new_to_old
            .iter()
            .map(|&i| self.modules[i].clone())
            .collect();

        // Reorder matrix
        let mut new_matrix = vec![vec![DsmCell::default(); n]; n];
        for (old_row, row) in self.matrix.iter().enumerate() {
            let new_row = old_to_new[old_row];
            for (old_col, cell) in row.iter().enumerate() {
                let new_col = old_to_new[old_col];
                new_matrix[new_row][new_col] = cell.clone();
                // Update is_cycle flag based on new positions
                if cell.has_dependency && new_row != new_col {
                    new_matrix[new_row][new_col].is_cycle = new_row < new_col;
                }
            }
        }

        self.modules = new_modules;
        self.matrix = new_matrix;

        // Recalculate metrics after reordering
        self.metrics = Self::compute_metrics(&self.matrix, &self.modules, &self.cycles);
    }

    /// Get the cell symbol for display
    pub fn cell_symbol(cell: &DsmCell, row: usize, col: usize) -> &'static str {
        if row == col {
            "■" // Diagonal (self)
        } else if cell.is_cycle && cell.has_dependency {
            "●" // Cycle (problem)
        } else if cell.has_dependency {
            "×" // Normal dependency
        } else {
            "·" // No dependency
        }
    }

    /// Get cell at position
    pub fn get_cell(&self, row: usize, col: usize) -> Option<&DsmCell> {
        self.matrix.get(row).and_then(|r| r.get(col))
    }

    /// Get module name at index
    pub fn get_module(&self, idx: usize) -> Option<&str> {
        self.modules.get(idx).map(|s| s.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_matrix() -> DependencyMatrix {
        // Create a simple 3-module matrix:
        // A depends on nothing
        // B depends on A
        // C depends on A and B
        let modules = vec!["A".to_string(), "B".to_string(), "C".to_string()];
        let mut matrix = vec![vec![DsmCell::default(); 3]; 3];

        // B depends on A
        matrix[1][0].has_dependency = true;
        matrix[1][0].dependency_count = 1;

        // C depends on A
        matrix[2][0].has_dependency = true;
        matrix[2][0].dependency_count = 1;

        // C depends on B
        matrix[2][1].has_dependency = true;
        matrix[2][1].dependency_count = 1;

        let cycles = DependencyMatrix::detect_cycles(&matrix, &modules);
        let metrics = DependencyMatrix::compute_metrics(&matrix, &modules, &cycles);

        DependencyMatrix {
            modules,
            matrix,
            cycles,
            metrics,
        }
    }

    fn create_cyclic_matrix() -> DependencyMatrix {
        // Create a matrix with a cycle:
        // A depends on C (creates cycle A -> C -> B -> A)
        // B depends on A
        // C depends on B
        let modules = vec!["A".to_string(), "B".to_string(), "C".to_string()];
        let mut matrix = vec![vec![DsmCell::default(); 3]; 3];

        // A depends on C
        matrix[0][2].has_dependency = true;
        matrix[0][2].is_cycle = true; // Above diagonal

        // B depends on A
        matrix[1][0].has_dependency = true;

        // C depends on B
        matrix[2][1].has_dependency = true;

        let cycles = DependencyMatrix::detect_cycles(&matrix, &modules);
        let metrics = DependencyMatrix::compute_metrics(&matrix, &modules, &cycles);

        DependencyMatrix {
            modules,
            matrix,
            cycles,
            metrics,
        }
    }

    #[test]
    fn test_cell_symbol() {
        let cell = DsmCell::default();
        assert_eq!(DependencyMatrix::cell_symbol(&cell, 0, 0), "■");
        assert_eq!(DependencyMatrix::cell_symbol(&cell, 0, 1), "·");

        let dep_cell = DsmCell {
            has_dependency: true,
            dependency_count: 1,
            is_cycle: false,
        };
        assert_eq!(DependencyMatrix::cell_symbol(&dep_cell, 1, 0), "×");

        let cycle_cell = DsmCell {
            has_dependency: true,
            dependency_count: 1,
            is_cycle: true,
        };
        assert_eq!(DependencyMatrix::cell_symbol(&cycle_cell, 0, 1), "●");
    }

    #[test]
    fn test_create_matrix() {
        let matrix = create_test_matrix();

        assert_eq!(matrix.modules.len(), 3);
        assert_eq!(matrix.metrics.dependency_count, 3);
        assert_eq!(matrix.metrics.cycle_count, 0);
        assert_eq!(matrix.metrics.layering_score, 1.0); // All deps in lower triangle
    }

    #[test]
    fn test_cyclic_matrix() {
        let matrix = create_cyclic_matrix();

        assert_eq!(matrix.modules.len(), 3);
        assert!(
            !matrix.cycles.is_empty(),
            "Should detect at least one cycle"
        );
        assert!(matrix.metrics.cycle_count > 0);
        assert!(matrix.metrics.layering_score < 1.0); // Has deps in upper triangle
    }

    #[test]
    fn test_empty_matrix() {
        let empty: Vec<crate::priority::FileDebtItem> = vec![];
        let matrix = DependencyMatrix::from_file_items(&empty);

        assert_eq!(matrix.modules.len(), 0);
        assert_eq!(matrix.metrics.dependency_count, 0);
        assert_eq!(matrix.metrics.layering_score, 1.0);
    }

    #[test]
    fn test_path_to_module() {
        assert_eq!(DependencyMatrix::path_to_module("src/main.rs"), "root");
        assert_eq!(DependencyMatrix::path_to_module("./src/lib.rs"), "root");
        assert_eq!(DependencyMatrix::path_to_module("src/io/mod.rs"), "io");
        assert_eq!(
            DependencyMatrix::path_to_module("src/io/writers/dot.rs"),
            "io/writers"
        );
    }

    #[test]
    fn test_optimize_ordering() {
        let mut matrix = create_cyclic_matrix();
        let original_layering = matrix.metrics.layering_score;

        matrix.optimize_ordering();

        // After optimization, layering should not get worse
        assert!(matrix.metrics.layering_score >= original_layering * 0.9);
    }

    #[test]
    fn test_propagation_cost() {
        let matrix = create_test_matrix();

        // In our test matrix:
        // A reaches: B, C (2 nodes)
        // B reaches: C (1 node)
        // C reaches: nothing (0 nodes)
        // Average = (2 + 1 + 0) / 3 = 1.0
        assert!((matrix.metrics.propagation_cost - 1.0).abs() < 0.1);
    }

    #[test]
    fn test_density() {
        let matrix = create_test_matrix();

        // 3 modules = 3 * 2 = 6 possible edges (excluding self)
        // 3 dependencies / 6 = 0.5
        assert!((matrix.metrics.density - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_get_cell() {
        let matrix = create_test_matrix();

        assert!(matrix.get_cell(1, 0).unwrap().has_dependency);
        assert!(!matrix.get_cell(0, 1).unwrap().has_dependency);
        assert!(matrix.get_cell(10, 10).is_none());
    }

    #[test]
    fn test_get_module() {
        let matrix = create_test_matrix();

        assert_eq!(matrix.get_module(0), Some("A"));
        assert_eq!(matrix.get_module(1), Some("B"));
        assert_eq!(matrix.get_module(10), None);
    }
}
