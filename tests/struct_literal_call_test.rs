use debtmap::analyzers::rust_call_graph::extract_call_graph;
use std::path::PathBuf;
use syn;

#[test]
fn test_function_calls_in_struct_literals_are_tracked() {
    let code = r#"
        struct ComplexityRefactoring {
            suggested_functions: Vec<String>,
        }

        fn analyze_complexity() -> ComplexityRefactoring {
            ComplexityRefactoring {
                suggested_functions: generate_suggested_functions("base", 3),
            }
        }

        fn generate_suggested_functions(base_name: &str, count: u32) -> Vec<String> {
            vec![format!("{}_validate", base_name)]
        }
    "#;

    let syntax = syn::parse_file(code).expect("Failed to parse code");
    let path = PathBuf::from("test.rs");
    let call_graph = extract_call_graph(&syntax, &path);

    // Debug: print all functions in the graph
    let all_functions = call_graph.find_all_functions();
    println!("Functions in graph:");
    for func in &all_functions {
        println!("  {:?}", func);
    }

    // Find the actual function IDs from the graph
    let analyze_id = all_functions
        .iter()
        .find(|f| f.name == "analyze_complexity")
        .expect("analyze_complexity should be in the graph")
        .clone();

    let generate_id = all_functions
        .iter()
        .find(|f| f.name == "generate_suggested_functions")
        .expect("generate_suggested_functions should be in the graph")
        .clone();

    // Check that analyze_complexity calls generate_suggested_functions
    let calls_from_analyze = call_graph.get_callees(&analyze_id);
    assert!(
        calls_from_analyze.contains(&generate_id),
        "analyze_complexity should call generate_suggested_functions, but calls were: {:?}",
        calls_from_analyze
    );

    // Check that generate_suggested_functions has callers
    let callers_of_generate = call_graph.get_callers(&generate_id);
    assert!(
        !callers_of_generate.is_empty(),
        "generate_suggested_functions should have callers"
    );
    assert!(
        callers_of_generate.contains(&analyze_id),
        "generate_suggested_functions should be called by analyze_complexity"
    );
}

#[test]
fn test_nested_struct_literal_calls() {
    let code = r#"
        struct Inner {
            value: String,
        }
        
        struct Outer {
            inner: Inner,
            computed: i32,
        }

        fn create_outer() -> Outer {
            Outer {
                inner: Inner {
                    value: format_value("test"),
                },
                computed: calculate_value(42),
            }
        }

        fn format_value(s: &str) -> String {
            s.to_string()
        }
        
        fn calculate_value(n: i32) -> i32 {
            n * 2
        }
    "#;

    let syntax = syn::parse_file(code).expect("Failed to parse code");
    let path = PathBuf::from("test.rs");
    let call_graph = extract_call_graph(&syntax, &path);

    // Find the actual function IDs from the graph
    let all_functions = call_graph.find_all_functions();

    let create_id = all_functions
        .iter()
        .find(|f| f.name == "create_outer")
        .expect("create_outer should be in the graph")
        .clone();

    let format_id = all_functions
        .iter()
        .find(|f| f.name == "format_value")
        .expect("format_value should be in the graph")
        .clone();

    let calculate_id = all_functions
        .iter()
        .find(|f| f.name == "calculate_value")
        .expect("calculate_value should be in the graph")
        .clone();

    // Check that create_outer calls both helper functions
    let calls_from_create = call_graph.get_callees(&create_id);
    assert!(
        calls_from_create.contains(&format_id),
        "create_outer should call format_value"
    );
    assert!(
        calls_from_create.contains(&calculate_id),
        "create_outer should call calculate_value"
    );

    // Check that both functions have callers
    assert!(
        !call_graph.get_callers(&format_id).is_empty(),
        "format_value should have callers"
    );
    assert!(
        !call_graph.get_callers(&calculate_id).is_empty(),
        "calculate_value should have callers"
    );
}
