use super::{TraitRegistry, visitor::TraitVisitor};
use crate::analyzers::rust_call_graph::extract_call_graph;
use std::path::PathBuf;

fn fixture() -> syn::File {
    syn::parse_file(
        "trait Action { fn hit(&self); } struct A; impl Action for A { fn hit(&self) {} } fn caller(a: Missing) { a.hit(); a.hit(); }",
    )
    .expect("valid syntax fixture")
}

#[test]
fn declaration_visitor_does_not_recollect_unknown_receivers() {
    let result = TraitVisitor::new(PathBuf::from("src/lib.rs")).visit_and_extract(&fixture());
    assert_eq!(result.trait_definitions.len(), 1);
    assert_eq!(result.trait_implementations.len(), 1);
    assert!(result.trait_method_calls.is_empty());
}

#[test]
fn shared_uncertainty_survives_without_legacy_promotion() {
    let path = PathBuf::from("src/lib.rs");
    let mut graph = extract_call_graph(&fixture(), &path);
    let expected: Vec<_> = graph.uncertain_calls().cloned().collect();
    assert_eq!(
        expected.len(),
        2,
        "Distinct columns identify distinct calls"
    );
    let mut registry = TraitRegistry::new();
    registry.analyze_file(&path, &fixture()).unwrap();
    registry.ingest_shared_uncertainty(&graph);
    registry.ingest_shared_uncertainty(&graph);
    assert_eq!(
        registry
            .shared_uncertain_calls()
            .cloned()
            .collect::<Vec<_>>(),
        expected
    );
    assert_eq!(registry.get_statistics().total_unresolved_calls, 2);
    assert_eq!(
        registry.resolve_trait_call(&registry.get_unresolved_trait_calls()[0]),
        expected[0].candidates
    );
    assert_eq!(registry.resolve_trait_method_calls(&mut graph), 0);
    assert!(graph.get_all_calls().is_empty());
}
