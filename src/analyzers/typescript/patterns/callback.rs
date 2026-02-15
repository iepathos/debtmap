//! Callback pattern detection
//!
//! Detects callback patterns including callback nesting depth.

use crate::analyzers::typescript::types::AsyncPattern;
use crate::core::ast::TypeScriptAst;
use tree_sitter::Node;

/// Detect callback patterns
pub fn detect_callback_patterns(node: &Node, _ast: &TypeScriptAst) -> Vec<AsyncPattern> {
    let mut patterns = Vec::new();

    let (depth, count) = analyze_callback_nesting(node, 0);

    if depth > 1 || count > 2 {
        patterns.push(AsyncPattern::CallbackNesting {
            depth,
            callback_count: count,
        });
    }

    patterns
}

/// Analyze callback nesting within a node
fn analyze_callback_nesting(node: &Node, current_depth: u32) -> (u32, u32) {
    let mut max_depth = current_depth;
    let mut total_callbacks = 0u32;

    let new_depth = if is_callback_function(node) {
        total_callbacks += 1;
        current_depth + 1
    } else {
        current_depth
    };

    max_depth = max_depth.max(new_depth);

    // Recurse into children
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        let (child_depth, child_count) = analyze_callback_nesting(&child, new_depth);
        max_depth = max_depth.max(child_depth);
        total_callbacks += child_count;
    }

    (max_depth, total_callbacks)
}

/// Check if a node is a callback function
fn is_callback_function(node: &Node) -> bool {
    // A callback is a function that appears as an argument to another function call
    if !matches!(
        node.kind(),
        "arrow_function" | "function_expression" | "function"
    ) {
        return false;
    }

    // Check if parent is arguments of a call expression
    if let Some(parent) = node.parent() {
        if parent.kind() == "arguments" {
            return true;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzers::typescript::parser::parse_source;
    use crate::core::ast::JsLanguageVariant;
    use std::path::PathBuf;

    #[test]
    fn test_detect_callback_nesting() {
        let source = r#"
getData(function(a) {
    processA(a, function(b) {
        processB(b);
    });
});
"#;
        let path = PathBuf::from("test.js");
        let ast = parse_source(source, &path, JsLanguageVariant::JavaScript).unwrap();

        let patterns = detect_callback_patterns(&ast.tree.root_node(), &ast);

        assert!(!patterns.is_empty());
        if let AsyncPattern::CallbackNesting {
            depth,
            callback_count,
        } = &patterns[0]
        {
            assert!(*depth >= 2);
            assert!(*callback_count >= 2);
        }
    }

    #[test]
    fn test_no_callback_for_simple_function() {
        let source = "const fn = () => 42;";
        let path = PathBuf::from("test.js");
        let ast = parse_source(source, &path, JsLanguageVariant::JavaScript).unwrap();

        let patterns = detect_callback_patterns(&ast.tree.root_node(), &ast);

        // Simple arrow function should not be detected as callback pattern
        assert!(patterns.is_empty());
    }
}
