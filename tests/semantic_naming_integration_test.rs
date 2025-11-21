//! Integration tests for semantic naming functionality
//!
//! Tests that verify the semantic naming feature works end-to-end.

use debtmap::organization::semantic_naming::SemanticNameGenerator;
use std::collections::HashSet;

#[test]
fn test_no_generic_names_in_output() {
    // Verify that semantic naming produces no generic names like "unknown", "misc", etc.

    let name_generator = SemanticNameGenerator::new();

    // Test with various method groups that might tempt generic naming
    let test_cases = vec![
        vec!["format_output".to_string(), "format_summary".to_string()],
        vec!["validate_index".to_string(), "validate_data".to_string()],
        vec!["parse_input".to_string(), "parse_config".to_string()],
        vec![
            "calculate_coverage".to_string(),
            "calculate_total_size".to_string(),
        ],
    ];

    // Generic terms that should not appear
    let generic_terms = [
        "unknown", "misc", "utils", "helpers", "module", "base", "core",
    ];

    for methods in test_cases {
        let candidates = name_generator.generate_names(&methods, None);

        // Check that no generic terms are used
        for candidate in candidates.iter() {
            let name_lower = candidate.module_name.to_lowercase();
            for generic in &generic_terms {
                assert!(
                    !name_lower.contains(generic),
                    "Generated name '{}' contains generic term '{}' for methods: {:?}",
                    candidate.module_name,
                    generic,
                    methods
                );
            }
        }
    }
}

#[test]
fn test_name_uniqueness_across_splits() {
    // Verify that when multiple method groups are analyzed, each gets a unique name

    let name_generator = SemanticNameGenerator::new();

    // Simulate multiple splits with different method groups
    let method_groups = vec![
        vec!["format_output".to_string(), "format_summary".to_string()],
        vec!["validate_index".to_string(), "validate_data".to_string()],
        vec!["parse_input".to_string(), "parse_config".to_string()],
        vec![
            "calculate_coverage".to_string(),
            "calculate_total_size".to_string(),
        ],
    ];

    let mut all_names = Vec::new();

    for methods in method_groups {
        let candidates = name_generator.generate_names(&methods, None);
        assert!(
            !candidates.is_empty(),
            "Should generate at least one candidate"
        );

        // Take the best (first) candidate
        all_names.push(candidates[0].module_name.clone());
    }

    // Check for uniqueness
    let unique_names: HashSet<_> = all_names.iter().collect();
    assert_eq!(
        all_names.len(),
        unique_names.len(),
        "All module names should be unique. Found duplicates in: {:?}",
        all_names
    );

    // Check that names are not just "needs_review_N"
    let review_names = all_names
        .iter()
        .filter(|n| n.starts_with("needs_review"))
        .count();
    let total_names = all_names.len();

    // At most 25% of names should be fallback "needs_review" names
    assert!(
        review_names as f64 / total_names as f64 <= 0.25,
        "Too many fallback names: {}/{} in {:?}",
        review_names,
        total_names,
        all_names
    );
}

#[test]
fn test_high_confidence_names() {
    // Verify that semantic naming generates high-confidence names for clear patterns

    let name_generator = SemanticNameGenerator::new();

    // Test formatting methods
    let formatting_methods = vec!["format_output".to_string(), "format_summary".to_string()];

    let candidates = name_generator.generate_names(&formatting_methods, None);
    assert!(
        !candidates.is_empty(),
        "Should generate at least one candidate"
    );

    let best_candidate = &candidates[0];
    assert!(
        best_candidate.confidence >= 0.6,
        "Should have high confidence for clear pattern. Got: {}",
        best_candidate.confidence
    );
    assert!(
        best_candidate.module_name.contains("format"),
        "Should recognize formatting pattern. Got: {}",
        best_candidate.module_name
    );

    // Test validation methods
    let validation_methods = vec!["validate_index".to_string(), "validate_data".to_string()];

    let candidates = name_generator.generate_names(&validation_methods, None);
    assert!(
        !candidates.is_empty(),
        "Should generate at least one candidate"
    );

    let best_candidate = &candidates[0];
    assert!(
        best_candidate.confidence >= 0.6,
        "Should have high confidence for clear pattern. Got: {}",
        best_candidate.confidence
    );
    assert!(
        best_candidate.module_name.contains("validat"),
        "Should recognize validation pattern. Got: {}",
        best_candidate.module_name
    );
}

#[test]
fn test_debtmap_self_analysis_naming() {
    // Test semantic naming with diverse real-world method patterns

    let name_generator = SemanticNameGenerator::new();

    // Test cases based on common patterns in debtmap's codebase
    let test_cases = vec![
        (
            vec![
                "serialize_report".to_string(),
                "serialize_metrics".to_string(),
            ],
            "serializ", // Should contain some form of "serialize"
        ),
        (
            vec![
                "transform_ast".to_string(),
                "transform_functions".to_string(),
            ],
            "transform", // Should contain "transform"
        ),
        (
            vec!["detect_pattern".to_string(), "detect_smell".to_string()],
            "detect", // Should contain some form of "detect"
        ),
    ];

    for (methods, expected_substring) in test_cases {
        let candidates = name_generator.generate_names(&methods, None);

        // Verify we get reasonable results
        assert!(
            !candidates.is_empty(),
            "Should generate at least one name candidate for {:?}",
            methods
        );

        // Check that the best candidate has acceptable quality
        let best = &candidates[0];
        assert!(
            best.specificity_score >= 0.4,
            "Should produce acceptable names for {:?}. Got: {} with score {}",
            methods,
            best.module_name,
            best.specificity_score
        );

        // Check that it recognized the pattern
        assert!(
            best.module_name.to_lowercase().contains(expected_substring),
            "Should recognize pattern in {:?}. Expected substring '{}', got '{}'",
            methods,
            expected_substring,
            best.module_name
        );
    }
}
