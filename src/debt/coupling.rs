use crate::core::{Dependency, DependencyKind, ModuleDependency};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

/// Coupling metrics for a module
#[derive(Debug, Clone)]
pub struct CouplingMetrics {
    pub module: String,
    pub afferent_coupling: usize, // Number of modules that depend on this module
    pub efferent_coupling: usize, // Number of modules this module depends on
    pub instability: f64,         // Efferent / (Afferent + Efferent)
    pub abstractness: f64,        // Ratio of abstract types to total types
}

impl CouplingMetrics {
    /// Calculate instability metric (0 = stable, 1 = unstable)
    pub fn calculate_instability(&mut self) {
        let total = self.afferent_coupling + self.efferent_coupling;
        if total > 0 {
            self.instability = self.efferent_coupling as f64 / total as f64;
        } else {
            self.instability = 0.0;
        }
    }

    /// Check if module has high coupling
    pub fn is_highly_coupled(&self, threshold: usize) -> bool {
        self.efferent_coupling > threshold || self.afferent_coupling > threshold
    }
}

/// Calculate coupling metrics for all modules
pub fn calculate_coupling_metrics(
    modules: &[ModuleDependency],
) -> HashMap<String, CouplingMetrics> {
    let mut metrics_map = HashMap::new();

    for module in modules {
        let mut metrics = CouplingMetrics {
            module: module.module.clone(),
            afferent_coupling: module.dependents.len(),
            efferent_coupling: module.dependencies.len(),
            instability: 0.0,
            abstractness: 0.0, // Would need type analysis to calculate properly
        };

        metrics.calculate_instability();
        metrics_map.insert(module.module.clone(), metrics);
    }

    metrics_map
}

/// Identify modules with problematic coupling
pub fn identify_coupling_issues(
    metrics: &HashMap<String, CouplingMetrics>,
    coupling_threshold: usize,
) -> Vec<String> {
    let mut issues = Vec::new();

    for (module, metric) in metrics {
        if metric.is_highly_coupled(coupling_threshold) {
            issues.push(format!(
                "Module '{}' has high coupling (afferent: {}, efferent: {})",
                module, metric.afferent_coupling, metric.efferent_coupling
            ));
        }

        // Stable Dependencies Principle violation
        if metric.instability > 0.8 && metric.afferent_coupling > 2 {
            issues.push(format!(
                "Module '{}' violates Stable Dependencies Principle (instability: {:.2}, depended on by {} modules)",
                module, metric.instability, metric.afferent_coupling
            ));
        }
    }

    issues
}

/// Analyze cohesion within a module (simplified version)
pub fn analyze_module_cohesion(
    _module_path: &Path,
    functions: &[String],
    shared_data: &[String],
) -> f64 {
    if functions.is_empty() || shared_data.is_empty() {
        return 0.0;
    }

    // Simplified cohesion: ratio of functions using shared data
    // In a real implementation, we'd analyze actual data usage
    let cohesion = shared_data.len() as f64 / functions.len() as f64;
    cohesion.clamp(0.0, 1.0)
}

/// Detect inappropriate intimacy between modules
pub fn detect_inappropriate_intimacy(module_deps: &[ModuleDependency]) -> Vec<(String, String)> {
    let mut intimate_pairs = Vec::new();

    for i in 0..module_deps.len() {
        for j in i + 1..module_deps.len() {
            let module_a = &module_deps[i];
            let module_b = &module_deps[j];

            // Check if modules have bidirectional dependencies
            let a_depends_on_b = module_a.dependencies.contains(&module_b.module);
            let b_depends_on_a = module_b.dependencies.contains(&module_a.module);

            if a_depends_on_b && b_depends_on_a {
                intimate_pairs.push((module_a.module.clone(), module_b.module.clone()));
            }
        }
    }

    intimate_pairs
}

/// Calculate the distance from the main sequence
/// D = |A + I - 1| where A is abstractness and I is instability
pub fn calculate_distance_from_main_sequence(metrics: &CouplingMetrics) -> f64 {
    (metrics.abstractness + metrics.instability - 1.0).abs()
}

/// Identify modules in the "zone of pain" (low abstractness, low instability)
pub fn identify_zone_of_pain(metrics: &HashMap<String, CouplingMetrics>) -> Vec<String> {
    let mut problematic = Vec::new();

    for (module, metric) in metrics {
        if metric.abstractness < 0.2 && metric.instability < 0.2 && metric.afferent_coupling > 3 {
            problematic.push(format!(
                "Module '{module}' is in the zone of pain (rigid and hard to change)"
            ));
        }
    }

    problematic
}

/// Identify modules in the "zone of uselessness" (high abstractness, high instability)
pub fn identify_zone_of_uselessness(metrics: &HashMap<String, CouplingMetrics>) -> Vec<String> {
    let mut problematic = Vec::new();

    for (module, metric) in metrics {
        if metric.abstractness > 0.8 && metric.instability > 0.8 {
            problematic.push(format!(
                "Module '{module}' is in the zone of uselessness (too abstract and unstable)"
            ));
        }
    }

    problematic
}

/// Build a module dependency map from file dependencies
pub fn build_module_dependency_map(
    file_dependencies: &[(PathBuf, Vec<Dependency>)],
) -> Vec<ModuleDependency> {
    let (module_map, reverse_map) = build_dependency_maps(file_dependencies);
    convert_to_module_dependencies(module_map, reverse_map)
}

/// Build forward and reverse dependency maps from file dependencies
fn build_dependency_maps(
    file_dependencies: &[(PathBuf, Vec<Dependency>)],
) -> (
    HashMap<String, HashSet<String>>,
    HashMap<String, HashSet<String>>,
) {
    let mut module_map: HashMap<String, HashSet<String>> = HashMap::new();
    let mut reverse_map: HashMap<String, HashSet<String>> = HashMap::new();

    for (file_path, deps) in file_dependencies {
        let module_name = extract_module_name(file_path);
        let dependencies = extract_import_dependencies(deps);

        module_map.insert(module_name.clone(), dependencies.clone());
        update_reverse_map(&mut reverse_map, &module_name, dependencies);
    }

    (module_map, reverse_map)
}

/// Extract import and module dependencies from a dependency list
fn extract_import_dependencies(deps: &[Dependency]) -> HashSet<String> {
    deps.iter()
        .filter(|dep| is_import_or_module_dependency(dep))
        .map(|dep| extract_module_from_import(&dep.name))
        .collect()
}

/// Check if a dependency is an import or module type
fn is_import_or_module_dependency(dep: &Dependency) -> bool {
    matches!(dep.kind, DependencyKind::Import | DependencyKind::Module)
}

/// Update reverse dependency map with module's dependencies
fn update_reverse_map(
    reverse_map: &mut HashMap<String, HashSet<String>>,
    module_name: &str,
    dependencies: HashSet<String>,
) {
    for dep in dependencies {
        reverse_map
            .entry(dep)
            .or_default()
            .insert(module_name.to_string());
    }
}

/// Convert dependency maps to ModuleDependency format
fn convert_to_module_dependencies(
    module_map: HashMap<String, HashSet<String>>,
    reverse_map: HashMap<String, HashSet<String>>,
) -> Vec<ModuleDependency> {
    let all_modules: HashSet<String> = module_map
        .keys()
        .chain(reverse_map.keys())
        .cloned()
        .collect();

    all_modules
        .into_iter()
        .map(|module| create_module_dependency(&module, &module_map, &reverse_map))
        .collect()
}

/// Create a ModuleDependency for a specific module
fn create_module_dependency(
    module: &str,
    module_map: &HashMap<String, HashSet<String>>,
    reverse_map: &HashMap<String, HashSet<String>>,
) -> ModuleDependency {
    ModuleDependency {
        module: module.to_string(),
        dependencies: module_map
            .get(module)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .collect(),
        dependents: reverse_map
            .get(module)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .collect(),
    }
}

fn extract_module_name(path: &Path) -> String {
    path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string()
}

fn extract_module_from_import(import: &str) -> String {
    import.split("::").next().unwrap_or(import).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coupling_metrics() {
        let module = ModuleDependency {
            module: "test_module".to_string(),
            dependencies: vec!["dep1".to_string(), "dep2".to_string()],
            dependents: vec!["dependent1".to_string()],
        };

        let metrics_map = calculate_coupling_metrics(&[module]);
        let metrics = metrics_map.get("test_module").unwrap();

        assert_eq!(metrics.efferent_coupling, 2);
        assert_eq!(metrics.afferent_coupling, 1);
        assert!((metrics.instability - 0.67).abs() < 0.01); // 2/(1+2) ≈ 0.67
    }

    #[test]
    fn test_inappropriate_intimacy() {
        let modules = vec![
            ModuleDependency {
                module: "A".to_string(),
                dependencies: vec!["B".to_string()],
                dependents: vec!["B".to_string()],
            },
            ModuleDependency {
                module: "B".to_string(),
                dependencies: vec!["A".to_string()],
                dependents: vec!["A".to_string()],
            },
        ];

        let intimate = detect_inappropriate_intimacy(&modules);
        assert_eq!(intimate.len(), 1);
        assert!(intimate.contains(&("A".to_string(), "B".to_string())));
    }

    #[test]
    fn test_identify_coupling_issues_no_issues() {
        let mut metrics = HashMap::new();

        let mut metric1 = CouplingMetrics {
            module: "module1".to_string(),
            afferent_coupling: 2,
            efferent_coupling: 2,
            instability: 0.0,
            abstractness: 0.0,
        };
        metric1.calculate_instability();

        let mut metric2 = CouplingMetrics {
            module: "module2".to_string(),
            afferent_coupling: 1,
            efferent_coupling: 1,
            instability: 0.0,
            abstractness: 0.0,
        };
        metric2.calculate_instability();

        metrics.insert("module1".to_string(), metric1);
        metrics.insert("module2".to_string(), metric2);

        let issues = identify_coupling_issues(&metrics, 5);
        assert_eq!(issues.len(), 0);
    }

    #[test]
    fn test_identify_coupling_issues_high_coupling() {
        let mut metrics = HashMap::new();

        let mut metric = CouplingMetrics {
            module: "highly_coupled".to_string(),
            afferent_coupling: 8,
            efferent_coupling: 2,
            instability: 0.0,
            abstractness: 0.0,
        };
        metric.calculate_instability();

        metrics.insert("highly_coupled".to_string(), metric);

        let issues = identify_coupling_issues(&metrics, 5);
        assert_eq!(issues.len(), 1);
        assert!(issues[0].contains("high coupling"));
        assert!(issues[0].contains("highly_coupled"));
        assert!(issues[0].contains("afferent: 8"));
    }

    #[test]
    fn test_identify_coupling_issues_stable_dependencies_violation() {
        let mut metrics = HashMap::new();

        let mut metric = CouplingMetrics {
            module: "unstable_but_depended_on".to_string(),
            afferent_coupling: 3,
            efferent_coupling: 14,
            instability: 0.0,
            abstractness: 0.0,
        };
        metric.calculate_instability();

        metrics.insert("unstable_but_depended_on".to_string(), metric);

        let issues = identify_coupling_issues(&metrics, 20);
        assert_eq!(issues.len(), 1);
        assert!(issues[0].contains("Stable Dependencies Principle"));
        assert!(issues[0].contains("unstable_but_depended_on"));
    }

    #[test]
    fn test_identify_coupling_issues_multiple_problems() {
        let mut metrics = HashMap::new();

        let mut metric = CouplingMetrics {
            module: "problematic".to_string(),
            afferent_coupling: 3,
            efferent_coupling: 13,
            instability: 0.0,
            abstractness: 0.0,
        };
        metric.calculate_instability();

        metrics.insert("problematic".to_string(), metric);

        let issues = identify_coupling_issues(&metrics, 5);
        assert_eq!(issues.len(), 2);

        let has_coupling_issue = issues.iter().any(|i| i.contains("high coupling"));
        let has_sdp_issue = issues
            .iter()
            .any(|i| i.contains("Stable Dependencies Principle"));

        assert!(has_coupling_issue);
        assert!(has_sdp_issue);
    }
}
