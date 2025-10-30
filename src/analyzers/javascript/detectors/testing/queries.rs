// Query building and node extraction utilities for testing pattern detection

use crate::analyzers::javascript::detectors::get_node_text;
use tree_sitter::{Node, Query, QueryError};

/// Builds a tree-sitter query for detecting async test patterns
///
/// This query identifies test function calls (e.g., `it()`, `test()`) that
/// contain arrow functions with async bodies.
pub(super) fn build_async_test_query(language: &tree_sitter::Language) -> Result<Query, QueryError> {
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

/// Extracts the test function name node from a query match (e.g., "it", "test", "describe")
///
/// The function name is captured at index 0 in the query.
pub(super) fn extract_test_function_name<'a>(
    match_: &tree_sitter::QueryMatch<'a, '_>,
) -> Option<&'a Node<'a>> {
    match_
        .captures
        .iter()
        .find(|c| c.index == 0)
        .map(|c| &c.node)
}

/// Extracts the test name string node from a query match
///
/// The test name is captured at index 1 in the query.
pub(super) fn extract_test_name<'a>(match_: &tree_sitter::QueryMatch<'a, '_>) -> Option<&'a Node<'a>> {
    match_
        .captures
        .iter()
        .find(|c| c.index == 1)
        .map(|c| &c.node)
}

/// Extracts the test body node from a query match
///
/// The test body is captured at index 2 in the query.
pub(super) fn extract_test_body<'a>(match_: &tree_sitter::QueryMatch<'a, '_>) -> Option<&'a Node<'a>> {
    match_
        .captures
        .iter()
        .find(|c| c.index == 2)
        .map(|c| &c.node)
}

/// Parses a test name from a string literal node
///
/// Removes surrounding quotes (single, double, or backtick) from test name strings.
pub(super) fn parse_test_name(node: Node, source: &str) -> String {
    get_node_text(node, source)
        .trim_matches('"')
        .trim_matches('\'')
        .trim_matches('`')
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tree_sitter::Parser;

    #[test]
    fn test_build_async_test_query() {
        let language = tree_sitter_javascript::LANGUAGE.into();
        let query = build_async_test_query(&language);
        assert!(query.is_ok());
    }

    #[test]
    fn test_parse_test_name_double_quotes() {
        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_javascript::LANGUAGE.into()).unwrap();
        let source = r#""my test""#;
        let tree = parser.parse(source, None).unwrap();
        let root = tree.root_node();
        let result = parse_test_name(root, source);
        assert_eq!(result, "my test");
    }

    #[test]
    fn test_parse_test_name_single_quotes() {
        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_javascript::LANGUAGE.into()).unwrap();
        let source = r#"'my test'"#;
        let tree = parser.parse(source, None).unwrap();
        let root = tree.root_node();
        let result = parse_test_name(root, source);
        assert_eq!(result, "my test");
    }

    #[test]
    fn test_parse_test_name_backticks() {
        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_javascript::LANGUAGE.into()).unwrap();
        let source = r#"`my test`"#;
        let tree = parser.parse(source, None).unwrap();
        let root = tree.root_node();
        let result = parse_test_name(root, source);
        assert_eq!(result, "my test");
    }
}
