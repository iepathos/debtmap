/// Deep diagnostic test to understand WHY cross-file calls aren't detected
/// for process_rust_files_for_call_graph

use debtmap::{
    builders::call_graph,
    priority::call_graph::CallGraph,
};
use std::path::PathBuf;

#[test]
fn diagnose_missing_calls() {
    let project_path = PathBuf::from(".");
    let mut call_graph = CallGraph::new();

    let result = call_graph::process_rust_files_for_call_graph(
        &project_path,
        &mut call_graph,
        false,
        false,
    );

    assert!(result.is_ok(), "Failed to build call graph");

    // Find the target function
    let all_functions = call_graph.find_all_functions();
    let target_functions: Vec<_> = all_functions
        .iter()
        .filter(|f| f.name == "process_rust_files_for_call_graph")
        .collect();

    println!("\n=== TARGET FUNCTION ===");
    println!("Found {} instances of process_rust_files_for_call_graph", target_functions.len());
    for func in &target_functions {
        println!("  File: {:?}", func.file);
        println!("  Line: {}", func.line);
        println!("  Module path: {}", func.module_path);

        let callers = call_graph.get_callers(func);
        println!("  Callers: {}", callers.len());
        for caller in callers {
            println!("    - {} ({}:{})", caller.name, caller.file.display(), caller.line);
        }
        println!();
    }

    // Find functions in validate.rs that should be calling it
    let validate_funcs: Vec<_> = all_functions
        .iter()
        .filter(|f| f.file.to_string_lossy().contains("validate.rs"))
        .collect();

    println!("\n=== VALIDATE.RS FUNCTIONS ===");
    println!("Total functions: {}", validate_funcs.len());
    for func in validate_funcs.iter().take(10) {
        println!("  {} at line {}", func.name, func.line);

        // Check what this function calls
        let callees = call_graph.get_callees(&func);
        if !callees.is_empty() {
            println!("    Calls: {}", callees.len());
            for callee in callees.iter().take(5) {
                println!("      -> {}", callee.name);
            }
        }
    }

    // Find functions in unified_analysis.rs
    let unified_funcs: Vec<_> = all_functions
        .iter()
        .filter(|f| f.file.to_string_lossy().contains("unified_analysis.rs"))
        .collect();

    println!("\n=== UNIFIED_ANALYSIS.RS FUNCTIONS ===");
    println!("Total functions: {}", unified_funcs.len());

    // Look specifically for functions that should call process_rust_files_for_call_graph
    let expected_callers = vec![
        "build_cached_call_graph",
        "build_sequential_call_graph",
        "build_parallel_call_graph",
    ];

    for expected in &expected_callers {
        let found: Vec<_> = unified_funcs
            .iter()
            .filter(|f| f.name.contains(expected))
            .collect();

        if found.is_empty() {
            println!("\n⚠️  Function '{}' NOT FOUND in call graph", expected);
        } else {
            for func in found {
                println!("\n  Function: {} at line {}", func.name, func.line);
                let callees = call_graph.get_callees(&func);
                println!("    Calls {} functions:", callees.len());

                let calls_target = callees.iter().any(|c| c.name == "process_rust_files_for_call_graph");
                if calls_target {
                    println!("    ✓ DOES call process_rust_files_for_call_graph");
                } else {
                    println!("    ✗ Does NOT call process_rust_files_for_call_graph");
                    println!("    Actually calls:");
                    for callee in callees.iter().take(10) {
                        println!("      -> {}", callee.name);

                        // Check if indirect calls exist
                        if callee.name == "build_and_cache_graph" || callee.name == "DataFlowGraph::call_graph" {
                            let indirect_callees = call_graph.get_callees(&callee);
                            println!("         (which calls {} functions)", indirect_callees.len());
                            let indirect_calls_target = indirect_callees.iter().any(|c| c.name == "process_rust_files_for_call_graph");
                            if indirect_calls_target {
                                println!("         ✓ INDIRECTLY calls process_rust_files_for_call_graph!");
                            }
                        }
                    }
                }
            }
        }
    }

    // Check if there's a namespace issue
    // Check build_and_cache_graph specifically
    println!("\n=== CRITICAL: CHECK build_and_cache_graph ===");
    let build_and_cache: Vec<_> = unified_funcs
        .iter()
        .filter(|f| f.name == "build_and_cache_graph")
        .collect();

    for func in build_and_cache {
        println!("Function: {} at line {}", func.name, func.line);
        let callees = call_graph.get_callees(&func);
        println!("  Calls {} functions:", callees.len());
        for callee in &callees {
            println!("    -> {}", callee.name);
        }

        let calls_target = callees.iter().any(|c| c.name == "process_rust_files_for_call_graph");
        if calls_target {
            println!("  ✓ DIRECTLY calls process_rust_files_for_call_graph - THIS IS THE MISSING LINK!");
        } else {
            println!("  ✗ Does NOT call process_rust_files_for_call_graph");
            println!("  ⚠️  BUT SOURCE CODE SHOWS IT DOES (line 555)");
            println!("  ⚠️  THIS IS THE BUG - the call isn't being detected!");
        }
    }

    println!("\n=== NAMESPACE CHECK ===");
    let call_graph_namespace_funcs: Vec<_> = all_functions
        .iter()
        .filter(|f| f.module_path.contains("call_graph"))
        .collect();
    println!("Functions with 'call_graph' in module path: {}", call_graph_namespace_funcs.len());

    let process_funcs: Vec<_> = all_functions
        .iter()
        .filter(|f| f.name.contains("process") && f.name.contains("call_graph"))
        .collect();
    println!("Functions with 'process' and 'call_graph' in name: {}", process_funcs.len());
    for func in process_funcs.iter().take(5) {
        println!("  {} (module: {})", func.name, func.module_path);
    }
}

#[test]
fn check_actual_source_code_for_calls() {
    // Read the actual source files and verify the calls exist
    use std::fs;

    let validate_rs = fs::read_to_string("src/commands/validate.rs")
        .expect("Should be able to read validate.rs");

    let has_call = validate_rs.contains("process_rust_files_for_call_graph");
    println!("\nvalidate.rs contains 'process_rust_files_for_call_graph': {}", has_call);

    if has_call {
        // Find the line
        for (i, line) in validate_rs.lines().enumerate() {
            if line.contains("process_rust_files_for_call_graph") {
                println!("  Line {}: {}", i + 1, line.trim());
            }
        }
    }

    let unified_rs = fs::read_to_string("src/builders/unified_analysis.rs")
        .expect("Should be able to read unified_analysis.rs");

    let has_call_unified = unified_rs.contains("process_rust_files_for_call_graph");
    println!("\nunified_analysis.rs contains 'process_rust_files_for_call_graph': {}", has_call_unified);

    if has_call_unified {
        for (i, line) in unified_rs.lines().enumerate() {
            if line.contains("process_rust_files_for_call_graph") {
                println!("  Line {}: {}", i + 1, line.trim());
            }
        }
    }

    assert!(has_call, "validate.rs should contain calls to process_rust_files_for_call_graph");
    assert!(has_call_unified, "unified_analysis.rs should contain calls to process_rust_files_for_call_graph");
}
