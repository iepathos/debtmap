//! Caller classification for dependency metrics (Spec 267)
//!
//! This module provides pure functions to classify function callers as either
//! test or production code. This separation prevents well-tested code from
//! being penalized for having comprehensive test coverage.
//!
//! # Motivation
//!
//! When calculating dependency metrics like blast radius, counting all callers
//! equally creates false positives. A function with 90 callers where 85 are
//! tests is NOT a high-risk change target - it's well-tested code.
//!
//! # Design
//!
//! The classification follows a two-tier approach:
//! 1. **Call graph lookup**: If available, use the call graph's `is_test_function()`
//! 2. **Heuristic fallback**: Pattern matching on function names and paths
//!
//! All functions in this module are pure - they take inputs and return outputs
//! with no side effects.

use super::call_graph::{CallGraph, FunctionId};

/// Classification of a function caller as test or production code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallerType {
    /// Production code caller
    Production,
    /// Test code caller (test functions, test helpers, mocks, fixtures)
    Test,
}

/// Result of classifying a set of callers into production and test categories.
#[derive(Debug, Clone, Default)]
pub struct ClassifiedCallers {
    /// Production callers
    pub production: Vec<String>,
    /// Test callers
    pub test: Vec<String>,
    /// Count of production callers
    pub production_count: usize,
    /// Count of test callers
    pub test_count: usize,
}

impl ClassifiedCallers {
    /// Create a new ClassifiedCallers with empty vectors
    pub fn new() -> Self {
        Self::default()
    }

    /// Total count of all callers
    pub fn total_count(&self) -> usize {
        self.production_count + self.test_count
    }
}

/// Classify a single caller as test or production.
///
/// This is the main entry point for caller classification. It first attempts
/// to use the call graph for accurate classification, then falls back to
/// heuristics if the call graph is unavailable or doesn't contain the caller.
///
/// # Arguments
///
/// * `caller` - The name/identifier of the calling function
/// * `call_graph` - Optional call graph for accurate classification
///
/// # Returns
///
/// `CallerType::Test` if the caller is a test function, `CallerType::Production` otherwise.
///
/// # Examples
///
/// ```rust,ignore
/// let caller_type = classify_caller("test_parse_array", None);
/// assert_eq!(caller_type, CallerType::Test);
///
/// let caller_type = classify_caller("process_data", None);
/// assert_eq!(caller_type, CallerType::Production);
/// ```
pub fn classify_caller(caller: &str, call_graph: Option<&CallGraph>) -> CallerType {
    // Try call graph first for accurate classification
    if let Some(cg) = call_graph {
        // Parse the caller string to extract file and function info
        // Caller format: "file::function" or just "function"
        if let Some(func_id) = parse_caller_to_func_id(caller) {
            if cg.is_test_function(&func_id) {
                return CallerType::Test;
            }
            // Also check if this is a test helper (only called by tests)
            if cg.is_test_helper(&func_id) {
                return CallerType::Test;
            }
        }
    }

    // Fallback to heuristics
    classify_by_heuristics(caller)
}

/// Parse a caller string into a FunctionId for call graph lookup.
///
/// Caller strings may be in various formats:
/// - "function_name" (simple)
/// - "module::function" (with module)
/// - "path/to/file.rs::function" (with file path)
fn parse_caller_to_func_id(caller: &str) -> Option<FunctionId> {
    // Handle common formats
    if caller.contains("::") {
        let parts: Vec<&str> = caller.rsplitn(2, "::").collect();
        if parts.len() == 2 {
            let func_name = parts[0];
            let path_or_module = parts[1];

            // If it looks like a file path, use it
            if path_or_module.contains('/') || path_or_module.ends_with(".rs") {
                return Some(FunctionId::new(
                    std::path::PathBuf::from(path_or_module),
                    func_name.to_string(),
                    0, // Line unknown
                ));
            }

            // Otherwise, use module path as a pseudo file path
            return Some(FunctionId::new(
                std::path::PathBuf::from(format!("{}.rs", path_or_module.replace("::", "/"))),
                func_name.to_string(),
                0,
            ));
        }
    }

    // Simple function name - can't look up without file context
    None
}

/// Classify a caller using heuristic patterns.
///
/// This function uses name-based heuristics to classify callers when
/// call graph data is unavailable. It errs on the side of caution,
/// only classifying as test when patterns are clear.
///
/// # Test patterns recognized:
///
/// - Prefix patterns: `test_`, `tests::`, `test::`
/// - Suffix patterns: `_test`, `_tests`
/// - BDD patterns: `should_`, `it_`, `spec_`, `verify_`, `when_`, `given_`
/// - Path patterns: `/tests/`, `/test/`, `#[cfg(test)]` module indicators
/// - Framework patterns: `mock_`, `stub_`, `fake_`, `fixture_`
///
/// # Arguments
///
/// * `caller` - The name/path of the calling function
///
/// # Returns
///
/// `CallerType::Test` if the caller matches test patterns, `CallerType::Production` otherwise.
pub fn classify_by_heuristics(caller: &str) -> CallerType {
    let caller_lower = caller.to_lowercase();

    // Path-based patterns (highest confidence)
    let path_patterns = ["/tests/", "/test/", "::tests::", "::test::"];

    for pattern in path_patterns {
        if caller_lower.contains(pattern) {
            return CallerType::Test;
        }
    }

    // Extract the function name part for prefix/suffix matching
    let func_name = extract_function_name(&caller_lower);

    // Prefix patterns (high confidence)
    let prefix_patterns = [
        "test_",    // Rust/Python standard
        "tests_",   // Alternative
        "should_",  // BDD style
        "it_",      // BDD style
        "spec_",    // BDD style
        "verify_",  // Verification tests
        "when_",    // BDD given-when-then
        "given_",   // BDD given-when-then
        "mock_",    // Test infrastructure
        "stub_",    // Test infrastructure
        "fake_",    // Test infrastructure
        "fixture_", // Test fixtures
    ];

    for pattern in prefix_patterns {
        if func_name.starts_with(pattern) {
            return CallerType::Test;
        }
    }

    // Suffix patterns (medium confidence)
    let suffix_patterns = ["_test", "_tests", "_spec", "_mock", "_stub", "_fixture"];

    for pattern in suffix_patterns {
        if func_name.ends_with(pattern) {
            return CallerType::Test;
        }
    }

    // Word boundary patterns (lower confidence - require underscore boundaries)
    let word_patterns = [
        "_test_",     // test in the middle
        "_spec_",     // spec in the middle
        "_assert_",   // assertion helpers
        "_expect_",   // expectation helpers
        "_setup_",    // test setup
        "_teardown_", // test teardown
    ];

    for pattern in word_patterns {
        if func_name.contains(pattern) {
            return CallerType::Test;
        }
    }

    // Default to production
    CallerType::Production
}

/// Extract the function name from a full path/module string.
///
/// Examples:
/// - "module::function" -> "function"
/// - "path/to/file.rs::function" -> "function"
/// - "function" -> "function"
fn extract_function_name(caller: &str) -> &str {
    caller
        .rsplit("::")
        .next()
        .unwrap_or(caller)
        .rsplit('/')
        .next()
        .unwrap_or(caller)
}

/// Classify a list of callers into production and test categories.
///
/// This is a convenience function that classifies multiple callers at once
/// and returns a structured result with counts.
///
/// # Arguments
///
/// * `callers` - Iterator of caller names/identifiers
/// * `call_graph` - Optional call graph for accurate classification
///
/// # Returns
///
/// A `ClassifiedCallers` struct containing separated production and test callers.
pub fn classify_callers<'a>(
    callers: impl Iterator<Item = &'a String>,
    call_graph: Option<&CallGraph>,
) -> ClassifiedCallers {
    let mut result = ClassifiedCallers::new();

    for caller in callers {
        match classify_caller(caller, call_graph) {
            CallerType::Production => {
                result.production.push(caller.clone());
                result.production_count += 1;
            }
            CallerType::Test => {
                result.test.push(caller.clone());
                result.test_count += 1;
            }
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_by_name_patterns() {
        // Test prefix patterns
        assert_eq!(classify_by_heuristics("test_parse_array"), CallerType::Test);
        assert_eq!(
            classify_by_heuristics("should_reflow_long_lines"),
            CallerType::Test
        );
        assert_eq!(
            classify_by_heuristics("it_formats_correctly"),
            CallerType::Test
        );
        assert_eq!(classify_by_heuristics("spec_overflow"), CallerType::Test);
        assert_eq!(classify_by_heuristics("verify_output"), CallerType::Test);
        assert_eq!(
            classify_by_heuristics("mock_database_connection"),
            CallerType::Test
        );
        assert_eq!(classify_by_heuristics("stub_api_client"), CallerType::Test);
        assert_eq!(
            classify_by_heuristics("fixture_user_data"),
            CallerType::Test
        );

        // Test suffix patterns
        assert_eq!(classify_by_heuristics("parse_array_test"), CallerType::Test);
        assert_eq!(
            classify_by_heuristics("overflow_handler_spec"),
            CallerType::Test
        );

        // Test production patterns
        assert_eq!(
            classify_by_heuristics("process_file"),
            CallerType::Production
        );
        assert_eq!(classify_by_heuristics("main"), CallerType::Production);
        assert_eq!(
            classify_by_heuristics("parse_tokens"),
            CallerType::Production
        );
        assert_eq!(
            classify_by_heuristics("handle_request"),
            CallerType::Production
        );
    }

    #[test]
    fn test_classify_by_path_patterns() {
        // Path-based test detection
        assert_eq!(
            classify_by_heuristics("src/tests/helpers::create_mock"),
            CallerType::Test
        );
        assert_eq!(
            classify_by_heuristics("module::tests::test_function"),
            CallerType::Test
        );
        assert_eq!(
            classify_by_heuristics("crate::test::helpers::setup"),
            CallerType::Test
        );
    }

    #[test]
    fn test_classify_production_functions() {
        // Edge cases that should NOT be classified as test
        assert_eq!(
            classify_by_heuristics("attest_function"),
            CallerType::Production
        );
        assert_eq!(
            classify_by_heuristics("contest_winner"),
            CallerType::Production
        );
        assert_eq!(
            classify_by_heuristics("latest_version"),
            CallerType::Production
        );
        assert_eq!(
            classify_by_heuristics("testing_mode_check"),
            CallerType::Production
        );
    }

    #[test]
    fn test_classify_callers_separates_correctly() {
        let callers = vec![
            "test_parse_array".to_string(),
            "process_file".to_string(),
            "should_format".to_string(),
            "main".to_string(),
            "verify_output".to_string(),
        ];

        let result = classify_callers(callers.iter(), None);

        assert_eq!(result.production_count, 2);
        assert_eq!(result.test_count, 3);
        assert!(result.production.contains(&"process_file".to_string()));
        assert!(result.production.contains(&"main".to_string()));
        assert!(result.test.contains(&"test_parse_array".to_string()));
        assert!(result.test.contains(&"should_format".to_string()));
        assert!(result.test.contains(&"verify_output".to_string()));
    }

    #[test]
    fn test_extract_function_name() {
        assert_eq!(extract_function_name("module::function"), "function");
        assert_eq!(extract_function_name("path/to/file::function"), "function");
        assert_eq!(extract_function_name("function"), "function");
        assert_eq!(extract_function_name("a::b::c::function"), "function");
    }

    #[test]
    fn test_classify_bdd_patterns() {
        assert_eq!(
            classify_by_heuristics("when_user_clicks_button"),
            CallerType::Test
        );
        assert_eq!(
            classify_by_heuristics("given_valid_input"),
            CallerType::Test
        );
    }

    #[test]
    fn test_classify_word_boundary_patterns() {
        assert_eq!(classify_by_heuristics("user_test_helper"), CallerType::Test);
        assert_eq!(classify_by_heuristics("setup_test_data"), CallerType::Test);
        assert_eq!(
            classify_by_heuristics("assert_valid_output"),
            CallerType::Production
        ); // No underscore before assert
    }

    #[test]
    fn test_classified_callers_total_count() {
        let mut result = ClassifiedCallers::new();
        result.production_count = 5;
        result.test_count = 10;
        assert_eq!(result.total_count(), 15);
    }
}
