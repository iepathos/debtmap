//! Rust source positions remain authoritative across extraction and enhancement.

use debtmap::analysis::call_graph::RustCallGraphBuilder;
use debtmap::analyzers::rust_call_graph::extract_call_graph;
use debtmap::extraction::{UnifiedFileExtractor, adapters};
use debtmap::priority::call_graph::{CallGraph, FunctionId};
use std::collections::{BTreeSet, HashMap};
use std::path::PathBuf;

const SOURCE: &str = "struct Foo; trait First { fn run(&self); } trait Second { fn run(&self); } impl First for Foo { fn run(&self) { first(); } } impl Second for Foo { fn run(&self) { second(); } } fn first() {} fn second() {}";

fn implementation_ids(graph: &CallGraph) -> Vec<FunctionId> {
    let mut ids: Vec<_> = graph
        .get_all_functions()
        .filter(|id| id.name == "Foo::run")
        .cloned()
        .collect();
    ids.sort();
    ids
}

fn assert_separate_bodies(graph: &CallGraph) {
    let ids = implementation_ids(graph);
    assert_eq!(ids.len(), 2);
    assert_eq!(ids[0].line, ids[1].line);
    assert_ne!(ids[0].column, ids[1].column);
    assert_eq!(
        graph
            .get_callees_exact(&ids[0])
            .iter()
            .map(|id| id.name.as_str())
            .collect::<Vec<_>>(),
        ["first"]
    );
    assert_eq!(
        graph
            .get_callees_exact(&ids[1])
            .iter()
            .map(|id| id.name.as_str())
            .collect::<Vec<_>>(),
        ["second"]
    );
    assert!(
        graph
            .find_function(&ids[0].clone().with_column(None))
            .is_none()
    );
    assert!(
        graph
            .find_function_at_location(&ids[0].file, ids[0].line)
            .is_none()
    );
}

#[test]
fn same_line_implementations_keep_separate_nodes_edges_and_metrics() {
    let path = PathBuf::from("src/lib.rs");
    let ast = syn::parse_file(SOURCE).unwrap();
    let direct = extract_call_graph(&ast, &path);
    assert_separate_bodies(&direct);
    let mut extracted = UnifiedFileExtractor::extract(&path, SOURCE).unwrap();
    for (ordinal, function) in extracted.functions.iter_mut().enumerate() {
        function.cyclomatic = ordinal as u32 + 10;
    }
    let cached =
        adapters::call_graph::build_call_graph(&HashMap::from([(path.clone(), extracted.clone())]));
    assert_separate_bodies(&cached);
    let metrics = adapters::metrics::all_function_metrics(&extracted);
    for metric in metrics.iter().filter(|metric| metric.name == "Foo::run") {
        let query = FunctionId::new(path.clone(), metric.name.clone(), metric.line)
            .with_column(metric.column);
        let id = cached.find_function(&query).unwrap();
        assert_eq!(cached.get_function_info(&id).unwrap().2, metric.cyclomatic);
    }
    let mut merged = cached.clone();
    merged.merge(cached);
    assert_separate_bodies(&merged);
}

#[test]
fn inline_trait_enhancement_updates_canonical_nodes_without_ghosts() {
    let source = "mod nested { struct Foo; impl Default for Foo { fn default() -> Self { helper(); Foo } } fn helper() {} fn visit_item() {} }";
    let path = PathBuf::from("src/lib.rs");
    let ast = syn::parse_file(source).unwrap();
    let graph = extract_call_graph(&ast, &path);
    let ids: BTreeSet<_> = graph.get_all_functions().cloned().collect();
    let mut builder = RustCallGraphBuilder::from_base_graph(graph);
    builder.analyze_trait_dispatch(&path, &ast).unwrap();
    builder.analyze_framework_patterns(&path, &ast).unwrap();
    builder.finalize_trait_analysis().unwrap();
    builder.finalize_trait_analysis().unwrap();
    let enhanced = builder.build();
    assert_eq!(
        ids,
        enhanced.base_graph.get_all_functions().cloned().collect()
    );
    let method = ids
        .iter()
        .find(|id| id.name == "nested::Foo::default")
        .unwrap();
    assert!(enhanced.base_graph.is_entry_point(method));
    assert!(enhanced.trait_registry.has_trait_implementations(method));
    let visitor = ids
        .iter()
        .find(|id| id.name == "nested::visit_item")
        .unwrap();
    assert!(
        enhanced
            .framework_patterns
            .get_exclusions()
            .contains(visitor)
    );
    assert!(
        enhanced
            .base_graph
            .get_all_functions()
            .all(|id| id.name != "Foo::default")
    );
}

#[test]
fn unknown_enhancement_metadata_does_not_create_a_definition() {
    let mut graph = CallGraph::new();
    graph.mark_as_trait_dispatch(FunctionId::new("test.rs".into(), "Foo::default".into(), 1));
    assert_eq!(graph.node_count(), 0);
}

#[test]
fn legacy_json_defaults_column_and_current_postcard_retains_it() {
    let legacy = r#"{"file":"test.rs","name":"Foo::run","line":1}"#;
    let id: FunctionId = serde_json::from_str(legacy).unwrap();
    assert_eq!(id.column, None);
    let id = id.with_column(Some(37));
    let bytes = postcard::to_allocvec(&id).unwrap();
    assert_eq!(postcard::from_bytes::<FunctionId>(&bytes).unwrap(), id);
    let extracted = UnifiedFileExtractor::extract(&PathBuf::from("test.rs"), SOURCE).unwrap();
    let function = &extracted.functions[0];
    let mut legacy = serde_json::to_value(function).unwrap();
    legacy.as_object_mut().unwrap().remove("column");
    let legacy: debtmap::extraction::ExtractedFunctionData =
        serde_json::from_value(legacy).unwrap();
    assert_eq!(legacy.column, None);
    let bytes = postcard::to_allocvec(function).unwrap();
    let restored: debtmap::extraction::ExtractedFunctionData =
        postcard::from_bytes(&bytes).unwrap();
    assert_eq!(restored.column, function.column);
}

#[test]
fn same_line_trait_roles_attach_to_the_exact_implementation() {
    let source = "struct Foo; trait Alternate { fn default() -> Self; } impl Default for Foo { fn default() -> Self { Foo } } impl Alternate for Foo { fn default() -> Self { Foo } }";
    let path = PathBuf::from("src/lib.rs");
    let ast = syn::parse_file(source).unwrap();
    let graph = extract_call_graph(&ast, &path);
    let mut methods: Vec<_> = graph
        .get_all_functions()
        .filter(|id| id.name == "Foo::default")
        .cloned()
        .collect();
    methods.sort();
    assert_eq!(methods.len(), 2);
    let mut builder = RustCallGraphBuilder::from_base_graph(graph);
    builder.analyze_trait_dispatch(&path, &ast).unwrap();
    builder.finalize_trait_analysis().unwrap();
    let enhanced = builder.build();
    assert!(enhanced.base_graph.is_entry_point(&methods[0]));
    assert!(!enhanced.base_graph.is_entry_point(&methods[1]));
    assert!(
        enhanced
            .trait_registry
            .has_trait_implementations(&methods[0])
    );
    assert!(
        enhanced
            .trait_registry
            .has_trait_implementations(&methods[1])
    );
}
