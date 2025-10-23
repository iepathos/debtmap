use debtmap::analyzers::rust_call_graph::extract_call_graph;
use debtmap::priority::call_graph::FunctionId;
use std::path::PathBuf;

#[test]
fn test_basic_function_calls() {
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

    // Check that functions are detected
    let functions = call_graph.find_all_functions();
    assert_eq!(functions.len(), 4, "Should find 4 functions");

    // Check main's callees
    let main_id = FunctionId::new(path.clone(), "main".to_string(), 2);
    let main_callees = call_graph.get_callees(&main_id);
    assert_eq!(main_callees.len(), 2, "main should call 2 functions");

    // Check helper's callers
    let helper_id = FunctionId::new(path.clone(), "helper".to_string(), 7);
    let helper_callers = call_graph.get_callers(&helper_id);
    assert_eq!(helper_callers.len(), 1, "helper should have 1 caller");
    assert_eq!(
        helper_callers[0].name, "main",
        "helper should be called by main"
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
    let process_id = functions
        .iter()
        .find(|f| f.name == "Processor::process")
        .expect("Should find process method");
    let process_callees = call_graph.get_callees(process_id);
    assert_eq!(process_callees.len(), 2, "process should call 2 methods");
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

    let handle_callees = call_graph.get_callees(handle_fn);
    assert_eq!(handle_callees.len(), 3, "handle should call 3 functions");
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
    let functions = call_graph.find_all_functions();
    let calculate_fn = functions
        .iter()
        .find(|f| f.name.contains("calculate"))
        .expect("Should find calculate function");

    let calculate_callees = call_graph.get_callees(calculate_fn);
    assert_eq!(
        calculate_callees.len(),
        2,
        "calculate should call 2 functions"
    );
}

#[test]
fn test_closure_and_higher_order_functions() {
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

    // Check that closure calls are detected
    let process_id = FunctionId::new(path.clone(), "process_items".to_string(), 2);
    let process_callees = call_graph.get_callees(&process_id);
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
    let main_id = FunctionId::new(path.clone(), "main".to_string(), 2);
    let main_callees = call_graph.get_callees(&main_id);
    assert_eq!(main_callees.len(), 2, "main should call 2 async functions");
}

#[test]
fn test_macro_generated_calls() {
    let code = r#"
macro_rules! call_helper {
    () => {
        helper()
    };
}

fn main() {
    call_helper!();
    direct_call();
}

fn helper() {
    // helper
}

fn direct_call() {
    // direct
}
"#;

    let parsed = syn::parse_file(code).unwrap();
    let path = PathBuf::from("test.rs");
    let call_graph = extract_call_graph(&parsed, &path);

    // Macro calls might not be detected - this is a known limitation
    let main_id = FunctionId::new(path.clone(), "main".to_string(), 8);
    let main_callees = call_graph.get_callees(&main_id);
    // Should at least detect direct_call
    assert!(
        main_callees.iter().any(|f| f.name == "direct_call"),
        "Should detect direct_call"
    );
}

#[test]
fn test_qualified_path_calls() {
    let code = r#"
mod utils {
    pub fn helper() {
        // helper
    }
}

fn main() {
    utils::helper();
    crate::process();
    super::validate();
}

fn process() {
    // process
}

fn validate() {
    // validate
}
"#;

    let parsed = syn::parse_file(code).unwrap();
    let path = PathBuf::from("test.rs");
    let call_graph = extract_call_graph(&parsed, &path);

    // Check qualified path calls
    let main_id = FunctionId::new(path.clone(), "main".to_string(), 8);
    let main_callees = call_graph.get_callees(&main_id);
    assert!(
        !main_callees.is_empty(),
        "Should detect at least some qualified calls"
    );
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
    let process_id = FunctionId::new(path.clone(), "process".to_string(), 2);
    let process_callees = call_graph.get_callees(&process_id);
    assert_eq!(
        process_callees.len(),
        2,
        "process should call validate and transform"
    );
}

#[test]
fn test_function_pointer_and_fn_traits() {
    let code = r#"
fn apply_operation(value: i32, op: fn(i32) -> i32) -> i32 {
    op(value)
}

fn double(x: i32) -> i32 {
    x * 2
}

fn main() {
    apply_operation(21, double);
    apply_operation(10, |x| x + 1);
}
"#;

    let parsed = syn::parse_file(code).unwrap();
    let path = PathBuf::from("test.rs");
    let call_graph = extract_call_graph(&parsed, &path);

    // Function pointers are hard to track statically
    let main_id = FunctionId::new(path.clone(), "main".to_string(), 10);
    let main_callees = call_graph.get_callees(&main_id);
    assert!(
        main_callees.iter().any(|f| f.name == "apply_operation"),
        "Should detect apply_operation call"
    );
    // Note: detecting that 'double' is passed as a function pointer is challenging
}
