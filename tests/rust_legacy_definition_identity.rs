//! Missing legacy columns may be upgraded only for unique source definitions.
use debtmap::builders::call_graph::{
    build_initial_call_graph, process_rust_files_for_call_graph_with_files,
};
use debtmap::builders::parallel_call_graph::build_call_graph_from_extracted;
use debtmap::extraction::adapters::metrics::all_function_metrics;
use debtmap::extraction::{ExtractedFileData, UnifiedFileExtractor};
use debtmap::priority::call_graph::CallGraph;
use std::collections::HashMap;
use std::path::PathBuf;

const UNIQUE: &str = "fn target() {} fn caller() { target(); }";
const TIED: &str = "trait A { fn run(&self); } trait B { fn run(&self); } struct Foo; impl A for Foo { fn run(&self) {} } impl B for Foo { fn run(&self) {} }";

fn legacy_data(source: &str) -> ExtractedFileData {
    let mut data = UnifiedFileExtractor::extract(&PathBuf::from("src/lib.rs"), source).unwrap();
    for function in &mut data.functions {
        function.column = None;
        function.cyclomatic = 77;
    }
    data
}

fn assert_unique(graph: &CallGraph) {
    assert_eq!(graph.node_count(), 2);
    assert!(graph.get_all_functions().all(|id| id.column.is_some()));
    for id in graph.get_all_functions() {
        assert_eq!(graph.get_function_info(id).unwrap().2, 77);
    }
    let caller = graph
        .get_all_functions()
        .find(|id| id.name == "caller")
        .unwrap();
    let targets = graph.get_callees_exact(caller);
    assert_eq!(targets.len(), 1);
    assert_eq!(targets[0].name, "target");
}

#[test]
fn unique_legacy_cached_records_upgrade_to_source_columns() {
    let data = legacy_data(UNIQUE);
    let base = build_initial_call_graph(&all_function_metrics(&data));
    let extracted = HashMap::from([(data.path.clone(), data)]);
    let (graph, _, _) = build_call_graph_from_extracted(base, &extracted);
    assert_unique(&graph);
    assert_unique(&debtmap::extraction::adapters::call_graph::build_call_graph(&extracted));
}

#[test]
fn unique_legacy_source_metrics_upgrade_to_source_columns() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("lib.rs");
    std::fs::write(&path, UNIQUE).unwrap();
    let mut data = legacy_data(UNIQUE);
    data.path = path.clone();
    let mut graph = build_initial_call_graph(&all_function_metrics(&data));
    process_rust_files_for_call_graph_with_files(
        directory.path(),
        &mut graph,
        false,
        false,
        Some(&[path]),
        |_| {},
    )
    .unwrap();
    assert_unique(&graph);
}

#[test]
fn tied_legacy_metrics_never_attach_to_either_source_definition() {
    let data = legacy_data(TIED);
    assert_eq!(data.functions.len(), 2);
    let base = build_initial_call_graph(&all_function_metrics(&data));
    let extracted = HashMap::from([(data.path.clone(), data)]);
    let (graph, _, _) = build_call_graph_from_extracted(base, &extracted);
    let exact: Vec<_> = graph
        .get_all_functions()
        .filter(|id| id.column.is_some())
        .collect();
    assert_eq!(exact.len(), 2);
    assert_ne!(exact[0], exact[1]);
    for id in exact {
        assert_eq!(graph.get_function_info(id).unwrap().2, 0);
    }
    let legacy = graph
        .get_all_functions()
        .find(|id| id.column.is_none())
        .unwrap();
    assert_eq!(graph.get_function_info(legacy).unwrap().2, 77);
    assert!(graph.get_callers_exact(legacy).is_empty());
    assert!(graph.find_function(legacy).is_none());
}

#[test]
fn same_line_debt_profiles_keep_definition_columns() {
    use debtmap::priority::debt_aggregator::{DebtAggregator, FunctionId};
    let left = FunctionId::new(PathBuf::from("lib.rs"), "Foo::run".into(), 1).with_column(Some(10));
    let right =
        FunctionId::new(PathBuf::from("lib.rs"), "Foo::run".into(), 1).with_column(Some(40));
    let mut aggregator = DebtAggregator::new();
    aggregator.aggregate_debt(vec![], &[(left.clone(), 1, 1), (right.clone(), 1, 1)]);
    assert_eq!(aggregator.get_profile(&left).unwrap().function_id, left);
    assert_eq!(aggregator.get_profile(&right).unwrap().function_id, right);
}

#[test]
fn canonical_cached_merge_keeps_base_metrics_and_source_role_evidence() {
    let path = PathBuf::from("src/lib.rs");
    let source = "struct Foo; impl Default for Foo { fn default() -> Self { helper(); Foo } } fn helper() {}";
    let data = UnifiedFileExtractor::extract(&path, source).unwrap();
    let mut metrics = all_function_metrics(&data);
    for metric in &mut metrics {
        metric.cyclomatic = 77;
    }
    let (graph, _, _) = build_call_graph_from_extracted(
        build_initial_call_graph(&metrics),
        &HashMap::from([(path, data)]),
    );
    assert_eq!(graph.node_count(), 2);
    for id in graph.get_all_functions() {
        assert_eq!(graph.get_function_info(id).unwrap().2, 77);
    }
    let constructor = graph
        .get_all_functions()
        .find(|id| id.name == "Foo::default")
        .unwrap();
    assert!(graph.is_entry_point(constructor));
    assert_eq!(graph.get_callees_exact(constructor).len(), 1);
}
