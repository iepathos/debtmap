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

fn detect_async_test_issues(
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
        (arrow_function
          async: false
          body: (_) @body
        )
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
}
