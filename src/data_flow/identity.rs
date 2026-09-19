//! Preserve exact facts while bridging uniquely identified legacy declarations.
use super::DataFlowGraph;
use crate::priority::call_graph::FunctionId;
use std::collections::HashMap;

impl DataFlowGraph {
    fn canonical_identity(&self, id: &FunctionId) -> Option<FunctionId> {
        if id.column.is_some() || self.call_graph.is_empty() {
            Some(id.clone())
        } else {
            self.call_graph.find_exact_or_unique_legacy(id)
        }
    }

    pub(super) fn storage_identity(&self, id: FunctionId) -> FunctionId {
        if id.column.is_some() || self.call_graph.is_empty() {
            id
        } else {
            self.call_graph
                .find_exact_or_unique_legacy(&id)
                .unwrap_or(id)
        }
    }

    pub(super) fn identity_candidates(&self, id: &FunctionId) -> Option<[Option<FunctionId>; 4]> {
        let canonical = self.canonical_identity(id)?;
        let legacy = canonical.clone().with_column(None);
        let mut without_module = legacy.clone();
        without_module.module_path.clear();
        let unique = |legacy: FunctionId| {
            (self
                .call_graph
                .find_exact_or_unique_legacy(&legacy)
                .as_ref()
                == Some(&canonical))
            .then_some(legacy)
        };
        let legacy = unique(legacy);
        let without_module = unique(without_module);
        Some([Some(canonical), Some(id.clone()), legacy, without_module])
    }

    pub(super) fn identity_lookup<'a, T>(
        &self,
        values: &'a HashMap<FunctionId, T>,
        id: &FunctionId,
    ) -> Option<&'a T> {
        if id.column.is_some()
            && let Some(value) = values.get(id)
        {
            return Some(value);
        }
        self.identity_candidates(id)?
            .into_iter()
            .flatten()
            .find_map(|key| values.get(&key))
    }
}
