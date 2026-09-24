//! Source positions distinguish nested and repeated method call sites.
use debtmap::analyzers::rust_call_graph::extract_call_graph;
use std::{collections::HashSet, path::Path};

#[test]
fn nested_identical_methods_keep_distinct_uncertain_sites() {
    let ast = syn::parse_file("fn caller(a: Missing) { a.hit().hit(); }").unwrap();
    let graph = extract_call_graph(&ast, Path::new("sites.rs"));
    let sites: HashSet<_> = graph
        .uncertain_calls()
        .map(|c| c.call_site.clone())
        .collect();
    assert_eq!(sites.len(), 2);
    assert_eq!(graph.uncertain_calls().count(), 2);
}

#[test]
fn repeated_resolved_methods_keep_distinct_evidence_sites() {
    let ast = syn::parse_file(
        "struct A; impl A { fn hit(&self) -> Self { A } } fn caller(a: A) { a.hit().hit(); }",
    )
    .unwrap();
    let graph = extract_call_graph(&ast, Path::new("sites.rs"));
    assert_eq!(graph.get_all_calls().len(), 1);
    let sites: HashSet<_> = graph.edge_evidence().map(|e| e.call_site.clone()).collect();
    assert_eq!(sites.len(), 2);
}
