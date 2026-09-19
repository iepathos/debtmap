//! External dependency views preserve graph recursion while excluding self-coupling.
use super::{CallGraph, FunctionId};

impl CallGraph {
    pub(crate) fn is_test_dependency(&self, id: &FunctionId) -> bool {
        if let Some(node) = self.nodes.get(id) {
            node.is_test
                || self.caller_index.get(id).is_some_and(|callers| {
                    !callers.is_empty()
                        && callers.iter().all(|caller| self.is_test_function(caller))
                })
        } else {
            crate::priority::caller_classification::classify_by_heuristics(&id.name)
                == crate::priority::caller_classification::CallerType::Test
        }
    }

    pub(crate) fn external_callers(&self, query: &FunctionId) -> Vec<FunctionId> {
        let canonical = self.find_function(query).unwrap_or_else(|| query.clone());
        self.get_callers_exact(&canonical)
            .into_iter()
            .filter(|caller| caller != &canonical)
            .collect()
    }

    pub(crate) fn external_callees(&self, query: &FunctionId) -> Vec<FunctionId> {
        let canonical = self.find_function(query).unwrap_or_else(|| query.clone());
        self.get_callees_exact(&canonical)
            .into_iter()
            .filter(|callee| callee != &canonical)
            .collect()
    }
}
