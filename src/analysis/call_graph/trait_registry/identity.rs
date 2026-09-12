//! Reconcile legacy enhancement metadata with authoritative workspace definitions.

use super::TraitRegistry;
use crate::priority::call_graph::{CallGraph, FunctionId};
use std::collections::HashMap;
use std::path::PathBuf;

pub(crate) type Locations = HashMap<(PathBuf, usize), Vec<FunctionId>>;

impl TraitRegistry {
    pub(crate) fn canonicalize_definitions(&mut self, graph: &CallGraph) {
        let locations = definition_locations(graph);
        for method in self.trait_definitions.values_mut().flatten() {
            method.method_id = canonical_id(&method.method_id, &locations);
        }
        for method in self
            .trait_implementations
            .values_mut()
            .flatten()
            .flat_map(|implementation| &mut implementation.method_implementations)
        {
            method.method_id = canonical_id(&method.method_id, &locations);
        }
        self.visit_trait_methods = self
            .visit_trait_methods
            .iter()
            .map(|id| canonical_id(id, &locations))
            .collect();
        for method in self.visit_implementations.values_mut().flatten() {
            method.method_id = canonical_id(&method.method_id, &locations);
        }
    }
}

pub(crate) fn definition_locations(graph: &CallGraph) -> Locations {
    graph
        .get_all_functions()
        .fold(HashMap::new(), |mut locations, id| {
            locations
                .entry((id.file.clone(), id.line))
                .or_default()
                .push(id.clone());
            locations
        })
}

pub(crate) fn canonical_id(id: &FunctionId, locations: &Locations) -> FunctionId {
    let mut candidates = locations
        .get(&(id.file.clone(), id.line))
        .into_iter()
        .flatten()
        .filter(|candidate| same_definition(id, candidate));
    let first = candidates.next();
    match (first, candidates.next()) {
        (Some(candidate), None) => candidate.clone(),
        _ => id.clone(),
    }
}

fn same_definition(metadata: &FunctionId, definition: &FunctionId) -> bool {
    match (metadata.column, definition.column) {
        (Some(column), Some(other)) => column == other,
        _ => {
            definition.name == metadata.name
                || definition.name.ends_with(&format!("::{}", metadata.name))
        }
    }
}
