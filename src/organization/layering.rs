//! # Module Layering Analysis
//!
//! Computes layering score for module dependencies to detect architectural violations.
//!
//! ## Stillwater Architecture
//!
//! This module is part of the **Pure Core** - deterministic functions with no side effects.
//! All functions are:
//! - Deterministic: Same inputs → same outputs
//! - Side-effect free: No I/O, no mutations
//! - Composable: Can be chained together
//! - 100% testable: No mocks needed
//!
//! ## Layering Concept
//!
//! A well-layered architecture has dependencies flowing in one direction:
//! - UI → Domain → Infrastructure (good, forward dependencies)
//! - Infrastructure → Domain (bad, backward dependency)
//!
//! God objects typically break layering by depending on everything and having
//! everything depend on them.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Dependency between two modules
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ModuleDependency {
    /// Module that has the dependency
    pub from_module: String,
    /// Module being depended upon
    pub to_module: String,
}

impl ModuleDependency {
    pub fn new(from: impl Into<String>, to: impl Into<String>) -> Self {
        Self {
            from_module: from.into(),
            to_module: to.into(),
        }
    }
}

/// Result of layering analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayeringAnalysis {
    /// Score from 0.0 (all backward deps) to 1.0 (perfect layers)
    pub score: f64,
    /// Number of backward dependencies (violations)
    pub backward_dep_count: usize,
    /// Total number of dependencies analyzed
    pub total_dep_count: usize,
    /// Modules causing the most backward deps, sorted by count
    pub problematic_modules: Vec<(String, usize)>,
}

impl Default for LayeringAnalysis {
    fn default() -> Self {
        Self {
            score: 1.0,
            backward_dep_count: 0,
            total_dep_count: 0,
            problematic_modules: vec![],
        }
    }
}

/// Impact of a god object on module layering
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LayeringImpact {
    /// Number of backward dependencies caused by this god object
    pub backward_deps_caused: usize,
    /// Modules affected by the backward dependencies
    pub affected_modules: Vec<String>,
    /// Estimated layering score improvement after refactoring
    pub estimated_improvement: f64,
    /// Current layering score of the module containing this god object
    pub current_score: f64,
}

impl Default for LayeringImpact {
    fn default() -> Self {
        Self {
            backward_deps_caused: 0,
            affected_modules: vec![],
            estimated_improvement: 0.0,
            current_score: 1.0,
        }
    }
}

/// Compute layering score from module dependencies.
///
/// A well-layered architecture has dependencies flowing one direction.
/// Backward dependencies (lower-level depending on higher-level) indicate
/// architectural problems.
///
/// # Algorithm
///
/// 1. Build module dependency graph
/// 2. Compute topological ordering (or best-effort if cycles)
/// 3. Count deps in lower triangle (forward) vs upper triangle (backward)
/// 4. Return score = forward_deps / total_deps
///
/// # Arguments
///
/// * `dependencies` - List of module dependencies
///
/// # Returns
///
/// LayeringAnalysis with score and backward dependency details
pub fn compute_layering_score(dependencies: &[ModuleDependency]) -> LayeringAnalysis {
    if dependencies.is_empty() {
        return LayeringAnalysis::default();
    }

    // Collect unique modules
    let mut modules: HashSet<&str> = HashSet::new();
    for dep in dependencies {
        modules.insert(&dep.from_module);
        modules.insert(&dep.to_module);
    }

    // Create sorted module list for consistent ordering
    let mut module_list: Vec<&str> = modules.into_iter().collect();
    module_list.sort();

    // Create module index map
    let module_index: HashMap<&str, usize> = module_list
        .iter()
        .enumerate()
        .map(|(i, m)| (*m, i))
        .collect();

    // Attempt topological sort to get optimal ordering
    let ordering = compute_topological_order(dependencies, &module_list, &module_index);

    // Create position map from ordering
    let position: HashMap<&str, usize> = ordering
        .iter()
        .enumerate()
        .map(|(pos, &module)| (module, pos))
        .collect();

    // Count forward and backward dependencies
    let mut forward_deps = 0;
    let mut backward_deps = 0;
    let mut backward_by_module: HashMap<&str, usize> = HashMap::new();

    for dep in dependencies {
        if dep.from_module == dep.to_module {
            continue; // Skip self-dependencies
        }

        let from_pos = position.get(dep.from_module.as_str()).copied().unwrap_or(0);
        let to_pos = position.get(dep.to_module.as_str()).copied().unwrap_or(0);

        if from_pos > to_pos {
            // Forward dependency: from depends on something before it
            forward_deps += 1;
        } else {
            // Backward dependency: from depends on something after it
            backward_deps += 1;
            *backward_by_module.entry(&dep.from_module).or_insert(0) += 1;
        }
    }

    let total_deps = forward_deps + backward_deps;
    let score = if total_deps > 0 {
        forward_deps as f64 / total_deps as f64
    } else {
        1.0
    };

    // Sort problematic modules by backward dep count (descending)
    let mut problematic_modules: Vec<(String, usize)> = backward_by_module
        .into_iter()
        .map(|(m, c)| (m.to_string(), c))
        .collect();
    problematic_modules.sort_by(|a, b| b.1.cmp(&a.1));

    LayeringAnalysis {
        score,
        backward_dep_count: backward_deps,
        total_dep_count: total_deps,
        problematic_modules,
    }
}

/// Compute topological order for modules.
///
/// Uses Kahn's algorithm with alphabetical tie-breaking.
/// Falls back to alphabetical order if cycles prevent full ordering.
fn compute_topological_order<'a>(
    dependencies: &[ModuleDependency],
    modules: &[&'a str],
    module_index: &HashMap<&str, usize>,
) -> Vec<&'a str> {
    let n = modules.len();
    let mut in_degree = vec![0usize; n];
    let mut adj: Vec<Vec<usize>> = vec![Vec::new(); n];

    // Build graph: edge from to -> from (dependency direction)
    for dep in dependencies {
        if dep.from_module == dep.to_module {
            continue;
        }
        if let (Some(&from_idx), Some(&to_idx)) = (
            module_index.get(dep.from_module.as_str()),
            module_index.get(dep.to_module.as_str()),
        ) {
            adj[to_idx].push(from_idx);
            in_degree[from_idx] += 1;
        }
    }

    // Kahn's algorithm
    let mut queue: Vec<usize> = (0..n).filter(|&i| in_degree[i] == 0).collect();
    queue.sort(); // Alphabetical tie-breaking

    let mut order = Vec::with_capacity(n);

    while let Some(node) = queue.pop() {
        order.push(modules[node]);
        for &neighbor in &adj[node] {
            in_degree[neighbor] -= 1;
            if in_degree[neighbor] == 0 {
                // Insert in sorted position for alphabetical tie-breaking
                let pos = queue.partition_point(|&x| x < neighbor);
                queue.insert(pos, neighbor);
            }
        }
    }

    // If cycles exist, add remaining modules alphabetically
    if order.len() < n {
        for (i, &module) in modules.iter().enumerate() {
            if !order.contains(&module) {
                order.push(modules[i]);
            }
        }
    }

    order
}

/// Extract module name from file path.
///
/// Converts a file path to a module name by taking the parent directory
/// and stripping common prefixes like "src/".
///
/// # Examples
///
/// - `"src/analysis/dsm.rs"` → `"analysis"`
/// - `"src/cli/args.rs"` → `"cli"`
/// - `"src/main.rs"` → `"root"`
pub fn path_to_module(path: &str) -> String {
    let path = std::path::Path::new(path);

    path.parent()
        .and_then(|p| p.to_str())
        .map(|s| {
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

/// Compute layering impact for a specific file/module.
///
/// Determines how many backward dependencies a specific module causes
/// and estimates the improvement if it were refactored.
///
/// # Arguments
///
/// * `module` - The module to analyze
/// * `dependencies` - All module dependencies
///
/// # Returns
///
/// LayeringImpact with details about backward deps caused
pub fn compute_layering_impact(module: &str, dependencies: &[ModuleDependency]) -> LayeringImpact {
    // Filter dependencies related to this module
    let module_deps: Vec<_> = dependencies
        .iter()
        .filter(|d| d.from_module == module || d.to_module == module)
        .cloned()
        .collect();

    if module_deps.is_empty() {
        return LayeringImpact::default();
    }

    // Compute current layering score for the full graph
    let current_analysis = compute_layering_score(dependencies);

    // Compute layering score without this module's dependencies
    let deps_without_module: Vec<_> = dependencies
        .iter()
        .filter(|d| d.from_module != module && d.to_module != module)
        .cloned()
        .collect();

    let analysis_without = compute_layering_score(&deps_without_module);

    // Find backward deps caused by this module
    let backward_caused: Vec<_> = current_analysis
        .problematic_modules
        .iter()
        .filter(|(m, _)| m == module)
        .cloned()
        .collect();

    let backward_count = backward_caused.first().map(|(_, c)| *c).unwrap_or(0);

    // Find affected modules (modules that depend on or are depended upon)
    let affected: Vec<String> = module_deps
        .iter()
        .flat_map(|d| {
            if d.from_module == module {
                Some(d.to_module.clone())
            } else {
                Some(d.from_module.clone())
            }
        })
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();

    LayeringImpact {
        backward_deps_caused: backward_count,
        affected_modules: affected,
        estimated_improvement: analysis_without.score - current_analysis.score,
        current_score: current_analysis.score,
    }
}

/// Calculate layering penalty for god object scoring.
///
/// Returns a multiplier (0.0 to 0.20) based on how many backward
/// dependencies the module causes. Used to boost priority of god
/// objects that cause architectural damage.
///
/// # Returns
///
/// - 0.0 if backward_deps < 3
/// - 0.15 if backward_deps >= 3 (15% priority boost)
/// - 0.20 if backward_deps >= 5 (20% priority boost)
pub fn calculate_layering_penalty(backward_deps: usize) -> f64 {
    match backward_deps {
        0..=2 => 0.0,
        3..=4 => 0.15,
        _ => 0.20,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_perfect_layering_returns_1() {
        // A -> B -> C (perfect layering, all forward deps)
        let deps = vec![
            ModuleDependency::new("b", "a"),
            ModuleDependency::new("c", "a"),
            ModuleDependency::new("c", "b"),
        ];
        let result = compute_layering_score(&deps);
        assert_eq!(result.score, 1.0);
        assert_eq!(result.backward_dep_count, 0);
    }

    #[test]
    fn test_cyclic_deps_returns_low_score() {
        // A -> B -> C -> A (cycle creates backward dependencies)
        let deps = vec![
            ModuleDependency::new("a", "b"),
            ModuleDependency::new("b", "c"),
            ModuleDependency::new("c", "a"), // This creates a cycle
        ];
        let result = compute_layering_score(&deps);
        // With a cycle, there must be at least one backward dependency
        assert!(result.score < 1.0, "Cyclic graph should have backward deps");
        assert!(result.backward_dep_count > 0);
    }

    #[test]
    fn test_mixed_layering() {
        // Mix of forward and backward deps
        let deps = vec![
            ModuleDependency::new("b", "a"), // forward
            ModuleDependency::new("c", "b"), // forward
            ModuleDependency::new("a", "c"), // backward (cycle)
        ];
        let result = compute_layering_score(&deps);
        assert!(result.score > 0.0);
        assert!(result.score < 1.0);
        assert!(result.backward_dep_count > 0);
    }

    #[test]
    fn test_empty_deps_returns_perfect() {
        let deps: Vec<ModuleDependency> = vec![];
        let result = compute_layering_score(&deps);
        assert_eq!(result.score, 1.0);
        assert_eq!(result.backward_dep_count, 0);
    }

    #[test]
    fn test_self_deps_ignored() {
        let deps = vec![
            ModuleDependency::new("a", "a"),
            ModuleDependency::new("b", "a"),
        ];
        let result = compute_layering_score(&deps);
        // Self-dep should be ignored, only b->a counts
        assert_eq!(result.total_dep_count, 1);
    }

    #[test]
    fn test_path_to_module_extracts_correctly() {
        assert_eq!(path_to_module("src/analysis/dsm.rs"), "analysis");
        assert_eq!(path_to_module("src/cli/args.rs"), "cli");
        assert_eq!(path_to_module("src/main.rs"), "root");
        assert_eq!(path_to_module("./src/lib.rs"), "root");
        assert_eq!(path_to_module("src/io/writers/dsm.rs"), "io/writers");
    }

    #[test]
    fn test_problematic_modules_sorted() {
        let deps = vec![
            ModuleDependency::new("a", "b"),
            ModuleDependency::new("a", "c"),
            ModuleDependency::new("a", "d"),
            ModuleDependency::new("b", "d"),
        ];
        let result = compute_layering_score(&deps);
        // Check that problematic modules are sorted by count
        if result.problematic_modules.len() > 1 {
            let counts: Vec<usize> = result.problematic_modules.iter().map(|(_, c)| *c).collect();
            for i in 1..counts.len() {
                assert!(counts[i - 1] >= counts[i]);
            }
        }
    }

    #[test]
    fn test_layering_penalty_thresholds() {
        assert_eq!(calculate_layering_penalty(0), 0.0);
        assert_eq!(calculate_layering_penalty(1), 0.0);
        assert_eq!(calculate_layering_penalty(2), 0.0);
        assert_eq!(calculate_layering_penalty(3), 0.15);
        assert_eq!(calculate_layering_penalty(4), 0.15);
        assert_eq!(calculate_layering_penalty(5), 0.20);
        assert_eq!(calculate_layering_penalty(10), 0.20);
    }

    #[test]
    fn test_layering_impact_empty() {
        let impact = compute_layering_impact("nonexistent", &[]);
        assert_eq!(impact.backward_deps_caused, 0);
        assert!(impact.affected_modules.is_empty());
    }

    #[test]
    fn test_layering_impact_with_deps() {
        let deps = vec![
            ModuleDependency::new("god_object", "utils"),
            ModuleDependency::new("god_object", "config"),
            ModuleDependency::new("utils", "config"),
        ];
        let impact = compute_layering_impact("god_object", &deps);
        assert!(!impact.affected_modules.is_empty());
    }

    #[test]
    fn test_deterministic_results() {
        let deps = vec![
            ModuleDependency::new("a", "b"),
            ModuleDependency::new("b", "c"),
            ModuleDependency::new("c", "a"),
        ];
        let result1 = compute_layering_score(&deps);
        let result2 = compute_layering_score(&deps);
        assert_eq!(result1.score, result2.score);
        assert_eq!(result1.backward_dep_count, result2.backward_dep_count);
    }
}
