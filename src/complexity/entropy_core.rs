use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::hash::Hash;

/// Language-agnostic entropy score
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

/// Generic token category for language-agnostic entropy calculation
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub enum TokenCategory {
    Keyword,
    Operator,
    Identifier,
    Literal,
    ControlFlow,
    FunctionCall,
    Custom(String),
}

/// Pattern metrics for entropy calculation
#[derive(Debug, Clone)]
pub struct PatternMetrics {
    pub total_patterns: usize,
    pub unique_patterns: usize,
    pub repetition_ratio: f64,
}

impl Default for PatternMetrics {
    fn default() -> Self {
        Self::new()
    }
}

impl PatternMetrics {
    pub fn new() -> Self {
        Self {
            total_patterns: 0,
            unique_patterns: 0,
            repetition_ratio: 0.0,
        }
    }

    pub fn calculate_repetition(&mut self) {
        if self.total_patterns > 0 {
            self.repetition_ratio =
                1.0 - (self.unique_patterns as f64 / self.total_patterns as f64);
        }
    }
}

/// Configuration for entropy calculation
#[derive(Debug, Clone)]
pub struct EntropyConfig {
    pub enabled: bool,
    pub max_cache_size: usize,
    pub base_threshold: f64,
    pub pattern_weight: f64,
    pub similarity_weight: f64,
}

impl Default for EntropyConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_cache_size: 1000,
            base_threshold: 0.5,
            pattern_weight: 0.3,
            similarity_weight: 0.2,
        }
    }
}

/// Universal entropy calculator for language-agnostic entropy calculation
pub struct UniversalEntropyCalculator {
    cache: HashMap<String, EntropyScore>,
    config: EntropyConfig,
    cache_hits: usize,
    cache_misses: usize,
}

impl UniversalEntropyCalculator {
    pub fn new(config: EntropyConfig) -> Self {
        Self {
            cache: HashMap::new(),
            config,
            cache_hits: 0,
            cache_misses: 0,
        }
    }

    /// Calculate entropy for a given analyzer and node
    pub fn calculate<L: LanguageEntropyAnalyzer>(
        &mut self,
        analyzer: &L,
        node: &L::AstNode,
    ) -> EntropyScore {
        // Generate cache key
        let cache_key = analyzer.generate_cache_key(node);

        // Check cache
        if let Some(cached) = self.cache.get(&cache_key) {
            self.cache_hits += 1;
            return cached.clone();
        }

        self.cache_misses += 1;

        // Extract tokens and calculate Shannon entropy
        let tokens = analyzer.extract_tokens(node);
        let token_entropy = self.shannon_entropy(&tokens);

        // Detect pattern repetition
        let patterns = analyzer.detect_patterns(node);

        // Calculate branch similarity
        let branch_similarity = analyzer.calculate_branch_similarity(node);

        // Get structural information
        let (unique_variables, max_nesting) = analyzer.analyze_structure(node);

        // Calculate effective complexity
        let effective_complexity =
            self.adjust_complexity(token_entropy, patterns.repetition_ratio, branch_similarity);

        let score = EntropyScore {
            token_entropy,
            pattern_repetition: patterns.repetition_ratio,
            branch_similarity,
            effective_complexity,
            unique_variables,
            max_nesting,
            dampening_applied: 1.0, // Will be calculated by apply_dampening
        };

        // Cache the result
        if self.cache.len() >= self.config.max_cache_size {
            self.evict_oldest();
        }
        self.cache.insert(cache_key, score.clone());

        score
    }

    /// Calculate Shannon entropy for a sequence of tokens
    pub fn shannon_entropy<T: EntropyToken>(&self, tokens: &[T]) -> f64 {
        if tokens.is_empty() {
            return 0.0;
        }

        let mut frequency_map: HashMap<TokenCategory, f64> = HashMap::new();
        let total_weight: f64 = tokens.iter().map(|t| t.weight()).sum();

        // Count weighted frequency of each token category
        for token in tokens {
            let category = token.to_category();
            let weight = token.weight();
            *frequency_map.entry(category).or_insert(0.0) += weight;
        }

        // Calculate entropy
        let mut entropy = 0.0;
        for &freq in frequency_map.values() {
            if freq > 0.0 {
                let probability = freq / total_weight;
                entropy -= probability * probability.log2();
            }
        }

        // Normalize to 0-1 range
        if frequency_map.len() > 1 {
            let max_entropy = (frequency_map.len() as f64).log2();
            entropy / max_entropy
        } else {
            0.0
        }
    }

    /// Adjust complexity based on patterns and similarity
    pub fn adjust_complexity(&self, entropy: f64, patterns: f64, similarity: f64) -> f64 {
        let pattern_factor = 1.0 - (patterns * self.config.pattern_weight);
        let similarity_factor = 1.0 - (similarity * self.config.similarity_weight);

        let adjusted = entropy * pattern_factor * similarity_factor;

        // Apply threshold
        if adjusted < self.config.base_threshold {
            adjusted * 0.5 // Reduce score for low complexity
        } else {
            adjusted
        }
    }

    /// Apply dampening to entropy score
    pub fn apply_dampening(&self, score: &EntropyScore) -> f64 {
        let nesting_factor = 1.0 + (score.max_nesting as f64 * 0.1);
        let variable_factor = 1.0 + (score.unique_variables as f64 * 0.01);

        let dampening = (nesting_factor * variable_factor).min(2.0);
        score.effective_complexity * dampening
    }

    /// Evict oldest cache entry when cache is full
    fn evict_oldest(&mut self) {
        if let Some(oldest_key) = self.cache.keys().next().cloned() {
            self.cache.remove(&oldest_key);
        }
    }

    /// Get cache statistics
    pub fn cache_stats(&self) -> (usize, usize, f64) {
        let total = self.cache_hits + self.cache_misses;
        let hit_rate = if total > 0 {
            self.cache_hits as f64 / total as f64
        } else {
            0.0
        };
        (self.cache_hits, self.cache_misses, hit_rate)
    }
}

/// Trait for language-specific entropy token
pub trait EntropyToken: Clone + Hash + Eq {
    fn to_category(&self) -> TokenCategory;
    fn weight(&self) -> f64;
}

/// Trait for language-specific entropy analysis
pub trait LanguageEntropyAnalyzer: Send + Sync {
    type AstNode;
    type Token: EntropyToken;

    /// Extract tokens from AST node
    fn extract_tokens(&self, node: &Self::AstNode) -> Vec<Self::Token>;

    /// Detect patterns in AST node
    fn detect_patterns(&self, node: &Self::AstNode) -> PatternMetrics;

    /// Calculate branch similarity
    fn calculate_branch_similarity(&self, node: &Self::AstNode) -> f64;

    /// Analyze structure (unique variables, max nesting)
    fn analyze_structure(&self, node: &Self::AstNode) -> (usize, u32);

    /// Generate cache key for node
    fn generate_cache_key(&self, node: &Self::AstNode) -> String;
}
