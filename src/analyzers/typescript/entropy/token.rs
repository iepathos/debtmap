//! Token extraction for JavaScript/TypeScript entropy analysis
//!
//! Maps tree-sitter node kinds to `TokenCategory` with appropriate weights.

use crate::complexity::entropy_core::{EntropyToken, TokenCategory};
use std::hash::{Hash, Hasher};
use tree_sitter::Node;

/// JavaScript/TypeScript entropy token
#[derive(Debug, Clone)]
pub struct JsEntropyToken {
    category: TokenCategory,
    weight: f64,
    value: String,
}

impl JsEntropyToken {
    pub fn new(category: TokenCategory, weight: f64, value: String) -> Self {
        Self {
            category,
            weight,
            value,
        }
    }

    /// Create a control flow token (if, switch, for, while)
    pub fn control_flow(value: String) -> Self {
        Self::new(TokenCategory::ControlFlow, 1.2, value)
    }

    /// Create a keyword token (function, async, await, return, throw)
    pub fn keyword(value: String) -> Self {
        Self::new(TokenCategory::Keyword, 1.0, value)
    }

    /// Create an operator token (+, -, &&, ||, ??)
    pub fn operator(value: String) -> Self {
        Self::new(TokenCategory::Operator, 0.8, value)
    }

    /// Create an identifier token
    pub fn identifier(value: String) -> Self {
        Self::new(TokenCategory::Identifier, 0.5, value)
    }

    /// Create a literal token (number, string, true, false, null)
    pub fn literal(value: String) -> Self {
        Self::new(TokenCategory::Literal, 0.3, value)
    }

    /// Create a function call token
    pub fn function_call(value: String) -> Self {
        Self::new(TokenCategory::FunctionCall, 0.9, value)
    }
}

impl Hash for JsEntropyToken {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.category.hash(state);
        self.value.hash(state);
    }
}

impl PartialEq for JsEntropyToken {
    fn eq(&self, other: &Self) -> bool {
        self.category == other.category && self.value == other.value
    }
}

impl Eq for JsEntropyToken {}

impl EntropyToken for JsEntropyToken {
    fn to_category(&self) -> TokenCategory {
        self.category.clone()
    }

    fn weight(&self) -> f64 {
        self.weight
    }

    fn value(&self) -> &str {
        &self.value
    }
}

/// Extract tokens from a tree-sitter node recursively
pub fn extract_tokens_recursive(node: &Node, source: &str) -> Vec<JsEntropyToken> {
    let mut tokens = Vec::new();
    extract_tokens_inner(node, source, &mut tokens);
    tokens
}

fn extract_tokens_inner(node: &Node, source: &str, tokens: &mut Vec<JsEntropyToken>) {
    let kind = node.kind();
    let text = node_text(node, source);

    // Map node kinds to token categories
    match kind {
        // Control flow statements - highest weight
        "if_statement" => {
            tokens.push(JsEntropyToken::control_flow("if".to_string()));
        }
        "switch_statement" => {
            tokens.push(JsEntropyToken::control_flow("switch".to_string()));
        }
        "for_statement" | "for_in_statement" | "for_of_statement" => {
            tokens.push(JsEntropyToken::control_flow("for".to_string()));
        }
        "while_statement" => {
            tokens.push(JsEntropyToken::control_flow("while".to_string()));
        }
        "do_statement" => {
            tokens.push(JsEntropyToken::control_flow("do".to_string()));
        }
        "try_statement" => {
            tokens.push(JsEntropyToken::control_flow("try".to_string()));
        }
        "catch_clause" => {
            tokens.push(JsEntropyToken::control_flow("catch".to_string()));
        }
        "ternary_expression" | "conditional_expression" => {
            tokens.push(JsEntropyToken::control_flow("?:".to_string()));
        }

        // Keywords
        "function" | "function_declaration" | "function_expression" => {
            tokens.push(JsEntropyToken::keyword("function".to_string()));
        }
        "arrow_function" => {
            tokens.push(JsEntropyToken::keyword("=>".to_string()));
        }
        "async" => {
            tokens.push(JsEntropyToken::keyword("async".to_string()));
        }
        "await_expression" => {
            tokens.push(JsEntropyToken::keyword("await".to_string()));
        }
        "return_statement" => {
            tokens.push(JsEntropyToken::keyword("return".to_string()));
        }
        "throw_statement" => {
            tokens.push(JsEntropyToken::keyword("throw".to_string()));
        }
        "break_statement" => {
            tokens.push(JsEntropyToken::keyword("break".to_string()));
        }
        "continue_statement" => {
            tokens.push(JsEntropyToken::keyword("continue".to_string()));
        }
        "yield_expression" => {
            tokens.push(JsEntropyToken::keyword("yield".to_string()));
        }

        // Operators
        "binary_expression" => {
            if let Some(op) = node.child_by_field_name("operator") {
                let op_text = node_text(&op, source);
                tokens.push(JsEntropyToken::operator(op_text.to_string()));
            }
        }
        "unary_expression" => {
            if let Some(op) = node.child_by_field_name("operator") {
                let op_text = node_text(&op, source);
                tokens.push(JsEntropyToken::operator(op_text.to_string()));
            }
        }
        "update_expression" => {
            tokens.push(JsEntropyToken::operator("++/--".to_string()));
        }
        "assignment_expression" => {
            tokens.push(JsEntropyToken::operator("=".to_string()));
        }

        // Identifiers
        "identifier" | "property_identifier" => {
            tokens.push(JsEntropyToken::identifier(text.to_string()));
        }
        "shorthand_property_identifier" | "shorthand_property_identifier_pattern" => {
            tokens.push(JsEntropyToken::identifier(text.to_string()));
        }

        // Literals
        "number" | "string" | "template_string" => {
            // Normalize literals to reduce entropy from varying values
            tokens.push(JsEntropyToken::literal(kind.to_string()));
        }
        "true" | "false" => {
            tokens.push(JsEntropyToken::literal("boolean".to_string()));
        }
        "null" | "undefined" => {
            tokens.push(JsEntropyToken::literal("nullish".to_string()));
        }

        // Function calls
        "call_expression" => {
            // Get the function name if available
            if let Some(func) = node.child_by_field_name("function") {
                let call_name = get_call_name(&func, source);
                tokens.push(JsEntropyToken::function_call(call_name));
            } else {
                tokens.push(JsEntropyToken::function_call("call".to_string()));
            }
        }
        "new_expression" => {
            tokens.push(JsEntropyToken::function_call("new".to_string()));
        }

        // Member expressions for method chains
        "member_expression" => {
            if let Some(prop) = node.child_by_field_name("property") {
                let prop_text = node_text(&prop, source);
                // Track common method names for pattern detection
                match prop_text {
                    "map" | "filter" | "reduce" | "forEach" | "find" | "some" | "every" => {
                        tokens.push(JsEntropyToken::function_call(prop_text.to_string()));
                    }
                    "then" | "catch" | "finally" => {
                        tokens.push(JsEntropyToken::function_call(prop_text.to_string()));
                    }
                    _ => {
                        tokens.push(JsEntropyToken::identifier(prop_text.to_string()));
                    }
                }
            }
        }

        // Object and array patterns (common in validation code)
        "object" | "object_pattern" => {
            tokens.push(JsEntropyToken::operator("{}".to_string()));
        }
        "array" | "array_pattern" => {
            tokens.push(JsEntropyToken::operator("[]".to_string()));
        }

        // Default: recurse into children
        _ => {}
    }

    // Recurse into child nodes
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        extract_tokens_inner(&child, source, tokens);
    }
}

/// Get function/method call name
fn get_call_name(node: &Node, source: &str) -> String {
    match node.kind() {
        "identifier" => node_text(node, source).to_string(),
        "member_expression" => {
            if let Some(prop) = node.child_by_field_name("property") {
                node_text(&prop, source).to_string()
            } else {
                "method".to_string()
            }
        }
        _ => "call".to_string(),
    }
}

/// Extract text from a tree-sitter node
fn node_text<'a>(node: &Node, source: &'a str) -> &'a str {
    let start = node.start_byte();
    let end = node.end_byte();
    &source[start..end]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzers::typescript::parser::parse_source;
    use crate::core::ast::JsLanguageVariant;
    use std::path::PathBuf;

    fn parse_js(source: &str) -> tree_sitter::Tree {
        let path = PathBuf::from("test.js");
        let ast = parse_source(source, &path, JsLanguageVariant::JavaScript).unwrap();
        ast.tree
    }

    #[test]
    fn test_extract_control_flow_tokens() {
        let source = "if (x) { return 1; } else { return 2; }";
        let tree = parse_js(source);
        let tokens = extract_tokens_recursive(&tree.root_node(), source);

        let control_flow_count = tokens
            .iter()
            .filter(|t| matches!(t.to_category(), TokenCategory::ControlFlow))
            .count();

        assert!(control_flow_count >= 1, "Should detect if statement");
    }

    #[test]
    fn test_extract_operator_tokens() {
        let source = "const result = a + b && c || d;";
        let tree = parse_js(source);
        let tokens = extract_tokens_recursive(&tree.root_node(), source);

        let operator_count = tokens
            .iter()
            .filter(|t| matches!(t.to_category(), TokenCategory::Operator))
            .count();

        assert!(operator_count >= 3, "Should detect +, &&, || operators");
    }

    #[test]
    fn test_extract_function_call_tokens() {
        let source = "const result = foo().bar().baz();";
        let tree = parse_js(source);
        let tokens = extract_tokens_recursive(&tree.root_node(), source);

        let call_count = tokens
            .iter()
            .filter(|t| matches!(t.to_category(), TokenCategory::FunctionCall))
            .count();

        assert!(call_count >= 1, "Should detect function calls");
    }

    #[test]
    fn test_extract_array_method_tokens() {
        let source = "items.map(x => x * 2).filter(x => x > 0).reduce((a, b) => a + b, 0);";
        let tree = parse_js(source);
        let tokens = extract_tokens_recursive(&tree.root_node(), source);

        let method_calls: Vec<_> = tokens
            .iter()
            .filter(|t| matches!(t.to_category(), TokenCategory::FunctionCall))
            .map(|t| t.value())
            .collect();

        assert!(
            method_calls.contains(&"map"),
            "Should detect map method: {:?}",
            method_calls
        );
        assert!(
            method_calls.contains(&"filter"),
            "Should detect filter method"
        );
        assert!(
            method_calls.contains(&"reduce"),
            "Should detect reduce method"
        );
    }

    #[test]
    fn test_token_weights() {
        assert_eq!(JsEntropyToken::control_flow("if".to_string()).weight(), 1.2);
        assert_eq!(JsEntropyToken::keyword("return".to_string()).weight(), 1.0);
        assert_eq!(JsEntropyToken::operator("+".to_string()).weight(), 0.8);
        assert_eq!(JsEntropyToken::identifier("foo".to_string()).weight(), 0.5);
        assert_eq!(JsEntropyToken::literal("number".to_string()).weight(), 0.3);
        assert_eq!(
            JsEntropyToken::function_call("bar".to_string()).weight(),
            0.9
        );
    }
}
