//! Prove that the matrix oracle rejects plausible but incorrect graph outcomes.
#[path = "rust_resolution_matrix_support/mod.rs"]
mod support;
use debtmap::priority::call_graph::{
    CallEdgeEvidence, CallEdgeProvenance, CallGraph, CallSite, CallType, FunctionCall, FunctionId,
    UncertainCall, UncertaintyReason,
};
use support::Expectation as E;

const SOURCE: &str = "trait A { fn hit(); } trait B { fn hit(); } struct X; impl A for X { /*@left*/ fn hit() {} } impl B for X { /*@right*/ fn hit() {} } /*@caller*/ fn caller() { /*#call*/ <X as A>::hit(); }";

fn id(marker: &str, name: &str) -> FunctionId {
    let start = SOURCE.find(marker).unwrap();
    let column = start + SOURCE[start..].find("fn ").unwrap() + 3;
    FunctionId::new("src/lib.rs".into(), name.into(), 1).with_column(Some(column))
}

fn nodes() -> CallGraph {
    let mut graph = CallGraph::new();
    for (marker, name) in [
        ("/*@left*/", "X::hit"),
        ("/*@right*/", "X::hit"),
        ("/*@caller*/", "caller"),
    ] {
        graph.add_function(id(marker, name), false, false, 0, 0);
    }
    graph
}

fn site() -> CallSite {
    CallSite {
        file: "src/lib.rs".into(),
        line: 1,
        column: SOURCE.find("<X as A>"),
    }
}

fn resolved(target: &str) -> CallGraph {
    resolved_identities(id("/*@caller*/", "caller"), id(target, "X::hit"))
}

fn resolved_identities(caller: FunctionId, callee: FunctionId) -> CallGraph {
    let mut graph = nodes();
    graph.add_call_with_evidence(CallEdgeEvidence {
        call: FunctionCall {
            caller,
            callee,
            call_type: CallType::Direct,
        },
        provenance: CallEdgeProvenance::TypeResolution,
        confidence: 100,
        call_site: Some(site()),
    });
    graph
}

fn possible_identities(caller: FunctionId, target: FunctionId) -> CallGraph {
    let mut graph = nodes();
    graph.record_uncertain_call(UncertainCall {
        caller,
        call_site: site(),
        call_ordinal: None,
        lexical_module: String::new(),
        call_type: CallType::Direct,
        query: "hit".into(),
        receiver: None,
        candidates: vec![target],
        reason: UncertaintyReason::AmbiguousDeclaration,
    });
    graph
}

fn assert_dangling_rejected(graph: &CallGraph, expected: E) {
    let errors = support::inspect(SOURCE, graph, &[expected]);
    assert!(
        errors
            .iter()
            .any(|error| error.contains("not a graph node")),
        "missing exact membership rejection: {errors:?}"
    );
}

#[test]
fn dangling_resolved_target_at_a_valid_location_is_rejected() {
    let graph = resolved_identities(id("/*@caller*/", "caller"), id("/*@left*/", "missing"));
    assert_dangling_rejected(&graph, E::resolved("call", "<X", &["left"]));
}

#[test]
fn dangling_possible_target_at_a_valid_location_is_rejected() {
    let graph = possible_identities(id("/*@caller*/", "caller"), id("/*@left*/", "missing"));
    assert_dangling_rejected(&graph, E::uncertain("call", "<X", &["left"]));
}

#[test]
fn dangling_callers_at_a_valid_location_are_rejected() {
    let caller = id("/*@caller*/", "missing");
    let target = id("/*@left*/", "X::hit");
    assert_dangling_rejected(
        &resolved_identities(caller.clone(), target.clone()),
        E::resolved("call", "<X", &["left"]),
    );
    assert_dangling_rejected(
        &possible_identities(caller, target),
        E::uncertain("call", "<X", &["left"]),
    );
}

#[test]
fn wrong_same_name_definition_and_missing_edge_are_rejected() {
    let expected = [E::resolved("call", "<X", &["left"])];
    assert!(support::inspect(SOURCE, &resolved("/*@left*/"), &expected).is_empty());
    assert!(!support::inspect(SOURCE, &resolved("/*@right*/"), &expected).is_empty());
    assert!(!support::inspect(SOURCE, &nodes(), &expected).is_empty());
}

#[test]
fn extra_possible_target_duplicate_site_and_ghost_node_are_rejected() {
    let expected = [E::uncertain("call", "<X", &["left"])];
    let mut graph = nodes();
    let call = UncertainCall {
        caller: id("/*@caller*/", "caller"),
        call_site: site(),
        call_ordinal: None,
        lexical_module: String::new(),
        call_type: CallType::Direct,
        query: "hit".into(),
        receiver: None,
        candidates: vec![id("/*@left*/", "X::hit")],
        reason: UncertaintyReason::AmbiguousDeclaration,
    };
    graph.record_uncertain_call(call.clone());
    assert!(support::inspect(SOURCE, &graph, &expected).is_empty());
    let mut extra = call.clone();
    extra.call_ordinal = Some(2);
    graph.record_uncertain_call(extra);
    assert!(!support::inspect(SOURCE, &graph, &expected).is_empty());
    let mut graph = nodes();
    let mut extra = call;
    extra.candidates.push(id("/*@right*/", "X::hit"));
    graph.record_uncertain_call(extra);
    assert!(!support::inspect(SOURCE, &graph, &expected).is_empty());
    let mut graph = resolved("/*@left*/");
    graph.add_function(
        FunctionId::new("src/lib.rs".into(), "ghost".into(), 99),
        false,
        false,
        0,
        0,
    );
    assert!(!support::inspect(SOURCE, &graph, &[E::resolved("call", "<X", &["left"])]).is_empty());
}
