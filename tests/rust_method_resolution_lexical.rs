//! Source-identity regressions for lexical lookup and successful condition facts.
#[path = "rust_method_resolution_support/source_builders.rs"]
mod source_builders;
use debtmap::analyzers::rust_call_graph::extract_call_graph_multi_file;
use debtmap::builders::parallel_call_graph::build_call_graph_from_extracted;
use debtmap::extraction::UnifiedFileExtractor;
use debtmap::priority::call_graph::{CallGraph, FunctionId};
use std::collections::{BTreeSet, HashMap};
use std::path::PathBuf;

fn graphs(source: &str) -> Vec<CallGraph> {
    let path = PathBuf::from("src/lib.rs");
    let graph = extract_call_graph_multi_file(&[(syn::parse_file(source).unwrap(), path.clone())]);
    let extracted = HashMap::from([(
        path.clone(),
        UnifiedFileExtractor::extract(&path, source).unwrap(),
    )]);
    [
        graph,
        build_call_graph_from_extracted(CallGraph::new(), &extracted).0,
    ]
    .into_iter()
    .chain(source_builders::graphs(&[("src/lib.rs", source)]))
    .collect()
}

fn definition(graph: &CallGraph, name: &str, line: usize) -> FunctionId {
    let found: Vec<_> = graph
        .get_all_functions()
        .filter(|id| id.name == name && id.line == line)
        .collect();
    assert_eq!(found.len(), 1, "unique definition {name} at line {line}");
    found[0].clone()
}

fn assert_targets(graph: &CallGraph, caller: (&str, usize), expected: &[(&str, usize)]) {
    let caller = definition(graph, caller.0, caller.1);
    let expected: BTreeSet<_> = expected
        .iter()
        .map(|(name, line)| definition(graph, name, *line))
        .collect();
    let actual: BTreeSet<_> = graph.get_callees(&caller).into_iter().collect();
    assert_eq!(actual, expected);
    assert!(
        !graph.uncertain_calls().any(|call| call.caller == caller),
        "unexpected possible edges for {caller:?}"
    );
}

#[test]
fn local_type_and_function_shadow_glob_imports() {
    let source = "mod other { pub struct A; impl A { pub fn hit(&self) {} } pub fn run() {} }\nuse other::*;\nstruct A;\nimpl A { fn hit(&self) {} }\nfn run() {}\nfn caller(a: A) { a.hit(); run(); }\n";
    for graph in graphs(source) {
        assert_targets(&graph, ("caller", 6), &[("A::hit", 4), ("run", 5)]);
    }
}

#[test]
fn explicit_import_shadows_glob_import() {
    let source = "mod one { pub struct A; impl A { pub fn hit(&self) {} } }\nmod two { pub struct A; impl A { pub fn hit(&self) {} } }\nuse one::*;\nuse two::A;\nfn caller(a: A) { a.hit(); }\n";
    for graph in graphs(source) {
        assert_targets(&graph, ("caller", 5), &[("two::A::hit", 2)]);
    }
}

#[test]
fn conflicting_globs_preserve_both_owners_and_exclude_unrelated_methods() {
    let source = "mod one { pub struct A; impl A { pub fn hit(&self) {} } }\nmod two { pub struct A; impl A { pub fn hit(&self) {} } }\nstruct B; impl B { fn hit(&self) {} }\nuse one::*; use two::*;\nfn caller(a: A) { a.hit(); }\n";
    for graph in graphs(source) {
        let caller = definition(&graph, "caller", 5);
        assert!(graph.get_callees(&caller).is_empty());
        let calls: Vec<_> = graph
            .uncertain_calls()
            .filter(|call| call.caller == caller)
            .collect();
        assert_eq!(calls.len(), 1);
        let expected: BTreeSet<_> = [
            definition(&graph, "one::A::hit", 1),
            definition(&graph, "two::A::hit", 2),
        ]
        .into();
        assert_eq!(
            calls[0].candidates.iter().cloned().collect::<BTreeSet<_>>(),
            expected
        );
    }
}

#[test]
fn tuple_constructors_propagate_fields_and_explicit_type_arguments() {
    let source = "struct A; impl A { fn hit(&self) {} }\nstruct B; impl B { fn hit(&self) {} }\nstruct Wrap<T>(T);\nfn make() -> A { A }\nfn caller() { let w = Wrap::<A>(make()); w.0.hit(); }\n";
    for graph in graphs(source) {
        assert_targets(&graph, ("caller", 5), &[("make", 4), ("A::hit", 1)]);
        assert!(!graph.get_all_functions().any(|id| id.name == "Wrap"));
    }
}

#[test]
fn nongeneric_tuple_constructor_propagates_nominal_owner() {
    let source = "struct A; impl A { fn hit(&self) {} }\nstruct Wrap(A); impl Wrap { fn hit(&self) {} }\nfn caller() { let w = Wrap(A); w.hit(); w.0.hit(); }\n";
    for graph in graphs(source) {
        assert_targets(&graph, ("caller", 3), &[("Wrap::hit", 2), ("A::hit", 1)]);
    }
}

#[test]
fn block_value_items_do_not_shadow_types() {
    let source = "struct A; impl A { fn hit(&self) {} }\nfn caller() { fn A() {} let value: A = external(); value.hit(); }\n";
    for graph in graphs(source) {
        let caller = definition(&graph, "caller", 2);
        assert_eq!(
            graph.get_callees(&caller),
            vec![definition(&graph, "A::hit", 1)]
        );
    }
}

#[test]
fn block_type_items_do_not_shadow_values() {
    let source = "struct A; impl A { fn hit(&self) {} }\nconst VALUE: A = A;\nfn caller() { type VALUE = (); VALUE.hit(); }\n";
    for graph in graphs(source) {
        assert_targets(&graph, ("caller", 3), &[("A::hit", 1)]);
    }
}

#[test]
fn successful_nested_let_chain_retains_binding_and_visits_calls_once() {
    let source = "struct A; impl A { fn hit(&self) -> bool { true } }\nstruct B; impl B { fn hit(&self) -> bool { true } }\nfn make() -> A { A }\nfn caller(flag: bool) { if flag && let x = make() && x.hit() { x.hit(); } }\n";
    for graph in graphs(source) {
        assert_targets(&graph, ("caller", 4), &[("make", 3), ("A::hit", 1)]);
        assert_eq!(
            graph
                .edge_evidence()
                .filter(|edge| edge.call.caller.name == "caller")
                .count(),
            3
        );
    }
}

#[test]
fn while_let_chain_keeps_success_bindings() {
    let source = "struct A; impl A { fn hit(&self) -> bool { true } }\nfn make() -> A { A }\nfn caller(flag: bool) { while flag && let x = make() && x.hit() { x.hit(); break; } }\n";
    for graph in graphs(source) {
        assert_targets(&graph, ("caller", 3), &[("make", 2), ("A::hit", 1)]);
    }
}

#[test]
fn false_short_circuit_path_carries_only_executed_effects() {
    let source = "struct A; impl A { fn hit(&self) {} }\nstruct B; impl B { fn hit(&self) {} }\nfn caller(flag: bool) { let mut x = external(); if flag || { x = A; false } {} else { x.hit(); } }\n";
    for graph in graphs(source) {
        let caller = definition(&graph, "caller", 3);
        assert_eq!(
            graph.get_callees(&caller),
            vec![definition(&graph, "A::hit", 1)]
        );
    }
}

#[test]
fn conflicting_explicit_bindings_keep_admissible_owner_constraints() {
    let source = "mod other { pub struct A; impl A { pub fn hit(&self) {} } }\nuse other::A;\nstruct A; impl A { fn hit(&self) {} }\nstruct B; impl B { fn hit(&self) {} }\nfn caller(a: A) { a.hit(); }\n";
    for graph in graphs(source) {
        let caller = definition(&graph, "caller", 5);
        assert!(graph.get_callees(&caller).is_empty());
        let calls: Vec<_> = graph
            .uncertain_calls()
            .filter(|call| call.caller == caller)
            .collect();
        assert_eq!(calls.len(), 1);
        let expected: BTreeSet<_> = [
            definition(&graph, "other::A::hit", 1),
            definition(&graph, "A::hit", 3),
        ]
        .into();
        assert_eq!(
            calls[0].candidates.iter().cloned().collect::<BTreeSet<_>>(),
            expected
        );
    }
}

#[test]
fn local_module_first_segment_shadows_glob_module() {
    let source = "mod other { pub mod m { pub fn run() {} } }\nuse other::*;\nmod m { pub fn run() {} }\nfn caller() { m::run(); }\n";
    for graph in graphs(source) {
        assert_targets(&graph, ("caller", 4), &[("m::run", 3)]);
    }
}

#[test]
fn block_import_namespace_does_not_hide_an_unrelated_value() {
    let source = "struct A; impl A { fn hit(&self) {} }\nconst VALUE: A = A;\nmod other { pub type VALUE = (); }\nfn caller() { use other::VALUE; VALUE.hit(); }\n";
    for graph in graphs(source) {
        assert_targets(&graph, ("caller", 4), &[("A::hit", 1)]);
    }
}

#[test]
fn shadowing_module_does_not_fall_back_to_glob_for_a_missing_member() {
    let source = "mod other { pub mod m { pub fn run() {} } }\nuse other::*;\nmod m {}\nfn caller() { m::run(); }\n";
    for graph in graphs(source) {
        let caller = definition(&graph, "caller", 4);
        assert!(graph.get_callees(&caller).is_empty());
        let calls: Vec<_> = graph
            .uncertain_calls()
            .filter(|call| call.caller == caller)
            .collect();
        assert_eq!(calls.len(), 1);
        assert!(calls[0].candidates.is_empty());
    }
}

#[test]
fn constructor_type_arguments_are_not_inferred_from_arguments() {
    let source = "struct A; impl A { fn hit(&self) {} }\nstruct B; impl B { fn hit(&self) {} }\nstruct Wrap<T>(T);\nfn caller() { let w = Wrap(A); w.0.hit(); }\n";
    for graph in graphs(source) {
        let caller = definition(&graph, "caller", 4);
        assert!(graph.get_callees(&caller).is_empty());
        let calls: Vec<_> = graph
            .uncertain_calls()
            .filter(|call| call.caller == caller)
            .collect();
        assert_eq!(calls.len(), 1, "constructor is not an unavailable function");
        assert_eq!(calls[0].query, "hit");
    }
}

#[test]
fn success_bindings_do_not_escape_conditional_scope() {
    let source = "struct A; impl A { fn hit(&self) {} }\nstruct B; impl B { fn hit(&self) {} }\nfn caller(flag: bool) { let x = B; if flag && let x = A { x.hit(); } x.hit(); }\n";
    for graph in graphs(source) {
        assert_targets(&graph, ("caller", 3), &[("A::hit", 1), ("B::hit", 2)]);
    }
}

#[test]
fn block_type_import_does_not_hide_loop_writes_to_value_binding() {
    let source = "struct A; impl A { fn hit(&self) {} }\nmod other { pub type x = (); }\nfn caller() { let mut x = A; loop { use other::x; x.hit(); x = external(); } }\n";
    for graph in graphs(source) {
        let caller = definition(&graph, "caller", 3);
        assert!(graph.get_callees(&caller).is_empty());
        assert!(
            graph
                .uncertain_calls()
                .any(|call| call.caller == caller && call.query == "hit")
        );
    }
}

#[test]
fn local_type_alias_does_not_shadow_tuple_constructor_value() {
    let source = "struct A; impl A { fn hit(&self) {} }\nstruct Wrap(A);\nfn caller() { type Wrap = (); let w = Wrap(A); w.0.hit(); }\n";
    for graph in graphs(source) {
        assert_targets(&graph, ("caller", 3), &[("A::hit", 1)]);
    }
}

#[test]
fn constructor_generic_argument_respects_block_type_shadowing() {
    let source = "struct A; impl A { fn hit(&self) {} }\nstruct Wrap<T>(T);\nfn caller() { type A = (); let w = Wrap::<A>(()); w.0.hit(); }\n";
    for graph in graphs(source) {
        let caller = definition(&graph, "caller", 3);
        assert!(graph.get_callees(&caller).is_empty());
        let calls: Vec<_> = graph
            .uncertain_calls()
            .filter(|call| call.caller == caller)
            .collect();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].query, "hit");
    }
}

#[test]
fn unavailable_explicit_type_conflict_keeps_known_owner_uncertain() {
    let source = "struct A;\nimpl A { fn hit(&self) {} }\nuse missing::A;\nstruct B; impl B { fn hit(&self) {} }\nfn caller(a: A) { a.hit(); }\n";
    for graph in graphs(source) {
        let caller = definition(&graph, "caller", 5);
        assert!(graph.get_callees(&caller).is_empty());
        let calls: Vec<_> = graph
            .uncertain_calls()
            .filter(|call| call.caller == caller)
            .collect();
        assert_eq!(calls.len(), 1);
        assert_eq!(
            calls[0].reason,
            debtmap::priority::call_graph::UncertaintyReason::AmbiguousDeclaration
        );
        assert_eq!(calls[0].candidates, vec![definition(&graph, "A::hit", 2)]);
    }
}

#[test]
fn unavailable_explicit_function_conflict_keeps_known_target_uncertain() {
    let source = "fn run() {}\nuse missing::run;\nfn caller() { run(); }\n";
    for graph in graphs(source) {
        let caller = definition(&graph, "caller", 3);
        assert!(graph.get_callees(&caller).is_empty());
        let calls: Vec<_> = graph
            .uncertain_calls()
            .filter(|call| call.caller == caller)
            .collect();
        assert_eq!(calls.len(), 1);
        assert_eq!(
            calls[0].reason,
            debtmap::priority::call_graph::UncertaintyReason::AmbiguousDeclaration
        );
        assert_eq!(calls[0].candidates, vec![definition(&graph, "run", 1)]);
    }
}

#[test]
fn unavailable_reexport_conflict_preserves_qualified_owner_constraints() {
    let source = "mod m { pub struct A; impl A { pub fn hit(&self) {} } use missing::A; }\nfn caller(a: m::A) { a.hit(); }\n";
    for graph in graphs(source) {
        let caller = definition(&graph, "caller", 2);
        assert!(graph.get_callees(&caller).is_empty());
        let calls: Vec<_> = graph
            .uncertain_calls()
            .filter(|call| call.caller == caller)
            .collect();
        assert_eq!(calls.len(), 1);
        assert_eq!(
            calls[0].candidates,
            vec![definition(&graph, "m::A::hit", 1)]
        );
    }
}

#[test]
fn unavailable_trait_conflict_prevents_qualified_call_promotion() {
    let source = "struct A;\ntrait T { fn hit(&self); }\nuse missing::T;\nimpl T for A { fn hit(&self) {} }\nfn caller(a: A) { <A as T>::hit(&a); }\n";
    for graph in graphs(source) {
        let caller = definition(&graph, "caller", 5);
        assert!(graph.get_callees(&caller).is_empty());
        let calls: Vec<_> = graph
            .uncertain_calls()
            .filter(|call| call.caller == caller)
            .collect();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].candidates, vec![definition(&graph, "A::hit", 4)]);
        assert_eq!(
            calls[0].reason,
            debtmap::priority::call_graph::UncertaintyReason::AmbiguousDeclaration
        );
    }
}
