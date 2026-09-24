//! Possible call relations, isolated from resolved graph metrics and propagation.

use super::{CallGraph, FunctionId, UncertainCall};
use crate::collections::HashSet;

impl CallGraph {
    /// Retain a call even when no admissible project target is known.
    pub fn record_uncertain_call(&mut self, mut call: UncertainCall) {
        call.candidates.sort();
        call.candidates.dedup();
        if let Err(position) = self.uncertain_calls.binary_search(&call) {
            self.index_uncertain_call(&call);
            self.uncertain_calls.insert(position, call);
        }
    }

    fn index_uncertain_call(&mut self, call: &UncertainCall) {
        for target in &call.candidates {
            self.possible_caller_index
                .entry(target.clone())
                .or_default()
                .insert(call.caller.clone());
            self.possible_callee_index
                .entry(call.caller.clone())
                .or_default()
                .insert(target.clone());
        }
    }

    /// Source calls in deterministic order, including zero-candidate diagnostics.
    pub fn uncertain_calls(&self) -> impl Iterator<Item = &UncertainCall> {
        self.uncertain_calls.iter()
    }

    /// Possible callers only; ordinary resolved callers are queried separately.
    pub fn get_possible_callers(&self, target: &FunctionId) -> Vec<FunctionId> {
        let target = self.find_function(target).unwrap_or_else(|| target.clone());
        let mut callers: Vec<_> = self
            .possible_caller_index
            .get(&target)
            .into_iter()
            .flatten()
            .filter(|caller| self.nodes.contains_key(*caller))
            .cloned()
            .collect();
        callers.sort();
        callers
    }

    /// Possible callees only, restricted to definitions present in the graph.
    pub fn get_possible_callees(&self, caller: &FunctionId) -> Vec<FunctionId> {
        let caller = self.find_function(caller).unwrap_or_else(|| caller.clone());
        let mut callees: Vec<_> = self
            .possible_callee_index
            .get(&caller)
            .into_iter()
            .flatten()
            .filter(|target| self.nodes.contains_key(*target))
            .cloned()
            .collect();
        callees.sort();
        callees
    }

    /// Traverse both resolved and possible relations without introducing live roots.
    /// The starting function itself is included in the returned set.
    pub fn get_transitive_possible_callees(&self, root: &FunctionId) -> HashSet<FunctionId> {
        self.get_possible_reachable_functions([root.clone()])
    }

    /// Traverse resolved and possible calls once across all supplied live roots.
    pub fn get_possible_reachable_functions(
        &self,
        roots: impl IntoIterator<Item = FunctionId>,
    ) -> HashSet<FunctionId> {
        let mut visited = HashSet::new();
        let mut pending: Vec<_> = roots.into_iter().collect();
        while let Some(current) = pending.pop() {
            if visited.insert(current.clone()) {
                pending.extend(self.get_callees(&current));
                pending.extend(self.get_possible_callees(&current));
            }
        }
        visited
    }
}
