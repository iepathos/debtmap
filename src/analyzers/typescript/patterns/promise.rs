//! Promise pattern detection
//!
//! Detects Promise usage patterns including chains and Promise.all.

use crate::analyzers::typescript::parser::node_text;
use crate::analyzers::typescript::types::AsyncPattern;
use crate::core::ast::TypeScriptAst;
use tree_sitter::Node;

/// Detect promise patterns
pub fn detect_promise_patterns(node: &Node, ast: &TypeScriptAst) -> Vec<AsyncPattern> {
    let mut patterns = Vec::new();

    detect_patterns_recursive(node, ast, &mut patterns);

    patterns
}

fn detect_patterns_recursive(node: &Node, ast: &TypeScriptAst, patterns: &mut Vec<AsyncPattern>) {
    if node.kind() == "call_expression" {
        // Check for Promise.all, Promise.race, etc.
        if let Some(pattern) = detect_promise_all(node, ast) {
            patterns.push(pattern);
        }

        // Check for promise chains
        if let Some(pattern) = detect_promise_chain(node, ast) {
            patterns.push(pattern);
        }
    }

    // Recurse
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        detect_patterns_recursive(&child, ast, patterns);
    }
}

fn detect_promise_all(node: &Node, ast: &TypeScriptAst) -> Option<AsyncPattern> {
    if let Some(func) = node.child_by_field_name("function") {
        if func.kind() == "member_expression" {
            let object = func.child_by_field_name("object")?;
            let property = func.child_by_field_name("property")?;

            let obj_text = node_text(&object, &ast.source);
            let prop_text = node_text(&property, &ast.source);

            if obj_text == "Promise" {
                let method = prop_text.to_string();
                if matches!(method.as_str(), "all" | "allSettled" | "race" | "any") {
                    // Count promises in the array argument
                    let count = count_array_elements(node);
                    return Some(AsyncPattern::PromiseAll {
                        promise_count: count,
                        method,
                    });
                }
            }
        }
    }
    None
}

fn detect_promise_chain(node: &Node, ast: &TypeScriptAst) -> Option<AsyncPattern> {
    if let Some(func) = node.child_by_field_name("function") {
        if func.kind() == "member_expression" {
            if let Some(property) = func.child_by_field_name("property") {
                let prop_text = node_text(&property, &ast.source);
                // Only detect at the end of the chain (then, catch, or finally)
                // Check if this node is the outermost by ensuring parent is not a method call on this
                if matches!(prop_text, "then" | "catch" | "finally") {
                    // Check if parent is also a chained call - if so, skip this node
                    if let Some(parent) = node.parent() {
                        // If our node is the object of a member_expression, we're not at the end
                        if parent.kind() == "member_expression" {
                            if let Some(obj) = parent.child_by_field_name("object") {
                                if obj.id() == node.id() {
                                    return None; // Not at the end of chain
                                }
                            }
                        }
                    }

                    // We're at the outermost call - analyze the full chain
                    let (chain_length, has_catch, has_finally) = analyze_chain(node, ast);
                    if chain_length > 0 {
                        return Some(AsyncPattern::PromiseChain {
                            chain_length,
                            has_catch,
                            has_finally,
                        });
                    }
                }
            }
        }
    }
    None
}

fn count_array_elements(node: &Node) -> u32 {
    if let Some(args) = node.child_by_field_name("arguments") {
        let mut cursor = args.walk();
        for child in args.children(&mut cursor) {
            if child.kind() == "array" {
                return child.named_child_count() as u32;
            }
        }
    }
    0
}

fn analyze_chain(node: &Node, ast: &TypeScriptAst) -> (u32, bool, bool) {
    let text = node_text(node, &ast.source);

    let then_count = text.matches(".then(").count() as u32;
    let has_catch = text.contains(".catch(");
    let has_finally = text.contains(".finally(");

    (then_count, has_catch, has_finally)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzers::typescript::parser::parse_source;
    use crate::core::ast::JsLanguageVariant;
    use std::path::PathBuf;

    #[test]
    fn test_detect_promise_all() {
        let source = "Promise.all([p1, p2, p3])";
        let path = PathBuf::from("test.js");
        let ast = parse_source(source, &path, JsLanguageVariant::JavaScript).unwrap();

        let patterns = detect_promise_patterns(&ast.tree.root_node(), &ast);

        assert!(patterns
            .iter()
            .any(|p| matches!(p, AsyncPattern::PromiseAll { method, .. } if method == "all")));
    }

    #[test]
    fn test_detect_promise_chain() {
        let source = r#"
fetch('/api')
    .then(r => r.json())
    .then(data => process(data))
    .catch(err => console.error(err));
"#;
        let path = PathBuf::from("test.js");
        let ast = parse_source(source, &path, JsLanguageVariant::JavaScript).unwrap();

        let patterns = detect_promise_patterns(&ast.tree.root_node(), &ast);

        assert!(patterns.iter().any(|p| matches!(
            p,
            AsyncPattern::PromiseChain {
                has_catch: true,
                ..
            }
        )));
    }
}
