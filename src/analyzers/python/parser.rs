//! Tree-sitter parser integration for Python
//!
//! Provides parsing using tree-sitter grammars for Python (.py, .pyw).

use crate::core::ast::PythonAst;
use anyhow::{Context, Result};
use std::path::Path;
use tree_sitter::{Language as TsLanguage, Parser, Tree};

/// Get the tree-sitter language for Python
fn get_language() -> TsLanguage {
    tree_sitter_python::LANGUAGE.into()
}

/// Parse Python source code into a tree-sitter AST
pub fn parse_source(content: &str, path: &Path) -> Result<PythonAst> {
    let mut parser = Parser::new();
    let language = get_language();

    parser
        .set_language(&language)
        .context("Failed to set tree-sitter language")?;

    let tree = parser
        .parse(content, None)
        .context("Failed to parse source code")?;

    if has_parse_errors(&tree) {
        anyhow::bail!("Python parse tree contains syntax errors");
    }

    Ok(PythonAst {
        tree,
        path: path.to_path_buf(),
        source: content.to_string(),
    })
}

/// Check if a parse tree has errors
pub fn has_parse_errors(tree: &Tree) -> bool {
    tree.root_node().has_error()
}

/// Get text for a tree-sitter node
pub fn node_text<'a>(node: &tree_sitter::Node, source: &'a str) -> &'a str {
    let start = node.start_byte();
    let end = node.end_byte();
    &source[start..end]
}

/// Get the line number for a tree-sitter node (1-indexed)
pub fn node_line(node: &tree_sitter::Node) -> usize {
    node.start_position().row + 1
}

/// Get the column number for a tree-sitter node (1-indexed)
pub fn node_column(node: &tree_sitter::Node) -> usize {
    node.start_position().column + 1
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_parse_python() {
        let source = "def hello():\n    return 'world'";
        let path = PathBuf::from("test.py");
        let result = parse_source(source, &path);
        assert!(result.is_ok());

        let ast = result.unwrap();
        assert!(!has_parse_errors(&ast.tree));
    }

    #[test]
    fn test_node_text() {
        let source = "x = 42";
        let path = PathBuf::from("test.py");
        let ast = parse_source(source, &path).unwrap();

        let root = ast.tree.root_node();
        let text = node_text(&root, &ast.source);
        assert_eq!(text, source);
    }

    #[test]
    fn test_node_line() {
        let source = "x = 42\ny = 24";
        let path = PathBuf::from("test.py");
        let ast = parse_source(source, &path).unwrap();

        let root = ast.tree.root_node();
        assert_eq!(node_line(&root), 1);
    }

    #[test]
    fn test_parse_invalid_python_fails() {
        let source = "def broken(:\n    pass";
        let path = PathBuf::from("broken.py");
        let result = parse_source(source, &path);
        assert!(result.is_err());
    }
}
