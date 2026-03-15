//! Entropy-based complexity analysis for Python
//!
//! This module provides entropy analysis for Python code,
//! enabling cognitive complexity dampening for repetitive patterns.

use crate::complexity::entropy_analysis::{calculate_repetition_score, calculate_weighted_entropy};
use crate::complexity::entropy_core::{
    EntropyConfig, EntropyScore, EntropyToken, LanguageEntropyAnalyzer, TokenCategory,
};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::hash::{Hash, Hasher};
use tree_sitter::Node;

/// Python entropy analyzer
pub struct PythonEntropyAnalyzer<'a> {
    #[allow(dead_code)]
    source: &'a str,
}

impl<'a> PythonEntropyAnalyzer<'a> {
    pub fn new(source: &'a str) -> Self {
        Self { source }
    }
}

/// Python entropy token
#[derive(Debug, Clone)]
pub struct PyEntropyToken {
    category: TokenCategory,
    weight: f64,
    value: String,
}

impl PyEntropyToken {
    pub fn new(category: TokenCategory, weight: f64, value: String) -> Self {
        Self {
            category,
            weight,
            value,
        }
    }

    pub fn control_flow(value: String) -> Self {
        Self::new(TokenCategory::ControlFlow, 1.2, value)
    }

    pub fn keyword(value: String) -> Self {
        Self::new(TokenCategory::Keyword, 1.0, value)
    }

    pub fn operator(value: String) -> Self {
        Self::new(TokenCategory::Operator, 0.8, value)
    }

    pub fn identifier(value: String) -> Self {
        Self::new(TokenCategory::Identifier, 0.5, value)
    }

    pub fn literal(value: String) -> Self {
        Self::new(TokenCategory::Literal, 0.3, value)
    }

    pub fn function_call(value: String) -> Self {
        Self::new(TokenCategory::FunctionCall, 0.9, value)
    }
}

impl Hash for PyEntropyToken {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.category.hash(state);
        self.value.hash(state);
    }
}

impl PartialEq for PyEntropyToken {
    fn eq(&self, other: &Self) -> bool {
        self.category == other.category && self.value == other.value
    }
}

impl Eq for PyEntropyToken {}

impl EntropyToken for PyEntropyToken {
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

impl LanguageEntropyAnalyzer for PythonEntropyAnalyzer<'_> {
    type AstNode = Node<'static>;
    type Token = PyEntropyToken;

    fn extract_tokens(&self, _node: &Self::AstNode) -> Vec<Self::Token> {
        Vec::new()
    }

    fn detect_patterns(
        &self,
        _node: &Self::AstNode,
    ) -> crate::complexity::entropy_core::PatternMetrics {
        crate::complexity::entropy_core::PatternMetrics::new()
    }

    fn calculate_branch_similarity(&self, _node: &Self::AstNode) -> f64 {
        0.0
    }

    fn analyze_structure(&self, _node: &Self::AstNode) -> (usize, u32) {
        (0, 0)
    }

    fn generate_cache_key(&self, _node: &Self::AstNode) -> String {
        String::new()
    }
}

/// Calculate entropy score for a Python function body node
pub fn calculate_entropy(node: &Node, source: &str, config: &EntropyConfig) -> EntropyScore {
    let tokens = extract_tokens_recursive(node, source);

    let token_entropy = if tokens.is_empty() {
        0.0
    } else {
        calculate_weighted_entropy(&tokens)
    };

    let token_repetition = if tokens.is_empty() {
        0.0
    } else {
        calculate_repetition_score(&tokens)
    };

    // For now, pattern repetition and branch similarity are simplified
    let pattern_repetition = token_repetition;
    let branch_similarity = 0.0;

    let (unique_variables, max_nesting) = analyze_structure(node, source);

    let effective_complexity = calculate_effective_complexity(
        token_entropy,
        pattern_repetition,
        branch_similarity,
        config,
    );

    EntropyScore {
        token_entropy,
        pattern_repetition,
        branch_similarity,
        effective_complexity,
        unique_variables,
        max_nesting,
        dampening_applied: 1.0,
    }
}

fn calculate_effective_complexity(
    entropy: f64,
    patterns: f64,
    similarity: f64,
    config: &EntropyConfig,
) -> f64 {
    let pattern_factor = 1.0 - (patterns * config.pattern_weight);
    let similarity_factor = 1.0 - (similarity * config.similarity_weight);
    let adjusted = entropy * pattern_factor * similarity_factor;

    if adjusted < config.base_threshold {
        adjusted * 0.5
    } else {
        adjusted
    }
}

fn analyze_structure(node: &Node, source: &str) -> (usize, u32) {
    let mut variables: HashSet<String> = HashSet::new();
    let mut max_nesting = 0;
    collect_structure(node, source, &mut variables, 0, &mut max_nesting);
    (variables.len(), max_nesting)
}

fn collect_structure(
    node: &Node,
    source: &str,
    variables: &mut HashSet<String>,
    current_depth: u32,
    max_nesting: &mut u32,
) {
    let kind = node.kind();
    let mut depth = current_depth;

    match kind {
        "identifier" => {
            let name = &source[node.start_byte()..node.end_byte()];
            if !is_keyword(name) {
                variables.insert(name.to_string());
            }
        }
        "if_statement" | "for_statement" | "while_statement" | "with_statement"
        | "try_statement" | "match_statement" => {
            depth += 1;
            if depth > *max_nesting {
                *max_nesting = depth;
            }
        }
        _ => {}
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_structure(&child, source, variables, depth, max_nesting);
    }
}

fn is_keyword(name: &str) -> bool {
    matches!(
        name,
        "def"
            | "class"
            | "if"
            | "else"
            | "elif"
            | "for"
            | "while"
            | "try"
            | "except"
            | "finally"
            | "with"
            | "as"
            | "import"
            | "from"
            | "return"
            | "yield"
            | "break"
            | "continue"
            | "pass"
            | "raise"
            | "assert"
            | "global"
            | "nonlocal"
            | "lambda"
            | "async"
            | "await"
            | "match"
            | "case"
            | "and"
            | "or"
            | "not"
            | "is"
            | "in"
            | "True"
            | "False"
            | "None"
    )
}

fn extract_tokens_recursive(node: &Node, source: &str) -> Vec<PyEntropyToken> {
    let mut tokens = Vec::new();
    extract_tokens_inner(node, source, &mut tokens);
    tokens
}

fn extract_tokens_inner(node: &Node, source: &str, tokens: &mut Vec<PyEntropyToken>) {
    let kind = node.kind();
    let text = &source[node.start_byte()..node.end_byte()];

    match kind {
        "if_statement" | "elif_clause" | "else_clause" => {
            tokens.push(PyEntropyToken::control_flow("if".to_string()));
        }
        "for_statement" | "while_statement" => {
            tokens.push(PyEntropyToken::control_flow("loop".to_string()));
        }
        "try_statement" | "except_clause" | "finally_clause" => {
            tokens.push(PyEntropyToken::control_flow("try".to_string()));
        }
        "match_statement" | "case_clause" => {
            tokens.push(PyEntropyToken::control_flow("match".to_string()));
        }
        "with_statement" => {
            tokens.push(PyEntropyToken::control_flow("with".to_string()));
        }
        "return_statement" | "yield" => {
            tokens.push(PyEntropyToken::keyword("return".to_string()));
        }
        "raise_statement" => {
            tokens.push(PyEntropyToken::keyword("raise".to_string()));
        }
        "assignment" => {
            tokens.push(PyEntropyToken::operator("=".to_string()));
        }
        "binary_operator" | "boolean_operator" | "comparison_operator" => {
            tokens.push(PyEntropyToken::operator(text.to_string()));
        }
        "identifier" => {
            tokens.push(PyEntropyToken::identifier(text.to_string()));
        }
        "integer" | "float" | "true" | "false" | "none" => {
            tokens.push(PyEntropyToken::literal(kind.to_string()));
        }
        "(" | ")" | "[" | "]" | "{" | "}" | ":" | "," => {
            tokens.push(PyEntropyToken::operator(text.to_string()));
        }
        "call" => {
            if let Some(func) = node.child_by_field_name("function") {
                let func_name = &source[func.start_byte()..func.end_byte()];
                tokens.push(PyEntropyToken::function_call(func_name.to_string()));
            }
        }
        "list_comprehension"
        | "dictionary_comprehension"
        | "set_comprehension"
        | "generator_expression" => {
            tokens.push(PyEntropyToken::control_flow("comprehension".to_string()));
        }
        _ => {}
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        extract_tokens_inner(&child, source, tokens);
    }
}

pub fn generate_cache_key(node: &Node, source: &str) -> String {
    let text = &source[node.start_byte()..node.end_byte()];
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    let result = hasher.finalize();
    format!("{:x}", result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzers::python::parser::parse_source;
    use std::path::PathBuf;

    fn parse_py(source: &str) -> tree_sitter::Tree {
        let path = PathBuf::from("test.py");
        let ast = parse_source(source, &path).unwrap();
        ast.tree
    }

    #[test]
    fn test_calculate_entropy_repetitive_validation() {
        let source = r#"
def validate(data):
    if not data.get('id'): raise ValueError('id missing')
    if not data.get('name'): raise ValueError('name missing')
    if not data.get('email'): raise ValueError('email missing')
    if not data.get('age'): raise ValueError('age missing')
"#;
        let tree = parse_py(source);
        let config = EntropyConfig::default();
        let score = calculate_entropy(&tree.root_node(), source, &config);

        assert!(
            score.pattern_repetition > 0.3,
            "Repetitive validation should have high pattern repetition: {}",
            score.pattern_repetition
        );
    }
}
