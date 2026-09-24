//! Regressions for verified Rust namespace, qualification, and module-path findings.
use debtmap::analysis::effect_evidence::EffectClassification;
use debtmap::analyzers::rust_call_graph::extract_call_graph_multi_file;
use debtmap::builders::parallel_call_graph::build_call_graph_from_extracted;
use debtmap::extraction::UnifiedFileExtractor;
use debtmap::priority::call_graph::CallGraph;
use std::collections::{BTreeSet, HashMap};
use std::path::PathBuf;

fn graphs(files: &[(&str, &str)]) -> [CallGraph; 2] {
    let parsed: Vec<_> = files
        .iter()
        .map(|(path, source)| (syn::parse_file(source).unwrap(), PathBuf::from(path)))
        .collect();
    let ast = extract_call_graph_multi_file(&parsed);
    drop(parsed);
    let extracted: HashMap<_, _> = files
        .iter()
        .map(|(path, source)| {
            let path = PathBuf::from(path);
            let data = UnifiedFileExtractor::extract(&path, source).unwrap();
            (path, data)
        })
        .collect();
    [
        ast,
        build_call_graph_from_extracted(CallGraph::new(), &extracted).0,
    ]
}

fn targets(graph: &CallGraph, caller: &str) -> BTreeSet<String> {
    let caller = graph
        .get_all_functions()
        .find(|id| id.name == caller)
        .unwrap();
    graph
        .get_callees(caller)
        .into_iter()
        .map(|id| id.name)
        .collect()
}

#[test]
fn const_and_static_values_are_not_confused_with_enum_or_alias_types() {
    let source = "struct A;\nimpl A { fn bar(&self) {} }\nstruct B;\nimpl B { fn bar(&self) {} }\nenum E { Variant }\nimpl E { fn bar(&self) {} }\nconst E: A = A;\ntype Alias = A;\nconst Alias: B = B;\nstatic VALUE: &B = &B;\nfn enum_value() { E.bar(); }\nfn alias_value() { Alias.bar(); }\nfn static_value() { VALUE.bar(); }\nfn unit_value() { A.bar(); }\n";
    for graph in graphs(&[("src/lib.rs", source)]) {
        assert_eq!(targets(&graph, "enum_value"), ["A::bar".into()].into());
        assert_eq!(targets(&graph, "alias_value"), ["B::bar".into()].into());
        assert_eq!(targets(&graph, "static_value"), ["B::bar".into()].into());
        assert_eq!(targets(&graph, "unit_value"), ["A::bar".into()].into());
    }
}

#[test]
fn external_absolute_paths_and_imports_do_not_resolve_local_std_symbols() {
    let source = "mod std { pub mod string { pub struct String; impl String { pub fn len(&self) -> usize { 42 } } } pub mod mem { pub fn drop() {} } }\nuse ::std::string::String as ExternalString;\nfn absolute(s: &::std::string::String) { s.len(); }\nfn imported(s: &ExternalString) { s.len(); }\nfn function() { ::std::mem::drop(1); }\n";
    for graph in graphs(&[("src/lib.rs", source)]) {
        for caller in ["absolute", "imported"] {
            assert!(targets(&graph, caller).is_empty(), "{caller}");
            assert!(
                !graph
                    .uncertain_calls()
                    .any(|call| call.caller.name == caller),
                "{caller}"
            );
            let id = graph
                .get_all_functions()
                .find(|id| id.name == caller)
                .unwrap();
            assert_eq!(
                graph.effect_assessment(id).unwrap().classification(),
                EffectClassification::StrictlyPure,
                "{caller}"
            );
        }
        assert!(targets(&graph, "function").is_empty());
        let calls: Vec<_> = graph
            .uncertain_calls()
            .filter(|call| call.caller.name == "function")
            .collect();
        assert_eq!(calls.len(), 1);
        assert!(calls[0].candidates.is_empty());
    }
}

#[test]
fn qualified_self_uses_the_enclosing_impl_for_return_chains() {
    let source = "struct A;\ntrait Work { fn make() -> Self; }\nimpl Work for A { fn make() -> Self { A } }\nimpl A { fn bar(&self) {} fn caller() { <Self as Work>::make().bar(); } }\n";
    for graph in graphs(&[("src/lib.rs", source)]) {
        assert_eq!(
            targets(&graph, "A::caller"),
            ["A::make".into(), "A::bar".into()].into()
        );
        assert!(
            !graph
                .uncertain_calls()
                .any(|call| call.caller.name == "A::caller")
        );
    }
}

#[test]
fn parent_components_in_module_attributes_preserve_input_file_identity() {
    let source = "#[path = \"../shared.rs\"] mod shared;\nuse shared::Foo;\nfn caller(value: &Foo) { value.bar(); }\n";
    let shared = "pub struct Foo;\nimpl Foo { pub fn bar(&self) {} }\n";
    for graph in graphs(&[
        ("/project/src/lib.rs", source),
        ("/project/shared.rs", shared),
    ]) {
        let caller = graph
            .get_all_functions()
            .find(|id| id.name == "caller")
            .unwrap();
        let callees = graph.get_callees(caller);
        assert_eq!(callees.len(), 1);
        assert_eq!(callees[0].file, PathBuf::from("/project/shared.rs"));
        assert_eq!(callees[0].name, "Foo::bar");
    }
}

#[test]
fn const_generic_parameters_shadow_module_constants_in_value_namespace() {
    let source = "struct A; impl A { fn bar(&self) {} }\nconst N: A = A;\ntrait Bar { fn bar(&self); }\nimpl Bar for usize { fn bar(&self) {} }\nfn caller<const N: usize>() { N.bar(); }\nstruct Wrapper<const N: usize>;\nimpl<const N: usize> Wrapper<N> { fn method() { N.bar(); } }\n";
    for graph in graphs(&[("src/lib.rs", source)]) {
        assert!(targets(&graph, "caller").is_empty());
        assert!(targets(&graph, "Wrapper::method").is_empty());
    }
}

#[test]
fn type_generic_parameters_do_not_shadow_constants_in_value_namespace() {
    let source =
        "struct A; impl A { fn bar(&self) {} }\nconst T: A = A;\nfn caller<T>() { T.bar(); }\n";
    for graph in graphs(&[("src/lib.rs", source)]) {
        assert_eq!(targets(&graph, "caller"), ["A::bar".into()].into());
    }
}
