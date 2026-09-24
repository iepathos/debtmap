//! Recursion remains in the graph without counting as external coupling.
use debtmap::analyzers::call_graph_integration::populate_call_graph_data;
use debtmap::core::FunctionMetrics;
use debtmap::output::unified::FunctionDebtItemOutput;
use debtmap::priority::call_graph::{CallGraph, CallType, FunctionCall, FunctionId};
use debtmap::priority::scoring::construction::create_unified_debt_item_enhanced;

fn metrics() -> FunctionMetrics {
    FunctionMetrics {
        name: "dispatch".into(),
        file: "src/lib.rs".into(),
        line: 1,
        column: Some(3),
        cyclomatic: 15,
        cognitive: 20,
        nesting: 2,
        length: 40,
        is_test: false,
        visibility: None,
        is_trait_method: false,
        in_test_module: false,
        entropy_score: None,
        is_pure: None,
        purity_confidence: None,
        purity_reason: None,
        call_dependencies: None,
        detected_patterns: None,
        upstream_callers: None,
        downstream_callees: None,
        mapping_pattern_result: None,
        adjusted_complexity: None,
        composition_metrics: None,
        language_specific: None,
        purity_level: None,
        error_swallowing_count: None,
        error_swallowing_patterns: None,
        entropy_analysis: None,
    }
}

fn id(name: &str, line: usize) -> FunctionId {
    FunctionId::new("src/lib.rs".into(), name.into(), line).with_column(Some(3))
}

fn call(graph: &mut CallGraph, caller: &FunctionId, callee: &FunctionId) {
    for node in [caller, callee] {
        graph.add_function(node.clone(), false, false, 1, 1);
    }
    graph.add_call(FunctionCall {
        caller: caller.clone(),
        callee: callee.clone(),
        call_type: CallType::Direct,
    });
}

#[test]
fn recursion_preserves_edges_without_inflating_scores_or_display_counts() {
    let focus = id("dispatch", 1);
    let other = id("other", 2);
    let mut graph = CallGraph::new();
    call(&mut graph, &other, &focus);
    let baseline = create_unified_debt_item_enhanced(&metrics(), &graph, None, None).unwrap();
    call(&mut graph, &focus, &focus);
    let populated = populate_call_graph_data(vec![metrics()], &graph);
    assert_eq!(populated[0].upstream_callers.as_ref().unwrap().len(), 1);
    assert!(populated[0].downstream_callees.is_none());
    let item = create_unified_debt_item_enhanced(&populated[0], &graph, None, None).unwrap();
    assert_eq!(
        item.unified_score.dependency_factor,
        baseline.unified_score.dependency_factor
    );
    assert_eq!(
        (item.upstream_dependencies, item.downstream_dependencies),
        (1, 0)
    );
    assert!(graph.get_callees_exact(&focus).contains(&focus));
}

#[test]
fn immediate_neighborhood_deduplicates_ids_across_directions_not_display_names() {
    let focus = id("dispatch", 1);
    let first = id("same_name", 2);
    let second = id("same_name", 3);
    let mut graph = CallGraph::new();
    for neighbor in [&first, &second] {
        call(&mut graph, &focus, neighbor);
        call(&mut graph, neighbor, &focus);
    }
    let item = create_unified_debt_item_enhanced(&metrics(), &graph, None, None).unwrap();
    let output = FunctionDebtItemOutput::from_function_item(&item, false);
    assert_eq!(
        (
            output.dependencies.upstream_count,
            output.dependencies.downstream_count
        ),
        (2, 2)
    );
    assert_eq!(output.dependencies.blast_radius, 2);
    assert_eq!(output.dependencies.production_blast_radius, 2);
    assert_eq!(output.dependencies.upstream_callers.len(), 2);
    assert_eq!(output.dependencies.downstream_callees.len(), 2);
    assert!(
        serde_json::to_value(&item)
            .unwrap()
            .get("immediate_neighbor_count")
            .is_none()
    );
}

#[test]
fn high_degree_alone_does_not_establish_a_critical_execution_path() {
    let focus = id("dispatch", 1);
    let mut graph = CallGraph::new();
    for line in 2..14 {
        call(&mut graph, &focus, &id("helper", line));
    }
    let item = create_unified_debt_item_enhanced(&metrics(), &graph, None, None).unwrap();
    let output = FunctionDebtItemOutput::from_function_item(&item, false);
    assert!(!output.dependencies.critical_path);
}

#[test]
fn same_named_test_does_not_reclassify_a_production_caller() {
    let focus = id("dispatch", 1);
    let production = id("same_name", 2);
    let test = id("same_name", 3);
    let mut graph = CallGraph::new();
    call(&mut graph, &production, &focus);
    let baseline = create_unified_debt_item_enhanced(&metrics(), &graph, None, None).unwrap();
    call(&mut graph, &test, &focus);
    graph.add_function(test, false, true, 1, 1);
    let item = create_unified_debt_item_enhanced(&metrics(), &graph, None, None).unwrap();
    assert_eq!(item.upstream_production_callers.len(), 1);
    assert_eq!(item.upstream_test_callers.len(), 1);
    assert_eq!(
        item.unified_score.dependency_factor,
        baseline.unified_score.dependency_factor
    );
    assert_eq!(item.production_blast_radius, 1);
    assert_eq!(item.immediate_neighbors(), 2);
}

#[test]
fn unrelated_same_named_definition_does_not_replace_legacy_dependency_metadata() {
    let mut function = metrics();
    function.upstream_callers = Some(vec!["known_caller".into()]);
    let mut foreign = id("dispatch", 1);
    foreign.file = "src/other.rs".into();
    let mut graph = CallGraph::new();
    call(&mut graph, &id("foreign_caller", 2), &foreign);
    let item = create_unified_debt_item_enhanced(&function, &graph, None, None).unwrap();
    assert_eq!(item.upstream_callers, ["known_caller"]);
}
