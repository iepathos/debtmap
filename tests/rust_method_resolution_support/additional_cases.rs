//! Small cases that exercise contracts independently of the timed source corpus.
use super::support::{callees, function, possible};
use debtmap::analyzers::rust_call_graph::{extract_call_graph, extract_call_graph_multi_file};
use debtmap::priority::call_graph::CallGraph;
use std::path::PathBuf;

fn analyze(source: &str) -> CallGraph {
    extract_call_graph_multi_file(&[(
        syn::parse_file(source).unwrap(),
        PathBuf::from("src/lib.rs"),
    )])
}

#[test]
fn mutable_and_parenthesized_parameters_preserve_receiver_identity() {
    let source = "struct A; impl A { fn hit(&mut self) {} } fn caller(a: (&mut A)) { (a).hit(); }";
    let graph = analyze(source);
    assert_eq!(callees(&graph, "caller"), ["A::hit".into()].into());
    let single = extract_call_graph(
        &syn::parse_file(source).unwrap(),
        &PathBuf::from("src/lib.rs"),
    );
    assert_eq!(callees(&single, "caller"), callees(&graph, "caller"));
}

#[test]
fn branch_join_keeps_agreed_facts() {
    let graph = analyze(
        "struct A; impl A { fn hit(&self) {} } fn caller(flag: bool) { let mut a = A; if flag { a = A; } else { a = A; } a.hit(); }",
    );
    assert_eq!(callees(&graph, "caller"), ["A::hit".into()].into());
}

#[test]
fn contradictory_assignment_does_not_replace_explicit_declaration() {
    let graph = analyze(
        "struct A; struct B; impl A { fn hit(&self) {} } impl B { fn hit(&self) {} } fn caller() { let mut a: A = A; a = B; a.hit(); }",
    );
    assert!(callees(&graph, "caller").is_empty());
    assert_eq!(possible(&graph, "caller", "hit"), ["A::hit".into()].into());
}

#[test]
fn nested_generic_field_and_explicit_constructor_arguments_propagate() {
    let graph = analyze(
        "struct A; impl A { fn hit(&self) {} } struct Wrap<T> { value: T } impl<T> Wrap<T> { fn new() -> Self { unavailable() } } fn caller() { let outer = Wrap::<Wrap<A>>::new(); outer.value.value.hit(); }",
    );
    assert_eq!(
        callees(&graph, "caller"),
        ["A::hit".into(), "Wrap::new".into()].into()
    );
}

#[test]
fn loop_scope_restores_outer_binding() {
    let graph = analyze(
        "struct A; impl A { fn hit(&self) {} } fn caller(a: A, values: Missing) { for a in values { a.hit(); } a.hit(); }",
    );
    assert_eq!(callees(&graph, "caller"), ["A::hit".into()].into());
    assert_eq!(
        graph
            .uncertain_calls()
            .filter(|call| call.caller == function(&graph, "caller"))
            .count(),
        1
    );
}

#[test]
fn separate_module_import_alias_selects_exact_owner() {
    let files = [
        (
            "src/lib.rs",
            "mod left; mod right; use left::Same as Chosen; fn caller(a: Chosen) { a.hit(); }",
        ),
        (
            "src/left.rs",
            "pub struct Same; impl Same { pub fn hit(&self) {} }",
        ),
        (
            "src/right.rs",
            "pub struct Same; impl Same { pub fn hit(&self) {} }",
        ),
    ]
    .map(|(path, source)| (syn::parse_file(source).unwrap(), PathBuf::from(path)));
    let graph = extract_call_graph_multi_file(&files);
    let targets = graph.get_callees(&function(&graph, "caller"));
    assert_eq!(targets.len(), 1);
    assert_eq!(targets[0].file, PathBuf::from("src/left.rs"));
}

#[test]
fn trait_candidate_sets_follow_bounds_and_exclude_declarations() {
    let graph = super::support::graph("traits.rs");
    for (caller, expected) in [
        ("competing", vec![4, 5]),
        ("dynamic", vec![4]),
        ("generic_trait", vec![4]),
        ("unsupported_blanket", vec![11]),
        ("recursive_alias", vec![4, 5]),
    ] {
        let calls: Vec<_> = graph
            .uncertain_calls()
            .filter(|call| call.caller.name == caller)
            .collect();
        assert_eq!(calls.len(), 1, "One source call for {caller}");
        let mut actual: Vec<_> = calls[0]
            .candidates
            .iter()
            .map(|target| target.line)
            .collect();
        actual.sort();
        assert_eq!(
            actual, expected,
            "Admissible implementation bodies for {caller}"
        );
    }
}
