use super::*;
use crate::analysis::call_graph::RustCallGraph;
use crate::analysis::role_policy::{RoleEvidence, RoleSignal};
use crate::priority::parallel_call_graph::ParallelCallGraph;

fn id(name: &str) -> FunctionId {
    FunctionId::new("src/service.rs".into(), name.into(), 1)
}

fn possible(caller: &str, candidates: &[&str], line: usize) -> UncertainCall {
    UncertainCall {
        call_ordinal: None,
        caller: id(caller),
        call_site: CallSite {
            file: id(caller).file,
            line,
            column: Some(4),
        },
        lexical_module: "crate::service".into(),
        call_type: CallType::Direct,
        query: "run".into(),
        receiver: None,
        candidates: candidates.iter().map(|name| id(name)).collect(),
        reason: UncertaintyReason::UnknownReceiver,
    }
}

fn graph() -> CallGraph {
    let mut graph = CallGraph::new();
    for name in ["root", "possible", "descendant", "unused", "unreachable"] {
        graph.add_function(id(name), name == "root", false, 1, 3);
    }
    graph
}

#[test]
fn possible_relations_do_not_change_resolved_counts() {
    let mut graph = graph();
    graph.record_uncertain_call(possible("root", &["possible", "possible"], 2));
    graph.record_uncertain_call(possible("root", &[], 3));
    assert!(graph.get_callees(&id("root")).is_empty());
    assert!(graph.get_callers(&id("possible")).is_empty());
    assert_eq!(graph.get_dependency_count(&id("possible")), 0);
    assert_eq!(
        graph.get_possible_callers(&id("possible")),
        vec![id("root")]
    );
    assert_eq!(
        graph.get_possible_callees(&id("root")),
        vec![id("possible")]
    );
    assert!(graph.get_possible_callers(&id("unused")).is_empty());
    assert_eq!(graph.uncertain_calls().count(), 2);
    assert_eq!(graph.uncertain_calls().next().unwrap().candidates.len(), 1);
}

#[test]
fn uncertainty_reachability_protects_descendants_without_creating_roots() {
    let mut graph = graph();
    graph.record_uncertain_call(possible("root", &["possible"], 2));
    graph.record_uncertain_call(possible("unreachable", &["unused"], 3));
    graph.add_call_parts(id("possible"), id("descendant"), CallType::Direct);
    let enhanced = RustCallGraph {
        base_graph: graph,
        ..RustCallGraph::new()
    };
    let definite = enhanced.get_live_functions();
    assert!(definite.contains(&id("root")));
    assert!(!definite.contains(&id("possible")));
    let dead = enhanced.get_potential_dead_code();
    assert!(!dead.contains(&id("possible")));
    assert!(!dead.contains(&id("descendant")));
    assert!(dead.contains(&id("unused")));
    assert!(dead.contains(&id("unreachable")));
}

fn graph_with_evidence() -> CallGraph {
    let mut graph = graph();
    graph.add_role_evidence(
        &id("possible"),
        RoleEvidence {
            signals: vec![
                RoleSignal::PublicExport,
                RoleSignal::Framework {
                    name: "local".into(),
                    kind: "registration".into(),
                },
            ],
        },
    );
    for line in [2, 3] {
        graph.add_call_with_evidence(CallEdgeEvidence {
            call: FunctionCall {
                caller: id("root"),
                callee: id("possible"),
                call_type: CallType::Direct,
            },
            provenance: CallEdgeProvenance::TypeResolution,
            confidence: 90,
            call_site: Some(CallSite {
                file: id("root").file,
                line,
                column: Some(7),
            }),
        });
        graph.record_uncertain_call(possible("root", &["unused", "possible", "possible"], line));
    }
    graph
}

#[test]
fn repeated_merges_preserve_distinct_sites_and_deduplicate_records() {
    let source = graph_with_evidence();
    let mut merged = CallGraph::new();
    merged.merge(source.clone());
    merged.merge(source.clone());
    assert_eq!(merged.get_all_calls().len(), 1);
    assert_eq!(merged.edge_evidence().count(), 2);
    assert_eq!(merged.uncertain_calls().count(), 2);
    assert_eq!(
        merged.get_role_evidence(&id("possible")),
        source.get_role_evidence(&id("possible"))
    );
    assert_eq!(merged.find_function(&id("possible")), Some(id("possible")));
}

#[test]
fn parallel_roundtrip_preserves_roles_sites_and_uncertainty() {
    let source = graph_with_evidence();
    let parallel = ParallelCallGraph::new(1);
    parallel.merge_concurrent(source.clone());
    parallel.merge_concurrent(source.clone());
    let restored = parallel.to_call_graph();
    assert_eq!(restored.get_all_calls(), source.get_all_calls());
    assert_eq!(
        restored.edge_evidence().collect::<Vec<_>>(),
        source.edge_evidence().collect::<Vec<_>>()
    );
    assert_eq!(
        restored.uncertain_calls().collect::<Vec<_>>(),
        source.uncertain_calls().collect::<Vec<_>>()
    );
    assert_eq!(
        restored.get_role_evidence(&id("possible")),
        source.get_role_evidence(&id("possible"))
    );
}

#[test]
fn serialization_rebuilds_possible_indexes_and_supports_legacy_graphs() {
    let source = graph_with_evidence();
    let json = serde_json::to_value(&source).unwrap();
    let mut restored: CallGraph = serde_json::from_value(json.clone()).unwrap();
    restored.merge(source);
    assert_eq!(restored.edge_evidence().count(), 2);
    assert_eq!(restored.uncertain_calls().count(), 2);
    assert_eq!(
        restored.get_possible_callers(&id("unused")),
        vec![id("root")]
    );
    let mut legacy = json;
    legacy.as_object_mut().unwrap().remove("uncertain_calls");
    let legacy: CallGraph = serde_json::from_value(legacy).unwrap();
    assert_eq!(legacy.uncertain_calls().count(), 0);
    assert!(legacy.get_possible_callers(&id("unused")).is_empty());
}

#[test]
fn missing_caller_definitions_do_not_protect_targets() {
    let mut graph = graph();
    graph.record_uncertain_call(possible("missing", &["unused"], 5));
    assert!(graph.get_possible_callers(&id("unused")).is_empty());
}

#[test]
fn trait_dispatch_entry_point_survives_both_merge_paths() {
    let mut source = graph();
    source.mark_as_trait_dispatch(id("unused"));
    let mut sequential = CallGraph::new();
    sequential.merge(source.clone());
    let parallel = ParallelCallGraph::new(1);
    parallel.merge_concurrent(source);
    for graph in [sequential, parallel.to_call_graph()] {
        assert!(graph.is_entry_point(&id("unused")));
        assert!(graph.get_roles(&id("unused")).unwrap().is_entry_point);
    }
}

#[test]
fn legacy_role_flags_survive_loading_and_parallel_conversion() {
    let source = graph();
    let mut json = serde_json::to_value(&source).unwrap();
    for entry in json["nodes"].as_array_mut().unwrap() {
        let node = entry[1].as_object_mut().unwrap();
        node.remove("role_evidence");
        node.remove("roles");
        if node["id"]["name"] == "unused" {
            node.insert("is_test".into(), true.into());
        }
    }
    let restored: CallGraph = serde_json::from_value(json).unwrap();
    let parallel = ParallelCallGraph::new(1);
    parallel.merge_concurrent(restored.clone());
    for graph in [restored, parallel.to_call_graph()] {
        assert!(graph.get_roles(&id("root")).unwrap().is_entry_point);
        assert!(graph.get_roles(&id("unused")).unwrap().is_test);
    }
}

#[test]
fn legacy_duplicate_records_are_normalized_and_indexes_rebuilt() {
    let source = graph_with_evidence();
    let mut json = serde_json::to_value(&source).unwrap();
    for key in ["edges", "edge_evidence"] {
        let records = json[key].as_array_mut().unwrap();
        records.push(records[0].clone());
    }
    json["caller_index"] = serde_json::json!([]);
    json["callee_index"] = serde_json::json!([]);
    json.as_object_mut().unwrap().remove("uncertain_calls");
    let mut restored: CallGraph = serde_json::from_value(json).unwrap();
    restored.merge(restored.clone());
    assert_eq!(restored.get_all_calls().len(), 1);
    assert_eq!(restored.edge_evidence().count(), 2);
    assert_eq!(restored.get_callees(&id("root")), vec![id("possible")]);
    assert_eq!(restored.get_callers(&id("possible")), vec![id("root")]);
    assert_eq!(restored.uncertain_calls().count(), 0);
    let parallel = ParallelCallGraph::new(1);
    parallel.merge_concurrent(restored.clone());
    let parallel = parallel.to_call_graph();
    assert_eq!(restored.get_all_calls(), parallel.get_all_calls());
    assert_eq!(
        restored.edge_evidence().collect::<Vec<_>>(),
        parallel.edge_evidence().collect::<Vec<_>>()
    );
}

#[test]
fn legacy_edges_without_evidence_receive_one_compatibility_record() {
    let mut json = serde_json::to_value(graph_with_evidence()).unwrap();
    let object = json.as_object_mut().unwrap();
    object.remove("edge_evidence");
    object.remove("uncertain_calls");
    let restored: CallGraph = serde_json::from_value(json).unwrap();
    assert_eq!(restored.get_all_calls().len(), 1);
    assert_eq!(restored.edge_evidence().count(), 1);
    assert_eq!(
        restored.edge_evidence().next().unwrap().provenance,
        CallEdgeProvenance::Legacy
    );
}
