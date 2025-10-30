//! Pattern detection implementations for JavaScript/TypeScript testing anti-patterns
//!
//! This module contains the core detection logic for identifying test quality issues.
//! Each detector function focuses on a specific anti-pattern and operates independently.
//!
//! # Detectors
//!
//! - `detect_missing_assertions`: Finds tests without expect/assert calls
//! - `detect_complex_tests`: Identifies overly complex test logic
//! - `detect_timing_dependent_tests`: Finds tests depending on setTimeout, Date, etc.
//! - `detect_react_test_issues`: Detects missing cleanup in React component tests
//! - `detect_async_test_issues`: Finds async operations without proper await or done()
//! - `detect_snapshot_overuse`: Identifies excessive use of snapshot testing
//!
//! All detectors follow a consistent pattern: they analyze the AST, use validator
//! functions to check for issues, and push detected anti-patterns to the issues vector.

use super::{
    build_async_test_query, calculate_test_complexity, contains_async_operations,
    count_snapshot_methods, detect_timing_dependency, extract_test_body,
    extract_test_function_name, extract_test_name, has_assertions, is_test_function,
    parse_test_name,
};
use super::{get_node_text, SourceLocation, TestingAntiPattern};
use tree_sitter::{Node, Query, QueryCursor, StreamingIterator};

/// Detects tests that are missing assertions
pub(super) fn detect_missing_assertions(
    root: Node,
    source: &str,
    language: &tree_sitter::Language,
    issues: &mut Vec<TestingAntiPattern>,
) {
    // Find test functions
    let test_query = r#"
    (call_expression
      function: (identifier) @func
      arguments: (arguments
        (string) @test_name
        (_) @test_body
      )
    ) @test_call
    "#;

    if let Ok(query) = Query::new(language, test_query) {
        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(&query, root, source.as_bytes());

        while let Some(match_) = matches.next() {
            if let Some(func) = match_.captures.iter().find(|c| c.index == 0) {
                let func_name = get_node_text(func.node, source);

                if is_test_function(func_name) {
                    if let (Some(name), Some(body)) = (
                        match_.captures.iter().find(|c| c.index == 1),
                        match_.captures.iter().find(|c| c.index == 2),
                    ) {
                        let test_name = get_node_text(name.node, source)
                            .trim_matches('"')
                            .trim_matches('\'');
                        let body_text = get_node_text(body.node, source);

                        if !has_assertions(body_text) {
                            issues.push(TestingAntiPattern::MissingAssertions {
                                location: SourceLocation::from_node(body.node),
                                test_name: test_name.to_string(),
                            });
                        }
                    }
                }
            }
        }
    }
}

/// Detects tests that are too complex
pub(super) fn detect_complex_tests(
    root: Node,
    source: &str,
    language: &tree_sitter::Language,
    issues: &mut Vec<TestingAntiPattern>,
) {
    let test_query = r#"
    (call_expression
      function: (identifier) @func
      arguments: (arguments
        (string) @test_name
        (_) @test_body
      )
    ) @test_call
    "#;

    if let Ok(query) = Query::new(language, test_query) {
        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(&query, root, source.as_bytes());

        while let Some(match_) = matches.next() {
            if let Some(func) = match_.captures.iter().find(|c| c.index == 0) {
                let func_name = get_node_text(func.node, source);

                if is_test_function(func_name) {
                    if let (Some(name), Some(body)) = (
                        match_.captures.iter().find(|c| c.index == 1),
                        match_.captures.iter().find(|c| c.index == 2),
                    ) {
                        let test_name = get_node_text(name.node, source)
                            .trim_matches('"')
                            .trim_matches('\'');
                        let complexity = calculate_test_complexity(body.node);

                        if complexity > 10 {
                            issues.push(TestingAntiPattern::ComplexTest {
                                location: SourceLocation::from_node(body.node),
                                test_name: test_name.to_string(),
                                complexity,
                            });
                        }
                    }
                }
            }
        }
    }
}

/// Detects tests that depend on timing
pub(super) fn detect_timing_dependent_tests(
    root: Node,
    source: &str,
    language: &tree_sitter::Language,
    issues: &mut Vec<TestingAntiPattern>,
) {
    let test_query = r#"
    (call_expression
      function: (identifier) @func
      arguments: (arguments
        (string) @test_name
        (_) @test_body
      )
    ) @test_call
    "#;

    if let Ok(query) = Query::new(language, test_query) {
        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(&query, root, source.as_bytes());

        while let Some(match_) = matches.next() {
            if let Some(func) = match_.captures.iter().find(|c| c.index == 0) {
                let func_name = get_node_text(func.node, source);

                if is_test_function(func_name) {
                    if let (Some(name), Some(body)) = (
                        match_.captures.iter().find(|c| c.index == 1),
                        match_.captures.iter().find(|c| c.index == 2),
                    ) {
                        let test_name = get_node_text(name.node, source)
                            .trim_matches('"')
                            .trim_matches('\'');
                        let body_text = get_node_text(body.node, source);

                        if let Some(timing_type) = detect_timing_dependency(body_text) {
                            issues.push(TestingAntiPattern::TimingDependentTest {
                                location: SourceLocation::from_node(body.node),
                                test_name: test_name.to_string(),
                                timing_type,
                            });
                        }
                    }
                }
            }
        }
    }
}

/// Detects React test issues (missing cleanup)
pub(super) fn detect_react_test_issues(
    root: Node,
    source: &str,
    language: &tree_sitter::Language,
    issues: &mut Vec<TestingAntiPattern>,
) {
    // Detect render without cleanup
    let render_query = r#"
    (call_expression
      function: (identifier) @func
    ) @render_call
    "#;

    let cleanup_query = r#"
    (call_expression
      function: [
        (identifier) @func
        (member_expression
          property: (property_identifier) @prop
        )
      ]
    ) @cleanup_call
    "#;

    let mut render_count = 0;
    let mut cleanup_count = 0;

    if let Ok(query) = Query::new(language, render_query) {
        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(&query, root, source.as_bytes());

        while let Some(match_) = matches.next() {
            if let Some(func) = match_.captures.iter().find(|c| c.index == 0) {
                let func_name = get_node_text(func.node, source);
                if func_name == "render" || func_name == "mount" {
                    render_count += 1;
                }
            }
        }
    }

    if let Ok(query) = Query::new(language, cleanup_query) {
        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(&query, root, source.as_bytes());

        while let Some(match_) = matches.next() {
            let is_cleanup = match_
                .captures
                .iter()
                .filter(|c| c.index == 0 || c.index == 1) // Only check @func and @prop
                .map(|c| get_node_text(c.node, source))
                .any(|text| {
                    let lower = text.to_lowercase();
                    text == "cleanup" || text == "unmount" || lower.contains("unmount")
                });

            if is_cleanup {
                cleanup_count += 1;
            }
        }
    }

    if render_count > cleanup_count {
        issues.push(TestingAntiPattern::MissingCleanup {
            location: SourceLocation::from_node(root),
            test_name: "React test".to_string(),
            resource_type: "React components".to_string(),
        });
    }
}

/// Creates an async test issue pattern
pub(super) fn create_async_test_issue(body_node: Node, test_name: String) -> TestingAntiPattern {
    TestingAntiPattern::AsyncTestIssue {
        location: SourceLocation::from_node(body_node),
        test_name,
        issue_type: "async operations without await or done callback".to_string(),
    }
}

/// Detects async test issues (missing await or done callback)
pub(super) fn detect_async_test_issues(
    root: Node,
    source: &str,
    language: &tree_sitter::Language,
    issues: &mut Vec<TestingAntiPattern>,
) {
    if let Ok(query) = build_async_test_query(language) {
        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(&query, root, source.as_bytes());

        while let Some(match_) = matches.next() {
            if let Some(func) = extract_test_function_name(match_) {
                let func_name = get_node_text(*func, source);

                if is_test_function(func_name) {
                    if let (Some(name), Some(body)) =
                        (extract_test_name(match_), extract_test_body(match_))
                    {
                        let test_name = parse_test_name(*name, source);
                        let body_text = get_node_text(*body, source);

                        // Check if test contains async operations without proper handling
                        if contains_async_operations(body_text) {
                            issues.push(create_async_test_issue(*body, test_name));
                        }
                    }
                }
            }
        }
    }
}

/// Detects overuse of snapshot testing
pub(super) fn detect_snapshot_overuse(
    root: Node,
    source: &str,
    language: &tree_sitter::Language,
    issues: &mut Vec<TestingAntiPattern>,
) {
    let snapshot_query = r#"
    (call_expression
      function: (member_expression
        property: (property_identifier) @method
      )
    ) @snapshot_call
    "#;

    if let Ok(query) = Query::new(language, snapshot_query) {
        let snapshot_count = count_snapshot_methods(&query, root, source);

        if snapshot_count > 5 {
            issues.push(TestingAntiPattern::SnapshotOveruse {
                location: SourceLocation::from_node(root),
                snapshot_count,
            });
        }
    }
}
