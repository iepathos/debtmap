//! Tests for the explain_coverage module.
//!
//! These tests verify the pure formatting functions following the Stillwater
//! philosophy - testing the "still" core without I/O concerns.

use super::formatter::{
    find_matching_functions, format_attempt, format_available_files, format_available_functions,
    format_coverage_status, format_header, format_matching_attempts, format_text_report,
    format_truncated_list,
};
use super::types::{ExplainCoverageResult, StrategyAttempt};

// =============================================================================
// Test Data Builders
// =============================================================================

fn make_result(
    function_name: &str,
    file_path: Option<&str>,
    coverage_found: bool,
) -> ExplainCoverageResult {
    ExplainCoverageResult {
        function_name: function_name.to_string(),
        file_path: file_path.map(String::from),
        coverage_found,
        coverage_percentage: if coverage_found { Some(0.85) } else { None },
        matched_by_strategy: if coverage_found {
            Some("exact_match".to_string())
        } else {
            None
        },
        attempts: Vec::new(),
        available_functions: Vec::new(),
        available_files: Vec::new(),
    }
}

fn make_attempt(strategy: &str, success: bool) -> StrategyAttempt {
    if success {
        StrategyAttempt::success(strategy, "test_fn".to_string(), "test.rs".to_string(), 0.75)
    } else {
        StrategyAttempt::failure(strategy)
    }
}

// =============================================================================
// Header Formatting Tests
// =============================================================================

#[test]
fn format_header_without_file() {
    let result = make_result("my_function", None, false);
    let output = format_header(&result);

    assert!(output.contains("Coverage Detection Explanation"));
    assert!(output.contains("Function: my_function"));
    assert!(!output.contains("File:"));
}

#[test]
fn format_header_with_file() {
    let result = make_result("my_function", Some("src/lib.rs"), false);
    let output = format_header(&result);

    assert!(output.contains("Function: my_function"));
    assert!(output.contains("File: src/lib.rs"));
}

// =============================================================================
// Coverage Status Tests
// =============================================================================

#[test]
fn format_coverage_status_found() {
    let result = make_result("test_fn", None, true);
    let output = format_coverage_status(&result);

    assert!(output.contains("✓ Coverage Found!"));
    assert!(output.contains("Strategy: exact_match"));
    assert!(output.contains("Coverage: 85.0%"));
}

#[test]
fn format_coverage_status_not_found() {
    let result = make_result("test_fn", None, false);
    let output = format_coverage_status(&result);

    assert!(output.contains("✗ Coverage Not Found"));
    assert!(!output.contains("Strategy:"));
}

// =============================================================================
// Attempt Formatting Tests
// =============================================================================

#[test]
fn format_attempt_success() {
    let attempt = make_attempt("filename_match", true);
    let output = format_attempt(&attempt);

    assert!(output.contains("✓ filename_match"));
    assert!(output.contains("File: test.rs"));
    assert!(output.contains("Function: test_fn"));
    assert!(output.contains("Coverage: 75.0%"));
}

#[test]
fn format_attempt_failure() {
    let attempt = make_attempt("fuzzy_search", false);
    let output = format_attempt(&attempt);

    assert!(output.contains("✗ fuzzy_search"));
    assert!(!output.contains("File:"));
    assert!(!output.contains("Function:"));
}

#[test]
fn format_matching_attempts_multiple() {
    let attempts = vec![
        make_attempt("exact_match", false),
        make_attempt("filename_match", true),
    ];
    let output = format_matching_attempts(&attempts);

    assert!(output.contains("Matching Attempts:"));
    assert!(output.contains("✗ exact_match"));
    assert!(output.contains("✓ filename_match"));
}

// =============================================================================
// List Formatting Tests
// =============================================================================

#[test]
fn format_truncated_list_under_limit() {
    let items = vec!["a.rs", "b.rs", "c.rs"];
    let output = format_truncated_list(&items, 10);

    assert!(output.contains("1. a.rs"));
    assert!(output.contains("2. b.rs"));
    assert!(output.contains("3. c.rs"));
    assert!(!output.contains("... and"));
}

#[test]
fn format_truncated_list_over_limit() {
    let items: Vec<String> = (1..=15).map(|i| format!("file{}.rs", i)).collect();
    let output = format_truncated_list(&items, 10);

    assert!(output.contains("1. file1.rs"));
    assert!(output.contains("10. file10.rs"));
    assert!(!output.contains("11. file11.rs"));
    assert!(output.contains("... and 5 more"));
}

// =============================================================================
// Function Matching Tests
// =============================================================================

#[test]
fn find_matching_functions_case_insensitive() {
    let available = vec![
        "module::MyFunction".to_string(),
        "other::my_function".to_string(),
        "unrelated::foo".to_string(),
    ];
    let matches = find_matching_functions("function", &available);

    assert_eq!(matches.len(), 2);
    assert!(matches.iter().any(|f| f.contains("MyFunction")));
    assert!(matches.iter().any(|f| f.contains("my_function")));
}

#[test]
fn find_matching_functions_no_matches() {
    let available = vec!["module::foo".to_string(), "other::bar".to_string()];
    let matches = find_matching_functions("nonexistent", &available);

    assert!(matches.is_empty());
}

// =============================================================================
// Available Data Formatting Tests
// =============================================================================

#[test]
fn format_available_files_with_truncation() {
    let files: Vec<String> = (1..=15).map(|i| format!("src/file{}.rs", i)).collect();
    let output = format_available_files(&files);

    assert!(output.contains("Available Files in LCOV (15 total):"));
    assert!(output.contains("1. src/file1.rs"));
    assert!(output.contains("... and 5 more"));
}

#[test]
fn format_available_functions_with_matches() {
    let mut result = make_result("parse", None, false);
    result.available_functions = vec![
        "module::parse_file".to_string(),
        "other::parse_line".to_string(),
        "unrelated::foo".to_string(),
    ];
    let output = format_available_functions(&result);

    assert!(output.contains("Functions containing 'parse':"));
    assert!(output.contains("- module::parse_file"));
    assert!(output.contains("- other::parse_line"));
    assert!(!output.contains("- unrelated::foo"));
}

#[test]
fn format_available_functions_no_matches() {
    let mut result = make_result("nonexistent", None, false);
    result.available_functions = vec!["module::foo".to_string(), "other::bar".to_string()];
    let output = format_available_functions(&result);

    assert!(output.contains("No functions found containing 'nonexistent'"));
    assert!(output.contains("First 10 available functions:"));
    assert!(output.contains("- module::foo"));
}

// =============================================================================
// Complete Report Formatting Tests
// =============================================================================

#[test]
fn format_text_report_coverage_found_not_verbose() {
    let result = make_result("test_fn", Some("src/lib.rs"), true);
    let output = format_text_report(&result, false);

    assert!(output.contains("✓ Coverage Found!"));
    // Should NOT show attempts when coverage found and not verbose
    assert!(!output.contains("Matching Attempts:"));
    // Should NOT show available files/functions when coverage found
    assert!(!output.contains("Available Files"));
}

#[test]
fn format_text_report_coverage_found_verbose() {
    let mut result = make_result("test_fn", Some("src/lib.rs"), true);
    result.attempts = vec![make_attempt("exact_match", true)];
    let output = format_text_report(&result, true);

    assert!(output.contains("✓ Coverage Found!"));
    // SHOULD show attempts when verbose=true
    assert!(output.contains("Matching Attempts:"));
}

#[test]
fn format_text_report_coverage_not_found() {
    let mut result = make_result("missing_fn", None, false);
    result.attempts = vec![make_attempt("exact_match", false)];
    result.available_files = vec!["src/main.rs".to_string()];
    result.available_functions = vec!["main::run".to_string()];
    let output = format_text_report(&result, false);

    assert!(output.contains("✗ Coverage Not Found"));
    // Should show attempts, available files, and functions when not found
    assert!(output.contains("Matching Attempts:"));
    assert!(output.contains("Available Files in LCOV"));
    assert!(output.contains("Available Functions in LCOV"));
}
