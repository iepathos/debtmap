//! Tree-sitter parser integration for JavaScript/TypeScript
//!
//! Provides parsing using tree-sitter grammars for JS/TS/JSX/TSX.

use crate::core::ast::{JsLanguageVariant, TypeScriptAst};
use anyhow::{Context, Result};
use std::path::Path;
use tree_sitter::{Language as TsLanguage, Parser, Tree};

/// Get the tree-sitter language for a JS variant
fn get_language(variant: JsLanguageVariant) -> TsLanguage {
    match variant {
        JsLanguageVariant::JavaScript | JsLanguageVariant::Jsx => {
            tree_sitter_javascript::LANGUAGE.into()
        }
        JsLanguageVariant::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
        JsLanguageVariant::Tsx => tree_sitter_typescript::LANGUAGE_TSX.into(),
    }
}

/// Parse JavaScript/TypeScript source code into a tree-sitter AST
pub fn parse_source(
    content: &str,
    path: &Path,
    variant: JsLanguageVariant,
) -> Result<TypeScriptAst> {
    let mut parser = Parser::new();
    let language = get_language(variant);

    parser
        .set_language(&language)
        .context("Failed to set tree-sitter language")?;

    let tree = parser
        .parse(content, None)
        .context("Failed to parse source code")?;

    Ok(TypeScriptAst {
        tree,
        path: path.to_path_buf(),
        source: content.to_string(),
        language_variant: variant,
    })
}

/// Determine language variant from file path
pub fn detect_variant(path: &Path) -> JsLanguageVariant {
    path.extension()
        .and_then(|ext| ext.to_str())
        .and_then(JsLanguageVariant::from_extension)
        .unwrap_or(JsLanguageVariant::JavaScript)
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
    fn test_detect_variant() {
        assert_eq!(
            detect_variant(Path::new("test.js")),
            JsLanguageVariant::JavaScript
        );
        assert_eq!(
            detect_variant(Path::new("test.mjs")),
            JsLanguageVariant::JavaScript
        );
        assert_eq!(
            detect_variant(Path::new("test.jsx")),
            JsLanguageVariant::Jsx
        );
        assert_eq!(
            detect_variant(Path::new("test.ts")),
            JsLanguageVariant::TypeScript
        );
        assert_eq!(
            detect_variant(Path::new("test.tsx")),
            JsLanguageVariant::Tsx
        );
    }

    #[test]
    fn test_parse_javascript() {
        let source = "function hello() { return 'world'; }";
        let path = PathBuf::from("test.js");
        let result = parse_source(source, &path, JsLanguageVariant::JavaScript);
        assert!(result.is_ok());

        let ast = result.unwrap();
        assert!(!has_parse_errors(&ast.tree));
        assert_eq!(ast.language_variant, JsLanguageVariant::JavaScript);
    }

    #[test]
    fn test_parse_typescript() {
        let source = "function hello(name: string): string { return `Hello ${name}`; }";
        let path = PathBuf::from("test.ts");
        let result = parse_source(source, &path, JsLanguageVariant::TypeScript);
        assert!(result.is_ok());

        let ast = result.unwrap();
        assert!(!has_parse_errors(&ast.tree));
        assert_eq!(ast.language_variant, JsLanguageVariant::TypeScript);
    }

    #[test]
    fn test_parse_jsx() {
        let source = "const App = () => <div>Hello</div>;";
        let path = PathBuf::from("test.jsx");
        let result = parse_source(source, &path, JsLanguageVariant::Jsx);
        assert!(result.is_ok());

        let ast = result.unwrap();
        // JSX is parsed by JavaScript parser
        assert_eq!(ast.language_variant, JsLanguageVariant::Jsx);
    }

    #[test]
    fn test_parse_tsx() {
        let source = "const App: React.FC = () => <div>Hello</div>;";
        let path = PathBuf::from("test.tsx");
        let result = parse_source(source, &path, JsLanguageVariant::Tsx);
        assert!(result.is_ok());

        let ast = result.unwrap();
        assert_eq!(ast.language_variant, JsLanguageVariant::Tsx);
    }

    #[test]
    fn test_node_text() {
        let source = "const x = 42;";
        let path = PathBuf::from("test.js");
        let ast = parse_source(source, &path, JsLanguageVariant::JavaScript).unwrap();

        let root = ast.tree.root_node();
        let text = node_text(&root, &ast.source);
        assert_eq!(text, source);
    }

    #[test]
    fn test_node_line() {
        let source = "const x = 42;\nconst y = 24;";
        let path = PathBuf::from("test.js");
        let ast = parse_source(source, &path, JsLanguageVariant::JavaScript).unwrap();

        let root = ast.tree.root_node();
        assert_eq!(node_line(&root), 1);
    }
}
