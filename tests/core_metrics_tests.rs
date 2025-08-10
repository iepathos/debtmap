use debtmap::core::metrics::{
    calculate_average_complexity, calculate_length_penalty, calculate_nesting_penalty,
    count_high_complexity, find_max_complexity, group_by_file, sort_by_complexity,
};
use debtmap::core::FunctionMetrics;
use std::path::PathBuf;

#[test]
fn test_group_by_file_empty() {
    let metrics = vec![];
    let grouped = group_by_file(metrics);
    assert!(grouped.is_empty());
}

#[test]
fn test_group_by_file_single_file() {
    let metrics = vec![
        FunctionMetrics::new("func1".to_string(), PathBuf::from("file1.rs"), 10),
        FunctionMetrics::new("func2".to_string(), PathBuf::from("file1.rs"), 20),
    ];

    let grouped = group_by_file(metrics);

    assert_eq!(grouped.len(), 1);
    assert!(grouped.contains_key(&PathBuf::from("file1.rs")));
    assert_eq!(grouped[&PathBuf::from("file1.rs")].len(), 2);
}

#[test]
fn test_group_by_file_multiple_files() {
    let metrics = vec![
        FunctionMetrics::new("func1".to_string(), PathBuf::from("file1.rs"), 10),
        FunctionMetrics::new("func2".to_string(), PathBuf::from("file2.rs"), 20),
        FunctionMetrics::new("func3".to_string(), PathBuf::from("file1.rs"), 30),
        FunctionMetrics::new("func4".to_string(), PathBuf::from("file3.rs"), 40),
    ];

    let grouped = group_by_file(metrics);

    assert_eq!(grouped.len(), 3);
    assert_eq!(grouped[&PathBuf::from("file1.rs")].len(), 2);
    assert_eq!(grouped[&PathBuf::from("file2.rs")].len(), 1);
    assert_eq!(grouped[&PathBuf::from("file3.rs")].len(), 1);
}

#[test]
fn test_group_by_file_preserves_metrics() {
    let mut metric = FunctionMetrics::new("complex_func".to_string(), PathBuf::from("file.rs"), 10);
    metric.cyclomatic = 5;
    metric.cognitive = 10;
    metric.nesting = 3;
    metric.length = 50;

    let metrics = vec![metric.clone()];
    let grouped = group_by_file(metrics);

    let result = &grouped[&PathBuf::from("file.rs")][0];
    assert_eq!(result.name, "complex_func");
    assert_eq!(result.cyclomatic, 5);
    assert_eq!(result.cognitive, 10);
    assert_eq!(result.nesting, 3);
    assert_eq!(result.length, 50);
}

#[test]
fn test_calculate_average_complexity_empty() {
    let metrics = vec![];
    let avg = calculate_average_complexity(&metrics);
    assert_eq!(avg, 0.0);
}

#[test]
fn test_calculate_average_complexity_single() {
    let mut metric = FunctionMetrics::new("func".to_string(), PathBuf::from("file.rs"), 10);
    metric.cyclomatic = 5;

    let metrics = vec![metric];
    let avg = calculate_average_complexity(&metrics);
    assert_eq!(avg, 5.0);
}

#[test]
fn test_calculate_average_complexity_multiple() {
    let mut metric1 = FunctionMetrics::new("func1".to_string(), PathBuf::from("file.rs"), 10);
    metric1.cyclomatic = 5;

    let mut metric2 = FunctionMetrics::new("func2".to_string(), PathBuf::from("file.rs"), 20);
    metric2.cyclomatic = 15;

    let mut metric3 = FunctionMetrics::new("func3".to_string(), PathBuf::from("file.rs"), 30);
    metric3.cyclomatic = 20;

    let metrics = vec![metric1, metric2, metric3];
    let avg = calculate_average_complexity(&metrics);
    assert_eq!(avg, 40.0 / 3.0);
}

#[test]
fn test_find_max_complexity_empty() {
    let metrics = vec![];
    let max = find_max_complexity(&metrics);
    assert_eq!(max, 0);
}

#[test]
fn test_find_max_complexity_single() {
    let mut metric = FunctionMetrics::new("func".to_string(), PathBuf::from("file.rs"), 10);
    metric.cyclomatic = 7;

    let metrics = vec![metric];
    let max = find_max_complexity(&metrics);
    assert_eq!(max, 7);
}

#[test]
fn test_find_max_complexity_multiple() {
    let mut metric1 = FunctionMetrics::new("func1".to_string(), PathBuf::from("file.rs"), 10);
    metric1.cyclomatic = 5;

    let mut metric2 = FunctionMetrics::new("func2".to_string(), PathBuf::from("file.rs"), 20);
    metric2.cyclomatic = 15;

    let mut metric3 = FunctionMetrics::new("func3".to_string(), PathBuf::from("file.rs"), 30);
    metric3.cyclomatic = 10;

    let metrics = vec![metric1, metric2, metric3];
    let max = find_max_complexity(&metrics);
    assert_eq!(max, 15);
}

#[test]
fn test_count_high_complexity_none() {
    let mut metric1 = FunctionMetrics::new("func1".to_string(), PathBuf::from("file.rs"), 10);
    metric1.cyclomatic = 5;

    let mut metric2 = FunctionMetrics::new("func2".to_string(), PathBuf::from("file.rs"), 20);
    metric2.cyclomatic = 8;

    let metrics = vec![metric1, metric2];
    let count = count_high_complexity(&metrics, 10);
    assert_eq!(count, 0);
}

#[test]
fn test_count_high_complexity_some() {
    let mut metric1 = FunctionMetrics::new("func1".to_string(), PathBuf::from("file.rs"), 10);
    metric1.cyclomatic = 5;

    let mut metric2 = FunctionMetrics::new("func2".to_string(), PathBuf::from("file.rs"), 20);
    metric2.cyclomatic = 15;

    let mut metric3 = FunctionMetrics::new("func3".to_string(), PathBuf::from("file.rs"), 30);
    metric3.cyclomatic = 20;

    let metrics = vec![metric1, metric2, metric3];
    let count = count_high_complexity(&metrics, 10);
    assert_eq!(count, 2); // 15 and 20 are > 10
}

#[test]
fn test_sort_by_complexity() {
    let mut metric1 = FunctionMetrics::new("low".to_string(), PathBuf::from("file.rs"), 10);
    metric1.cyclomatic = 3;

    let mut metric2 = FunctionMetrics::new("high".to_string(), PathBuf::from("file.rs"), 20);
    metric2.cyclomatic = 20;

    let mut metric3 = FunctionMetrics::new("medium".to_string(), PathBuf::from("file.rs"), 30);
    metric3.cyclomatic = 10;

    let metrics = vec![metric1, metric2, metric3];
    let sorted = sort_by_complexity(metrics);

    assert_eq!(sorted[0].name, "high");
    assert_eq!(sorted[1].name, "medium");
    assert_eq!(sorted[2].name, "low");
}

#[test]
fn test_calculate_nesting_penalty() {
    assert_eq!(calculate_nesting_penalty(0), 0);
    assert_eq!(calculate_nesting_penalty(1), 0);
    assert_eq!(calculate_nesting_penalty(2), 0);
    assert_eq!(calculate_nesting_penalty(3), 1);
    assert_eq!(calculate_nesting_penalty(4), 1);
    assert_eq!(calculate_nesting_penalty(5), 2);
    assert_eq!(calculate_nesting_penalty(6), 2);
    assert_eq!(calculate_nesting_penalty(7), 3);
}

#[test]
fn test_calculate_length_penalty() {
    assert_eq!(calculate_length_penalty(10), 0);
    assert_eq!(calculate_length_penalty(20), 0);
    assert_eq!(calculate_length_penalty(21), 1);
    assert_eq!(calculate_length_penalty(50), 1);
    assert_eq!(calculate_length_penalty(51), 2);
    assert_eq!(calculate_length_penalty(100), 2);
    assert_eq!(calculate_length_penalty(101), 3);
}

#[test]
fn test_complexity_metrics_from_function() {
    use debtmap::core::ComplexityMetrics;

    // Test with a simple function
    let func = FunctionMetrics {
        name: "test_func".to_string(),
        file: PathBuf::from("test.rs"),
        line: 42,
        cyclomatic: 5,
        cognitive: 8,
        nesting: 2,
        length: 30,
    };

    let metrics = ComplexityMetrics::from_function(&func);

    // Should contain exactly one function
    assert_eq!(metrics.functions.len(), 1);
    assert_eq!(metrics.functions[0].name, "test_func");
    assert_eq!(metrics.functions[0].cyclomatic, 5);
    assert_eq!(metrics.functions[0].cognitive, 8);
    assert_eq!(metrics.functions[0].nesting, 2);
    assert_eq!(metrics.functions[0].length, 30);

    // Overall metrics should match the single function
    assert_eq!(metrics.cyclomatic_complexity, 5);
    assert_eq!(metrics.cognitive_complexity, 8);
}

#[test]
fn test_complexity_metrics_from_function_zero_values() {
    use debtmap::core::ComplexityMetrics;

    // Test with zero complexity values
    let func = FunctionMetrics {
        name: "simple_func".to_string(),
        file: PathBuf::from("simple.rs"),
        line: 1,
        cyclomatic: 0,
        cognitive: 0,
        nesting: 0,
        length: 1,
    };

    let metrics = ComplexityMetrics::from_function(&func);

    assert_eq!(metrics.functions.len(), 1);
    assert_eq!(metrics.cyclomatic_complexity, 0);
    assert_eq!(metrics.cognitive_complexity, 0);
}

#[test]
fn test_complexity_metrics_from_function_high_values() {
    use debtmap::core::ComplexityMetrics;

    // Test with high complexity values
    let func = FunctionMetrics {
        name: "complex_func".to_string(),
        file: PathBuf::from("complex.rs"),
        line: 100,
        cyclomatic: 50,
        cognitive: 75,
        nesting: 10,
        length: 500,
    };

    let metrics = ComplexityMetrics::from_function(&func);

    assert_eq!(metrics.functions.len(), 1);
    assert_eq!(metrics.cyclomatic_complexity, 50);
    assert_eq!(metrics.cognitive_complexity, 75);
    assert_eq!(metrics.functions[0].line, 100);
    assert_eq!(metrics.functions[0].length, 500);
}

#[test]
fn test_complexity_metrics_from_function_preserves_path() {
    use debtmap::core::ComplexityMetrics;

    // Test that file path is preserved correctly
    let complex_path = PathBuf::from("/src/deeply/nested/module/impl.rs");
    let func = FunctionMetrics {
        name: "nested_func".to_string(),
        file: complex_path.clone(),
        line: 200,
        cyclomatic: 3,
        cognitive: 4,
        nesting: 1,
        length: 15,
    };

    let metrics = ComplexityMetrics::from_function(&func);

    assert_eq!(metrics.functions[0].file, complex_path);
}
