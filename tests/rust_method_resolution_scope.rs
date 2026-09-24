//! Source-flow regressions: unavailable facts must never promote stale method edges.
use debtmap::analyzers::rust_call_graph::extract_call_graph_multi_file;
use debtmap::priority::call_graph::CallGraph;
use std::collections::BTreeSet;
use std::path::PathBuf;

const DECLARATIONS: &str = "
struct A; struct B;
impl A { fn hit(&self) {} fn change(&mut self) {} }
impl B { fn hit(&self) {} }
";

fn analyze(body: &str) -> CallGraph {
    let source = format!("{DECLARATIONS}\n{body}");
    extract_call_graph_multi_file(&[(
        syn::parse_file(&source).unwrap(),
        PathBuf::from("src/lib.rs"),
    )])
}

fn resolved(graph: &CallGraph) -> BTreeSet<String> {
    let caller = graph
        .get_all_functions()
        .find(|id| id.name == "caller")
        .unwrap();
    graph
        .get_callees(caller)
        .into_iter()
        .map(|id| id.name)
        .collect()
}

fn assert_uncertain(graph: &CallGraph, method: &str) {
    assert!(
        resolved(graph).is_empty(),
        "unexpected resolved edges: {:?}",
        resolved(graph)
    );
    assert!(
        graph
            .uncertain_calls()
            .any(|call| call.caller.name == "caller" && call.query == method),
        "missing diagnostic for caller's {method} call"
    );
}

#[test]
fn repeated_loop_invalidates_receivers_before_recording_body_calls() {
    let graph = analyze("fn caller() { let mut a = A; loop { a.hit(); a = external(); } }");
    assert_uncertain(&graph, "hit");
}

#[test]
fn repeated_for_loop_invalidates_receivers_before_recording_body_calls() {
    let graph = analyze(
        "fn caller(values: Missing) { let mut a = A; for _ in values { a.hit(); a = external(); } }",
    );
    assert_uncertain(&graph, "hit");
}

#[test]
fn repeated_while_condition_is_not_assumed_to_be_its_first_iteration() {
    let graph = analyze(
        "fn caller(flag: bool) { let mut a = A; while { a.hit(); a = external(); flag } {} }",
    );
    assert_uncertain(&graph, "hit");
}

#[test]
fn loop_local_shadow_writes_do_not_invalidate_outer_binding() {
    let graph =
        analyze("fn caller() { let a = A; loop { let mut a = B; a = B; break; } a.hit(); }");
    assert_eq!(resolved(&graph), ["A::hit".into()].into());
    assert!(
        !graph
            .uncertain_calls()
            .any(|call| call.caller.name == "caller" && call.query == "hit")
    );
}

#[test]
fn else_branch_observes_effects_of_the_condition() {
    let graph = analyze(
        "fn caller(flag: bool) { let mut a = A; if { a = external(); flag } {} else { a.hit(); } }",
    );
    assert_uncertain(&graph, "hit");
}

#[test]
fn condition_effects_are_retained_after_both_branches() {
    let graph = analyze(
        "fn caller(flag: bool) { let mut a = external(); if { a = A; flag } {} else {} a.hit(); }",
    );
    assert_eq!(resolved(&graph), ["A::hit".into()].into());
}

#[test]
fn conditional_initializer_does_not_propagate_pre_condition_facts() {
    let graph = analyze(
        "fn caller(flag: bool) { let mut a = A; let alias = if { a = external(); flag } { a } else { a }; alias.hit(); }",
    );
    assert_uncertain(&graph, "hit");
}

#[test]
fn conflicting_initializer_retains_declared_owner_as_uncertain() {
    let graph = analyze("fn caller() { let a: A = B; a.hit(); }");
    assert_uncertain(&graph, "hit");
    let candidates: BTreeSet<_> = graph
        .uncertain_calls()
        .filter(|call| call.caller.name == "caller" && call.query == "hit")
        .flat_map(|call| call.candidates.iter().map(|id| id.name.clone()))
        .collect();
    assert_eq!(candidates, ["A::hit".into()].into());
}

#[test]
fn shared_reference_cannot_justify_mutable_method_receiver() {
    let graph = analyze("fn caller(a: &A) { a.change(); }");
    assert_uncertain(&graph, "change");
}

#[test]
fn outer_mutable_reference_does_not_erase_inner_shared_reference() {
    let graph = analyze("fn caller(a: &mut &A) { a.change(); }");
    assert_uncertain(&graph, "change");
}

#[test]
fn nested_mutable_references_support_mutable_receiver_adjustments() {
    let graph = analyze("fn caller(a: &mut &mut A) { a.change(); }");
    assert_eq!(resolved(&graph), ["A::change".into()].into());
}

#[test]
fn unsupported_custom_deref_does_not_promote_a_named_target() {
    let graph = analyze(
        "struct Wrap; impl std::ops::Deref for Wrap { type Target = A; fn deref(&self) -> &A { external() } } fn caller(w: Wrap) { (*w).hit(); }",
    );
    assert_uncertain(&graph, "hit");
}

#[test]
fn failed_match_guard_effects_are_not_discarded_for_later_arms() {
    let graph = analyze(
        "fn caller(flag: bool) { let mut a = A; match flag { true if { a = external(); false } => {}, _ => { a.hit(); } } }",
    );
    assert_uncertain(&graph, "hit");
}

#[test]
fn short_circuit_expression_does_not_make_conditional_assignment_definite() {
    let graph = analyze(
        "fn caller(flag: bool) { let mut a = external(); flag && { a = A; true }; a.hit(); }",
    );
    assert_uncertain(&graph, "hit");
}

#[test]
fn tuple_rest_pattern_uses_trailing_components() {
    let graph = analyze("fn caller() { let (_, .., last) = (A, B, B, A); last.hit(); }");
    assert_eq!(resolved(&graph), ["A::hit".into()].into());
}

#[test]
fn ref_pattern_preserves_shared_reference_constraint() {
    let graph = analyze("fn caller() { let ref a = A; a.change(); }");
    assert_uncertain(&graph, "change");
}

#[test]
fn conditional_let_binding_shadows_outer_fact_even_after_a_join() {
    let graph = analyze(
        "fn caller(flag: bool) { let a = A; if flag && let Some(a) = external() { a.hit(); } }",
    );
    assert_uncertain(&graph, "hit");
}

#[test]
fn branch_conflict_preserves_the_explicit_owner_constraint() {
    let graph = analyze("fn caller(flag: bool) { let mut a: A = A; if flag { a = B; } a.hit(); }");
    assert_uncertain(&graph, "hit");
    let candidates: BTreeSet<_> = graph
        .uncertain_calls()
        .filter(|call| call.caller.name == "caller" && call.query == "hit")
        .flat_map(|call| call.candidates.iter().map(|id| id.name.clone()))
        .collect();
    assert_eq!(candidates, ["A::hit".into()].into());
}

#[test]
fn block_local_function_shadows_module_function_before_its_declaration() {
    let graph = analyze("fn hit() {} fn caller() { hit(); fn hit() {} }");
    assert_uncertain(&graph, "hit");
}

#[test]
fn block_local_import_alias_does_not_select_module_function() {
    let graph =
        analyze("fn hit() {} mod other { pub fn hit() {} } fn caller() { use other::hit; hit(); }");
    assert_uncertain(&graph, "hit");
}

#[test]
fn block_local_type_does_not_select_same_named_module_owner() {
    let graph = analyze("fn caller() { struct A; let value: A = A; value.hit(); }");
    assert_uncertain(&graph, "hit");
}

#[test]
fn local_glob_import_preserves_known_bindings_but_not_unproven_function_names() {
    let graph = analyze(
        "fn hit() {} mod other { pub fn hit() {} } fn caller(a: A) { use other::*; hit(); a.hit(); }",
    );
    assert_eq!(resolved(&graph), ["A::hit".into()].into());
    assert!(
        graph
            .uncertain_calls()
            .any(|call| call.caller.name == "caller" && call.query == "hit")
    );
}
