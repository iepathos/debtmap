//! Exercise the complete Rust pipeline around its 200-file batch boundary.
use criterion::{BenchmarkId, Criterion};
use debtmap::{
    builders::{
        call_graph::process_rust_files_for_call_graph_with_files,
        parallel_call_graph::ParallelCallGraphBuilder,
    },
    priority::call_graph::CallGraph,
};
use std::{hint::black_box, path::PathBuf};

struct Workspace {
    directory: tempfile::TempDir,
    paths: Vec<PathBuf>,
}

fn workspace(count: usize) -> Workspace {
    let directory = tempfile::tempdir().expect("Create benchmark workspace");
    let modules = (0..count - 1)
        .map(|index| format!("mod m{index};\n"))
        .collect::<String>();
    let root = directory.path().join("lib.rs");
    std::fs::write(&root, modules + "pub fn root() { m0::Owner.run(); }\n")
        .expect("Write module root");
    let mut paths = vec![root];
    paths.extend((0..count - 1).map(|index| {
        let path = directory.path().join(format!("m{index}.rs"));
        let following = (index + 1) % (count - 1);
        let source = format!(
            "pub struct Owner;\nimpl Owner {{ pub fn run(&self) {{ crate::m{following}::Owner.run(); }} }}\n"
        );
        std::fs::write(&path, source).expect("Write module source");
        path
    }));
    Workspace { directory, paths }
}

fn resolve(workspace: &Workspace, parallel: bool) -> CallGraph {
    let root = workspace.directory.path();
    let paths = Some(workspace.paths.as_slice());
    if parallel {
        return ParallelCallGraphBuilder::new()
            .build_parallel_with_files(root, CallGraph::new(), paths, |_| {})
            .expect("Build parallel workspace graph")
            .0;
    }
    let mut graph = CallGraph::new();
    process_rust_files_for_call_graph_with_files(root, &mut graph, false, false, paths, |_| {})
        .expect("Build sequential workspace graph");
    graph
}

pub fn bench_workspace_resolution(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("rust_workspace_resolution");
    for count in [199, 200, 201, 401] {
        let workspace = workspace(count);
        for (name, parallel) in [("sequential", false), ("parallel", true)] {
            let graph = resolve(&workspace, parallel);
            assert_eq!(graph.get_all_functions().count(), count);
            assert_eq!(graph.edge_evidence().count(), count);
            assert_eq!(graph.uncertain_calls().count(), 0);
            group.bench_function(BenchmarkId::new(name, count), |bencher| {
                bencher.iter(|| black_box(resolve(black_box(&workspace), parallel)));
            });
        }
    }
    group.finish();
}
