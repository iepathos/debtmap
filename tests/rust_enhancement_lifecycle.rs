//! Public extraction entry points must return complete, canonical enhancements.
use debtmap::{
    analysis::call_graph::{RustCallGraph, RustCallGraphBuilder, effects::build_call_graph_result},
    analyzers::rust_call_graph::extract_call_graph,
    config::DebtmapConfig,
    priority::call_graph::FunctionId,
};
use std::path::PathBuf;

const SOURCE: &str = "mod nested {\nstruct Foo;\ntrait Visit { fn custom(&self); }\nimpl Visit for Foo { fn custom(&self) { helper(); } }\nfn helper() {}\nfn caller() { let callback: fn() = helper; callback(); }\n}\n";

fn definition(graph: &RustCallGraph, line: usize) -> FunctionId {
    let ids: Vec<_> = graph
        .base_graph
        .get_all_functions()
        .filter(|id| id.line == line)
        .collect();
    assert_eq!(ids.len(), 1);
    ids[0].clone()
}

fn assert_complete(graph: &RustCallGraph) {
    assert_eq!(graph.base_graph.node_count(), 3);
    let visitor = definition(graph, 4);
    assert!(visitor.column.is_some());
    assert!(graph.framework_patterns.get_exclusions().contains(&visitor));
    assert_eq!(
        graph.base_graph.get_callees_exact(&visitor),
        vec![definition(graph, 5)]
    );
}

#[test]
fn effects_entry_point_finishes_canonical_enhancement() {
    let ast = syn::parse_file(SOURCE).unwrap();
    let graph = build_call_graph_result(
        &PathBuf::from("src/lib.rs"),
        &ast,
        &DebtmapConfig::default(),
    )
    .unwrap();
    assert_complete(&graph);
}

#[test]
fn automatic_finalization_respects_disabled_trait_analysis() {
    let ast = syn::parse_file("fn new() {}").unwrap();
    let mut config = DebtmapConfig::default();
    config
        .analysis
        .get_or_insert_with(Default::default)
        .enable_trait_analysis = Some(false);
    let graph = build_call_graph_result(&PathBuf::from("src/lib.rs"), &ast, &config).unwrap();
    assert!(!graph.base_graph.is_entry_point(&definition(&graph, 1)));
}

fn builder() -> RustCallGraphBuilder {
    let path = PathBuf::from("src/lib.rs");
    let ast = syn::parse_file(SOURCE).unwrap();
    let mut builder = RustCallGraphBuilder::from_base_graph(extract_call_graph(&ast, &path));
    builder.analyze_trait_dispatch(&path, &ast).unwrap();
    builder.analyze_function_pointers(&path, &ast).unwrap();
    builder.analyze_framework_patterns(&path, &ast).unwrap();
    builder
}

#[test]
fn public_build_finishes_without_explicit_finalization() {
    assert_complete(&builder().build());
}

#[test]
fn repeated_finalization_and_build_preserve_one_set_of_patterns() {
    let automatic = builder().build();
    let mut explicit = builder();
    explicit.finalize_trait_analysis().unwrap();
    explicit.finalize_trait_analysis().unwrap();
    let explicit = explicit.build();
    assert_complete(&explicit);
    assert_eq!(
        automatic.framework_patterns.get_detected_patterns().len(),
        explicit.framework_patterns.get_detected_patterns().len()
    );
    assert_eq!(
        automatic.base_graph.get_all_calls(),
        explicit.base_graph.get_all_calls()
    );
}

#[test]
fn collecting_more_metadata_after_finalization_updates_build_once() {
    let source =
        "struct Foo; trait Visit { fn custom(&self); } impl Visit for Foo { fn custom(&self) {} }";
    let files: Vec<_> = ["src/first.rs", "src/second.rs"]
        .into_iter()
        .map(|path| (syn::parse_file(source).unwrap(), PathBuf::from(path)))
        .collect();
    let base = debtmap::analyzers::rust_call_graph::extract_call_graph_multi_file(&files);
    let mut builder = RustCallGraphBuilder::from_base_graph(base);
    builder
        .analyze_trait_dispatch(&files[0].1, &files[0].0)
        .unwrap();
    builder.finalize_trait_analysis().unwrap();
    builder
        .analyze_trait_dispatch(&files[1].1, &files[1].0)
        .unwrap();
    let graph = builder.build();
    assert_eq!(graph.base_graph.node_count(), 2);
    assert_eq!(graph.framework_patterns.get_detected_patterns().len(), 2);
    for id in graph.base_graph.get_all_functions() {
        assert!(graph.framework_patterns.get_exclusions().contains(id));
    }
}
