//! Align source outcomes with existing metric definitions without fuzzy selection.
use crate::priority::call_graph::{CallGraph, FunctionId};
use std::collections::HashMap;
use std::path::PathBuf;

type DefinitionKey = (PathBuf, String, usize, Option<usize>);

pub(crate) fn merge(base: CallGraph, source: CallGraph) -> CallGraph {
    if base.is_empty() {
        return source;
    }
    let candidates = base_definitions(&base);
    let source_definitions = definitions_without_columns(&source);
    let base_definitions = definitions_without_columns(&base);
    let mut base_ids = HashMap::new();
    let source_ids = source
        .get_all_functions()
        .map(|id| {
            let canonical = match candidates.get(&key(id)).map(Vec::as_slice) {
                Some([candidate]) => candidate.clone(),
                _ => legacy_identity(id, &source_definitions, &base_definitions)
                    .map(|legacy| {
                        let canonical = legacy.clone().with_column(id.column);
                        base_ids.insert(legacy, canonical.clone());
                        canonical
                    })
                    .unwrap_or_else(|| id.clone()),
            };
            (id.clone(), canonical)
        })
        .collect();
    let mut graph = remap(source, &source_ids);
    // Metric values replace source-only zero placeholders; evidence is merged.
    graph.merge(remap(base, &base_ids));
    graph
}

fn definitions_without_columns(graph: &CallGraph) -> HashMap<DefinitionKey, Vec<FunctionId>> {
    let mut definitions = HashMap::<_, Vec<_>>::new();
    for id in graph.get_all_functions() {
        let mut definition = key(id);
        definition.3 = None;
        definitions.entry(definition).or_default().push(id.clone());
    }
    definitions
}

fn legacy_identity(
    id: &FunctionId,
    source: &HashMap<DefinitionKey, Vec<FunctionId>>,
    base: &HashMap<DefinitionKey, Vec<FunctionId>>,
) -> Option<FunctionId> {
    let mut definition = key(id);
    definition.3 = None;
    match (
        source.get(&definition)?.as_slice(),
        base.get(&definition)?.as_slice(),
    ) {
        ([_], [legacy]) if legacy.column.is_none() => Some(legacy.clone()),
        _ => None,
    }
}

fn base_definitions(base: &CallGraph) -> HashMap<DefinitionKey, Vec<FunctionId>> {
    let mut definitions = HashMap::<_, Vec<_>>::new();
    for id in base.get_all_functions() {
        definitions.entry(key(id)).or_default().push(id.clone());
    }
    definitions
}

fn key(id: &FunctionId) -> DefinitionKey {
    (id.file.clone(), id.name.clone(), id.line, id.column)
}

fn remap(source: CallGraph, ids: &HashMap<FunctionId, FunctionId>) -> CallGraph {
    let identity = |id: &FunctionId| ids.get(id).cloned().unwrap_or_else(|| id.clone());
    let mut graph = CallGraph::new();
    for node in source.nodes.values() {
        graph.add_function_with_evidence(
            identity(&node.id),
            node.role_evidence.clone(),
            node.complexity,
            node._lines,
        );
    }
    for evidence in source.edge_evidence() {
        let mut evidence = evidence.clone();
        evidence.call.caller = identity(&evidence.call.caller);
        evidence.call.callee = identity(&evidence.call.callee);
        graph.add_call_with_evidence(evidence);
    }
    for uncertain in source.uncertain_calls() {
        let mut uncertain = uncertain.clone();
        uncertain.caller = identity(&uncertain.caller);
        uncertain.candidates = uncertain.candidates.iter().map(identity).collect();
        graph.record_uncertain_call(uncertain);
    }
    graph
}
