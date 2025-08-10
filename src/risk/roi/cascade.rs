use super::super::priority::TestTarget;
use super::{Context, DependencyGraph};
use std::collections::{HashMap as StdHashMap, HashSet};

#[derive(Clone, Debug)]
pub struct CascadeImpact {
    pub total_risk_reduction: f64,
    pub affected_modules: Vec<AffectedModule>,
    pub propagation_depth: usize,
}

impl Default for CascadeImpact {
    fn default() -> Self {
        Self {
            total_risk_reduction: 0.0,
            affected_modules: Vec::new(),
            propagation_depth: 0,
        }
    }
}

#[derive(Clone, Debug)]
pub struct AffectedModule {
    pub id: String,
    pub risk_reduction: f64,
    pub confidence: f64,
    pub depth: usize,
}

pub struct CascadeCalculator {
    propagation_decay: f64,
    min_strength: f64,
    max_depth: usize,
}

impl Default for CascadeCalculator {
    fn default() -> Self {
        Self::new()
    }
}

impl CascadeCalculator {
    pub fn new() -> Self {
        Self {
            propagation_decay: 0.7,
            min_strength: 0.1,
            max_depth: 3,
        }
    }

    pub fn calculate(&self, target: &TestTarget, context: &Context) -> CascadeImpact {
        let mut impact = CascadeImpact::default();
        let mut visited = HashSet::new();
        let mut module_impacts: StdHashMap<String, AffectedModule> = StdHashMap::new();

        self.propagate_impact(
            target.id.clone(),
            1.0,
            &mut visited,
            &mut module_impacts,
            0,
            &context.dependency_graph,
            target.current_risk,
        );

        impact.affected_modules = module_impacts.into_values().collect();
        impact
            .affected_modules
            .sort_by(|a, b| b.risk_reduction.partial_cmp(&a.risk_reduction).unwrap());

        impact.total_risk_reduction = impact
            .affected_modules
            .iter()
            .map(|m| m.risk_reduction)
            .sum();

        impact.propagation_depth = impact
            .affected_modules
            .iter()
            .map(|m| m.depth)
            .max()
            .unwrap_or(0);

        impact
    }

    #[allow(clippy::too_many_arguments)]
    fn propagate_impact(
        &self,
        node_id: String,
        strength: f64,
        visited: &mut HashSet<String>,
        module_impacts: &mut StdHashMap<String, AffectedModule>,
        depth: usize,
        graph: &DependencyGraph,
        source_risk: f64,
    ) {
        if depth > self.max_depth || strength < self.min_strength {
            return;
        }

        if !visited.insert(node_id.clone()) {
            return;
        }

        let dependents = self.get_dependents(&node_id, graph);

        for dependent_id in dependents {
            if let Some(dependent_node) = graph.nodes.get(&dependent_id) {
                let edge_weight = self.calculate_edge_weight(&node_id, &dependent_id, graph);
                let propagated_strength =
                    strength * edge_weight * self.propagation_decay.powi(depth as i32);

                let risk_reduction = self.calculate_risk_reduction(
                    source_risk,
                    dependent_node.risk,
                    propagated_strength,
                );

                module_impacts
                    .entry(dependent_id.clone())
                    .and_modify(|m| {
                        if risk_reduction > m.risk_reduction {
                            m.risk_reduction = risk_reduction;
                            m.confidence = propagated_strength;
                            m.depth = depth + 1;
                        }
                    })
                    .or_insert(AffectedModule {
                        id: dependent_id.clone(),
                        risk_reduction,
                        confidence: propagated_strength,
                        depth: depth + 1,
                    });

                self.propagate_impact(
                    dependent_id,
                    propagated_strength,
                    visited,
                    module_impacts,
                    depth + 1,
                    graph,
                    source_risk,
                );
            }
        }
    }

    fn get_dependents(&self, node_id: &str, graph: &DependencyGraph) -> Vec<String> {
        graph
            .edges
            .iter()
            .filter(|edge| edge.from == node_id)
            .map(|edge| edge.to.clone())
            .collect()
    }

    fn calculate_edge_weight(&self, from: &str, to: &str, graph: &DependencyGraph) -> f64 {
        graph
            .edges
            .iter()
            .find(|edge| edge.from == from && edge.to == to)
            .map(|edge| edge.weight)
            .unwrap_or(0.5)
    }

    fn calculate_risk_reduction(&self, source_risk: f64, target_risk: f64, strength: f64) -> f64 {
        let base_reduction = (source_risk * 0.1).min(1.0);

        let risk_factor = (target_risk / 10.0).min(1.0);

        base_reduction * risk_factor * strength
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::ComplexityMetrics;
    use im::{HashMap, Vector};
    use std::path::PathBuf;

    fn create_test_graph() -> DependencyGraph {
        let mut nodes = HashMap::new();

        nodes.insert(
            "module_a".to_string(),
            super::super::DependencyNode {
                id: "module_a".to_string(),
                path: PathBuf::from("src/module_a.rs"),
                risk: 5.0,
                complexity: ComplexityMetrics {
                    cyclomatic_complexity: 10,
                    cognitive_complexity: 15,
                    functions: vec![],
                },
            },
        );

        nodes.insert(
            "module_b".to_string(),
            super::super::DependencyNode {
                id: "module_b".to_string(),
                path: PathBuf::from("src/module_b.rs"),
                risk: 3.0,
                complexity: ComplexityMetrics {
                    cyclomatic_complexity: 5,
                    cognitive_complexity: 7,
                    functions: vec![],
                },
            },
        );

        let mut edges = Vector::new();
        edges.push_back(super::super::DependencyEdge {
            from: "module_a".to_string(),
            to: "module_b".to_string(),
            weight: 0.8,
        });

        DependencyGraph { nodes, edges }
    }

    #[test]
    fn test_cascade_calculation() {
        let calculator = CascadeCalculator::new();
        let target = TestTarget {
            id: "module_a".to_string(),
            path: PathBuf::from("src/module_a.rs"),
            function: Some("test_fn".to_string()),
            module_type: super::super::super::priority::ModuleType::Core,
            current_coverage: 0.0,
            current_risk: 8.0,
            complexity: ComplexityMetrics {
                cyclomatic_complexity: 10,
                cognitive_complexity: 15,
                functions: vec![],
            },
            dependencies: vec!["dep1".to_string()],
            dependents: vec!["module_b".to_string()],
            lines: 100,
            priority_score: 0.0,
            debt_items: 2,
        };

        let context = Context {
            dependency_graph: create_test_graph(),
            critical_paths: vec![],
            historical_data: None,
        };

        let impact = calculator.calculate(&target, &context);

        assert!(impact.total_risk_reduction > 0.0);
        assert!(!impact.affected_modules.is_empty());
        assert_eq!(impact.affected_modules[0].id, "module_b");
    }
}
