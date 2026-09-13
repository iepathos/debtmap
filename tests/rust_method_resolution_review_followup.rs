//! Review regressions retain source identities through every graph entry point.
#[path = "rust_method_resolution_support/source_builders.rs"]
mod source_builders;

use debtmap::analysis::call_graph::RustCallGraphBuilder;
use debtmap::analyzers::rust_call_graph::extract_call_graph_multi_file;
use debtmap::builders::parallel_call_graph::build_call_graph_from_extracted;
use debtmap::extraction::UnifiedFileExtractor;
use debtmap::priority::call_graph::{CallGraph, FunctionId};
use std::collections::{BTreeSet, HashMap};
use std::path::PathBuf;

fn graphs(source: &str) -> Vec<CallGraph> {
    let path = PathBuf::from("src/lib.rs");
    let ast = syn::parse_file(source).unwrap();
    let direct = extract_call_graph_multi_file(&[(ast.clone(), path.clone())]);
    let mut enhancement = RustCallGraphBuilder::from_base_graph(direct.clone());
    enhancement.analyze_trait_dispatch(&path, &ast).unwrap();
    enhancement.analyze_framework_patterns(&path, &ast).unwrap();
    enhancement.finalize_trait_analysis().unwrap();
    enhancement.finalize_trait_analysis().unwrap();
    let extracted = HashMap::from([(
        path.clone(),
        UnifiedFileExtractor::extract(&path, source).unwrap(),
    )]);
    [
        direct,
        enhancement.build().base_graph,
        build_call_graph_from_extracted(CallGraph::new(), &extracted).0,
    ]
    .into_iter()
    .chain(source_builders::graphs(&[("src/lib.rs", source)]))
    .flat_map(|graph| {
        let mut merged = graph.clone();
        merged.merge(graph.clone());
        merged.merge(graph.clone());
        [graph, merged]
    })
    .collect()
}

fn definition(graph: &CallGraph, name: &str, line: usize) -> FunctionId {
    let ids: Vec<_> = graph
        .get_all_functions()
        .filter(|id| id.name == name && id.line == line && id.file == PathBuf::from("src/lib.rs"))
        .cloned()
        .collect();
    assert_eq!(ids.len(), 1, "definition {name} at line {line}");
    assert!(ids[0].column.is_some());
    ids[0].clone()
}

fn assert_targets(graph: &CallGraph, caller: (&str, usize), targets: &[(&str, usize)]) {
    let caller = definition(graph, caller.0, caller.1);
    let expected: BTreeSet<_> = targets
        .iter()
        .map(|(name, line)| definition(graph, name, *line))
        .collect();
    let actual = graph.get_callees_exact(&caller).into_iter().collect();
    assert_eq!(expected, actual);
    assert!(!graph.uncertain_calls().any(|call| call.caller == caller));
}

#[test]
fn unit_constructor_keeps_its_value_namespace_identity() {
    let source = "mod other { pub struct Item; impl Item { pub fn hit(&self) {} } }\nuse other::*;\nstruct Item {}\nimpl Item { fn hit(&self) {} }\nfn caller() { let value = Item; value.hit(); }\n";
    for graph in graphs(source) {
        assert_targets(&graph, ("caller", 5), &[("other::Item::hit", 1)]);
    }
}

#[test]
fn tuple_constructor_ignores_type_only_namesakes() {
    let source = "mod other { pub struct Item(pub u32); impl Item { pub fn hit(&self) {} } }\nuse other::*;\nstruct Item {}\nimpl Item { fn hit(&self) {} }\nfn argument() -> u32 { 1 }\nfn caller() { let value = Item(argument()); value.hit(); }\n";
    for graph in graphs(source) {
        assert_targets(
            &graph,
            ("caller", 6),
            &[("other::Item::hit", 1), ("argument", 5)],
        );
        assert!(!graph.get_all_functions().any(|id| id.name == "Item"));
    }
}

#[test]
fn unit_constructor_keeps_explicit_import_conflicts_uncertain() {
    let source = "mod other { pub struct Item; impl Item { pub fn hit(&self) {} } }\nuse other::Item;\nuse unavailable::Item;\nstruct Unrelated; impl Unrelated { fn hit(&self) {} }\nfn caller() { let value = Item; value.hit(); }\n";
    for graph in graphs(source) {
        let caller = definition(&graph, "caller", 5);
        assert!(graph.get_callees_exact(&caller).is_empty());
        let calls: Vec<_> = graph
            .uncertain_calls()
            .filter(|call| call.caller == caller)
            .collect();
        assert_eq!(calls.len(), 1);
        assert_eq!(
            calls[0].candidates,
            [definition(&graph, "other::Item::hit", 1)]
        );
    }
}

#[test]
fn qualified_receivers_respect_block_type_shadows() {
    let source = "struct A; struct B;\ntrait T { fn hit(); }\nimpl T for A { fn hit() {} }\nimpl T for B { fn hit() {} }\nfn caller() { type A = B; <A as T>::hit(); }\nfn control() { type A = B; <crate::A as T>::hit(); <B as T>::hit(); }\n";
    for graph in graphs(source) {
        assert_shadowed_call(&graph, ("caller", 5));
        assert_targets(&graph, ("control", 6), &[("A::hit", 3), ("B::hit", 4)]);
    }
}

#[test]
fn dynamic_bounds_respect_block_trait_shadows() {
    let source = "trait T { fn hit(&self); }\nstruct A;\nimpl T for A { fn hit(&self) {} }\nstruct B;\nfn caller() { trait T { fn hit(&self); } impl T for B { fn hit(&self) {} } let x: &dyn T = &B; x.hit(); }\nfn control() { trait T {} let x: &dyn crate::T = &A; x.hit(); }\n";
    for graph in graphs(source) {
        assert_shadowed_call(&graph, ("caller", 5));
        let control = definition(&graph, "control", 6);
        let calls: Vec<_> = graph
            .uncertain_calls()
            .filter(|call| call.caller == control)
            .collect();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].candidates, [definition(&graph, "A::hit", 3)]);
    }
}

fn assert_shadowed_call(graph: &CallGraph, caller: (&str, usize)) {
    let caller = definition(graph, caller.0, caller.1);
    assert!(graph.get_callees_exact(&caller).is_empty());
    let calls: Vec<_> = graph
        .uncertain_calls()
        .filter(|call| call.caller == caller)
        .collect();
    assert_eq!(calls.len(), 1);
    assert!(
        calls[0].candidates.is_empty(),
        "forbidden global target: {:?}",
        calls[0]
    );
}
