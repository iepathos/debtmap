//! Validation and helper functions for testing pattern detection
//!
//! This module contains pure validation predicates and helper functions used
//! by the pattern detectors. All functions are stateless and have no side effects,
//! making them easy to test and compose.
//!
//! Functions are organized by purpose:
//! - File and function identification
//! - Assertion detection
//! - Timing dependency analysis
//! - Async operation detection
//! - Complexity calculation

use crate::analyzers::javascript::detectors::get_node_text;
use std::path::Path;
use tree_sitter::{Node, Query, QueryCursor, StreamingIterator};

/// Checks if a file path represents a test file
///
/// Identifies test files by common patterns:
/// - Contains ".test." or ".spec." in filename
/// - Located in "__tests__" directory
/// - Located in "test" or "tests" directory
pub(super) fn is_test_file(path: &Path) -> bool {
    let path_str = path.to_string_lossy();
    path_str.contains(".test.")
        || path_str.contains(".spec.")
        || path_str.contains("__tests__")
        || path_str.contains("/test/")
        || path_str.contains("/tests/")
}

/// Checks if a function name represents a test function
///
/// Recognizes common test framework function names
pub(super) fn is_test_function(name: &str) -> bool {
    matches!(name, "test" | "it" | "describe" | "suite" | "context")
}

/// Checks if a method name represents a snapshot testing method
pub(super) fn is_snapshot_method(method_name: &str) -> bool {
    method_name == "toMatchSnapshot" || method_name == "toMatchInlineSnapshot"
}

/// Checks if test body contains assertions
///
/// Looks for common assertion patterns from various testing frameworks
pub(super) fn has_assertions(body: &str) -> bool {
    body.contains("expect")
        || body.contains("assert")
        || body.contains("should")
        || body.contains("chai.")
        || body.contains("jest.")
        || body.contains("sinon.")
}

/// Detects timing-dependent patterns in test code
///
/// Returns the type of timing dependency if found
pub(super) fn detect_timing_dependency(body: &str) -> Option<String> {
    if body.contains("setTimeout") {
        return Some("setTimeout".to_string());
    }
    if body.contains("setInterval") {
        return Some("setInterval".to_string());
    }
    if body.contains("Date.now()") || body.contains("new Date()") {
        return Some("Date dependency".to_string());
    }
    if body.contains("Math.random()") {
        return Some("random values".to_string());
    }
    if body.contains("performance.now()") {
        return Some("performance timing".to_string());
    }
    None
}

/// Checks if test body contains async operations without proper handling
pub(super) fn contains_async_operations(body: &str) -> bool {
    (body.contains("fetch")
        || body.contains("axios")
        || body.contains("$.ajax")
        || body.contains("Promise")
        || body.contains(".then("))
        && !body.contains("await")
        && !body.contains("done()")
}

/// Calculates the complexity of a test by counting control flow structures
///
/// Complexity scoring:
/// - if/conditional: +1
/// - for/while/do: +2
/// - try: +1
/// - function call: +1
pub(super) fn calculate_test_complexity(node: Node) -> usize {
    let mut complexity = 0;
    let mut cursor = node.walk();

    loop {
        let node_kind = cursor.node().kind();

        // Count complexity indicators
        match node_kind {
            "if_statement" | "conditional_expression" => complexity += 1,
            "for_statement" | "while_statement" | "do_statement" => complexity += 2,
            "try_statement" => complexity += 1,
            "call_expression" => {
                // Count mock/stub calls as complexity
                // Note: We'd need the source string to get text content
                // For now, just count all call expressions as adding complexity
                complexity += 1;
            }
            _ => {}
        }

        if !cursor.goto_first_child() {
            while !cursor.goto_next_sibling() {
                if !cursor.goto_parent() {
                    return complexity;
                }
            }
        }
    }
}

/// Counts the number of snapshot method calls in the AST
pub(super) fn count_snapshot_methods(query: &Query, root: Node, source: &str) -> usize {
    let mut cursor = QueryCursor::new();
    let matches = cursor.matches(query, root, source.as_bytes());

    matches
        .filter_map(|match_| match_.captures.iter().find(|c| c.index == 0))
        .filter(|method| is_snapshot_method(get_node_text(method.node, source)))
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tree_sitter::Parser;

    #[test]
    fn test_is_test_file_with_test_extension() {
        let path = PathBuf::from("src/component.test.js");
        assert!(is_test_file(&path));
    }

    #[test]
    fn test_is_test_file_with_spec_extension() {
        let path = PathBuf::from("src/component.spec.ts");
        assert!(is_test_file(&path));
    }

    #[test]
    fn test_is_test_file_in_tests_directory() {
        let path = PathBuf::from("src/__tests__/component.js");
        assert!(is_test_file(&path));
    }

    #[test]
    fn test_is_test_file_in_test_directory() {
        let path = PathBuf::from("src/test/component.js");
        assert!(is_test_file(&path));
    }

    #[test]
    fn test_is_test_file_regular_file() {
        let path = PathBuf::from("src/component.js");
        assert!(!is_test_file(&path));
    }

    #[test]
    fn test_is_test_function_recognizes_test() {
        assert!(is_test_function("test"));
    }

    #[test]
    fn test_is_test_function_recognizes_it() {
        assert!(is_test_function("it"));
    }

    #[test]
    fn test_is_test_function_recognizes_describe() {
        assert!(is_test_function("describe"));
    }

    #[test]
    fn test_is_test_function_rejects_regular_function() {
        assert!(!is_test_function("myFunction"));
    }

    #[test]
    fn test_has_assertions_with_expect() {
        assert!(has_assertions("expect(result).toBe(42)"));
    }

    #[test]
    fn test_has_assertions_with_assert() {
        assert!(has_assertions("assert.equal(a, b)"));
    }

    #[test]
    fn test_has_assertions_without_assertions() {
        assert!(!has_assertions("console.log('test')"));
    }

    #[test]
    fn test_detect_timing_dependency_with_settimeout() {
        let body = "setTimeout(() => {}, 1000)";
        assert_eq!(
            detect_timing_dependency(body),
            Some("setTimeout".to_string())
        );
    }

    #[test]
    fn test_detect_timing_dependency_with_date() {
        let body = "const now = Date.now()";
        assert_eq!(
            detect_timing_dependency(body),
            Some("Date dependency".to_string())
        );
    }

    #[test]
    fn test_detect_timing_dependency_none() {
        let body = "const result = add(1, 2)";
        assert_eq!(detect_timing_dependency(body), None);
    }

    #[test]
    fn test_contains_async_operations_with_fetch_no_await() {
        assert!(contains_async_operations("fetch('/api/data')"));
    }

    #[test]
    fn test_contains_async_operations_with_fetch_and_await() {
        assert!(!contains_async_operations("await fetch('/api/data')"));
    }

    #[test]
    fn test_contains_async_operations_with_done() {
        assert!(!contains_async_operations(
            "fetch('/api/data').then(() => done())"
        ));
    }

    #[test]
    fn test_calculate_test_complexity_simple() {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_javascript::LANGUAGE.into())
            .unwrap();
        let source = "expect(1).toBe(1);";
        let tree = parser.parse(source, None).unwrap();
        let complexity = calculate_test_complexity(tree.root_node());
        assert!(complexity > 0, "Should have some complexity");
    }

    #[test]
    fn test_calculate_test_complexity_with_conditionals() {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_javascript::LANGUAGE.into())
            .unwrap();
        let source = r#"
            if (condition) {
                expect(1).toBe(1);
            }
            for (let i = 0; i < 10; i++) {
                doSomething();
            }
        "#;
        let tree = parser.parse(source, None).unwrap();
        let complexity = calculate_test_complexity(tree.root_node());
        assert!(
            complexity > 3,
            "Should have higher complexity with control structures"
        );
    }

    #[test]
    fn test_is_snapshot_method_match_snapshot() {
        assert!(is_snapshot_method("toMatchSnapshot"));
    }

    #[test]
    fn test_is_snapshot_method_inline() {
        assert!(is_snapshot_method("toMatchInlineSnapshot"));
    }

    #[test]
    fn test_is_snapshot_method_regular_matcher() {
        assert!(!is_snapshot_method("toBe"));
    }
}
