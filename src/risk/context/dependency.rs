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
            } => match *blast_radius {
                r if r > 10 => format!(
                    "Critical dependency with blast radius {} affecting {} modules",
                    blast_radius,
                    dependents.len()
                ),
                r if r > 5 => format!(
                    "Important dependency with {} dependents (risk: {:.1})",
                    dependents.len(),
                    propagated_risk
                ),
                r if r > 0 => format!("Dependency depth {depth} with limited impact"),
                _ => "Isolated component with no dependencies".to_string(),
            },
            _ => "No dependency information".to_string(),
        }
    }
}

impl DependencyRiskProvider {
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
}
