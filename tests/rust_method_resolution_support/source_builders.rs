//! Run source fixtures through both file builders and preserve their fixture identities.
use debtmap::builders::{
    call_graph::process_rust_files_for_call_graph_with_files,
    parallel_call_graph::ParallelCallGraphBuilder,
};
use debtmap::priority::call_graph::{CallGraph, CallSite, FunctionCall, FunctionId};
use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};

type Paths = BTreeMap<PathBuf, PathBuf>;

pub fn graphs(files: &[(&str, &str)]) -> Vec<CallGraph> {
    let directory = tempfile::tempdir().unwrap();
    let mapping = write_sources(directory.path(), files);
    let mut paths: Vec<_> = mapping.keys().cloned().collect();
    let metrics = mapping
        .keys()
        .flat_map(|path| {
            let source = std::fs::read_to_string(path).unwrap();
            let data = debtmap::extraction::UnifiedFileExtractor::extract(path, &source).unwrap();
            debtmap::extraction::adapters::metrics::all_function_metrics(&data)
        })
        .collect::<Vec<_>>();
    let base = debtmap::builders::call_graph::build_initial_call_graph(&metrics);
    let mut sequential = base.clone();
    let (sequential_exclusions, sequential_used) = process_rust_files_for_call_graph_with_files(
        directory.path(),
        &mut sequential,
        false,
        false,
        Some(&paths),
        |_| {},
    )
    .unwrap();
    paths.reverse();
    let (parallel, parallel_exclusions, parallel_used) = ParallelCallGraphBuilder::new()
        .build_parallel_with_files(directory.path(), base, Some(&paths), |_| {})
        .unwrap();
    assert_eq!(
        normalized(&sequential),
        normalized(&parallel),
        "full sequential/parallel graph differs"
    );
    assert_eq!(
        sequential.node_count(),
        metrics.len(),
        "source/metric merge introduced duplicate nodes"
    );
    for metric in &metrics {
        let id = FunctionId::new(metric.file.clone(), metric.name.clone(), metric.line)
            .with_column(metric.column);
        assert_eq!(
            sequential
                .get_function_info(&id)
                .map(|info| (info.2, info.3)),
            Some((metric.cyclomatic, metric.length))
        );
    }
    let mut merged = parallel.clone();
    merged.merge(sequential.clone());
    merged.merge(parallel.clone());
    assert_eq!(normalized(&merged), normalized(&parallel));
    assert_eq!(sequential_exclusions, parallel_exclusions);
    assert_eq!(sequential_used, parallel_used);
    vec![rebase(&sequential, &mapping), rebase(&parallel, &mapping)]
}

fn write_sources(root: &Path, files: &[(&str, &str)]) -> Paths {
    files
        .iter()
        .map(|(file, source)| {
            let requested = PathBuf::from(file);
            let path = root.join(requested.strip_prefix("/").unwrap_or(&requested));
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, source).unwrap();
            (path, requested)
        })
        .collect()
}

fn normalized(graph: &CallGraph) -> [Vec<String>; 4] {
    let nodes = graph
        .get_all_functions()
        .map(|id| {
            serde_json::to_string(&(
                id,
                graph.get_function_info(id),
                graph.get_roles(id),
                graph.get_role_evidence(id),
            ))
            .unwrap()
        })
        .collect();
    let calls = graph
        .get_all_calls()
        .iter()
        .map(|call| serde_json::to_string(call).unwrap())
        .collect();
    let evidence = graph
        .edge_evidence()
        .map(|edge| serde_json::to_string(edge).unwrap())
        .collect();
    let uncertainty = graph
        .uncertain_calls()
        .map(|call| serde_json::to_string(call).unwrap())
        .collect();
    [nodes, calls, evidence, uncertainty].map(|mut values: Vec<String>| {
        values.sort();
        values
    })
}

fn rebase(graph: &CallGraph, paths: &Paths) -> CallGraph {
    let mut rebased = CallGraph::new();
    for id in graph.get_all_functions() {
        let (_, _, complexity, lines) = graph.get_function_info(id).unwrap();
        rebased.add_function_with_evidence(
            rebase_id(id, paths),
            graph.get_role_evidence(id).unwrap().clone(),
            complexity,
            lines,
        );
    }
    copy_calls(graph, &mut rebased, paths);
    for call in graph.uncertain_calls() {
        let mut call = call.clone();
        call.caller = rebase_id(&call.caller, paths);
        call.call_site = rebase_site(&call.call_site, paths);
        call.candidates = call
            .candidates
            .iter()
            .map(|id| rebase_id(id, paths))
            .collect();
        call.receiver = call.receiver.map(|text| rebase_diagnostic(text, paths));
        rebased.record_uncertain_call(call);
    }
    rebased
}

fn copy_calls(graph: &CallGraph, rebased: &mut CallGraph, paths: &Paths) {
    let covered: HashSet<_> = graph
        .edge_evidence()
        .map(|edge| edge.call.clone())
        .collect();
    for evidence in graph.edge_evidence() {
        let mut evidence = evidence.clone();
        evidence.call = rebase_call(&evidence.call, paths);
        evidence.call_site = evidence
            .call_site
            .as_ref()
            .map(|site| rebase_site(site, paths));
        rebased.add_call_with_evidence(evidence);
    }
    for call in graph
        .get_all_calls()
        .iter()
        .filter(|call| !covered.contains(*call))
    {
        rebased.add_call(rebase_call(call, paths));
    }
}

fn rebase_id(id: &FunctionId, paths: &Paths) -> FunctionId {
    FunctionId {
        file: rebase_path(&id.file, paths),
        ..id.clone()
    }
}

fn rebase_call(call: &FunctionCall, paths: &Paths) -> FunctionCall {
    FunctionCall {
        caller: rebase_id(&call.caller, paths),
        callee: rebase_id(&call.callee, paths),
        ..call.clone()
    }
}

fn rebase_site(site: &CallSite, paths: &Paths) -> CallSite {
    CallSite {
        file: rebase_path(&site.file, paths),
        ..site.clone()
    }
}

fn rebase_path(path: &Path, paths: &Paths) -> PathBuf {
    paths
        .get(path)
        .unwrap_or_else(|| panic!("graph identity outside source fixture: {path:?}"))
        .clone()
}

fn rebase_diagnostic(text: String, paths: &Paths) -> String {
    paths.iter().fold(text, |text, (actual, requested)| {
        text.replace(actual.to_str().unwrap(), requested.to_str().unwrap())
    })
}
