//! Branch similarity analysis for JavaScript/TypeScript
//!
//! Analyzes similarity between conditional branches:
//! - If-else chains: Compare consequence vs alternative token sequences
//! - Switch cases: Compare case body token sequences
//! - Ternary expressions: Compare branches

use super::token::{extract_tokens_recursive, JsEntropyToken};
use crate::complexity::entropy_core::EntropyToken;
use tree_sitter::Node;

/// Analyze branch similarity for a function body
///
/// Returns a value between 0.0 (completely different branches) and 1.0 (identical branches)
pub fn analyze_branch_similarity(node: &Node, source: &str) -> f64 {
    let mut all_similarities: Vec<f64> = Vec::new();

    collect_branch_similarities(node, source, &mut all_similarities);

    if all_similarities.is_empty() {
        return 0.0;
    }

    // Return weighted average, giving more weight to higher similarities
    let sum: f64 = all_similarities.iter().sum();
    sum / all_similarities.len() as f64
}

fn collect_branch_similarities(node: &Node, source: &str, similarities: &mut Vec<f64>) {
    match node.kind() {
        "if_statement" => {
            if let Some(sim) = analyze_if_else_similarity(node, source) {
                similarities.push(sim);
            }
        }
        "switch_statement" => {
            if let Some(sim) = analyze_switch_similarity(node, source) {
                similarities.push(sim);
            }
        }
        "ternary_expression" | "conditional_expression" => {
            if let Some(sim) = analyze_ternary_similarity(node, source) {
                similarities.push(sim);
            }
        }
        _ => {}
    }

    // Recurse into children
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_branch_similarities(&child, source, similarities);
    }
}

/// Analyze similarity between if and else branches
fn analyze_if_else_similarity(node: &Node, source: &str) -> Option<f64> {
    let consequence = node.child_by_field_name("consequence")?;
    let alternative = node.child_by_field_name("alternative")?;

    // Extract tokens from both branches
    let tokens1 = extract_tokens_recursive(&consequence, source);
    let tokens2 = extract_tokens_recursive(&alternative, source);

    // If the alternative is another if statement (else if chain), analyze it recursively
    if alternative.kind() == "if_statement" {
        // For else-if chains, check if all branches are similar
        let mut chain_tokens: Vec<Vec<JsEntropyToken>> = vec![tokens1];
        collect_if_chain_tokens(&alternative, source, &mut chain_tokens);

        return Some(calculate_chain_similarity(&chain_tokens));
    }

    Some(calculate_sequence_similarity(&tokens1, &tokens2))
}

/// Collect tokens from all branches in an if-else-if chain
fn collect_if_chain_tokens(node: &Node, source: &str, chain: &mut Vec<Vec<JsEntropyToken>>) {
    if let Some(consequence) = node.child_by_field_name("consequence") {
        chain.push(extract_tokens_recursive(&consequence, source));
    }

    if let Some(alternative) = node.child_by_field_name("alternative") {
        if alternative.kind() == "if_statement" {
            collect_if_chain_tokens(&alternative, source, chain);
        } else {
            chain.push(extract_tokens_recursive(&alternative, source));
        }
    }
}

/// Analyze similarity between switch cases
fn analyze_switch_similarity(node: &Node, source: &str) -> Option<f64> {
    let body = node.child_by_field_name("body")?;

    let mut case_tokens: Vec<Vec<JsEntropyToken>> = Vec::new();
    let mut cursor = body.walk();

    for child in body.children(&mut cursor) {
        if child.kind() == "switch_case" || child.kind() == "switch_default" {
            let tokens = extract_case_body_tokens(&child, source);
            if !tokens.is_empty() {
                case_tokens.push(tokens);
            }
        }
    }

    if case_tokens.len() < 2 {
        return None;
    }

    Some(calculate_chain_similarity(&case_tokens))
}

/// Extract tokens from a switch case body
fn extract_case_body_tokens(node: &Node, source: &str) -> Vec<JsEntropyToken> {
    let mut tokens = Vec::new();
    let mut cursor = node.walk();
    let mut in_body = false;

    for child in node.children(&mut cursor) {
        // Skip the "case" keyword and condition, start collecting after ":"
        if child.kind() == ":" {
            in_body = true;
            continue;
        }

        if in_body {
            let body_tokens = extract_tokens_recursive(&child, source);
            tokens.extend(body_tokens);
        }
    }

    tokens
}

/// Analyze similarity between ternary branches
fn analyze_ternary_similarity(node: &Node, source: &str) -> Option<f64> {
    let consequence = node.child_by_field_name("consequence")?;
    let alternative = node.child_by_field_name("alternative")?;

    let tokens1 = extract_tokens_recursive(&consequence, source);
    let tokens2 = extract_tokens_recursive(&alternative, source);

    Some(calculate_sequence_similarity(&tokens1, &tokens2))
}

/// Calculate similarity between two token sequences
fn calculate_sequence_similarity(seq1: &[JsEntropyToken], seq2: &[JsEntropyToken]) -> f64 {
    if seq1.is_empty() && seq2.is_empty() {
        return 1.0;
    }
    if seq1.is_empty() || seq2.is_empty() {
        return 0.0;
    }

    // Category-based similarity (structural)
    let cat1: Vec<_> = seq1.iter().map(|t| t.to_category()).collect();
    let cat2: Vec<_> = seq2.iter().map(|t| t.to_category()).collect();

    let category_sim = jaccard_similarity(&cat1, &cat2);

    // Value-based similarity (exact tokens)
    let val1: Vec<_> = seq1.iter().map(|t| t.value().to_string()).collect();
    let val2: Vec<_> = seq2.iter().map(|t| t.value().to_string()).collect();

    let value_sim = jaccard_similarity(&val1, &val2);

    // Length similarity
    let len1 = seq1.len() as f64;
    let len2 = seq2.len() as f64;
    let length_sim = len1.min(len2) / len1.max(len2);

    // Weighted combination: structure matters most
    category_sim * 0.5 + value_sim * 0.3 + length_sim * 0.2
}

/// Calculate similarity across multiple branches (for chains)
fn calculate_chain_similarity(chains: &[Vec<JsEntropyToken>]) -> f64 {
    if chains.len() < 2 {
        return 0.0;
    }

    let mut total_similarity = 0.0;
    let mut comparisons = 0;

    // Compare all pairs
    for i in 0..chains.len() {
        for j in (i + 1)..chains.len() {
            total_similarity += calculate_sequence_similarity(&chains[i], &chains[j]);
            comparisons += 1;
        }
    }

    if comparisons > 0 {
        total_similarity / comparisons as f64
    } else {
        0.0
    }
}

/// Calculate Jaccard-like similarity between two collections
/// Uses frequency-based comparison to handle duplicates properly
fn jaccard_similarity<T: PartialEq + Clone>(a: &[T], b: &[T]) -> f64 {
    if a.is_empty() && b.is_empty() {
        return 1.0;
    }
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }

    // Count matching elements using position-independent comparison
    // For each unique element in a, count min(count in a, count in b)
    let total_a = a.len();
    let total_b = b.len();
    let mut matched = 0;

    let mut b_matched: Vec<bool> = vec![false; b.len()];

    for item_a in a.iter() {
        for (j, item_b) in b.iter().enumerate() {
            if !b_matched[j] && item_a == item_b {
                matched += 1;
                b_matched[j] = true;
                break;
            }
        }
    }

    // Similarity = 2 * intersection / (|a| + |b|)
    // This is the Dice coefficient, bounded [0, 1]
    let similarity = (2 * matched) as f64 / (total_a + total_b) as f64;
    similarity.min(1.0)
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
    fn test_identical_if_else_branches() {
        let source = r#"
function test(x) {
    if (x) {
        console.log("hello");
        return 1;
    } else {
        console.log("hello");
        return 1;
    }
}
"#;
        let tree = parse_js(source);
        let similarity = analyze_branch_similarity(&tree.root_node(), source);

        assert!(
            similarity > 0.7,
            "Identical branches should have high similarity: {}",
            similarity
        );
    }

    #[test]
    fn test_different_if_else_branches() {
        let source = r#"
function test(x) {
    if (x) {
        const a = fetchData();
        processData(a);
        return transform(a);
    } else {
        throw new Error("invalid");
    }
}
"#;
        let tree = parse_js(source);
        let similarity = analyze_branch_similarity(&tree.root_node(), source);

        assert!(
            similarity < 0.5,
            "Different branches should have low similarity: {}",
            similarity
        );
    }

    #[test]
    fn test_similar_switch_cases() {
        let source = r#"
function handleType(type) {
    switch (type) {
        case 'A': return process(dataA);
        case 'B': return process(dataB);
        case 'C': return process(dataC);
        default: return process(defaultData);
    }
}
"#;
        let tree = parse_js(source);
        let similarity = analyze_branch_similarity(&tree.root_node(), source);

        assert!(
            similarity > 0.5,
            "Similar switch cases should have moderate to high similarity: {}",
            similarity
        );
    }

    #[test]
    fn test_ternary_similarity() {
        let source = "const x = condition ? getValue(a) : getValue(b);";
        let tree = parse_js(source);
        let similarity = analyze_branch_similarity(&tree.root_node(), source);

        assert!(
            similarity > 0.7,
            "Similar ternary branches should have high similarity: {}",
            similarity
        );
    }

    #[test]
    fn test_no_branches() {
        let source = "function simple() { return 42; }";
        let tree = parse_js(source);
        let similarity = analyze_branch_similarity(&tree.root_node(), source);

        assert_eq!(similarity, 0.0, "No branches should return 0");
    }

    #[test]
    fn test_if_else_if_chain_similarity() {
        let source = r#"
function getStatus(code) {
    if (code === 200) {
        return { status: 'ok', message: 'Success' };
    } else if (code === 404) {
        return { status: 'error', message: 'Not found' };
    } else if (code === 500) {
        return { status: 'error', message: 'Server error' };
    } else {
        return { status: 'unknown', message: 'Unknown' };
    }
}
"#;
        let tree = parse_js(source);
        let similarity = analyze_branch_similarity(&tree.root_node(), source);

        assert!(
            similarity > 0.4,
            "Similar if-else-if chain should have moderate similarity: {}",
            similarity
        );
    }
}
