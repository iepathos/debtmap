//! Exercise public extraction paths on identical source contents.
use debtmap::{
    analysis::call_graph::RustCallGraphBuilder,
    analyzers::rust_call_graph::extract_call_graph_multi_file,
    builders::{
        call_graph::{build_initial_call_graph, process_rust_files_for_call_graph_with_files},
        parallel_call_graph::{ParallelCallGraphBuilder, build_call_graph_from_extracted},
    },
    extraction::{UnifiedFileExtractor, adapters},
    priority::call_graph::CallGraph,
};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

pub fn graphs(root: &Path, files: &[(PathBuf, String)]) -> Vec<(&'static str, CallGraph)> {
    let parsed: Vec<_> = files
        .iter()
        .map(|(path, source)| {
            (
                syn::parse_file(source).expect("parse matrix source"),
                path.clone(),
            )
        })
        .collect();
    let direct = extract_call_graph_multi_file(&parsed);
    let mut enhancement = RustCallGraphBuilder::from_base_graph(direct.clone());
    for (ast, path) in &parsed {
        enhancement.analyze_trait_dispatch(path, ast).unwrap();
        enhancement.analyze_framework_patterns(path, ast).unwrap();
    }
    enhancement.finalize_trait_analysis().unwrap();
    enhancement.finalize_trait_analysis().unwrap();
    let enhanced = enhancement.build().base_graph;
    drop(parsed); // Cached/source builders reset thread-local span storage.
    let extracted: HashMap<_, _> = files
        .iter()
        .map(|(path, source)| {
            (
                path.clone(),
                UnifiedFileExtractor::extract(path, source).unwrap(),
            )
        })
        .collect();
    let metrics: Vec<_> = extracted
        .values()
        .flat_map(adapters::metrics::all_function_metrics)
        .collect();
    let base = build_initial_call_graph(&metrics);
    let mut sequential = base.clone();
    let paths: Vec<_> = files.iter().map(|(path, _)| path.clone()).collect();
    process_rust_files_for_call_graph_with_files(
        root,
        &mut sequential,
        false,
        false,
        Some(&paths),
        |_| {},
    )
    .unwrap();
    let reversed: Vec<_> = paths.iter().rev().cloned().collect();
    let parallel = ParallelCallGraphBuilder::new()
        .build_parallel_with_files(root, base, Some(&reversed), |_| {})
        .unwrap()
        .0;
    vec![
        ("direct", direct),
        ("enhanced", enhanced),
        (
            "cached-adapter",
            adapters::call_graph::build_call_graph(&extracted),
        ),
        (
            "cached-builder",
            build_call_graph_from_extracted(CallGraph::new(), &extracted).0,
        ),
        ("sequential", sequential),
        ("parallel-reversed", parallel),
    ]
}

pub fn normalized(graph: &CallGraph) -> [Vec<String>; 4] {
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
    [nodes, calls, evidence, uncertainty].map(|mut entries: Vec<String>| {
        entries.sort();
        entries
    })
}
