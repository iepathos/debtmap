use debtmap::risk::coverage_index::CoverageIndex;
use debtmap::risk::lcov::{FunctionCoverage, LcovData, NormalizedFunctionName};
use std::path::{Path, PathBuf};

fn create_function_coverage(
    name: &str,
    start_line: usize,
    execution_count: u64,
    coverage_percentage: f64,
) -> FunctionCoverage {
    FunctionCoverage {
        name: name.to_string(),
        start_line,
        execution_count,
        coverage_percentage,
        uncovered_lines: vec![],
        normalized: NormalizedFunctionName {
            full_path: name.to_string(),
            method_name: name
                .rsplit("::")
                .next()
                .unwrap_or(name)
                .split('<')
                .next()
                .unwrap_or(name)
                .to_string(),
            original: name.to_string(),
        },
    }
}

#[test]
fn test_async_function_coverage_attribution() {
    let mut coverage = LcovData::default();

    // Simulate LCOV storing closure instead of parent function
    coverage.functions.insert(
        PathBuf::from("src/workflow.rs"),
        vec![create_function_coverage(
            "process_data::{{closure}}",
            100,
            50,
            85.0,
        )],
    );

    let index = CoverageIndex::from_coverage(&coverage);

    // Query for parent function should find closure coverage
    let result =
        index.get_function_coverage_with_line(Path::new("src/workflow.rs"), "process_data", 100);

    assert!(
        result.is_some(),
        "Should find coverage for async function via closure attribution"
    );
    assert_eq!(result.unwrap(), 0.85);
}

#[test]
fn test_async_function_with_numbered_closure() {
    let mut coverage = LcovData::default();

    coverage.functions.insert(
        PathBuf::from("src/executor.rs"),
        vec![create_function_coverage(
            "execute_task::{{closure}}#0",
            200,
            100,
            92.5,
        )],
    );

    let index = CoverageIndex::from_coverage(&coverage);

    let result =
        index.get_function_coverage_with_line(Path::new("src/executor.rs"), "execute_task", 200);

    assert!(result.is_some());
    assert_eq!(result.unwrap(), 0.925);
}

#[test]
fn test_trait_method_variant_matching() {
    let mut coverage = LcovData::default();

    // LCOV stores just the method name (common for trait implementations)
    coverage.functions.insert(
        PathBuf::from("src/visitor.rs"),
        vec![create_function_coverage("visit_expr", 177, 3507, 90.2)],
    );

    let index = CoverageIndex::from_coverage(&coverage);

    // Query with full qualified name
    let result = index.get_function_coverage_with_line(
        Path::new("src/visitor.rs"),
        "RecursiveDetector::visit_expr",
        177,
    );

    assert!(
        result.is_some(),
        "Should match trait method via variant matching"
    );
    assert_eq!(result.unwrap(), 0.902);
}

#[test]
fn test_generic_monomorphization_matching() {
    let mut coverage = LcovData::default();

    // Multiple monomorphizations of the same generic function
    coverage.functions.insert(
        PathBuf::from("src/executor.rs"),
        vec![
            create_function_coverage("execute::<WorkflowExecutor>", 50, 100, 75.0),
            create_function_coverage("execute::<TestExecutor>", 50, 80, 85.0),
        ],
    );

    let index = CoverageIndex::from_coverage(&coverage);

    // Query with base name (no generics)
    let result = index.get_function_coverage_with_line(Path::new("src/executor.rs"), "execute", 50);

    assert!(
        result.is_some(),
        "Should aggregate coverage from multiple monomorphizations"
    );
    // Should aggregate both versions
    let coverage = result.unwrap();
    assert!(
        coverage > 0.7 && coverage < 0.9,
        "Coverage should be aggregated: got {}",
        coverage
    );
}

#[test]
fn test_nested_path_method_matching() {
    let mut coverage = LcovData::default();

    coverage.functions.insert(
        PathBuf::from("src/test.rs"),
        vec![create_function_coverage(
            "crate::module::Type::method",
            10,
            50,
            88.0,
        )],
    );

    let index = CoverageIndex::from_coverage(&coverage);

    // Query with just the method name
    let result = index.get_function_coverage_with_line(Path::new("src/test.rs"), "method", 10);

    assert!(
        result.is_some(),
        "Should match method from nested path via variant"
    );
    assert_eq!(result.unwrap(), 0.88);
}

#[test]
fn test_method_with_generics_matching() {
    let mut coverage = LcovData::default();

    coverage.functions.insert(
        PathBuf::from("src/processor.rs"),
        vec![create_function_coverage(
            "Processor::process<T, U, V>",
            30,
            75,
            95.0,
        )],
    );

    let index = CoverageIndex::from_coverage(&coverage);

    // Query without generics should match
    let result = index.get_function_coverage_with_line(
        Path::new("src/processor.rs"),
        "Processor::process",
        30,
    );

    assert!(
        result.is_some(),
        "Should match function with generics stripped"
    );
    assert_eq!(result.unwrap(), 0.95);

    // Query with just method name should also match
    let result2 =
        index.get_function_coverage_with_line(Path::new("src/processor.rs"), "process", 30);

    assert!(result2.is_some(), "Should match via method name variant");
    assert_eq!(result2.unwrap(), 0.95);
}

#[test]
fn test_exact_match_preferred_over_variant() {
    let mut coverage = LcovData::default();

    // Two functions: one exact match, one variant match
    coverage.functions.insert(
        PathBuf::from("src/test.rs"),
        vec![
            create_function_coverage("Type::method", 10, 50, 80.0),
            create_function_coverage("OtherType::method", 20, 30, 60.0),
        ],
    );

    let index = CoverageIndex::from_coverage(&coverage);

    // Query with full name should get exact match
    let result =
        index.get_function_coverage_with_line(Path::new("src/test.rs"), "Type::method", 10);

    assert!(result.is_some());
    assert_eq!(
        result.unwrap(),
        0.80,
        "Should prefer exact match over variant"
    );
}

#[test]
fn test_unicode_function_names() {
    let mut coverage = LcovData::default();

    coverage.functions.insert(
        PathBuf::from("src/unicode.rs"),
        vec![create_function_coverage("测试函数", 5, 10, 100.0)],
    );

    let index = CoverageIndex::from_coverage(&coverage);

    let result = index.get_function_coverage_with_line(Path::new("src/unicode.rs"), "测试函数", 5);

    assert!(result.is_some(), "Should handle unicode function names");
    assert_eq!(result.unwrap(), 1.0);
}

#[test]
fn test_complex_trait_implementation() {
    let mut coverage = LcovData::default();

    // Simulate a complex trait implementation
    coverage.functions.insert(
        PathBuf::from("src/traits.rs"),
        vec![create_function_coverage(
            "impl<T: Clone> Trait for Type<T>::complicated_method",
            100,
            200,
            78.5,
        )],
    );

    let index = CoverageIndex::from_coverage(&coverage);

    // Try matching with just the method name
    let result = index.get_function_coverage_with_line(
        Path::new("src/traits.rs"),
        "complicated_method",
        100,
    );

    assert!(
        result.is_some(),
        "Should handle complex trait implementations"
    );
    assert_eq!(result.unwrap(), 0.785);
}

#[test]
fn test_empty_function_name() {
    let mut coverage = LcovData::default();

    coverage
        .functions
        .insert(PathBuf::from("src/test.rs"), vec![]);

    let index = CoverageIndex::from_coverage(&coverage);

    let result = index.get_function_coverage_with_line(Path::new("src/test.rs"), "", 10);

    assert!(result.is_none(), "Empty function name should not match");
}

#[test]
fn test_multiple_closures_same_function() {
    let mut coverage = LcovData::default();

    // Multiple closures from the same async function
    coverage.functions.insert(
        PathBuf::from("src/async_fn.rs"),
        vec![
            create_function_coverage("async_process::{{closure}}#0", 50, 100, 90.0),
            create_function_coverage("async_process::{{closure}}#1", 55, 80, 85.0),
        ],
    );

    let index = CoverageIndex::from_coverage(&coverage);

    // Querying for parent should find one of the closures
    let result =
        index.get_function_coverage_with_line(Path::new("src/async_fn.rs"), "async_process", 50);

    assert!(result.is_some(), "Should find closure for async function");
    // Should get coverage from one of the closures
    let coverage = result.unwrap();
    assert!((0.85..=0.90).contains(&coverage));
}

#[test]
fn test_no_false_positives_with_similar_names() {
    let mut coverage = LcovData::default();

    coverage.functions.insert(
        PathBuf::from("src/test.rs"),
        vec![
            create_function_coverage("process_data", 10, 50, 80.0),
            create_function_coverage("process_data_async", 20, 30, 60.0),
        ],
    );

    let index = CoverageIndex::from_coverage(&coverage);

    // Query for exact name should not match similar name
    let result =
        index.get_function_coverage_with_line(Path::new("src/test.rs"), "process_data", 10);

    assert!(result.is_some());
    assert_eq!(
        result.unwrap(),
        0.80,
        "Should match exact name, not similar"
    );
}
