//! Legacy display locations must preserve unique data-flow access without merging definitions.
use debtmap::data_flow::{DataFlowGraph, DataTransformation, MutationInfo, PurityInfo};
use debtmap::priority::call_graph::{CallGraph, FunctionId};
use std::path::PathBuf;

fn id(column: Option<usize>) -> FunctionId {
    FunctionId::new(PathBuf::from("src/lib.rs"), "Foo::run".into(), 1).with_column(column)
}

fn purity(is_pure: bool) -> PurityInfo {
    PurityInfo {
        is_pure,
        confidence: 1.0,
        impurity_reasons: vec![],
    }
}

#[test]
fn legacy_display_queries_recover_unique_data_and_reject_same_line_ties() {
    let first = id(Some(10));
    let second = id(Some(40));
    let mut calls = CallGraph::new();
    calls.add_function(first.clone(), false, false, 1, 1);
    let mut graph = DataFlowGraph::from_call_graph(calls.clone());
    graph.set_purity_info(first.clone(), purity(true));
    graph.set_mutation_info(first.clone(), MutationInfo::none());
    graph.add_data_transformation(
        first.clone(),
        first.clone(),
        DataTransformation {
            input_vars: vec![],
            output_vars: vec![],
            transformation_type: "map".into(),
        },
    );
    assert!(
        graph
            .get_data_transformation(&id(None), &id(None))
            .is_some()
    );
    assert!(graph.get_purity_info(&id(None)).unwrap().is_pure);
    assert!(!graph.get_mutation_info(&id(None)).unwrap().has_mutations);

    calls.add_function(second.clone(), false, false, 1, 1);
    let mut graph = DataFlowGraph::from_call_graph(calls);
    graph.set_purity_info(first.clone(), purity(true));
    graph.set_purity_info(second.clone(), purity(false));
    assert!(graph.get_purity_info(&id(None)).is_none());
    assert!(graph.get_purity_info(&first).unwrap().is_pure);
    assert!(!graph.get_purity_info(&second).unwrap().is_pure);
}

#[test]
fn serialized_data_flow_retains_column_and_pair_identities() {
    let first = id(Some(10));
    let second = id(Some(40));
    let mut graph = DataFlowGraph::new();
    graph.set_purity_info(first.clone(), purity(true));
    graph.set_purity_info(second.clone(), purity(false));
    for (from, to, kind) in [(&first, &second, "map"), (&second, &first, "filter")] {
        graph.add_data_transformation(
            from.clone(),
            to.clone(),
            DataTransformation {
                input_vars: vec![],
                output_vars: vec![],
                transformation_type: kind.into(),
            },
        );
    }
    let json = serde_json::to_string(&graph).unwrap();
    let restored: DataFlowGraph = serde_json::from_str(&json).unwrap();
    assert!(restored.get_purity_info(&first).unwrap().is_pure);
    assert!(!restored.get_purity_info(&second).unwrap().is_pure);
    assert_eq!(
        restored
            .get_data_transformation(&first, &second)
            .unwrap()
            .transformation_type,
        "map"
    );
    assert_eq!(
        restored
            .get_data_transformation(&second, &first)
            .unwrap()
            .transformation_type,
        "filter"
    );
}

#[test]
fn legacy_internal_map_keys_remain_readable() {
    let mut json = serde_json::to_value(DataFlowGraph::new()).unwrap();
    json["purity_analysis"] = serde_json::json!({"src/lib.rs:Foo::run:1": purity(true), "src/café.rs:modèle::run:2": purity(false)});
    json["data_transformations"] = serde_json::json!({"src/lib.rs:Foo::run:1|src/lib.rs:other:2": {
        "input_vars": [], "output_vars": [], "transformation_type": "map"
    }});
    let restored: DataFlowGraph = serde_json::from_value(json).unwrap();
    assert!(restored.get_purity_info(&id(None)).unwrap().is_pure);
    let unicode = FunctionId::new("src/café.rs".into(), "modèle::run".into(), 2);
    assert!(!restored.get_purity_info(&unicode).unwrap().is_pure);
    let other = FunctionId::new(PathBuf::from("src/lib.rs"), "other".into(), 2);
    assert!(
        restored
            .get_data_transformation(&id(None), &other)
            .is_some()
    );
}
