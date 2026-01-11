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
        // Try to parse the caller string to extract file and function info
        if let Some(func_id) = parse_caller_to_func_id(caller) {
            if cg.is_test_function(&func_id) {
                return CallerType::Test;
            }
            if cg.is_test_helper(&func_id) {
                return CallerType::Test;
            }
            // Path-based lookup failed (paths don't match exactly)
            // Fall back to name-based lookup using just the function name
            if is_test_function_by_name(&func_id.name, cg) {
                return CallerType::Test;
            }
        } else {
            // Caller string is just a function name - search by name in call graph
            // This handles cases where upstream_callers only contains function names
            if is_test_function_by_name(caller, cg) {
                return CallerType::Test;
            }
        }
    }

    // Fallback to heuristics
    classify_by_heuristics(caller)
}

/// Check if any function with this name in the call graph is a test function.
///
/// This is used when we only have a function name without file context.
/// It searches all functions in the call graph with the given name and returns
/// true if any of them is marked as a test function.
///
/// Handles module-qualified names: call graph may store `test::my_func` but
/// caller string may just be `my_func`. We match if:
/// - Exact match: `func_id.name == name`
/// - Suffix match: `func_id.name` ends with `::name` (e.g., `test::my_func` matches `my_func`)
fn is_test_function_by_name(name: &str, call_graph: &CallGraph) -> bool {
    let suffix_pattern = format!("::{}", name);
    call_graph.get_all_functions().any(|func_id| {
        let matches =
            func_id.name == name || func_id.name.ends_with(&suffix_pattern);
        matches && call_graph.is_test_function(func_id)
    })
}

/// Parse a caller string into a FunctionId for call graph lookup.
///
/// Caller strings may be in various formats:
/// - "function_name" (simple)
/// - "module::function" (with module, double colon)
/// - "file.rs:function" (with file, single colon)
/// - "path/to/file.rs::function" (with file path)
fn parse_caller_to_func_id(caller: &str) -> Option<FunctionId> {
    // Handle double-colon format: "module::function" or "path/file.rs::function"
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

    // Handle single-colon format: "file.rs:function" (common in debtmap output)
    if caller.contains(':') && !caller.contains("::") {
        let parts: Vec<&str> = caller.rsplitn(2, ':').collect();
        if parts.len() == 2 {
            let func_name = parts[0];
            let file_path = parts[1];

            // Must look like a file path
            if file_path.ends_with(".rs")
                || file_path.ends_with(".py")
                || file_path.ends_with(".js")
                || file_path.ends_with(".ts")
            {
                return Some(FunctionId::new(
                    std::path::PathBuf::from(file_path),
                    func_name.to_string(),
                    0,
                ));
            }
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
    // Include both double-colon (::) and single-colon (:) variants
    let path_patterns = [
        "/tests/",
        "/test/",
        "::tests::",
        "::test::",
        ":test:",   // Single-colon variant for test module
        ":tests:",  // Single-colon variant for tests module
    ];

    for pattern in path_patterns {
        if caller_lower.contains(pattern) {
            return CallerType::Test;
        }
    }

    // Check if caller is from a test file (e.g., test_*.rs or *_test.rs)
    if is_test_file_path(&caller_lower) {
        return CallerType::Test;
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

/// Check if the caller path indicates a test file.
///
/// Test file patterns:
/// - test_*.rs - Rust test file prefix
/// - *_test.rs - Rust test file suffix
/// - tests/*.rs - Files in tests directory
fn is_test_file_path(caller: &str) -> bool {
    // Extract file name from path (before function separator)
    let file_part = caller.split(':').next().unwrap_or("");

    // Check for test file naming conventions
    let file_name = file_part.rsplit('/').next().unwrap_or(file_part);

    // Test file name patterns
    if file_name.starts_with("test_") && file_name.ends_with(".rs") {
        return true;
    }
    if file_name.ends_with("_test.rs") || file_name.ends_with("_tests.rs") {
        return true;
    }

    // Check for tests directory
    if file_part.contains("/tests/") || file_part.starts_with("tests/") {
        return true;
    }

    false
}

/// Extract the function name from a full path/module string.
///
/// Examples:
/// - "module::function" -> "function"
/// - "path/to/file.rs::function" -> "function"
/// - "file.rs:function" -> "function"
/// - "function" -> "function"
fn extract_function_name(caller: &str) -> &str {
    caller
        .rsplit("::")
        .next()
        .unwrap_or(caller)
        .rsplit(':')
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

    #[test]
    fn test_parse_single_colon_format() {
        // Single colon format: file.rs:function
        let func_id = parse_caller_to_func_id("overflow.rs:inline_table_containing_array");
        assert!(func_id.is_some());
        let id = func_id.unwrap();
        assert_eq!(id.name, "inline_table_containing_array");
        assert_eq!(
            id.file.to_string_lossy(),
            "overflow.rs"
        );
    }

    #[test]
    fn test_parse_double_colon_format() {
        // Double colon format: module::function
        let func_id = parse_caller_to_func_id("overflow::test::inline_table");
        assert!(func_id.is_some());
        let id = func_id.unwrap();
        assert_eq!(id.name, "inline_table");
    }

    #[test]
    fn test_is_test_file_path() {
        // Test file naming patterns
        assert!(is_test_file_path("test_overflow.rs:some_func"));
        assert!(is_test_file_path("path/to/test_helpers.rs:setup"));
        assert!(is_test_file_path("overflow_test.rs:verify"));
        assert!(is_test_file_path("tests/integration.rs:test_flow"));

        // Production files
        assert!(!is_test_file_path("overflow.rs:reflow_arrays"));
        assert!(!is_test_file_path("src/main.rs:main"));
        assert!(!is_test_file_path("formatting.rs:process"));
    }

    #[test]
    fn test_extract_function_name_single_colon() {
        // Single colon format
        assert_eq!(extract_function_name("file.rs:function"), "function");
        assert_eq!(
            extract_function_name("overflow.rs:inline_table"),
            "inline_table"
        );
    }

    #[test]
    fn test_is_test_function_by_name_with_call_graph() {
        use std::path::PathBuf;

        // Create a call graph with a test function
        let mut call_graph = CallGraph::new();
        let test_fn = FunctionId::new(
            PathBuf::from("overflow.rs"),
            "inline_table_containing_array".to_string(),
            100,
        );
        call_graph.add_function(
            test_fn.clone(),
            false, // not entry point
            true,  // IS A TEST
            5,
            10,
        );

        let prod_fn = FunctionId::new(
            PathBuf::from("overflow.rs"),
            "reflow_arrays".to_string(),
            50,
        );
        call_graph.add_function(
            prod_fn.clone(),
            true,  // entry point
            false, // NOT a test
            10,
            25,
        );

        // Test function lookup by name only
        assert!(is_test_function_by_name(
            "inline_table_containing_array",
            &call_graph
        ));
        assert!(!is_test_function_by_name("reflow_arrays", &call_graph));
        assert!(!is_test_function_by_name("unknown_function", &call_graph));

        // Verify classify_caller works with call graph
        assert_eq!(
            classify_caller("inline_table_containing_array", Some(&call_graph)),
            CallerType::Test
        );
        assert_eq!(
            classify_caller("reflow_arrays", Some(&call_graph)),
            CallerType::Production
        );
    }

    #[test]
    fn test_path_mismatch_falls_back_to_name_lookup() {
        use std::path::PathBuf;

        // Simulate real scenario: call graph has FULL path, caller string has SHORT path
        let mut call_graph = CallGraph::new();

        // Call graph stores function with full path (as built during analysis)
        let test_fn = FunctionId::new(
            PathBuf::from("./src/formatting/overflow.rs"), // Full path in call graph
            "vertical_with_comment_stays_vertical".to_string(),
            450,
        );
        call_graph.add_function(
            test_fn.clone(),
            false, // not entry point
            true,  // IS A TEST
            5,
            10,
        );

        // Caller string only has filename (as appears in debt item output)
        // This creates a FunctionId with path "overflow.rs" which won't match
        // "./src/formatting/overflow.rs" in the call graph HashMap lookup.
        // The fix: if path lookup fails, fall back to name-based lookup.
        let caller = "overflow.rs:vertical_with_comment_stays_vertical";

        assert_eq!(
            classify_caller(caller, Some(&call_graph)),
            CallerType::Test,
            "Path mismatch should fall back to name-based lookup"
        );
    }

    #[test]
    fn test_prod_function_with_path_mismatch_stays_production() {
        use std::path::PathBuf;

        let mut call_graph = CallGraph::new();

        // Production function with full path
        let prod_fn = FunctionId::new(
            PathBuf::from("./src/formatting/overflow.rs"),
            "reflow_arrays".to_string(),
            100,
        );
        call_graph.add_function(
            prod_fn.clone(),
            true,  // entry point
            false, // NOT a test
            10,
            25,
        );

        // Caller string with short path - should still be Production
        let caller = "overflow.rs:reflow_arrays";

        assert_eq!(
            classify_caller(caller, Some(&call_graph)),
            CallerType::Production,
            "Production functions should remain Production even with path mismatch"
        );
    }

    #[test]
    fn test_module_qualified_name_matching() {
        use std::path::PathBuf;

        // Simulate real scenario: call graph stores functions with module prefix
        // (e.g., "test::my_func") but caller strings only have the base name ("my_func")
        let mut call_graph = CallGraph::new();

        // Call graph stores function with module-qualified name: "test::short_array_not_reflowed"
        let test_fn = FunctionId::new(
            PathBuf::from("./src/formatting/overflow.rs"),
            "test::short_array_not_reflowed".to_string(), // Module-qualified
            650,
        );
        call_graph.add_function(
            test_fn.clone(),
            false, // not entry point
            true,  // IS A TEST
            3,
            15,
        );

        // Caller string only has base name (as appears in debt item output)
        let caller = "overflow.rs:short_array_not_reflowed";

        // The fix: is_test_function_by_name now matches suffix after "::"
        assert_eq!(
            classify_caller(caller, Some(&call_graph)),
            CallerType::Test,
            "Module-qualified name (test::func) should match base name (func)"
        );
    }

    #[test]
    fn test_is_test_function_by_name_with_module_prefix() {
        use std::path::PathBuf;

        let mut call_graph = CallGraph::new();

        // Function with module prefix in name
        let test_fn = FunctionId::new(
            PathBuf::from("overflow.rs"),
            "test::vertical_stays_when_too_wide".to_string(),
            1241,
        );
        call_graph.add_function(
            test_fn.clone(),
            false,
            true, // IS A TEST
            5,
            10,
        );

        // Should match by suffix
        assert!(
            is_test_function_by_name("vertical_stays_when_too_wide", &call_graph),
            "Should match 'test::vertical_stays_when_too_wide' when searching for 'vertical_stays_when_too_wide'"
        );

        // Should also match exact name
        assert!(
            is_test_function_by_name("test::vertical_stays_when_too_wide", &call_graph),
            "Should match exact name"
        );
    }
}
