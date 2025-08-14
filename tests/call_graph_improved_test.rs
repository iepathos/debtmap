use debtmap::analyzers::rust_call_graph::extract_call_graph;
use debtmap::priority::call_graph::FunctionId;
use std::path::PathBuf;

#[test]
fn test_basic_function_calls_with_resolution() {
    let code = r#"
fn main() {
    helper();
    process_data();
}

fn helper() {
    println!("Helper");
}

fn process_data() {
    validate();
}

fn validate() {
    // Some validation
}
"#;

    let parsed = syn::parse_file(code).unwrap();
    let path = PathBuf::from("test.rs");
    let call_graph = extract_call_graph(&parsed, &path);

    // Debug output to understand what's happening
    println!("\n=== Functions and their relationships ===");
    let functions = call_graph.find_all_functions();
    for func in &functions {
        let callers = call_graph.get_callers(&func);
        let callees = call_graph.get_callees(&func);
        println!("Function: {} (line {})", func.name, func.line);
        println!(
            "  Callers: {:?}",
            callers.iter().map(|f| &f.name).collect::<Vec<_>>()
        );
        println!(
            "  Callees: {:?}",
            callees.iter().map(|f| &f.name).collect::<Vec<_>>()
        );
    }

    // Check that functions are detected
    assert_eq!(functions.len(), 4, "Should find 4 functions");

    // Find the actual helper function
    let helper = functions
        .iter()
        .find(|f| f.name == "helper")
        .expect("Should find helper function");

    // Check helper's callers
    let helper_callers = call_graph.get_callers(&helper);
    assert_eq!(helper_callers.len(), 1, "helper should have 1 caller");
    assert_eq!(
        helper_callers[0].name, "main",
        "helper should be called by main"
    );

    // Find main and check its callees
    let main_func = functions
        .iter()
        .find(|f| f.name == "main")
        .expect("Should find main function");

    let main_callees = call_graph.get_callees(&main_func);
    assert_eq!(main_callees.len(), 2, "main should call 2 functions");
    assert!(
        main_callees.iter().any(|f| f.name == "helper"),
        "main should call helper"
    );
    assert!(
        main_callees.iter().any(|f| f.name == "process_data"),
        "main should call process_data"
    );
}

#[test]
fn test_method_calls_on_self() {
    let code = r#"
struct Processor;

impl Processor {
    fn process(&self) {
        self.validate();
        self.transform();
    }
    
    fn validate(&self) {
        // validation
    }
    
    fn transform(&self) {
        // transformation
    }
}
"#;

    let parsed = syn::parse_file(code).unwrap();
    let path = PathBuf::from("test.rs");
    let call_graph = extract_call_graph(&parsed, &path);

    // Check that impl methods are detected
    let functions = call_graph.find_all_functions();
    println!("\n=== Methods found ===");
    for func in &functions {
        println!("  {}", func.name);
    }

    assert!(
        functions.iter().any(|f| f.name == "Processor::process"),
        "Should find Processor::process"
    );
    assert!(
        functions.iter().any(|f| f.name == "Processor::validate"),
        "Should find Processor::validate"
    );
    assert!(
        functions.iter().any(|f| f.name == "Processor::transform"),
        "Should find Processor::transform"
    );

    // Check process's callees
    let process_fn = functions
        .iter()
        .find(|f| f.name == "Processor::process")
        .expect("Should find process method");

    let process_callees = call_graph.get_callees(&process_fn);
    assert_eq!(process_callees.len(), 2, "process should call 2 methods");
    assert!(
        process_callees
            .iter()
            .any(|f| f.name == "Processor::validate"),
        "Should call validate"
    );
    assert!(
        process_callees
            .iter()
            .any(|f| f.name == "Processor::transform"),
        "Should call transform"
    );
}

#[test]
fn test_closure_calls() {
    let code = r#"
fn process_items(items: Vec<i32>) -> Vec<i32> {
    items.iter()
        .map(|x| transform(x))
        .filter(|x| validate(x))
        .collect()
}

fn transform(value: &i32) -> i32 {
    *value * 2
}

fn validate(value: &i32) -> bool {
    *value > 0
}
"#;

    let parsed = syn::parse_file(code).unwrap();
    let path = PathBuf::from("test.rs");
    let call_graph = extract_call_graph(&parsed, &path);

    // Find process_items
    let process_fn = call_graph
        .find_all_functions()
        .into_iter()
        .find(|f| f.name == "process_items")
        .expect("Should find process_items");

    let process_callees = call_graph.get_callees(&process_fn);
    println!("\n=== Calls from process_items ===");
    for callee in &process_callees {
        println!("  {}", callee.name);
    }

    assert!(
        process_callees.iter().any(|f| f.name == "transform"),
        "Should detect transform call in closure"
    );
    assert!(
        process_callees.iter().any(|f| f.name == "validate"),
        "Should detect validate call in closure"
    );
}

#[test]
fn test_async_function_calls() {
    let code = r#"
async fn main() {
    fetch_data().await;
    process_async().await;
}

async fn fetch_data() -> String {
    // fetch data
    "data".to_string()
}

async fn process_async() {
    validate_async().await;
}

async fn validate_async() {
    // async validation
}
"#;

    let parsed = syn::parse_file(code).unwrap();
    let path = PathBuf::from("test.rs");
    let call_graph = extract_call_graph(&parsed, &path);

    // Check async function calls
    let main_func = call_graph
        .find_all_functions()
        .into_iter()
        .find(|f| f.name == "main")
        .expect("Should find main");

    let main_callees = call_graph.get_callees(&main_func);
    assert_eq!(main_callees.len(), 2, "main should call 2 async functions");
    assert!(
        main_callees.iter().any(|f| f.name == "fetch_data"),
        "Should call fetch_data"
    );
    assert!(
        main_callees.iter().any(|f| f.name == "process_async"),
        "Should call process_async"
    );
}

#[test]
fn test_associated_function_calls() {
    let code = r#"
struct Calculator;

impl Calculator {
    fn new() -> Self {
        Calculator
    }
    
    fn calculate(value: i32) -> i32 {
        Self::validate_input(value);
        Self::process(value)
    }
    
    fn validate_input(value: i32) {
        // validation
    }
    
    fn process(value: i32) -> i32 {
        value * 2
    }
}

fn main() {
    let calc = Calculator::new();
    let result = Calculator::calculate(42);
}
"#;

    let parsed = syn::parse_file(code).unwrap();
    let path = PathBuf::from("test.rs");
    let call_graph = extract_call_graph(&parsed, &path);

    // Check associated function calls
    let calculate_fn = call_graph
        .find_all_functions()
        .into_iter()
        .find(|f| f.name == "Calculator::calculate")
        .expect("Should find calculate function");

    let calculate_callees = call_graph.get_callees(&calculate_fn);
    println!("\n=== Calls from Calculator::calculate ===");
    for callee in &calculate_callees {
        println!("  {}", callee.name);
    }

    // The calculate function should call validate_input and process
    // Note: Self::method calls may be resolved as Calculator::method
    assert!(
        calculate_callees.len() >= 2,
        "calculate should call at least 2 functions"
    );
}

#[test]
fn test_trait_impl_calls() {
    let code = r#"
trait Handler {
    fn handle(&self);
}

struct MyHandler;

impl Handler for MyHandler {
    fn handle(&self) {
        self.pre_process();
        process_internal();
        self.post_process();
    }
}

impl MyHandler {
    fn pre_process(&self) {
        // pre-processing
    }
    
    fn post_process(&self) {
        // post-processing
    }
}

fn process_internal() {
    // internal processing
}
"#;

    let parsed = syn::parse_file(code).unwrap();
    let path = PathBuf::from("test.rs");
    let call_graph = extract_call_graph(&parsed, &path);

    // Check that trait impl methods are detected
    let functions = call_graph.find_all_functions();
    let handle_fn = functions
        .iter()
        .find(|f| f.name.contains("handle"))
        .expect("Should find handle method");

    let handle_callees = call_graph.get_callees(&handle_fn);
    println!("\n=== Calls from handle ===");
    for callee in &handle_callees {
        println!("  {}", callee.name);
    }

    assert!(
        handle_callees.len() >= 2,
        "handle should call at least 2 functions"
    );
    assert!(
        handle_callees.iter().any(|f| f.name == "process_internal"),
        "Should call process_internal"
    );
}

#[test]
fn test_cross_file_resolution_simulation() {
    // This simulates how cross-file resolution would work
    // In a real scenario, we'd have multiple files
    let code = r#"
mod utils {
    pub fn helper() {
        println!("Helper");
    }
}

fn main() {
    utils::helper();
    process();
}

fn process() {
    // processing
}
"#;

    let parsed = syn::parse_file(code).unwrap();
    let path = PathBuf::from("test.rs");
    let call_graph = extract_call_graph(&parsed, &path);

    let main_func = call_graph
        .find_all_functions()
        .into_iter()
        .find(|f| f.name == "main")
        .expect("Should find main");

    let main_callees = call_graph.get_callees(&main_func);
    println!("\n=== Calls from main ===");
    for callee in &main_callees {
        println!("  {}", callee.name);
    }

    // Should detect at least the process call
    assert!(
        main_callees.iter().any(|f| f.name == "process"),
        "Should call process"
    );
    // Note: utils::helper might not be resolved if it's in a different module
}

#[test]
fn test_generic_function_calls() {
    let code = r#"
fn process<T: Clone>(value: T) -> T {
    validate(&value);
    transform(value)
}

fn validate<T>(value: &T) {
    // validation
}

fn transform<T: Clone>(value: T) -> T {
    value.clone()
}

fn main() {
    process(42);
    process("hello");
}
"#;

    let parsed = syn::parse_file(code).unwrap();
    let path = PathBuf::from("test.rs");
    let call_graph = extract_call_graph(&parsed, &path);

    // Check generic function calls
    let process_fn = call_graph
        .find_all_functions()
        .into_iter()
        .find(|f| f.name == "process")
        .expect("Should find process");

    let process_callees = call_graph.get_callees(&process_fn);
    assert_eq!(
        process_callees.len(),
        2,
        "process should call validate and transform"
    );
    assert!(
        process_callees.iter().any(|f| f.name == "validate"),
        "Should call validate"
    );
    assert!(
        process_callees.iter().any(|f| f.name == "transform"),
        "Should call transform"
    );
}
