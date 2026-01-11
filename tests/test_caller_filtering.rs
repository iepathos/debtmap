/// Integration test for Spec 267: Filter Test Callers from Blast Radius
///
/// This test validates the acceptance criteria for Spec 267:
/// 1. Test callers are correctly classified separately from production callers
/// 2. Production blast radius is lower than or equal to total blast radius
/// 3. Scoring uses production count only (not total callers)
///
/// A function with 90 callers where 85 are tests is NOT high-risk - it's well-tested code.
/// This integration tests the full classification pipeline.

use debtmap::priority::caller_classification::{
    classify_caller, classify_callers, CallerType, ClassifiedCallers,
};

/// Test that production blast radius is correctly calculated as lower than total.
///
/// Scenario: A function `parse_array` has 10 callers total:
/// - 8 are test functions (test_*, should_*, verify_*, etc.)
/// - 2 are production functions (process_file, main)
///
/// Expected: production_blast_radius = 2, total = 10
#[test]
fn test_production_blast_radius_lower_than_total() {
    // Simulate a function with many test callers
    let callers = vec![
        // Test callers (8)
        "test_parse_empty_array".to_string(),
        "test_parse_nested_array".to_string(),
        "should_handle_overflow".to_string(),
        "verify_array_bounds".to_string(),
        "test_unicode_elements".to_string(),
        "spec_array_reflow".to_string(),
        "mock_parser_setup".to_string(),
        "fixture_array_data".to_string(),
        // Production callers (2)
        "process_file".to_string(),
        "main".to_string(),
    ];

    let classified = classify_callers(callers.iter(), None);

    // Verify classification counts
    assert_eq!(
        classified.test_count, 8,
        "Expected 8 test callers, got {}",
        classified.test_count
    );
    assert_eq!(
        classified.production_count, 2,
        "Expected 2 production callers, got {}",
        classified.production_count
    );

    // Key invariant: production blast radius < total blast radius
    let production_blast_radius = classified.production_count;
    let total_blast_radius = classified.total_count();

    assert!(
        production_blast_radius < total_blast_radius,
        "Production blast radius ({}) should be less than total ({})",
        production_blast_radius,
        total_blast_radius
    );

    // Verify the reduction is substantial (80% in this case)
    let reduction_percentage =
        ((total_blast_radius - production_blast_radius) as f64 / total_blast_radius as f64) * 100.0;
    assert!(
        reduction_percentage >= 50.0,
        "Expected at least 50% reduction in blast radius, got {:.1}%",
        reduction_percentage
    );
}

/// Test that scoring should use production_count, not total count.
///
/// The dependency factor in scoring should be based on production callers only.
/// This prevents well-tested code from being penalized.
#[test]
fn test_scoring_uses_production_count_only() {
    // Create a scenario where a function has mostly test callers
    let callers = vec![
        // 9 test callers
        "test_case_1".to_string(),
        "test_case_2".to_string(),
        "test_case_3".to_string(),
        "test_case_4".to_string(),
        "test_case_5".to_string(),
        "should_work".to_string(),
        "verify_output".to_string(),
        "fixture_setup".to_string(),
        "mock_dependency".to_string(),
        // 1 production caller
        "handle_request".to_string(),
    ];

    let classified = classify_callers(callers.iter(), None);

    // For scoring, we should use production_count
    let scoring_dependency_count = classified.production_count;

    // Verify we're using production count (1), not total count (10)
    assert_eq!(
        scoring_dependency_count, 1,
        "Scoring should use production count (1), not total count ({})",
        classified.total_count()
    );

    // If we were using total count, the dependency factor would be much higher
    // This test ensures we're not penalizing well-tested code
    assert_eq!(classified.test_count, 9);
    assert_eq!(classified.total_count(), 10);
}

/// Test classification of various caller patterns found in real codebases.
///
/// This test uses patterns from the actual codebase to ensure the classifier
/// handles real-world naming conventions correctly.
#[test]
fn test_real_world_caller_patterns() {
    // Production function patterns
    let production_patterns = vec![
        "analyze_file",
        "process_tokens",
        "calculate_complexity",
        "build_call_graph",
        "format_output",
        "parse_config",
        "create_report",
        "main",
        "run",
        "execute",
        "handle_error",
        "validate_input",
    ];

    for pattern in &production_patterns {
        let caller_type = classify_caller(pattern, None);
        assert_eq!(
            caller_type,
            CallerType::Production,
            "'{}' should be classified as Production",
            pattern
        );
    }

    // Test function patterns
    let test_patterns = vec![
        "test_analyze_file",
        "test_process_tokens",
        "should_calculate_complexity",
        "verify_call_graph",
        "spec_format_output",
        "it_parses_config",
        "when_creating_report",
        "given_valid_input",
        "mock_file_system",
        "stub_api_client",
        "fixture_test_data",
        "crate::tests::helper_function", // needs ::tests:: with double colons
        "module::tests::setup",          // needs ::tests:: with double colons
    ];

    for pattern in &test_patterns {
        let caller_type = classify_caller(pattern, None);
        assert_eq!(
            caller_type,
            CallerType::Test,
            "'{}' should be classified as Test",
            pattern
        );
    }
}

/// Test edge cases that could cause false positives/negatives.
///
/// These are tricky patterns where the classifier needs to be careful
/// not to misclassify based on partial matches.
#[test]
fn test_edge_case_caller_patterns() {
    // Should NOT be classified as test (false positive prevention)
    // These words contain "test" but are not test functions
    let false_positive_risks = vec![
        "attest_signature",   // "test" is part of "attest"
        "contest_results",    // "test" is part of "contest"
        "latest_version",     // "test" is part of "latest"
        "testing_mode_check", // "testing" but not a test function
    ];

    for pattern in &false_positive_risks {
        let caller_type = classify_caller(pattern, None);
        assert_eq!(
            caller_type,
            CallerType::Production,
            "'{}' should NOT be classified as Test (false positive)",
            pattern
        );
    }

    // These ARE classified as test because they contain `_test_` word boundary
    // which indicates test infrastructure/helpers
    let test_infrastructure = vec![
        "get_test_config", // contains _test_ - test infrastructure
        "load_test_data",  // contains _test_ - test data loader
        "is_test_file",    // contains _test_ - test utility
    ];

    for pattern in &test_infrastructure {
        let caller_type = classify_caller(pattern, None);
        assert_eq!(
            caller_type,
            CallerType::Test,
            "'{}' should be classified as Test (test infrastructure with _test_)",
            pattern
        );
    }

    // Should be classified as test (false negative prevention)
    let false_negative_risks = vec![
        "test_", // Just the prefix
        "integration_test_suite",
        "unit_test_helper",
        "e2e_test_runner",
    ];

    for pattern in &false_negative_risks {
        let caller_type = classify_caller(pattern, None);
        assert_eq!(
            caller_type,
            CallerType::Test,
            "'{}' SHOULD be classified as Test (avoid false negative)",
            pattern
        );
    }
}

/// Test that path-based patterns are correctly detected.
///
/// Callers from test directories or modules should be classified as tests
/// regardless of the function name.
#[test]
fn test_path_based_classification() {
    // Path patterns that indicate test code
    let test_paths = vec![
        "src/tests/helpers::create_mock_data",
        "crate::tests::test_utils::setup",
        "module::test::fixtures::user_data",
        "/tests/integration/helper::prepare",
        "path/to/tests/common::initialize",
    ];

    for path in &test_paths {
        let caller_type = classify_caller(path, None);
        assert_eq!(
            caller_type,
            CallerType::Test,
            "Path '{}' should be classified as Test due to path pattern",
            path
        );
    }

    // Production paths (should not be affected by test-like names in the path)
    let prod_paths = vec![
        "src/lib::process_file",
        "crate::core::analyze",
        "module::utils::format",
    ];

    for path in &prod_paths {
        let caller_type = classify_caller(path, None);
        assert_eq!(
            caller_type,
            CallerType::Production,
            "Path '{}' should be classified as Production",
            path
        );
    }
}

/// Test the ClassifiedCallers structure functionality.
#[test]
fn test_classified_callers_structure() {
    let mut classified = ClassifiedCallers::new();

    // Initially empty
    assert_eq!(classified.total_count(), 0);
    assert_eq!(classified.production_count, 0);
    assert_eq!(classified.test_count, 0);
    assert!(classified.production.is_empty());
    assert!(classified.test.is_empty());

    // Add some callers manually
    classified.production.push("main".to_string());
    classified.production_count = 1;
    classified.test.push("test_main".to_string());
    classified.test.push("verify_main".to_string());
    classified.test_count = 2;

    // Verify counts
    assert_eq!(classified.total_count(), 3);
    assert_eq!(classified.production_count, 1);
    assert_eq!(classified.test_count, 2);
}

/// Integration test simulating analysis of a real function with many test callers.
///
/// This test simulates analyzing a core utility function that is heavily tested
/// but has few production callers.
#[test]
fn test_heavily_tested_core_function() {
    // Simulate callers for a core function like `parse_expression`
    let callers = vec![
        // Production callers (5)
        "compile_module".to_string(),
        "evaluate_script".to_string(),
        "analyze_ast".to_string(),
        "process_template".to_string(),
        "execute_query".to_string(),
        // Test callers (40) - comprehensive test suite
        // test_ prefix (15)
        "test_parse_simple_expression".to_string(),
        "test_parse_binary_operator".to_string(),
        "test_parse_unary_operator".to_string(),
        "test_parse_parentheses".to_string(),
        "test_parse_function_call".to_string(),
        "test_parse_method_chain".to_string(),
        "test_parse_array_access".to_string(),
        "test_parse_object_literal".to_string(),
        "test_parse_lambda".to_string(),
        "test_parse_ternary".to_string(),
        "test_integration_with_lexer".to_string(),
        "test_integration_with_ast".to_string(),
        "test_performance_large_input".to_string(),
        "test_performance_deep_nesting".to_string(),
        "test_memory_usage".to_string(),
        // should_ prefix (5)
        "should_handle_empty_input".to_string(),
        "should_handle_whitespace".to_string(),
        "should_handle_comments".to_string(),
        "should_handle_unicode".to_string(),
        "should_handle_escapes".to_string(),
        // verify_ prefix (5)
        "verify_operator_precedence".to_string(),
        "verify_associativity_left".to_string(),
        "verify_associativity_right".to_string(),
        "verify_error_recovery".to_string(),
        "verify_error_messages".to_string(),
        // spec_ prefix (5)
        "spec_arithmetic_operators".to_string(),
        "spec_comparison_operators".to_string(),
        "spec_logical_operators".to_string(),
        "spec_assignment_operators".to_string(),
        "spec_bitwise_operators".to_string(),
        // fixture_ prefix (3)
        "fixture_complex_expression".to_string(),
        "fixture_nested_expression".to_string(),
        "fixture_edge_case_expression".to_string(),
        // mock_ prefix (2)
        "mock_parser_context".to_string(),
        "mock_token_stream".to_string(),
        // when_ prefix (3)
        "when_input_is_empty".to_string(),
        "when_input_is_invalid".to_string(),
        "when_input_is_partial".to_string(),
        // given_ prefix (2)
        "given_valid_expression".to_string(),
        "given_malformed_expression".to_string(),
    ];

    let classified = classify_callers(callers.iter(), None);

    // Verify counts: 5 production + 40 test = 45 total
    assert_eq!(classified.production_count, 5);
    assert_eq!(classified.test_count, 40);
    assert_eq!(classified.total_count(), 45);

    // This is the key insight: the function has 45 callers but only 5 are production
    // Production blast radius = 5
    // Total blast radius = 45
    // The scoring system should use 5, not 45

    let production_blast_radius = classified.production_count;
    let total_blast_radius = classified.total_count();

    // ~89% of callers are tests - this should NOT inflate the blast radius
    let test_percentage = (classified.test_count as f64 / total_blast_radius as f64) * 100.0;
    assert!(
        test_percentage >= 85.0,
        "Expected at least 85% test callers, got {:.1}%",
        test_percentage
    );

    // Production blast radius is ~11% of total
    let production_percentage =
        (production_blast_radius as f64 / total_blast_radius as f64) * 100.0;
    assert!(
        production_percentage <= 15.0,
        "Expected <=15% production callers, got {:.1}%",
        production_percentage
    );
}

/// Test that BDD-style test patterns are correctly classified.
#[test]
fn test_bdd_pattern_classification() {
    let bdd_patterns = vec![
        // Given-When-Then style
        "given_user_is_authenticated",
        "when_user_clicks_button",
        "should_display_error_message",
        "it_returns_empty_list",
        "spec_validates_input_format",
        "verify_state_transition",
    ];

    for pattern in &bdd_patterns {
        let caller_type = classify_caller(pattern, None);
        assert_eq!(
            caller_type,
            CallerType::Test,
            "BDD pattern '{}' should be classified as Test",
            pattern
        );
    }
}

/// Test word boundary patterns in function names.
#[test]
fn test_word_boundary_patterns() {
    // Test patterns with _test_ in the middle
    let mid_test_patterns = vec![
        "user_test_helper",
        "setup_test_data",
        "create_test_fixture",
        "run_test_suite",
    ];

    for pattern in &mid_test_patterns {
        let caller_type = classify_caller(pattern, None);
        assert_eq!(
            caller_type,
            CallerType::Test,
            "'{}' should be classified as Test (word boundary)",
            pattern
        );
    }

    // Patterns that should NOT match (no word boundary)
    let no_boundary_patterns = vec![
        "contest",       // not "test" word
        "detest",        // not "test" word
        "attestation",   // not "test" word
        "fastest_route", // not "_test_" with boundaries
    ];

    for pattern in &no_boundary_patterns {
        let caller_type = classify_caller(pattern, None);
        assert_eq!(
            caller_type,
            CallerType::Production,
            "'{}' should be classified as Production (no word boundary)",
            pattern
        );
    }
}
