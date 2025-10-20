// debtmap:ignore-start -- This file contains test pattern detection and may trigger false security positives
// Testing pattern detection for JavaScript/TypeScript

use super::{get_node_text, SourceLocation};
use crate::core::{DebtItem, DebtType, Priority};
use std::path::{Path, PathBuf};
use tree_sitter::{Node, Query, QueryCursor, StreamingIterator};

#[derive(Debug, Clone)]
pub enum TestingAntiPattern {
    MissingAssertions {
        location: SourceLocation,
        test_name: String,
    },
    ComplexTest {
        location: SourceLocation,
        test_name: String,
        complexity: usize,
    },
    TimingDependentTest {
        location: SourceLocation,
        test_name: String,
        timing_type: String,
    },
    MissingCleanup {
        location: SourceLocation,
        test_name: String,
        resource_type: String,
    },
    AsyncTestIssue {
        location: SourceLocation,
        test_name: String,
        issue_type: String,
    },
    SnapshotOveruse {
        location: SourceLocation,
        snapshot_count: usize,
    },
    MissingErrorHandling {
        location: SourceLocation,
        test_name: String,
    },
}

impl TestingAntiPattern {
    pub fn to_debt_item(&self, path: &Path) -> DebtItem {
        let (message, priority) = match self {
            Self::MissingAssertions { test_name, .. } => (
                format!("Test '{}' has no assertions", test_name),
                Priority::High,
            ),
            Self::ComplexTest {
                test_name,
                complexity,
                ..
            } => (
                format!(
                    "Test '{}' is too complex (complexity: {})",
                    test_name, complexity
                ),
                Priority::Medium,
            ),
            Self::TimingDependentTest {
                test_name,
                timing_type,
                ..
            } => (
                format!("Test '{}' depends on timing ({})", test_name, timing_type),
                Priority::High,
            ),
            Self::MissingCleanup {
                test_name,
                resource_type,
                ..
            } => (
                format!("Test '{}' doesn't clean up {}", test_name, resource_type),
                Priority::Medium,
            ),
            Self::AsyncTestIssue {
                test_name,
                issue_type,
                ..
            } => (
                format!("Async test '{}' has {}", test_name, issue_type),
                Priority::Medium,
            ),
            Self::SnapshotOveruse { snapshot_count, .. } => (
                format!("Excessive snapshot testing ({} snapshots)", snapshot_count),
                Priority::Low,
            ),
            Self::MissingErrorHandling { test_name, .. } => (
                format!("Test '{}' lacks error handling", test_name),
                Priority::Medium,
            ),
        };

        let location = match self {
            Self::MissingAssertions { location, .. }
            | Self::ComplexTest { location, .. }
            | Self::TimingDependentTest { location, .. }
            | Self::MissingCleanup { location, .. }
            | Self::AsyncTestIssue { location, .. }
            | Self::SnapshotOveruse { location, .. }
            | Self::MissingErrorHandling { location, .. } => location,
        };

        DebtItem {
            id: format!("test-{}-{}", path.display(), location.line),
            debt_type: DebtType::TestQuality,
            priority,
            file: path.to_path_buf(),
            line: location.line,
            column: location.column,
            message,
            context: Some("Improve test quality for better maintainability".to_string()),
        }
    }
}

pub fn detect_testing_patterns(
    root: Node,
    source: &str,
    language: &tree_sitter::Language,
    path: PathBuf,
    issues: &mut Vec<TestingAntiPattern>,
) {
    // Only run test detection on test files
    if !is_test_file(&path) {
        return;
    }

    detect_missing_assertions(root, source, language, issues);
    detect_complex_tests(root, source, language, issues);
    detect_timing_dependent_tests(root, source, language, issues);
    detect_react_test_issues(root, source, language, issues);
    detect_async_test_issues(root, source, language, issues);
    detect_snapshot_overuse(root, source, language, issues);
}

fn is_test_file(path: &Path) -> bool {
    let path_str = path.to_string_lossy();
    path_str.contains(".test.")
        || path_str.contains(".spec.")
        || path_str.contains("__tests__")
        || path_str.contains("/test/")
        || path_str.contains("/tests/")
}

fn detect_missing_assertions(
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

fn detect_complex_tests(
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

fn detect_timing_dependent_tests(
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

fn detect_react_test_issues(
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
            if let Some(func) = match_.captures.first() {
                let text = get_node_text(func.node, source);
                if text == "cleanup" || text == "unmount" || text.contains("unmount") {
                    cleanup_count += 1;
                }
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

fn build_async_test_query(
    language: &tree_sitter::Language,
) -> Result<Query, tree_sitter::QueryError> {
    let query_string = r#"
    (call_expression
      function: (identifier) @func
      arguments: (arguments
        (string) @test_name
        (arrow_function
          body: (_) @body
        )
      )
    ) @test_call
    "#;
    Query::new(language, query_string)
}

fn detect_async_test_issues(
    root: Node,
    source: &str,
    language: &tree_sitter::Language,
    issues: &mut Vec<TestingAntiPattern>,
) {
    if let Ok(query) = build_async_test_query(language) {
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

                        // Check if test contains async operations without proper handling
                        if contains_async_operations(body_text) {
                            issues.push(TestingAntiPattern::AsyncTestIssue {
                                location: SourceLocation::from_node(body.node),
                                test_name: test_name.to_string(),
                                issue_type: "async operations without await or done callback"
                                    .to_string(),
                            });
                        }
                    }
                }
            }
        }
    }
}

fn is_snapshot_method(method_name: &str) -> bool {
    method_name == "toMatchSnapshot" || method_name == "toMatchInlineSnapshot"
}

fn count_snapshot_methods(query: &Query, root: Node, source: &str) -> usize {
    let mut cursor = QueryCursor::new();
    let matches = cursor.matches(query, root, source.as_bytes());

    matches
        .filter_map(|match_| match_.captures.iter().find(|c| c.index == 0))
        .filter(|method| is_snapshot_method(get_node_text(method.node, source)))
        .count()
}

fn detect_snapshot_overuse(
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

// Helper functions
fn is_test_function(name: &str) -> bool {
    matches!(name, "test" | "it" | "describe" | "suite" | "context")
}

fn has_assertions(body: &str) -> bool {
    body.contains("expect")
        || body.contains("assert")
        || body.contains("should")
        || body.contains("chai.")
        || body.contains("jest.")
        || body.contains("sinon.")
}

fn calculate_test_complexity(node: Node) -> usize {
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

fn detect_timing_dependency(body: &str) -> Option<String> {
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

fn contains_async_operations(body: &str) -> bool {
    (body.contains("fetch")
        || body.contains("axios")
        || body.contains("$.ajax")
        || body.contains("Promise")
        || body.contains(".then("))
        && !body.contains("await")
        && !body.contains("done()")
}
// debtmap:ignore-end

#[cfg(test)]
mod tests {
    use super::*;
    use tree_sitter::Parser;

    #[test]
    fn test_is_snapshot_method_match_snapshot() {
        assert!(is_snapshot_method("toMatchSnapshot"));
    }

    #[test]
    fn test_is_snapshot_method_inline_snapshot() {
        assert!(is_snapshot_method("toMatchInlineSnapshot"));
    }

    #[test]
    fn test_is_snapshot_method_non_snapshot() {
        assert!(!is_snapshot_method("toBe"));
        assert!(!is_snapshot_method("toEqual"));
        assert!(!is_snapshot_method("toContain"));
    }

    #[test]
    fn test_is_snapshot_method_empty_string() {
        assert!(!is_snapshot_method(""));
    }

    #[test]
    fn test_detect_snapshot_overuse_threshold() {
        let javascript = tree_sitter_javascript::LANGUAGE.into();
        let mut parser = Parser::new();
        parser.set_language(&javascript).unwrap();

        // Test with exactly 5 snapshots (should not trigger)
        let source_5 = r#"
            test('test1', () => {
                expect(result1).toMatchSnapshot();
                expect(result2).toMatchSnapshot();
                expect(result3).toMatchSnapshot();
                expect(result4).toMatchSnapshot();
                expect(result5).toMatchSnapshot();
            });
        "#;

        let tree = parser.parse(source_5, None).unwrap();
        let mut issues = Vec::new();
        detect_snapshot_overuse(tree.root_node(), source_5, &javascript, &mut issues);
        assert_eq!(issues.len(), 0, "5 snapshots should not trigger overuse");

        // Test with 6 snapshots (should trigger)
        let source_6 = r#"
            test('test2', () => {
                expect(result1).toMatchSnapshot();
                expect(result2).toMatchSnapshot();
                expect(result3).toMatchInlineSnapshot();
                expect(result4).toMatchSnapshot();
                expect(result5).toMatchSnapshot();
                expect(result6).toMatchInlineSnapshot();
            });
        "#;

        let tree = parser.parse(source_6, None).unwrap();
        let mut issues = Vec::new();
        detect_snapshot_overuse(tree.root_node(), source_6, &javascript, &mut issues);
        assert_eq!(issues.len(), 1, "6 snapshots should trigger overuse");

        if let TestingAntiPattern::SnapshotOveruse { snapshot_count, .. } = &issues[0] {
            assert_eq!(*snapshot_count, 6, "Should count 6 snapshots");
        } else {
            panic!("Expected SnapshotOveruse pattern");
        }
    }

    #[test]
    fn test_detect_snapshot_overuse_no_snapshots() {
        let javascript = tree_sitter_javascript::LANGUAGE.into();
        let mut parser = Parser::new();
        parser.set_language(&javascript).unwrap();

        let source = r#"
            test('regular test', () => {
                expect(result).toBe(42);
                expect(name).toEqual('test');
                expect(array).toContain('item');
            });
        "#;

        let tree = parser.parse(source, None).unwrap();
        let mut issues = Vec::new();
        detect_snapshot_overuse(tree.root_node(), source, &javascript, &mut issues);
        assert_eq!(issues.len(), 0, "No snapshots should not trigger");
    }

    #[test]
    fn test_detect_async_test_issues_fetch_without_await() {
        let javascript = tree_sitter_javascript::LANGUAGE.into();
        let mut parser = Parser::new();
        parser.set_language(&javascript).unwrap();

        let source = r#"
            test('fetch without await', () => {
                const data = fetch('/api/data');
                expect(data).toBeDefined();
            });
        "#;

        let tree = parser.parse(source, None).unwrap();
        let mut issues = Vec::new();
        detect_async_test_issues(tree.root_node(), source, &javascript, &mut issues);
        assert_eq!(issues.len(), 1, "Should detect fetch without await");

        if let TestingAntiPattern::AsyncTestIssue {
            test_name,
            issue_type,
            ..
        } = &issues[0]
        {
            assert_eq!(test_name, "fetch without await");
            assert_eq!(
                issue_type,
                "async operations without await or done callback"
            );
        } else {
            panic!("Expected AsyncTestIssue pattern");
        }
    }

    #[test]
    fn test_detect_async_test_issues_axios_without_await() {
        let javascript = tree_sitter_javascript::LANGUAGE.into();
        let mut parser = Parser::new();
        parser.set_language(&javascript).unwrap();

        let source = r#"
            it('axios without await', () => {
                const response = axios.get('/api/users');
                expect(response).toBeTruthy();
            });
        "#;

        let tree = parser.parse(source, None).unwrap();
        let mut issues = Vec::new();
        detect_async_test_issues(tree.root_node(), source, &javascript, &mut issues);
        assert_eq!(issues.len(), 1, "Should detect axios without await");
    }

    #[test]
    fn test_detect_async_test_issues_promise_without_await() {
        let javascript = tree_sitter_javascript::LANGUAGE.into();
        let mut parser = Parser::new();
        parser.set_language(&javascript).unwrap();

        let source = r#"
            test('Promise without await', () => {
                const promise = new Promise((resolve) => resolve(42));
                expect(promise).toBeDefined();
            });
        "#;

        let tree = parser.parse(source, None).unwrap();
        let mut issues = Vec::new();
        detect_async_test_issues(tree.root_node(), source, &javascript, &mut issues);
        assert_eq!(issues.len(), 1, "Should detect Promise without await");
    }

    #[test]
    fn test_detect_async_test_issues_then_without_await() {
        let javascript = tree_sitter_javascript::LANGUAGE.into();
        let mut parser = Parser::new();
        parser.set_language(&javascript).unwrap();

        let source = r#"
            test('then() without await', () => {
                getData().then(data => {
                    expect(data).toBe(42);
                });
            });
        "#;

        let tree = parser.parse(source, None).unwrap();
        let mut issues = Vec::new();
        detect_async_test_issues(tree.root_node(), source, &javascript, &mut issues);
        assert_eq!(issues.len(), 1, "Should detect .then() without await");
    }

    #[test]
    fn test_detect_async_test_issues_with_await_should_not_trigger() {
        let javascript = tree_sitter_javascript::LANGUAGE.into();
        let mut parser = Parser::new();
        parser.set_language(&javascript).unwrap();

        let source = r#"
            test('fetch with await', () => {
                const data = await fetch('/api/data');
                expect(data).toBeDefined();
            });
        "#;

        let tree = parser.parse(source, None).unwrap();
        let mut issues = Vec::new();
        detect_async_test_issues(tree.root_node(), source, &javascript, &mut issues);
        assert_eq!(issues.len(), 0, "Should not trigger when await is present");
    }

    #[test]
    fn test_detect_async_test_issues_with_done_callback_should_not_trigger() {
        let javascript = tree_sitter_javascript::LANGUAGE.into();
        let mut parser = Parser::new();
        parser.set_language(&javascript).unwrap();

        let source = r#"
            test('fetch with done', () => {
                fetch('/api/data').then(() => {
                    done();
                });
            });
        "#;

        let tree = parser.parse(source, None).unwrap();
        let mut issues = Vec::new();
        detect_async_test_issues(tree.root_node(), source, &javascript, &mut issues);
        assert_eq!(issues.len(), 0, "Should not trigger when done() is present");
    }

    #[test]
    fn test_detect_async_test_issues_non_test_function_should_not_trigger() {
        let javascript = tree_sitter_javascript::LANGUAGE.into();
        let mut parser = Parser::new();
        parser.set_language(&javascript).unwrap();

        let source = r#"
            function helperFunction() {
                const data = fetch('/api/data');
                return data;
            }
        "#;

        let tree = parser.parse(source, None).unwrap();
        let mut issues = Vec::new();
        detect_async_test_issues(tree.root_node(), source, &javascript, &mut issues);
        assert_eq!(issues.len(), 0, "Should not trigger for non-test functions");
    }

    #[test]
    fn test_detect_async_test_issues_no_async_operations() {
        let javascript = tree_sitter_javascript::LANGUAGE.into();
        let mut parser = Parser::new();
        parser.set_language(&javascript).unwrap();

        let source = r#"
            test('synchronous test', () => {
                const result = 1 + 1;
                expect(result).toBe(2);
            });
        "#;

        let tree = parser.parse(source, None).unwrap();
        let mut issues = Vec::new();
        detect_async_test_issues(tree.root_node(), source, &javascript, &mut issues);
        assert_eq!(issues.len(), 0, "Should not trigger for synchronous tests");
    }

    #[test]
    fn test_detect_async_test_issues_multiple_async_patterns() {
        let javascript = tree_sitter_javascript::LANGUAGE.into();
        let mut parser = Parser::new();
        parser.set_language(&javascript).unwrap();

        let source = r#"
            test('multiple async patterns', () => {
                const data1 = fetch('/api/data1');
                const data2 = axios.get('/api/data2');
                const promise = new Promise((resolve) => resolve(42));
                expect(data1).toBeDefined();
            });
        "#;

        let tree = parser.parse(source, None).unwrap();
        let mut issues = Vec::new();
        detect_async_test_issues(tree.root_node(), source, &javascript, &mut issues);
        assert_eq!(
            issues.len(),
            1,
            "Should detect multiple async patterns as one issue"
        );
    }

    #[test]
    fn test_detect_async_test_issues_ajax_without_await() {
        let javascript = tree_sitter_javascript::LANGUAGE.into();
        let mut parser = Parser::new();
        parser.set_language(&javascript).unwrap();

        let source = r#"
            test('jQuery ajax without await', () => {
                const data = $.ajax({ url: '/api/data' });
                expect(data).toBeDefined();
            });
        "#;

        let tree = parser.parse(source, None).unwrap();
        let mut issues = Vec::new();
        detect_async_test_issues(tree.root_node(), source, &javascript, &mut issues);
        assert_eq!(issues.len(), 1, "Should detect $.ajax without await");
    }

    #[test]
    fn test_build_async_test_query_success() {
        let javascript = tree_sitter_javascript::LANGUAGE.into();
        let result = build_async_test_query(&javascript);
        assert!(result.is_ok(), "Query construction should succeed");
    }

    #[test]
    fn test_build_async_test_query_returns_valid_query() {
        let javascript = tree_sitter_javascript::LANGUAGE.into();
        let query = build_async_test_query(&javascript).unwrap();

        // Verify the query has the expected capture count
        // The query should have captures for: func, test_name, body, test_call
        assert!(
            query.capture_names().len() > 0,
            "Query should have capture names"
        );
    }
}
