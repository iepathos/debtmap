use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::hash::Hash;

// =============================================================================
// EntropyAnalysis - Unified Analysis Type (Spec 218)
// =============================================================================

/// Unified entropy analysis type - SINGLE SOURCE OF TRUTH.
///
/// This struct consolidates all entropy-related data into one canonical type
/// that flows through the entire pipeline: extraction → analysis → output.
///
/// **Spec 218**: This replaces the following scattered types:
/// - `priority::unified_scorer::EntropyDetails`
/// - `core::EntropyDetails`
///
/// All entropy consumers should use this type directly.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EntropyAnalysis {
    /// Core entropy value (Shannon entropy of tokens, 0.0-1.0)
    /// Higher values indicate more unpredictable/complex code.
    pub entropy_score: f64,

    /// Pattern repetition score (0.0-1.0, higher = more repetitive)
    /// Used for dampening complexity in pattern-heavy code.
    pub pattern_repetition: f64,

    /// Branch similarity score (0.0-1.0, higher = similar branches)
    /// Used for dampening complexity in similar conditional branches.
    pub branch_similarity: f64,

    /// Dampening factor applied to complexity (0.5-1.0)
    /// Lower values mean more complexity reduction was applied.
    pub dampening_factor: f64,

    /// Whether dampening was actually applied (entropy < threshold)
    pub dampening_was_applied: bool,

    /// Original complexity value before dampening
    pub original_complexity: u32,

    /// Adjusted complexity after applying dampening
    pub adjusted_complexity: u32,

    /// Human-readable explanation of why dampening was applied
    pub reasoning: Vec<String>,
}

impl EntropyAnalysis {
    /// Create an EntropyAnalysis from raw EntropyScore and complexity values.
    ///
    /// This is the primary constructor that converts raw entropy calculation
    /// output into the unified analysis format.
    pub fn from_raw(raw: &EntropyScore, original_complexity: u32, config: &EntropyConfig) -> Self {
        let dampening_factor =
            calculate_dampening_factor_pure(raw.token_entropy, raw.pattern_repetition, config);

        let dampening_was_applied = dampening_factor < 1.0;
        let adjusted = (original_complexity as f64 * dampening_factor) as u32;
        let reasoning = build_entropy_reasoning(raw, dampening_factor, dampening_was_applied);

        Self {
            entropy_score: raw.token_entropy,
            pattern_repetition: raw.pattern_repetition,
            branch_similarity: raw.branch_similarity,
            dampening_factor,
            dampening_was_applied,
            original_complexity,
            adjusted_complexity: adjusted,
            reasoning,
        }
    }

    /// Create a default/empty analysis with no dampening.
    pub fn neutral(complexity: u32) -> Self {
        Self {
            entropy_score: 0.5,
            pattern_repetition: 0.0,
            branch_similarity: 0.0,
            dampening_factor: 1.0,
            dampening_was_applied: false,
            original_complexity: complexity,
            adjusted_complexity: complexity,
            reasoning: vec![],
        }
    }
}

/// Calculate dampening factor from entropy and repetition values.
///
/// Pure function that computes how much to reduce complexity based on
/// code patterns indicating repetitive/simple structure.
///
/// Returns a factor in range [0.5, 1.0]:
/// - 1.0 = no dampening (genuine complexity)
/// - 0.5 = maximum dampening (very repetitive code)
fn calculate_dampening_factor_pure(
    token_entropy: f64,
    pattern_repetition: f64,
    config: &EntropyConfig,
) -> f64 {
    // Only apply dampening for low entropy (repetitive patterns)
    // High entropy indicates real complexity that shouldn't be dampened
    let is_repetitive = token_entropy < config.base_threshold;

    if !is_repetitive {
        return 1.0;
    }

    // Use pattern_repetition to determine dampening amount
    // High repetition = more dampening (lower factor)
    let repetition_dampening = 1.0 - (pattern_repetition * config.pattern_weight);
    repetition_dampening.clamp(0.5, 1.0)
}

/// Build human-readable reasoning for entropy dampening.
fn build_entropy_reasoning(
    raw: &EntropyScore,
    dampening_factor: f64,
    was_applied: bool,
) -> Vec<String> {
    let mut reasoning = Vec::new();

    if raw.pattern_repetition > 0.6 {
        reasoning.push(format!(
            "High pattern repetition detected ({}%)",
            (raw.pattern_repetition * 100.0) as i32
        ));
    }

    if raw.token_entropy < 0.4 {
        reasoning.push(format!(
            "Low token entropy indicates simple patterns ({:.2})",
            raw.token_entropy
        ));
    }

    if raw.branch_similarity > 0.7 {
        reasoning.push(format!(
            "Similar branch structures found ({}% similarity)",
            (raw.branch_similarity * 100.0) as i32
        ));
    }

    if was_applied {
        let reduction_pct = ((1.0 - dampening_factor) * 100.0) as i32;
        reasoning.push(format!(
            "Complexity reduced by {}% due to pattern-based code",
            reduction_pct
        ));
    } else {
        reasoning.push("Genuine complexity detected - minimal reduction applied".to_string());
    }

    reasoning
}

// =============================================================================
// Aggregation Functions (Spec 218)
// =============================================================================

/// Aggregate entropy analysis from multiple functions.
///
/// Uses length-weighted averaging for entropy values and sums for complexity values.
/// This is the SINGLE aggregation function for all entropy aggregation needs.
///
/// # Arguments
///
/// * `items` - Iterator of (EntropyAnalysis reference, function_length) tuples
///
/// # Returns
///
/// Aggregated EntropyAnalysis, or None if input is empty or total length is 0.
pub fn aggregate_entropy<'a>(
    items: impl Iterator<Item = (&'a EntropyAnalysis, usize)>,
) -> Option<EntropyAnalysis> {
    let data: Vec<_> = items.collect();
    if data.is_empty() {
        return None;
    }

    let total_length: usize = data.iter().map(|(_, len)| len).sum();
    if total_length == 0 {
        return None;
    }

    // Weighted averages
    let entropy_score = weighted_avg(&data, total_length, |e| e.entropy_score);
    let pattern_repetition = weighted_avg(&data, total_length, |e| e.pattern_repetition);
    let branch_similarity = weighted_avg(&data, total_length, |e| e.branch_similarity);
    let dampening_factor = weighted_avg(&data, total_length, |e| e.dampening_factor);

    // Sums
    let original_complexity: u32 = data.iter().map(|(e, _)| e.original_complexity).sum();
    let adjusted_complexity: u32 = data.iter().map(|(e, _)| e.adjusted_complexity).sum();

    Some(EntropyAnalysis {
        entropy_score,
        pattern_repetition,
        branch_similarity,
        dampening_factor,
        dampening_was_applied: dampening_factor < 1.0,
        original_complexity,
        adjusted_complexity,
        reasoning: vec![format!(
            "Aggregated from {} functions (weighted by length)",
            data.len()
        )],
    })
}

/// Helper: Calculate weighted average of a field.
fn weighted_avg<F>(data: &[(&EntropyAnalysis, usize)], total_length: usize, f: F) -> f64
where
    F: Fn(&EntropyAnalysis) -> f64,
{
    data.iter()
        .map(|(e, len)| f(e) * (*len as f64))
        .sum::<f64>()
        / total_length as f64
}

// =============================================================================
// EntropyScore - Raw Calculation Output (Existing)
// =============================================================================

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

// =============================================================================
// Tests for EntropyAnalysis (Spec 218)
// =============================================================================

#[cfg(test)]
mod entropy_analysis_tests {
    use super::*;

    fn create_test_entropy_score(
        token_entropy: f64,
        pattern_repetition: f64,
        branch_similarity: f64,
    ) -> EntropyScore {
        EntropyScore {
            token_entropy,
            pattern_repetition,
            branch_similarity,
            effective_complexity: 0.5,
            unique_variables: 5,
            max_nesting: 2,
            dampening_applied: 1.0,
        }
    }

    #[test]
    fn test_entropy_analysis_from_raw_applies_dampening() {
        let raw = create_test_entropy_score(0.15, 0.8, 0.0); // Low entropy, high repetition
        let config = EntropyConfig::default();
        let result = EntropyAnalysis::from_raw(&raw, 100, &config);

        assert!(result.dampening_was_applied);
        assert!(result.adjusted_complexity < 100);
        assert!(result.dampening_factor < 1.0);
        assert!(!result.reasoning.is_empty());
    }

    #[test]
    fn test_entropy_analysis_from_raw_no_dampening_high_entropy() {
        let raw = create_test_entropy_score(0.7, 0.2, 0.1); // High entropy
        let config = EntropyConfig::default();
        let result = EntropyAnalysis::from_raw(&raw, 100, &config);

        assert!(!result.dampening_was_applied);
        assert_eq!(result.adjusted_complexity, 100);
        assert!((result.dampening_factor - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_entropy_analysis_neutral() {
        let result = EntropyAnalysis::neutral(50);

        assert_eq!(result.original_complexity, 50);
        assert_eq!(result.adjusted_complexity, 50);
        assert!(!result.dampening_was_applied);
        assert!((result.dampening_factor - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_aggregate_entropy_weighted_average() {
        let e1 = EntropyAnalysis {
            entropy_score: 0.4,
            pattern_repetition: 0.6,
            branch_similarity: 0.2,
            dampening_factor: 0.8,
            dampening_was_applied: true,
            original_complexity: 20,
            adjusted_complexity: 16,
            reasoning: vec![],
        };
        let e2 = EntropyAnalysis {
            entropy_score: 0.6,
            pattern_repetition: 0.3,
            branch_similarity: 0.4,
            dampening_factor: 0.9,
            dampening_was_applied: true,
            original_complexity: 30,
            adjusted_complexity: 27,
            reasoning: vec![],
        };

        // length 100 for e1, length 200 for e2
        let result = aggregate_entropy([(&e1, 100), (&e2, 200)].into_iter()).unwrap();

        // (100*0.4 + 200*0.6) / 300 ≈ 0.533
        assert!((result.entropy_score - 0.533).abs() < 0.01);

        // (100*0.6 + 200*0.3) / 300 = 120/300 = 0.4
        assert!((result.pattern_repetition - 0.4).abs() < 0.01);

        // (100*0.2 + 200*0.4) / 300 = 100/300 ≈ 0.333
        assert!((result.branch_similarity - 0.333).abs() < 0.01);

        // Original: 20 + 30 = 50
        assert_eq!(result.original_complexity, 50);

        // Adjusted: 16 + 27 = 43
        assert_eq!(result.adjusted_complexity, 43);
    }

    #[test]
    fn test_aggregate_entropy_empty() {
        let result = aggregate_entropy(std::iter::empty());
        assert!(result.is_none());
    }

    #[test]
    fn test_aggregate_entropy_zero_total_length() {
        let e = EntropyAnalysis::neutral(10);
        let result = aggregate_entropy([(&e, 0)].into_iter());
        assert!(result.is_none());
    }

    #[test]
    fn test_aggregate_entropy_single_item() {
        let e = EntropyAnalysis {
            entropy_score: 0.5,
            pattern_repetition: 0.3,
            branch_similarity: 0.1,
            dampening_factor: 0.9,
            dampening_was_applied: true,
            original_complexity: 25,
            adjusted_complexity: 22,
            reasoning: vec![],
        };

        let result = aggregate_entropy([(&e, 100)].into_iter()).unwrap();

        assert!((result.entropy_score - 0.5).abs() < 0.001);
        assert!((result.pattern_repetition - 0.3).abs() < 0.001);
        assert_eq!(result.original_complexity, 25);
        assert_eq!(result.adjusted_complexity, 22);
    }

    #[test]
    fn test_reasoning_includes_high_repetition() {
        let raw = create_test_entropy_score(0.3, 0.75, 0.0);
        let config = EntropyConfig::default();
        let result = EntropyAnalysis::from_raw(&raw, 100, &config);

        let has_repetition_message = result.reasoning.iter().any(|r| r.contains("repetition"));
        assert!(has_repetition_message);
    }

    #[test]
    fn test_reasoning_includes_low_entropy() {
        let raw = create_test_entropy_score(0.2, 0.5, 0.0);
        let config = EntropyConfig::default();
        let result = EntropyAnalysis::from_raw(&raw, 100, &config);

        let has_low_entropy_message = result
            .reasoning
            .iter()
            .any(|r| r.contains("Low token entropy"));
        assert!(has_low_entropy_message);
    }

    #[test]
    fn test_reasoning_includes_branch_similarity() {
        let raw = create_test_entropy_score(0.3, 0.5, 0.8);
        let config = EntropyConfig::default();
        let result = EntropyAnalysis::from_raw(&raw, 100, &config);

        let has_similarity_message = result
            .reasoning
            .iter()
            .any(|r| r.contains("Similar branch"));
        assert!(has_similarity_message);
    }
}
