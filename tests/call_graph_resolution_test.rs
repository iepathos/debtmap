use debtmap::priority::call_graph::{CallGraph, CallType, FunctionCall, FunctionId};
use std::path::PathBuf;

#[test]
fn test_resolve_cross_file_method_calls() {
    let mut call_graph = CallGraph::new();

    // Add a function in cache.rs
    let store_func = FunctionId::new(
        PathBuf::from("src/expansion/cache.rs"),
        "store".to_string(),
        126,
    );
    call_graph.add_function(store_func.clone(), false, false, 3, 36);

    // Add a function in expander.rs that calls store
    let expand_func = FunctionId::new(
        PathBuf::from("src/expansion/expander.rs"),
        "expand_file".to_string(),
        200,
    );
    call_graph.add_function(expand_func.clone(), false, false, 5, 50);

    // Add an unresolved call from expand_file to store
    // This simulates what happens when parsing sees self.cache.store()
    // The parser doesn't know the actual file, so it uses the current file with line 0
    let unresolved_store = FunctionId::new(
        PathBuf::from("src/expansion/expander.rs"),
        "store".to_string(),
        0,
    );

    call_graph.add_call(FunctionCall {
        caller: expand_func.clone(),
        callee: unresolved_store.clone(),
        call_type: CallType::Direct,
    });

    // Before resolution, store should have no callers
    assert_eq!(
        call_graph.get_callers(&store_func).len(),
        0,
        "Before resolution, store should have no callers"
    );

    // Resolve cross-file calls
    call_graph.resolve_cross_file_calls();

    // After resolution, store should have one caller
    assert_eq!(
        call_graph.get_callers(&store_func).len(),
        1,
        "After resolution, store should have one caller"
    );

    // The caller should be expand_file
    let callers = call_graph.get_callers(&store_func);
    assert_eq!(callers[0], expand_func);
}

#[test]
fn test_resolve_handles_multiple_candidates() {
    let mut call_graph = CallGraph::new();

    // Add two functions with the same name in different files
    let process1 = FunctionId::new(PathBuf::from("src/module1.rs"), "process".to_string(), 10);
    call_graph.add_function(process1.clone(), false, false, 2, 20);

    let process2 = FunctionId::new(PathBuf::from("src/module2.rs"), "process".to_string(), 15);
    call_graph.add_function(process2.clone(), false, false, 3, 25);

    // Add a caller
    let main_func = FunctionId::new(PathBuf::from("src/main.rs"), "main".to_string(), 5);
    call_graph.add_function(main_func.clone(), true, false, 1, 10);

    // Add an unresolved call to "process"
    let unresolved_process =
        FunctionId::new(PathBuf::from("src/main.rs"), "process".to_string(), 0);

    call_graph.add_call(FunctionCall {
        caller: main_func.clone(),
        callee: unresolved_process.clone(),
        call_type: CallType::Direct,
    });

    // Resolve cross-file calls
    call_graph.resolve_cross_file_calls();

    // With multiple candidates, the call should remain unresolved
    // Neither process1 nor process2 should have callers
    assert_eq!(
        call_graph.get_callers(&process1).len(),
        0,
        "process1 should have no callers when ambiguous"
    );
    assert_eq!(
        call_graph.get_callers(&process2).len(),
        0,
        "process2 should have no callers when ambiguous"
    );
}

#[test]
fn test_resolve_preserves_resolved_calls() {
    let mut call_graph = CallGraph::new();

    // Add functions
    let func_a = FunctionId::new(PathBuf::from("src/a.rs"), "func_a".to_string(), 10);
    call_graph.add_function(func_a.clone(), false, false, 2, 20);

    let func_b = FunctionId::new(PathBuf::from("src/b.rs"), "func_b".to_string(), 15);
    call_graph.add_function(func_b.clone(), false, false, 3, 25);

    // Add a properly resolved call (line != 0)
    call_graph.add_call(FunctionCall {
        caller: func_a.clone(),
        callee: func_b.clone(),
        call_type: CallType::Direct,
    });

    // func_b should have one caller before resolution
    assert_eq!(call_graph.get_callers(&func_b).len(), 1);

    // Resolve cross-file calls
    call_graph.resolve_cross_file_calls();

    // func_b should still have one caller after resolution
    assert_eq!(
        call_graph.get_callers(&func_b).len(),
        1,
        "Already resolved calls should be preserved"
    );
}
