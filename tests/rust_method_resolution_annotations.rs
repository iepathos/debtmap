//! Explicit annotations retain component identity despite unavailable initializer facts.
use debtmap::analyzers::rust_call_graph::extract_call_graph_multi_file;
use debtmap::priority::call_graph::CallGraph;
use std::collections::BTreeSet;

fn analyze(body: &str) -> CallGraph {
    let source = format!(
        "struct A; struct B; impl A {{ fn hit(&self) {{}} }} impl B {{ fn hit(&self) {{}} }} struct Wrap<T> {{ value: T }} {body}"
    );
    extract_call_graph_multi_file(&[(syn::parse_file(&source).unwrap(), "src/lib.rs".into())])
}

fn resolved(graph: &CallGraph) -> BTreeSet<String> {
    let caller = graph
        .get_all_functions()
        .find(|id| id.name == "caller")
        .unwrap();
    graph
        .get_callees(caller)
        .iter()
        .map(|id| id.name.clone())
        .collect()
}

fn possible(graph: &CallGraph) -> BTreeSet<String> {
    graph
        .uncertain_calls()
        .filter(|call| call.caller.name == "caller" && call.query == "hit")
        .flat_map(|call| call.candidates.iter().map(|id| id.name.clone()))
        .collect()
}

#[test]
fn tuple_annotation_recovers_unknown_initializer_component() {
    let graph = analyze("fn caller() { let (a,): (A,) = (external(),); a.hit(); }");
    assert_eq!(resolved(&graph), ["A::hit".into()].into());
    assert!(possible(&graph).is_empty());
}

#[test]
fn tuple_conflict_preserves_only_the_declared_component_owner() {
    let graph = analyze("fn caller() { let (a,): (A,) = (B,); a.hit(); }");
    assert!(resolved(&graph).is_empty());
    assert_eq!(possible(&graph), ["A::hit".into()].into());
}

#[test]
fn tuple_conflict_does_not_taint_an_agreeing_sibling_component() {
    let graph = analyze("fn caller() { let (a, _): (A, A) = (A, B); a.hit(); }");
    assert_eq!(resolved(&graph), ["A::hit".into()].into());
}

#[test]
fn nested_tuple_annotation_propagates_partial_unknown_facts() {
    let graph = analyze("fn caller() { let ((a,), _): ((A,), B) = ((external(),), B); a.hit(); }");
    assert_eq!(resolved(&graph), ["A::hit".into()].into());
}

#[test]
fn tuple_shape_conflict_retains_component_constraints() {
    let graph = analyze("fn caller() { let (a,): (A,) = B; a.hit(); }");
    assert!(resolved(&graph).is_empty());
    assert_eq!(possible(&graph), ["A::hit".into()].into());
}

#[test]
fn reference_annotation_recovers_unknown_borrowed_value() {
    let graph = analyze("fn caller() { let a: &A = &external(); a.hit(); }");
    assert_eq!(resolved(&graph), ["A::hit".into()].into());
}

#[test]
fn assignment_with_unknown_tuple_component_keeps_declared_type() {
    let graph = analyze("fn caller() { let mut a: (A,) = (A,); a = (external(),); a.0.hit(); }");
    assert_eq!(resolved(&graph), ["A::hit".into()].into());
}

#[test]
fn explicit_generic_annotation_supplies_missing_struct_type_arguments() {
    let graph =
        analyze("fn caller() { let a: Wrap<A> = Wrap { value: external() }; a.value.hit(); }");
    assert_eq!(resolved(&graph), ["A::hit".into()].into());
}
