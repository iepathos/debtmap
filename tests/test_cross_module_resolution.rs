//! Integration tests for cross-module call resolution (spec 104)

use debtmap::analysis::python_call_graph::{analyze_python_project, build_cross_module_context};
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

/// Helper to create a test Python file
fn create_test_file(dir: &TempDir, name: &str, content: &str) -> PathBuf {
    let path = dir.path().join(name);
    fs::write(&path, content).unwrap();
    path
}

#[test]
fn test_basic_import_resolution() {
    let temp_dir = TempDir::new().unwrap();

    // Module with exported functions
    let helpers_content = r#"
def validate_input(data):
    """Validate input data"""
    return data is not None

def process_data(data):
    """Process the data"""
    if validate_input(data):
        return data * 2
    return None

def log_message(msg):
    """Log a message"""
    print(f"LOG: {msg}")
"#;

    // Module that imports and uses the helpers
    let main_content = r#"
from helpers import validate_input, process_data, log_message

def main():
    """Main entry point"""
    data = 42
    if validate_input(data):
        result = process_data(data)
        log_message(f"Processed: {result}")
    return result

main()
"#;

    let helpers_file = create_test_file(&temp_dir, "helpers.py", helpers_content);
    let main_file = create_test_file(&temp_dir, "main.py", main_content);

    let files = vec![helpers_file.clone(), main_file.clone()];
    let call_graph = analyze_python_project(&files).unwrap();

    // Verify that imported functions are properly linked
    // Find the actual FunctionId with correct line number
    let log_func = call_graph
        .get_all_functions()
        .find(|f| f.name == "log_message" && f.file == helpers_file)
        .expect("log_message function should exist in call graph");

    // log_message should have callers (called from main)
    let callers = call_graph.get_callers(log_func);
    assert!(
        !callers.is_empty(),
        "log_message should have callers from main module"
    );

    // validate_input should have callers from both process_data and main
    let validate_func = call_graph
        .get_all_functions()
        .find(|f| f.name == "validate_input" && f.file == helpers_file)
        .expect("validate_input function should exist in call graph");

    let validate_callers = call_graph.get_callers(validate_func);
    assert!(
        validate_callers.len() >= 2,
        "validate_input should have multiple callers"
    );
}

#[test]
fn test_aliased_import_resolution() {
    let temp_dir = TempDir::new().unwrap();

    // Module with helper functions
    let helpers_content = r#"
def transform_data(data):
    """Transform the data"""
    return data.upper() if isinstance(data, str) else str(data)

def validate_format(text):
    """Validate text format"""
    return len(text) > 0
"#;

    // Module using aliased imports
    let processor_content = r#"
from helpers import transform_data as transform
from helpers import validate_format as validate

def process_text(text):
    """Process text using aliased imports"""
    if validate(text):
        return transform(text)
    return None
"#;

    let helpers_file = create_test_file(&temp_dir, "helpers.py", helpers_content);
    let processor_file = create_test_file(&temp_dir, "processor.py", processor_content);

    let files = vec![helpers_file.clone(), processor_file.clone()];
    let call_graph = analyze_python_project(&files).unwrap();

    // Verify aliased imports are resolved
    let transform_func = call_graph
        .get_all_functions()
        .find(|f| f.name == "transform_data" && f.file == helpers_file)
        .expect("transform_data function should exist in call graph");

    let callers = call_graph.get_callers(transform_func);
    assert!(
        !callers.is_empty(),
        "transform_data should have callers through alias 'transform'"
    );

    let validate_func = call_graph
        .get_all_functions()
        .find(|f| f.name == "validate_format" && f.file == helpers_file)
        .expect("validate_format function should exist in call graph");

    let validate_callers = call_graph.get_callers(validate_func);
    assert!(
        !validate_callers.is_empty(),
        "validate_format should have callers through alias 'validate'"
    );
}

#[test]
fn test_module_import_with_attribute_access() {
    let temp_dir = TempDir::new().unwrap();

    // Module with functions
    let utils_content = r#"
def calculate(x, y):
    """Calculate result"""
    return x + y

def format_result(result):
    """Format the result"""
    return f"Result: {result}"
"#;

    // Module importing the whole module
    let app_content = r#"
import utils

def run_calculation():
    """Run calculation using module.function syntax"""
    result = utils.calculate(10, 20)
    formatted = utils.format_result(result)
    return formatted
"#;

    let utils_file = create_test_file(&temp_dir, "utils.py", utils_content);
    let app_file = create_test_file(&temp_dir, "app.py", app_content);

    let files = vec![utils_file.clone(), app_file.clone()];
    let call_graph = analyze_python_project(&files).unwrap();

    // Verify module.function calls are resolved
    let calculate_func = call_graph
        .get_all_functions()
        .find(|f| f.name == "calculate" && f.file == utils_file)
        .expect("calculate function should exist in call graph");

    let callers = call_graph.get_callers(calculate_func);
    assert!(
        !callers.is_empty(),
        "calculate should have callers through utils.calculate()"
    );

    let format_func = call_graph
        .get_all_functions()
        .find(|f| f.name == "format_result" && f.file == utils_file)
        .expect("format_result function should exist in call graph");

    let format_callers = call_graph.get_callers(format_func);
    assert!(
        !format_callers.is_empty(),
        "format_result should have callers through utils.format_result()"
    );
}

#[test]
fn test_module_import_with_alias() {
    let temp_dir = TempDir::new().unwrap();

    // Module with functions
    let database_content = r#"
def connect():
    """Connect to database"""
    return "connection"

def query(conn, sql):
    """Execute a query"""
    return f"Results for: {sql}"

def close(conn):
    """Close connection"""
    pass
"#;

    // Module using aliased module import
    let service_content = r#"
import database as db

def fetch_data():
    """Fetch data using aliased module"""
    conn = db.connect()
    results = db.query(conn, "SELECT * FROM users")
    db.close(conn)
    return results
"#;

    let db_file = create_test_file(&temp_dir, "database.py", database_content);
    let service_file = create_test_file(&temp_dir, "service.py", service_content);

    let files = vec![db_file.clone(), service_file.clone()];
    let call_graph = analyze_python_project(&files).unwrap();

    // Verify aliased module calls are resolved
    let connect_func = call_graph
        .get_all_functions()
        .find(|f| f.name == "connect" && f.file == db_file)
        .expect("connect function should exist in call graph");

    let callers = call_graph.get_callers(connect_func);
    assert!(
        !callers.is_empty(),
        "connect should have callers through db.connect()"
    );

    let query_func = call_graph
        .get_all_functions()
        .find(|f| f.name == "query" && f.file == db_file)
        .expect("query function should exist in call graph");

    let query_callers = call_graph.get_callers(query_func);
    assert!(
        !query_callers.is_empty(),
        "query should have callers through db.query()"
    );

    let close_func = call_graph
        .get_all_functions()
        .find(|f| f.name == "close" && f.file == db_file)
        .expect("close function should exist in call graph");

    let close_callers = call_graph.get_callers(close_func);
    assert!(
        !close_callers.is_empty(),
        "close should have callers through db.close()"
    );
}

#[test]
fn test_wildcard_import_resolution() {
    let temp_dir = TempDir::new().unwrap();

    // Module with multiple functions
    let math_utils_content = r#"
def add(a, b):
    """Add two numbers"""
    return a + b

def multiply(a, b):
    """Multiply two numbers"""
    return a * b

def divide(a, b):
    """Divide two numbers"""
    return a / b if b != 0 else None
"#;

    // Module using wildcard import
    let calculator_content = r#"
from math_utils import *

def calculate_expression():
    """Calculate using wildcard imported functions"""
    x = add(10, 5)
    y = multiply(x, 2)
    z = divide(y, 3)
    return z
"#;

    let math_file = create_test_file(&temp_dir, "math_utils.py", math_utils_content);
    let calc_file = create_test_file(&temp_dir, "calculator.py", calculator_content);

    let files = vec![math_file.clone(), calc_file.clone()];
    let call_graph = analyze_python_project(&files).unwrap();

    // Verify wildcard imported functions are resolved
    let add_func = call_graph
        .get_all_functions()
        .find(|f| f.name == "add" && f.file == math_file)
        .expect("add function should exist in call graph");

    let add_callers = call_graph.get_callers(add_func);
    assert!(
        !add_callers.is_empty(),
        "add should have callers through wildcard import"
    );

    let multiply_func = call_graph
        .get_all_functions()
        .find(|f| f.name == "multiply" && f.file == math_file)
        .expect("multiply function should exist in call graph");

    let multiply_callers = call_graph.get_callers(multiply_func);
    assert!(
        !multiply_callers.is_empty(),
        "multiply should have callers through wildcard import"
    );

    let divide_func = call_graph
        .get_all_functions()
        .find(|f| f.name == "divide" && f.file == math_file)
        .expect("divide function should exist in call graph");

    let divide_callers = call_graph.get_callers(divide_func);
    assert!(
        !divide_callers.is_empty(),
        "divide should have callers through wildcard import"
    );
}

#[test]
fn test_class_method_import_resolution() {
    let temp_dir = TempDir::new().unwrap();

    // Module with a class
    let manager_content = r#"
class DataManager:
    def __init__(self):
        self.data = []

    def add_item(self, item):
        """Add an item"""
        self.data.append(item)

    def process_all(self):
        """Process all items"""
        for item in self.data:
            self._process_item(item)

    def _process_item(self, item):
        """Process a single item"""
        print(f"Processing: {item}")
"#;

    // Module using the imported class
    let app_content = r#"
from manager import DataManager

def main():
    """Main application"""
    mgr = DataManager()
    mgr.add_item("item1")
    mgr.add_item("item2")
    mgr.process_all()
    return mgr
"#;

    let manager_file = create_test_file(&temp_dir, "manager.py", manager_content);
    let app_file = create_test_file(&temp_dir, "app.py", app_content);

    let files = vec![manager_file.clone(), app_file.clone()];
    let call_graph = analyze_python_project(&files).unwrap();

    // Verify class methods are resolved
    let add_method = call_graph
        .get_all_functions()
        .find(|f| f.name == "DataManager.add_item" && f.file == manager_file)
        .expect("DataManager.add_item method should exist in call graph");

    let add_callers = call_graph.get_callers(add_method);
    assert!(
        !add_callers.is_empty(),
        "add_item should have callers from main"
    );

    let process_method = call_graph
        .get_all_functions()
        .find(|f| f.name == "DataManager.process_all" && f.file == manager_file)
        .expect("DataManager.process_all method should exist in call graph");

    let process_callers = call_graph.get_callers(process_method);
    assert!(
        !process_callers.is_empty(),
        "process_all should have callers from main"
    );
}

#[test]
fn test_chained_import_resolution() {
    let temp_dir = TempDir::new().unwrap();

    // Module C with base functionality
    let module_c_content = r#"
def core_function():
    """Core functionality"""
    return "core_result"
"#;

    // Module B that imports from C and adds wrapper
    let module_b_content = r#"
from module_c import core_function

def wrapper_function():
    """Wrapper around core"""
    return core_function() + "_wrapped"
"#;

    // Module A that imports from B
    let module_a_content = r#"
from module_b import wrapper_function

def main():
    """Main function using chained imports"""
    result = wrapper_function()
    return result
"#;

    let c_file = create_test_file(&temp_dir, "module_c.py", module_c_content);
    let b_file = create_test_file(&temp_dir, "module_b.py", module_b_content);
    let a_file = create_test_file(&temp_dir, "module_a.py", module_a_content);

    let files = vec![c_file.clone(), b_file.clone(), a_file.clone()];
    let call_graph = analyze_python_project(&files).unwrap();

    // Verify chained imports are resolved
    let core_func = call_graph
        .get_all_functions()
        .find(|f| f.name == "core_function" && f.file == c_file)
        .expect("core_function should exist in call graph");

    let core_callers = call_graph.get_callers(core_func);
    assert!(
        !core_callers.is_empty(),
        "core_function should have callers through chained imports"
    );

    let wrapper_func = call_graph
        .get_all_functions()
        .find(|f| f.name == "wrapper_function" && f.file == b_file)
        .expect("wrapper_function should exist in call graph");

    let wrapper_callers = call_graph.get_callers(wrapper_func);
    assert!(
        !wrapper_callers.is_empty(),
        "wrapper_function should have callers from module_a"
    );
}

#[test]
fn test_no_false_positives_with_imports() {
    let temp_dir = TempDir::new().unwrap();

    // Module with utility functions
    let utils_content = r#"
def utility_function():
    """A utility function that is imported and used"""
    return "utility"

def unused_function():
    """This function is never imported or used"""
    return "unused"
"#;

    // Module that only imports and uses one function
    let app_content = r#"
from utils import utility_function

def main():
    """Main function"""
    result = utility_function()
    return result
"#;

    let utils_file = create_test_file(&temp_dir, "utils.py", utils_content);
    let app_file = create_test_file(&temp_dir, "app.py", app_content);

    let files = vec![utils_file.clone(), app_file.clone()];
    let context = build_cross_module_context(&files).unwrap();
    let call_graph = context.merge_call_graphs(
        files
            .iter()
            .map(|f| {
                let content = fs::read_to_string(f).unwrap();
                let module = rustpython_parser::parse(
                    &content,
                    rustpython_parser::Mode::Module,
                    f.to_str().unwrap(),
                )
                .unwrap();
                let mut extractor =
                    debtmap::analysis::python_type_tracker::TwoPassExtractor::new_with_context(
                        f.clone(),
                        &content,
                        context.clone(),
                    );
                extractor.extract(&module)
            })
            .collect(),
    );

    // Verify correct functions are marked as used/unused
    let utility_func = call_graph
        .get_all_functions()
        .find(|f| f.name == "utility_function" && f.file == utils_file)
        .expect("utility_function should exist in call graph");

    let utility_callers = call_graph.get_callers(utility_func);
    assert!(
        !utility_callers.is_empty(),
        "utility_function should have callers"
    );

    let unused_func = call_graph
        .get_all_functions()
        .find(|f| f.name == "unused_function" && f.file == utils_file)
        .expect("unused_function should exist in call graph");

    let unused_callers = call_graph.get_callers(unused_func);
    assert!(
        unused_callers.is_empty(),
        "unused_function should have no callers"
    );
}
