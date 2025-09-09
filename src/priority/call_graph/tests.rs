//! Unit tests for the call graph module

use super::*;
use std::path::PathBuf;

#[test]
fn test_call_graph_basic() {
    let mut graph = CallGraph::new();
    let (main_id, helper_id) = create_test_functions();

    add_functions_to_graph(&mut graph, &main_id, &helper_id);
    add_call_edge(&mut graph, &main_id, &helper_id);

    verify_basic_graph_properties(&graph, &main_id, &helper_id);
}

fn create_test_functions() -> (FunctionId, FunctionId) {
    let main_id = FunctionId {
        file: PathBuf::from("main.rs"),
        name: "main".to_string(),
        line: 1,
    };
    let helper_id = FunctionId {
        file: PathBuf::from("lib.rs"),
        name: "helper".to_string(),
        line: 10,
    };
    (main_id, helper_id)
}

fn add_functions_to_graph(graph: &mut CallGraph, main_id: &FunctionId, helper_id: &FunctionId) {
    graph.add_function(main_id.clone(), true, false, 2, 20);
    graph.add_function(helper_id.clone(), false, false, 5, 30);
}

fn add_call_edge(graph: &mut CallGraph, caller: &FunctionId, callee: &FunctionId) {
    graph.add_call(FunctionCall {
        caller: caller.clone(),
        callee: callee.clone(),
        call_type: CallType::Direct,
    });
}

fn verify_basic_graph_properties(graph: &CallGraph, main_id: &FunctionId, helper_id: &FunctionId) {
    assert_eq!(graph.get_callees(main_id).len(), 1);
    assert_eq!(graph.get_callers(helper_id).len(), 1);
    assert!(graph.is_entry_point(main_id));
    assert!(!graph.is_entry_point(helper_id));
}

#[test]
fn test_transitive_dependencies() {
    let mut graph = CallGraph::new();
    let (a, b, c) = create_chain_functions();

    add_chain_to_graph(&mut graph, &a, &b, &c);
    verify_transitive_callees(&graph, &a, &b, &c);
}

fn create_chain_functions() -> (FunctionId, FunctionId, FunctionId) {
    let a = FunctionId {
        file: PathBuf::from("a.rs"),
        name: "a".to_string(),
        line: 1,
    };
    let b = FunctionId {
        file: PathBuf::from("b.rs"),
        name: "b".to_string(),
        line: 1,
    };
    let c = FunctionId {
        file: PathBuf::from("c.rs"),
        name: "c".to_string(),
        line: 1,
    };
    (a, b, c)
}

fn add_chain_to_graph(graph: &mut CallGraph, a: &FunctionId, b: &FunctionId, c: &FunctionId) {
    graph.add_function(a.clone(), true, false, 1, 10);
    graph.add_function(b.clone(), false, false, 2, 20);
    graph.add_function(c.clone(), false, false, 3, 30);

    add_call_edge(graph, a, b);
    add_call_edge(graph, b, c);
}

fn verify_transitive_callees(graph: &CallGraph, a: &FunctionId, b: &FunctionId, c: &FunctionId) {
    let transitive = graph.get_transitive_callees(a, 3);
    assert_eq!(transitive.len(), 2);
    assert!(transitive.contains(b));
    assert!(transitive.contains(c));
}

#[test]
fn test_find_function_at_location() {
    let mut graph = CallGraph::new();
    let file = PathBuf::from("test.rs");
    let functions = create_located_functions(&file);

    add_located_functions(&mut graph, &functions);
    verify_function_location_finding(&graph, &file, &functions);
}

fn create_located_functions(file: &PathBuf) -> Vec<FunctionId> {
    vec![
        FunctionId {
            file: file.clone(),
            name: "function_one".to_string(),
            line: 10,
        },
        FunctionId {
            file: file.clone(),
            name: "function_two".to_string(),
            line: 30,
        },
        FunctionId {
            file: file.clone(),
            name: "function_three".to_string(),
            line: 50,
        },
    ]
}

fn add_located_functions(graph: &mut CallGraph, functions: &[FunctionId]) {
    graph.add_function(functions[0].clone(), false, false, 5, 15);
    graph.add_function(functions[1].clone(), false, false, 3, 10);
    graph.add_function(functions[2].clone(), false, false, 4, 20);
}

fn verify_function_location_finding(graph: &CallGraph, file: &PathBuf, _functions: &[FunctionId]) {
    // Test exact line numbers
    assert_eq!(
        graph
            .find_function_at_location(file, 10)
            .as_ref()
            .map(|f| &f.name),
        Some(&"function_one".to_string())
    );
    assert_eq!(
        graph
            .find_function_at_location(file, 30)
            .as_ref()
            .map(|f| &f.name),
        Some(&"function_two".to_string())
    );

    // Test within range
    assert_eq!(
        graph
            .find_function_at_location(file, 35)
            .as_ref()
            .map(|f| &f.name),
        Some(&"function_two".to_string())
    );

    // Test before any function
    assert_eq!(graph.find_function_at_location(file, 5), None);
}

#[test]
fn test_delegation_detection() {
    let mut graph = CallGraph::new();
    let (orchestrator, workers) = create_delegation_setup();

    setup_delegation_graph(&mut graph, &orchestrator, &workers);
    assert!(graph.detect_delegation_pattern(&orchestrator));
}

fn create_delegation_setup() -> (FunctionId, Vec<FunctionId>) {
    let orchestrator = FunctionId {
        file: PathBuf::from("orch.rs"),
        name: "orchestrate".to_string(),
        line: 1,
    };
    let workers = vec![
        FunctionId {
            file: PathBuf::from("work.rs"),
            name: "complex_work1".to_string(),
            line: 10,
        },
        FunctionId {
            file: PathBuf::from("work.rs"),
            name: "complex_work2".to_string(),
            line: 20,
        },
    ];
    (orchestrator, workers)
}

fn setup_delegation_graph(
    graph: &mut CallGraph,
    orchestrator: &FunctionId,
    workers: &[FunctionId],
) {
    graph.add_function(orchestrator.clone(), false, false, 2, 15);
    graph.add_function(workers[0].clone(), false, false, 10, 50);
    graph.add_function(workers[1].clone(), false, false, 8, 40);

    for worker in workers {
        graph.add_call(FunctionCall {
            caller: orchestrator.clone(),
            callee: worker.clone(),
            call_type: CallType::Delegate,
        });
    }
}

#[test]
fn test_get_transitive_callers_single_level() {
    let mut graph = CallGraph::new();
    let functions = create_caller_test_functions();

    setup_single_level_callers(&mut graph, &functions);
    verify_single_level_callers(&graph, &functions);
}

fn create_caller_test_functions() -> Vec<FunctionId> {
    vec![
        FunctionId {
            file: PathBuf::from("test.rs"),
            name: "func_a".to_string(),
            line: 1,
        },
        FunctionId {
            file: PathBuf::from("test.rs"),
            name: "func_b".to_string(),
            line: 10,
        },
        FunctionId {
            file: PathBuf::from("test.rs"),
            name: "func_c".to_string(),
            line: 20,
        },
    ]
}

fn setup_single_level_callers(graph: &mut CallGraph, funcs: &[FunctionId]) {
    for func in funcs {
        graph.add_function(func.clone(), false, false, 1, 10);
    }

    // a -> b, c -> b (b has two callers)
    add_call_edge(graph, &funcs[0], &funcs[1]);
    add_call_edge(graph, &funcs[2], &funcs[1]);
}

fn verify_single_level_callers(graph: &CallGraph, funcs: &[FunctionId]) {
    let callers = graph.get_transitive_callers(&funcs[1], 1);
    assert_eq!(callers.len(), 2);
    assert!(callers.contains(&funcs[0]));
    assert!(callers.contains(&funcs[2]));
}

#[test]
fn test_get_transitive_callers_multi_level() {
    let mut graph = CallGraph::new();
    let chain = create_caller_chain();

    setup_caller_chain(&mut graph, &chain);
    verify_multi_level_callers(&graph, &chain);
}

fn create_caller_chain() -> Vec<FunctionId> {
    vec![
        FunctionId {
            file: PathBuf::from("test.rs"),
            name: "func_a".to_string(),
            line: 1,
        },
        FunctionId {
            file: PathBuf::from("test.rs"),
            name: "func_b".to_string(),
            line: 10,
        },
        FunctionId {
            file: PathBuf::from("test.rs"),
            name: "func_c".to_string(),
            line: 20,
        },
        FunctionId {
            file: PathBuf::from("test.rs"),
            name: "func_d".to_string(),
            line: 30,
        },
    ]
}

fn setup_caller_chain(graph: &mut CallGraph, chain: &[FunctionId]) {
    for func in chain {
        graph.add_function(func.clone(), false, false, 1, 10);
    }

    // a -> b -> c -> d (chain of calls)
    for i in 0..chain.len() - 1 {
        add_call_edge(graph, &chain[i], &chain[i + 1]);
    }
}

fn verify_multi_level_callers(graph: &CallGraph, chain: &[FunctionId]) {
    // Get all transitive callers of d with max_depth 3
    let callers = graph.get_transitive_callers(&chain[3], 3);
    assert_eq!(callers.len(), 3);
    for i in 0..3 {
        assert!(callers.contains(&chain[i]));
    }

    // Test with limited depth
    let callers_depth_1 = graph.get_transitive_callers(&chain[3], 1);
    assert_eq!(callers_depth_1.len(), 1);
    assert!(callers_depth_1.contains(&chain[2]));
}

#[test]
fn test_get_transitive_callers_with_cycles() {
    let mut graph = CallGraph::new();
    let cycle = create_cycle_functions();

    setup_cycle_graph(&mut graph, &cycle);
    verify_cycle_handling(&graph, &cycle);
}

fn create_cycle_functions() -> Vec<FunctionId> {
    vec![
        FunctionId {
            file: PathBuf::from("test.rs"),
            name: "func_a".to_string(),
            line: 1,
        },
        FunctionId {
            file: PathBuf::from("test.rs"),
            name: "func_b".to_string(),
            line: 10,
        },
        FunctionId {
            file: PathBuf::from("test.rs"),
            name: "func_c".to_string(),
            line: 20,
        },
    ]
}

fn setup_cycle_graph(graph: &mut CallGraph, funcs: &[FunctionId]) {
    for func in funcs {
        graph.add_function(func.clone(), false, false, 1, 10);
    }

    // Create a cycle: a -> b -> c -> a
    add_call_edge(graph, &funcs[0], &funcs[1]);
    add_call_edge(graph, &funcs[1], &funcs[2]);
    add_call_edge(graph, &funcs[2], &funcs[0]);
}

fn verify_cycle_handling(graph: &CallGraph, funcs: &[FunctionId]) {
    // Should handle cycles without infinite loop
    let callers = graph.get_transitive_callers(&funcs[0], 10);
    assert_eq!(callers.len(), 2);
    assert!(callers.contains(&funcs[1]));
    assert!(callers.contains(&funcs[2]));
}

#[test]
fn test_get_transitive_callers_no_callers() {
    let mut graph = CallGraph::new();
    let (a, b) = create_simple_pair();

    setup_simple_graph(&mut graph, &a, &b);
    verify_no_callers(&graph, &a);
}

fn create_simple_pair() -> (FunctionId, FunctionId) {
    let a = FunctionId {
        file: PathBuf::from("test.rs"),
        name: "func_a".to_string(),
        line: 1,
    };
    let b = FunctionId {
        file: PathBuf::from("test.rs"),
        name: "func_b".to_string(),
        line: 10,
    };
    (a, b)
}

fn setup_simple_graph(graph: &mut CallGraph, a: &FunctionId, b: &FunctionId) {
    graph.add_function(a.clone(), false, false, 1, 10);
    graph.add_function(b.clone(), false, false, 1, 10);
    add_call_edge(graph, a, b);
}

fn verify_no_callers(graph: &CallGraph, func: &FunctionId) {
    let callers = graph.get_transitive_callers(func, 5);
    assert_eq!(callers.len(), 0);
}

#[test]
fn test_get_transitive_callers_complex_graph() {
    let mut graph = CallGraph::new();
    let nodes = create_complex_graph_nodes();

    setup_complex_graph(&mut graph, &nodes);
    verify_complex_graph_callers(&graph, &nodes);
}

fn create_complex_graph_nodes() -> Vec<FunctionId> {
    (0..6)
        .map(|i| FunctionId {
            file: PathBuf::from("test.rs"),
            name: format!("func_{}", ('a' as u8 + i) as char),
            line: i as usize * 10 + 1,
        })
        .collect()
}

fn setup_complex_graph(graph: &mut CallGraph, nodes: &[FunctionId]) {
    for node in nodes {
        graph.add_function(node.clone(), false, false, 1, 10);
    }

    // Create complex structure:
    //      a
    //     / \
    //    b   c
    //    |\ /|
    //    | X |
    //    |/ \|
    //    d   e
    //     \ /
    //      f
    add_call_edge(graph, &nodes[0], &nodes[1]); // a -> b
    add_call_edge(graph, &nodes[0], &nodes[2]); // a -> c
    add_call_edge(graph, &nodes[1], &nodes[3]); // b -> d
    add_call_edge(graph, &nodes[1], &nodes[4]); // b -> e
    add_call_edge(graph, &nodes[2], &nodes[3]); // c -> d
    add_call_edge(graph, &nodes[2], &nodes[4]); // c -> e
    add_call_edge(graph, &nodes[3], &nodes[5]); // d -> f
    add_call_edge(graph, &nodes[4], &nodes[5]); // e -> f
}

fn verify_complex_graph_callers(graph: &CallGraph, nodes: &[FunctionId]) {
    // Test transitive callers of f
    let callers_f = graph.get_transitive_callers(&nodes[5], 10);
    assert_eq!(callers_f.len(), 5); // All except f itself
    for i in 0..5 {
        assert!(callers_f.contains(&nodes[i]));
    }

    // Test with limited depth
    let callers_f_depth_2 = graph.get_transitive_callers(&nodes[5], 2);
    assert_eq!(callers_f_depth_2.len(), 4); // d, e, b, c
}

#[test]
fn test_call_graph_serialization_roundtrip() {
    use serde_json;

    let mut graph = CallGraph::new();
    let (func1, func2) = create_serialization_test_functions();

    setup_serialization_graph(&mut graph, &func1, &func2);
    verify_serialization_roundtrip(&graph, &func1, &func2);
}

fn create_serialization_test_functions() -> (FunctionId, FunctionId) {
    let func1 = FunctionId {
        file: PathBuf::from("src/main.rs"),
        name: "main".to_string(),
        line: 10,
    };
    let func2 = FunctionId {
        file: PathBuf::from("src/lib.rs"),
        name: "helper".to_string(),
        line: 25,
    };
    (func1, func2)
}

fn setup_serialization_graph(graph: &mut CallGraph, func1: &FunctionId, func2: &FunctionId) {
    graph.add_function(func1.clone(), true, false, 5, 50);
    graph.add_function(func2.clone(), false, false, 3, 30);
    add_call_edge(graph, func1, func2);
}

fn verify_serialization_roundtrip(graph: &CallGraph, func1: &FunctionId, func2: &FunctionId) {
    let json = serde_json::to_string(graph).unwrap();
    let deserialized: CallGraph = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.get_callees(func1).len(), 1);
    assert_eq!(deserialized.get_callers(func2).len(), 1);
    assert!(deserialized.is_entry_point(func1));
    assert!(!deserialized.is_entry_point(func2));
}

#[test]
fn test_is_cross_file_call_match() {
    verify_exact_match();
    verify_associated_function_match();
    verify_method_with_type_context();
    verify_suffix_matching();
    verify_base_name_extraction();
    verify_non_matches();
}

fn verify_exact_match() {
    assert!(CallGraph::is_cross_file_call_match(
        "my_function",
        "my_function",
        None
    ));
}

fn verify_associated_function_match() {
    assert!(CallGraph::is_cross_file_call_match(
        "ContextualRisk::new",
        "ContextualRisk::new",
        None
    ));
}

fn verify_method_with_type_context() {
    assert!(CallGraph::is_cross_file_call_match(
        "MyStruct::method",
        "method",
        Some("MyStruct")
    ));
}

fn verify_suffix_matching() {
    assert!(CallGraph::is_cross_file_call_match(
        "module::MyStruct::method",
        "MyStruct::method",
        None
    ));
}

fn verify_base_name_extraction() {
    assert!(CallGraph::is_cross_file_call_match(
        "MyStruct::new",
        "new",
        None
    ));
}

fn verify_non_matches() {
    assert!(!CallGraph::is_cross_file_call_match(
        "different_function",
        "my_function",
        None
    ));
    assert!(!CallGraph::is_cross_file_call_match(
        "MyStruct::method",
        "other_method",
        None
    ));
}

#[test]
fn test_select_best_cross_file_match() {
    verify_single_candidate_selection();
    verify_exact_match_preference();
    verify_qualification_preference();
}

fn verify_single_candidate_selection() {
    let caller_file = PathBuf::from("src/caller.rs");
    let other_file = PathBuf::from("src/other.rs");

    let single_candidate = vec![FunctionId {
        file: other_file.clone(),
        name: "test_func".to_string(),
        line: 10,
    }];

    let result = CallGraph::select_best_cross_file_match(
        single_candidate.clone(),
        &caller_file,
        "test_func",
    );
    assert!(result.is_some());
    assert_eq!(result.unwrap().name, "test_func");
}

fn verify_exact_match_preference() {
    let caller_file = PathBuf::from("src/caller.rs");
    let other_file = PathBuf::from("src/other.rs");
    let third_file = PathBuf::from("src/third.rs");

    let candidates = vec![
        FunctionId {
            file: other_file.clone(),
            name: "test_func".to_string(),
            line: 10,
        },
        FunctionId {
            file: third_file.clone(),
            name: "MyStruct::test_func".to_string(),
            line: 20,
        },
    ];

    let result = CallGraph::select_best_cross_file_match(candidates, &caller_file, "test_func");
    assert!(result.is_some());
    assert_eq!(result.unwrap().name, "test_func");
}

fn verify_qualification_preference() {
    let caller_file = PathBuf::from("src/caller.rs");
    let other_file = PathBuf::from("src/other.rs");
    let third_file = PathBuf::from("src/third.rs");

    let candidates = vec![
        FunctionId {
            file: other_file.clone(),
            name: "deep::module::MyStruct::method".to_string(),
            line: 10,
        },
        FunctionId {
            file: third_file.clone(),
            name: "MyStruct::method".to_string(),
            line: 20,
        },
    ];

    let result = CallGraph::select_best_cross_file_match(candidates, &caller_file, "method");
    assert!(result.is_some());
    assert_eq!(result.unwrap().name, "MyStruct::method");
}

#[test]
fn test_is_test_helper_detection() {
    let mut graph = CallGraph::new();
    let functions = create_test_helper_setup();

    setup_test_helper_graph(&mut graph, &functions);
    verify_test_helper_detection(&graph, &functions);
}

fn create_test_helper_setup() -> Vec<FunctionId> {
    vec![
        FunctionId {
            file: PathBuf::from("tests/test.rs"),
            name: "test_something".to_string(),
            line: 10,
        },
        FunctionId {
            file: PathBuf::from("tests/test.rs"),
            name: "test_another".to_string(),
            line: 30,
        },
        FunctionId {
            file: PathBuf::from("src/lib.rs"),
            name: "validate_initial_state".to_string(),
            line: 100,
        },
        FunctionId {
            file: PathBuf::from("src/lib.rs"),
            name: "process_data".to_string(),
            line: 200,
        },
        FunctionId {
            file: PathBuf::from("src/main.rs"),
            name: "main".to_string(),
            line: 1,
        },
    ]
}

fn setup_test_helper_graph(graph: &mut CallGraph, funcs: &[FunctionId]) {
    // Add functions to graph
    graph.add_function(funcs[0].clone(), false, true, 3, 20); // test_something
    graph.add_function(funcs[1].clone(), false, true, 4, 25); // test_another
    graph.add_function(funcs[2].clone(), false, false, 5, 30); // validate_initial_state
    graph.add_function(funcs[3].clone(), false, false, 6, 40); // process_data
    graph.add_function(funcs[4].clone(), true, false, 2, 15); // main

    // Test functions call the helper
    add_call_edge(graph, &funcs[0], &funcs[2]);
    add_call_edge(graph, &funcs[1], &funcs[2]);

    // Main calls regular_func
    add_call_edge(graph, &funcs[4], &funcs[3]);
}

fn verify_test_helper_detection(graph: &CallGraph, funcs: &[FunctionId]) {
    assert!(
        graph.is_test_helper(&funcs[2]),
        "validate_initial_state should be identified as a test helper"
    );
    assert!(
        !graph.is_test_helper(&funcs[3]),
        "process_data should not be identified as a test helper"
    );
    assert!(
        !graph.is_test_helper(&funcs[0]),
        "Test functions should not be identified as test helpers"
    );
}
