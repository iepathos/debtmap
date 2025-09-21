use debtmap::*;
use std::path::PathBuf;

#[test]
fn test_complexity_metrics() {
    let functions = vec![
        FunctionMetrics {
            name: "simple".to_string(),
            file: PathBuf::from("test.rs"),
            line: 1,
            cyclomatic: 2,
            cognitive: 3,
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
        },
        FunctionMetrics {
            name: "complex".to_string(),
            file: PathBuf::from("test.rs"),
            line: 20,
            cyclomatic: 15,
            cognitive: 20,
            nesting: 5,
            length: 50,
            is_test: false,
            visibility: None,
            is_trait_method: false,
            in_test_module: false,
            entropy_score: None,
            is_pure: None,
            purity_confidence: None,
        detected_patterns: None,
        },
        FunctionMetrics {
            name: "medium".to_string(),
            file: PathBuf::from("test.rs"),
            line: 80,
            cyclomatic: 8,
            cognitive: 10,
            nesting: 3,
            length: 30,
            is_test: false,
            visibility: None,
            is_trait_method: false,
            in_test_module: false,
            entropy_score: None,
            is_pure: None,
            purity_confidence: None,
        detected_patterns: None,
        },
    ];

    let avg = calculate_average_complexity(&functions);
    assert_eq!(
        avg, 8.333333333333334,
        "Should calculate correct average complexity"
    );

    let max = find_max_complexity(&functions);
    assert_eq!(max, 15, "Should find maximum complexity");

    let high_count = count_high_complexity(&functions, 10);
    assert_eq!(
        high_count, 1,
        "Should count high complexity functions correctly"
    );
}

#[test]
fn test_function_is_complex() {
    let func = FunctionMetrics {
        name: "test".to_string(),
        file: PathBuf::from("test.rs"),
        line: 1,
        cyclomatic: 15,
        cognitive: 8,
        nesting: 2,
        length: 30,
        is_test: false,
        visibility: None,
        is_trait_method: false,
        in_test_module: false,
        entropy_score: None,
        is_pure: None,
        purity_confidence: None,
        detected_patterns: None,
    };

    assert!(
        func.is_complex(10),
        "Should be complex when cyclomatic > threshold"
    );
    assert!(
        !func.is_complex(20),
        "Should not be complex when both metrics < threshold"
    );
}
