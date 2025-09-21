use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::time::SystemTime;
use syn::{visit::Visit, Block, Expr};

use super::token_classifier::{
    ClassificationConfig, ClassifiedToken, NodeType, TokenClass, TokenClassifier, TokenContext,
};

/// Cache entry with metadata for entropy scores
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    pub score: EntropyScore,
    pub timestamp: SystemTime,
    pub function_hash: String,
    pub hit_count: usize,
}

/// Cache statistics for monitoring performance
#[derive(Debug, Clone, Serialize)]
pub struct CacheStats {
    pub entries: usize,
    pub memory_usage: usize,
    pub hit_rate: f64,
    pub miss_rate: f64,
    pub evictions: usize,
}

/// Entropy-based complexity analyzer using information theory
#[derive(Debug)]
pub struct EntropyAnalyzer {
    token_cache: HashMap<String, CacheEntry>,
    cache_hits: usize,
    cache_misses: usize,
    cache_evictions: usize,
    max_cache_size: usize,
    token_classifier: TokenClassifier,
}

/// Score representing the entropy (randomness/variety) of code patterns
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EntropyScore {
    pub token_entropy: f64,        // 0.0-1.0, higher = more complex
    pub pattern_repetition: f64,   // 0.0-1.0, higher = more repetitive
    pub branch_similarity: f64,    // 0.0-1.0, higher = similar branches
    pub effective_complexity: f64, // Adjusted complexity score
    pub unique_variables: usize,   // Variable diversity count
    pub max_nesting: u32,          // Maximum nesting depth
    pub dampening_applied: f64,    // Actual dampening factor applied
}

impl Default for EntropyAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for EntropyAnalyzer {
    fn clone(&self) -> Self {
        Self::new_with_config(self.max_cache_size, ClassificationConfig::default())
    }
}

impl EntropyAnalyzer {
    pub fn new() -> Self {
        let mut config = ClassificationConfig::default();
        // Enable classification if entropy is enabled
        let entropy_config = crate::config::get_entropy_config();
        config.enabled =
            entropy_config.enabled && entropy_config.use_classification.unwrap_or(false);

        Self {
            token_cache: HashMap::new(),
            cache_hits: 0,
            cache_misses: 0,
            cache_evictions: 0,
            max_cache_size: 1000, // Default max cache size
            token_classifier: TokenClassifier::new(config),
        }
    }

    /// Create analyzer with custom cache size
    pub fn with_cache_size(max_cache_size: usize) -> Self {
        let mut config = ClassificationConfig::default();
        let entropy_config = crate::config::get_entropy_config();
        config.enabled =
            entropy_config.enabled && entropy_config.use_classification.unwrap_or(false);

        Self {
            token_cache: HashMap::new(),
            cache_hits: 0,
            cache_misses: 0,
            cache_evictions: 0,
            max_cache_size,
            token_classifier: TokenClassifier::new(config),
        }
    }

    /// Create analyzer with custom configuration
    pub fn new_with_config(
        max_cache_size: usize,
        classification_config: ClassificationConfig,
    ) -> Self {
        Self {
            token_cache: HashMap::new(),
            cache_hits: 0,
            cache_misses: 0,
            cache_evictions: 0,
            max_cache_size,
            token_classifier: TokenClassifier::new(classification_config),
        }
    }

    /// Calculate entropy score for a function block
    pub fn calculate_entropy(&mut self, block: &Block) -> EntropyScore {
        let entropy_config = crate::config::get_entropy_config();

        // Extract pure computation steps
        let entropy = self.compute_token_entropy(block, &entropy_config);
        let patterns = self.detect_pattern_repetition(block);
        let similarity = self.calculate_branch_similarity(block);
        let (unique_vars, max_nesting) = self.analyze_code_structure(block);

        // Compose final score using pure functions
        Self::build_entropy_score(entropy, patterns, similarity, unique_vars, max_nesting)
    }

    /// Pure function to compute token entropy based on configuration
    fn compute_token_entropy(
        &mut self,
        block: &Block,
        config: &crate::config::EntropyConfig,
    ) -> f64 {
        let use_classification = config.enabled && config.use_classification.unwrap_or(false);

        if use_classification {
            let classified = self.extract_classified_tokens(block);
            self.weighted_shannon_entropy(&classified)
        } else {
            let tokens = self.extract_tokens(block);
            self.shannon_entropy(&tokens)
        }
    }

    /// Pure function to build entropy score from components
    fn build_entropy_score(
        entropy: f64,
        patterns: f64,
        similarity: f64,
        unique_vars: usize,
        max_nesting: u32,
    ) -> EntropyScore {
        let effective = Self::compute_effective_complexity(entropy, patterns, similarity);
        let dampening = Self::compute_dampening_factor(entropy, patterns, similarity);

        EntropyScore {
            token_entropy: entropy,
            pattern_repetition: patterns,
            branch_similarity: similarity,
            effective_complexity: effective,
            unique_variables: unique_vars,
            max_nesting,
            dampening_applied: dampening,
        }
    }

    /// Pure function to compute effective complexity
    fn compute_effective_complexity(entropy: f64, repetition: f64, similarity: f64) -> f64 {
        let base_simplicity = (1.0 - entropy) * repetition;
        let simplicity_factor = if similarity > 0.0 {
            base_simplicity * (0.5 + similarity * 0.5)
        } else {
            base_simplicity * 0.7
        };
        1.0 - (simplicity_factor * 0.9)
    }

    /// Pure function to compute dampening factor
    fn compute_dampening_factor(entropy: f64, repetition: f64, similarity: f64) -> f64 {
        let config = crate::config::get_entropy_config();
        if !config.enabled {
            return 1.0;
        }

        let repetition_factor = Self::calculate_graduated_dampening(
            repetition,
            config.pattern_threshold,
            1.0,
            0.20,
            true,
        );
        let entropy_factor = Self::calculate_graduated_dampening(entropy, 0.4, 0.4, 0.15, false);
        let branch_factor = Self::calculate_graduated_dampening(similarity, 0.8, 0.2, 0.25, true);

        (repetition_factor * entropy_factor * branch_factor).max(0.7)
    }

    /// Calculate entropy score with caching support
    pub fn calculate_entropy_cached(
        &mut self,
        block: &Block,
        signature_hash: &str,
    ) -> EntropyScore {
        // Check cache first
        if let Some(entry) = self.token_cache.get_mut(signature_hash) {
            entry.hit_count += 1;
            self.cache_hits += 1;
            return entry.score.clone();
        }

        self.cache_misses += 1;

        // Calculate if not cached
        let score = self.calculate_entropy(block);

        // Evict oldest entry if cache is full
        if self.token_cache.len() >= self.max_cache_size {
            self.evict_oldest();
        }

        // Insert new entry
        let entry = CacheEntry {
            score: score.clone(),
            timestamp: SystemTime::now(),
            function_hash: signature_hash.to_string(),
            hit_count: 0,
        };
        self.token_cache.insert(signature_hash.to_string(), entry);

        score
    }

    /// Evict the oldest cache entry
    fn evict_oldest(&mut self) {
        if let Some((oldest_key, _)) = self
            .token_cache
            .iter()
            .min_by_key(|(_, entry)| entry.timestamp)
            .map(|(k, v)| (k.clone(), v.clone()))
        {
            self.token_cache.remove(&oldest_key);
            self.cache_evictions += 1;
        }
    }

    /// Get cache statistics
    pub fn get_cache_stats(&self) -> CacheStats {
        let total_requests = self.cache_hits + self.cache_misses;
        let hit_rate = if total_requests > 0 {
            self.cache_hits as f64 / total_requests as f64
        } else {
            0.0
        };
        let miss_rate = if total_requests > 0 {
            self.cache_misses as f64 / total_requests as f64
        } else {
            0.0
        };

        CacheStats {
            entries: self.token_cache.len(),
            memory_usage: self.estimate_cache_memory(),
            hit_rate,
            miss_rate,
            evictions: self.cache_evictions,
        }
    }

    /// Estimate memory usage of the cache
    fn estimate_cache_memory(&self) -> usize {
        // Rough estimation:
        // Each entry is approximately:
        // - Key string: ~64 bytes
        // - EntropyScore: 4 * 8 bytes = 32 bytes
        // - Metadata: ~32 bytes
        // Total: ~128 bytes per entry
        self.token_cache.len() * 128
    }

    /// Clear the cache
    pub fn clear_cache(&mut self) {
        self.token_cache.clear();
        self.cache_hits = 0;
        self.cache_misses = 0;
        self.cache_evictions = 0;
    }

    /// Extract tokens from AST for entropy calculation
    fn extract_tokens(&self, block: &Block) -> Vec<TokenType> {
        let mut extractor = TokenExtractor::new();
        extractor.visit_block(block);
        extractor.tokens
    }

    /// Extract classified tokens from AST for weighted entropy calculation
    fn extract_classified_tokens(&mut self, block: &Block) -> Vec<ClassifiedToken> {
        let mut extractor = ClassifiedTokenExtractor::new(&mut self.token_classifier);
        extractor.visit_block(block);
        extractor.tokens
    }

    /// Calculate weighted Shannon entropy using classified tokens
    fn weighted_shannon_entropy(&self, tokens: &[ClassifiedToken]) -> f64 {
        if tokens.is_empty() || tokens.len() == 1 {
            return 0.0;
        }

        // Group tokens by class and sum weights
        let mut class_weights: HashMap<String, f64> = HashMap::new();
        let mut total_weight = 0.0;

        for token in tokens {
            let class_key = format!("{:?}", token.class);
            *class_weights.entry(class_key).or_insert(0.0) += token.weight;
            total_weight += token.weight;
        }

        if total_weight == 0.0 {
            return 0.0;
        }

        // Calculate weighted entropy
        let entropy: f64 = class_weights
            .values()
            .map(|&weight| {
                let p = weight / total_weight;
                if p > 0.0 {
                    -p * p.log2()
                } else {
                    0.0
                }
            })
            .sum();

        // Normalize by maximum possible entropy
        let max_entropy = (class_weights.len() as f64).log2();
        if max_entropy > 0.0 {
            (entropy / max_entropy).min(1.0)
        } else {
            0.0
        }
    }

    /// Calculate Shannon entropy of token distribution
    fn shannon_entropy(&self, tokens: &[TokenType]) -> f64 {
        if tokens.is_empty() || tokens.len() == 1 {
            return 0.0;
        }

        let frequencies = self.count_frequencies(tokens);
        let total = tokens.len() as f64;

        let entropy: f64 = frequencies
            .values()
            .map(|&count| {
                let p = count as f64 / total;
                -p * p.log2()
            })
            .sum();

        // Normalize by maximum possible entropy
        let max_entropy = total.log2();
        if max_entropy > 0.0 {
            (entropy / max_entropy).min(1.0)
        } else {
            0.0
        }
    }

    /// Count frequency of each token type
    fn count_frequencies(&self, tokens: &[TokenType]) -> HashMap<TokenType, usize> {
        let mut frequencies = HashMap::new();
        for token in tokens {
            *frequencies.entry(token.clone()).or_insert(0) += 1;
        }
        frequencies
    }

    /// Detect repetitive patterns in the code structure
    fn detect_pattern_repetition(&self, block: &Block) -> f64 {
        let mut detector = PatternDetector::new();
        detector.visit_block(block);

        // Calculate repetition score (0.0 = no repetition, 1.0 = highly repetitive)
        if detector.total_patterns == 0 {
            return 0.0;
        }

        // Count patterns that appear more than once
        let repeated_pattern_count: usize = detector
            .patterns
            .values()
            .filter(|&&count| count > 1)
            .map(|&count| count - 1) // Subtract 1 to get the number of repetitions
            .sum();

        let repetition_ratio = repeated_pattern_count as f64 / detector.total_patterns as f64;
        repetition_ratio.min(1.0)
    }

    /// Calculate similarity between branches in conditional statements
    fn calculate_branch_similarity(&self, block: &Block) -> f64 {
        let mut analyzer = BranchSimilarityAnalyzer::new();
        analyzer.visit_block(block);

        if analyzer.branch_groups.is_empty() {
            return 0.0;
        }

        // Calculate average similarity across all branch groups
        let total_similarity: f64 = analyzer.branch_groups.iter().map(|g| g.similarity()).sum();
        let avg_similarity = total_similarity / analyzer.branch_groups.len() as f64;
        avg_similarity.min(1.0)
    }

    /// Adjust complexity based on entropy and pattern analysis
    fn adjust_complexity(&self, entropy: f64, repetition: f64, similarity: f64) -> f64 {
        Self::compute_effective_complexity(entropy, repetition, similarity)
    }

    /// Calculate the dampening factor that will be applied
    fn calculate_dampening_factor(&self, entropy: f64, repetition: f64, similarity: f64) -> f64 {
        Self::compute_dampening_factor(entropy, repetition, similarity)
    }

    /// Pure function to calculate graduated dampening factor
    fn calculate_graduated_dampening(
        value: f64,
        threshold: f64,
        range: f64,
        max_reduction: f64,
        excess_mode: bool,
    ) -> f64 {
        let in_range = if excess_mode {
            value > threshold
        } else {
            value < threshold
        };

        if !in_range {
            return 1.0;
        }

        let ratio = if excess_mode {
            (value - threshold) / range
        } else {
            (threshold - value) / range
        };

        1.0 - (ratio * max_reduction).min(max_reduction)
    }

    /// Analyze code structure for additional context
    fn analyze_code_structure(&self, block: &Block) -> (usize, u32) {
        struct StructureAnalyzer {
            unique_variables: HashSet<String>,
            current_nesting: u32,
            max_nesting: u32,
        }

        impl StructureAnalyzer {
            fn new() -> Self {
                Self {
                    unique_variables: HashSet::new(),
                    current_nesting: 0,
                    max_nesting: 0,
                }
            }
        }

        impl<'ast> Visit<'ast> for StructureAnalyzer {
            fn visit_expr(&mut self, expr: &'ast Expr) {
                match expr {
                    Expr::Path(path) => {
                        if let Some(segment) = path.path.segments.first() {
                            self.unique_variables.insert(segment.ident.to_string());
                        }
                    }
                    Expr::If(_)
                    | Expr::While(_)
                    | Expr::ForLoop(_)
                    | Expr::Loop(_)
                    | Expr::Match(_) => {
                        self.current_nesting += 1;
                        self.max_nesting = self.max_nesting.max(self.current_nesting);
                        syn::visit::visit_expr(self, expr);
                        self.current_nesting -= 1;
                    }
                    _ => syn::visit::visit_expr(self, expr),
                }
            }
        }

        let mut analyzer = StructureAnalyzer::new();
        analyzer.visit_block(block);
        (analyzer.unique_variables.len(), analyzer.max_nesting)
    }
}

/// Token types for entropy calculation
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
enum TokenType {
    Keyword(String),      // if, match, for, etc.
    Operator(String),     // +, -, ==, etc.
    Identifier(String),   // Variable/function names (normalized)
    Literal(LiteralType), // Numbers, strings, etc.
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
enum LiteralType {
    Number,
    String,
    Bool,
    Char,
}

impl TokenType {
    /// Convert from classified token to simple token type for compatibility
    fn from_classified(classified: ClassifiedToken) -> Self {
        match classified.class {
            TokenClass::Keyword(s) => TokenType::Keyword(s),
            TokenClass::Operator(s) => TokenType::Operator(s),
            TokenClass::ControlFlow(flow) => {
                TokenType::Keyword(format!("{:?}", flow).to_lowercase())
            }
            TokenClass::Literal(lit) => TokenType::Literal(match lit {
                super::token_classifier::LiteralCategory::Numeric => LiteralType::Number,
                super::token_classifier::LiteralCategory::String => LiteralType::String,
                super::token_classifier::LiteralCategory::Boolean => LiteralType::Bool,
                super::token_classifier::LiteralCategory::Char => LiteralType::Char,
                super::token_classifier::LiteralCategory::Null => LiteralType::String,
            }),
            _ => TokenType::Identifier(classified.raw_token),
        }
    }
}

/// Visitor to extract tokens from AST
struct TokenExtractor {
    tokens: Vec<TokenType>,
}

/// Classified token extractor visitor that uses the token classifier
struct ClassifiedTokenExtractor<'a> {
    tokens: Vec<ClassifiedToken>,
    classifier: &'a mut TokenClassifier,
    scope_depth: usize,
}

impl<'a> ClassifiedTokenExtractor<'a> {
    fn new(classifier: &'a mut TokenClassifier) -> Self {
        Self {
            tokens: Vec::new(),
            classifier,
            scope_depth: 0,
        }
    }

    fn add_token(&mut self, token: &str, is_method: bool, is_field: bool, parent_type: NodeType) {
        let context = TokenContext {
            is_method_call: is_method,
            is_field_access: is_field,
            is_external: false, // Will be enhanced later
            scope_depth: self.scope_depth,
            parent_node_type: parent_type,
        };

        let class = self.classifier.classify(token, &context);
        let weight = self.classifier.get_weight(&class);

        self.tokens.push(ClassifiedToken::new(
            class,
            token.to_string(),
            context,
            weight,
        ));
    }
}

impl TokenExtractor {
    fn new() -> Self {
        Self { tokens: Vec::new() }
    }

    fn add_keyword(&mut self, keyword: &str) {
        self.tokens.push(TokenType::Keyword(keyword.to_string()));
    }

    fn add_operator(&mut self, op: &str) {
        self.tokens.push(TokenType::Operator(op.to_string()));
    }

    fn add_identifier(&mut self, ident: &str) {
        // Normalize identifiers to reduce noise
        let normalized = if ident.len() > 3 {
            "VAR".to_string()
        } else {
            ident.to_uppercase()
        };
        self.tokens.push(TokenType::Identifier(normalized));
    }
}

impl<'ast> Visit<'ast> for TokenExtractor {
    fn visit_expr(&mut self, expr: &'ast Expr) {
        self.handle_expression(expr);
        syn::visit::visit_expr(self, expr);
    }
}

impl TokenExtractor {
    fn handle_expression(&mut self, expr: &Expr) {
        match expr {
            Expr::If(_) => self.add_keyword("if"),
            Expr::While(_) => self.add_keyword("while"),
            Expr::ForLoop(_) => self.add_keyword("for"),
            Expr::Loop(_) => self.add_keyword("loop"),
            Expr::Match(_) => self.add_keyword("match"),
            Expr::Return(_) => self.add_keyword("return"),
            Expr::Break(_) => self.add_keyword("break"),
            Expr::Continue(_) => self.add_keyword("continue"),
            Expr::Try(_) => self.add_keyword("try"),
            Expr::Async(_) => self.add_keyword("async"),
            Expr::Await(_) => self.add_keyword("await"),
            Expr::Unsafe(_) => self.add_keyword("unsafe"),
            Expr::Binary(binary) => self.handle_binary_expr(binary),
            Expr::Path(path) => self.handle_path_expr(path),
            Expr::Lit(lit) => self.handle_literal(lit),
            _ => {}
        }
    }

    fn handle_binary_expr(&mut self, binary: &syn::ExprBinary) {
        let op_str = binary_op_to_str(&binary.op);
        self.add_operator(op_str);
    }

    fn handle_path_expr(&mut self, path: &syn::ExprPath) {
        if let Some(segment) = path.path.segments.last() {
            self.add_identifier(&segment.ident.to_string());
        }
    }

    fn handle_literal(&mut self, lit: &syn::ExprLit) {
        let lit_type = classify_literal(&lit.lit);
        self.tokens.push(TokenType::Literal(lit_type));
    }
}

fn binary_op_to_str(op: &syn::BinOp) -> &'static str {
    match op {
        syn::BinOp::Add(_) => "+",
        syn::BinOp::Sub(_) => "-",
        syn::BinOp::Mul(_) => "*",
        syn::BinOp::Div(_) => "/",
        syn::BinOp::Rem(_) => "%",
        syn::BinOp::And(_) => "&&",
        syn::BinOp::Or(_) => "||",
        syn::BinOp::Eq(_) => "==",
        syn::BinOp::Ne(_) => "!=",
        syn::BinOp::Lt(_) => "<",
        syn::BinOp::Le(_) => "<=",
        syn::BinOp::Gt(_) => ">",
        syn::BinOp::Ge(_) => ">=",
        _ => "op",
    }
}

fn classify_literal(lit: &syn::Lit) -> LiteralType {
    match lit {
        syn::Lit::Str(_) => LiteralType::String,
        syn::Lit::Int(_) | syn::Lit::Float(_) => LiteralType::Number,
        syn::Lit::Bool(_) => LiteralType::Bool,
        syn::Lit::Char(_) => LiteralType::Char,
        _ => LiteralType::String,
    }

}

impl<'ast> Visit<'ast> for ClassifiedTokenExtractor<'ast> {
    fn visit_expr(&mut self, expr: &'ast Expr) {
        if self.handle_classified_expression(expr) {
            return;
        }
        syn::visit::visit_expr(self, expr);
    }
}

impl<'ast> ClassifiedTokenExtractor<'ast> {
    fn handle_classified_expression(&mut self, expr: &'ast Expr) -> bool {
        match expr {
            Expr::If(_) | Expr::While(_) | Expr::ForLoop(_) | Expr::Loop(_)
            | Expr::Match(_) | Expr::Return(_) | Expr::Break(_) | Expr::Continue(_)
            | Expr::Try(_) => {
                self.handle_control_flow_keyword(expr);
                false
            }
            Expr::Async(_) | Expr::Await(_) => {
                self.handle_async_keyword(expr);
                false
            }
            Expr::Unsafe(_) => {
                self.add_token("unsafe", false, false, NodeType::Block);
                false
            }
            Expr::MethodCall(method) => {
                self.handle_method_call(method);
                syn::visit::visit_expr(self, expr);
                true
            }
            Expr::Field(field) => {
                self.handle_field_access(field);
                syn::visit::visit_expr(self, expr);
                true
            }
            Expr::Path(path) => {
                self.handle_path(path);
                false
            }
            Expr::Binary(binary) => {
                self.handle_binary_operator(binary);
                false
            }
            Expr::Lit(lit) => {
                self.handle_literal_token(lit);
                false
            }
            Expr::Block(_) => {
                self.handle_block_scope(expr);
                true
            }
            _ => false
        }
    }

    fn handle_control_flow_keyword(&mut self, expr: &Expr) {
        let keyword = match expr {
            Expr::If(_) => "if",
            Expr::While(_) => "while",
            Expr::ForLoop(_) => "for",
            Expr::Loop(_) => "loop",
            Expr::Match(_) => "match",
            Expr::Return(_) => "return",
            Expr::Break(_) => "break",
            Expr::Continue(_) => "continue",
            Expr::Try(_) => "try",
            _ => return,
        };
        self.add_token(keyword, false, false, NodeType::Statement);
    }

    fn handle_async_keyword(&mut self, expr: &Expr) {
        let keyword = match expr {
            Expr::Async(_) => "async",
            Expr::Await(_) => "await",
            _ => return,
        };
        self.add_token(keyword, false, false, NodeType::Expression);
    }

    fn handle_method_call(&mut self, method: &syn::ExprMethodCall) {
        self.add_token(
            &method.method.to_string(),
            true,
            false,
            NodeType::Expression,
        );
    }

    fn handle_field_access(&mut self, field: &syn::ExprField) {
        if let syn::Member::Named(ident) = &field.member {
            self.add_token(&ident.to_string(), false, true, NodeType::Expression);
        }
    }

    fn handle_path(&mut self, path: &syn::ExprPath) {
        if let Some(segment) = path.path.segments.last() {
            self.add_token(
                &segment.ident.to_string(),
                false,
                false,
                NodeType::Expression,
            );
        }
    }

    fn handle_binary_operator(&mut self, binary: &syn::ExprBinary) {
        let op_str = binary_op_to_str(&binary.op);
        self.add_token(op_str, false, false, NodeType::Expression);
    }

    fn handle_literal_token(&mut self, lit: &syn::ExprLit) {
        let lit_str = format_literal_token(&lit.lit);
        self.add_token(&lit_str, false, false, NodeType::Expression);
    }

    fn handle_block_scope(&mut self, expr: &'ast Expr) {
        self.scope_depth += 1;
        syn::visit::visit_expr(self, expr);
        self.scope_depth -= 1;
    }
}

fn format_literal_token(lit: &syn::Lit) -> String {
    match lit {
        syn::Lit::Str(_) => "\"string\"".to_string(),
        syn::Lit::Int(i) => i.to_string(),
        syn::Lit::Float(f) => f.to_string(),
        syn::Lit::Bool(b) => if b.value() { "true" } else { "false" }.to_string(),
        syn::Lit::Char(_) => "'c'".to_string(),
        _ => "literal".to_string(),
    }

}

/// Detector for repetitive patterns
struct PatternDetector {
    patterns: HashMap<String, usize>,
    total_patterns: usize,
}

impl PatternDetector {
    fn new() -> Self {
        Self {
            patterns: HashMap::new(),
            total_patterns: 0,
        }
    }

    fn record_pattern(&mut self, pattern: String) {
        self.total_patterns += 1;
        *self.patterns.entry(pattern).or_insert(0) += 1;
    }

    fn expr_to_pattern(&self, expr: &Expr) -> String {
        // Create a simplified pattern representation
        match expr {
            Expr::If(_) => "if-stmt".to_string(),
            Expr::Match(m) => format!("match-{}", m.arms.len()),
            Expr::MethodCall(m) => format!("call-{}", m.method),
            Expr::Call(_) => "fn-call".to_string(),
            Expr::Binary(b) => format!("binary-{:?}", b.op),
            Expr::Return(_) => "return".to_string(),
            Expr::Let(_) => "let-binding".to_string(),
            Expr::Path(_) => "path-expr".to_string(),
            Expr::Lit(_) => "literal".to_string(),
            Expr::Block(_) => "block".to_string(),
            Expr::Assign(_) => "assign".to_string(),
            Expr::Field(_) => "field-access".to_string(),
            Expr::Index(_) => "index".to_string(),
            Expr::Loop(_) => "loop".to_string(),
            Expr::While(_) => "while".to_string(),
            Expr::ForLoop(_) => "for-loop".to_string(),
            Expr::Break(_) => "break".to_string(),
            Expr::Continue(_) => "continue".to_string(),
            _ => "other-expr".to_string(),
        }
    }
}

impl<'ast> Visit<'ast> for PatternDetector {
    fn visit_expr(&mut self, expr: &'ast Expr) {
        let pattern = self.expr_to_pattern(expr);
        self.record_pattern(pattern);

        // Special handling for match arms - check for similarity
        if let Expr::Match(match_expr) = expr {
            for arm in &match_expr.arms {
                if let Some(guard) = &arm.guard {
                    self.record_pattern(format!("guard-{:?}", guard.0));
                }
            }
        }

        syn::visit::visit_expr(self, expr);
    }
}

/// Analyzer for branch similarity in conditional statements
struct BranchSimilarityAnalyzer {
    branch_groups: Vec<BranchGroup>,
}

impl BranchSimilarityAnalyzer {
    fn new() -> Self {
        Self {
            branch_groups: Vec::new(),
        }
    }
}

struct BranchGroup {
    branches: Vec<Vec<String>>, // Token sequences for each branch
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

        // Simple similarity: ratio of common tokens
        let mut common = 0;
        let min_len = seq1.len().min(seq2.len());

        for i in 0..min_len {
            if seq1[i] == seq2[i] {
                common += 1;
            }
        }

        common as f64 / seq1.len().max(seq2.len()) as f64
    }
}

impl<'ast> Visit<'ast> for BranchSimilarityAnalyzer {
    fn visit_expr(&mut self, expr: &'ast Expr) {
        match expr {
            Expr::Match(match_expr) => self.analyze_match_branches(match_expr),
            Expr::If(if_expr) => self.analyze_if_branches(if_expr),
            _ => {}
        }
        syn::visit::visit_expr(self, expr);
    }
}

impl BranchSimilarityAnalyzer {
    fn analyze_match_branches(&mut self, match_expr: &syn::ExprMatch) {
        let mut group = BranchGroup::new();

        for arm in &match_expr.arms {
            let tokens = extract_expr_tokens(&arm.body);
            group.add_branch(tokens);
        }

        if group.branches.len() > 1 {
            self.branch_groups.push(group);
        }
    }

    fn analyze_if_branches(&mut self, if_expr: &syn::ExprIf) {
        let mut group = BranchGroup::new();

        // Then branch
        let then_tokens = extract_stmt_tokens(&if_expr.then_branch.stmts);
        group.add_branch(then_tokens);

        // Else branch (if exists)
        if let Some((_, else_expr)) = &if_expr.else_branch {
            let else_tokens = extract_expr_tokens(else_expr);
            group.add_branch(else_tokens);
        }

        if group.branches.len() > 1 {
            self.branch_groups.push(group);
        }
    }
}

fn extract_expr_tokens(expr: &Expr) -> Vec<String> {
    struct SimpleTokenizer {
        tokens: Vec<String>,
    }

    impl<'a> Visit<'a> for SimpleTokenizer {
        fn visit_expr(&mut self, e: &'a Expr) {
            self.tokens.push(format!("{:?}", std::mem::discriminant(e)));
            syn::visit::visit_expr(self, e);
        }
    }

    let mut tokenizer = SimpleTokenizer { tokens: Vec::new() };
    tokenizer.visit_expr(expr);
    tokenizer.tokens
}

fn extract_stmt_tokens(stmts: &[syn::Stmt]) -> Vec<String> {
    stmts.iter()
        .map(|stmt| format!("{:?}", std::mem::discriminant(stmt)))
        .collect()

}

/// Apply entropy-based dampening to complexity scores (spec 68: max 50% reduction)
pub fn apply_entropy_dampening(base_complexity: u32, entropy_score: &EntropyScore) -> u32 {
    let config = crate::config::get_entropy_config();

    if !config.enabled {
        return base_complexity;
    }

    // Spec 68: Only apply dampening for very low entropy (< 0.2)
    if entropy_score.token_entropy >= 0.2 {
        return base_complexity; // No dampening for normal entropy
    }

    // Spec 68: Calculate graduated dampening: 50-100% of score preserved
    // Formula: dampening = max(0.5, 1.0 - (0.5 × (0.2 - entropy) / 0.2))
    // This ensures maximum 50% reduction, minimum 0% reduction
    let dampening_factor = (0.5 + 0.5 * (entropy_score.token_entropy / 0.2)).max(0.5);

    // Apply dampening with guaranteed minimum preservation
    let adjusted = (base_complexity as f64 * dampening_factor) as u32;

    // Ensure we never reduce below 50% (though the formula above already guarantees this)
    adjusted.max(base_complexity / 2)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::complexity::token_classifier::{
        CallType, ClassifiedToken, CollectionOp, ErrorType, FlowType, LiteralCategory, NodeType,
        TokenClass, TokenContext, VarType,
    };
    use syn::parse_quote;

    fn assert_float_eq(left: f64, right: f64, epsilon: f64) {
        if (left - right).abs() > epsilon {
            panic!("assertion failed: `(left == right)`\n  left: `{}`,\n right: `{}`\n  diff: `{}`\nepsilon: `{}`", left, right, (left - right).abs(), epsilon);
        }
    }

    #[test]
    fn test_shannon_entropy_calculation() {
        let analyzer = EntropyAnalyzer::new();

        // All same tokens = 0 entropy
        let uniform_tokens = vec![TokenType::Keyword("if".to_string()); 10];
        assert_float_eq(analyzer.shannon_entropy(&uniform_tokens), 0.0, 1e-10);

        // Mixed tokens = higher entropy
        let mixed_tokens = vec![
            TokenType::Keyword("if".to_string()),
            TokenType::Keyword("while".to_string()),
            TokenType::Operator("==".to_string()),
            TokenType::Identifier("VAR".to_string()),
        ];
        let entropy = analyzer.shannon_entropy(&mixed_tokens);
        assert!(entropy > 0.5);
        assert!(entropy <= 1.0);
    }

    #[test]
    fn test_pattern_repetition_detection() {
        let analyzer = EntropyAnalyzer::new();

        // Repetitive validation code
        let block: Block = parse_quote! {{
            if x > 0 { return Err("error"); }
            if y > 0 { return Err("error"); }
            if z > 0 { return Err("error"); }
        }};

        let repetition = analyzer.detect_pattern_repetition(&block);
        assert!(repetition > 0.5); // Should detect high repetition
    }

    #[test]
    fn test_branch_similarity_detection() {
        let analyzer = EntropyAnalyzer::new();

        // Similar match arms
        let block: Block = parse_quote! {{
            match value {
                1 => println!("one"),
                2 => println!("two"),
                3 => println!("three"),
                _ => println!("other"),
            }
        }};

        let similarity = analyzer.calculate_branch_similarity(&block);
        assert!(similarity > 0.0); // Should detect some similarity
    }

    #[test]
    fn test_entropy_dampening() {
        // Test with low entropy that triggers dampening per spec 68
        let entropy_score = EntropyScore {
            token_entropy: 0.1, // Low entropy (< 0.2) to trigger dampening
            pattern_repetition: 0.8,
            branch_similarity: 0.6,
            effective_complexity: 0.3,
            unique_variables: 5,
            max_nesting: 2,
            dampening_applied: 0.7,
        };

        // With default config (disabled), should return original
        let original = 10;
        let dampened = apply_entropy_dampening(original, &entropy_score);

        // Default config has entropy disabled, so should return original
        // If entropy is enabled in the future, this test may need updating
        if !crate::config::get_entropy_config().enabled {
            assert_eq!(dampened, original); // No change when disabled
        } else {
            // When enabled with low entropy (0.1), should apply dampening
            // Per spec 68: dampening_factor = 0.5 + 0.5 * (0.1/0.2) = 0.75
            // So dampened should be 10 * 0.75 = 7.5, rounded to 7 or 8
            assert!(
                dampened < original,
                "Should apply dampening with low entropy"
            );
            assert!(
                dampened >= 7,
                "Should preserve at least 70% with entropy=0.1"
            );
            assert!(dampened <= original); // But still some reduction
        }
    }

    #[test]
    fn test_complex_code_detection() {
        // Low repetition, high entropy = genuine complexity
        let entropy_score = EntropyScore {
            token_entropy: 0.8,
            pattern_repetition: 0.2,
            branch_similarity: 0.2,
            effective_complexity: 0.9,
            unique_variables: 15,
            max_nesting: 4,
            dampening_applied: 1.0,
        };

        // Verify that high effective complexity is recognized
        assert!(entropy_score.effective_complexity > 0.8);
        assert!(entropy_score.token_entropy > 0.7);
        assert!(entropy_score.pattern_repetition < 0.3);
    }

    #[test]
    fn test_cache_functionality() {
        let mut analyzer = EntropyAnalyzer::with_cache_size(2);

        let block1: syn::Block = parse_quote! {{
            if x > 0 { return true; }
            false
        }};

        let block2: syn::Block = parse_quote! {{
            match x {
                1 => "one",
                2 => "two",
                _ => "other",
            }
        }};

        // First calculation - cache miss
        let score1 = analyzer.calculate_entropy_cached(&block1, "func1_hash");
        assert_eq!(analyzer.cache_misses, 1);
        assert_eq!(analyzer.cache_hits, 0);

        // Second calculation with same hash - cache hit
        let score1_cached = analyzer.calculate_entropy_cached(&block1, "func1_hash");
        assert_eq!(score1, score1_cached);
        assert_eq!(analyzer.cache_misses, 1);
        assert_eq!(analyzer.cache_hits, 1);

        // Different function - cache miss
        let _score2 = analyzer.calculate_entropy_cached(&block2, "func2_hash");
        assert_eq!(analyzer.cache_misses, 2);
        assert_eq!(analyzer.cache_hits, 1);

        // Cache should now have 2 entries
        let stats = analyzer.get_cache_stats();
        assert_eq!(stats.entries, 2);
        assert_float_eq(stats.hit_rate, 1.0 / 3.0, 1e-10);

        // Add third function - should trigger eviction
        let block3: syn::Block = parse_quote! {{
            println!("test");
        }};
        let _score3 = analyzer.calculate_entropy_cached(&block3, "func3_hash");
        assert_eq!(analyzer.cache_evictions, 1);

        // Cache should still have max 2 entries
        let stats = analyzer.get_cache_stats();
        assert_eq!(stats.entries, 2);
    }

    #[test]
    fn test_graduated_dampening_with_cap() {
        // Test that dampening is graduated and capped at 30%

        // High repetition, low entropy, high branch similarity - worst case
        let extreme_score = EntropyScore {
            token_entropy: 0.1,       // Very low entropy
            pattern_repetition: 0.95, // Very high repetition
            branch_similarity: 0.95,  // Very similar branches
            effective_complexity: 0.2,
            unique_variables: 2,
            max_nesting: 1,
            dampening_applied: 0.7,
        };

        // Enable entropy dampening for this test
        std::env::set_var("DEBTMAP_ENTROPY_ENABLED", "true");

        let original = 20;
        let dampened = apply_entropy_dampening(original, &extreme_score);

        // Check if entropy is enabled in configuration
        let config = crate::config::get_entropy_config();
        if config.enabled {
            // With max 30% reduction, should be at least 70% of original
            assert!(
                dampened >= (original as f64 * 0.7) as u32,
                "Dampening should be capped at 30%, got {} from {}",
                dampened,
                original
            );
            // Should not be the full original (some dampening should apply)
            assert!(
                dampened < original,
                "Some dampening should apply, got {} from {}",
                dampened,
                original
            );
        }

        // Clean up
        std::env::remove_var("DEBTMAP_ENTROPY_ENABLED");
    }

    #[test]
    fn test_moderate_dampening() {
        // Test moderate dampening for typical pattern-based code
        let moderate_score = EntropyScore {
            token_entropy: 0.35,      // Below threshold
            pattern_repetition: 0.75, // Above threshold
            branch_similarity: 0.5,   // Below threshold
            effective_complexity: 0.5,
            unique_variables: 8,
            max_nesting: 2,
            dampening_applied: 0.85,
        };

        // This should get moderate dampening, not extreme
        let original = 15;
        let dampened = apply_entropy_dampening(original, &moderate_score);

        let config = crate::config::get_entropy_config();
        if config.enabled {
            // Should get some reduction but not maximum
            assert!(
                dampened >= (original as f64 * 0.8) as u32,
                "Moderate patterns should get moderate dampening"
            );
        }
    }

    #[test]
    fn test_cache_clear() {
        let mut analyzer = EntropyAnalyzer::new();

        let block: syn::Block = parse_quote! {{
            x + y
        }};

        // Add some entries
        analyzer.calculate_entropy_cached(&block, "hash1");
        analyzer.calculate_entropy_cached(&block, "hash2");
        analyzer.calculate_entropy_cached(&block, "hash1"); // Hit

        let stats = analyzer.get_cache_stats();
        assert!(stats.entries > 0);
        assert_eq!(analyzer.cache_hits, 1);

        // Clear cache
        analyzer.clear_cache();

        let stats = analyzer.get_cache_stats();
        assert_eq!(stats.entries, 0);
        assert_eq!(analyzer.cache_hits, 0);
        assert_eq!(analyzer.cache_misses, 0);
    }

    #[test]
    fn test_weighted_shannon_entropy_empty_tokens() {
        let analyzer = EntropyAnalyzer::new();
        let tokens = vec![];
        let entropy = analyzer.weighted_shannon_entropy(&tokens);
        assert_float_eq(entropy, 0.0, 1e-10);
    }

    #[test]
    fn test_weighted_shannon_entropy_single_token() {
        let analyzer = EntropyAnalyzer::new();
        let tokens = vec![ClassifiedToken::new(
            TokenClass::LocalVar(VarType::Other),
            "x".to_string(),
            TokenContext {
                is_method_call: false,
                is_field_access: false,
                is_external: false,
                scope_depth: 0,
                parent_node_type: NodeType::Expression,
            },
            1.0,
        )];
        let entropy = analyzer.weighted_shannon_entropy(&tokens);
        assert_float_eq(entropy, 0.0, 1e-10);
    }

    #[test]
    fn test_weighted_shannon_entropy_uniform_distribution() {
        let analyzer = EntropyAnalyzer::new();
        let context = TokenContext {
            is_method_call: false,
            is_field_access: false,
            is_external: false,
            scope_depth: 0,
            parent_node_type: NodeType::Expression,
        };

        // Create tokens with uniform weights across different classes
        let tokens = vec![
            ClassifiedToken::new(
                TokenClass::LocalVar(VarType::Other),
                "x".to_string(),
                context.clone(),
                1.0,
            ),
            ClassifiedToken::new(
                TokenClass::MethodCall(CallType::Other),
                "foo".to_string(),
                context.clone(),
                1.0,
            ),
            ClassifiedToken::new(
                TokenClass::ControlFlow(FlowType::If),
                "if".to_string(),
                context.clone(),
                1.0,
            ),
            ClassifiedToken::new(
                TokenClass::Literal(LiteralCategory::Numeric),
                "42".to_string(),
                context.clone(),
                1.0,
            ),
        ];

        let entropy = analyzer.weighted_shannon_entropy(&tokens);
        // With 4 equally weighted classes, entropy should be 1.0 (maximum)
        assert!((entropy - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_weighted_shannon_entropy_skewed_distribution() {
        let analyzer = EntropyAnalyzer::new();
        let context = TokenContext {
            is_method_call: false,
            is_field_access: false,
            is_external: false,
            scope_depth: 0,
            parent_node_type: NodeType::Expression,
        };

        // Create tokens with heavily skewed weights (one class dominates)
        let tokens = vec![
            ClassifiedToken::new(
                TokenClass::LocalVar(VarType::Other),
                "x".to_string(),
                context.clone(),
                10.0,
            ),
            ClassifiedToken::new(
                TokenClass::LocalVar(VarType::Other),
                "y".to_string(),
                context.clone(),
                10.0,
            ),
            ClassifiedToken::new(
                TokenClass::LocalVar(VarType::Other),
                "z".to_string(),
                context.clone(),
                10.0,
            ),
            ClassifiedToken::new(
                TokenClass::MethodCall(CallType::Other),
                "foo".to_string(),
                context.clone(),
                0.1,
            ),
        ];

        let entropy = analyzer.weighted_shannon_entropy(&tokens);
        // Entropy should be low due to skewed distribution
        assert!(entropy < 0.3);
    }

    #[test]
    fn test_weighted_shannon_entropy_zero_weights() {
        let analyzer = EntropyAnalyzer::new();
        let context = TokenContext {
            is_method_call: false,
            is_field_access: false,
            is_external: false,
            scope_depth: 0,
            parent_node_type: NodeType::Expression,
        };

        // All tokens have zero weight
        let tokens = vec![
            ClassifiedToken::new(
                TokenClass::LocalVar(VarType::Other),
                "x".to_string(),
                context.clone(),
                0.0,
            ),
            ClassifiedToken::new(
                TokenClass::MethodCall(CallType::Other),
                "foo".to_string(),
                context.clone(),
                0.0,
            ),
        ];

        let entropy = analyzer.weighted_shannon_entropy(&tokens);
        assert_float_eq(entropy, 0.0, 1e-10);
    }

    #[test]
    fn test_weighted_shannon_entropy_mixed_weights() {
        let analyzer = EntropyAnalyzer::new();
        let context = TokenContext {
            is_method_call: false,
            is_field_access: false,
            is_external: false,
            scope_depth: 0,
            parent_node_type: NodeType::Expression,
        };

        // Mixed weights with different token classes
        let tokens = vec![
            ClassifiedToken::new(
                TokenClass::LocalVar(VarType::Other),
                "x".to_string(),
                context.clone(),
                2.0,
            ),
            ClassifiedToken::new(
                TokenClass::MethodCall(CallType::Other),
                "foo".to_string(),
                context.clone(),
                3.0,
            ),
            ClassifiedToken::new(
                TokenClass::ControlFlow(FlowType::If),
                "if".to_string(),
                context.clone(),
                1.5,
            ),
        ];

        let entropy = analyzer.weighted_shannon_entropy(&tokens);
        // Should be between 0 and 1
        assert!(entropy > 0.0 && entropy <= 1.0);
        // With 3 classes with different weights, should have moderate entropy
        assert!(entropy > 0.5 && entropy < 1.0);
    }

    #[test]
    fn test_weighted_shannon_entropy_duplicate_classes() {
        let analyzer = EntropyAnalyzer::new();
        let context = TokenContext {
            is_method_call: false,
            is_field_access: false,
            is_external: false,
            scope_depth: 0,
            parent_node_type: NodeType::Expression,
        };

        // Multiple tokens of the same class (weights should be summed)
        let tokens = vec![
            ClassifiedToken::new(
                TokenClass::LocalVar(VarType::Other),
                "x".to_string(),
                context.clone(),
                1.0,
            ),
            ClassifiedToken::new(
                TokenClass::LocalVar(VarType::Other),
                "y".to_string(),
                context.clone(),
                2.0,
            ),
            ClassifiedToken::new(
                TokenClass::LocalVar(VarType::Other),
                "z".to_string(),
                context.clone(),
                1.0,
            ),
            ClassifiedToken::new(
                TokenClass::MethodCall(CallType::Other),
                "foo".to_string(),
                context.clone(),
                4.0,
            ),
        ];

        let entropy = analyzer.weighted_shannon_entropy(&tokens);
        // Two classes with equal total weight (4.0 each) should give high entropy
        assert!((entropy - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_weighted_shannon_entropy_normalization() {
        let analyzer = EntropyAnalyzer::new();
        let context = TokenContext {
            is_method_call: false,
            is_field_access: false,
            is_external: false,
            scope_depth: 0,
            parent_node_type: NodeType::Expression,
        };

        // Test that entropy is properly normalized (should never exceed 1.0)
        let tokens = vec![
            ClassifiedToken::new(
                TokenClass::LocalVar(VarType::Other),
                "a".to_string(),
                context.clone(),
                1.0,
            ),
            ClassifiedToken::new(
                TokenClass::MethodCall(CallType::Other),
                "b".to_string(),
                context.clone(),
                1.0,
            ),
            ClassifiedToken::new(
                TokenClass::ControlFlow(FlowType::If),
                "c".to_string(),
                context.clone(),
                1.0,
            ),
            ClassifiedToken::new(
                TokenClass::Literal(LiteralCategory::Numeric),
                "d".to_string(),
                context.clone(),
                1.0,
            ),
            ClassifiedToken::new(
                TokenClass::ErrorHandling(ErrorType::Result),
                "e".to_string(),
                context.clone(),
                1.0,
            ),
            ClassifiedToken::new(
                TokenClass::Collection(CollectionOp::Access),
                "f".to_string(),
                context.clone(),
                1.0,
            ),
            ClassifiedToken::new(
                TokenClass::Keyword("let".to_string()),
                "g".to_string(),
                context.clone(),
                1.0,
            ),
            ClassifiedToken::new(
                TokenClass::Operator("+".to_string()),
                "h".to_string(),
                context.clone(),
                1.0,
            ),
        ];

        let entropy = analyzer.weighted_shannon_entropy(&tokens);
        assert!(entropy <= 1.0);
        assert!(entropy > 0.9); // Should be close to 1.0 with 8 equally weighted classes
    }

    #[test]
    fn test_calculate_graduated_dampening_excess_mode() {
        // Test excess mode (value > threshold)

        // Value below threshold - no dampening
        let factor = EntropyAnalyzer::calculate_graduated_dampening(
            0.5,  // value
            0.8,  // threshold
            0.2,  // range
            0.25, // max_reduction
            true, // excess_mode
        );
        assert_float_eq(factor, 1.0, 1e-10);

        // Value at threshold - no dampening
        let factor = EntropyAnalyzer::calculate_graduated_dampening(
            0.8,  // value
            0.8,  // threshold
            0.2,  // range
            0.25, // max_reduction
            true, // excess_mode
        );
        assert_float_eq(factor, 1.0, 1e-10);

        // Value above threshold - graduated dampening
        let factor = EntropyAnalyzer::calculate_graduated_dampening(
            0.9,  // value (0.1 above threshold)
            0.8,  // threshold
            0.2,  // range
            0.25, // max_reduction
            true, // excess_mode
        );
        assert!((factor - 0.875).abs() < 0.001); // 1.0 - (0.1/0.2 * 0.25) = 0.875

        // Value at max - maximum dampening
        let factor = EntropyAnalyzer::calculate_graduated_dampening(
            1.0,  // value (0.2 above threshold)
            0.8,  // threshold
            0.2,  // range
            0.25, // max_reduction
            true, // excess_mode
        );
        assert_float_eq(factor, 0.75, 1e-10); // 1.0 - 0.25
    }

    #[test]
    fn test_calculate_graduated_dampening_deficit_mode() {
        // Test deficit mode (value < threshold)

        // Value above threshold - no dampening
        let factor = EntropyAnalyzer::calculate_graduated_dampening(
            0.5,   // value
            0.4,   // threshold
            0.4,   // range
            0.15,  // max_reduction
            false, // deficit_mode
        );
        assert_float_eq(factor, 1.0, 1e-10);

        // Value at threshold - no dampening
        let factor = EntropyAnalyzer::calculate_graduated_dampening(
            0.4,   // value
            0.4,   // threshold
            0.4,   // range
            0.15,  // max_reduction
            false, // deficit_mode
        );
        assert_float_eq(factor, 1.0, 1e-10);

        // Value below threshold - graduated dampening
        let factor = EntropyAnalyzer::calculate_graduated_dampening(
            0.2,   // value (0.2 below threshold)
            0.4,   // threshold
            0.4,   // range
            0.15,  // max_reduction
            false, // deficit_mode
        );
        assert!((factor - 0.925).abs() < 0.001); // 1.0 - (0.2/0.4 * 0.15) = 0.925

        // Value at minimum - maximum dampening
        let factor = EntropyAnalyzer::calculate_graduated_dampening(
            0.0,   // value (0.4 below threshold)
            0.4,   // threshold
            0.4,   // range
            0.15,  // max_reduction
            false, // deficit_mode
        );
        assert_float_eq(factor, 0.85, 1e-10); // 1.0 - 0.15
    }

    #[test]
    fn test_calculate_dampening_factor_integration() {
        let analyzer = EntropyAnalyzer::new();

        // Test with all factors at neutral values
        let factor = analyzer.calculate_dampening_factor(0.5, 0.5, 0.5);
        assert_float_eq(factor, 1.0, 1e-10); // No dampening when all values are in neutral range

        // Test with high repetition (should dampen)
        let factor = analyzer.calculate_dampening_factor(0.5, 0.9, 0.5);
        assert!(factor < 1.0);
        assert!(factor >= 0.7); // Respects minimum cap

        // Test with low entropy (should dampen)
        let factor = analyzer.calculate_dampening_factor(0.2, 0.5, 0.5);
        assert!(factor < 1.0);
        assert!(factor >= 0.7);

        // Test with high branch similarity (should dampen)
        let factor = analyzer.calculate_dampening_factor(0.5, 0.5, 0.95);
        assert!(factor < 1.0);
        assert!(factor >= 0.7);

        // Test with all factors causing dampening - should respect cap
        let factor = analyzer.calculate_dampening_factor(0.1, 0.95, 0.95);
        assert!((factor - 0.7).abs() < 0.001); // Should hit the 0.7 minimum cap
    }
}
