use debtmap::analyzers::rust_call_graph::{extract_call_graph, extract_call_graph_multi_file};
use debtmap::core::FunctionMetrics;
use debtmap::priority::call_graph::{CallGraph, FunctionId};
use debtmap::priority::unified_scorer::is_dead_code_with_exclusions;
use std::collections::HashSet;
use std::path::PathBuf;
use syn::parse_file;

/// Test that reproduces the false positive for ContextualRisk::new
/// This test simulates the exact pattern found in the real codebase:
/// - A function defined in one module (context/mod.rs)
/// - Called from another module (mod.rs) using Type::new() syntax
#[test]
fn test_cross_module_associated_function_call_false_positive() {
    // Simulate the caller module (risk/mod.rs)
    let caller_code = r#"
        use self::context::ContextualRisk;
        
        pub fn analyze_function() -> Option<ContextualRisk> {
            let base_risk = 5.0;
            let context_map = create_context_map();
            Some(ContextualRisk::new(base_risk, &context_map))
        }
        
        fn create_context_map() -> ContextMap {
            ContextMap::new()
        }
        
        struct ContextMap;
        impl ContextMap {
            fn new() -> Self { ContextMap }
        }
    "#;

    // Simulate the callee module (risk/context/mod.rs)
    let callee_code = r#"
        pub struct ContextualRisk {
            pub base_risk: f64,
        }
        
        impl ContextualRisk {
            pub fn new(base_risk: f64, context_map: &ContextMap) -> Self {
                Self {
                    base_risk,
                }
            }
        }
        
        use super::ContextMap;
    "#;

    // Parse both files
    let caller_ast = parse_file(caller_code).unwrap();
    let callee_ast = parse_file(callee_code).unwrap();

    // Use the new multi-file extraction that handles cross-file resolution properly
    let caller_path = PathBuf::from("src/risk/mod.rs");
    let callee_path = PathBuf::from("src/risk/context/mod.rs");

    let files = vec![
        (caller_ast, caller_path.clone()),
        (callee_ast, callee_path.clone()),
    ];

    let combined_graph = extract_call_graph_multi_file(&files);

    // Find the actual ContextualRisk::new function from the call graph
    let all_functions = combined_graph.find_all_functions();
    let contextual_risk_new = all_functions
        .iter()
        .find(|f| f.name == "ContextualRisk::new" && f.file == callee_path)
        .expect("Should find ContextualRisk::new function")
        .clone();

    // Debug: Show all functions in the combined graph
    let all_functions = combined_graph.find_all_functions();
    println!("All functions in combined graph:");
    for func in &all_functions {
        println!(
            "  - {} in {} at line {}",
            func.name,
            func.file.display(),
            func.line
        );
    }

    println!("\nLooking for function: {:?}", contextual_risk_new);

    // Check if the function has callers in the call graph
    let callers = combined_graph.get_callers(&contextual_risk_new);

    // THIS IS THE BUG: The call graph should detect the call from analyze_function
    // but currently it doesn't, so callers will be empty
    println!("Callers found: {:?}", callers);

    // This assertion will FAIL, demonstrating the false positive bug
    assert!(
        !callers.is_empty(),
        "Bug reproduced: ContextualRisk::new should have callers but call graph shows none"
    );

    // Create a mock FunctionMetrics for the function
    let func_metrics = FunctionMetrics {
        file: contextual_risk_new.file.clone(),
        name: contextual_risk_new.name.clone(),
        line: contextual_risk_new.line,
        length: 5,
        cyclomatic: 1,
        cognitive: 1,
        nesting: 0,
        is_test: false,
        visibility: Some("pub".to_string()),
        is_trait_method: false,
    };

    // Test dead code detection - this should return FALSE but will return TRUE due to the bug
    let framework_exclusions = HashSet::new();
    let is_dead = is_dead_code_with_exclusions(
        &func_metrics,
        &combined_graph,
        &contextual_risk_new,
        &framework_exclusions,
    );

    // This assertion will FAIL, showing the false positive
    assert!(
        !is_dead,
        "False positive reproduced: ContextualRisk::new incorrectly flagged as dead code"
    );
}

/// Test specifically for the call resolution mechanism
#[test]
fn test_associated_function_call_resolution() {
    let code = r#"
        mod context {
            pub struct ContextualRisk;
            impl ContextualRisk {
                pub fn new() -> Self { Self }
            }
        }
        
        use context::ContextualRisk;
        
        fn caller() {
            let risk = ContextualRisk::new();
        }
    "#;

    let ast = parse_file(code).unwrap();
    let file_path = PathBuf::from("test_file.rs");

    let call_graph = extract_call_graph(&ast, &file_path);

    // Look for the ContextualRisk::new function
    let all_functions = call_graph.find_all_functions();
    let contextual_risk_new = all_functions
        .iter()
        .find(|f| f.name.contains("ContextualRisk::new"))
        .expect("Should find ContextualRisk::new function");

    // Check if it has callers
    let callers = call_graph.get_callers(contextual_risk_new);

    println!("Found function: {:?}", contextual_risk_new);
    println!("Callers: {:?}", callers);

    // This should pass once we fix the bug
    assert!(
        !callers.is_empty(),
        "ContextualRisk::new should have at least one caller (the 'caller' function)"
    );
}
