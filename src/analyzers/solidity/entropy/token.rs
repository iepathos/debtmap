use crate::analyzers::solidity::parser::node_text;
use crate::complexity::entropy_core::{EntropyToken, TokenCategory};
use std::hash::{Hash, Hasher};
use tree_sitter::Node;

#[derive(Debug, Clone)]
pub struct SolidityEntropyToken {
    category: TokenCategory,
    weight: f64,
    value: String,
}

impl SolidityEntropyToken {
    fn new(category: TokenCategory, weight: f64, value: impl Into<String>) -> Self {
        Self {
            category,
            weight,
            value: value.into(),
        }
    }

    fn control_flow(value: impl Into<String>) -> Self {
        Self::new(TokenCategory::ControlFlow, 1.2, value)
    }

    fn keyword(value: impl Into<String>) -> Self {
        Self::new(TokenCategory::Keyword, 1.0, value)
    }

    fn operator(value: impl Into<String>) -> Self {
        Self::new(TokenCategory::Operator, 0.8, value)
    }

    fn identifier(value: impl Into<String>) -> Self {
        Self::new(TokenCategory::Identifier, 0.5, value)
    }

    fn literal(value: impl Into<String>) -> Self {
        Self::new(TokenCategory::Literal, 0.3, value)
    }

    fn function_call(value: impl Into<String>) -> Self {
        Self::new(TokenCategory::FunctionCall, 0.9, value)
    }
}

impl Hash for SolidityEntropyToken {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.category.hash(state);
        self.value.hash(state);
    }
}

impl PartialEq for SolidityEntropyToken {
    fn eq(&self, other: &Self) -> bool {
        self.category == other.category && self.value == other.value
    }
}

impl Eq for SolidityEntropyToken {}

impl EntropyToken for SolidityEntropyToken {
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

pub fn extract_tokens_recursive(node: Node, source: &str) -> Vec<SolidityEntropyToken> {
    let mut tokens = Vec::new();
    extract_tokens_inner(node, source, &mut tokens);
    tokens
}

fn extract_tokens_inner(node: Node, source: &str, tokens: &mut Vec<SolidityEntropyToken>) {
    push_token_for_node(node, source, tokens);

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        extract_tokens_inner(child, source, tokens);
    }
}

fn push_token_for_node(node: Node, source: &str, tokens: &mut Vec<SolidityEntropyToken>) {
    match node.kind() {
        "if_statement" => tokens.push(SolidityEntropyToken::control_flow("if")),
        "for_statement" => tokens.push(SolidityEntropyToken::control_flow("for")),
        "while_statement" => tokens.push(SolidityEntropyToken::control_flow("while")),
        "do_while_statement" => tokens.push(SolidityEntropyToken::control_flow("do")),
        "emit_statement" => tokens.push(SolidityEntropyToken::keyword("emit")),
        "return_statement" => tokens.push(SolidityEntropyToken::keyword("return")),
        "revert_statement" => tokens.push(SolidityEntropyToken::keyword("revert")),
        "assignment_expression" | "augmented_assignment_expression" => {
            tokens.push(SolidityEntropyToken::operator("="));
        }
        "binary_expression" | "unary_expression" => {
            tokens.push(SolidityEntropyToken::operator(operator_text(node, source)));
        }
        "identifier" => {
            let text = node_text(&node, source);
            if !is_solidity_keyword(text) {
                tokens.push(SolidityEntropyToken::identifier(text));
            }
        }
        "number_literal" | "string_literal" | "hex_string_literal" | "boolean_literal" => {
            tokens.push(SolidityEntropyToken::literal(node.kind()));
        }
        "call_expression" => {
            tokens.push(SolidityEntropyToken::function_call(call_name(node, source)));
        }
        _ => {}
    }
}

fn operator_text<'a>(node: Node, source: &'a str) -> &'a str {
    node.child_by_field_name("operator")
        .map(|operator| node_text(&operator, source))
        .unwrap_or(node.kind())
}

fn call_name(node: Node, source: &str) -> String {
    node.child_by_field_name("function")
        .map(|function| normalize_call_name(node_text(&function, source)))
        .unwrap_or_else(|| "call".to_string())
}

fn normalize_call_name(text: &str) -> String {
    match text {
        "require" | "assert" | "revert" => text.to_string(),
        _ if text.ends_with(".transfer") => "transfer".to_string(),
        _ if text.ends_with(".send") => "send".to_string(),
        _ if text.ends_with(".call") => "call".to_string(),
        _ if text.ends_with(".delegatecall") => "delegatecall".to_string(),
        _ => "call".to_string(),
    }
}

fn is_solidity_keyword(text: &str) -> bool {
    matches!(
        text,
        "contract"
            | "interface"
            | "library"
            | "function"
            | "modifier"
            | "constructor"
            | "returns"
            | "public"
            | "external"
            | "internal"
            | "private"
            | "view"
            | "pure"
            | "payable"
            | "memory"
            | "storage"
            | "calldata"
            | "if"
            | "else"
            | "for"
            | "while"
            | "return"
            | "emit"
            | "revert"
            | "true"
            | "false"
    )
}
