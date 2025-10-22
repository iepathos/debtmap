/// Integration test that reproduces the method vs function name disambiguation bug
///
/// Bug: When a method call like `resolver.analyze_imports()` happens,
/// the call graph incorrectly attributes it to ANY function named `analyze_imports`,
/// including standalone functions with the same simple name.
///
/// This test should FAIL with current code, PASS when bug is fixed.
use debtmap::analyzers::rust_call_graph::extract_call_graph;
use debtmap::priority::call_graph::FunctionId;
use std::path::PathBuf;

#[test]
fn test_method_call_does_not_match_standalone_function() {
    // This reproduces the exact bug from the codebase:
    // - standalone.rs has `fn analyze_imports()`
    // - resolver.rs has `impl Resolver { fn analyze_imports() }`
    // - test calls `resolver.analyze_imports()` (the METHOD)
    // - But call graph says the standalone function is called!

    let code = r#"
// File 1: Standalone function (DEAD CODE)
#[allow(dead_code)]
pub fn analyze_imports(path: &str) -> String {
    format!("standalone: {}", path)
}

// File 2: Struct with method (USED CODE)
pub struct ImportResolver {
    prefix: String,
}

impl ImportResolver {
    pub fn analyze_imports(&self, path: &str) -> String {
        format!("{}: {}", self.prefix, path)
    }
}

// File 3: Regular function that calls the METHOD (not standalone)
pub fn test_import_analysis() {
    let resolver = ImportResolver { prefix: "TEST".to_string() };
    let result = resolver.analyze_imports("module.py");
    assert_eq!(result, "TEST: module.py");
}
"#;

    let parsed = syn::parse_file(code).unwrap();
    let path = PathBuf::from("test.rs");
    let call_graph = extract_call_graph(&parsed, &path);

    // Find the standalone function
    let standalone_id = FunctionId {
        file: path.clone(),
        name: "analyze_imports".to_string(), // Simple name
        line: 3,                             // Line where the standalone function is defined
    };

    // Find the method
    let method_id = FunctionId {
        file: path.clone(),
        name: "ImportResolver::analyze_imports".to_string(), // Qualified name
        line: 14,                                            // Line where the method is defined
    };

    // Find the test
    let test_id = FunctionId {
        file: path.clone(),
        name: "test_import_analysis".to_string(),
        line: 20,
    };

    // Verify functions exist in graph
    let all_functions = call_graph.find_all_functions();
    println!("All functions found:");
    for func in &all_functions {
        println!("  - {} at line {}", func.name, func.line);
    }

    // THE BUG: Standalone function incorrectly shows callers
    let standalone_callers = call_graph.get_callers(&standalone_id);
    assert_eq!(
        standalone_callers.len(),
        0,
        "BUG REPRODUCED! Standalone `analyze_imports()` at line {} shows {} caller(s): {:?}, but should have 0! \
         The test calls the METHOD `resolver.analyze_imports()`, not the standalone function.",
        standalone_id.line,
        standalone_callers.len(),
        standalone_callers.iter().map(|c| format!("{}:{}", c.name, c.line)).collect::<Vec<_>>()
    );

    // THE FIX: Method correctly shows callers
    let method_callers = call_graph.get_callers(&method_id);
    assert_eq!(
        method_callers.len(),
        1,
        "Method `ImportResolver::analyze_imports()` should have 1 caller (the test)"
    );
    assert_eq!(
        method_callers[0].name, "test_import_analysis",
        "The test should be calling the method"
    );

    // Verify test calls the method, not the standalone function
    let test_callees = call_graph.get_callees(&test_id);

    println!("\nTest callees:");
    for callee in &test_callees {
        println!("  - {} at line {}", callee.name, callee.line);
    }

    assert!(
        test_callees
            .iter()
            .any(|c| c.name == "ImportResolver::analyze_imports"),
        "Test should call ImportResolver::analyze_imports (the method). Called: {:?}",
        test_callees.iter().map(|c| &c.name).collect::<Vec<_>>()
    );
    assert!(
        !test_callees
            .iter()
            .any(|c| c.name == "analyze_imports" && c.line == 3),
        "Test should NOT call standalone analyze_imports() function"
    );
}

#[test]
fn test_simple_reproduction_method_vs_function() {
    // Simplified version: method and standalone function with same name
    let code = r#"
fn process() {
    println!("standalone");
}

struct Handler;

impl Handler {
    fn process(&self) {
        println!("method");
    }
}

fn caller() {
    let h = Handler;
    h.process();  // Calls the METHOD, not the standalone function
}
"#;

    let parsed = syn::parse_file(code).unwrap();
    let path = PathBuf::from("simple.rs");
    let call_graph = extract_call_graph(&parsed, &path);

    let standalone = FunctionId {
        file: path.clone(),
        name: "process".to_string(),
        line: 2,
    };

    let method = FunctionId {
        file: path.clone(),
        name: "Handler::process".to_string(),
        line: 9,
    };

    println!("\nSimple test - All functions:");
    for func in call_graph.find_all_functions() {
        let callers = call_graph.get_callers(&func);
        println!(
            "  {} (line {}) - {} callers",
            func.name,
            func.line,
            callers.len()
        );
    }

    let standalone_callers = call_graph.get_callers(&standalone);
    let method_callers = call_graph.get_callers(&method);

    assert_eq!(
        standalone_callers.len(),
        0,
        "Standalone process() should have 0 callers, but has {}: {:?}",
        standalone_callers.len(),
        standalone_callers
            .iter()
            .map(|c| &c.name)
            .collect::<Vec<_>>()
    );

    assert_eq!(
        method_callers.len(),
        1,
        "Method Handler::process() should have 1 caller"
    );
}
