//! Test-related analysis including test helpers and test-only functions

use super::types::{CallGraph, FunctionId, FunctionNode};
use im::{HashMap, HashSet};

impl CallGraph {
    pub fn find_test_functions(&self) -> Vec<FunctionId> {
        self.nodes
            .values()
            .filter(|node| node.is_test)
            .map(|node| node.id.clone())
            .collect()
    }

    /// Check if a function is only called by test functions (test helper)
    /// Returns true if:
    /// - The function has at least one caller
    /// - All callers are test functions
    pub fn is_test_helper(&self, func_id: &FunctionId) -> bool {
        let callers = self.get_callers(func_id);

        // If no callers, it's not a test helper
        if callers.is_empty() {
            return false;
        }

        // Check if all callers are test functions
        callers.iter().all(|caller| self.is_test_function(caller))
    }

    /// Pure function to identify all test functions from nodes
    fn collect_test_functions(nodes: &HashMap<FunctionId, FunctionNode>) -> HashSet<FunctionId> {
        nodes
            .iter()
            .filter(|(_, node)| node.is_test)
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// Pure function to check if a node is a production entry point
    pub fn is_production_entry_point(node: &FunctionNode, callers: &[FunctionId]) -> bool {
        !node.is_test && (node.is_entry_point || callers.is_empty())
    }

    /// Pure function to filter test-only functions from reachable sets
    fn filter_test_only_functions(
        reachable_from_tests: HashSet<FunctionId>,
        reachable_from_production: &HashSet<FunctionId>,
        nodes: &HashMap<FunctionId, FunctionNode>,
    ) -> HashSet<FunctionId> {
        reachable_from_tests
            .into_iter()
            .filter(|id| {
                !reachable_from_production.contains(id)
                    && nodes.get(id).is_some_and(|node| !node.is_test)
            })
            .collect()
    }

    /// Identify functions that are only reachable from test functions
    /// These are test infrastructure functions (mocks, helpers, fixtures, etc.)
    pub fn find_test_only_functions(&self) -> HashSet<FunctionId> {
        let test_functions = Self::collect_test_functions(&self.nodes);
        let reachable_from_tests = self.find_functions_reachable_from_tests(&test_functions);
        let reachable_from_production = self.find_functions_reachable_from_production();
        
        Self::filter_test_only_functions(
            reachable_from_tests,
            &reachable_from_production,
            &self.nodes,
        )
    }

    /// Find all functions reachable from test functions (including tests themselves)
    fn find_functions_reachable_from_tests(
        &self,
        test_functions: &HashSet<FunctionId>,
    ) -> HashSet<FunctionId> {
        let mut reachable_from_tests = test_functions.clone();
        for test_fn in test_functions {
            let callees = self.get_transitive_callees(test_fn, usize::MAX);
            reachable_from_tests.extend(callees);
        }
        reachable_from_tests
    }

    /// Find all functions reachable from production (non-test) entry points
    fn find_functions_reachable_from_production(&self) -> HashSet<FunctionId> {
        let mut reachable_from_production = HashSet::new();
        for (id, node) in &self.nodes {
            let callers = self.get_callers(id);
            if Self::is_production_entry_point(node, &callers) {
                reachable_from_production.insert(id.clone());
                let callees = self.get_transitive_callees(id, usize::MAX);
                reachable_from_production.extend(callees);
            }
        }
        reachable_from_production
    }
}