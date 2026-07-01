use super::token::{SolidityEntropyToken, extract_tokens_recursive};
use crate::analyzers::solidity::parser::node_text;
use crate::complexity::entropy_analysis::{calculate_repetition_score, calculate_weighted_entropy};
use crate::complexity::entropy_core::{
    EntropyConfig, EntropyScore, LanguageEntropyAnalyzer, PatternMetrics,
};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use tree_sitter::Node;

pub struct SolidityEntropyAnalyzer<'a> {
    #[allow(dead_code)]
    source: &'a str,
}

impl<'a> SolidityEntropyAnalyzer<'a> {
    #[allow(dead_code)]
    pub fn new(source: &'a str) -> Self {
        Self { source }
    }
}

impl LanguageEntropyAnalyzer for SolidityEntropyAnalyzer<'_> {
    type AstNode = Node<'static>;
    type Token = SolidityEntropyToken;

    fn extract_tokens(&self, _node: &Self::AstNode) -> Vec<Self::Token> {
        Vec::new()
    }

    fn detect_patterns(&self, _node: &Self::AstNode) -> PatternMetrics {
        PatternMetrics::new()
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

pub fn calculate_entropy(node: Node, source: &str, config: &EntropyConfig) -> EntropyScore {
    let tokens = extract_tokens_recursive(node, source);
    let token_entropy = token_entropy(&tokens);
    let token_repetition = token_repetition(&tokens);
    let pattern_repetition = token_repetition.max(guard_repetition(node, source));
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

pub fn generate_cache_key(node: Node, source: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(node_text(&node, source).as_bytes());
    format!("{:x}", hasher.finalize())
}

fn token_entropy(tokens: &[SolidityEntropyToken]) -> f64 {
    if tokens.is_empty() {
        0.0
    } else {
        calculate_weighted_entropy(tokens)
    }
}

fn token_repetition(tokens: &[SolidityEntropyToken]) -> f64 {
    if tokens.is_empty() {
        0.0
    } else {
        calculate_repetition_score(tokens)
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

fn guard_repetition(node: Node, source: &str) -> f64 {
    let mut guard_calls = 0;
    let mut total_calls = 0;
    collect_call_counts(node, source, &mut guard_calls, &mut total_calls);

    if total_calls == 0 {
        0.0
    } else {
        guard_calls as f64 / total_calls as f64
    }
}

fn collect_call_counts(node: Node, source: &str, guard_calls: &mut usize, total_calls: &mut usize) {
    if node.kind() == "call_expression" {
        *total_calls += 1;
        if is_guard_call(node, source) {
            *guard_calls += 1;
        }
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_call_counts(child, source, guard_calls, total_calls);
    }
}

fn is_guard_call(node: Node, source: &str) -> bool {
    node.child_by_field_name("function")
        .map(|function| {
            matches!(
                node_text(&function, source),
                "require" | "assert" | "revert"
            )
        })
        .unwrap_or(false)
}

fn analyze_structure(node: Node, source: &str) -> (usize, u32) {
    let mut identifiers = HashSet::new();
    let max_nesting = collect_structure(node, source, 0, &mut identifiers);
    (identifiers.len(), max_nesting)
}

fn collect_structure(
    node: Node,
    source: &str,
    current_depth: u32,
    identifiers: &mut HashSet<String>,
) -> u32 {
    let depth = if is_nesting_node(node) {
        current_depth + 1
    } else {
        current_depth
    };

    if node.kind() == "identifier" {
        identifiers.insert(node_text(&node, source).to_string());
    }

    let mut max_depth = depth;
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        max_depth = max_depth.max(collect_structure(child, source, depth, identifiers));
    }
    max_depth
}

fn is_nesting_node(node: Node) -> bool {
    matches!(
        node.kind(),
        "if_statement" | "for_statement" | "while_statement" | "do_while_statement"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzers::solidity::parser::parse_source;
    use std::path::Path;

    fn body_for_first_function(tree: &tree_sitter::Tree) -> Node<'_> {
        find_first_function_body(tree.root_node()).expect("function body")
    }

    fn find_first_function_body(node: Node) -> Option<Node> {
        if node.kind() == "function_definition" {
            return node.child_by_field_name("body");
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if let Some(body) = find_first_function_body(child) {
                return Some(body);
            }
        }
        None
    }

    #[test]
    fn test_repetitive_require_chain_has_pattern_repetition() {
        let source = r#"pragma solidity 0.8.20;
contract C {
    function validate(address a, address b, address c) public pure {
        require(a != address(0));
        require(b != address(0));
        require(c != address(0));
    }
}
"#;
        let ast = parse_source(source, Path::new("C.sol")).unwrap();
        let body = body_for_first_function(&ast.tree);
        let score = calculate_entropy(body, source, &EntropyConfig::default());

        assert!(score.pattern_repetition > 0.5);
        assert!(score.token_entropy >= 0.0 && score.token_entropy <= 1.0);
    }

    #[test]
    fn test_nested_control_flow_tracks_nesting() {
        let source = r#"pragma solidity 0.8.20;
contract C {
    function run(uint256 x) public pure returns (uint256) {
        if (x > 10) {
            while (x > 1) {
                x--;
            }
        }
        return x;
    }
}
"#;
        let ast = parse_source(source, Path::new("C.sol")).unwrap();
        let body = body_for_first_function(&ast.tree);
        let score = calculate_entropy(body, source, &EntropyConfig::default());

        assert!(score.max_nesting >= 2);
    }
}
