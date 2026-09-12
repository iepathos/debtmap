//! Cached graph identities must match metrics, including same-line module declarations.
use debtmap::builders::call_graph::build_initial_call_graph;
use debtmap::builders::parallel_call_graph::build_call_graph_from_extracted;
use debtmap::extraction::UnifiedFileExtractor;
use debtmap::extraction::adapters::metrics::to_function_metrics;
use debtmap::priority::call_graph::FunctionId;
use std::collections::{BTreeSet, HashMap};
use std::path::PathBuf;

#[test]
fn same_line_inline_modules_share_exact_metric_and_graph_identities() {
    let source = "mod left { pub struct A; impl A {pub fn bar(&self){}} pub fn caller(a:A){a.bar();} } mod right { pub struct A; impl A {pub fn bar(&self){}} pub fn caller(a:A){a.bar();} }";
    let path = PathBuf::from("src/lib.rs");
    let data = UnifiedFileExtractor::extract(&path, source).unwrap();
    let metrics: Vec<_> = data
        .functions
        .iter()
        .map(|f| to_function_metrics(&path, f))
        .collect();
    let base = build_initial_call_graph(&metrics);
    assert_eq!(
        base.node_count(),
        4,
        "Metrics must preserve declaration identity"
    );
    let (graph, _, _) =
        build_call_graph_from_extracted(base, &HashMap::from([(path.clone(), data)]));
    assert_eq!(graph.node_count(), 4, "No orphan summary aliases");
    for module in ["left", "right"] {
        let caller = FunctionId::new(path.clone(), format!("{module}::caller"), 1);
        let target = FunctionId::new(path.clone(), format!("{module}::A::bar"), 1);
        assert_eq!(graph.get_callees_exact(&caller), vec![target.clone()]);
        assert_eq!(graph.get_callers_exact(&target), vec![caller]);
    }
}

#[test]
fn nested_modules_and_same_line_free_functions_keep_qualified_names() {
    let source = "fn bar() {} mod outer { mod inner { struct A; impl A { fn bar(&self) {} } fn caller(a: A) { a.bar(); } } }";
    let path = PathBuf::from("src/lib.rs");
    let data = UnifiedFileExtractor::extract(&path, source).unwrap();
    let names: BTreeSet<_> = data
        .functions
        .iter()
        .map(|f| f.qualified_name.clone())
        .collect();
    assert_eq!(
        names,
        ["bar", "outer::inner::A::bar", "outer::inner::caller"]
            .map(str::to_string)
            .into()
    );
    let metrics: Vec<_> = data
        .functions
        .iter()
        .map(|f| to_function_metrics(&path, f))
        .collect();
    let (graph, _, _) = build_call_graph_from_extracted(
        build_initial_call_graph(&metrics),
        &HashMap::from([(path.clone(), data)]),
    );
    assert_eq!(graph.node_count(), 3);
    let caller = FunctionId::new(path.clone(), "outer::inner::caller".into(), 1);
    assert_eq!(
        graph.get_callees_exact(&caller),
        vec![FunctionId::new(path, "outer::inner::A::bar".into(), 1)]
    );
}
