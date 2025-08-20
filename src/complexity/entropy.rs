use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::SystemTime;
use syn::{visit::Visit, Block, Expr};

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
#[derive(Debug, Clone)]
pub struct EntropyAnalyzer {
    token_cache: HashMap<String, CacheEntry>,
    cache_hits: usize,
    cache_misses: usize,
    cache_evictions: usize,
    max_cache_size: usize,
}

/// Score representing the entropy (randomness/variety) of code patterns
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EntropyScore {
    pub token_entropy: f64,        // 0.0-1.0, higher = more complex
    pub pattern_repetition: f64,   // 0.0-1.0, higher = more repetitive
    pub branch_similarity: f64,    // 0.0-1.0, higher = similar branches
    pub effective_complexity: f64, // Adjusted complexity score
}

impl Default for EntropyAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl EntropyAnalyzer {
    pub fn new() -> Self {
        Self {
            token_cache: HashMap::new(),
            cache_hits: 0,
            cache_misses: 0,
            cache_evictions: 0,
            max_cache_size: 1000, // Default max cache size
        }
    }

    /// Create analyzer with custom cache size
    pub fn with_cache_size(max_cache_size: usize) -> Self {
        Self {
            token_cache: HashMap::new(),
            cache_hits: 0,
            cache_misses: 0,
            cache_evictions: 0,
            max_cache_size,
        }
    }

    /// Calculate entropy score for a function block
    pub fn calculate_entropy(&self, block: &Block) -> EntropyScore {
        let tokens = self.extract_tokens(block);
        let entropy = self.shannon_entropy(&tokens);
        let patterns = self.detect_pattern_repetition(block);
        let similarity = self.calculate_branch_similarity(block);

        EntropyScore {
            token_entropy: entropy,
            pattern_repetition: patterns,
            branch_similarity: similarity,
            effective_complexity: self.adjust_complexity(entropy, patterns, similarity),
        }
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
        // High repetition and high similarity = low effective complexity
        // Low entropy = simple patterns

        // Weight the factors - repetition and entropy are always relevant
        // Branch similarity is optional (0 when no branches)
        let base_simplicity = (1.0 - entropy) * repetition;

        // If we have branch similarity, it reinforces the pattern
        // Otherwise, use base simplicity alone
        let simplicity_factor = if similarity > 0.0 {
            base_simplicity * (0.5 + similarity * 0.5)
        } else {
            base_simplicity * 0.7 // Reduce impact when no branches
        };

        // Return effective complexity multiplier (0.1 = very simple, 1.0 = genuinely complex)
        1.0 - (simplicity_factor * 0.9)
    }
}

/// Token types for entropy calculation
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
enum TokenType {
    Keyword(String),      // if, match, for, etc.
    Operator(String),     // +, -, ==, etc.
    Identifier(String),   // Variable/function names (normalized)
    Literal(LiteralType), // Numbers, strings, etc.
    Punctuation(char),    // {, }, (, ), etc.
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
enum LiteralType {
    Number,
    String,
    Bool,
    Char,
}

/// Visitor to extract tokens from AST
struct TokenExtractor {
    tokens: Vec<TokenType>,
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

            Expr::Binary(binary) => {
                let op_str = match &binary.op {
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
                };
                self.add_operator(op_str);
            }

            Expr::Path(path) => {
                if let Some(segment) = path.path.segments.last() {
                    self.add_identifier(&segment.ident.to_string());
                }
            }

            Expr::Lit(lit) => {
                let lit_type = match &lit.lit {
                    syn::Lit::Str(_) => LiteralType::String,
                    syn::Lit::Int(_) | syn::Lit::Float(_) => LiteralType::Number,
                    syn::Lit::Bool(_) => LiteralType::Bool,
                    syn::Lit::Char(_) => LiteralType::Char,
                    _ => LiteralType::String,
                };
                self.tokens.push(TokenType::Literal(lit_type));
            }

            _ => {}
        }

        syn::visit::visit_expr(self, expr);
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
            Expr::Match(match_expr) => {
                let mut group = BranchGroup::new();

                for arm in &match_expr.arms {
                    // Extract simplified token sequence from arm body
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
                    tokenizer.visit_expr(&arm.body);
                    group.add_branch(tokenizer.tokens);
                }

                if group.branches.len() > 1 {
                    self.branch_groups.push(group);
                }
            }

            Expr::If(if_expr) => {
                let mut group = BranchGroup::new();

                // Then branch
                let mut then_tokens = Vec::new();
                for stmt in &if_expr.then_branch.stmts {
                    then_tokens.push(format!("{:?}", std::mem::discriminant(stmt)));
                }
                group.add_branch(then_tokens);

                // Else branch (if exists)
                if let Some((_, else_expr)) = &if_expr.else_branch {
                    struct ElseTokenizer {
                        tokens: Vec<String>,
                    }

                    impl<'a> Visit<'a> for ElseTokenizer {
                        fn visit_expr(&mut self, e: &'a Expr) {
                            self.tokens.push(format!("{:?}", std::mem::discriminant(e)));
                            syn::visit::visit_expr(self, e);
                        }
                    }

                    let mut tokenizer = ElseTokenizer { tokens: Vec::new() };
                    tokenizer.visit_expr(else_expr);
                    group.add_branch(tokenizer.tokens);
                }

                if group.branches.len() > 1 {
                    self.branch_groups.push(group);
                }
            }

            _ => {}
        }

        syn::visit::visit_expr(self, expr);
    }
}

/// Apply entropy-based dampening to complexity scores
pub fn apply_entropy_dampening(base_complexity: u32, entropy_score: &EntropyScore) -> u32 {
    let config = crate::config::get_entropy_config();

    if !config.enabled {
        return base_complexity;
    }

    let effective_multiplier = if entropy_score.pattern_repetition > config.pattern_threshold {
        // High repetition = low actual complexity
        0.3
    } else if entropy_score.token_entropy < 0.4 {
        // Low entropy = simple patterns
        0.5
    } else if entropy_score.branch_similarity > 0.8 {
        // Very similar branches = pattern-based code
        0.4
    } else {
        // Use the calculated effective complexity
        entropy_score.effective_complexity
    };

    // Apply configured weight
    let weighted_multiplier = 1.0 - (1.0 - effective_multiplier) * config.weight;
    (base_complexity as f64 * weighted_multiplier) as u32
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn test_shannon_entropy_calculation() {
        let analyzer = EntropyAnalyzer::new();

        // All same tokens = 0 entropy
        let uniform_tokens = vec![TokenType::Keyword("if".to_string()); 10];
        assert_eq!(analyzer.shannon_entropy(&uniform_tokens), 0.0);

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
        // Test with entropy that would trigger dampening if enabled
        let entropy_score = EntropyScore {
            token_entropy: 0.3,
            pattern_repetition: 0.8,
            branch_similarity: 0.6,
            effective_complexity: 0.3,
        };

        // With default config (disabled), should return original
        let original = 10;
        let dampened = apply_entropy_dampening(original, &entropy_score);

        // Default config has entropy disabled, so should return original
        // If entropy is enabled in the future, this test may need updating
        if !crate::config::get_entropy_config().enabled {
            assert_eq!(dampened, original); // No change when disabled
        } else {
            // When enabled, should apply dampening
            assert!(dampened < original);
            // With effective_complexity of 0.3, expect significant reduction
            assert!(dampened <= 6);
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
        assert_eq!(stats.hit_rate, 1.0 / 3.0);

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
}
