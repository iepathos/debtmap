//! Async/await pattern detection
//!
//! Detects async/await usage patterns including nested awaits.

use crate::analyzers::typescript::types::AsyncPattern;
use crate::core::ast::TypeScriptAst;
use tree_sitter::Node;

/// Detect async/await patterns in a function body
pub fn detect_async_patterns(node: &Node, _ast: &TypeScriptAst) -> Vec<AsyncPattern> {
    let mut patterns = Vec::new();

    let (await_count, nested_depth) = count_await_expressions(node, 0);

    if await_count > 0 {
        patterns.push(AsyncPattern::AsyncAwait {
            await_count,
            nested_await_depth: nested_depth,
        });
    }

    patterns
}

/// Count await expressions and track nesting depth
fn count_await_expressions(node: &Node, current_depth: u32) -> (u32, u32) {
    let mut total_count = 0u32;
    let mut max_depth = current_depth;

    if node.kind() == "await_expression" {
        total_count += 1;
        max_depth = max_depth.max(current_depth + 1);

        // Check for nested awaits within this await expression
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            let (child_count, child_depth) = count_await_expressions(&child, current_depth + 1);
            total_count += child_count;
            max_depth = max_depth.max(child_depth);
        }
    } else {
        // Not an await, recurse normally
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            let (child_count, child_depth) = count_await_expressions(&child, current_depth);
            total_count += child_count;
            max_depth = max_depth.max(child_depth);
        }
    }

    (total_count, max_depth)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzers::typescript::parser::parse_source;
    use crate::core::ast::JsLanguageVariant;
    use std::path::PathBuf;

    #[test]
    fn test_detect_single_await() {
        let source = "async function foo() { await bar(); }";
        let path = PathBuf::from("test.js");
        let ast = parse_source(source, &path, JsLanguageVariant::JavaScript).unwrap();

        let patterns = detect_async_patterns(&ast.tree.root_node(), &ast);

        assert_eq!(patterns.len(), 1);
        if let AsyncPattern::AsyncAwait {
            await_count,
            nested_await_depth: _,
        } = &patterns[0]
        {
            assert_eq!(*await_count, 1);
        }
    }

    #[test]
    fn test_detect_multiple_awaits() {
        let source = r#"
async function foo() {
    await bar();
    await baz();
    await qux();
}
"#;
        let path = PathBuf::from("test.js");
        let ast = parse_source(source, &path, JsLanguageVariant::JavaScript).unwrap();

        let patterns = detect_async_patterns(&ast.tree.root_node(), &ast);

        assert_eq!(patterns.len(), 1);
        if let AsyncPattern::AsyncAwait { await_count, .. } = &patterns[0] {
            assert_eq!(*await_count, 3);
        }
    }
}
