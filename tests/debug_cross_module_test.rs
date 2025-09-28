/// Debug test for cross-module functionality
use debtmap::analysis::python_call_graph::{analyze_python_project, build_cross_module_context};
use std::fs;
use tempfile::TempDir;

#[test]
fn test_debug_context_building() {
    let temp_dir = TempDir::new().unwrap();

    // Simple module with functions
    let test_path = temp_dir.path().join("test.py");
    fs::write(
        &test_path,
        r#"
def func_a():
    return 1

def func_b():
    return func_a() + 1
"#,
    )
    .unwrap();

    // Build context
    let files = vec![test_path.clone()];
    let context = build_cross_module_context(&files).unwrap();

    // Check what's in the context
    println!(
        "Context symbols: {:?}",
        context.symbols.keys().collect::<Vec<_>>()
    );
    println!("Context exports: {:?}", context.exports);

    // Now analyze with the project function
    let call_graph = analyze_python_project(&files).unwrap();

    // Check functions
    let all_functions: Vec<_> = call_graph.get_all_functions().collect();
    println!("Found {} functions in call graph", all_functions.len());
    for func in &all_functions {
        println!("  - {}", func.name);
    }

    assert!(all_functions.len() > 0, "Should find some functions");
}
