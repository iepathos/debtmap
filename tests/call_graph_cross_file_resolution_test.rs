/// Test to reproduce the cross-file call resolution bug where
/// `process_rust_files_for_call_graph()` shows 0 callers despite
/// being called from 3 locations.
///
/// This test creates a minimal reproduction case with:
/// - A "builder" module that has a function to build a call graph
/// - A "command" module that calls the builder function
/// - A "unified" module that also calls the builder function
///
/// Expected: The call graph should detect 2 callers for the builder function
/// Actual (bug): The call graph shows 0 callers

use debtmap::{
    builders::call_graph,
    priority::call_graph::CallGraph,
};
use std::path::PathBuf;
use tempfile::TempDir;

/// Create a test project structure that mirrors the actual issue
fn create_test_project() -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    std::fs::create_dir_all(&src_dir).unwrap();

    // Create the "builder" module that contains the call graph building function
    // This mirrors src/builders/call_graph.rs
    let builder_content = r#"
use std::path::Path;
use std::collections::HashMap;

pub type CallGraph = HashMap<String, Vec<String>>;

/// This function builds a call graph by analyzing all Rust files.
/// It should have callers from command.rs and unified.rs
pub fn build_project_call_graph(
    project_path: &Path,
    graph: &mut CallGraph,
) -> Result<(), String> {
    // Simulate call graph building
    // This function calls some helpers
    analyze_rust_files(project_path)?;
    resolve_cross_file_calls(graph)?;
    Ok(())
}

fn analyze_rust_files(_path: &Path) -> Result<(), String> {
    Ok(())
}

fn resolve_cross_file_calls(_graph: &mut CallGraph) -> Result<(), String> {
    Ok(())
}
"#;
    std::fs::write(src_dir.join("builder.rs"), builder_content).unwrap();

    // Create the "command" module that calls the builder function
    // This mirrors src/commands/validate.rs
    let command_content = r#"
use std::path::Path;
use crate::builder::{CallGraph, build_project_call_graph};

pub fn execute_validate_command(project_path: &Path) -> Result<(), String> {
    let mut call_graph = CallGraph::new();

    // This is a call to build_project_call_graph - should be detected
    build_project_call_graph(project_path, &mut call_graph)?;

    println!("Validation complete");
    Ok(())
}
"#;
    std::fs::write(src_dir.join("command.rs"), command_content).unwrap();

    // Create the "unified" module that also calls the builder function
    // This mirrors src/builders/unified_analysis.rs
    let unified_content = r#"
use std::path::Path;
use crate::builder::{CallGraph, build_project_call_graph};

pub fn perform_unified_analysis(project_path: &Path) -> Result<(), String> {
    let mut call_graph = CallGraph::new();

    // This is another call to build_project_call_graph - should be detected
    build_project_call_graph(project_path, &mut call_graph)?;

    Ok(())
}

pub fn perform_analysis_with_cache(project_path: &Path) -> Result<(), String> {
    let mut call_graph = CallGraph::new();

    // Third call to build_project_call_graph - should be detected
    build_project_call_graph(project_path, &mut call_graph)?;

    Ok(())
}
"#;
    std::fs::write(src_dir.join("unified.rs"), unified_content).unwrap();

    // Create lib.rs that declares the modules
    let lib_content = r#"
pub mod builder;
pub mod command;
pub mod unified;
"#;
    std::fs::write(src_dir.join("lib.rs"), lib_content).unwrap();

    temp_dir
}

#[test]
fn test_cross_file_call_resolution_detects_callers() {
    let test_project = create_test_project();
    let project_path = test_project.path();

    // Build the call graph for this test project
    let mut call_graph = CallGraph::new();

    let result = call_graph::process_rust_files_for_call_graph(
        project_path,
        &mut call_graph,
        false, // verbose_macro_warnings
        false, // show_macro_stats
    );

    assert!(result.is_ok(), "Call graph construction failed: {:?}", result.err());

    // Get the callers for build_project_call_graph using the name-based API
    let caller_names = call_graph.get_callers_by_name("build_project_call_graph");

    println!("Found callers: {:?}", caller_names);

    // We expect 3 callers:
    // 1. command::execute_validate_command
    // 2. unified::perform_unified_analysis
    // 3. unified::perform_analysis_with_cache
    assert!(
        caller_names.len() >= 3,
        "Expected at least 3 callers for build_project_call_graph, found {}: {:?}",
        caller_names.len(),
        caller_names
    );

    // At minimum, we should find calls from command and unified modules
    assert!(
        caller_names.iter().any(|name| name.contains("execute_validate_command")),
        "Should find caller from command module"
    );
    assert!(
        caller_names.iter().any(|name| name.contains("perform_unified_analysis")),
        "Should find caller from unified module"
    );
}

#[test]
fn test_qualified_call_resolution() {
    let test_project = create_test_project();
    let project_path = test_project.path();

    // Build the call graph
    let mut call_graph = CallGraph::new();

    let result = call_graph::process_rust_files_for_call_graph(
        project_path,
        &mut call_graph,
        false,
        false,
    );

    assert!(result.is_ok());

    // The calls in our test use qualified names like:
    // crate::builder::build_project_call_graph(...)
    // or
    // builder::build_project_call_graph(...)
    //
    // The call graph should resolve these qualified calls
    let caller_names = call_graph.get_callers_by_name("build_project_call_graph");

    assert!(
        !caller_names.is_empty(),
        "Qualified calls should be resolved. Found 0 callers for build_project_call_graph"
    );
}

#[test]
fn test_self_referential_call_detection() {
    // This test directly reproduces the actual bug:
    // process_rust_files_for_call_graph is called from validate.rs and unified_analysis.rs
    // but shows 0 callers

    let project_path = PathBuf::from(".");
    let mut call_graph = CallGraph::new();

    // Build call graph on debtmap's own codebase
    let result = call_graph::process_rust_files_for_call_graph(
        &project_path,
        &mut call_graph,
        false,
        false,
    );

    if result.is_err() {
        // Skip test if we can't build the call graph
        println!("Skipping test - couldn't build call graph: {:?}", result.err());
        return;
    }

    // Debug: Print all functions in the call graph
    let all_functions = call_graph.find_all_functions();
    println!("\nTotal functions in call graph: {}", all_functions.len());

    // Check if the expected callers are in the graph
    let validate_functions: Vec<_> = all_functions.iter()
        .filter(|f| f.file.to_string_lossy().contains("validate.rs"))
        .collect();
    let unified_functions: Vec<_> = all_functions.iter()
        .filter(|f| f.file.to_string_lossy().contains("unified_analysis.rs"))
        .collect();

    println!("Functions in validate.rs: {}", validate_functions.len());
    println!("Functions in unified_analysis.rs: {}", unified_functions.len());

    // Look for process_rust_files_for_call_graph itself
    let caller_names = call_graph.get_callers_by_name("process_rust_files_for_call_graph");

    println!("\nCallers for process_rust_files_for_call_graph: {}", caller_names.len());
    println!("Caller names: {:?}", caller_names);

    // Check if validate.rs and unified_analysis.rs are even in the call graph
    if validate_functions.is_empty() {
        println!("\n⚠️  ISSUE: validate.rs has NO functions in call graph - file may be excluded");
    }
    if unified_functions.is_empty() {
        println!("\n⚠️  ISSUE: unified_analysis.rs has NO functions in call graph - file may be excluded");
    }

    // This is the actual bug: we expect >= 3 callers (from validate.rs and unified_analysis.rs)
    // but currently get 0
    assert!(
        caller_names.len() >= 3,
        "BUG REPRODUCED: Expected >= 3 callers for process_rust_files_for_call_graph, found {}",
        caller_names.len()
    );
}

#[test]
fn test_namespace_resolution_with_use_statements() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    std::fs::create_dir_all(&src_dir).unwrap();

    // Create module A with a public function
    let module_a = r#"
pub fn target_function() {
    println!("target");
}
"#;
    std::fs::write(src_dir.join("module_a.rs"), module_a).unwrap();

    // Create module B that imports and calls the function with different syntaxes
    let module_b = r#"
use crate::module_a::target_function;
use crate::module_a;

pub fn caller_with_direct_import() {
    // Direct call via use statement
    target_function();
}

pub fn caller_with_qualified_name() {
    // Qualified call
    module_a::target_function();
}

pub fn caller_with_full_path() {
    // Fully qualified call
    crate::module_a::target_function();
}
"#;
    std::fs::write(src_dir.join("module_b.rs"), module_b).unwrap();

    let lib_content = r#"
pub mod module_a;
pub mod module_b;
"#;
    std::fs::write(src_dir.join("lib.rs"), lib_content).unwrap();

    // Build call graph
    let mut call_graph = CallGraph::new();
    let result = call_graph::process_rust_files_for_call_graph(
        temp_dir.path(),
        &mut call_graph,
        false,
        false,
    );

    assert!(result.is_ok());

    let caller_names = call_graph.get_callers_by_name("target_function");

    println!("Found {} callers for target_function", caller_names.len());

    // Should detect all 3 different call syntaxes
    assert!(
        caller_names.len() >= 3,
        "Expected 3 callers (direct import, qualified, full path), found {}",
        caller_names.len()
    );
}

#[test]
fn test_module_path_resolution() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    std::fs::create_dir_all(&src_dir).unwrap();

    // Create nested module structure: src/builders/call_graph.rs
    let builders_dir = src_dir.join("builders");
    std::fs::create_dir_all(&builders_dir).unwrap();

    let call_graph_module = r#"
pub fn process_files() {
    helper_function();
}

fn helper_function() {
    println!("helper");
}
"#;
    std::fs::write(builders_dir.join("call_graph.rs"), call_graph_module).unwrap();

    // Create commands module that calls into builders
    let commands_dir = src_dir.join("commands");
    std::fs::create_dir_all(&commands_dir).unwrap();

    let validate_module = r#"
use crate::builders::call_graph;

pub fn validate() {
    // Should be detected as a call to builders::call_graph::process_files
    call_graph::process_files();
}
"#;
    std::fs::write(commands_dir.join("validate.rs"), validate_module).unwrap();

    // Create builders/mod.rs
    std::fs::write(builders_dir.join("mod.rs"), "pub mod call_graph;").unwrap();

    // Create commands/mod.rs
    std::fs::write(commands_dir.join("mod.rs"), "pub mod validate;").unwrap();

    let lib_content = r#"
pub mod builders;
pub mod commands;
"#;
    std::fs::write(src_dir.join("lib.rs"), lib_content).unwrap();

    // Build call graph
    let mut call_graph = CallGraph::new();
    let result = call_graph::process_rust_files_for_call_graph(
        temp_dir.path(),
        &mut call_graph,
        false,
        false,
    );

    assert!(result.is_ok());

    let caller_names = call_graph.get_callers_by_name("process_files");

    println!("Found {} callers for builders::call_graph::process_files", caller_names.len());

    assert!(
        caller_names.len() >= 1,
        "Should find call from commands::validate, found {}",
        caller_names.len()
    );
}
