/// Simple test for cross-module Python call graph
use debtmap::analysis::python_type_tracker::TwoPassExtractor;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

#[test]
fn test_simple_single_module() {
    let temp_dir = TempDir::new().unwrap();

    // Simple single module to test basic functionality
    let test_path = temp_dir.path().join("test.py");
    fs::write(
        &test_path,
        r#"
def helper():
    return 42

def main():
    result = helper()
    return result
"#,
    )
    .unwrap();

    // Use TwoPassExtractor directly
    let content = fs::read_to_string(&test_path).unwrap();
    let module =
        rustpython_parser::parse(&content, rustpython_parser::Mode::Module, "test.py").unwrap();

    let mut extractor = TwoPassExtractor::new_with_source(test_path.clone(), &content);

    let call_graph = extractor.extract(&module);

    // Check that functions are detected
    let all_functions: Vec<_> = call_graph.get_all_functions().collect();

    println!("Found {} functions", all_functions.len());
    for func in &all_functions {
        println!("  - {}", func.name);
    }

    assert!(
        all_functions.iter().any(|f| f.name == "helper"),
        "helper function should be detected"
    );
    assert!(
        all_functions.iter().any(|f| f.name == "main"),
        "main function should be detected"
    );

    // Check that helper has callers
    let helper = all_functions
        .iter()
        .find(|f| f.name == "helper")
        .expect("helper should exist");

    let callers = call_graph.get_callers(helper);
    assert!(!callers.is_empty(), "helper should have callers from main");
}
