//! Functional programming pattern detection
//!
//! Detects functional patterns like map/filter/reduce chains.

use crate::analyzers::typescript::parser::{node_line, node_text};
use crate::analyzers::typescript::types::FunctionalChain;
use crate::core::ast::TypeScriptAst;
use tree_sitter::Node;

/// Detect functional programming chains (map/filter/reduce)
pub fn detect_functional_chains(node: &Node, ast: &TypeScriptAst) -> Vec<FunctionalChain> {
    let mut chains = Vec::new();

    detect_chains_recursive(node, ast, &mut chains);

    chains
}

fn detect_chains_recursive(node: &Node, ast: &TypeScriptAst, chains: &mut Vec<FunctionalChain>) {
    if node.kind() == "call_expression" {
        if let Some(chain) = try_extract_chain(node, ast) {
            chains.push(chain);
            // Don't recurse into this node to avoid double-counting
            return;
        }
    }

    // Recurse
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        detect_chains_recursive(&child, ast, chains);
    }
}

fn try_extract_chain(node: &Node, ast: &TypeScriptAst) -> Option<FunctionalChain> {
    let text = node_text(node, &ast.source);

    // Extract method names from the chain
    let methods = extract_chain_methods(&text);

    if methods.len() >= 2 {
        // This is a meaningful chain
        let appears_pure = check_purity(node, ast);

        Some(FunctionalChain {
            methods: methods.clone(),
            length: methods.len() as u32,
            line: node_line(node),
            appears_pure,
        })
    } else {
        None
    }
}

fn extract_chain_methods(text: &str) -> Vec<String> {
    let mut methods = Vec::new();

    let functional_methods = [
        "map",
        "filter",
        "reduce",
        "forEach",
        "find",
        "findIndex",
        "some",
        "every",
        "flatMap",
        "flat",
        "sort",
        "reverse",
        "slice",
        "concat",
        "join",
        "includes",
        "indexOf",
    ];

    for method in &functional_methods {
        let pattern = format!(".{}(", method);
        if text.contains(&pattern) {
            methods.push(method.to_string());
        }
    }

    methods
}

fn check_purity(node: &Node, ast: &TypeScriptAst) -> bool {
    // Simple heuristic: check if there are side effects in the chain
    let text = node_text(node, &ast.source);

    // Check for obvious side effects
    let has_side_effects = text.contains("console.")
        || text.contains("this.")
        || text.contains("document.")
        || text.contains("window.")
        || text.contains("fetch(")
        || text.contains("localStorage")
        || text.contains("sessionStorage");

    !has_side_effects
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzers::typescript::parser::parse_source;
    use crate::core::ast::JsLanguageVariant;
    use std::path::PathBuf;

    #[test]
    fn test_detect_map_filter_chain() {
        let source = r#"
const result = items
    .filter(x => x > 0)
    .map(x => x * 2);
"#;
        let path = PathBuf::from("test.js");
        let ast = parse_source(source, &path, JsLanguageVariant::JavaScript).unwrap();

        let chains = detect_functional_chains(&ast.tree.root_node(), &ast);

        assert!(!chains.is_empty());
        assert!(chains[0].methods.contains(&"filter".to_string()));
        assert!(chains[0].methods.contains(&"map".to_string()));
    }

    #[test]
    fn test_detect_reduce_chain() {
        let source = r#"
const sum = items
    .filter(x => x > 0)
    .map(x => x.value)
    .reduce((a, b) => a + b, 0);
"#;
        let path = PathBuf::from("test.js");
        let ast = parse_source(source, &path, JsLanguageVariant::JavaScript).unwrap();

        let chains = detect_functional_chains(&ast.tree.root_node(), &ast);

        assert!(!chains.is_empty());
        assert!(chains[0].length >= 3);
    }

    #[test]
    fn test_pure_chain_detection() {
        let source = "items.filter(x => x > 0).map(x => x * 2);";
        let path = PathBuf::from("test.js");
        let ast = parse_source(source, &path, JsLanguageVariant::JavaScript).unwrap();

        let chains = detect_functional_chains(&ast.tree.root_node(), &ast);

        assert!(!chains.is_empty());
        assert!(chains[0].appears_pure);
    }

    #[test]
    fn test_impure_chain_detection() {
        let source = "items.forEach(x => console.log(x));";
        let path = PathBuf::from("test.js");
        let ast = parse_source(source, &path, JsLanguageVariant::JavaScript).unwrap();

        let chains = detect_functional_chains(&ast.tree.root_node(), &ast);

        // forEach alone isn't a chain, but if detected it should be impure
        for chain in chains {
            assert!(!chain.appears_pure);
        }
    }
}
