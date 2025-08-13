use debtmap::analyzers::rust::extract_rust_call_graph_with_expansion;
use debtmap::core::ast::RustAst;
use debtmap::expansion::ExpansionConfig;
use std::path::PathBuf;

fn create_test_ast(code: &str, path: &str) -> RustAst {
    let file = syn::parse_str::<syn::File>(code).expect("Failed to parse test code");
    RustAst {
        file,
        path: PathBuf::from(path),
    }
}

#[test]
fn test_extract_call_graph_with_expansion_disabled() {
    // Test with expansion disabled - should use regular analysis
    let code = r#"
        fn main() {
            helper();
        }
        
        fn helper() {
            println!("Hello");
        }
    "#;

    let ast = create_test_ast(code, "test_disabled.rs");
    let config = ExpansionConfig {
        enabled: false,
        ..ExpansionConfig::default()
    };

    let call_graph = extract_rust_call_graph_with_expansion(&ast, &config);

    // Should have functions in the call graph
    assert!(!call_graph.is_empty(), "Call graph should have functions");

    let functions = call_graph.find_all_functions();

    // Should find the main function
    let main_func = functions.iter().find(|f| f.name == "main");
    assert!(main_func.is_some(), "Should find main function");

    // Should find the helper function
    let helper_func = functions.iter().find(|f| f.name == "helper");
    assert!(helper_func.is_some(), "Should find helper function");
}

#[test]
fn test_extract_call_graph_with_expansion_fallback() {
    // Test fallback behavior when expansion fails
    let code = r#"
        macro_rules! call_func {
            ($func:ident) => {
                $func()
            };
        }
        
        fn main() {
            call_func!(process);
        }
        
        fn process() {
            println!("Processing");
        }
    "#;

    let ast = create_test_ast(code, "test_fallback.rs");
    let config = ExpansionConfig {
        enabled: true,
        fallback_on_error: true,
        // Use a non-existent file path to ensure expansion fails
        cache_dir: PathBuf::from("/nonexistent/cache/dir"),
        ..ExpansionConfig::default()
    };

    let call_graph = extract_rust_call_graph_with_expansion(&ast, &config);

    // Should still produce a call graph via fallback
    assert!(
        !call_graph.is_empty(),
        "Should fallback to regular analysis"
    );

    let functions = call_graph.find_all_functions();

    // Should find main and process functions
    assert!(
        functions.iter().any(|f| f.name == "main"),
        "Should find main"
    );
    assert!(
        functions.iter().any(|f| f.name == "process"),
        "Should find process"
    );
}

#[test]
fn test_extract_call_graph_with_empty_file() {
    // Test with an empty file
    let code = "";
    let ast = create_test_ast(code, "empty.rs");
    let config = ExpansionConfig::default();

    let call_graph = extract_rust_call_graph_with_expansion(&ast, &config);

    // Should produce an empty call graph
    assert!(
        call_graph.is_empty(),
        "Empty file should produce empty call graph"
    );
}

#[test]
fn test_extract_call_graph_with_complex_structure() {
    // Test with methods, traits, and nested calls
    let code = r#"
        struct Calculator;
        
        impl Calculator {
            fn add(&self, a: i32, b: i32) -> i32 {
                self.validate(a);
                self.validate(b);
                a + b
            }
            
            fn validate(&self, val: i32) {
                if val < 0 {
                    panic!("Negative value");
                }
            }
        }
        
        fn main() {
            let calc = Calculator;
            calc.add(1, 2);
        }
    "#;

    let ast = create_test_ast(code, "complex.rs");
    let config = ExpansionConfig {
        enabled: false, // Use regular analysis for predictable results
        ..ExpansionConfig::default()
    };

    let call_graph = extract_rust_call_graph_with_expansion(&ast, &config);

    assert!(!call_graph.is_empty(), "Should have functions for methods");

    let functions = call_graph.find_all_functions();

    // Should find the add method
    assert!(
        functions.iter().any(|f| f.name == "add"),
        "Should find add method"
    );

    // Should find the validate method
    assert!(
        functions.iter().any(|f| f.name == "validate"),
        "Should find validate method"
    );

    // Should find main
    assert!(
        functions.iter().any(|f| f.name == "main"),
        "Should find main function"
    );
}

#[test]
fn test_extract_call_graph_with_async_functions() {
    // Test with async/await code
    let code = r#"
        async fn fetch_data() -> String {
            process_async().await
        }
        
        async fn process_async() -> String {
            "data".to_string()
        }
        
        #[tokio::main]
        async fn main() {
            let result = fetch_data().await;
            println!("{}", result);
        }
    "#;

    let ast = create_test_ast(code, "async_test.rs");
    let config = ExpansionConfig {
        enabled: false, // Use regular analysis
        ..ExpansionConfig::default()
    };

    let call_graph = extract_rust_call_graph_with_expansion(&ast, &config);

    assert!(!call_graph.is_empty(), "Should handle async functions");

    let functions = call_graph.find_all_functions();

    // Should find all async functions
    assert!(
        functions.iter().any(|f| f.name == "fetch_data"),
        "Should find fetch_data"
    );
    assert!(
        functions.iter().any(|f| f.name == "process_async"),
        "Should find process_async"
    );
    assert!(
        functions.iter().any(|f| f.name == "main"),
        "Should find async main"
    );

    // Check for async call relationships
    if let Some(fetch_func) = functions.iter().find(|f| f.name == "fetch_data") {
        let callees = call_graph.get_callees(fetch_func);
        assert!(
            callees.iter().any(|c| c.name == "process_async"),
            "fetch_data should call process_async"
        );
    }
}
