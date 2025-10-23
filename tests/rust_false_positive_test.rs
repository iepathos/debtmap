use debtmap::analyzers::rust::RustAnalyzer;
use debtmap::analyzers::rust_call_graph::extract_call_graph;
use debtmap::analyzers::Analyzer;
use debtmap::priority::call_graph::FunctionId;
use debtmap::priority::scoring::classification::{
    classify_debt_type_with_exclusions, is_dead_code_with_exclusions,
};
use debtmap::priority::DebtType;
use im::HashSet;
use std::path::PathBuf;

#[test]
fn test_rust_method_with_same_name_as_function_not_false_positive() {
    // Test case for the false positive where DependencyGraph::calculate_coupling_metrics
    // is flagged as dead code even though it's used in multiple places
    let rust_code = r#"
use std::collections::HashMap;

pub struct DependencyGraph {
    modules: Vec<String>,
}

#[derive(Debug)]
pub struct ModuleDependency {
    pub name: String,
    pub dependency_count: usize,
}

// Standalone function (also exported)
pub fn calculate_coupling_metrics(modules: &[String]) -> Vec<ModuleDependency> {
    modules.iter().map(|name| ModuleDependency {
        name: name.clone(),
        dependency_count: 0,
    }).collect()
}

impl DependencyGraph {
    pub fn new() -> Self {
        Self {
            modules: Vec::new(),
        }
    }

    // Method with same name as the standalone function - this should NOT be flagged as dead code
    pub fn calculate_coupling_metrics(&self) -> Vec<ModuleDependency> {
        self.modules.iter().map(|module| {
            ModuleDependency {
                name: module.clone(),
                dependency_count: self.get_dependencies(module).len(),
            }
        }).collect()
    }

    fn get_dependencies(&self, _module: &str) -> Vec<String> {
        vec![]
    }
}

// Usage in analysis utils
pub fn analyze_dependencies() -> Vec<ModuleDependency> {
    let dep_graph = DependencyGraph::new();
    // This call should prevent the method from being marked as dead code
    dep_graph.calculate_coupling_metrics()
}

// Usage in tests
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coupling_analysis() {
        let graph = DependencyGraph::new();
        // Another call that should prevent dead code detection
        let metrics = graph.calculate_coupling_metrics();
        assert!(metrics.is_empty());
    }
}
"#;

    // Parse and analyze the Rust code
    let analyzer = RustAnalyzer::new();
    let path = PathBuf::from("dependency_graph.rs");
    let ast = analyzer.parse(rust_code, path.clone()).unwrap();
    let metrics = analyzer.analyze(&ast);

    // Parse the Rust code and extract call graph
    let syntax_tree = syn::parse_file(rust_code).unwrap();
    let call_graph = extract_call_graph(&syntax_tree, &path);

    // Find the calculate_coupling_metrics METHOD (not the standalone function)
    let method_func = metrics
        .complexity
        .functions
        .iter()
        .find(|f| {
            f.name.contains("calculate_coupling_metrics") && f.name.contains("DependencyGraph::")
            // This identifies it as the method
        })
        .expect("Should find DependencyGraph::calculate_coupling_metrics method");

    // Create function ID for the method using the actual line number from metrics
    let func_id = FunctionId::new(
        path.clone(),
        "DependencyGraph::calculate_coupling_metrics".to_string(),
        method_func.line,
    );

    // Check if it's marked as dead code
    let framework_exclusions_im = HashSet::new();
    let framework_exclusions_std: std::collections::HashSet<FunctionId> =
        framework_exclusions_im.clone().into_iter().collect();
    let is_dead = is_dead_code_with_exclusions(
        method_func,
        &call_graph,
        &func_id,
        &framework_exclusions_std,
        None,
    );

    // Should NOT be marked as dead code because it has 2 callers
    assert!(
        !is_dead,
        "DependencyGraph::calculate_coupling_metrics should NOT be marked as dead code because it has 2 callers: analyze_dependencies() and tests::test_coupling_analysis()"
    );

    // Also check the debt type classification
    let debt_type = classify_debt_type_with_exclusions(
        method_func,
        &call_graph,
        &func_id,
        &framework_exclusions_std,
        None,
        None, // No coverage data for this test
    );

    // It should NOT be classified as DeadCode
    match debt_type {
        DebtType::DeadCode { .. } => {
            panic!("DependencyGraph::calculate_coupling_metrics should not be classified as DeadCode! This is the false positive we're testing for.");
        }
        _ => {
            // Good - it's not dead code
        }
    }
}

#[test]
fn test_rust_function_vs_method_distinction() {
    // Test that the analyzer can distinguish between a standalone function
    // and a method with the same name
    let rust_code = r#"
pub struct Calculator {
    value: i32,
}

// Standalone function - this one IS dead code
pub fn calculate(x: i32) -> i32 {
    x * 2
}

impl Calculator {
    // Method with same name - this one is NOT dead code
    pub fn calculate(&self) -> i32 {
        self.value * 3
    }
}

pub fn use_calculator() -> i32 {
    let calc = Calculator { value: 10 };
    calc.calculate() // Only the method is called, not the standalone function
}
"#;

    let analyzer = RustAnalyzer::new();
    let path = PathBuf::from("calculator.rs");
    let ast = analyzer.parse(rust_code, path.clone()).unwrap();
    let metrics = analyzer.analyze(&ast);

    let syntax_tree = syn::parse_file(rust_code).unwrap();
    let call_graph = extract_call_graph(&syntax_tree, &path);

    // Find the standalone function
    let standalone_func = metrics
        .complexity
        .functions
        .iter()
        .find(|f| {
            f.name.contains("calculate") 
            && !f.name.contains("Calculator::") // This is the standalone function
            && !f.name.contains("use_calculator")
        })
        .expect("Should find standalone calculate function");

    // Find the method
    let method_func = metrics
        .complexity
        .functions
        .iter()
        .find(|f| {
            f.name.contains("calculate") && f.name.contains("Calculator::") // This is the method
        })
        .expect("Should find Calculator::calculate method");

    // Test standalone function (should be dead code)
    let standalone_func_id =
        FunctionId::new(path.clone(), "calculate".to_string(), standalone_func.line);

    // For this test, we need to bypass the language-level dead code detection setting
    // and test the core logic directly.

    // Check if hardcoded exclusions would prevent detection
    let standalone_excluded = standalone_func.name == "main"
        || standalone_func.name.starts_with("_start")
        || standalone_func.is_test
        || standalone_func.name.starts_with("test_")
        || standalone_func.file.to_string_lossy().contains("/tests/")
        || standalone_func.in_test_module
        || standalone_func.name.contains("<closure@")
        || standalone_func.name.contains("extern")
        || standalone_func.name.starts_with("__")
        || standalone_func.is_trait_method;

    let standalone_has_callers = !call_graph.get_callers(&standalone_func_id).is_empty();

    // Since Rust dead code detection is disabled by default, we'll manually check the logic
    let standalone_is_dead = if !standalone_excluded && !standalone_has_callers {
        true // Would be dead code if detection was enabled
    } else {
        false
    };

    // Test method (should NOT be dead code)
    let method_func_id = FunctionId::new(
        path.clone(),
        "Calculator::calculate".to_string(),
        method_func.line,
    );

    // Check if method would be dead code using the same logic
    let method_excluded = method_func.name == "main"
        || method_func.name.starts_with("_start")
        || method_func.is_test
        || method_func.name.starts_with("test_")
        || method_func.file.to_string_lossy().contains("/tests/")
        || method_func.in_test_module
        || method_func.name.contains("<closure@")
        || method_func.name.contains("extern")
        || method_func.name.starts_with("__")
        || method_func.is_trait_method;

    let method_has_callers = !call_graph.get_callers(&method_func_id).is_empty();

    let method_is_dead = if !method_excluded && !method_has_callers {
        true // Would be dead code if detection was enabled
    } else {
        false
    };

    assert!(
        standalone_is_dead,
        "Standalone calculate function should be marked as dead code. Found: {}",
        standalone_is_dead
    );

    assert!(
        !method_is_dead,
        "Calculator::calculate method should NOT be marked as dead code"
    );
}
