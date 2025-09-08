use debtmap::analyzers::rust_call_graph::extract_call_graph;
use std::path::PathBuf;

#[test]
fn test_struct_literal_field_function_call() {
    // Test that function calls within struct literal fields are detected
    let code = r#"
struct Config {
    value: String,
}

fn create_value() -> String {
    "test".to_string()
}

fn main() {
    let config = Config {
        value: create_value(),
    };
}
    "#;

    let syntax = syn::parse_file(code).expect("Failed to parse code");
    let path = PathBuf::from("test.rs");
    let call_graph = extract_call_graph(&syntax, &path);

    // Find function IDs
    let all_functions = call_graph.find_all_functions();
    let main_fn = all_functions
        .iter()
        .find(|f| f.name == "main")
        .expect("main function should exist");
    let create_value_fn = all_functions
        .iter()
        .find(|f| f.name == "create_value")
        .expect("create_value function should exist");

    // Check that main calls create_value
    let calls_from_main = call_graph.get_callees(main_fn);
    assert!(
        calls_from_main.contains(create_value_fn),
        "main should call create_value through struct literal field"
    );
}

#[test]
fn test_nested_struct_literal_calls() {
    // Test that nested struct literals with function calls are handled
    let code = r#"
struct Inner {
    data: i32,
}

struct Outer {
    inner: Inner,
}

fn get_data() -> i32 {
    42
}

fn build_structure() -> Outer {
    Outer {
        inner: Inner {
            data: get_data(),
        },
    }
}
    "#;

    let syntax = syn::parse_file(code).expect("Failed to parse code");
    let path = PathBuf::from("test.rs");
    let call_graph = extract_call_graph(&syntax, &path);

    // Find function IDs
    let all_functions = call_graph.find_all_functions();
    let build_fn = all_functions
        .iter()
        .find(|f| f.name == "build_structure")
        .expect("build_structure function should exist");
    let get_data_fn = all_functions
        .iter()
        .find(|f| f.name == "get_data")
        .expect("get_data function should exist");

    // Check that build_structure calls get_data
    let calls_from_build = call_graph.get_callees(build_fn);
    assert!(
        calls_from_build.contains(get_data_fn),
        "build_structure should call get_data through nested struct literal"
    );
}

#[test]
fn test_struct_literal_in_vec_macro() {
    // Test that struct literals inside vec! macro with function calls are detected
    let code = r#"
struct Item {
    name: String,
    value: i32,
}

fn generate_name() -> String {
    "item".to_string()
}

fn compute_value() -> i32 {
    100
}

fn create_items() -> Vec<Item> {
    vec![Item {
        name: generate_name(),
        value: compute_value(),
    }]
}
    "#;

    let syntax = syn::parse_file(code).expect("Failed to parse code");
    let path = PathBuf::from("test.rs");
    let call_graph = extract_call_graph(&syntax, &path);

    // Find function IDs
    let all_functions = call_graph.find_all_functions();
    let create_items_fn = all_functions
        .iter()
        .find(|f| f.name == "create_items")
        .expect("create_items function should exist");
    let generate_name_fn = all_functions
        .iter()
        .find(|f| f.name == "generate_name")
        .expect("generate_name function should exist");
    let compute_value_fn = all_functions
        .iter()
        .find(|f| f.name == "compute_value")
        .expect("compute_value function should exist");

    // Check that create_items calls both helper functions
    let calls_from_create = call_graph.get_callees(create_items_fn);
    assert!(
        calls_from_create.contains(generate_name_fn),
        "create_items should call generate_name through vec! macro struct literal"
    );
    assert!(
        calls_from_create.contains(compute_value_fn),
        "create_items should call compute_value through vec! macro struct literal"
    );
}

#[test]
fn test_method_call_with_struct_literal_receiver() {
    // Test that method calls on struct literal instances are detected correctly
    let code = r#"
struct Calculator {
    value: i32,
}

impl Calculator {
    fn compute(&self) -> i32 {
        self.value * 2
    }
}

fn get_initial_value() -> i32 {
    10
}

fn calculate() -> i32 {
    let calc = Calculator {
        value: get_initial_value(),
    };
    calc.compute()
}
    "#;

    let syntax = syn::parse_file(code).expect("Failed to parse code");
    let path = PathBuf::from("test.rs");
    let call_graph = extract_call_graph(&syntax, &path);

    // Find function IDs
    let all_functions = call_graph.find_all_functions();
    let calculate_fn = all_functions
        .iter()
        .find(|f| f.name == "calculate")
        .expect("calculate function should exist");
    let get_initial_fn = all_functions
        .iter()
        .find(|f| f.name == "get_initial_value")
        .expect("get_initial_value function should exist");
    let compute_method = all_functions
        .iter()
        .find(|f| f.name == "Calculator::compute")
        .expect("Calculator::compute method should exist");

    // Check that calculate calls both functions
    let calls_from_calculate = call_graph.get_callees(calculate_fn);
    assert!(
        calls_from_calculate.contains(get_initial_fn),
        "calculate should call get_initial_value through struct literal"
    );
    assert!(
        calls_from_calculate.contains(compute_method),
        "calculate should call Calculator::compute method"
    );
}
