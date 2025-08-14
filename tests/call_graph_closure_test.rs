use debtmap::analyzers::rust_call_graph::{extract_call_graph, CallGraphExtractor};
use debtmap::priority::call_graph::{CallGraph, CallType, FunctionCall, FunctionId};
use std::path::PathBuf;
use syn::visit::Visit;

/// Helper to parse code and extract call graph
fn parse_and_extract(code: &str) -> CallGraph {
    let file = syn::parse_file(code).expect("Failed to parse code");
    extract_call_graph(&file, &PathBuf::from("test.rs"))
}

/// Helper to check if a specific call exists in the graph
fn has_call(graph: &CallGraph, caller_name: &str, callee_name: &str) -> bool {
    graph.get_all_calls().iter().any(|call| {
        call.caller.name == caller_name && call.callee.name == callee_name
    })
}

#[test]
fn test_simple_closure_with_function_call() {
    let code = r#"
fn outer() {
    let items = vec![1, 2, 3];
    items.iter().map(|x| helper(x)).collect();
}

fn helper(x: i32) -> i32 {
    x * 2
}
"#;

    let graph = parse_and_extract(code);
    
    // Should detect the call from outer to helper
    assert!(
        has_call(&graph, "outer", "helper"),
        "Should detect call to helper inside closure"
    );
}

#[test]
fn test_closure_in_filter_map_with_some() {
    let code = r#"
fn process_items(items: Vec<String>) {
    let result: Vec<String> = items
        .iter()
        .filter_map(|item| {
            if item.len() > 0 {
                Some(transform(item))
            } else {
                None
            }
        })
        .collect();
}

fn transform(s: &str) -> String {
    s.to_uppercase()
}
"#;

    let graph = parse_and_extract(code);
    
    // Should detect the call from process_items to transform
    assert!(
        has_call(&graph, "process_items", "transform"),
        "Should detect call to transform inside filter_map closure"
    );
}

#[test]
fn test_nested_closures() {
    let code = r#"
fn outer() {
    let items = vec![vec![1, 2], vec![3, 4]];
    items.iter().map(|inner| {
        inner.iter().map(|x| process(x)).collect()
    }).collect();
}

fn process(x: &i32) -> i32 {
    *x * 2
}
"#;

    let graph = parse_and_extract(code);
    
    // Should detect the call from outer to process (even in nested closure)
    assert!(
        has_call(&graph, "outer", "process"),
        "Should detect call to process inside nested closures"
    );
}

#[test]
fn test_closure_with_multiple_calls() {
    let code = r#"
fn orchestrator() {
    let items = vec![1, 2, 3];
    items.iter().map(|x| {
        let validated = validate(x);
        let processed = process(validated);
        finalize(processed)
    }).collect();
}

fn validate(x: &i32) -> i32 { *x }
fn process(x: i32) -> i32 { x * 2 }
fn finalize(x: i32) -> i32 { x + 1 }
"#;

    let graph = parse_and_extract(code);
    
    assert!(
        has_call(&graph, "orchestrator", "validate"),
        "Should detect call to validate"
    );
    assert!(
        has_call(&graph, "orchestrator", "process"),
        "Should detect call to process"
    );
    assert!(
        has_call(&graph, "orchestrator", "finalize"),
        "Should detect call to finalize"
    );
}

#[test]
fn test_closure_with_method_calls() {
    let code = r#"
fn process_strings(items: Vec<String>) {
    items.iter().filter_map(|s| {
        if s.is_empty() {
            None
        } else {
            Some(transform_string(s))
        }
    }).collect();
}

fn transform_string(s: &str) -> String {
    s.trim().to_uppercase()
}
"#;

    let graph = parse_and_extract(code);
    
    assert!(
        has_call(&graph, "process_strings", "transform_string"),
        "Should detect call to transform_string inside closure"
    );
}

#[test]
fn test_closure_passed_as_argument() {
    let code = r#"
fn setup() {
    register_handler(|| {
        handle_event()
    });
}

fn register_handler<F: Fn()>(f: F) {
    f();
}

fn handle_event() {
    println!("Event handled");
}
"#;

    let graph = parse_and_extract(code);
    
    assert!(
        has_call(&graph, "setup", "handle_event"),
        "Should detect call to handle_event inside closure passed as argument"
    );
}

#[test]
fn test_closure_with_if_else_branches() {
    let code = r#"
fn process_conditionally(items: Vec<i32>) {
    items.iter().map(|x| {
        if *x > 0 {
            process_positive(x)
        } else {
            process_negative(x)
        }
    }).collect();
}

fn process_positive(x: &i32) -> i32 { *x }
fn process_negative(x: &i32) -> i32 { -*x }
"#;

    let graph = parse_and_extract(code);
    
    assert!(
        has_call(&graph, "process_conditionally", "process_positive"),
        "Should detect call to process_positive in if branch"
    );
    assert!(
        has_call(&graph, "process_conditionally", "process_negative"),
        "Should detect call to process_negative in else branch"
    );
}

#[test]
fn test_async_closure() {
    let code = r#"
async fn process_async(items: Vec<i32>) {
    let futures = items.iter().map(|x| async move {
        process_item(x).await
    });
}

async fn process_item(x: &i32) -> i32 {
    *x * 2
}
"#;

    let graph = parse_and_extract(code);
    
    assert!(
        has_call(&graph, "process_async", "process_item"),
        "Should detect call to process_item inside async closure"
    );
}

#[test]
fn test_function_reference_in_closure() {
    let code = r#"
fn apply_operations(items: Vec<i32>) {
    items.iter()
        .map(|x| transform(x))
        .for_each(print_result);
}

fn transform(x: &i32) -> i32 { *x * 2 }
fn print_result(x: i32) { println!("{}", x); }
"#;

    let graph = parse_and_extract(code);
    
    assert!(
        has_call(&graph, "apply_operations", "transform"),
        "Should detect call to transform"
    );
    // Function references passed directly (not called) might be detected differently
    // This tests the current behavior
}

#[test]
fn test_calls_not_in_closures() {
    let code = r#"
fn regular_function() {
    helper1();
    let x = helper2(5);
    helper3(x);
}

fn helper1() {}
fn helper2(x: i32) -> i32 { x }
fn helper3(x: i32) {}
"#;

    let graph = parse_and_extract(code);
    
    assert!(has_call(&graph, "regular_function", "helper1"));
    assert!(has_call(&graph, "regular_function", "helper2"));
    assert!(has_call(&graph, "regular_function", "helper3"));
}

// This is the exact pattern from coupling.rs that's failing
#[test]
fn test_extract_module_pattern() {
    let code = r#"
use std::collections::HashSet;

pub fn build_module_dependency_map(deps: Vec<Dependency>) {
    let dependencies: HashSet<String> = deps
        .iter()
        .filter_map(|dep| {
            if dep.is_import() {
                Some(extract_module_from_import(&dep.name))
            } else {
                None
            }
        })
        .collect();
}

fn extract_module_from_import(import: &str) -> String {
    import.split("::").next().unwrap_or(import).to_string()
}

struct Dependency {
    name: String,
}

impl Dependency {
    fn is_import(&self) -> bool { true }
}
"#;

    let graph = parse_and_extract(code);
    
    // This is the critical test - should detect the call inside filter_map
    assert!(
        has_call(&graph, "build_module_dependency_map", "extract_module_from_import"),
        "Should detect call to extract_module_from_import inside filter_map closure with Some()"
    );
    
    // Also check that the function exists in the graph
    let all_functions: Vec<String> = graph.find_all_functions()
        .into_iter()
        .map(|f| f.name)
        .collect();
    
    assert!(
        all_functions.contains(&"extract_module_from_import".to_string()),
        "extract_module_from_import should be registered as a function"
    );
}

#[test]
fn test_closure_with_match_expression() {
    let code = r#"
fn process_options(items: Vec<Option<i32>>) {
    items.iter().map(|opt| {
        match opt {
            Some(x) => process_some(x),
            None => process_none(),
        }
    }).collect();
}

fn process_some(x: &i32) -> i32 { *x }
fn process_none() -> i32 { 0 }
"#;

    let graph = parse_and_extract(code);
    
    assert!(
        has_call(&graph, "process_options", "process_some"),
        "Should detect call in match arm"
    );
    assert!(
        has_call(&graph, "process_options", "process_none"),
        "Should detect call in match arm"
    );
}

#[test]
fn test_closure_with_early_return() {
    let code = r#"
fn process_with_validation(items: Vec<i32>) {
    items.iter().filter_map(|x| {
        if !validate(x) {
            return None;
        }
        Some(transform(x))
    }).collect();
}

fn validate(x: &i32) -> bool { *x > 0 }
fn transform(x: &i32) -> i32 { *x * 2 }
"#;

    let graph = parse_and_extract(code);
    
    assert!(
        has_call(&graph, "process_with_validation", "validate"),
        "Should detect validate call"
    );
    assert!(
        has_call(&graph, "process_with_validation", "transform"),
        "Should detect transform call after early return"
    );
}