use super::{AnalysisTarget, Context, ContextDetails, ContextProvider};
use anyhow::Result;
use im::{HashMap, HashSet, Vector};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Represents a module in the dependency graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Module {
    pub name: String,
    pub path: PathBuf,
    pub intrinsic_risk: f64,
    pub functions: Vector<String>,
}

/// Edge in the dependency graph
#[derive(Debug, Clone)]
pub struct DependencyEdge {
    pub from: String,
    pub to: String,
    pub coupling_strength: f64,
}

/// Dependency graph for risk propagation
#[derive(Debug, Clone)]
pub struct DependencyGraph {
    modules: HashMap<String, Module>,
    edges: Vector<DependencyEdge>,
}

impl Default for DependencyGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl DependencyGraph {
    pub fn new() -> Self {
        Self {
            modules: HashMap::new(),
            edges: Vector::new(),
        }
    }

    pub fn add_module(&mut self, module: Module) {
        self.modules.insert(module.name.clone(), module);
    }

    pub fn add_dependency(&mut self, from: String, to: String, coupling_strength: f64) {
        self.edges.push_back(DependencyEdge {
            from,
            to,
            coupling_strength,
        });
    }

    pub fn get_dependencies(&self, module: &str) -> Vector<&DependencyEdge> {
        self.edges.iter().filter(|e| e.from == module).collect()
    }

    pub fn get_dependents(&self, module: &str) -> Vector<&DependencyEdge> {
        self.edges.iter().filter(|e| e.to == module).collect()
    }

    pub fn get_module(&self, name: &str) -> Option<&Module> {
        self.modules.get(name)
    }

    pub fn modules(&self) -> impl Iterator<Item = &Module> {
        self.modules.values()
    }
}

/// Calculates and propagates risk through the dependency graph
pub struct DependencyRiskCalculator {
    graph: DependencyGraph,
    risk_scores: HashMap<String, f64>,
}

impl DependencyRiskCalculator {
    pub fn new(graph: DependencyGraph) -> Self {
        let mut risk_scores = HashMap::new();

        // Initialize with intrinsic risks
        for module in graph.modules() {
            risk_scores.insert(module.name.clone(), module.intrinsic_risk);
        }

        Self { graph, risk_scores }
    }

    /// Propagate risk through the dependency graph
    pub fn propagate_risk(&mut self) {
        let mut changed = true;
        let mut iterations = 0;
        const MAX_ITERATIONS: usize = 10;
        const CONVERGENCE_THRESHOLD: f64 = 0.01;

        while changed && iterations < MAX_ITERATIONS {
            changed = false;
            let mut new_scores = self.risk_scores.clone();

            for module in self.graph.modules() {
                let old_risk = self.risk_scores.get(&module.name).copied().unwrap_or(0.0);
                let propagated_risk = self.calculate_propagated_risk(&module.name);

                if (propagated_risk - old_risk).abs() > CONVERGENCE_THRESHOLD {
                    new_scores.insert(module.name.clone(), propagated_risk);
                    changed = true;
                }
            }

            self.risk_scores = new_scores;
            iterations += 1;
        }

        log::debug!("Risk propagation converged after {iterations} iterations");
    }

    fn calculate_propagated_risk(&self, module_name: &str) -> f64 {
        let module = match self.graph.get_module(module_name) {
            Some(m) => m,
            None => return 0.0,
        };

        let base_risk = module.intrinsic_risk;
        let mut dependency_risk = 0.0;

        // Risk from dependencies (what this module depends on)
        for dep in self.graph.get_dependencies(module_name) {
            let dep_risk = self.risk_scores.get(&dep.to).copied().unwrap_or(0.0);
            dependency_risk += dep_risk * dep.coupling_strength * 0.3;
        }

        // Risk propagated to dependents (how critical this module is to others)
        let dependents = self.graph.get_dependents(module_name);
        let criticality_factor = 1.0 + (dependents.len() as f64 * 0.1).min(0.5);

        (base_risk * criticality_factor + dependency_risk).min(10.0)
    }

    /// Calculate the blast radius of a change to a module
    pub fn calculate_blast_radius(&self, module_name: &str) -> usize {
        let mut affected = HashSet::new();
        let mut to_visit = Vector::new();

        to_visit.push_back(module_name.to_string());

        while let Some(current) = to_visit.pop_front() {
            if affected.contains(&current) {
                continue;
            }

            affected.insert(current.clone());

            // Add all modules that depend on this one
            for dep in self.graph.get_dependents(&current) {
                if !affected.contains(&dep.from) {
                    to_visit.push_back(dep.from.clone());
                }
            }
        }

        affected.len()
    }

    /// Get the risk score for a module
    pub fn get_risk(&self, module_name: &str) -> f64 {
        self.risk_scores.get(module_name).copied().unwrap_or(0.0)
    }

    /// Find the module containing a function
    pub fn find_module_for_function(&self, function_name: &str) -> Option<&Module> {
        self.graph
            .modules()
            .find(|m| m.functions.contains(&function_name.to_string()))
    }
}

/// Context provider for dependency risk analysis
pub struct DependencyRiskProvider {
    calculator: DependencyRiskCalculator,
}

impl DependencyRiskProvider {
    pub fn new(graph: DependencyGraph) -> Self {
        let mut calculator = DependencyRiskCalculator::new(graph);
        calculator.propagate_risk();
        Self { calculator }
    }
}

impl ContextProvider for DependencyRiskProvider {
    fn name(&self) -> &str {
        "dependency_risk"
    }

    fn gather(&self, target: &AnalysisTarget) -> Result<Context> {
        let module = self
            .calculator
            .find_module_for_function(&target.function_name);

        let (propagated_risk, blast_radius, dependents) = if let Some(module) = module {
            let risk = self.calculator.get_risk(&module.name);
            let radius = self.calculator.calculate_blast_radius(&module.name);
            let deps: Vec<String> = self
                .calculator
                .graph
                .get_dependents(&module.name)
                .iter()
                .map(|d| d.from.clone())
                .collect();

            (risk, radius, deps)
        } else {
            (0.0, 0, vec![])
        };

        // Calculate depth in dependency tree
        let depth = self.calculate_dependency_depth(&target.function_name);

        // Higher contribution for functions with large blast radius or many dependents
        let contribution = match blast_radius {
            r if r > 10 => 1.5,
            r if r > 5 => 1.0,
            r if r > 2 => 0.5,
            _ => 0.2,
        };

        Ok(Context {
            provider: self.name().to_string(),
            weight: self.weight(),
            contribution,
            details: ContextDetails::DependencyChain {
                depth,
                propagated_risk,
                dependents,
                blast_radius,
            },
        })
    }

    fn weight(&self) -> f64 {
        1.2 // Dependency risk has moderate-high weight
    }

    fn explain(&self, context: &Context) -> String {
        match &context.details {
            ContextDetails::DependencyChain {
                depth,
                propagated_risk,
                dependents,
                blast_radius,
            } => Self::classify_blast_radius_impact(
                *blast_radius,
                *depth,
                *propagated_risk,
                dependents.len(),
            ),
            _ => "No dependency information".to_string(),
        }
    }
}

impl DependencyRiskProvider {
    fn classify_blast_radius_impact(
        blast_radius: usize,
        depth: usize,
        propagated_risk: f64,
        dependents_count: usize,
    ) -> String {
        match blast_radius {
            r if r > 10 => format!(
                "Critical dependency with blast radius {} affecting {} modules",
                blast_radius, dependents_count
            ),
            r if r > 5 => format!(
                "Important dependency with {} dependents (risk: {:.1})",
                dependents_count, propagated_risk
            ),
            r if r > 0 => format!("Dependency depth {} with limited impact", depth),
            _ => "Isolated component with no dependencies".to_string(),
        }
    }

    fn calculate_dependency_depth(&self, function_name: &str) -> usize {
        if let Some(module) = self.calculator.find_module_for_function(function_name) {
            self.calculate_module_depth(&module.name, &mut HashSet::new())
        } else {
            0
        }
    }

    fn calculate_module_depth(&self, module_name: &str, visited: &mut HashSet<String>) -> usize {
        if visited.contains(module_name) {
            return 0;
        }

        visited.insert(module_name.to_string());

        let deps = self.calculator.graph.get_dependencies(module_name);
        if deps.is_empty() {
            return 0;
        }

        let max_depth = deps
            .iter()
            .map(|d| self.calculate_module_depth(&d.to, visited))
            .max()
            .unwrap_or(0);

        max_depth + 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use im::vector;

    #[test]
    fn test_dependency_graph() {
        let mut graph = DependencyGraph::new();

        graph.add_module(Module {
            name: "core".to_string(),
            path: PathBuf::from("src/core"),
            intrinsic_risk: 5.0,
            functions: vector!["process".to_string()],
        });

        graph.add_module(Module {
            name: "api".to_string(),
            path: PathBuf::from("src/api"),
            intrinsic_risk: 3.0,
            functions: vector!["handle".to_string()],
        });

        graph.add_dependency("api".to_string(), "core".to_string(), 0.8);

        let deps = graph.get_dependencies("api");
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].to, "core");

        let dependents = graph.get_dependents("core");
        assert_eq!(dependents.len(), 1);
        assert_eq!(dependents[0].from, "api");
    }

    #[test]
    fn test_risk_propagation() {
        let mut graph = DependencyGraph::new();

        graph.add_module(Module {
            name: "high_risk".to_string(),
            path: PathBuf::from("src/high"),
            intrinsic_risk: 8.0,
            functions: vector!["critical".to_string()],
        });

        graph.add_module(Module {
            name: "low_risk".to_string(),
            path: PathBuf::from("src/low"),
            intrinsic_risk: 2.0,
            functions: vector!["simple".to_string()],
        });

        graph.add_dependency("low_risk".to_string(), "high_risk".to_string(), 0.7);

        let mut calculator = DependencyRiskCalculator::new(graph);
        calculator.propagate_risk();

        let low_risk_score = calculator.get_risk("low_risk");
        assert!(low_risk_score > 2.0); // Should be increased due to dependency on high_risk
        assert!(low_risk_score < 8.0); // But not as high as high_risk itself
    }

    #[test]
    fn test_blast_radius() {
        let mut graph = DependencyGraph::new();

        for i in 0..5 {
            graph.add_module(Module {
                name: format!("module_{i}"),
                path: PathBuf::from(format!("src/mod{i}")),
                intrinsic_risk: 3.0,
                functions: vector![format!("func_{}", i)],
            });
        }

        // Create a chain: 4 -> 3 -> 2 -> 1 -> 0
        // (module i depends on module i+1)
        for i in 0..4 {
            graph.add_dependency(format!("module_{}", i + 1), format!("module_{i}"), 0.5);
        }

        let calculator = DependencyRiskCalculator::new(graph);

        // Module 4 affects no one (only itself)
        assert_eq!(calculator.calculate_blast_radius("module_4"), 1);

        // Module 3 affects module 4
        assert_eq!(calculator.calculate_blast_radius("module_3"), 2);

        // Module 2 affects modules 3 and 4
        assert_eq!(calculator.calculate_blast_radius("module_2"), 3);

        // Module 0 affects all modules (0, 1, 2, 3, 4)
        assert_eq!(calculator.calculate_blast_radius("module_0"), 5);
    }

    #[test]
    fn test_explain_critical_dependency_blast_radius_over_10() {
        let provider = DependencyRiskProvider {
            calculator: DependencyRiskCalculator::new(DependencyGraph::new()),
        };

        let context = Context {
            provider: "dependency".to_string(),
            weight: 1.2,
            contribution: 5.0,
            details: ContextDetails::DependencyChain {
                depth: 3,
                propagated_risk: 8.5,
                dependents: vec!["module_a".to_string(), "module_b".to_string()],
                blast_radius: 15,
            },
        };

        let explanation = provider.explain(&context);
        assert_eq!(
            explanation,
            "Critical dependency with blast radius 15 affecting 2 modules"
        );
    }

    #[test]
    fn test_explain_important_dependency_blast_radius_6_to_10() {
        let provider = DependencyRiskProvider {
            calculator: DependencyRiskCalculator::new(DependencyGraph::new()),
        };

        let context = Context {
            provider: "dependency".to_string(),
            weight: 1.2,
            contribution: 3.0,
            details: ContextDetails::DependencyChain {
                depth: 2,
                propagated_risk: 6.3,
                dependents: vec![
                    "module_x".to_string(),
                    "module_y".to_string(),
                    "module_z".to_string(),
                ],
                blast_radius: 7,
            },
        };

        let explanation = provider.explain(&context);
        assert_eq!(
            explanation,
            "Important dependency with 3 dependents (risk: 6.3)"
        );
    }

    #[test]
    fn test_explain_limited_impact_blast_radius_1_to_5() {
        let provider = DependencyRiskProvider {
            calculator: DependencyRiskCalculator::new(DependencyGraph::new()),
        };

        let context = Context {
            provider: "dependency".to_string(),
            weight: 1.2,
            contribution: 1.5,
            details: ContextDetails::DependencyChain {
                depth: 1,
                propagated_risk: 2.0,
                dependents: vec!["module_single".to_string()],
                blast_radius: 3,
            },
        };

        let explanation = provider.explain(&context);
        assert_eq!(explanation, "Dependency depth 1 with limited impact");
    }

    #[test]
    fn test_explain_isolated_component_blast_radius_0() {
        let provider = DependencyRiskProvider {
            calculator: DependencyRiskCalculator::new(DependencyGraph::new()),
        };

        let context = Context {
            provider: "dependency".to_string(),
            weight: 1.2,
            contribution: 0.0,
            details: ContextDetails::DependencyChain {
                depth: 0,
                propagated_risk: 0.0,
                dependents: vec![],
                blast_radius: 0,
            },
        };

        let explanation = provider.explain(&context);
        assert_eq!(explanation, "Isolated component with no dependencies");
    }

    #[test]
    fn test_explain_non_dependency_context() {
        let provider = DependencyRiskProvider {
            calculator: DependencyRiskCalculator::new(DependencyGraph::new()),
        };

        let context = Context {
            provider: "dependency".to_string(),
            weight: 1.2,
            contribution: 0.0,
            details: ContextDetails::Historical {
                change_frequency: 5.0,
                bug_density: 0.1,
                age_days: 365,
                author_count: 3,
            },
        };

        let explanation = provider.explain(&context);
        assert_eq!(explanation, "No dependency information");
    }

    #[test]
    fn test_gather_with_high_blast_radius() {
        // Test gather() when module has blast radius > 10
        let mut graph = DependencyGraph::new();

        // Create a central module with many dependents
        graph.add_module(Module {
            name: "core".to_string(),
            path: PathBuf::from("src/core"),
            intrinsic_risk: 5.0,
            functions: vector!["critical_function".to_string()],
        });

        // Add 12 modules that depend on core
        for i in 0..12 {
            graph.add_module(Module {
                name: format!("dependent_{i}"),
                path: PathBuf::from(format!("src/dep{i}")),
                intrinsic_risk: 2.0,
                functions: vector![format!("func_{i}")],
            });
            graph.add_dependency(format!("dependent_{i}"), "core".to_string(), 0.8);
        }

        let calculator = DependencyRiskCalculator::new(graph);
        let provider = DependencyRiskProvider { calculator };

        let target = AnalysisTarget {
            root_path: PathBuf::from("."),
            file_path: PathBuf::from("src/core/lib.rs"),
            function_name: "critical_function".to_string(),
            line_range: (10, 50),
        };

        let context = provider.gather(&target).unwrap();

        assert_eq!(context.provider, "dependency_risk");
        assert_eq!(context.weight, 1.2);
        assert_eq!(context.contribution, 1.5); // High blast radius contribution

        match context.details {
            ContextDetails::DependencyChain {
                blast_radius,
                dependents,
                ..
            } => {
                assert!(blast_radius > 10);
                assert_eq!(dependents.len(), 12);
            }
            _ => panic!("Expected DependencyChain details"),
        }
    }

    #[test]
    fn test_gather_with_medium_blast_radius() {
        // Test gather() when module has blast radius between 6-10
        let mut graph = DependencyGraph::new();

        graph.add_module(Module {
            name: "service".to_string(),
            path: PathBuf::from("src/service"),
            intrinsic_risk: 4.0,
            functions: vector!["process".to_string()],
        });

        // Add 7 dependent modules
        for i in 0..7 {
            graph.add_module(Module {
                name: format!("consumer_{i}"),
                path: PathBuf::from(format!("src/consumer{i}")),
                intrinsic_risk: 2.0,
                functions: vector![format!("use_{i}")],
            });
            graph.add_dependency(format!("consumer_{i}"), "service".to_string(), 0.6);
        }

        let calculator = DependencyRiskCalculator::new(graph);
        let provider = DependencyRiskProvider { calculator };

        let target = AnalysisTarget {
            root_path: PathBuf::from("."),
            file_path: PathBuf::from("src/service/mod.rs"),
            function_name: "process".to_string(),
            line_range: (5, 25),
        };

        let context = provider.gather(&target).unwrap();

        assert_eq!(context.contribution, 1.0); // Medium blast radius contribution

        match context.details {
            ContextDetails::DependencyChain { blast_radius, .. } => {
                assert!(blast_radius > 5);
                assert!(blast_radius <= 10);
            }
            _ => panic!("Expected DependencyChain details"),
        }
    }

    #[test]
    fn test_gather_with_low_blast_radius() {
        // Test gather() when module has blast radius between 3-5
        let mut graph = DependencyGraph::new();

        graph.add_module(Module {
            name: "util".to_string(),
            path: PathBuf::from("src/util"),
            intrinsic_risk: 3.0,
            functions: vector!["helper".to_string()],
        });

        // Add 3 dependent modules
        for i in 0..3 {
            graph.add_module(Module {
                name: format!("user_{i}"),
                path: PathBuf::from(format!("src/user{i}")),
                intrinsic_risk: 2.0,
                functions: vector![format!("call_{i}")],
            });
            graph.add_dependency(format!("user_{i}"), "util".to_string(), 0.5);
        }

        let calculator = DependencyRiskCalculator::new(graph);
        let provider = DependencyRiskProvider { calculator };

        let target = AnalysisTarget {
            root_path: PathBuf::from("."),
            file_path: PathBuf::from("src/util/helpers.rs"),
            function_name: "helper".to_string(),
            line_range: (1, 10),
        };

        let context = provider.gather(&target).unwrap();

        assert_eq!(context.contribution, 0.5); // Low blast radius contribution

        match context.details {
            ContextDetails::DependencyChain { blast_radius, .. } => {
                assert!(blast_radius > 2);
                assert!(blast_radius <= 5);
            }
            _ => panic!("Expected DependencyChain details"),
        }
    }

    #[test]
    fn test_gather_with_minimal_blast_radius() {
        // Test gather() when module has blast radius <= 2
        let mut graph = DependencyGraph::new();

        graph.add_module(Module {
            name: "leaf".to_string(),
            path: PathBuf::from("src/leaf"),
            intrinsic_risk: 2.0,
            functions: vector!["simple".to_string()],
        });

        // Add just 1 dependent module
        graph.add_module(Module {
            name: "caller".to_string(),
            path: PathBuf::from("src/caller"),
            intrinsic_risk: 2.0,
            functions: vector!["invoke".to_string()],
        });
        graph.add_dependency("caller".to_string(), "leaf".to_string(), 0.3);

        let calculator = DependencyRiskCalculator::new(graph);
        let provider = DependencyRiskProvider { calculator };

        let target = AnalysisTarget {
            root_path: PathBuf::from("."),
            file_path: PathBuf::from("src/leaf/simple.rs"),
            function_name: "simple".to_string(),
            line_range: (1, 5),
        };

        let context = provider.gather(&target).unwrap();

        assert_eq!(context.contribution, 0.2); // Minimal blast radius contribution

        match context.details {
            ContextDetails::DependencyChain {
                blast_radius,
                dependents,
                ..
            } => {
                assert!(blast_radius <= 2);
                assert_eq!(dependents.len(), 1);
            }
            _ => panic!("Expected DependencyChain details"),
        }
    }

    #[test]
    fn test_gather_without_module() {
        // Test gather() when function has no associated module
        let graph = DependencyGraph::new();
        let calculator = DependencyRiskCalculator::new(graph);
        let provider = DependencyRiskProvider { calculator };

        let target = AnalysisTarget {
            root_path: PathBuf::from("."),
            file_path: PathBuf::from("src/unknown/file.rs"),
            function_name: "orphan_function".to_string(),
            line_range: (1, 10),
        };

        let context = provider.gather(&target).unwrap();

        assert_eq!(context.provider, "dependency_risk");
        assert_eq!(context.weight, 1.2);
        assert_eq!(context.contribution, 0.2); // Default minimal contribution

        match context.details {
            ContextDetails::DependencyChain {
                depth,
                propagated_risk,
                dependents,
                blast_radius,
            } => {
                assert_eq!(depth, 0);
                assert_eq!(propagated_risk, 0.0);
                assert!(dependents.is_empty());
                assert_eq!(blast_radius, 0);
            }
            _ => panic!("Expected DependencyChain details"),
        }
    }

    #[test]
    fn test_gather_with_module_no_dependencies() {
        // Test gather() when module exists but has no dependencies
        let mut graph = DependencyGraph::new();

        graph.add_module(Module {
            name: "isolated".to_string(),
            path: PathBuf::from("src/isolated"),
            intrinsic_risk: 3.0,
            functions: vector!["standalone".to_string()],
        });

        let calculator = DependencyRiskCalculator::new(graph);
        let provider = DependencyRiskProvider { calculator };

        let target = AnalysisTarget {
            root_path: PathBuf::from("."),
            file_path: PathBuf::from("src/isolated/mod.rs"),
            function_name: "standalone".to_string(),
            line_range: (10, 20),
        };

        let context = provider.gather(&target).unwrap();

        assert_eq!(context.contribution, 0.2); // Minimal contribution (blast_radius = 1)

        match context.details {
            ContextDetails::DependencyChain {
                blast_radius,
                dependents,
                propagated_risk,
                ..
            } => {
                assert_eq!(blast_radius, 1); // Only affects itself
                assert!(dependents.is_empty());
                assert_eq!(propagated_risk, 3.0); // Intrinsic risk only
            }
            _ => panic!("Expected DependencyChain details"),
        }
    }

    #[test]
    fn test_classify_blast_radius_critical() {
        // Test classification for critical dependencies (blast_radius > 10)
        let result = DependencyRiskProvider::classify_blast_radius_impact(15, 3, 8.5, 12);
        assert_eq!(
            result,
            "Critical dependency with blast radius 15 affecting 12 modules"
        );

        let result = DependencyRiskProvider::classify_blast_radius_impact(11, 2, 7.0, 8);
        assert_eq!(
            result,
            "Critical dependency with blast radius 11 affecting 8 modules"
        );
    }

    #[test]
    fn test_classify_blast_radius_important() {
        // Test classification for important dependencies (blast_radius 6-10)
        let result = DependencyRiskProvider::classify_blast_radius_impact(6, 2, 5.5, 4);
        assert_eq!(result, "Important dependency with 4 dependents (risk: 5.5)");

        let result = DependencyRiskProvider::classify_blast_radius_impact(10, 3, 7.2, 7);
        assert_eq!(result, "Important dependency with 7 dependents (risk: 7.2)");
    }

    #[test]
    fn test_classify_blast_radius_limited() {
        // Test classification for limited impact dependencies (blast_radius 1-5)
        let result = DependencyRiskProvider::classify_blast_radius_impact(1, 5, 2.0, 1);
        assert_eq!(result, "Dependency depth 5 with limited impact");

        let result = DependencyRiskProvider::classify_blast_radius_impact(5, 3, 3.5, 3);
        assert_eq!(result, "Dependency depth 3 with limited impact");
    }

    #[test]
    fn test_classify_blast_radius_isolated() {
        // Test classification for isolated components (blast_radius = 0)
        let result = DependencyRiskProvider::classify_blast_radius_impact(0, 0, 0.0, 0);
        assert_eq!(result, "Isolated component with no dependencies");
    }
}
