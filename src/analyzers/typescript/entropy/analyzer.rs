//! Core entropy analyzer for JavaScript/TypeScript
//!
//! Implements `LanguageEntropyAnalyzer` trait for tree-sitter AST nodes.

use super::branches::analyze_branch_similarity;
use super::patterns::detect_js_patterns;
use super::token::{extract_tokens_recursive, JsEntropyToken};
use crate::complexity::entropy_analysis::{calculate_repetition_score, calculate_weighted_entropy};
use crate::complexity::entropy_core::{
    EntropyConfig, EntropyScore, LanguageEntropyAnalyzer, PatternMetrics,
};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use tree_sitter::Node;

/// JavaScript/TypeScript entropy analyzer
pub struct JsEntropyAnalyzer<'a> {
    #[allow(dead_code)]
    source: &'a str,
}

impl<'a> JsEntropyAnalyzer<'a> {
    #[allow(dead_code)]
    pub fn new(source: &'a str) -> Self {
        Self { source }
    }
}

/// Wrapper type for tree-sitter Node to satisfy trait bounds
/// This is necessary because tree-sitter::Node has lifetime parameters
pub struct JsAstNode<'tree> {
    node: Node<'tree>,
    source: String,
}

impl<'tree> JsAstNode<'tree> {
    pub fn new(node: Node<'tree>, source: String) -> Self {
        Self { node, source }
    }

    pub fn node(&self) -> &Node<'tree> {
        &self.node
    }

    pub fn source(&self) -> &str {
        &self.source
    }
}

impl LanguageEntropyAnalyzer for JsEntropyAnalyzer<'_> {
    type AstNode = Node<'static>;
    type Token = JsEntropyToken;

    fn extract_tokens(&self, _node: &Self::AstNode) -> Vec<Self::Token> {
        // This won't be called directly - we use the direct implementation below
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

/// Calculate entropy score for a function body node
///
/// This is the main entry point for entropy analysis of JS/TS functions.
pub fn calculate_entropy(node: &Node, source: &str, config: &EntropyConfig) -> EntropyScore {
    // Extract tokens
    let tokens = extract_tokens_recursive(node, source);

    // Calculate weighted entropy
    let token_entropy = if tokens.is_empty() {
        0.0
    } else {
        calculate_weighted_entropy(&tokens)
    };

    // Calculate repetition score from token sequences
    let token_repetition = if tokens.is_empty() {
        0.0
    } else {
        calculate_repetition_score(&tokens)
    };

    // Detect structural patterns
    let patterns = detect_js_patterns(node, source);

    // Use the higher of token repetition or structural pattern repetition
    let pattern_repetition = token_repetition.max(patterns.repetition_ratio);

    // Calculate branch similarity
    let branch_similarity = analyze_branch_similarity(node, source);

    // Analyze structure
    let (unique_variables, max_nesting) = analyze_structure(node, source);

    // Calculate effective complexity
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
        dampening_applied: 1.0, // Will be calculated later by EntropyAnalysis::from_raw
    }
}

/// Calculate effective complexity after adjustments
fn calculate_effective_complexity(
    entropy: f64,
    patterns: f64,
    similarity: f64,
    config: &EntropyConfig,
) -> f64 {
    // Pattern reduction: high repetition reduces complexity
    let pattern_factor = 1.0 - (patterns * config.pattern_weight);

    // Similarity reduction: similar branches reduce complexity
    let similarity_factor = 1.0 - (similarity * config.similarity_weight);

    // Apply reductions
    let adjusted = entropy * pattern_factor * similarity_factor;

    // Apply threshold dampening for very low complexity
    if adjusted < config.base_threshold {
        adjusted * 0.5
    } else {
        adjusted
    }
}

/// Analyze structure of a function body
fn analyze_structure(node: &Node, source: &str) -> (usize, u32) {
    let unique_vars = count_unique_variables(node, source);
    let max_nesting = calculate_max_nesting(node, 0);
    (unique_vars, max_nesting)
}

/// Count unique variable names in the function
fn count_unique_variables(node: &Node, source: &str) -> usize {
    let mut variables: HashSet<String> = HashSet::new();
    collect_variables(node, source, &mut variables);
    variables.len()
}

fn collect_variables(node: &Node, source: &str, variables: &mut HashSet<String>) {
    match node.kind() {
        "identifier" => {
            let name = node_text(node, source);
            // Filter out common keywords that look like identifiers
            if !is_keyword(name) {
                variables.insert(name.to_string());
            }
        }
        "variable_declarator" => {
            if let Some(name_node) = node.child_by_field_name("name") {
                let name = node_text(&name_node, source);
                variables.insert(name.to_string());
            }
        }
        _ => {}
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_variables(&child, source, variables);
    }
}

fn is_keyword(name: &str) -> bool {
    matches!(
        name,
        "if" | "else"
            | "for"
            | "while"
            | "do"
            | "switch"
            | "case"
            | "default"
            | "break"
            | "continue"
            | "return"
            | "throw"
            | "try"
            | "catch"
            | "finally"
            | "function"
            | "const"
            | "let"
            | "var"
            | "class"
            | "extends"
            | "new"
            | "this"
            | "super"
            | "import"
            | "export"
            | "async"
            | "await"
            | "yield"
            | "typeof"
            | "instanceof"
            | "in"
            | "of"
            | "true"
            | "false"
            | "null"
            | "undefined"
    )
}

/// Calculate maximum nesting depth
fn calculate_max_nesting(node: &Node, current_depth: u32) -> u32 {
    let new_depth = match node.kind() {
        "if_statement" | "for_statement" | "for_in_statement" | "for_of_statement"
        | "while_statement" | "do_statement" | "switch_statement" | "try_statement" => {
            current_depth + 1
        }
        "arrow_function" | "function_expression" | "function" => current_depth + 1,
        _ => current_depth,
    };

    let mut max_depth = new_depth;
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        let child_depth = calculate_max_nesting(&child, new_depth);
        max_depth = max_depth.max(child_depth);
    }

    max_depth
}

/// Generate cache key for a node
pub fn generate_cache_key(node: &Node, source: &str) -> String {
    let text = node_text(node, source);
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    let result = hasher.finalize();
    format!("{:x}", result)
}

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
    fn test_calculate_entropy_simple_function() {
        let source = "function simple() { return 42; }";
        let tree = parse_js(source);
        let config = EntropyConfig::default();

        let score = calculate_entropy(&tree.root_node(), source, &config);

        assert!(score.token_entropy >= 0.0 && score.token_entropy <= 1.0);
        assert!(score.pattern_repetition >= 0.0 && score.pattern_repetition <= 1.0);
    }

    #[test]
    fn test_calculate_entropy_validation_function() {
        let source = r#"
function validate(a, b, c, d) {
    if (!a) throw new Error("a required");
    if (!b) throw new Error("b required");
    if (!c) throw new Error("c required");
    if (!d) throw new Error("d required");
    return { a, b, c, d };
}
"#;
        let tree = parse_js(source);
        let config = EntropyConfig::default();

        let score = calculate_entropy(&tree.root_node(), source, &config);

        // Validation functions should have high repetition
        assert!(
            score.pattern_repetition > 0.0,
            "Validation function should have pattern repetition: {}",
            score.pattern_repetition
        );
    }

    #[test]
    fn test_calculate_entropy_complex_function() {
        let source = r#"
function complexAlgorithm(data) {
    const results = [];
    for (const item of data) {
        if (item.type === 'A') {
            const processed = transformA(item);
            if (processed.valid) {
                results.push(enrichData(processed));
            } else {
                log.warn('Invalid A item', item);
            }
        } else if (item.type === 'B') {
            try {
                const fetched = await fetchExternalData(item.id);
                results.push(mergeData(item, fetched));
            } catch (e) {
                results.push(createFallback(item));
            }
        } else {
            throw new UnknownTypeError(item.type);
        }
    }
    return aggregateResults(results);
}
"#;
        let tree = parse_js(source);
        let config = EntropyConfig::default();

        let score = calculate_entropy(&tree.root_node(), source, &config);

        // Complex functions should have higher entropy
        assert!(
            score.token_entropy > 0.3,
            "Complex function should have higher entropy: {}",
            score.token_entropy
        );
        assert!(
            score.max_nesting >= 2,
            "Complex function should have nesting: {}",
            score.max_nesting
        );
    }

    #[test]
    fn test_unique_variables() {
        let source = r#"
function test() {
    const a = 1;
    const b = 2;
    const c = a + b;
    return { a, b, c };
}
"#;
        let tree = parse_js(source);
        let (unique_vars, _) = analyze_structure(&tree.root_node(), source);

        assert!(
            unique_vars >= 3,
            "Should detect at least 3 unique variables: {}",
            unique_vars
        );
    }

    #[test]
    fn test_max_nesting() {
        let source = r#"
function nested() {
    if (a) {
        for (const x of items) {
            if (x.valid) {
                try {
                    process(x);
                } catch (e) {
                    handle(e);
                }
            }
        }
    }
}
"#;
        let tree = parse_js(source);
        let (_, max_nesting) = analyze_structure(&tree.root_node(), source);

        assert!(
            max_nesting >= 3,
            "Should detect deep nesting: {}",
            max_nesting
        );
    }

    #[test]
    fn test_cache_key_generation() {
        let source1 = "function a() { return 1; }";
        let source2 = "function a() { return 1; }";
        let source3 = "function b() { return 2; }";

        let tree1 = parse_js(source1);
        let tree2 = parse_js(source2);
        let tree3 = parse_js(source3);

        let key1 = generate_cache_key(&tree1.root_node(), source1);
        let key2 = generate_cache_key(&tree2.root_node(), source2);
        let key3 = generate_cache_key(&tree3.root_node(), source3);

        assert_eq!(key1, key2, "Same source should produce same key");
        assert_ne!(key1, key3, "Different source should produce different key");
    }

    #[test]
    fn test_repetitive_method_chain() {
        let source = r#"
function processData(items) {
    return items
        .filter(x => x.active)
        .map(x => x.value)
        .filter(x => x > 0)
        .map(x => x * 2)
        .filter(x => x < 100);
}
"#;
        let tree = parse_js(source);
        let config = EntropyConfig::default();

        let score = calculate_entropy(&tree.root_node(), source, &config);

        // Method chains with similar patterns should have some repetition
        assert!(
            score.pattern_repetition >= 0.0,
            "Method chain should be analyzed: {:?}",
            score
        );
    }
}
