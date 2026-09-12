//! Legacy extraction summaries retain occurrence identity without inventing columns.
use debtmap::builders::parallel_call_graph::build_call_graph_from_extracted;
use debtmap::extraction::{UnifiedFileExtractor, adapters::call_graph::build_call_graph};
use debtmap::priority::{call_graph::CallGraph, parallel_call_graph::ParallelCallGraph};
use std::{collections::HashMap, path::Path};

fn graphs() -> [CallGraph; 2] {
    let source = "pub fn caller(a: Missing) { a.hit(); helper(); a.hit(); } fn helper() {}";
    let mut data = UnifiedFileExtractor::extract(Path::new("src/lib.rs"), source).unwrap();
    data.rust_source = None;
    let files = HashMap::from([(data.path.clone(), data)]);
    [
        build_call_graph_from_extracted(CallGraph::new(), &files).0,
        build_call_graph(&files),
    ]
}

#[test]
fn repeated_legacy_calls_remain_distinct_in_both_builders_and_merges() {
    let [source, adapter] = graphs();
    assert_eq!(source.uncertain_calls().count(), 2);
    assert_eq!(
        source.uncertain_calls().collect::<Vec<_>>(),
        adapter.uncertain_calls().collect::<Vec<_>>()
    );
    let mut merged = CallGraph::new();
    merged.merge(source.clone());
    merged.merge(source.clone());
    let parallel = ParallelCallGraph::new(1);
    parallel.merge_concurrent(merged);
    let restored: CallGraph =
        serde_json::from_str(&serde_json::to_string(&parallel.to_call_graph()).unwrap()).unwrap();
    assert_eq!(
        restored.uncertain_calls().collect::<Vec<_>>(),
        source.uncertain_calls().collect::<Vec<_>>()
    );
    assert!(
        restored
            .uncertain_calls()
            .all(|call| call.call_site.column.is_none())
    );
}

#[test]
fn legacy_uncertainty_without_ordinal_still_deserializes() {
    let [source, _] = graphs();
    let mut encoded = serde_json::to_value(source).unwrap();
    let calls = encoded["uncertain_calls"].as_array_mut().unwrap();
    calls.truncate(1);
    calls[0].as_object_mut().unwrap().remove("call_ordinal");
    let restored: CallGraph = serde_json::from_value(encoded).unwrap();
    let call = restored.uncertain_calls().next().unwrap();
    assert_eq!(call.call_ordinal, None);
    assert_eq!(call.call_site.column, None);
}
