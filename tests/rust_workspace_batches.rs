//! Source boundaries must not change definitions, evidence, or uncertainty.
use debtmap::{
    analyzers::rust_call_graph::extract_call_graph_multi_file,
    builders::{
        call_graph::process_rust_files_for_call_graph_with_files,
        parallel_call_graph::{ParallelCallGraphBuilder, build_call_graph_from_extracted},
    },
    extraction::UnifiedFileExtractor,
    priority::call_graph::CallGraph,
};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

fn corpus(root: &Path, count: usize) -> Vec<(PathBuf, String)> {
    let mut files = vec![(root.join("lib.rs"), "mod remote;\nfn caller(value: &remote::Remote) { value.run(); }\nfn unknown(value: Missing) { value.run(); }\n".into())];
    files.extend((1..count - 1).map(|i| {
        (
            root.join(format!("filler_{i}.rs")),
            format!("fn filler_{i}() {{}}"),
        )
    }));
    files.push((
        root.join("remote.rs"),
        "pub struct Remote;\nimpl Remote { pub fn run(&self) { let _ = std::fs::read(\"input\"); } }\n"
            .into(),
    ));
    files
}

fn normalized(graph: &CallGraph) -> [Vec<String>; 4] {
    let mut nodes: Vec<_> = graph
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
    let mut evidence: Vec<_> = graph
        .edge_evidence()
        .map(|edge| serde_json::to_string(edge).unwrap())
        .collect();
    let mut uncertain: Vec<_> = graph
        .uncertain_calls()
        .map(|call| serde_json::to_string(call).unwrap())
        .collect();
    let mut effects: Vec<_> = graph
        .effect_assessments()
        .map(|evidence| format!("{evidence:?}"))
        .collect();
    nodes.sort();
    evidence.sort();
    uncertain.sort();
    effects.sort();
    [nodes, evidence, uncertain, effects]
}

fn check_edge(graph: &CallGraph, root: &Path) {
    let caller = graph
        .get_all_functions()
        .find(|id| id.file == root.join("lib.rs") && id.line == 2)
        .unwrap();
    let targets = graph.get_callees_exact(caller);
    assert_eq!(targets.len(), 1);
    assert_eq!(targets[0].file, root.join("remote.rs"));
    assert_eq!(targets[0].line, 2);
    assert_eq!(targets[0].column, Some(21));
    assert!(!graph.uncertain_calls().any(|call| &call.caller == caller));
    let caller_assessment = graph.effect_assessment(caller).expect("caller evidence");
    assert_eq!(
        caller_assessment.dependencies().next().unwrap().target,
        targets[0]
    );
    assert_eq!(
        graph
            .effect_assessment(&targets[0])
            .expect("callee evidence")
            .classification(),
        debtmap::analysis::effect_evidence::EffectClassification::Impure
    );
}

// Stable permutation, independent of the hash-map seed and input order.
fn shuffled_key(path: &Path) -> u64 {
    path.to_string_lossy()
        .bytes()
        .fold(0xcbf29ce484222325, |hash, byte| {
            (hash ^ u64::from(byte)).wrapping_mul(0x100000001b3)
        })
}

#[test]
fn all_builders_resolve_complete_workspaces_across_batch_boundaries() {
    for count in [199, 200, 201, 401] {
        let directory = tempfile::tempdir().unwrap();
        let files = corpus(directory.path(), count);
        for (path, source) in &files {
            std::fs::write(path, source).unwrap();
        }
        let mut paths: Vec<_> = files.iter().map(|(path, _)| path.clone()).collect();
        let parsed: Vec<_> = files
            .iter()
            .map(|(path, source)| (syn::parse_file(source).unwrap(), path.clone()))
            .collect();
        let ast = extract_call_graph_multi_file(&parsed);
        drop(parsed);
        check_edge(&ast, directory.path());
        let extracted: HashMap<_, _> = files
            .iter()
            .map(|(path, source)| {
                (
                    path.clone(),
                    UnifiedFileExtractor::extract(path, source).unwrap(),
                )
            })
            .collect();
        let cached = build_call_graph_from_extracted(CallGraph::new(), &extracted).0;
        check_edge(&cached, directory.path());
        let mut expected = None;
        for order in 0..3 {
            match order {
                1 => paths.reverse(),
                2 => paths.sort_by_key(|path| shuffled_key(path)),
                _ => {}
            }
            let mut sequential = CallGraph::new();
            process_rust_files_for_call_graph_with_files(
                directory.path(),
                &mut sequential,
                false,
                false,
                Some(&paths),
                |_| {},
            )
            .unwrap();
            let parallel = ParallelCallGraphBuilder::new()
                .build_parallel_with_files(directory.path(), CallGraph::new(), Some(&paths), |_| {})
                .unwrap()
                .0;
            check_edge(&parallel, directory.path());
            check_edge(&sequential, directory.path());
            assert_eq!(
                normalized(&sequential),
                normalized(&parallel),
                "count={count}, order={order}"
            );
            if let Some(expected) = &expected {
                assert_eq!(&normalized(&parallel), expected);
            }
            expected = Some(normalized(&parallel));
        }
    }
}

#[test]
fn body_pass_uses_captured_sources_after_files_change() {
    use debtmap::builders::parallel_call_graph::CallGraphPhase;
    let directory = tempfile::tempdir().unwrap();
    let files = corpus(directory.path(), 401);
    for (path, source) in &files {
        std::fs::write(path, source).unwrap();
    }
    let paths: Vec<_> = files.iter().map(|(path, _)| path.clone()).collect();
    let remote = directory.path().join("remote.rs");
    let graph = ParallelCallGraphBuilder::new()
        .build_parallel_with_files(
            directory.path(),
            CallGraph::new(),
            Some(&paths),
            |progress| {
                if matches!(progress.phase, CallGraphPhase::ExtractingCalls)
                    && progress.current == 200
                {
                    std::fs::write(&remote, "pub struct Changed;").unwrap();
                }
            },
        )
        .unwrap()
        .0;
    check_edge(&graph, directory.path());
}
