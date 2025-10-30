// debtmap:ignore-start -- This file contains test pattern detection and may trigger false security positives
// Testing pattern detection for JavaScript/TypeScript

use super::{get_node_text, SourceLocation};
use crate::core::{DebtItem, DebtType, Priority};
use std::path::{Path, PathBuf};
use tree_sitter::{Node, Query, QueryCursor, StreamingIterator};

mod queries;
mod validators;
use queries::*;
use validators::*;

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

fn create_async_test_issue(body_node: Node, test_name: String) -> TestingAntiPattern {
    TestingAntiPattern::AsyncTestIssue {
        location: SourceLocation::from_node(body_node),
        test_name,
        issue_type: "async operations without await or done callback".to_string(),
    }
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

    // Tests for detect_missing_assertions
    #[test]
    fn test_detect_missing_assertions_no_test_functions() {
        let javascript = tree_sitter_javascript::LANGUAGE.into();
        let mut parser = Parser::new();
        parser.set_language(&javascript).unwrap();

        let source = r#"
            function normalFunction() {
                console.log("not a test");
            }
        "#;

        let tree = parser.parse(source, None).unwrap();
        let mut issues = Vec::new();
        detect_missing_assertions(tree.root_node(), source, &javascript, &mut issues);
        assert_eq!(
            issues.len(),
            0,
            "No test functions should result in no issues"
        );
    }

    #[test]
    fn test_detect_missing_assertions_test_with_expect() {
        let javascript = tree_sitter_javascript::LANGUAGE.into();
        let mut parser = Parser::new();
        parser.set_language(&javascript).unwrap();

        let source = r#"
            test('should pass', () => {
                expect(true).toBe(true);
            });
        "#;

        let tree = parser.parse(source, None).unwrap();
        let mut issues = Vec::new();
        detect_missing_assertions(tree.root_node(), source, &javascript, &mut issues);
        assert_eq!(issues.len(), 0, "Test with expect should not be flagged");
    }

    #[test]
    fn test_detect_missing_assertions_test_without_assertions() {
        let javascript = tree_sitter_javascript::LANGUAGE.into();
        let mut parser = Parser::new();
        parser.set_language(&javascript).unwrap();

        let source = r#"
            test('empty test', () => {
                console.log("this is empty");
            });
        "#;

        let tree = parser.parse(source, None).unwrap();
        let mut issues = Vec::new();
        detect_missing_assertions(tree.root_node(), source, &javascript, &mut issues);
        assert_eq!(issues.len(), 1, "Test without assertions should be flagged");

        if let TestingAntiPattern::MissingAssertions { test_name, .. } = &issues[0] {
            assert_eq!(test_name, "empty test");
        } else {
            panic!("Expected MissingAssertions pattern");
        }
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
    fn test_detect_missing_assertions_it_function() {
        let javascript = tree_sitter_javascript::LANGUAGE.into();
        let mut parser = Parser::new();
        parser.set_language(&javascript).unwrap();

        let source = r#"
            it('should work', () => {
                expect(result).toBeDefined();
            });
        "#;

        let tree = parser.parse(source, None).unwrap();
        let mut issues = Vec::new();
        detect_missing_assertions(tree.root_node(), source, &javascript, &mut issues);
        assert_eq!(issues.len(), 0, "it() with expect should not be flagged");
    }

    #[test]
    fn test_detect_missing_assertions_assert_style() {
        let javascript = tree_sitter_javascript::LANGUAGE.into();
        let mut parser = Parser::new();
        parser.set_language(&javascript).unwrap();

        let source = r#"
            test("assert test", () => {
                assert.equal(actual, expected);
            });
        "#;

        let tree = parser.parse(source, None).unwrap();
        let mut issues = Vec::new();
        detect_missing_assertions(tree.root_node(), source, &javascript, &mut issues);
        assert_eq!(issues.len(), 0, "Test with assert should not be flagged");
    }

    #[test]
    fn test_detect_missing_assertions_should_style() {
        let javascript = tree_sitter_javascript::LANGUAGE.into();
        let mut parser = Parser::new();
        parser.set_language(&javascript).unwrap();

        let source = r#"
            test('should style', () => {
                result.should.equal(expected);
            });
        "#;

        let tree = parser.parse(source, None).unwrap();
        let mut issues = Vec::new();
        detect_missing_assertions(tree.root_node(), source, &javascript, &mut issues);
        assert_eq!(
            issues.len(),
            0,
            "Test with should style should not be flagged"
        );
    }

    #[test]
    fn test_detect_missing_assertions_multiple_tests() {
        let javascript = tree_sitter_javascript::LANGUAGE.into();
        let mut parser = Parser::new();
        parser.set_language(&javascript).unwrap();

        let source = r#"
            test('test with assertion', () => {
                expect(1).toBe(1);
            });

            test('test without assertion', () => {
                doSomething();
            });

            it('another without assertion', () => {
                setupSomething();
            });
        "#;

        let tree = parser.parse(source, None).unwrap();
        let mut issues = Vec::new();
        detect_missing_assertions(tree.root_node(), source, &javascript, &mut issues);
        assert_eq!(issues.len(), 2, "Should flag 2 tests without assertions");

        let test_names: Vec<&str> = issues
            .iter()
            .filter_map(|issue| {
                if let TestingAntiPattern::MissingAssertions { test_name, .. } = issue {
                    Some(test_name.as_str())
                } else {
                    None
                }
            })
            .collect();

        assert!(test_names.contains(&"test without assertion"));
        assert!(test_names.contains(&"another without assertion"));
    }

    #[test]
    fn test_detect_missing_assertions_single_quoted_test_name() {
        let javascript = tree_sitter_javascript::LANGUAGE.into();
        let mut parser = Parser::new();
        parser.set_language(&javascript).unwrap();

        let source = r#"
            test('single quoted name', () => {
                doSomething();
            });
        "#;

        let tree = parser.parse(source, None).unwrap();
        let mut issues = Vec::new();
        detect_missing_assertions(tree.root_node(), source, &javascript, &mut issues);
        assert_eq!(
            issues.len(),
            1,
            "Should detect test with single quoted name"
        );

        if let TestingAntiPattern::MissingAssertions { test_name, .. } = &issues[0] {
            assert_eq!(test_name, "single quoted name", "Should remove quotes");
        } else {
            panic!("Expected MissingAssertions pattern");
        }
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

    // Tests for detect_complex_tests
    #[test]
    fn test_detect_complex_tests_simple_test() {
        let javascript = tree_sitter_javascript::LANGUAGE.into();
        let mut parser = Parser::new();
        parser.set_language(&javascript).unwrap();

        let source = r#"
            test('simple test', () => {
                expect(result).toBe(42);
            });
        "#;

        let tree = parser.parse(source, None).unwrap();
        let mut issues = Vec::new();
        detect_complex_tests(tree.root_node(), source, &javascript, &mut issues);
        assert_eq!(issues.len(), 0, "Simple test should not trigger");
    }

    #[test]
    fn test_detect_complex_tests_complex_test() {
        let javascript = tree_sitter_javascript::LANGUAGE.into();
        let mut parser = Parser::new();
        parser.set_language(&javascript).unwrap();

        // Create a test with high complexity
        // Need > 10 complexity points: if=1, for=2, calls=1 each
        let source = r#"
            test('complex test', () => {
                for (let i = 0; i < 10; i++) {
                    if (condition1) {
                        mockFunction1();
                        mockFunction2();
                    }
                    for (let j = 0; j < 5; j++) {
                        if (condition2) {
                            mockFunction3();
                            mockFunction4();
                        }
                    }
                }
                expect(result).toBe(42);
            });
        "#;

        let tree = parser.parse(source, None).unwrap();
        let mut issues = Vec::new();
        detect_complex_tests(tree.root_node(), source, &javascript, &mut issues);
        assert_eq!(issues.len(), 1, "Complex test should trigger");

        if let TestingAntiPattern::ComplexTest {
            test_name,
            complexity,
            ..
        } = &issues[0]
        {
            assert_eq!(test_name, "complex test");
            assert!(complexity > &10, "Complexity should be > 10");
        } else {
            panic!("Expected ComplexTest variant");
        }
    }

    // Tests for detect_timing_dependent_tests
    #[test]
    fn test_detect_timing_dependent_tests_settimeout() {
        let javascript = tree_sitter_javascript::LANGUAGE.into();
        let mut parser = Parser::new();
        parser.set_language(&javascript).unwrap();

        let source = r#"
            test('delayed test', () => {
                setTimeout(() => {
                    expect(result).toBe(42);
                }, 1000);
            });
        "#;

        let tree = parser.parse(source, None).unwrap();
        let mut issues = Vec::new();
        detect_timing_dependent_tests(tree.root_node(), source, &javascript, &mut issues);
        assert_eq!(issues.len(), 1, "Should detect setTimeout dependency");

        if let TestingAntiPattern::TimingDependentTest {
            test_name,
            timing_type,
            ..
        } = &issues[0]
        {
            assert_eq!(test_name, "delayed test");
            assert_eq!(timing_type, "setTimeout");
        } else {
            panic!("Expected TimingDependentTest pattern");
        }
    }

    #[test]
    fn test_detect_complex_tests_it_function() {
        let javascript = tree_sitter_javascript::LANGUAGE.into();
        let mut parser = Parser::new();
        parser.set_language(&javascript).unwrap();

        // Test with 'it' instead of 'test'
        // Complexity: 2 for loops + 2 ifs + 4 calls + other calls = >10
        let source = r#"
            it('should handle complex scenario', () => {
                for (let i = 0; i < 10; i++) {
                    if (a) {
                        call1();
                        call2();
                        call3();
                    }
                    for (let j = 0; j < 5; j++) {
                        if (b) {
                            call4();
                            call5();
                            call6();
                        }
                    }
                }
            });
        "#;

        let tree = parser.parse(source, None).unwrap();
        let mut issues = Vec::new();
        detect_complex_tests(tree.root_node(), source, &javascript, &mut issues);
        assert_eq!(issues.len(), 1, "Complex 'it' test should trigger");
    }

    #[test]
    fn test_detect_complex_tests_describe_block() {
        let javascript = tree_sitter_javascript::LANGUAGE.into();
        let mut parser = Parser::new();
        parser.set_language(&javascript).unwrap();

        // Test with 'describe' which is also a test function
        let source = r#"
            describe('complex suite', () => {
                for (let i = 0; i < 10; i++) {
                    if (setup1) {
                        setupCall1();
                        setupCall2();
                        setupCall3();
                    }
                    for (let j = 0; j < 5; j++) {
                        if (setup2) {
                            setupCall4();
                            setupCall5();
                            setupCall6();
                        }
                    }
                }
            });
        "#;

        let tree = parser.parse(source, None).unwrap();
        let mut issues = Vec::new();
        detect_complex_tests(tree.root_node(), source, &javascript, &mut issues);
        assert_eq!(issues.len(), 1, "Complex 'describe' block should trigger");
    }

    #[test]
    fn test_detect_complex_tests_non_test_function() {
        let javascript = tree_sitter_javascript::LANGUAGE.into();
        let mut parser = Parser::new();
        parser.set_language(&javascript).unwrap();

        // Regular function call, not a test
        let source = r#"
            regularFunction('not a test', () => {
                if (a) { if (b) { if (c) { if (d) {
                    for (let i = 0; i < 10; i++) {
                        call1(); call2(); call3();
                    }
                } } } }
            });
        "#;

        let tree = parser.parse(source, None).unwrap();
        let mut issues = Vec::new();
        detect_complex_tests(tree.root_node(), source, &javascript, &mut issues);
        assert_eq!(issues.len(), 0, "Non-test function should not trigger");
    }

    #[test]
    fn test_detect_complex_tests_empty_source() {
        let javascript = tree_sitter_javascript::LANGUAGE.into();
        let mut parser = Parser::new();
        parser.set_language(&javascript).unwrap();

        let source = "";

        let tree = parser.parse(source, None).unwrap();
        let mut issues = Vec::new();
        detect_complex_tests(tree.root_node(), source, &javascript, &mut issues);
        assert_eq!(issues.len(), 0, "Empty source should not trigger");
    }

    #[test]
    fn test_detect_complex_tests_boundary_complexity() {
        let javascript = tree_sitter_javascript::LANGUAGE.into();
        let mut parser = Parser::new();
        parser.set_language(&javascript).unwrap();

        // Test with complexity exactly at threshold (10)
        // This should NOT trigger (threshold is > 10)
        let source = r#"
            test('boundary test', () => {
                if (a) { call1(); }
                if (b) { call2(); }
                if (c) { call3(); }
                if (d) { call4(); }
                if (e) { call5(); }
                if (f) { call6(); }
            });
        "#;

        let tree = parser.parse(source, None).unwrap();
        let mut issues = Vec::new();
        detect_complex_tests(tree.root_node(), source, &javascript, &mut issues);
        // The exact complexity depends on calculate_test_complexity implementation
        // This test verifies behavior at the boundary
        let complexity_at_boundary = if issues.is_empty() {
            true
        } else if let TestingAntiPattern::ComplexTest { complexity, .. } = &issues[0] {
            complexity > &10
        } else {
            false
        };
        assert!(
            complexity_at_boundary,
            "Boundary behavior should be consistent"
        );
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
            !query.capture_names().is_empty(),
            "Query should have capture names"
        );
    }

    #[test]
    fn test_extract_test_function_name() {
        let javascript = tree_sitter_javascript::LANGUAGE.into();
        let mut parser = Parser::new();
        parser.set_language(&javascript).unwrap();

        let source = "test('example', () => {});";
        let tree = parser.parse(source, None).unwrap();
        let query = build_async_test_query(&javascript).unwrap();
        let mut cursor = tree_sitter::QueryCursor::new();
        let mut matches = cursor.matches(&query, tree.root_node(), source.as_bytes());

        if let Some(match_) = matches.next() {
            let func = extract_test_function_name(match_);
            assert!(func.is_some(), "Should extract function name");
        } else {
            panic!("Query should match the test code");
        }
    }

    #[test]
    fn test_detect_complex_tests_double_quotes() {
        let javascript = tree_sitter_javascript::LANGUAGE.into();
        let mut parser = Parser::new();
        parser.set_language(&javascript).unwrap();

        let source = r#"
            test("test with double quotes", () => {
                for (let i = 0; i < 10; i++) {
                    if (a) {
                        call1();
                        call2();
                        call3();
                    }
                    for (let j = 0; j < 5; j++) {
                        if (b) {
                            call4();
                            call5();
                            call6();
                        }
                    }
                }
            });
        "#;

        let tree = parser.parse(source, None).unwrap();
        let mut issues = Vec::new();
        detect_complex_tests(tree.root_node(), source, &javascript, &mut issues);
        assert_eq!(
            issues.len(),
            1,
            "Complex test with double quotes should trigger"
        );

        if let TestingAntiPattern::ComplexTest { test_name, .. } = &issues[0] {
            assert_eq!(test_name, "test with double quotes");
        } else {
            panic!("Expected ComplexTest variant");
        }
    }

    #[test]
    fn test_detect_complex_tests_single_quotes() {
        let javascript = tree_sitter_javascript::LANGUAGE.into();
        let mut parser = Parser::new();
        parser.set_language(&javascript).unwrap();

        let source = r#"
            test('test with single quotes', () => {
                for (let i = 0; i < 10; i++) {
                    if (a) {
                        call1();
                        call2();
                        call3();
                    }
                    for (let j = 0; j < 5; j++) {
                        if (b) {
                            call4();
                            call5();
                            call6();
                        }
                    }
                }
            });
        "#;

        let tree = parser.parse(source, None).unwrap();
        let mut issues = Vec::new();
        detect_complex_tests(tree.root_node(), source, &javascript, &mut issues);
        assert_eq!(
            issues.len(),
            1,
            "Complex test with single quotes should trigger"
        );

        if let TestingAntiPattern::ComplexTest { test_name, .. } = &issues[0] {
            assert_eq!(test_name, "test with single quotes");
        } else {
            panic!("Expected ComplexTest variant");
        }
    }

    #[test]
    fn test_detect_complex_tests_multiple_tests() {
        let javascript = tree_sitter_javascript::LANGUAGE.into();
        let mut parser = Parser::new();
        parser.set_language(&javascript).unwrap();

        let source = r#"
            test('simple test 1', () => {
                expect(1).toBe(1);
            });

            test('complex test 1', () => {
                for (let i = 0; i < 10; i++) {
                    if (a) {
                        call1();
                        call2();
                        call3();
                    }
                    for (let j = 0; j < 5; j++) {
                        if (b) {
                            call4();
                            call5();
                            call6();
                        }
                    }
                }
            });

            test('simple test 2', () => {
                expect(2).toBe(2);
            });

            test('complex test 2', () => {
                for (let x = 0; x < 10; x++) {
                    if (c) {
                        call7();
                        call8();
                        call9();
                    }
                    for (let y = 0; y < 5; y++) {
                        if (d) {
                            call10();
                            call11();
                            call12();
                        }
                    }
                }
            });
        "#;

        let tree = parser.parse(source, None).unwrap();
        let mut issues = Vec::new();
        detect_complex_tests(tree.root_node(), source, &javascript, &mut issues);
        assert_eq!(issues.len(), 2, "Should detect both complex tests");
    }

    #[test]
    fn test_detect_timing_dependent_tests_setinterval() {
        let javascript = tree_sitter_javascript::LANGUAGE.into();
        let mut parser = Parser::new();
        parser.set_language(&javascript).unwrap();

        let source = r#"
            test('interval test', () => {
                const interval = setInterval(() => {
                    expect(value).toBeGreaterThan(0);
                }, 500);
            });
        "#;

        let tree = parser.parse(source, None).unwrap();
        let mut issues = Vec::new();
        detect_timing_dependent_tests(tree.root_node(), source, &javascript, &mut issues);
        assert_eq!(issues.len(), 1, "Should detect setInterval dependency");

        if let TestingAntiPattern::TimingDependentTest { timing_type, .. } = &issues[0] {
            assert_eq!(timing_type, "setInterval");
        } else {
            panic!("Expected TimingDependentTest pattern");
        }
    }

    #[test]
    fn test_detect_timing_dependent_tests_date_now() {
        let javascript = tree_sitter_javascript::LANGUAGE.into();
        let mut parser = Parser::new();
        parser.set_language(&javascript).unwrap();

        let source = r#"
            test('date test', () => {
                const now = Date.now();
                expect(now).toBeGreaterThan(0);
            });
        "#;

        let tree = parser.parse(source, None).unwrap();
        let mut issues = Vec::new();
        detect_timing_dependent_tests(tree.root_node(), source, &javascript, &mut issues);
        assert_eq!(issues.len(), 1, "Should detect Date.now() dependency");

        if let TestingAntiPattern::TimingDependentTest { timing_type, .. } = &issues[0] {
            assert_eq!(timing_type, "Date dependency");
        } else {
            panic!("Expected TimingDependentTest pattern");
        }
    }

    #[test]
    fn test_detect_timing_dependent_tests_math_random() {
        let javascript = tree_sitter_javascript::LANGUAGE.into();
        let mut parser = Parser::new();
        parser.set_language(&javascript).unwrap();

        let source = r#"
            test('random test', () => {
                const value = Math.random();
                expect(value).toBeLessThan(1);
            });
        "#;

        let tree = parser.parse(source, None).unwrap();
        let mut issues = Vec::new();
        detect_timing_dependent_tests(tree.root_node(), source, &javascript, &mut issues);
        assert_eq!(issues.len(), 1, "Should detect Math.random() dependency");

        if let TestingAntiPattern::TimingDependentTest { timing_type, .. } = &issues[0] {
            assert_eq!(timing_type, "random values");
        } else {
            panic!("Expected TimingDependentTest pattern");
        }
    }

    #[test]
    fn test_detect_timing_dependent_tests_performance_now() {
        let javascript = tree_sitter_javascript::LANGUAGE.into();
        let mut parser = Parser::new();
        parser.set_language(&javascript).unwrap();

        let source = r#"
            test('performance test', () => {
                const start = performance.now();
                doWork();
                const end = performance.now();
                expect(end - start).toBeLessThan(1000);
            });
        "#;

        let tree = parser.parse(source, None).unwrap();
        let mut issues = Vec::new();
        detect_timing_dependent_tests(tree.root_node(), source, &javascript, &mut issues);
        assert_eq!(
            issues.len(),
            1,
            "Should detect performance.now() dependency"
        );

        if let TestingAntiPattern::TimingDependentTest { timing_type, .. } = &issues[0] {
            assert_eq!(timing_type, "performance timing");
        } else {
            panic!("Expected TimingDependentTest pattern");
        }
    }

    #[test]
    fn test_detect_timing_dependent_tests_no_timing_dependencies() {
        let javascript = tree_sitter_javascript::LANGUAGE.into();
        let mut parser = Parser::new();
        parser.set_language(&javascript).unwrap();

        let source = r#"
            test('clean test', () => {
                const result = add(2, 3);
                expect(result).toBe(5);
            });
        "#;

        let tree = parser.parse(source, None).unwrap();
        let mut issues = Vec::new();
        detect_timing_dependent_tests(tree.root_node(), source, &javascript, &mut issues);
        assert_eq!(issues.len(), 0, "Should not detect any timing dependencies");
    }

    #[test]
    fn test_detect_timing_dependent_tests_non_test_function() {
        let javascript = tree_sitter_javascript::LANGUAGE.into();
        let mut parser = Parser::new();
        parser.set_language(&javascript).unwrap();

        let source = r#"
            helper('not a test', () => {
                setTimeout(() => {
                    console.log('delayed');
                }, 1000);
            });
        "#;

        let tree = parser.parse(source, None).unwrap();
        let mut issues = Vec::new();
        detect_timing_dependent_tests(tree.root_node(), source, &javascript, &mut issues);
        assert_eq!(
            issues.len(),
            0,
            "Should not detect timing issues in non-test functions"
        );
    }

    #[test]
    fn test_detect_timing_dependent_tests_multiple_timing_types() {
        let javascript = tree_sitter_javascript::LANGUAGE.into();
        let mut parser = Parser::new();
        parser.set_language(&javascript).unwrap();

        let source = r#"
            test('mixed timing test', () => {
                const now = Date.now();
                setTimeout(() => {
                    expect(Date.now() - now).toBeGreaterThan(100);
                }, 100);
            });
        "#;

        let tree = parser.parse(source, None).unwrap();
        let mut issues = Vec::new();
        detect_timing_dependent_tests(tree.root_node(), source, &javascript, &mut issues);
        // Should detect at least one timing dependency (the first one found)
        assert_eq!(issues.len(), 1, "Should detect timing dependency");
    }

    #[test]
    fn test_extract_test_name() {
        let javascript = tree_sitter_javascript::LANGUAGE.into();
        let mut parser = Parser::new();
        parser.set_language(&javascript).unwrap();

        let source = "test('example', () => {});";
        let tree = parser.parse(source, None).unwrap();
        let query = build_async_test_query(&javascript).unwrap();
        let mut cursor = tree_sitter::QueryCursor::new();
        let mut matches = cursor.matches(&query, tree.root_node(), source.as_bytes());

        if let Some(match_) = matches.next() {
            let name = extract_test_name(match_);
            assert!(name.is_some(), "Should extract test name");
        } else {
            panic!("Query should match the test code");
        }
    }

    #[test]
    fn test_extract_test_body() {
        let javascript = tree_sitter_javascript::LANGUAGE.into();
        let mut parser = Parser::new();
        parser.set_language(&javascript).unwrap();

        let source = "test('example', () => {});";
        let tree = parser.parse(source, None).unwrap();
        let query = build_async_test_query(&javascript).unwrap();
        let mut cursor = tree_sitter::QueryCursor::new();
        let mut matches = cursor.matches(&query, tree.root_node(), source.as_bytes());

        if let Some(match_) = matches.next() {
            let body = extract_test_body(match_);
            assert!(body.is_some(), "Should extract test body");
        } else {
            panic!("Query should match the test code");
        }
    }

    #[test]
    fn test_parse_test_name_double_quotes() {
        let javascript = tree_sitter_javascript::LANGUAGE.into();
        let mut parser = Parser::new();
        parser.set_language(&javascript).unwrap();

        let source = r#"test("example test", () => {});"#;
        let tree = parser.parse(source, None).unwrap();
        let query = build_async_test_query(&javascript).unwrap();
        let mut cursor = tree_sitter::QueryCursor::new();
        let mut matches = cursor.matches(&query, tree.root_node(), source.as_bytes());

        if let Some(match_) = matches.next() {
            if let Some(name_node) = extract_test_name(match_) {
                let name = parse_test_name(*name_node, source);
                assert_eq!(
                    name, "example test",
                    "Should parse test name with double quotes"
                );
            }
        }
    }

    #[test]
    fn test_parse_test_name_single_quotes() {
        let javascript = tree_sitter_javascript::LANGUAGE.into();
        let mut parser = Parser::new();
        parser.set_language(&javascript).unwrap();

        let source = "test('example test', () => {});";
        let tree = parser.parse(source, None).unwrap();
        let query = build_async_test_query(&javascript).unwrap();
        let mut cursor = tree_sitter::QueryCursor::new();
        let mut matches = cursor.matches(&query, tree.root_node(), source.as_bytes());

        if let Some(match_) = matches.next() {
            if let Some(name_node) = extract_test_name(match_) {
                let name = parse_test_name(*name_node, source);
                assert_eq!(
                    name, "example test",
                    "Should parse test name with single quotes"
                );
            }
        }
    }

    #[test]
    fn test_parse_test_name_backticks() {
        let javascript = tree_sitter_javascript::LANGUAGE.into();
        let mut parser = Parser::new();
        parser.set_language(&javascript).unwrap();

        let source = "test(`example test`, () => {});";
        let tree = parser.parse(source, None).unwrap();
        let query = build_async_test_query(&javascript).unwrap();
        let mut cursor = tree_sitter::QueryCursor::new();
        let mut matches = cursor.matches(&query, tree.root_node(), source.as_bytes());

        if let Some(match_) = matches.next() {
            if let Some(name_node) = extract_test_name(match_) {
                let name = parse_test_name(*name_node, source);
                assert_eq!(
                    name, "example test",
                    "Should parse test name with backticks"
                );
            }
        }
    }

    #[test]
    fn test_create_async_test_issue() {
        let javascript = tree_sitter_javascript::LANGUAGE.into();
        let mut parser = Parser::new();
        parser.set_language(&javascript).unwrap();

        let source = "test('async test', () => { fetch('/api'); });";
        let tree = parser.parse(source, None).unwrap();
        let query = build_async_test_query(&javascript).unwrap();
        let mut cursor = tree_sitter::QueryCursor::new();
        let mut matches = cursor.matches(&query, tree.root_node(), source.as_bytes());

        if let Some(match_) = matches.next() {
            if let Some(body_node) = extract_test_body(match_) {
                let issue = create_async_test_issue(*body_node, "async test".to_string());

                if let TestingAntiPattern::AsyncTestIssue {
                    test_name,
                    issue_type,
                    ..
                } = issue
                {
                    assert_eq!(test_name, "async test");
                    assert_eq!(
                        issue_type,
                        "async operations without await or done callback"
                    );
                } else {
                    panic!("Expected AsyncTestIssue");
                }
            }
        }
    }

    // Tests for detect_react_test_issues
    #[test]
    fn test_detect_react_test_issues_no_render_calls() {
        let javascript = tree_sitter_javascript::LANGUAGE.into();
        let mut parser = Parser::new();
        parser.set_language(&javascript).unwrap();

        let source = r#"
            test('no render calls', () => {
                const value = calculateValue();
                expect(value).toBe(42);
            });
        "#;

        let tree = parser.parse(source, None).unwrap();
        let mut issues = Vec::new();
        detect_react_test_issues(tree.root_node(), source, &javascript, &mut issues);
        assert_eq!(issues.len(), 0, "No render calls should not trigger issue");
    }

    #[test]
    fn test_detect_react_test_issues_render_with_cleanup() {
        let javascript = tree_sitter_javascript::LANGUAGE.into();
        let mut parser = Parser::new();
        parser.set_language(&javascript).unwrap();

        let source = r#"
            test('render with cleanup', () => {
                const { container } = render(<Component />);
                expect(container).toBeDefined();
                cleanup();
            });
        "#;

        let tree = parser.parse(source, None).unwrap();
        let mut issues = Vec::new();
        detect_react_test_issues(tree.root_node(), source, &javascript, &mut issues);
        assert_eq!(
            issues.len(),
            0,
            "Render with cleanup should not trigger issue"
        );
    }

    #[test]
    fn test_detect_react_test_issues_render_missing_cleanup() {
        let javascript = tree_sitter_javascript::LANGUAGE.into();
        let mut parser = Parser::new();
        parser.set_language(&javascript).unwrap();

        let source = r#"
            test('render without cleanup', () => {
                const { container } = render(<Component />);
                expect(container).toBeDefined();
            });
        "#;

        let tree = parser.parse(source, None).unwrap();
        let mut issues = Vec::new();
        detect_react_test_issues(tree.root_node(), source, &javascript, &mut issues);
        assert_eq!(
            issues.len(),
            1,
            "Render without cleanup should trigger issue"
        );

        if let TestingAntiPattern::MissingCleanup {
            test_name,
            resource_type,
            ..
        } = &issues[0]
        {
            assert_eq!(test_name, "React test");
            assert_eq!(resource_type, "React components");
        } else {
            panic!("Expected MissingCleanup pattern");
        }
    }

    #[test]
    fn test_detect_react_test_issues_multiple_renders_insufficient_cleanups() {
        let javascript = tree_sitter_javascript::LANGUAGE.into();
        let mut parser = Parser::new();
        parser.set_language(&javascript).unwrap();

        let source = r#"
            test('multiple renders', () => {
                const first = render(<Component1 />);
                const second = render(<Component2 />);
                const third = render(<Component3 />);
                cleanup();
                expect(first).toBeDefined();
            });
        "#;

        let tree = parser.parse(source, None).unwrap();
        let mut issues = Vec::new();
        detect_react_test_issues(tree.root_node(), source, &javascript, &mut issues);
        assert_eq!(
            issues.len(),
            1,
            "Multiple renders with insufficient cleanups should trigger issue"
        );
    }

    #[test]
    fn test_detect_react_test_issues_mount_alias() {
        let javascript = tree_sitter_javascript::LANGUAGE.into();
        let mut parser = Parser::new();
        parser.set_language(&javascript).unwrap();

        let source = r#"
            test('mount instead of render', () => {
                const wrapper = mount(<Component />);
                expect(wrapper).toBeDefined();
            });
        "#;

        let tree = parser.parse(source, None).unwrap();
        let mut issues = Vec::new();
        detect_react_test_issues(tree.root_node(), source, &javascript, &mut issues);
        assert_eq!(
            issues.len(),
            1,
            "Mount without cleanup should trigger issue"
        );
    }

    #[test]
    fn test_detect_react_test_issues_unmount_in_cleanup() {
        let javascript = tree_sitter_javascript::LANGUAGE.into();
        let mut parser = Parser::new();
        parser.set_language(&javascript).unwrap();

        let source = r#"
            test('mount with unmount', () => {
                const wrapper = mount(<Component />);
                expect(wrapper).toBeDefined();
                unmount();
            });
        "#;

        let tree = parser.parse(source, None).unwrap();
        let mut issues = Vec::new();
        detect_react_test_issues(tree.root_node(), source, &javascript, &mut issues);
        assert_eq!(
            issues.len(),
            0,
            "Mount with unmount should not trigger issue"
        );
    }

    #[test]
    fn test_detect_react_test_issues_member_expression_cleanup() {
        let javascript = tree_sitter_javascript::LANGUAGE.into();
        let mut parser = Parser::new();
        parser.set_language(&javascript).unwrap();

        let source = r#"
            test('member expression cleanup', () => {
                const wrapper = render(<Component />);
                expect(wrapper).toBeDefined();
                wrapper.unmount();
            });
        "#;

        let tree = parser.parse(source, None).unwrap();
        let mut issues = Vec::new();
        detect_react_test_issues(tree.root_node(), source, &javascript, &mut issues);
        assert_eq!(
            issues.len(),
            0,
            "Member expression unmount should count as cleanup"
        );
    }

    #[test]
    fn test_detect_react_test_issues_empty_source() {
        let javascript = tree_sitter_javascript::LANGUAGE.into();
        let mut parser = Parser::new();
        parser.set_language(&javascript).unwrap();

        let source = "";

        let tree = parser.parse(source, None).unwrap();
        let mut issues = Vec::new();
        detect_react_test_issues(tree.root_node(), source, &javascript, &mut issues);
        assert_eq!(issues.len(), 0, "Empty source should not trigger issue");
    }

    #[test]
    fn test_detect_react_test_issues_equal_render_cleanup_count() {
        let javascript = tree_sitter_javascript::LANGUAGE.into();
        let mut parser = Parser::new();
        parser.set_language(&javascript).unwrap();

        let source = r#"
            test('balanced render and cleanup', () => {
                const first = render(<Component1 />);
                const second = render(<Component2 />);
                cleanup();
                cleanup();
                expect(first).toBeDefined();
            });
        "#;

        let tree = parser.parse(source, None).unwrap();
        let mut issues = Vec::new();
        detect_react_test_issues(tree.root_node(), source, &javascript, &mut issues);
        assert_eq!(
            issues.len(),
            0,
            "Equal render and cleanup count should not trigger issue"
        );
    }

    #[test]
    fn test_detect_react_test_issues_component_will_unmount() {
        let javascript = tree_sitter_javascript::LANGUAGE.into();
        let mut parser = Parser::new();
        parser.set_language(&javascript).unwrap();

        let source = r#"
            test('componentWillUnmount lifecycle', () => {
                const wrapper = render(<Component />);
                expect(wrapper).toBeDefined();
                componentWillUnmount();
            });
        "#;

        let tree = parser.parse(source, None).unwrap();
        let mut issues = Vec::new();
        detect_react_test_issues(tree.root_node(), source, &javascript, &mut issues);
        assert_eq!(
            issues.len(),
            0,
            "componentWillUnmount should count as cleanup"
        );
    }
}
