use debtmap::complexity::{
    average_complexity, combine_complexity, max_complexity, ComplexityCalculator,
};
use debtmap::core::FunctionMetrics;
use std::path::PathBuf;

#[test]
fn test_complexity_calculator_new() {
    let calc = ComplexityCalculator::new(10, 15);
    // Test that it creates successfully (internal fields are private)
    let metrics = FunctionMetrics {
        name: "test".to_string(),
        file: PathBuf::from("test.rs"),
        line: 1,
        cyclomatic: 5,
        cognitive: 5,
        nesting: 1,
        length: 10,
        is_test: false,
        visibility: None,
        is_trait_method: false,
        in_test_module: false,
        entropy_score: None,
        is_pure: None,
        purity_confidence: None,
        detected_patterns: None,
        upstream_callers: None,
        downstream_callees: None,
        mapping_pattern_result: None,
        adjusted_complexity: None,
    };
    assert!(!calc.is_complex(&metrics));
}

#[test]
fn test_complexity_calculator_is_complex_cyclomatic() {
    let calc = ComplexityCalculator::new(10, 15);
    let metrics = FunctionMetrics {
        name: "test".to_string(),
        file: PathBuf::from("test.rs"),
        line: 1,
        cyclomatic: 11,
        cognitive: 5,
        nesting: 1,
        length: 10,
        is_test: false,
        visibility: None,
        is_trait_method: false,
        in_test_module: false,
        entropy_score: None,
        is_pure: None,
        purity_confidence: None,
        detected_patterns: None,
        upstream_callers: None,
        downstream_callees: None,
        mapping_pattern_result: None,
        adjusted_complexity: None,
    };
    assert!(
        calc.is_complex(&metrics),
        "Should be complex when cyclomatic > threshold"
    );
}

#[test]
fn test_complexity_calculator_is_complex_cognitive() {
    let calc = ComplexityCalculator::new(10, 15);
    let metrics = FunctionMetrics {
        name: "test".to_string(),
        file: PathBuf::from("test.rs"),
        line: 1,
        cyclomatic: 5,
        cognitive: 16,
        nesting: 1,
        length: 10,
        is_test: false,
        visibility: None,
        is_trait_method: false,
        in_test_module: false,
        entropy_score: None,
        is_pure: None,
        purity_confidence: None,
        detected_patterns: None,
        upstream_callers: None,
        downstream_callees: None,
        mapping_pattern_result: None,
        adjusted_complexity: None,
    };
    assert!(
        calc.is_complex(&metrics),
        "Should be complex when cognitive > threshold"
    );
}

#[test]
fn test_complexity_calculator_is_complex_both() {
    let calc = ComplexityCalculator::new(10, 15);
    let metrics = FunctionMetrics {
        name: "test".to_string(),
        file: PathBuf::from("test.rs"),
        line: 1,
        cyclomatic: 11,
        cognitive: 16,
        nesting: 1,
        length: 10,
        is_test: false,
        visibility: None,
        is_trait_method: false,
        in_test_module: false,
        entropy_score: None,
        is_pure: None,
        purity_confidence: None,
        detected_patterns: None,
        upstream_callers: None,
        downstream_callees: None,
        mapping_pattern_result: None,
        adjusted_complexity: None,
    };
    assert!(
        calc.is_complex(&metrics),
        "Should be complex when both metrics > threshold"
    );
}

#[test]
fn test_complexity_calculator_is_not_complex() {
    let calc = ComplexityCalculator::new(10, 15);
    let metrics = FunctionMetrics {
        name: "test".to_string(),
        file: PathBuf::from("test.rs"),
        line: 1,
        cyclomatic: 10,
        cognitive: 15,
        nesting: 1,
        length: 10,
        is_test: false,
        visibility: None,
        is_trait_method: false,
        in_test_module: false,
        entropy_score: None,
        is_pure: None,
        purity_confidence: None,
        detected_patterns: None,
        upstream_callers: None,
        downstream_callees: None,
        mapping_pattern_result: None,
        adjusted_complexity: None,
    };
    assert!(
        !calc.is_complex(&metrics),
        "Should not be complex when both metrics <= threshold"
    );
}

#[test]
fn test_complexity_calculator_calculate_score_low() {
    let calc = ComplexityCalculator::new(10, 20);
    let metrics = FunctionMetrics {
        name: "test".to_string(),
        file: PathBuf::from("test.rs"),
        line: 1,
        cyclomatic: 5,
        cognitive: 10,
        nesting: 1,
        length: 10,
        is_test: false,
        visibility: None,
        is_trait_method: false,
        in_test_module: false,
        entropy_score: None,
        is_pure: None,
        purity_confidence: None,
        detected_patterns: None,
        upstream_callers: None,
        downstream_callees: None,
        mapping_pattern_result: None,
        adjusted_complexity: None,
    };
    let score = calc.calculate_score(&metrics);
    assert_eq!(
        score, 50,
        "Score should be 50 when metrics are half of thresholds"
    );
}

#[test]
fn test_complexity_calculator_calculate_score_medium() {
    let calc = ComplexityCalculator::new(10, 20);
    let metrics = FunctionMetrics {
        name: "test".to_string(),
        file: PathBuf::from("test.rs"),
        line: 1,
        cyclomatic: 10,
        cognitive: 20,
        nesting: 1,
        length: 10,
        is_test: false,
        visibility: None,
        is_trait_method: false,
        in_test_module: false,
        entropy_score: None,
        is_pure: None,
        purity_confidence: None,
        detected_patterns: None,
        upstream_callers: None,
        downstream_callees: None,
        mapping_pattern_result: None,
        adjusted_complexity: None,
    };
    let score = calc.calculate_score(&metrics);
    assert_eq!(
        score, 100,
        "Score should be 100 when metrics equal thresholds"
    );
}

#[test]
fn test_complexity_calculator_calculate_score_high() {
    let calc = ComplexityCalculator::new(10, 20);
    let metrics = FunctionMetrics {
        name: "test".to_string(),
        file: PathBuf::from("test.rs"),
        line: 1,
        cyclomatic: 20,
        cognitive: 40,
        nesting: 1,
        length: 10,
        is_test: false,
        visibility: None,
        is_trait_method: false,
        in_test_module: false,
        entropy_score: None,
        is_pure: None,
        purity_confidence: None,
        detected_patterns: None,
        upstream_callers: None,
        downstream_callees: None,
        mapping_pattern_result: None,
        adjusted_complexity: None,
    };
    let score = calc.calculate_score(&metrics);
    assert_eq!(
        score, 200,
        "Score should be 200 when metrics are double thresholds"
    );
}

#[test]
fn test_complexity_calculator_calculate_score_zero() {
    let calc = ComplexityCalculator::new(10, 20);
    let metrics = FunctionMetrics {
        name: "test".to_string(),
        file: PathBuf::from("test.rs"),
        line: 1,
        cyclomatic: 0,
        cognitive: 0,
        nesting: 0,
        length: 0,
        is_test: false,
        visibility: None,
        is_trait_method: false,
        in_test_module: false,
        entropy_score: None,
        is_pure: None,
        purity_confidence: None,
        detected_patterns: None,
        upstream_callers: None,
        downstream_callees: None,
        mapping_pattern_result: None,
        adjusted_complexity: None,
    };
    let score = calc.calculate_score(&metrics);
    assert_eq!(score, 0, "Score should be 0 when metrics are 0");
}

#[test]
fn test_combine_complexity() {
    assert_eq!(combine_complexity(0, 0), 0);
    assert_eq!(combine_complexity(5, 0), 5);
    assert_eq!(combine_complexity(0, 5), 5);
    assert_eq!(combine_complexity(3, 7), 10);
    assert_eq!(combine_complexity(100, 200), 300);
}

#[test]
fn test_max_complexity_empty() {
    let complexities: Vec<u32> = vec![];
    assert_eq!(max_complexity(&complexities), 0);
}

#[test]
fn test_max_complexity_single() {
    let complexities = vec![5];
    assert_eq!(max_complexity(&complexities), 5);
}

#[test]
fn test_max_complexity_multiple() {
    let complexities = vec![3, 7, 2, 9, 1];
    assert_eq!(max_complexity(&complexities), 9);
}

#[test]
fn test_max_complexity_all_same() {
    let complexities = vec![5, 5, 5, 5];
    assert_eq!(max_complexity(&complexities), 5);
}

#[test]
fn test_max_complexity_with_zero() {
    let complexities = vec![0, 3, 0, 7, 0];
    assert_eq!(max_complexity(&complexities), 7);
}

#[test]
fn test_average_complexity_empty() {
    let complexities: Vec<u32> = vec![];
    assert_eq!(average_complexity(&complexities), 0.0);
}

#[test]
fn test_average_complexity_single() {
    let complexities = vec![5];
    assert_eq!(average_complexity(&complexities), 5.0);
}

#[test]
fn test_average_complexity_multiple() {
    let complexities = vec![2, 4, 6, 8];
    assert_eq!(average_complexity(&complexities), 5.0);
}

#[test]
fn test_average_complexity_with_zeros() {
    let complexities = vec![0, 5, 0, 10, 0];
    assert_eq!(average_complexity(&complexities), 3.0);
}

#[test]
fn test_average_complexity_large_numbers() {
    let complexities = vec![100, 200, 300];
    assert_eq!(average_complexity(&complexities), 200.0);
}

#[test]
fn test_average_complexity_fractional_result() {
    let complexities = vec![1, 2, 3, 4, 5];
    assert_eq!(average_complexity(&complexities), 3.0);
}

#[test]
fn test_average_complexity_odd_count() {
    let complexities = vec![1, 2, 3];
    assert_eq!(average_complexity(&complexities), 2.0);
}

#[test]
fn test_complexity_calculator_boundary_values() {
    let calc = ComplexityCalculator::new(1, 1);

    let metrics_zero = FunctionMetrics {
        name: "zero".to_string(),
        file: PathBuf::from("test.rs"),
        line: 1,
        cyclomatic: 0,
        cognitive: 0,
        nesting: 0,
        length: 0,
        is_test: false,
        visibility: None,
        is_trait_method: false,
        in_test_module: false,
        entropy_score: None,
        is_pure: None,
        purity_confidence: None,
        detected_patterns: None,
        upstream_callers: None,
        downstream_callees: None,
        mapping_pattern_result: None,
        adjusted_complexity: None,
    };
    assert!(!calc.is_complex(&metrics_zero));

    let metrics_one = FunctionMetrics {
        name: "one".to_string(),
        file: PathBuf::from("test.rs"),
        line: 1,
        cyclomatic: 1,
        cognitive: 1,
        nesting: 0,
        length: 0,
        is_test: false,
        visibility: None,
        is_trait_method: false,
        in_test_module: false,
        entropy_score: None,
        is_pure: None,
        purity_confidence: None,
        detected_patterns: None,
        upstream_callers: None,
        downstream_callees: None,
        mapping_pattern_result: None,
        adjusted_complexity: None,
    };
    assert!(!calc.is_complex(&metrics_one));

    let metrics_two = FunctionMetrics {
        name: "two".to_string(),
        file: PathBuf::from("test.rs"),
        line: 1,
        cyclomatic: 2,
        cognitive: 2,
        nesting: 0,
        length: 0,
        is_test: false,
        visibility: None,
        is_trait_method: false,
        in_test_module: false,
        entropy_score: None,
        is_pure: None,
        purity_confidence: None,
        detected_patterns: None,
        upstream_callers: None,
        downstream_callees: None,
        mapping_pattern_result: None,
        adjusted_complexity: None,
    };
    assert!(calc.is_complex(&metrics_two));
}

#[test]
fn test_complexity_score_proportions() {
    let calc = ComplexityCalculator::new(10, 10);

    let metrics_cyclo_only = FunctionMetrics {
        name: "cyclo".to_string(),
        file: PathBuf::from("test.rs"),
        line: 1,
        cyclomatic: 10,
        cognitive: 0,
        nesting: 0,
        length: 0,
        is_test: false,
        visibility: None,
        is_trait_method: false,
        in_test_module: false,
        entropy_score: None,
        is_pure: None,
        purity_confidence: None,
        detected_patterns: None,
        upstream_callers: None,
        downstream_callees: None,
        mapping_pattern_result: None,
        adjusted_complexity: None,
    };
    let score_cyclo = calc.calculate_score(&metrics_cyclo_only);
    assert_eq!(
        score_cyclo, 50,
        "Only cyclomatic at threshold should give 50"
    );

    let metrics_cognitive_only = FunctionMetrics {
        name: "cognitive".to_string(),
        file: PathBuf::from("test.rs"),
        line: 1,
        cyclomatic: 0,
        cognitive: 10,
        nesting: 0,
        length: 0,
        is_test: false,
        visibility: None,
        is_trait_method: false,
        in_test_module: false,
        entropy_score: None,
        is_pure: None,
        purity_confidence: None,
        detected_patterns: None,
        upstream_callers: None,
        downstream_callees: None,
        mapping_pattern_result: None,
        adjusted_complexity: None,
    };
    let score_cognitive = calc.calculate_score(&metrics_cognitive_only);
    assert_eq!(
        score_cognitive, 50,
        "Only cognitive at threshold should give 50"
    );
}

#[test]
fn test_complexity_integration() {
    // Test that combine_complexity and max_complexity work together
    let branch1 = 5;
    let branch2 = 3;
    let combined = combine_complexity(branch1, branch2);

    let all_complexities = vec![branch1, branch2, combined];
    let max = max_complexity(&all_complexities);

    assert_eq!(combined, 8);
    assert_eq!(max, 8);

    let avg = average_complexity(&all_complexities);
    assert_eq!(avg, 16.0 / 3.0);
}
