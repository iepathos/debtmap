/// Integration test that reproduces the bug with Type::new() constructor pattern
///
/// Bug: When a method call like `EnhancedImportResolver::new()` returns Self,
/// and then a method is called on the result like `resolver.analyze_imports()`,
/// the call graph incorrectly attributes it to ANY function named `analyze_imports`,
/// including standalone functions with the same simple name.
///
/// This is the EXACT pattern from the real codebase that's still failing.
use debtmap::analyzers::rust_call_graph::extract_call_graph;
use debtmap::priority::call_graph::FunctionId;
use std::path::PathBuf;

#[test]
fn test_new_constructor_method_disambiguation() {
    // This reproduces the EXACT bug pattern from python_imports.rs:
    // - Standalone function: `fn analyze_imports()` (dead code)
    // - Struct with method: `impl EnhancedImportResolver { fn analyze_imports() }`
    // - Test calls: `let mut resolver = EnhancedImportResolver::new(); resolver.analyze_imports();`
    // - Bug: Call graph says standalone function is called!

    let code = r#"
// File 1: Standalone function (DEAD CODE)
pub fn analyze_imports() {
    println!("standalone - never called");
}

// File 2: Struct with methods
pub struct EnhancedImportResolver {
    data: String,
}

impl EnhancedImportResolver {
    pub fn new() -> Self {
        EnhancedImportResolver { data: String::new() }
    }

    pub fn analyze_imports(&mut self) {
        println!("method - actually called");
    }
}

// File 3: Test that uses the TYPE::new() pattern
#[cfg(test)]
mod tests {
    use super::*;

    fn test_direct_import() {
        let mut resolver = EnhancedImportResolver::new();
        resolver.analyze_imports();  // Should call the METHOD, NOT standalone function
    }
}
"#;

    let parsed = syn::parse_file(code).unwrap();
    let path = PathBuf::from("test.rs");
    let call_graph = extract_call_graph(&parsed, &path);

    // Find the standalone function
    let standalone_id = FunctionId::new(path.clone(), "analyze_imports".to_string(), 3);

    // Find the method
    let method_id = FunctionId::new(
        path.clone(),
        "EnhancedImportResolver::analyze_imports".to_string(),
        17,
    );

    // Find the test
    let test_id = FunctionId::new(path.clone(), "tests::test_direct_import".to_string(), 27);

    println!("\n=== All functions found ===");
    for func in call_graph.find_all_functions() {
        let callers = call_graph.get_callers(&func);
        println!(
            "  {} (line {}) - {} callers",
            func.name,
            func.line,
            callers.len()
        );
        for caller in callers {
            println!("    <- {}", caller.name);
        }
    }

    // THE BUG: Standalone function incorrectly shows callers
    let standalone_callers = call_graph.get_callers(&standalone_id);
    assert_eq!(
        standalone_callers.len(),
        0,
        "BUG REPRODUCED! Standalone `analyze_imports()` at line {} shows {} caller(s): {:?}, but should have 0! \
         The test calls `resolver.analyze_imports()` where resolver is created via `EnhancedImportResolver::new()`, \
         so it should ONLY call the METHOD, not the standalone function.",
        standalone_id.line,
        standalone_callers.len(),
        standalone_callers.iter().map(|c| format!("`{}` at line {}", c.name, c.line)).collect::<Vec<_>>()
    );

    // THE FIX: Method correctly shows callers
    let method_callers = call_graph.get_callers(&method_id);
    assert_eq!(
        method_callers.len(),
        1,
        "Method `EnhancedImportResolver::analyze_imports()` should have 1 caller (the test), but has {}",
        method_callers.len()
    );

    if !method_callers.is_empty() {
        assert_eq!(
            method_callers[0].name, "tests::test_direct_import",
            "The test should be calling the method"
        );
    }

    // Verify test calls the method, not the standalone function
    let test_callees = call_graph.get_callees(&test_id);

    println!("\n=== Test callees ===");
    for callee in &test_callees {
        println!("  -> {} at line {}", callee.name, callee.line);
    }

    assert!(
        test_callees
            .iter()
            .any(|c| c.name == "EnhancedImportResolver::analyze_imports"),
        "Test should call `EnhancedImportResolver::analyze_imports` (the method). Called: {:?}",
        test_callees.iter().map(|c| &c.name).collect::<Vec<_>>()
    );

    assert!(
        !test_callees
            .iter()
            .any(|c| c.name == "analyze_imports" && c.line == 3),
        "Test should NOT call standalone `analyze_imports()` function at line 3"
    );
}

#[test]
fn test_builder_pattern_method_disambiguation() {
    // Another common pattern: builder with method chaining
    let code = r#"
pub fn build() {
    println!("standalone - never called");
}

pub struct Builder {
    value: i32,
}

impl Builder {
    pub fn new() -> Self {
        Builder { value: 0 }
    }

    pub fn build(&self) -> i32 {
        self.value
    }
}

fn use_builder() {
    let builder = Builder::new();
    let result = builder.build();  // Should call METHOD, not standalone function
}
"#;

    let parsed = syn::parse_file(code).unwrap();
    let path = PathBuf::from("builder.rs");
    let call_graph = extract_call_graph(&parsed, &path);

    let standalone = FunctionId::new(path.clone(), "build".to_string(), 2);

    let method = FunctionId::new(path.clone(), "Builder::build".to_string(), 15);

    println!("\n=== Builder pattern test ===");
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
        "Standalone `build()` should have 0 callers, but has {}: {:?}",
        standalone_callers.len(),
        standalone_callers
            .iter()
            .map(|c| &c.name)
            .collect::<Vec<_>>()
    );

    assert_eq!(
        method_callers.len(),
        1,
        "Method `Builder::build()` should have 1 caller"
    );
}
