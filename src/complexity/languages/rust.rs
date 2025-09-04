use crate::complexity::entropy_core::{
    EntropyToken, LanguageEntropyAnalyzer, PatternMetrics, TokenCategory,
};
use crate::complexity::entropy_traits::{AnalyzerHelpers, GenericToken};
use crate::complexity::token_classifier::{
    CallType, ClassifiedToken, FlowType, NodeType, TokenClass, TokenClassifier, TokenContext,
};
use std::collections::HashSet;
use syn::visit::Visit;
use syn::{Block, Expr, ItemFn};

/// Rust-specific entropy analyzer implementation
pub struct RustEntropyAnalyzer {
    source_code: String,
    token_classifier: TokenClassifier,
}

impl RustEntropyAnalyzer {
    pub fn new(source_code: String, token_classifier: TokenClassifier) -> Self {
        Self {
            source_code,
            token_classifier,
        }
    }

    /// Extract classified tokens and convert to generic tokens
    fn extract_classified_tokens(&self, item_fn: &ItemFn) -> Vec<ClassifiedToken> {
        let mut visitor = TokenExtractor {
            tokens: Vec::new(),
            classifier: &self.token_classifier,
            source: &self.source_code,
        };
        visitor.visit_item_fn(item_fn);
        visitor.tokens
    }

    /// Count unique variables
    fn count_unique_variables(&self, item_fn: &ItemFn) -> usize {
        let mut visitor = VariableCounter {
            variables: HashSet::new(),
        };
        visitor.visit_item_fn(item_fn);
        visitor.variables.len()
    }

    /// Calculate maximum nesting depth
    fn calculate_max_nesting(&self, item_fn: &ItemFn) -> u32 {
        let mut visitor = NestingCalculator {
            current_depth: 0,
            max_depth: 0,
        };
        visitor.visit_item_fn(item_fn);
        visitor.max_depth
    }

    /// Detect patterns in the function
    fn detect_function_patterns(&self, item_fn: &ItemFn) -> Vec<String> {
        let mut visitor = PatternDetector {
            patterns: Vec::new(),
        };
        visitor.visit_item_fn(item_fn);
        visitor.patterns
    }

    /// Calculate branch similarity for if/else and match arms
    fn calculate_rust_branch_similarity(&self, item_fn: &ItemFn) -> f64 {
        let mut visitor = BranchSimilarityCalculator {
            branch_groups: Vec::new(),
        };
        visitor.visit_item_fn(item_fn);

        if visitor.branch_groups.is_empty() {
            return 0.0;
        }

        let total_similarity: f64 = visitor
            .branch_groups
            .iter()
            .map(|g| g.calculate_similarity())
            .sum();

        (total_similarity / visitor.branch_groups.len() as f64).min(1.0)
    }
}

impl AnalyzerHelpers for RustEntropyAnalyzer {}

impl LanguageEntropyAnalyzer for RustEntropyAnalyzer {
    type AstNode = ItemFn;
    type Token = GenericToken;

    fn extract_tokens(&self, node: &Self::AstNode) -> Vec<Self::Token> {
        let classified = self.extract_classified_tokens(node);
        classified
            .into_iter()
            .map(|ct| {
                let category = match &ct.class {
                    TokenClass::Keyword(_) => TokenCategory::Keyword,
                    TokenClass::Operator(_) => TokenCategory::Operator,
                    TokenClass::LocalVar(_) => TokenCategory::Identifier,
                    TokenClass::Literal(_) => TokenCategory::Literal,
                    TokenClass::ControlFlow(_) => TokenCategory::ControlFlow,
                    TokenClass::MethodCall(_) => TokenCategory::FunctionCall,
                    TokenClass::ExternalAPI(_) => TokenCategory::FunctionCall,
                    _ => TokenCategory::Custom(format!("{:?}", ct.class)),
                };
                GenericToken::new(category, ct.weight, ct.raw_token)
            })
            .collect()
    }

    fn detect_patterns(&self, node: &Self::AstNode) -> PatternMetrics {
        let patterns = self.detect_function_patterns(node);
        let unique_patterns: HashSet<_> = patterns.iter().cloned().collect();

        let mut metrics = PatternMetrics::new();
        metrics.total_patterns = patterns.len();
        metrics.unique_patterns = unique_patterns.len();
        metrics.calculate_repetition();

        metrics
    }

    fn calculate_branch_similarity(&self, node: &Self::AstNode) -> f64 {
        self.calculate_rust_branch_similarity(node)
    }

    fn analyze_structure(&self, node: &Self::AstNode) -> (usize, u32) {
        let unique_vars = self.count_unique_variables(node);
        let max_nesting = self.calculate_max_nesting(node);
        (unique_vars, max_nesting)
    }

    fn generate_cache_key(&self, node: &Self::AstNode) -> String {
        // Generate a unique key based on function signature and body hash
        use sha2::{Digest, Sha256};
        let fn_name = node.sig.ident.to_string();
        let fn_body = format!("{:?}", node.block);
        let mut hasher = Sha256::new();
        hasher.update(fn_body.as_bytes());
        let body_hash = hasher.finalize();
        format!("{}__{:x}", fn_name, body_hash)
    }
}

/// Visitor to extract tokens from Rust AST
struct TokenExtractor<'a> {
    tokens: Vec<ClassifiedToken>,
    classifier: &'a TokenClassifier,
    source: &'a str,
}

impl<'a> Visit<'_> for TokenExtractor<'a> {
    fn visit_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::Binary(e) => {
                let op = format!("{:?}", e.op);
                self.tokens.push(ClassifiedToken {
                    raw_token: op.clone(),
                    class: TokenClass::Operator(op.clone()),
                    weight: 0.8,
                    context: TokenContext {
                        is_method_call: false,
                        is_field_access: false,
                        is_external: false,
                        scope_depth: 0,
                        parent_node_type: NodeType::Expression,
                    },
                });
            }
            Expr::Call(e) => {
                if let Expr::Path(p) = &*e.func {
                    let name = p
                        .path
                        .segments
                        .last()
                        .map(|s| s.ident.to_string())
                        .unwrap_or_else(|| "call".to_string());
                    self.tokens.push(ClassifiedToken {
                        raw_token: name.clone(),
                        class: TokenClass::MethodCall(CallType::Other),
                        weight: 0.9,
                        context: TokenContext {
                            is_method_call: false,
                            is_field_access: false,
                            is_external: false,
                            scope_depth: 0,
                            parent_node_type: NodeType::Expression,
                        },
                    });
                }
            }
            Expr::If(_) => {
                self.tokens.push(ClassifiedToken {
                    raw_token: "if".to_string(),
                    class: TokenClass::ControlFlow(FlowType::If),
                    weight: 1.2,
                    context: TokenContext {
                        is_method_call: false,
                        is_field_access: false,
                        is_external: false,
                        scope_depth: 0,
                        parent_node_type: NodeType::Expression,
                    },
                });
            }
            Expr::Match(_) => {
                self.tokens.push(ClassifiedToken {
                    raw_token: "match".to_string(),
                    class: TokenClass::ControlFlow(FlowType::Match),
                    weight: 1.2,
                    context: TokenContext {
                        is_method_call: false,
                        is_field_access: false,
                        is_external: false,
                        scope_depth: 0,
                        parent_node_type: NodeType::Expression,
                    },
                });
            }
            Expr::While(_) => {
                self.tokens.push(ClassifiedToken {
                    raw_token: "while".to_string(),
                    class: TokenClass::ControlFlow(FlowType::While),
                    weight: 1.2,
                    context: TokenContext {
                        is_method_call: false,
                        is_field_access: false,
                        is_external: false,
                        scope_depth: 0,
                        parent_node_type: NodeType::Expression,
                    },
                });
            }
            Expr::Loop(_) => {
                self.tokens.push(ClassifiedToken {
                    raw_token: "loop".to_string(),
                    class: TokenClass::ControlFlow(FlowType::Loop),
                    weight: 1.2,
                    context: TokenContext {
                        is_method_call: false,
                        is_field_access: false,
                        is_external: false,
                        scope_depth: 0,
                        parent_node_type: NodeType::Expression,
                    },
                });
            }
            Expr::ForLoop(_) => {
                self.tokens.push(ClassifiedToken {
                    raw_token: "for".to_string(),
                    class: TokenClass::ControlFlow(FlowType::For),
                    weight: 1.2,
                    context: TokenContext {
                        is_method_call: false,
                        is_field_access: false,
                        is_external: false,
                        scope_depth: 0,
                        parent_node_type: NodeType::Expression,
                    },
                });
            }
            _ => {}
        }
        syn::visit::visit_expr(self, expr);
    }
}

/// Visitor to count unique variables
struct VariableCounter {
    variables: HashSet<String>,
}

impl Visit<'_> for VariableCounter {
    fn visit_pat_ident(&mut self, node: &syn::PatIdent) {
        self.variables.insert(node.ident.to_string());
        syn::visit::visit_pat_ident(self, node);
    }
}

/// Visitor to calculate nesting depth
struct NestingCalculator {
    current_depth: u32,
    max_depth: u32,
}

impl Visit<'_> for NestingCalculator {
    fn visit_block(&mut self, block: &Block) {
        self.current_depth += 1;
        self.max_depth = self.max_depth.max(self.current_depth);
        syn::visit::visit_block(self, block);
        self.current_depth -= 1;
    }
}

/// Visitor to detect patterns
struct PatternDetector {
    patterns: Vec<String>,
}

impl Visit<'_> for PatternDetector {
    fn visit_expr(&mut self, expr: &Expr) {
        let pattern = match expr {
            Expr::If(_) => "if-stmt",
            Expr::Match(_) => "match",
            Expr::While(_) => "while",
            Expr::Loop(_) => "loop",
            Expr::ForLoop(_) => "for",
            Expr::Call(_) => "call",
            Expr::MethodCall(_) => "method-call",
            Expr::Binary(_) => "binary",
            Expr::Return(_) => "return",
            Expr::Break(_) => "break",
            Expr::Continue(_) => "continue",
            _ => "",
        };

        if !pattern.is_empty() {
            self.patterns.push(pattern.to_string());
        }

        syn::visit::visit_expr(self, expr);
    }
}

/// Branch group for similarity calculation
#[derive(Debug)]
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

    fn calculate_similarity(&self) -> f64 {
        if self.branches.len() < 2 {
            return 0.0;
        }

        let mut total_similarity = 0.0;
        let mut pair_count = 0;

        for i in 0..self.branches.len() {
            for j in i + 1..self.branches.len() {
                total_similarity += Self::sequence_similarity(&self.branches[i], &self.branches[j]);
                pair_count += 1;
            }
        }

        if pair_count > 0 {
            total_similarity / pair_count as f64
        } else {
            0.0
        }
    }

    fn sequence_similarity(seq1: &[String], seq2: &[String]) -> f64 {
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

/// Visitor to calculate branch similarity
struct BranchSimilarityCalculator {
    branch_groups: Vec<BranchGroup>,
}

impl Visit<'_> for BranchSimilarityCalculator {
    fn visit_expr_if(&mut self, node: &syn::ExprIf) {
        let mut group = BranchGroup::new();

        // Extract then branch tokens
        let then_tokens = Self::extract_branch_tokens(&node.then_branch);
        group.add_branch(then_tokens);

        // Extract else branch tokens if exists
        if let Some((_, else_branch)) = &node.else_branch {
            let else_tokens = Self::extract_branch_tokens_from_expr(else_branch);
            group.add_branch(else_tokens);
        }

        if group.branches.len() > 1 {
            self.branch_groups.push(group);
        }

        syn::visit::visit_expr_if(self, node);
    }

    fn visit_expr_match(&mut self, node: &syn::ExprMatch) {
        let mut group = BranchGroup::new();

        for arm in &node.arms {
            let arm_tokens = Self::extract_arm_tokens(arm);
            group.add_branch(arm_tokens);
        }

        if group.branches.len() > 1 {
            self.branch_groups.push(group);
        }

        syn::visit::visit_expr_match(self, node);
    }
}

impl BranchSimilarityCalculator {
    fn extract_branch_tokens(block: &Block) -> Vec<String> {
        let mut tokens = Vec::new();
        for stmt in &block.stmts {
            tokens.push(format!("{:?}", stmt).chars().take(20).collect());
        }
        tokens
    }

    fn extract_branch_tokens_from_expr(expr: &Expr) -> Vec<String> {
        match expr {
            Expr::Block(b) => Self::extract_branch_tokens(&b.block),
            _ => vec![format!("{:?}", expr).chars().take(20).collect()],
        }
    }

    fn extract_arm_tokens(arm: &syn::Arm) -> Vec<String> {
        vec![format!("{:?}", arm.body).chars().take(20).collect()]
    }
}
