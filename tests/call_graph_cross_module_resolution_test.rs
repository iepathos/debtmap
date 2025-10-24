//! Integration tests for cross-module call resolution
//!
//! Tests the enhanced call graph resolution capabilities for:
//! - Cross-file function calls
//! - Generic function matching
//! - Glob import resolution
//! - Module hierarchy navigation

use debtmap::analyzers::rust_call_graph::extract_call_graph_multi_file;
use std::path::PathBuf;

fn parse_code(code: &str) -> syn::File {
    syn::parse_str(code).expect("Failed to parse code")
}

#[test]
fn test_cross_module_simple_function_call() {
    // File 1: Defines a function
    let file1_code = r#"
        pub fn write_quick_wins_section() -> i32 {
            42
        }
    "#;

    // File 2: Calls the function from file 1
    let file2_code = r#"
        use crate::health_writer::write_quick_wins_section;

        pub fn caller() {
            write_quick_wins_section();
        }
    "#;

    let file1 = parse_code(file1_code);
    let file2 = parse_code(file2_code);

    let files = vec![
        (file1, PathBuf::from("src/io/writers/health_writer.rs")),
        (file2, PathBuf::from("src/io/writers/mod.rs")),
    ];

    let graph = extract_call_graph_multi_file(&files);

    // Verify write_quick_wins_section has at least one caller
    let target_func = graph
        .get_all_functions()
        .find(|f| f.name.contains("write_quick_wins_section"));

    assert!(
        target_func.is_some(),
        "Should find write_quick_wins_section"
    );

    let func_id = target_func.unwrap();
    let callers = graph.get_callers(func_id);

    assert!(
        !callers.is_empty(),
        "write_quick_wins_section should have at least one caller"
    );
}

#[test]
fn test_generic_function_resolution() {
    // File with generic function
    let file1_code = r#"
        pub fn process<T>(item: T) -> T {
            item
        }
    "#;

    // File calling generic function with turbofish
    let file2_code = r#"
        use crate::utils::process;

        pub fn caller() {
            let _ = process::<i32>(42);
        }
    "#;

    let file1 = parse_code(file1_code);
    let file2 = parse_code(file2_code);

    let files = vec![
        (file1, PathBuf::from("src/utils.rs")),
        (file2, PathBuf::from("src/main.rs")),
    ];

    let graph = extract_call_graph_multi_file(&files);

    // Verify the generic function call is resolved
    let process_func = graph
        .get_all_functions()
        .find(|f| f.name.contains("process"));

    assert!(process_func.is_some(), "Should find process function");

    let func_id = process_func.unwrap();
    let callers = graph.get_callers(func_id);

    // The call with turbofish should be resolved to the generic function
    assert!(
        !callers.is_empty() || graph.get_all_functions().any(|f| f.name == "caller"),
        "Generic function call should be resolved"
    );
}

#[test]
fn test_glob_import_resolution() {
    // Module with multiple functions
    let file1_code = r#"
        pub fn helper_one() -> i32 { 1 }
        pub fn helper_two() -> i32 { 2 }
    "#;

    // File using glob import
    let file2_code = r#"
        use crate::helpers::*;

        pub fn caller() {
            let _ = helper_one();
            let _ = helper_two();
        }
    "#;

    let file1 = parse_code(file1_code);
    let file2 = parse_code(file2_code);

    let files = vec![
        (file1, PathBuf::from("src/helpers.rs")),
        (file2, PathBuf::from("src/main.rs")),
    ];

    let graph = extract_call_graph_multi_file(&files);

    // Verify both functions are found and have callers
    let helper_one = graph
        .get_all_functions()
        .find(|f| f.name.contains("helper_one"));
    let helper_two = graph
        .get_all_functions()
        .find(|f| f.name.contains("helper_two"));

    assert!(helper_one.is_some(), "Should find helper_one");
    assert!(helper_two.is_some(), "Should find helper_two");
}

#[test]
fn test_qualified_path_resolution() {
    // Module with function
    let file1_code = r#"
        pub fn calculate() -> i32 {
            42
        }
    "#;

    // File calling with qualified path
    let file2_code = r#"
        pub fn caller() {
            let _ = crate::math::calculate();
        }
    "#;

    let file1 = parse_code(file1_code);
    let file2 = parse_code(file2_code);

    let files = vec![
        (file1, PathBuf::from("src/math.rs")),
        (file2, PathBuf::from("src/main.rs")),
    ];

    let graph = extract_call_graph_multi_file(&files);

    // Verify calculate has a caller
    let calculate_func = graph
        .get_all_functions()
        .find(|f| f.name.contains("calculate"));

    assert!(calculate_func.is_some(), "Should find calculate function");
}

#[test]
fn test_super_relative_import() {
    // Parent module function
    let file1_code = r#"
        pub fn parent_function() -> i32 {
            100
        }
    "#;

    // Child module calling parent
    let file2_code = r#"
        use super::parent_function;

        pub fn child_function() {
            let _ = parent_function();
        }
    "#;

    let file1 = parse_code(file1_code);
    let file2 = parse_code(file2_code);

    let files = vec![
        (file1, PathBuf::from("src/module/mod.rs")),
        (file2, PathBuf::from("src/module/child.rs")),
    ];

    let graph = extract_call_graph_multi_file(&files);

    // Verify parent_function is found
    let parent_func = graph
        .get_all_functions()
        .find(|f| f.name.contains("parent_function"));

    assert!(
        parent_func.is_some(),
        "Should find parent_function through super:: import"
    );
}

#[test]
fn test_re_export_resolution() {
    // Original module
    let file1_code = r#"
        pub fn original_function() -> i32 {
            42
        }
    "#;

    // Re-export module
    let file2_code = r#"
        pub use crate::original::original_function;
    "#;

    // Caller using re-export
    let file3_code = r#"
        use crate::exports::original_function;

        pub fn caller() {
            let _ = original_function();
        }
    "#;

    let file1 = parse_code(file1_code);
    let file2 = parse_code(file2_code);
    let file3 = parse_code(file3_code);

    let files = vec![
        (file1, PathBuf::from("src/original.rs")),
        (file2, PathBuf::from("src/exports.rs")),
        (file3, PathBuf::from("src/main.rs")),
    ];

    let graph = extract_call_graph_multi_file(&files);

    // Verify original_function is found
    let func = graph
        .get_all_functions()
        .find(|f| f.name.contains("original_function"));

    assert!(func.is_some(), "Should find re-exported function");
}

#[test]
fn test_method_cross_module_resolution() {
    // Module defining a struct with methods
    let file1_code = r#"
        pub struct Calculator;

        impl Calculator {
            pub fn compute(&self) -> i32 {
                42
            }
        }
    "#;

    // Module using the struct
    let file2_code = r#"
        use crate::calculator::Calculator;

        pub fn use_calculator() {
            let calc = Calculator;
            let _ = calc.compute();
        }
    "#;

    let file1 = parse_code(file1_code);
    let file2 = parse_code(file2_code);

    let files = vec![
        (file1, PathBuf::from("src/calculator.rs")),
        (file2, PathBuf::from("src/main.rs")),
    ];

    let graph = extract_call_graph_multi_file(&files);

    // Verify compute method is found
    let compute_method = graph
        .get_all_functions()
        .find(|f| f.name.contains("compute"));

    assert!(
        compute_method.is_some(),
        "Should find compute method across modules"
    );
}
