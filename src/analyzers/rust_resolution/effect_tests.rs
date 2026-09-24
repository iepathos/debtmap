use super::extract;
use crate::analysis::effect_evidence::{
    EffectClassification, ObservedEffectKind, UnresolvedReason,
};
use crate::priority::call_graph::{CallGraph, CallType, FunctionId};
use std::path::PathBuf;

fn graph(source: &str) -> CallGraph {
    extract(&[(
        PathBuf::from("src/lib.rs"),
        syn::parse_file(source).expect("fixture parses"),
    )])
}

fn definition(graph: &CallGraph, name: &str) -> FunctionId {
    graph
        .get_all_functions()
        .find(|id| id.name == name)
        .cloned()
        .unwrap_or_else(|| panic!("missing definition {name}"))
}

#[test]
fn function_references_do_not_become_invocations() {
    let graph = graph("fn target() {} fn caller() { let _callback = target; }");
    let caller = definition(&graph, "caller");
    let target = definition(&graph, "target");
    let assessment = graph.effect_assessment(&caller).expect("source evidence");
    assert_eq!(graph.get_callees_exact(&caller), vec![target]);
    assert!(
        graph
            .get_all_calls()
            .iter()
            .all(|call| call.call_type == CallType::Callback)
    );
    assert_eq!(assessment.dependencies().count(), 0);
    assert_eq!(
        assessment.classification(),
        EffectClassification::StrictlyPure
    );
}

#[test]
fn direct_calls_record_exact_project_dependencies() {
    let graph =
        graph("fn target() {} fn direct() { target(); } fn parenthesized() { (target)(); }");
    let target = definition(&graph, "target");
    for caller_name in ["direct", "parenthesized"] {
        let caller = definition(&graph, caller_name);
        let assessment = graph.effect_assessment(&caller).expect("source evidence");
        assert_eq!(graph.get_callees_exact(&caller), vec![target.clone()]);
        let dependencies: Vec<_> = assessment.dependencies().collect();
        assert_eq!(dependencies.len(), 1);
        assert_eq!(dependencies[0].target, target);
        assert_eq!(dependencies[0].provenance.owner, caller);
    }
}

#[test]
fn closure_construction_does_not_execute_its_body() {
    let graph = graph("fn target() {} fn caller() { let _callback = || target(); }");
    let caller = definition(&graph, "caller");
    let target = definition(&graph, "target");
    let assessment = graph.effect_assessment(&caller).expect("source evidence");
    assert_eq!(graph.get_callees_exact(&caller), vec![target]);
    assert_eq!(assessment.dependencies().count(), 0);
    assert_eq!(
        assessment.classification(),
        EffectClassification::StrictlyPure
    );
}

#[test]
fn directly_invoked_closure_executes_its_body() {
    let graph = graph("fn target() {} fn caller() { (|| target())(); }");
    let caller = definition(&graph, "caller");
    let target = definition(&graph, "target");
    let assessment = graph.effect_assessment(&caller).expect("source evidence");

    assert_eq!(assessment.dependencies().next().unwrap().target, target);
    assert_eq!(
        assessment.classification(),
        EffectClassification::StrictlyPure
    );
}

#[test]
fn unresolved_and_indirect_calls_prevent_completeness() {
    for (source, expected) in [
        (
            "fn caller() { unavailable(); }",
            UnresolvedReason::UnresolvedCall,
        ),
        (
            "fn caller<F: Fn()>(callback: F) { callback(); }",
            UnresolvedReason::CallbackInvocation,
        ),
    ] {
        let graph = graph(source);
        let caller = definition(&graph, "caller");
        let assessment = graph.effect_assessment(&caller).expect("source evidence");
        assert_eq!(assessment.classification(), EffectClassification::Unknown);
        assert!(assessment.unresolved().any(|item| item.reason == expected));
    }
}

#[test]
fn reviewed_models_contribute_effects_without_unresolved_calls() {
    let source = r#"
        use std::fs::read_to_string as load;
        fn file_read() { let _ = load("input.txt"); }
        fn memory_write(mut values: Vec<i32>) { values.push(1); }
        fn console() { println!("hello"); }
    "#;
    let graph = graph(source);
    let file_read = graph
        .effect_assessment(&definition(&graph, "file_read"))
        .unwrap();
    assert_eq!(file_read.classification(), EffectClassification::Impure);
    assert!(
        file_read
            .observed()
            .any(|effect| effect.kind == ObservedEffectKind::Io)
    );
    assert_eq!(file_read.unresolved().count(), 0);

    let memory_write = graph
        .effect_assessment(&definition(&graph, "memory_write"))
        .unwrap();
    assert_eq!(
        memory_write.classification(),
        EffectClassification::LocallyPure
    );

    let console = graph
        .effect_assessment(&definition(&graph, "console"))
        .unwrap();
    assert_eq!(console.classification(), EffectClassification::Impure);
    assert!(
        console
            .unresolved()
            .all(|behavior| behavior.reason == UnresolvedReason::UnsupportedDispatch)
    );
}

#[test]
fn modeled_console_macros_still_analyze_argument_execution() {
    let graph = graph("fn value() -> i32 { 1 } fn caller() { println!(\"{}\", value()); }");
    let caller = definition(&graph, "caller");
    let value = definition(&graph, "value");
    let assessment = graph.effect_assessment(&caller).unwrap();

    assert_eq!(assessment.classification(), EffectClassification::Impure);
    assert_eq!(assessment.dependencies().next().unwrap().target, value);
}

#[test]
fn modeled_callback_consumers_execute_supported_callback_forms() {
    let source = r#"
        fn target(value: i32) -> i32 { value }
        fn closure_form(value: Option<i32>) { let _ = value.map(|item| target(item)); }
        fn reference_form(value: Option<i32>) { let _ = value.map(target); }
    "#;
    let graph = graph(source);
    let target = definition(&graph, "target");

    for name in ["closure_form", "reference_form"] {
        let assessment = graph.effect_assessment(&definition(&graph, name)).unwrap();
        assert_eq!(
            assessment.classification(),
            EffectClassification::StrictlyPure
        );
        assert_eq!(assessment.dependencies().next().unwrap().target, target);
    }
}

#[test]
fn project_callback_wrappers_execute_equivalent_supported_forms() {
    let graph = graph(
        r#"
        fn invoke<F: Fn(i32) -> i32>(value: i32, callback: F) -> i32 { callback(value) }
        fn target(value: i32) -> i32 { println!("{}", value); value }
        fn reference_form() { invoke(1, target); }
        fn closure_form() { invoke(1, |value| target(value)); }
        fn unused<F: Fn(i32) -> i32>(value: i32, callback: F) -> i32 { value }
        fn unused_reference() { unused(1, target); }
        struct Receiver;
        impl Receiver {
            fn dispatch(&self, callback: fn(i32) -> i32) -> i32 { callback(1) }
        }
        fn method_reference(receiver: &Receiver) { receiver.dispatch(target); }
        fn method_closure(receiver: &Receiver) { receiver.dispatch(|value| target(value)); }
    "#,
    );
    let propagated = crate::analysis::purity_propagation::propagate_graph_assessments(&graph);
    for name in [
        "reference_form",
        "closure_form",
        "method_reference",
        "method_closure",
    ] {
        let assessment = propagated.get(&definition(&graph, name)).unwrap();
        assert_eq!(
            assessment.classification(),
            EffectClassification::Impure,
            "{name}: {assessment:?}"
        );
        assert!(
            assessment
                .observed()
                .any(|effect| effect.kind == ObservedEffectKind::Io)
        );
        assert!(
            assessment
                .unresolved()
                .any(|item| item.reason == UnresolvedReason::CallbackInvocation)
        );
    }
    let unused = propagated
        .get(&definition(&graph, "unused_reference"))
        .unwrap();
    assert_eq!(unused.observed().count(), 0);
}

#[test]
fn bound_function_values_keep_identity_until_shadowed_or_reassigned() {
    let graph = graph(
        r#"
        fn target() { println!("effect"); }
        fn bound() { let callback = target; callback(); }
        fn shadowed<F: Fn()>(other: F) { let callback = target; { let callback = other; callback(); } }
        fn reassigned<F: Fn()>(other: F) { let mut callback = target; callback = other; callback(); }
    "#,
    );
    let propagated = crate::analysis::purity_propagation::propagate_graph_assessments(&graph);
    assert_eq!(
        propagated
            .get(&definition(&graph, "bound"))
            .unwrap()
            .classification(),
        EffectClassification::Impure
    );
    for name in ["shadowed", "reassigned"] {
        let assessment = propagated.get(&definition(&graph, name)).unwrap();
        assert_eq!(assessment.classification(), EffectClassification::Unknown);
        assert!(
            !assessment
                .observed()
                .any(|effect| effect.kind == ObservedEffectKind::Io)
        );
    }
}

#[test]
fn bound_closures_execute_only_when_invoked_and_keep_lexical_scope() {
    let graph = graph(
        r#"
        fn target() { println!("effect"); }
        fn unused() { let callback = || target(); }
        fn invoked() { let callback = || target(); callback(); }
        fn console() { let callback = || println!("effect"); callback(); }
        fn shadowed() { let callback = || target(); { let callback = || (); callback(); } }
        fn reference_shadowed() { let callback = || target(); let target = || (); callback(); }
        fn reassigned() { let mut callback = || target(); callback = || (); callback(); }
    "#,
    );
    let propagated = crate::analysis::purity_propagation::propagate_graph_assessments(&graph);
    for name in ["invoked", "console", "reference_shadowed"] {
        assert_eq!(
            propagated[&definition(&graph, name)].classification(),
            EffectClassification::Impure,
            "{name}"
        );
    }
    for name in ["unused", "shadowed", "reassigned"] {
        assert!(
            !propagated[&definition(&graph, name)]
                .observed()
                .any(|effect| effect.kind == ObservedEffectKind::Io),
            "{name}"
        );
    }
}

#[test]
fn async_method_construction_does_not_execute_body() {
    let graph = graph(
        r#"
        struct Worker;
        impl Worker { async fn run(&self) { println!("effect"); } }
        fn caller(worker: &Worker) { let _future = worker.run(); }
    "#,
    );
    let propagated = crate::analysis::purity_propagation::propagate_graph_assessments(&graph);
    let assessment = &propagated[&definition(&graph, "caller")];
    assert!(
        !assessment
            .observed()
            .any(|effect| effect.kind == ObservedEffectKind::Io)
    );
    assert_eq!(assessment.dependencies().count(), 0);
}

#[test]
fn wrapper_summary_respects_shadowing_and_lazy_bodies() {
    let graph = graph(
        r#"
        fn target() { println!("effect"); }
        fn ignore<F: Fn()>(callback: F) { let callback = || (); callback(); }
        async fn later<F: Fn()>(callback: F) { callback(); }
        fn shadowed() { ignore(target); }
        fn lazy() { let _future = later(target); }
    "#,
    );
    let propagated = crate::analysis::purity_propagation::propagate_graph_assessments(&graph);
    for name in ["shadowed", "lazy"] {
        let assessment = propagated.get(&definition(&graph, name)).unwrap();
        assert!(
            !assessment
                .observed()
                .any(|effect| effect.kind == ObservedEffectKind::Io)
        );
    }
}

#[test]
fn project_methods_take_precedence_over_familiar_model_names() {
    let source = r#"
        struct Writer;
        impl Writer { fn write(&mut self) {} }
        fn caller(mut writer: Writer) { writer.write(); }
    "#;
    let graph = graph(source);
    let caller = definition(&graph, "caller");
    let target = definition(&graph, "Writer::write");
    let assessment = graph.effect_assessment(&caller).unwrap();
    assert_eq!(graph.get_callees_exact(&caller), vec![target.clone()]);
    assert_eq!(assessment.classification(), EffectClassification::Unknown);
    assert_eq!(assessment.dependencies().next().unwrap().target, target);
}

#[test]
fn generic_names_cannot_substitute_modeled_receiver_identities() {
    let graph =
        graph(r#"fn caller<String>(mut value: String) { value.push_str("not a std string"); }"#);
    let assessment = graph
        .effect_assessment(&definition(&graph, "caller"))
        .unwrap();
    assert_eq!(assessment.classification(), EffectClassification::Unknown);
    assert_eq!(assessment.observed().count(), 0);
}

#[test]
fn assignments_distinguish_local_and_external_writes() {
    let source = r#"
        fn local() { let mut value = 0; value = 1; }
        struct State { value: i32 }
        impl State { fn update(&mut self) { self.value = 1; } }
    "#;
    let graph = graph(source);
    assert_eq!(
        graph
            .effect_assessment(&definition(&graph, "local"))
            .unwrap()
            .classification(),
        EffectClassification::LocallyPure
    );
    assert_eq!(
        graph
            .effect_assessment(&definition(&graph, "State::update"))
            .unwrap()
            .classification(),
        EffectClassification::Impure
    );
}

#[test]
fn unexpanded_macros_are_explicitly_unsupported() {
    let graph = graph("fn caller() { opaque_syntax!(); }");
    let assessment = graph
        .effect_assessment(&definition(&graph, "caller"))
        .unwrap();
    assert_eq!(assessment.classification(), EffectClassification::Unknown);
    assert!(
        assessment
            .unresolved()
            .any(|behavior| behavior.reason == UnresolvedReason::UnsupportedSyntax)
    );
}
