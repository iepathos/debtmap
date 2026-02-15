//! Async pattern debt detection
//!
//! Detects problematic async patterns like callback hell, unhandled promises, etc.

use crate::analyzers::typescript::parser::{node_line, node_text};
use crate::analyzers::typescript::types::JsFunctionMetrics;
use crate::core::ast::TypeScriptAst;
use crate::core::{DebtItem, Priority};
use crate::priority::DebtType;
use tree_sitter::Node;

/// Detect async-related debt patterns
pub fn detect_async_debt(
    ast: &TypeScriptAst,
    _js_functions: &[JsFunctionMetrics],
) -> Vec<DebtItem> {
    let mut items = Vec::new();
    let root = ast.tree.root_node();

    // Detect callback hell
    items.extend(detect_callback_hell(&root, ast));

    // Detect unhandled promises
    items.extend(detect_unhandled_promises(&root, ast));

    // Detect long promise chains
    items.extend(detect_long_promise_chains(&root, ast));

    items
}

/// Detect deeply nested callbacks (callback hell)
fn detect_callback_hell(node: &Node, ast: &TypeScriptAst) -> Vec<DebtItem> {
    let mut items = Vec::new();
    detect_callback_hell_recursive(node, ast, 0, &mut items);
    items
}

fn detect_callback_hell_recursive(
    node: &Node,
    ast: &TypeScriptAst,
    depth: u32,
    items: &mut Vec<DebtItem>,
) {
    let new_depth = match node.kind() {
        "arrow_function" | "function_expression" | "function" => depth + 1,
        _ => depth,
    };

    // Report callback hell if depth exceeds 3
    if new_depth > 3 && matches!(node.kind(), "arrow_function" | "function_expression") {
        items.push(DebtItem {
            id: format!(
                "js-callback-hell-{}-{}",
                ast.path.display(),
                node_line(node)
            ),
            debt_type: DebtType::AsyncMisuse {
                pattern: "callback_hell".to_string(),
                performance_impact: "Reduces readability and maintainability".to_string(),
            },
            priority: Priority::High,
            file: ast.path.clone(),
            line: node_line(node),
            column: None,
            message: format!("Callback hell detected ({} levels deep)", new_depth),
            context: Some(
                "Deeply nested callbacks make code hard to read and maintain. \
                 Consider using async/await, Promise.all, or breaking into smaller functions."
                    .to_string(),
            ),
        });
    }

    // Recurse into children
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        detect_callback_hell_recursive(&child, ast, new_depth, items);
    }
}

/// Detect promise chains without error handling
fn detect_unhandled_promises(node: &Node, ast: &TypeScriptAst) -> Vec<DebtItem> {
    let mut items = Vec::new();
    detect_unhandled_promises_recursive(node, ast, &mut items);
    items
}

fn detect_unhandled_promises_recursive(
    node: &Node,
    ast: &TypeScriptAst,
    items: &mut Vec<DebtItem>,
) {
    // Look for .then() calls
    if node.kind() == "call_expression" {
        if is_promise_then_call(node, ast) {
            // Check if there's a .catch() somewhere in the chain
            if !has_error_handling_in_chain(node, ast) {
                items.push(DebtItem {
                    id: format!(
                        "js-unhandled-promise-{}-{}",
                        ast.path.display(),
                        node_line(node)
                    ),
                    debt_type: DebtType::ErrorSwallowing {
                        pattern: "unhandled_promise".to_string(),
                        context: Some("Promise chain without .catch() or try/catch".to_string()),
                    },
                    priority: Priority::High,
                    file: ast.path.clone(),
                    line: node_line(node),
                    column: None,
                    message: "Promise chain without error handling".to_string(),
                    context: Some(
                        "Promise chains should have a .catch() handler or be wrapped in try/catch \
                         when using async/await to handle potential rejections."
                            .to_string(),
                    ),
                });
            }
        }
    }

    // Recurse
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        detect_unhandled_promises_recursive(&child, ast, items);
    }
}

fn is_promise_then_call(node: &Node, ast: &TypeScriptAst) -> bool {
    if node.kind() != "call_expression" {
        return false;
    }

    if let Some(func) = node.child_by_field_name("function") {
        if func.kind() == "member_expression" {
            if let Some(prop) = func.child_by_field_name("property") {
                let prop_text = node_text(&prop, &ast.source);
                return prop_text == "then";
            }
        }
    }

    false
}

fn has_error_handling_in_chain(node: &Node, ast: &TypeScriptAst) -> bool {
    // Check ancestors for .catch() or .finally() calls
    // In promise chains, error handling wraps the entire chain
    let mut current = node.parent();
    while let Some(parent) = current {
        if parent.kind() == "call_expression" {
            if let Some(func) = parent.child_by_field_name("function") {
                if func.kind() == "member_expression" {
                    if let Some(prop) = func.child_by_field_name("property") {
                        let prop_text = node_text(&prop, &ast.source);
                        if prop_text == "catch" || prop_text == "finally" {
                            return true;
                        }
                    }
                }
            }
        }
        current = parent.parent();
    }
    false
}

/// Detect overly long promise chains
fn detect_long_promise_chains(node: &Node, ast: &TypeScriptAst) -> Vec<DebtItem> {
    let mut items = Vec::new();
    detect_long_chains_recursive(node, ast, &mut items);
    items
}

fn detect_long_chains_recursive(node: &Node, ast: &TypeScriptAst, items: &mut Vec<DebtItem>) {
    if node.kind() == "call_expression" {
        let chain_length = count_promise_chain_length(node, ast);
        if chain_length > 4 {
            items.push(DebtItem {
                id: format!(
                    "js-long-promise-chain-{}-{}",
                    ast.path.display(),
                    node_line(node)
                ),
                debt_type: DebtType::AsyncMisuse {
                    pattern: "long_promise_chain".to_string(),
                    performance_impact: format!(
                        "Chain of {} methods reduces readability",
                        chain_length
                    ),
                },
                priority: Priority::Medium,
                file: ast.path.clone(),
                line: node_line(node),
                column: None,
                message: format!("Long promise chain ({} methods)", chain_length),
                context: Some(
                    "Long promise chains are hard to read and debug. Consider using async/await \
                     or breaking the chain into smaller, named functions."
                        .to_string(),
                ),
            });
        }
    }

    // Recurse
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        detect_long_chains_recursive(&child, ast, items);
    }
}

fn count_promise_chain_length(node: &Node, ast: &TypeScriptAst) -> u32 {
    let text = node_text(node, &ast.source);

    // Count method calls that are typically chained
    let chain_methods = [
        ".then(",
        ".catch(",
        ".finally(",
        ".map(",
        ".filter(",
        ".reduce(",
    ];

    chain_methods
        .iter()
        .map(|m| text.matches(m).count() as u32)
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzers::typescript::parser::parse_source;
    use crate::core::ast::JsLanguageVariant;
    use std::path::PathBuf;

    #[test]
    fn test_detect_callback_hell() {
        let source = r#"
getData(function(a) {
    processA(a, function(b) {
        processB(b, function(c) {
            processC(c, function(d) {
                // Deeply nested
            });
        });
    });
});
"#;
        let path = PathBuf::from("test.js");
        let ast = parse_source(source, &path, JsLanguageVariant::JavaScript).unwrap();

        let items = detect_async_debt(&ast, &[]);

        // Should detect callback hell
        assert!(!items.is_empty());
        assert!(items.iter().any(|i| i.message.contains("Callback hell")));
    }

    #[test]
    fn test_no_callback_hell_for_shallow() {
        let source = r#"
getData(function(a) {
    processA(a);
});
"#;
        let path = PathBuf::from("test.js");
        let ast = parse_source(source, &path, JsLanguageVariant::JavaScript).unwrap();

        let items = detect_async_debt(&ast, &[]);

        // Should not detect callback hell for shallow nesting
        assert!(!items.iter().any(|i| i.message.contains("Callback hell")));
    }

    #[test]
    fn test_detect_unhandled_promise() {
        let source = r#"
fetch('/api/data')
    .then(response => response.json())
    .then(data => console.log(data));
"#;
        let path = PathBuf::from("test.js");
        let ast = parse_source(source, &path, JsLanguageVariant::JavaScript).unwrap();

        let items = detect_async_debt(&ast, &[]);

        // Should detect unhandled promise
        assert!(items
            .iter()
            .any(|i| i.message.contains("without error handling")));
    }

    #[test]
    fn test_handled_promise_ok() {
        let source = r#"
fetch('/api/data')
    .then(response => response.json())
    .then(data => console.log(data))
    .catch(err => console.error(err));
"#;
        let path = PathBuf::from("test.js");
        let ast = parse_source(source, &path, JsLanguageVariant::JavaScript).unwrap();

        let items = detect_async_debt(&ast, &[]);

        // Should not detect unhandled promise when .catch is present
        assert!(!items
            .iter()
            .any(|i| i.message.contains("without error handling")));
    }
}
