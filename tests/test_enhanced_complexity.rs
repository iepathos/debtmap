use debtmap::complexity::if_else_analyzer::IfElseChainAnalyzer;
use debtmap::complexity::message_generator::generate_enhanced_message;
use debtmap::complexity::recursive_detector::RecursiveMatchDetector;
use debtmap::complexity::threshold_manager::{ComplexityThresholds, FunctionRole, ThresholdPreset};
use debtmap::core::FunctionMetrics;
use std::path::PathBuf;

#[test]
fn test_recursive_match_detection() {
    let code = r#"
        fn process_data(value: Option<u32>) -> u32 {
            match value {
                Some(v) => {
                    // Nested match inside closure
                    let result = (0..10).filter_map(|i| {
                        match i {
                            0..=3 => Some(i * 2),
                            4..=6 => Some(i * 3),
                            _ => None,
                        }
                    }).sum();
                    
                    // Another nested match in async block
                    let async_result = async {
                        match v {
                            0..=10 => 1,
                            11..=20 => 2,
                            _ => 3,
                        }
                    };
                    
                    result
                }
                None => 0,
            }
        }
    "#;

    let file = syn::parse_str::<syn::File>(code).unwrap();
    let func = match &file.items[0] {
        syn::Item::Fn(f) => f,
        _ => panic!("Expected function"),
    };

    let mut detector = RecursiveMatchDetector::new();
    let matches = detector.find_matches_in_block(&func.block);

    // Should find 3 match expressions (outer + 2 nested)
    assert_eq!(matches.len(), 3, "Should detect all nested matches");

    // Verify contexts
    assert!(!matches[0].context.in_closure);
    assert!(matches.iter().any(|m| m.context.in_closure));
}

#[test]
fn test_threshold_filtering() {
    let thresholds = ComplexityThresholds::from_preset(ThresholdPreset::Strict);

    let simple_func = FunctionMetrics {
        name: "simple".to_string(),
        file: PathBuf::from("test.rs"),
        line: 1,
        cyclomatic: 2,
        cognitive: 3,
        nesting: 1,
        length: 10,
        is_test: false,
        visibility: Some("pub".to_string()),
        is_trait_method: false,
        in_test_module: false,
        entropy_score: None,
        is_pure: Some(true),
        purity_confidence: Some(0.9),
        detected_patterns: None,
        upstream_callers: None,
        downstream_callees: None,
    };

    let complex_func = FunctionMetrics {
        name: "complex".to_string(),
        file: PathBuf::from("test.rs"),
        line: 20,
        cyclomatic: 15,
        cognitive: 25,
        nesting: 4,
        length: 100,
        is_test: false,
        visibility: Some("pub".to_string()),
        is_trait_method: false,
        in_test_module: false,
        entropy_score: None,
        is_pure: Some(false),
        purity_confidence: Some(0.8),
        detected_patterns: None,
        upstream_callers: None,
        downstream_callees: None,
    };

    // Simple function should not be flagged
    assert!(!thresholds.should_flag_function(&simple_func, FunctionRole::CoreLogic));

    // Complex function should be flagged
    assert!(thresholds.should_flag_function(&complex_func, FunctionRole::CoreLogic));

    // Test functions get higher threshold
    let test_func = FunctionMetrics {
        name: "test_something".to_string(),
        file: PathBuf::from("test.rs"),
        line: 1,
        cyclomatic: 8,
        cognitive: 12,
        nesting: 2,
        length: 30,
        is_test: true,
        visibility: None,
        is_trait_method: false,
        in_test_module: true,
        entropy_score: None,
        is_pure: Some(false),
        purity_confidence: Some(0.5),
        detected_patterns: None,
        upstream_callers: None,
        downstream_callees: None,
    };

    // Test function with moderate complexity should not be flagged due to multiplier
    assert!(!thresholds.should_flag_function(&test_func, FunctionRole::Test));
}

#[test]
fn test_if_else_chain_detection() {
    let code = r#"
        fn categorize_value(value: u32) -> &'static str {
            if value == 0 {
                "zero"
            } else if value == 1 {
                "one"
            } else if value == 2 {
                "two"
            } else if value == 3 {
                "three"
            } else if value == 4 {
                "four"
            } else {
                "many"
            }
        }
    "#;

    let file = syn::parse_str::<syn::File>(code).unwrap();
    let func = match &file.items[0] {
        syn::Item::Fn(f) => f,
        _ => panic!("Expected function"),
    };

    let mut analyzer = IfElseChainAnalyzer::new();
    let chains = analyzer.analyze_block(&func.block);

    assert_eq!(chains.len(), 1, "Should detect one if-else chain");
    assert_eq!(
        chains[0].length, 6,
        "Chain should have 6 branches (5 conditions + else)"
    );
    assert!(chains[0].has_final_else);
}

#[test]
fn test_enhanced_message_generation() {
    let metrics = FunctionMetrics {
        name: "complex_handler".to_string(),
        file: PathBuf::from("handler.rs"),
        line: 42,
        cyclomatic: 20,
        cognitive: 30,
        nesting: 5,
        length: 150,
        is_test: false,
        visibility: Some("pub".to_string()),
        is_trait_method: false,
        in_test_module: false,
        entropy_score: None,
        is_pure: Some(false),
        purity_confidence: Some(0.7),
        detected_patterns: None,
        upstream_callers: None,
        downstream_callees: None,
    };

    let thresholds = ComplexityThresholds::from_preset(ThresholdPreset::Balanced);

    let message = generate_enhanced_message(
        &metrics,
        &[], // No matches for this test
        &[], // No if-else chains
        &thresholds,
    );

    // Verify message contains expected elements
    assert!(!message.summary.is_empty());
    assert!(!message.details.is_empty());
    assert!(!message.recommendations.is_empty());

    // Should identify high complexity
    assert!(message.summary.contains("complex") || message.summary.contains("Complex"));
}

#[test]
fn test_depth_limit_protection() {
    // Create a deeply nested structure that approaches our depth limit
    // Using 45 levels to be safe with the 50 depth limit
    let mut code = String::from("fn deep() { ");
    for i in 0..45 {
        code.push_str(&format!("match {} {{ _ => {{ ", i));
    }
    code.push_str("42");
    for _ in 0..45 {
        code.push_str("} }");
    }
    code.push_str(" }");

    // This should not panic due to depth limits
    if let Ok(file) = syn::parse_str::<syn::File>(&code) {
        if let syn::Item::Fn(func) = &file.items[0] {
            let mut detector = RecursiveMatchDetector::new();
            let matches = detector.find_matches_in_block(&func.block);
            // Should complete without stack overflow and find some matches
            assert!(!matches.is_empty(), "Should find at least some matches");
            // Should find a reasonable number of matches
            assert!(
                matches.len() >= 10,
                "Should find a reasonable number of matches"
            );
            // The exact count depends on how depth is tracked
            eprintln!("Found {} matches out of 45 nested levels", matches.len());
        }
    }
}

#[test]
fn test_threshold_presets() {
    let strict = ComplexityThresholds::from_preset(ThresholdPreset::Strict);
    let balanced = ComplexityThresholds::from_preset(ThresholdPreset::Balanced);
    let lenient = ComplexityThresholds::from_preset(ThresholdPreset::Lenient);

    // Verify thresholds increase from strict to lenient
    assert!(strict.minimum_cyclomatic_complexity < balanced.minimum_cyclomatic_complexity);
    assert!(balanced.minimum_cyclomatic_complexity < lenient.minimum_cyclomatic_complexity);

    assert!(strict.minimum_cognitive_complexity < balanced.minimum_cognitive_complexity);
    assert!(balanced.minimum_cognitive_complexity < lenient.minimum_cognitive_complexity);

    assert!(strict.minimum_function_length < balanced.minimum_function_length);
    assert!(balanced.minimum_function_length < lenient.minimum_function_length);
}

#[test]
fn test_role_based_thresholds() {
    let thresholds = ComplexityThresholds::default();

    // Different roles should have different multipliers
    assert_ne!(
        thresholds.get_role_multiplier(FunctionRole::Test),
        thresholds.get_role_multiplier(FunctionRole::CoreLogic)
    );

    assert_ne!(
        thresholds.get_role_multiplier(FunctionRole::EntryPoint),
        thresholds.get_role_multiplier(FunctionRole::Utility)
    );

    // Test functions should be most lenient
    assert!(
        thresholds.get_role_multiplier(FunctionRole::Test)
            > thresholds.get_role_multiplier(FunctionRole::CoreLogic)
    );
}

#[test]
fn test_false_positive_reduction() {
    // Test that trivial functions are not flagged
    let trivial_functions = vec![
        ("getter", 1, 2, 5),
        ("setter", 2, 3, 8),
        ("simple_calc", 3, 4, 10),
        ("is_valid", 2, 2, 7),
    ];

    let thresholds = ComplexityThresholds::from_preset(ThresholdPreset::Balanced);

    for (name, cyclo, cog, lines) in trivial_functions {
        let func = FunctionMetrics {
            name: name.to_string(),
            file: PathBuf::from("test.rs"),
            line: 1,
            cyclomatic: cyclo,
            cognitive: cog,
            nesting: 1,
            length: lines,
            is_test: false,
            visibility: None,
            is_trait_method: false,
            in_test_module: false,
            entropy_score: None,
            is_pure: Some(true),
            purity_confidence: Some(0.9),
            detected_patterns: None,
        upstream_callers: None,
        downstream_callees: None,
        };

        assert!(
            !thresholds.should_flag_function(&func, FunctionRole::Utility),
            "Trivial function '{}' should not be flagged",
            name
        );
    }
}
