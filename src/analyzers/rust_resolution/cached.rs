//! Complete Rust workspace resolution from thread-safe extraction snapshots.

use crate::core::Language;
use crate::extraction::{ExtractedFileData, ExtractedFunctionData};
use crate::priority::call_graph::{CallGraph, FunctionId};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

/// Resolve available Rust snapshots and retain existing extracted node identities.
pub(crate) fn extract(
    extracted: &HashMap<PathBuf, ExtractedFileData>,
) -> (CallGraph, HashSet<PathBuf>) {
    let mut paths: Vec<_> = extracted.keys().collect();
    paths.sort();
    let files: Vec<_> = paths
        .into_iter()
        .filter_map(|path| parse_snapshot(path, &extracted[path]))
        .collect();
    let available = files.iter().map(|(path, _)| path.clone()).collect();
    if files.is_empty() {
        return (CallGraph::new(), available);
    }
    let graph = super::extract(&files);
    (remap_graph(graph, extracted), available)
}

fn parse_snapshot(path: &Path, data: &ExtractedFileData) -> Option<(PathBuf, syn::File)> {
    if Language::from_path(path) != Language::Rust {
        return None;
    }
    let source = data.rust_source.as_ref()?;
    match syn::parse_file(source) {
        Ok(ast) => Some((path.to_path_buf(), ast)),
        Err(error) => {
            log::warn!(
                "Cannot resolve cached Rust source {}: {error}",
                path.display()
            );
            None
        }
    }
}

fn remap_graph(graph: CallGraph, extracted: &HashMap<PathBuf, ExtractedFileData>) -> CallGraph {
    let identities: HashMap<_, _> = graph
        .get_all_functions()
        .map(|id| (id.clone(), extracted_identity(id, extracted)))
        .collect();
    let identity = |id: &FunctionId| identities.get(id).cloned().unwrap_or_else(|| id.clone());
    let mut remapped = CallGraph::new();
    for node in graph.nodes.values() {
        add_node(
            &mut remapped,
            (node.role_evidence.clone(), node.complexity, node._lines),
            identity(&node.id),
            extracted,
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
    remapped
}

fn add_node(
    graph: &mut CallGraph,
    metadata: (crate::analysis::role_policy::RoleEvidence, u32, usize),
    id: FunctionId,
    extracted: &HashMap<PathBuf, ExtractedFileData>,
) {
    let function = extracted.get(&id.file).and_then(|file| {
        let mut matches = file.functions.iter().filter(|function| {
            function.line == id.line
                && (function.column.is_none() || function.column == id.column)
                && function.qualified_name == id.name
        });
        let first = matches.next()?;
        matches.next().is_none().then_some(first)
    });
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

fn extracted_identity(
    id: &FunctionId,
    extracted: &HashMap<PathBuf, ExtractedFileData>,
) -> FunctionId {
    let candidates: Vec<_> = extracted
        .get(&id.file)
        .into_iter()
        .flat_map(|file| &file.functions)
        .filter(|function| {
            function.line == id.line
                && (function.column.is_none() || function.column == id.column)
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
        [function] => FunctionId::new(id.file.clone(), function.qualified_name.clone(), id.line)
            .with_column(function.column.or(id.column)),
        _ => id.clone(),
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
