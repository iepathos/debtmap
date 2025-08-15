use debtmap::analyzers::rust_call_graph::extract_call_graph;
use std::path::PathBuf;

#[test]
fn test_type_tracking_struct_literal() {
    let rust_code = r#"
pub struct DependencyGraph {
    modules: Vec<String>,
}

impl DependencyGraph {
    pub fn calculate_coupling_metrics(&self) -> Vec<String> {
        self.modules.clone()
    }
}

pub fn analyze_dependencies() -> Vec<String> {
    let dep_graph = DependencyGraph { modules: vec![] };
    dep_graph.calculate_coupling_metrics()
}
"#;

    let path = PathBuf::from("test.rs");
    let syntax_tree = syn::parse_file(rust_code).unwrap();
    let call_graph = extract_call_graph(&syntax_tree, &path);

    // Check that the method call was resolved correctly
    let functions = call_graph.find_all_functions();
    let method = functions
        .iter()
        .find(|f| f.name == "DependencyGraph::calculate_coupling_metrics")
        .expect("Should find method");

    let callers = call_graph.get_callers(method);
    assert_eq!(callers.len(), 1, "Method should have one caller");
    assert_eq!(callers[0].name, "analyze_dependencies");
}

#[test]
fn test_type_tracking_constructor() {
    let rust_code = r#"
pub struct Calculator {
    value: i32,
}

impl Calculator {
    pub fn new(value: i32) -> Self {
        Self { value }
    }
    
    pub fn calculate(&self) -> i32 {
        self.value * 2
    }
}

pub fn use_calculator() -> i32 {
    let calc = Calculator::new(10);
    calc.calculate()
}
"#;

    let path = PathBuf::from("test.rs");
    let syntax_tree = syn::parse_file(rust_code).unwrap();
    let call_graph = extract_call_graph(&syntax_tree, &path);

    // Check that both the constructor and method calls were resolved
    let functions = call_graph.find_all_functions();

    let new_method = functions
        .iter()
        .find(|f| f.name == "Calculator::new")
        .expect("Should find new method");
    let new_callers = call_graph.get_callers(new_method);
    assert_eq!(new_callers.len(), 1, "new() should have one caller");
    assert_eq!(new_callers[0].name, "use_calculator");

    let calc_method = functions
        .iter()
        .find(|f| f.name == "Calculator::calculate")
        .expect("Should find calculate method");
    let calc_callers = call_graph.get_callers(calc_method);
    assert_eq!(calc_callers.len(), 1, "calculate() should have one caller");
    assert_eq!(calc_callers[0].name, "use_calculator");
}
