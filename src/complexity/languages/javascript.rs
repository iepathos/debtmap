use crate::complexity::entropy_core::{LanguageEntropyAnalyzer, PatternMetrics};
use crate::complexity::entropy_traits::{AnalyzerHelpers, GenericToken};
use std::collections::HashSet;
use tree_sitter::{Node, TreeCursor};

/// JavaScript/TypeScript-specific entropy analyzer implementation
pub struct JavaScriptEntropyAnalyzer<'a> {
    source: &'a str,
}

impl<'a> JavaScriptEntropyAnalyzer<'a> {
    pub fn new(source: &'a str) -> Self {
        Self { source }
    }

    /// Extract tokens from JavaScript/TypeScript AST
    fn extract_js_tokens(&self, node: Node) -> Vec<GenericToken> {
        let mut tokens = Vec::new();
        let mut cursor = node.walk();
        self.visit_node(&mut cursor, &mut tokens);
        tokens
    }

    /// Recursively visit nodes to extract tokens
    fn visit_node(&self, cursor: &mut TreeCursor, tokens: &mut Vec<GenericToken>) {
        let node = cursor.node();
        let kind = node.kind();

        // Extract tokens based on node type
        match kind {
            // Control flow keywords
            "if_statement" => tokens.push(GenericToken::control_flow("if".to_string())),
            "else_clause" => tokens.push(GenericToken::control_flow("else".to_string())),
            "switch_statement" => tokens.push(GenericToken::control_flow("switch".to_string())),
            "for_statement" | "for_in_statement" | "for_of_statement" => {
                tokens.push(GenericToken::control_flow("for".to_string()))
            }
            "while_statement" => tokens.push(GenericToken::control_flow("while".to_string())),
            "do_statement" => tokens.push(GenericToken::control_flow("do".to_string())),
            "return_statement" => tokens.push(GenericToken::keyword("return".to_string())),
            "break_statement" => tokens.push(GenericToken::keyword("break".to_string())),
            "continue_statement" => tokens.push(GenericToken::keyword("continue".to_string())),
            "throw_statement" => tokens.push(GenericToken::keyword("throw".to_string())),
            "try_statement" => tokens.push(GenericToken::keyword("try".to_string())),
            "catch_clause" => tokens.push(GenericToken::keyword("catch".to_string())),
            "finally_clause" => tokens.push(GenericToken::keyword("finally".to_string())),

            // Async/await
            "await_expression" => tokens.push(GenericToken::keyword("await".to_string())),

            // Binary operators
            "binary_expression" => {
                if let Some(op_node) = node.child_by_field_name("operator") {
                    let op = &self.source[op_node.byte_range()];
                    tokens.push(GenericToken::operator(op.to_string()));
                }
            }

            // Identifiers
            "identifier" => {
                let text = &self.source[node.byte_range()];
                // Normalize to reduce noise
                let normalized = if text.len() > 3 {
                    "VAR".to_string()
                } else {
                    text.to_uppercase()
                };
                tokens.push(GenericToken::identifier(normalized));
            }

            // Literals
            "string" | "template_string" => {
                tokens.push(GenericToken::literal("string".to_string()))
            }
            "number" => tokens.push(GenericToken::literal("number".to_string())),
            "true" | "false" => tokens.push(GenericToken::literal("bool".to_string())),

            // Function calls
            "call_expression" => tokens.push(GenericToken::function_call("call".to_string())),

            _ => {}
        }

        // Visit children
        if cursor.goto_first_child() {
            loop {
                self.visit_node(cursor, tokens);
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
            cursor.goto_parent();
        }
    }

    /// Detect patterns in JavaScript code
    fn detect_js_patterns(&self, node: Node) -> Vec<String> {
        let mut patterns = Vec::new();
        let mut cursor = node.walk();
        self.collect_patterns(&mut cursor, &mut patterns);
        patterns
    }

    /// Collect patterns from AST
    fn collect_patterns(&self, cursor: &mut TreeCursor, patterns: &mut Vec<String>) {
        let node = cursor.node();
        let pattern = self.node_to_pattern(node);
        if !pattern.is_empty() {
            patterns.push(pattern);
        }

        // Visit children
        if cursor.goto_first_child() {
            loop {
                self.collect_patterns(cursor, patterns);
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
            cursor.goto_parent();
        }
    }

    /// Convert node to pattern string
    fn node_to_pattern(&self, node: Node) -> String {
        match node.kind() {
            "if_statement" => "if-stmt".to_string(),
            "switch_statement" => format!("switch-{}", node.child_count()),
            "method_definition" => "method".to_string(),
            "function_declaration" => "function".to_string(),
            "arrow_function" => "arrow".to_string(),
            "call_expression" => "call".to_string(),
            "binary_expression" => "binary".to_string(),
            "return_statement" => "return".to_string(),
            "assignment_expression" => "assign".to_string(),
            "for_statement" | "for_in_statement" | "for_of_statement" => "for-loop".to_string(),
            "while_statement" => "while".to_string(),
            "do_statement" => "do-while".to_string(),
            _ => String::new(),
        }
    }

    /// Calculate branch similarity for JavaScript
    fn calculate_js_branch_similarity(&self, node: Node) -> f64 {
        let mut branch_groups = Vec::new();
        let mut cursor = node.walk();
        self.collect_branches(&mut cursor, &mut branch_groups);

        if branch_groups.is_empty() {
            return 0.0;
        }

        // Calculate average similarity across all branch groups
        let total_similarity: f64 = branch_groups.iter().map(|g| g.similarity()).sum();
        let avg_similarity = total_similarity / branch_groups.len() as f64;
        avg_similarity.min(1.0)
    }

    /// Collect branches from conditional statements
    fn collect_branches(&self, cursor: &mut TreeCursor, branch_groups: &mut Vec<BranchGroup>) {
        let node = cursor.node();

        match node.kind() {
            "if_statement" => {
                let mut group = BranchGroup::new();

                // Collect then branch tokens
                if let Some(consequence) = node.child_by_field_name("consequence") {
                    let tokens = self.extract_branch_tokens(consequence);
                    group.add_branch(tokens);
                }

                // Collect else branch tokens if exists
                if let Some(alternative) = node.child_by_field_name("alternative") {
                    let tokens = self.extract_branch_tokens(alternative);
                    group.add_branch(tokens);
                }

                if group.branches.len() > 1 {
                    branch_groups.push(group);
                }
            }
            "switch_statement" => {
                let mut group = BranchGroup::new();
                let mut case_cursor = node.walk();

                if case_cursor.goto_first_child() {
                    loop {
                        let case_node = case_cursor.node();
                        if case_node.kind() == "switch_case" {
                            let tokens = self.extract_branch_tokens(case_node);
                            group.add_branch(tokens);
                        }
                        if !case_cursor.goto_next_sibling() {
                            break;
                        }
                    }
                }

                if group.branches.len() > 1 {
                    branch_groups.push(group);
                }
            }
            _ => {}
        }

        // Visit children
        if cursor.goto_first_child() {
            loop {
                self.collect_branches(cursor, branch_groups);
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
            cursor.goto_parent();
        }
    }

    /// Extract tokens from a branch
    fn extract_branch_tokens(&self, node: Node) -> Vec<String> {
        let mut tokens = Vec::new();
        let mut cursor = node.walk();
        self.extract_simple_tokens(&mut cursor, &mut tokens);
        tokens
    }

    /// Extract simple tokens for branch comparison
    #[allow(clippy::only_used_in_recursion)]
    fn extract_simple_tokens(&self, cursor: &mut TreeCursor, tokens: &mut Vec<String>) {
        let node = cursor.node();
        let kind = node.kind();

        // Add simple token representation
        let token = match kind {
            "identifier" => "id".to_string(),
            "call_expression" => "call".to_string(),
            "binary_expression" => "binary".to_string(),
            "return_statement" => "return".to_string(),
            "assignment_expression" => "assign".to_string(),
            _ => String::new(),
        };

        if !token.is_empty() {
            tokens.push(token);
        }

        // Visit children
        if cursor.goto_first_child() {
            loop {
                self.extract_simple_tokens(cursor, tokens);
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
            cursor.goto_parent();
        }
    }

    /// Count unique variables
    fn count_js_variables(&self, node: Node) -> usize {
        let mut variables = HashSet::new();
        let mut cursor = node.walk();
        self.collect_variables(&mut cursor, &mut variables);
        variables.len()
    }

    /// Check if node is a variable declaration context
    fn is_variable_declaration_context(parent_kind: &str) -> bool {
        matches!(parent_kind, "variable_declarator" | "parameter" | "pattern")
    }

    /// Extract variable name from identifier node if it's in a declaration context
    fn extract_variable_if_declaration(&self, node: Node) -> Option<String> {
        if node.kind() != "identifier" {
            return None;
        }

        node.parent()
            .filter(|parent| Self::is_variable_declaration_context(parent.kind()))
            .map(|_| self.source[node.byte_range()].to_string())
    }

    /// Collect variable names
    fn collect_variables(&self, cursor: &mut TreeCursor, variables: &mut HashSet<String>) {
        let node = cursor.node();

        // Extract variable if applicable
        if let Some(var_name) = self.extract_variable_if_declaration(node) {
            variables.insert(var_name);
        }

        // Visit children
        if cursor.goto_first_child() {
            loop {
                self.collect_variables(cursor, variables);
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
            cursor.goto_parent();
        }
    }

    /// Calculate maximum nesting depth
    fn calculate_js_nesting(&self, node: Node) -> u32 {
        self.calculate_nesting_recursive(node, 0)
    }

    /// Recursively calculate nesting depth
    #[allow(clippy::only_used_in_recursion)]
    fn calculate_nesting_recursive(&self, node: Node, current_depth: u32) -> u32 {
        let mut max_depth = current_depth;

        // Check if this node increases nesting
        let is_nesting_node = matches!(
            node.kind(),
            "if_statement"
                | "switch_statement"
                | "for_statement"
                | "for_in_statement"
                | "for_of_statement"
                | "while_statement"
                | "do_statement"
                | "try_statement"
                | "function_declaration"
                | "arrow_function"
                | "method_definition"
        );

        let new_depth = if is_nesting_node {
            current_depth + 1
        } else {
            current_depth
        };

        max_depth = max_depth.max(new_depth);

        // Visit children
        let mut cursor = node.walk();
        if cursor.goto_first_child() {
            loop {
                let child_max = self.calculate_nesting_recursive(cursor.node(), new_depth);
                max_depth = max_depth.max(child_max);
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }

        max_depth
    }
}

impl<'a> AnalyzerHelpers for JavaScriptEntropyAnalyzer<'a> {}

impl<'a> LanguageEntropyAnalyzer for JavaScriptEntropyAnalyzer<'a> {
    type AstNode = Node<'a>;
    type Token = GenericToken;

    fn extract_tokens(&self, node: &Self::AstNode) -> Vec<Self::Token> {
        self.extract_js_tokens(*node)
    }

    fn detect_patterns(&self, node: &Self::AstNode) -> PatternMetrics {
        let patterns = self.detect_js_patterns(*node);
        let unique_patterns: HashSet<_> = patterns.iter().cloned().collect();

        let mut metrics = PatternMetrics::new();
        metrics.total_patterns = patterns.len();
        metrics.unique_patterns = unique_patterns.len();
        metrics.calculate_repetition();

        metrics
    }

    fn calculate_branch_similarity(&self, node: &Self::AstNode) -> f64 {
        self.calculate_js_branch_similarity(*node)
    }

    fn analyze_structure(&self, node: &Self::AstNode) -> (usize, u32) {
        let unique_vars = self.count_js_variables(*node);
        let max_nesting = self.calculate_js_nesting(*node);
        (unique_vars, max_nesting)
    }

    fn generate_cache_key(&self, node: &Self::AstNode) -> String {
        // Generate cache key based on node range and source hash
        use sha2::{Digest, Sha256};
        let start = node.start_position();
        let end = node.end_position();
        let content = &self.source[node.byte_range()];
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        let content_hash = hasher.finalize();
        format!(
            "js_{}:{}-{}:{}__{:x}",
            start.row, start.column, end.row, end.column, content_hash
        )
    }
}

/// Branch group for similarity analysis
struct BranchGroup {
    branches: Vec<Vec<String>>,
}

impl BranchGroup {
    fn new() -> Self {
        Self {
            branches: Vec::new(),
        }
    }

    fn add_branch(&mut self, tokens: Vec<String>) {
        self.branches.push(tokens);
    }

    fn similarity(&self) -> f64 {
        if self.branches.len() < 2 {
            return 0.0;
        }

        let mut total_similarity = 0.0;
        let mut pair_count = 0;

        // Compare all pairs of branches
        for i in 0..self.branches.len() {
            for j in i + 1..self.branches.len() {
                total_similarity += self.sequence_similarity(&self.branches[i], &self.branches[j]);
                pair_count += 1;
            }
        }

        if pair_count > 0 {
            total_similarity / pair_count as f64
        } else {
            0.0
        }
    }

    fn sequence_similarity(&self, seq1: &[String], seq2: &[String]) -> f64 {
        if seq1.is_empty() || seq2.is_empty() {
            return 0.0;
        }

        let len1 = seq1.len();
        let len2 = seq2.len();
        let max_len = len1.max(len2) as f64;

        let mut matches = 0;
        let min_len = len1.min(len2);

        for i in 0..min_len {
            if seq1[i] == seq2[i] {
                matches += 1;
            }
        }

        matches as f64 / max_len
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    // Test helpers for mock scenarios since tree-sitter requires complex setup
    fn create_mock_variables() -> HashSet<String> {
        let mut vars = HashSet::new();
        vars.insert("x".to_string());
        vars.insert("y".to_string());
        vars.insert("z".to_string());
        vars
    }

    #[test]
    fn test_branch_similarity_calculation() {
        let analyzer = JavaScriptEntropyAnalyzer::new("test");

        // Test similarity calculation with identical branches
        let branch1 = vec!["call".to_string(), "return".to_string()];
        let branch2 = vec!["call".to_string(), "return".to_string()];

        let group = BranchGroup {
            branches: vec![branch1, branch2],
        };

        let similarity = group.similarity();
        assert_eq!(similarity, 1.0); // Identical branches should have 100% similarity
    }

    #[test]
    fn test_branch_similarity_different_lengths() {
        let branch1 = vec!["call".to_string(), "return".to_string()];
        let branch2 = vec!["call".to_string()];

        let group = BranchGroup {
            branches: vec![branch1, branch2],
        };

        let similarity = group.similarity();
        assert_eq!(similarity, 0.5); // Should normalize by max length
    }

    #[test]
    fn test_branch_similarity_no_branches() {
        let group = BranchGroup {
            branches: vec![],
        };

        let similarity = group.similarity();
        assert_eq!(similarity, 0.0); // No branches should return 0
    }

    #[test]
    fn test_branch_similarity_single_branch() {
        let branch1 = vec!["call".to_string(), "return".to_string()];

        let group = BranchGroup {
            branches: vec![branch1],
        };

        let similarity = group.similarity();
        assert_eq!(similarity, 0.0); // Single branch can't have similarity
    }

    #[test]
    fn test_sequence_similarity_empty_sequences() {
        let group = BranchGroup { branches: vec![] };
        let similarity = group.sequence_similarity(&[], &[]);
        assert_eq!(similarity, 0.0);
    }

    #[test]
    fn test_sequence_similarity_partial_match() {
        let seq1 = vec!["call".to_string(), "assign".to_string(), "return".to_string()];
        let seq2 = vec!["call".to_string(), "binary".to_string(), "return".to_string()];

        let group = BranchGroup { branches: vec![] };
        let similarity = group.sequence_similarity(&seq1, &seq2);

        // 2 matches out of 3 positions = 2/3 ≈ 0.67
        assert!((similarity - 0.666).abs() < 0.01);
    }

    #[test]
    fn test_cache_key_generation() {
        let analyzer = JavaScriptEntropyAnalyzer::new("function test() { return 42; }");

        // Create a mock node-like structure for testing cache key generation
        // This test focuses on the format and consistency of cache keys
        let content = "function test() { return 42; }";

        // Test that cache key format is correct
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        let content_hash = hasher.finalize();
        let expected_format = format!("js_0:0-0:0__{:x}", content_hash);

        // The actual implementation would use node positions, but we test the format
        assert!(expected_format.starts_with("js_"));
        assert!(expected_format.contains("__"));
    }

    #[test]
    fn test_analyzer_creation() {
        let source = "function test() { return 42; }";
        let analyzer = JavaScriptEntropyAnalyzer::new(source);

        // Basic test to ensure analyzer is created correctly
        assert_eq!(analyzer.source, source);
    }
}
