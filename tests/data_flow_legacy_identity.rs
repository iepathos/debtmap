//! Legacy data-flow facts follow uniquely upgraded Rust declaration identities.
use debtmap::data_flow::{DataFlowGraph, DataTransformation, PurityInfo};
use debtmap::extraction::{UnifiedFileExtractor, adapters};
use debtmap::priority::call_graph::{CallGraph, FunctionId};
use std::collections::HashMap;

fn id(column: Option<usize>) -> FunctionId {
    FunctionId::new("src/lib.rs".into(), "work".into(), 1).with_column(column)
}

fn graph(columns: &[usize]) -> DataFlowGraph {
    let mut calls = CallGraph::new();
    for column in columns {
        calls.add_function(id(Some(*column)), false, false, 1, 1);
    }
    DataFlowGraph::from_call_graph(calls)
}

fn purity() -> PurityInfo {
    PurityInfo {
        is_pure: true,
        confidence: 1.0,
        impurity_reasons: vec![],
    }
}

fn transformation() -> DataTransformation {
    DataTransformation {
        input_vars: vec![],
        output_vars: vec![],
        transformation_type: "map".into(),
    }
}

#[test]
fn unique_legacy_writes_support_exact_and_legacy_queries() {
    let mut flow = graph(&[3]);
    flow.set_purity_info(id(None), purity());
    flow.add_data_transformation(id(None), id(Some(3)), transformation());
    for query in [id(None), id(Some(3))] {
        assert!(flow.get_purity_info(&query).unwrap().is_pure);
        for target in [id(None), id(Some(3))] {
            assert!(flow.get_data_transformation(&query, &target).is_some());
        }
    }
    assert!(flow.get_purity_info(&id(Some(9))).is_none());
}

#[test]
fn cached_legacy_extraction_populates_the_upgraded_graph_identity() {
    let path = std::path::PathBuf::from("src/lib.rs");
    let mut file =
        UnifiedFileExtractor::extract(&path, "fn work(x: i32) -> i32 { x + 1 }").unwrap();
    for function in &mut file.functions {
        function.column = None;
    }
    let files = HashMap::from([(path, file)]);
    let calls = adapters::call_graph::build_call_graph(&files);
    let canonical = calls.get_all_functions().next().unwrap().clone();
    assert!(canonical.column.is_some());
    let mut flow = DataFlowGraph::from_call_graph(calls);
    adapters::data_flow::populate_data_flow(&mut flow, &files);
    assert!(flow.get_purity_info(&canonical).is_some());
    assert!(
        flow.get_variable_dependencies(&canonical)
            .unwrap()
            .contains("x")
    );
}

fn legacy_json(columns: &[usize]) -> DataFlowGraph {
    let mut json = serde_json::to_value(graph(columns)).unwrap();
    json["purity_analysis"] = serde_json::json!({"src/lib.rs:work:1": purity()});
    json["data_transformations"] =
        serde_json::json!({"src/lib.rs:work:1|src/lib.rs:work:1": transformation()});
    serde_json::from_value(json).unwrap()
}

#[test]
fn legacy_json_facts_are_accessible_only_for_the_unique_definition() {
    let flow = legacy_json(&[3]);
    for query in [id(None), id(Some(3))] {
        assert!(flow.get_purity_info(&query).unwrap().is_pure);
        assert!(flow.get_data_transformation(&query, &query).is_some());
    }
    assert!(flow.get_purity_info(&id(Some(9))).is_none());

    let flow = legacy_json(&[3, 9]);
    for query in [id(None), id(Some(3)), id(Some(9))] {
        assert!(flow.get_purity_info(&query).is_none());
        assert!(flow.get_data_transformation(&query, &query).is_none());
    }
}

#[test]
fn ambiguous_legacy_writes_do_not_attach_to_either_definition() {
    let mut flow = graph(&[3, 9]);
    flow.set_purity_info(id(None), purity());
    flow.add_data_transformation(id(None), id(Some(3)), transformation());
    for query in [id(None), id(Some(3)), id(Some(9))] {
        assert!(flow.get_purity_info(&query).is_none());
        assert!(flow.get_data_transformation(&query, &id(Some(3))).is_none());
    }
    flow.set_purity_info(id(Some(3)), purity());
    assert!(flow.get_purity_info(&id(Some(3))).is_some());
    assert!(flow.get_purity_info(&id(None)).is_none());
}

#[test]
fn appending_io_to_legacy_json_retains_existing_operations() {
    let operation = debtmap::data_flow::IoOperation {
        operation_type: "console".into(),
        variables: vec![],
        line: 1,
    };
    let mut json = serde_json::to_value(graph(&[3])).unwrap();
    json["io_operations"] = serde_json::json!({"src/lib.rs:work:1": [operation.clone()]});
    let mut flow: DataFlowGraph = serde_json::from_value(json).unwrap();
    flow.add_io_operation(id(None), operation);
    assert_eq!(flow.get_io_operations(&id(Some(3))).unwrap().len(), 2);
    assert_eq!(flow.get_io_operations(&id(None)).unwrap().len(), 2);
}

#[test]
fn legacy_module_metadata_must_agree_when_present() {
    let mut canonical = id(Some(3));
    canonical.module_path = "owner".into();
    for module in ["", "owner", "other"] {
        let mut calls = CallGraph::new();
        calls.add_function(canonical.clone(), false, false, 1, 1);
        let mut flow = DataFlowGraph::from_call_graph(calls);
        let mut legacy = id(None);
        legacy.module_path = module.into();
        flow.set_purity_info(legacy.clone(), purity());
        assert_eq!(
            flow.get_purity_info(&canonical).is_some(),
            module != "other"
        );
        if module != "other" {
            assert!(flow.get_purity_info(&legacy).is_some());
        }
    }
}

#[test]
fn a_legacy_node_does_not_override_a_same_line_identity_tie() {
    let mut calls = CallGraph::new();
    for column in [None, Some(3)] {
        calls.add_function(id(column), false, false, 1, 1);
    }
    let mut flow = DataFlowGraph::from_call_graph(calls);
    flow.set_purity_info(id(None), purity());
    assert!(flow.get_purity_info(&id(None)).is_none());
    assert!(flow.get_purity_info(&id(Some(3))).is_none());
}

#[test]
fn old_json_without_module_metadata_is_read_by_a_unique_exact_id() {
    let mut canonical = id(Some(3));
    canonical.module_path = "owner".into();
    let mut calls = CallGraph::new();
    calls.add_function(canonical.clone(), false, false, 1, 1);
    let mut json = serde_json::to_value(DataFlowGraph::from_call_graph(calls)).unwrap();
    json["purity_analysis"] = serde_json::json!({"src/lib.rs:work:1": purity()});
    let flow: DataFlowGraph = serde_json::from_value(json).unwrap();
    assert!(flow.get_purity_info(&canonical).is_some());
}
