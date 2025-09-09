//! Pattern detection algorithms for identifying delegation and other patterns

use super::types::{CallGraph, FunctionId, FunctionNode};
use im::HashMap;

impl CallGraph {
    /// Pure function to check if delegation pattern criteria are met
    pub fn meets_delegation_criteria(orchestrator_complexity: u32, callee_count: usize) -> bool {
        orchestrator_complexity <= 3 && callee_count >= 2
    }

    /// Pure function to calculate average complexity of callees
    pub fn calculate_average_callee_complexity(
        callees: &[FunctionId],
        nodes: &HashMap<FunctionId, FunctionNode>,
    ) -> f64 {
        let total_complexity: f64 = callees
            .iter()
            .filter_map(|id| nodes.get(id))
            .map(|n| n.complexity as f64)
            .sum();
        
        total_complexity / callees.len().max(1) as f64
    }

    /// Pure function to determine if complexity indicates delegation
    pub fn indicates_delegation(orchestrator_complexity: u32, avg_callee_complexity: f64) -> bool {
        avg_callee_complexity > orchestrator_complexity as f64 * 1.5
    }

    pub fn detect_delegation_pattern(&self, func_id: &FunctionId) -> bool {
        let Some(node) = self.nodes.get(func_id) else {
            return false;
        };
        
        let callees = self.get_callees(func_id);
        
        if !Self::meets_delegation_criteria(node.complexity, callees.len()) {
            return false;
        }
        
        let avg_callee_complexity = Self::calculate_average_callee_complexity(&callees, &self.nodes);
        Self::indicates_delegation(node.complexity, avg_callee_complexity)
    }
}