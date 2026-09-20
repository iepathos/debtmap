//! Complete Rust workspace resolution from thread-safe extraction snapshots.

use crate::core::Language;
use crate::extraction::{ExtractedFileData, ExtractedFunctionData};
use crate::priority::call_graph::{CallGraph, FunctionId};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

#[path = "cached_enhancement.rs"]
mod enhancement;
pub(crate) use enhancement::EnhancedExtraction;

/// Resolve available Rust snapshots and retain existing extracted node identities.
pub(crate) fn extract(
    extracted: &HashMap<PathBuf, ExtractedFileData>,
) -> (CallGraph, HashSet<PathBuf>) {
    let result = extract_with_enhancement(extracted);
    (result.graph, result.available)
}

pub(crate) fn extract_with_enhancement(
    extracted: &HashMap<PathBuf, ExtractedFileData>,
) -> EnhancedExtraction {
    let mut builder = crate::analysis::call_graph::RustCallGraphBuilder::new();
    let (graph, available) =
        super::workspace::extract_sources(&source_snapshots(extracted), |batch| {
            enhancement::collect(&mut builder, batch);
        });
    enhancement::finish(builder, remap_graph(graph, extracted), available)
}

fn source_snapshots(extracted: &HashMap<PathBuf, ExtractedFileData>) -> Vec<(PathBuf, String)> {
    let mut sources: Vec<_> = extracted
        .iter()
        .filter(|(path, _)| Language::from_path(path) == Language::Rust)
        .filter_map(|(path, data)| {
            data.rust_source
                .as_ref()
                .map(|source| (path.clone(), source.clone()))
        })
        .collect();
    sources.sort_by(|left, right| left.0.cmp(&right.0));
    sources
}

fn remap_graph(graph: CallGraph, extracted: &HashMap<PathBuf, ExtractedFileData>) -> CallGraph {
    let counts = legacy_source_counts(&graph);
    let functions: HashMap<_, _> = graph
        .get_all_functions()
        .map(|id| (id.clone(), extracted_function(id, extracted, &counts)))
        .collect();
    let identities: HashMap<_, _> = graph
        .get_all_functions()
        .map(|id| (id.clone(), extracted_identity(id, functions[id])))
        .collect();
    let identity = |id: &FunctionId| identities.get(id).cloned().unwrap_or_else(|| id.clone());
    let mut remapped = CallGraph::new();
    for node in graph.nodes.values() {
        add_node(
            &mut remapped,
            (node.role_evidence.clone(), node.complexity, node._lines),
            identity(&node.id),
            functions[&node.id],
        );
    }
    for evidence in graph.edge_evidence() {
        let mut evidence = evidence.clone();
        evidence.call.caller = identity(&evidence.call.caller);
        evidence.call.callee = identity(&evidence.call.callee);
        remapped.add_call_with_evidence(evidence);
    }
    for call in graph.uncertain_calls() {
        let mut call = call.clone();
        call.caller = identity(&call.caller);
        call.candidates = call.candidates.iter().map(identity).collect();
        remapped.record_uncertain_call(call);
    }
    for (id, assessment) in graph.effect_assessments() {
        remapped.record_effect_assessment(identity(id), assessment.remap_identities(identity));
    }
    remapped
}

fn add_node(
    graph: &mut CallGraph,
    metadata: (crate::analysis::role_policy::RoleEvidence, u32, usize),
    id: FunctionId,
    function: Option<&ExtractedFunctionData>,
) {
    let Some(function) = function else {
        graph.add_function_with_evidence(id, metadata.0, metadata.1, metadata.2);
        return;
    };
    let facts =
        crate::analysis::role_policy::evidence_for_facts(crate::analysis::role_policy::RoleFacts {
            path: &id.file,
            language: Language::Rust,
            name: &function.qualified_name,
            is_test: function.is_test,
            in_test_module: function.in_test_module,
            visibility: function.visibility.as_deref(),
        });
    let evidence = crate::analysis::role_policy::merge_evidence(&facts, &function.role_evidence);
    let evidence = crate::analysis::role_policy::merge_evidence(&evidence, &metadata.0);
    graph.add_function_with_evidence(id, evidence, function.cyclomatic, function.length);
}

type LegacyKey = (PathBuf, String, usize);

fn legacy_source_counts(graph: &CallGraph) -> HashMap<LegacyKey, usize> {
    let mut counts = HashMap::new();
    for id in graph.get_all_functions() {
        let segments: Vec<_> = id.name.split("::").collect();
        for position in 0..segments.len() {
            let key = (id.file.clone(), segments[position..].join("::"), id.line);
            *counts.entry(key).or_default() += 1;
        }
    }
    counts
}

fn extracted_function<'a>(
    id: &FunctionId,
    extracted: &'a HashMap<PathBuf, ExtractedFileData>,
    counts: &HashMap<LegacyKey, usize>,
) -> Option<&'a ExtractedFunctionData> {
    let candidates: Vec<_> = extracted
        .get(&id.file)
        .into_iter()
        .flat_map(|file| &file.functions)
        .filter(|function| {
            function.line == id.line
                && (function.column == id.column
                    || (function.column.is_none()
                        && counts.get(&(
                            id.file.clone(),
                            function.qualified_name.clone(),
                            id.line,
                        )) == Some(&1)))
                && same_source_name(&id.name, function)
        })
        .collect();
    let exact: Vec<_> = candidates
        .iter()
        .copied()
        .filter(|function| function.qualified_name == id.name)
        .collect();
    let matches = if exact.is_empty() {
        &candidates
    } else {
        &exact
    };
    match matches.as_slice() {
        [function] => Some(function),
        _ => None,
    }
}

fn extracted_identity(id: &FunctionId, function: Option<&ExtractedFunctionData>) -> FunctionId {
    match function {
        Some(function) => {
            FunctionId::new(id.file.clone(), function.qualified_name.clone(), id.line)
                .with_column(function.column.or(id.column))
        }
        None => id.clone(),
    }
}

fn same_source_name(source: &str, function: &ExtractedFunctionData) -> bool {
    let source: Vec<_> = source.split("::").collect();
    let extracted: Vec<_> = function.qualified_name.split("::").collect();
    source.ends_with(&extracted)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extraction::UnifiedFileExtractor;
    use std::collections::BTreeSet;

    #[test]
    fn generic_impl_snapshots_preserve_extracted_nodes_metrics_and_return_chain_edges() {
        let source = "struct Foo;\nimpl Foo { fn bar(&self) {} }\nstruct Wrapper<T>(T);\nimpl<T> Wrapper<T> { fn get(&self) -> &T { &self.0 } }\ntrait Work { fn run(&self); }\nimpl<T> Work for Wrapper<T> { fn run(&self) { helper(); } }\nfn helper() {}\npub fn caller(value: &Wrapper<Foo>) { value.get().bar(); value.run(); }\n";
        let path = PathBuf::from("src/lib.rs");
        let mut data = UnifiedFileExtractor::extract(&path, source).expect("fixture parses");
        for (position, function) in data.functions.iter_mut().enumerate() {
            function.cyclomatic = position as u32 + 7;
            function.length = position + 11;
        }
        assert!(
            data.functions
                .iter()
                .any(|function| function.qualified_name == "Wrapper::get")
        );
        let extracted = HashMap::from([(path.clone(), data.clone())]);
        let (graph, available) = extract(&extracted);
        assert!(available.contains(&path));
        assert_eq!(graph.node_count(), data.functions.len());
        for function in &data.functions {
            let id = FunctionId::new(path.clone(), function.qualified_name.clone(), function.line)
                .with_column(function.column);
            let node = graph.nodes.get(&id).expect("exact extracted node identity");
            assert_eq!(node.complexity, function.cyclomatic);
            assert_eq!(node._lines, function.length);
        }
        let caller = data
            .functions
            .iter()
            .find(|function| function.name == "caller")
            .expect("caller");
        let caller = FunctionId::new(path.clone(), caller.qualified_name.clone(), caller.line)
            .with_column(caller.column);
        let targets: BTreeSet<_> = graph
            .get_callees_exact(&caller)
            .into_iter()
            .map(|id| id.name)
            .collect();
        assert_eq!(
            targets,
            ["Foo::bar", "Wrapper::get", "Wrapper::run"]
                .map(str::to_string)
                .into()
        );
        assert!(graph.nodes[&caller].roles.is_public_api);
        assert!(graph.uncertain_calls().next().is_none());
    }
}
