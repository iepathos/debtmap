//! Integration test for test function detection on real files
//! This verifies that test functions in cargo-cargofmt's overflow.rs are correctly detected

use debtmap::analyzers::call_graph::CallGraphExtractor;
use std::path::PathBuf;

#[test]
fn test_detect_test_functions_in_cargo_cargofmt() {
    let overflow_path = PathBuf::from("/Users/glen/memento-mori/cargo-cargofmt/src/formatting/overflow.rs");

    // Skip test if file doesn't exist (for CI)
    if !overflow_path.exists() {
        println!("Skipping test: cargo-cargofmt not available");
        return;
    }

    let content = std::fs::read_to_string(&overflow_path).expect("Failed to read file");
    let ast = syn::parse_file(&content).expect("Failed to parse file");

    let extractor = CallGraphExtractor::new(overflow_path);
    let graph = extractor.extract(&ast);

    // Check specific test functions that should be detected
    let expected_tests = vec![
        "vertical_stays_when_too_wide",
        "short_array_not_reflowed",
        "long_array_reflowed",
        "inline_table_containing_array",
    ];

    let mut detected_tests = Vec::new();
    let mut missed_tests = Vec::new();

    for test_name in &expected_tests {
        // Check for exact match OR module-qualified match (test::func_name)
        let suffix_pattern = format!("::{}", test_name);
        let found = graph.get_all_functions().any(|func| {
            let name_matches = func.name == *test_name || func.name.ends_with(&suffix_pattern);
            name_matches && graph.is_test_function(func)
        });

        if found {
            detected_tests.push(*test_name);
        } else {
            missed_tests.push(*test_name);
        }
    }

    // Also count total test functions detected
    let total_tests: usize = graph.get_all_functions()
        .filter(|func| graph.is_test_function(func))
        .count();

    println!("\n=== Test Function Detection Results ===");
    println!("Total functions in graph: {}", graph.node_count());
    println!("Total test functions detected: {}", total_tests);
    println!("Detected expected tests: {:?}", detected_tests);
    println!("Missed expected tests: {:?}", missed_tests);

    // List first 10 test functions
    println!("\nFirst 10 detected test functions:");
    for func in graph.get_all_functions().filter(|f| graph.is_test_function(f)).take(10) {
        println!("  - {}", func.name);
    }

    assert!(
        missed_tests.is_empty(),
        "Failed to detect test functions: {:?}",
        missed_tests
    );

    // We expect at least 20 test functions (there are many more)
    assert!(
        total_tests >= 20,
        "Expected at least 20 test functions, found {}",
        total_tests
    );
}
