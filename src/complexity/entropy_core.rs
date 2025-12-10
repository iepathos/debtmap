use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::hash::Hash;

/// Language-agnostic entropy score
///
/// This structure contains multiple distinct metrics that should not be confused:
/// - `token_entropy`: Shannon entropy measuring code unpredictability (used for chaotic pattern detection)
/// - `effective_complexity`: Composite metric combining entropy, repetition, and similarity
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EntropyScore {
    /// Shannon entropy of code tokens (0.0-1.0, higher = more unpredictable)
    /// - Used for chaotic structure detection (threshold: 0.45)
    /// - Typical range: 0.2 (repetitive) to 0.8 (chaotic)
    /// - Example: Repetitive code = 0.2, Typical code = 0.4, Chaotic code = 0.7
    pub token_entropy: f64,

    /// Pattern repetition score (0.0-1.0, higher = more repetitive)
    /// - Used for dampening complexity in pattern-heavy code
    /// - Measures how often code patterns repeat
    pub pattern_repetition: f64,

    /// Branch similarity score (0.0-1.0, higher = similar branches)
    /// - Used for dampening complexity in similar conditional branches
    /// - Measures structural similarity between branches
    pub branch_similarity: f64,

    /// Composite complexity metric combining entropy, repetition, and similarity
    /// - NOT the same as token_entropy - this is the adjusted final score
    /// - Used for overall complexity assessment, not pattern detection
    /// - Accounts for dampening from repetition and branch similarity
    pub effective_complexity: f64,

    /// Variable diversity count
    pub unique_variables: usize,

    /// Maximum nesting depth
    pub max_nesting: u32,

    /// Actual dampening factor applied (0.0-1.0)
    pub dampening_applied: f64,
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

        // Extract tokens
        let tokens = analyzer.extract_tokens(node);

        // Use our new pure functions for better entropy calculation
        use crate::complexity::entropy_analysis::{
            calculate_repetition_score, calculate_weighted_entropy,
        };

        // Calculate weighted entropy (considers both category and token level)
        let token_entropy = calculate_weighted_entropy(&tokens);

        // Calculate repetition score from actual token sequences
        let token_repetition = calculate_repetition_score(&tokens);

        // Detect structural patterns (existing behavior for control flow patterns)
        let patterns = analyzer.detect_patterns(node);

        // Use the higher of token repetition or structural pattern repetition
        let pattern_repetition = token_repetition.max(patterns.repetition_ratio);

        // Calculate branch similarity
        let branch_similarity = analyzer.calculate_branch_similarity(node);

        // Get structural information
        let (unique_variables, max_nesting) = analyzer.analyze_structure(node);

        // Calculate effective complexity
        let effective_complexity =
            self.adjust_complexity(token_entropy, pattern_repetition, branch_similarity);

        let score = EntropyScore {
            token_entropy,
            pattern_repetition,
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
    /// This is a pure function that reduces entropy based on repetition
    pub fn adjust_complexity(&self, entropy: f64, patterns: f64, similarity: f64) -> f64 {
        // Pattern reduction: high repetition reduces complexity
        // If patterns = 1.0 (100% repetitive), factor = 0.7 (30% reduction max)
        let pattern_factor = 1.0 - (patterns * self.config.pattern_weight);

        // Similarity reduction: similar branches reduce complexity
        // If similarity = 1.0 (identical branches), factor = 0.8 (20% reduction max)
        let similarity_factor = 1.0 - (similarity * self.config.similarity_weight);

        // Apply reductions
        let adjusted = entropy * pattern_factor * similarity_factor;

        // Apply threshold dampening for very low complexity
        if adjusted < self.config.base_threshold {
            adjusted * 0.5 // Further reduce score for trivially simple code
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

    /// Calculate dampening factor from entropy and repetition values.
    ///
    /// This pure function computes a dampening factor without requiring a full
    /// EntropyScore struct. Useful for aggregation where only weighted averages
    /// of entropy and repetition are known.
    ///
    /// Returns a factor in range [0.5, 1.0] suitable for adjusting complexity.
    pub fn calculate_dampening_factor(&self, token_entropy: f64, pattern_repetition: f64) -> f64 {
        let effective = self.adjust_complexity(token_entropy, pattern_repetition, 0.0);
        // Scale effective complexity to a dampening factor
        // Higher effective complexity = higher dampening (less reduction)
        (effective / 2.0).clamp(0.5, 1.0)
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
    fn value(&self) -> &str;
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
