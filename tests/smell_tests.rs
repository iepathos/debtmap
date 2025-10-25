use debtmap::*;
use std::path::PathBuf;

#[test]
fn test_code_smell_long_parameter_list() {
    let func = FunctionMetrics {
        name: "test_function".to_string(),
        file: PathBuf::from("test.rs"),
        line: 10,
        cyclomatic: 5,
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
        upstream_callers: None,
        downstream_callees: None,
        mapping_pattern_result: None,
        adjusted_complexity: None,
    };

    // Test with 7 parameters (over threshold of 5)
    let smell = detect_long_parameter_list(&func, 7);
    assert!(smell.is_some(), "Should detect long parameter list");

    let smell = smell.unwrap();
    assert_eq!(smell.smell_type, SmellType::LongParameterList);
    assert_eq!(smell.severity, Priority::Medium);

    // Test with 3 parameters (under threshold)
    let no_smell = detect_long_parameter_list(&func, 3);
    assert!(
        no_smell.is_none(),
        "Should not detect smell for short parameter list"
    );
}

#[test]
fn test_code_smell_long_method() {
    let mut func = FunctionMetrics {
        name: "test_function".to_string(),
        file: PathBuf::from("test.rs"),
        line: 10,
        cyclomatic: 5,
        cognitive: 8,
        nesting: 2,
        length: 100, // Over threshold of 50
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

    let smell = detect_long_method(&func);
    assert!(smell.is_some(), "Should detect long method");

    let smell = smell.unwrap();
    assert_eq!(smell.smell_type, SmellType::LongMethod);
    assert_eq!(smell.severity, Priority::Medium);

    // Test with short method
    func.length = 30;
    let no_smell = detect_long_method(&func);
    assert!(
        no_smell.is_none(),
        "Should not detect smell for short method"
    );
}

#[test]
fn test_code_smell_deep_nesting() {
    let mut func = FunctionMetrics {
        name: "test_function".to_string(),
        file: PathBuf::from("test.rs"),
        line: 10,
        cyclomatic: 5,
        cognitive: 8,
        nesting: 6, // Over threshold of 4
        length: 30,
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

    let smell = detect_deep_nesting(&func);
    assert!(smell.is_some(), "Should detect deep nesting");

    let smell = smell.unwrap();
    assert_eq!(smell.smell_type, SmellType::DeepNesting);
    assert_eq!(smell.severity, Priority::Medium);

    // Test with shallow nesting
    func.nesting = 2;
    let no_smell = detect_deep_nesting(&func);
    assert!(
        no_smell.is_none(),
        "Should not detect smell for shallow nesting"
    );
}

#[test]
fn test_code_smell_detection_multiple() {
    let func = FunctionMetrics {
        name: "bad_function".to_string(),
        file: PathBuf::from("test.rs"),
        line: 10,
        cyclomatic: 15,
        cognitive: 20,
        nesting: 6,
        length: 100,
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

    let smells = analyze_function_smells(&func, 7);

    // Should detect multiple smells
    assert!(smells.len() >= 3, "Should detect multiple code smells");

    // Check specific smells are detected
    let has_long_params = smells
        .iter()
        .any(|s| matches!(s.smell_type, SmellType::LongParameterList));
    let has_long_method = smells
        .iter()
        .any(|s| matches!(s.smell_type, SmellType::LongMethod));
    let has_deep_nesting = smells
        .iter()
        .any(|s| matches!(s.smell_type, SmellType::DeepNesting));

    assert!(has_long_params, "Should detect long parameter list");
    assert!(has_long_method, "Should detect long method");
    assert!(has_deep_nesting, "Should detect deep nesting");
}

#[test]
fn test_module_smell_detection() {
    let path = PathBuf::from("large_module.rs");

    // Test large module detection
    let smells = analyze_module_smells(&path, 500);
    assert!(!smells.is_empty(), "Should detect large module");

    let smell = &smells[0];
    assert_eq!(smell.smell_type, SmellType::LargeClass);
    assert_eq!(smell.severity, Priority::Medium);

    // Test normal size module
    let no_smells = analyze_module_smells(&path, 200);
    assert!(
        no_smells.is_empty(),
        "Should not detect smell for normal size module"
    );
}

#[test]
fn test_code_smell_to_debt_item() {
    let smell = CodeSmell {
        smell_type: SmellType::LongMethod,
        location: PathBuf::from("test.rs"),
        line: 100,
        message: "Method is too long".to_string(),
        severity: Priority::High,
    };

    let debt_item = smell.to_debt_item();

    assert_eq!(debt_item.debt_type, DebtType::CodeSmell);
    assert_eq!(debt_item.priority, Priority::High);
    assert_eq!(debt_item.line, 100);
    assert_eq!(debt_item.message, "Method is too long");
}

#[test]
fn test_code_smell_suppression() {
    let content = r#"
// debtmap:ignore-next-line[codesmell]
                                                                                                    // This is a very long line that would normally trigger a code smell
// This is a normal line
"#;

    let path = PathBuf::from("test.rs");
    let items = find_code_smells_with_suppression(
        content,
        &path,
        Some(&parse_suppression_comments(content, Language::Rust, &path)),
    );

    assert_eq!(items.len(), 0, "Long line should be suppressed");
}
