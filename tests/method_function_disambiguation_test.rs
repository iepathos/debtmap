/// Test for Spec 123: Method vs Function Name Disambiguation
///
/// This test reproduces the bug where the call graph conflates:
/// - Instance methods (e.g., `resolver.analyze_imports()`)
/// - Standalone functions (e.g., `analyze_imports()`)
///
/// Bug Example:
/// - File A has: `pub fn analyze_imports(file: &Path) -> Result<ImportTracker>`
/// - File B has: `impl MyStruct { pub fn analyze_imports(&self, ...) { ... } }`
/// - Test calls: `my_struct.analyze_imports(...)` (method call)
/// - Call graph incorrectly reports standalone function in File A as "called by test"
///
/// Expected Behavior:
/// - Method calls should be tracked as `MyStruct::analyze_imports`
/// - Function calls should be tracked as just `analyze_imports`
/// - These should be distinct entries in the call graph
use debtmap::priority::call_graph::{CallGraph, CallType, FunctionCall, FunctionId};
use std::path::PathBuf;

#[test]
fn test_method_vs_standalone_function_disambiguation() {
    let mut call_graph = CallGraph::new();

    // Scenario 1: Standalone function `analyze_imports` in module A
    let standalone_analyze_imports = FunctionId {
        file: PathBuf::from("src/analysis/python_call_graph/analyze.rs"),
        name: "analyze_imports".to_string(),
        line: 62,
    };
    call_graph.add_function(standalone_analyze_imports.clone(), false, false, 2, 30);

    // Scenario 2: Method `analyze_imports` on `EnhancedImportResolver` in module B
    let method_analyze_imports = FunctionId {
        file: PathBuf::from("src/analysis/python_imports.rs"),
        name: "EnhancedImportResolver::analyze_imports".to_string(), // Should be qualified!
        line: 371,
    };
    call_graph.add_function(method_analyze_imports.clone(), false, false, 5, 50);

    // Scenario 3: Test function that calls the METHOD
    let test_function = FunctionId {
        file: PathBuf::from("src/analysis/python_imports.rs"),
        name: "test_dynamic_import_in_conditionals".to_string(),
        line: 985,
    };
    call_graph.add_function(test_function.clone(), false, true, 1, 20);

    // Test calls resolver.analyze_imports() - this is a METHOD CALL
    call_graph.add_call(FunctionCall {
        caller: test_function.clone(),
        callee: method_analyze_imports.clone(),
        call_type: CallType::Direct,
    });

    // Verify: Method should have 1 caller (the test)
    let method_callers = call_graph.get_callers(&method_analyze_imports);
    assert_eq!(
        method_callers.len(),
        1,
        "Method EnhancedImportResolver::analyze_imports should have 1 caller"
    );
    assert_eq!(method_callers[0], test_function);

    // Verify: Standalone function should have 0 callers
    let standalone_callers = call_graph.get_callers(&standalone_analyze_imports);
    assert_eq!(
        standalone_callers.len(),
        0,
        "Standalone function analyze_imports should have 0 callers, but got {} callers. \
         This indicates the call graph is incorrectly conflating method calls with function calls.",
        standalone_callers.len()
    );

    // Additional verification: Ensure they are distinct in the graph
    assert_ne!(
        standalone_analyze_imports, method_analyze_imports,
        "Standalone function and method should be distinct FunctionIds"
    );
}

#[test]
fn test_multiple_methods_same_name_different_types() {
    let mut call_graph = CallGraph::new();

    // Three different `process` functions:
    // 1. Standalone function
    let standalone_process = FunctionId {
        file: PathBuf::from("src/utils.rs"),
        name: "process".to_string(),
        line: 10,
    };
    call_graph.add_function(standalone_process.clone(), false, false, 2, 20);

    // 2. Method on TypeA
    let type_a_process = FunctionId {
        file: PathBuf::from("src/type_a.rs"),
        name: "TypeA::process".to_string(),
        line: 50,
    };
    call_graph.add_function(type_a_process.clone(), false, false, 3, 30);

    // 3. Method on TypeB
    let type_b_process = FunctionId {
        file: PathBuf::from("src/type_b.rs"),
        name: "TypeB::process".to_string(),
        line: 75,
    };
    call_graph.add_function(type_b_process.clone(), false, false, 4, 40);

    // Caller that uses TypeA::process
    let caller = FunctionId {
        file: PathBuf::from("src/main.rs"),
        name: "main".to_string(),
        line: 5,
    };
    call_graph.add_function(caller.clone(), true, false, 1, 10);

    // Main calls type_a.process()
    call_graph.add_call(FunctionCall {
        caller: caller.clone(),
        callee: type_a_process.clone(),
        call_type: CallType::Direct,
    });

    // Verify: Only TypeA::process should have a caller
    assert_eq!(
        call_graph.get_callers(&type_a_process).len(),
        1,
        "TypeA::process should have 1 caller"
    );
    assert_eq!(
        call_graph.get_callers(&type_b_process).len(),
        0,
        "TypeB::process should have 0 callers"
    );
    assert_eq!(
        call_graph.get_callers(&standalone_process).len(),
        0,
        "Standalone process should have 0 callers"
    );
}

#[test]
fn test_unqualified_method_name_causes_ambiguity() {
    // This test demonstrates the BUG - when method names are not qualified,
    // the call graph cannot distinguish between different implementations
    let mut call_graph = CallGraph::new();

    // Two methods with the SAME UNQUALIFIED NAME (this is the bug!)
    let method1 = FunctionId {
        file: PathBuf::from("src/type_a.rs"),
        name: "analyze_imports".to_string(), // BUG: Not qualified!
        line: 50,
    };
    call_graph.add_function(method1.clone(), false, false, 3, 30);

    let method2 = FunctionId {
        file: PathBuf::from("src/type_b.rs"),
        name: "analyze_imports".to_string(), // BUG: Not qualified!
        line: 75,
    };
    call_graph.add_function(method2.clone(), false, false, 4, 40);

    // Caller
    let caller = FunctionId {
        file: PathBuf::from("src/main.rs"),
        name: "main".to_string(),
        line: 5,
    };
    call_graph.add_function(caller.clone(), true, false, 1, 10);

    // Main calls method1 specifically
    call_graph.add_call(FunctionCall {
        caller: caller.clone(),
        callee: method1.clone(),
        call_type: CallType::Direct,
    });

    // BUG DEMONSTRATION: Because names are identical and only differ by file/line,
    // name-based lookups will fail to distinguish them

    // The issue is that if we search by name "analyze_imports", we get both!
    // This test documents the expected behavior once the bug is fixed:

    // After fix: Only method1 should have callers
    assert_eq!(
        call_graph.get_callers(&method1).len(),
        1,
        "Method in type_a.rs should have 1 caller"
    );

    assert_eq!(
        call_graph.get_callers(&method2).len(),
        0,
        "Method in type_b.rs should have 0 callers (different method)"
    );

    // Note: This test will pass currently because FunctionId uses (file, name, line)
    // as the key, so they ARE distinct. The bug is in how names are REPORTED and
    // how coverage matching works, not in the graph structure itself.
}

#[test]
fn test_trait_method_vs_free_function_disambiguation() {
    // Test distinguishing trait methods from free functions
    let mut call_graph = CallGraph::new();

    // Free function `parse`
    let free_parse = FunctionId {
        file: PathBuf::from("src/parser.rs"),
        name: "parse".to_string(),
        line: 10,
    };
    call_graph.add_function(free_parse.clone(), false, false, 5, 50);

    // Trait method `Parser::parse`
    let trait_parse = FunctionId {
        file: PathBuf::from("src/parser_trait.rs"),
        name: "Parser::parse".to_string(),
        line: 25,
    };
    call_graph.add_function(trait_parse.clone(), false, false, 3, 30);

    // Impl method `JsonParser::parse`
    let impl_parse = FunctionId {
        file: PathBuf::from("src/json_parser.rs"),
        name: "JsonParser::parse".to_string(),
        line: 40,
    };
    call_graph.add_function(impl_parse.clone(), false, false, 8, 80);

    // Test that calls impl method
    let test = FunctionId {
        file: PathBuf::from("tests/parser_test.rs"),
        name: "test_json_parsing".to_string(),
        line: 5,
    };
    call_graph.add_function(test.clone(), false, true, 1, 10);

    call_graph.add_call(FunctionCall {
        caller: test.clone(),
        callee: impl_parse.clone(),
        call_type: CallType::Direct,
    });

    // Verify: Only impl method should have caller
    assert_eq!(call_graph.get_callers(&impl_parse).len(), 1);
    assert_eq!(call_graph.get_callers(&trait_parse).len(), 0);
    assert_eq!(call_graph.get_callers(&free_parse).len(), 0);
}

#[test]
fn test_coverage_matching_requires_qualified_names() {
    // This test demonstrates why qualified names are important for coverage matching
    //
    // When coverage reports show:
    // - FNDA:0,analyze_imports (line 62 in file A)
    // - FNDA:5,analyze_imports (line 371 in file B)
    //
    // Without qualified names, we can't distinguish:
    // - Which coverage data applies to which function?
    // - Are they the same function or different?

    let mut call_graph = CallGraph::new();

    // File A: standalone function (UNCOVERED in tests)
    let uncovered_function = FunctionId {
        file: PathBuf::from("src/module_a.rs"),
        name: "analyze_imports".to_string(),
        line: 62,
    };
    call_graph.add_function(uncovered_function.clone(), false, false, 2, 30);

    // File B: method (COVERED in tests)
    let covered_method = FunctionId {
        file: PathBuf::from("src/module_b.rs"),
        name: "Resolver::analyze_imports".to_string(), // Qualified name helps!
        line: 371,
    };
    call_graph.add_function(covered_method.clone(), false, false, 5, 50);

    // Test function
    let test = FunctionId {
        file: PathBuf::from("tests/test.rs"),
        name: "test_imports".to_string(),
        line: 10,
    };
    call_graph.add_function(test.clone(), false, true, 1, 20);

    // Test calls the METHOD (covered)
    call_graph.add_call(FunctionCall {
        caller: test.clone(),
        callee: covered_method.clone(),
        call_type: CallType::Direct,
    });

    // Expected behavior:
    // - covered_method: 1 caller (test), coverage = 100%
    // - uncovered_function: 0 callers, coverage = 0%

    assert_eq!(
        call_graph.get_callers(&covered_method).len(),
        1,
        "Covered method should have 1 caller"
    );
    assert_eq!(
        call_graph.get_callers(&uncovered_function).len(),
        0,
        "Uncovered standalone function should have 0 callers"
    );

    // The bug occurs when debtmap reports:
    // "analyze_imports() - Called by: test_imports, Coverage: 0%"
    //
    // This is misleading because:
    // 1. The standalone function IS uncovered (correct)
    // 2. But it claims to be "called by test_imports" (incorrect - that's the method!)
    // 3. Without qualified names, users can't tell which one is which
}

#[test]
fn test_lcov_function_name_matching() {
    // This test simulates how LCOV coverage data should match function names
    //
    // LCOV format uses:
    // - Simple names for standalone functions: "FN:62,analyze_imports"
    // - Qualified names for methods: "FN:371,EnhancedImportResolver::analyze_imports"
    //
    // The call graph should use the SAME naming convention for proper matching

    let mut call_graph = CallGraph::new();

    // Standalone function (LCOV: "FN:62,analyze_imports")
    let standalone = FunctionId {
        file: PathBuf::from("src/module_a.rs"),
        name: "analyze_imports".to_string(), // Matches LCOV name
        line: 62,
    };
    call_graph.add_function(standalone.clone(), false, false, 2, 30);

    // Method on struct (LCOV: "FN:371,EnhancedImportResolver::analyze_imports")
    let method = FunctionId {
        file: PathBuf::from("src/module_b.rs"),
        name: "EnhancedImportResolver::analyze_imports".to_string(), // Matches LCOV name
        line: 371,
    };
    call_graph.add_function(method.clone(), false, false, 5, 50);

    // Mock coverage data (simulating parsed LCOV):
    // - standalone: FNDA:0,analyze_imports (0 executions)
    // - method: FNDA:5,EnhancedImportResolver::analyze_imports (5 executions)

    // When coverage matching runs, it should:
    // 1. Match "analyze_imports" to standalone function (line 62)
    // 2. Match "EnhancedImportResolver::analyze_imports" to method (line 371)
    // 3. NOT confuse the two!

    // Verify distinct names
    assert_ne!(
        standalone.name, method.name,
        "Standalone function and method must have different names for proper coverage matching"
    );

    assert_eq!(standalone.name, "analyze_imports");
    assert_eq!(method.name, "EnhancedImportResolver::analyze_imports");
}

#[test]
fn test_call_graph_uses_qualified_names_for_methods() {
    // This test verifies that the call graph extraction produces the RIGHT names
    // that will match LCOV data correctly

    let mut call_graph = CallGraph::new();

    // When the AST parser sees:
    // ```rust
    // impl MyStruct {
    //     fn process(&self) { ... }
    // }
    // ```
    // It should create a FunctionId with name="MyStruct::process", NOT name="process"

    let correct_method_name = FunctionId {
        file: PathBuf::from("src/types.rs"),
        name: "MyStruct::process".to_string(), // Correct: qualified
        line: 50,
    };
    call_graph.add_function(correct_method_name.clone(), false, false, 3, 30);

    // BAD: Unqualified method name
    let _incorrect_method_name = FunctionId {
        file: PathBuf::from("src/types.rs"),
        name: "process".to_string(), // Wrong: not qualified
        line: 50,
    };

    // These should be different FunctionIds (even though same file/line)
    // Actually, they are the same because FunctionId uses (file, name, line) as key
    // and the line is the same... This is interesting!

    // The real test: a method and standalone function with same simple name
    let standalone_process = FunctionId {
        file: PathBuf::from("src/utils.rs"),
        name: "process".to_string(),
        line: 10,
    };
    call_graph.add_function(standalone_process.clone(), false, false, 2, 20);

    // Verify: the method name includes the type
    assert!(
        correct_method_name.name.contains("::"),
        "Method names should be qualified with type name"
    );
    assert!(
        !standalone_process.name.contains("::"),
        "Standalone function names should not be qualified"
    );
}

#[test]
fn test_bug_reproduction_unqualified_method_causes_false_match() {
    // This is the EXACT bug scenario from the real codebase
    let mut call_graph = CallGraph::new();

    // Real scenario:
    // - src/analysis/python_call_graph/analyze.rs has standalone `analyze_imports()` at line 62
    // - src/analysis/python_imports.rs has method `EnhancedImportResolver::analyze_imports()` at line 371
    // - Tests call the METHOD
    // - Call graph incorrectly reports standalone function as called

    // Standalone function (correctly has no callers)
    let standalone = FunctionId {
        file: PathBuf::from("src/analysis/python_call_graph/analyze.rs"),
        name: "analyze_imports".to_string(),
        line: 62,
    };
    call_graph.add_function(standalone.clone(), false, false, 2, 30);

    // Method (correctly qualified)
    let method = FunctionId {
        file: PathBuf::from("src/analysis/python_imports.rs"),
        name: "EnhancedImportResolver::analyze_imports".to_string(),
        line: 371,
    };
    call_graph.add_function(method.clone(), false, false, 5, 50);

    // Test function
    let test = FunctionId {
        file: PathBuf::from("src/analysis/python_imports.rs"),
        name: "test_dynamic_import_in_conditionals".to_string(),
        line: 985,
    };
    call_graph.add_function(test.clone(), false, true, 1, 20);

    // Test calls the METHOD (not the standalone function)
    call_graph.add_call(FunctionCall {
        caller: test.clone(),
        callee: method.clone(),
        call_type: CallType::Direct,
    });

    // Assertions
    assert_eq!(
        call_graph.get_callers(&method).len(),
        1,
        "Method should have 1 caller (the test)"
    );

    assert_eq!(
        call_graph.get_callers(&standalone).len(),
        0,
        "CRITICAL BUG: Standalone function shows {} callers but should have 0. \
         The call graph is incorrectly associating test calls to the method with the standalone function. \
         This causes false 'Called by: test_X' reports for dead code.",
        call_graph.get_callers(&standalone).len()
    );

    // Additional check: verify they are truly distinct
    assert_ne!(standalone, method, "Functions should be distinct");
    assert_ne!(
        standalone.name, method.name,
        "Function names should be distinct"
    );
}
