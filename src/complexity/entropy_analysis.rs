//! Pure functions for entropy analysis
//! This module contains refactored, testable pure functions for entropy calculation

use crate::complexity::entropy_core::{EntropyToken, TokenCategory};
use std::collections::HashMap;

/// Represents a token sequence pattern
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct TokenPattern {
    tokens: Vec<String>,
}

impl TokenPattern {
    pub fn from_tokens(tokens: &[impl EntropyToken]) -> Self {
        Self {
            tokens: tokens.iter().map(|t| t.value().to_string()).collect(),
        }
    }
}

/// Detects repetitive token sequences in a window
/// Pure function that finds repeating patterns of tokens
pub fn detect_repetitive_sequences<T: EntropyToken>(
    tokens: &[T],
    window_size: usize,
) -> HashMap<TokenPattern, usize> {
    let mut pattern_counts = HashMap::new();

    if tokens.len() < window_size {
        return pattern_counts;
    }

    for i in 0..=tokens.len() - window_size {
        let window = &tokens[i..i + window_size];
        let pattern = TokenPattern::from_tokens(window);
        *pattern_counts.entry(pattern).or_insert(0) += 1;
    }

    // Filter out patterns that appear only once
    pattern_counts.retain(|_, &mut count| count > 1);
    pattern_counts
}

/// Calculate repetition score based on detected patterns
/// Returns a value between 0 (no repetition) and 1 (all tokens are repetitive)
pub fn calculate_repetition_score<T: EntropyToken>(tokens: &[T]) -> f64 {
    if tokens.is_empty() {
        return 0.0;
    }

    let mut total_repetitions = 0;
    let mut max_score: f64 = 0.0;

    // Check different window sizes (2 to 5 tokens)
    for window_size in 2..=5.min(tokens.len() / 2) {
        let patterns = detect_repetitive_sequences(tokens, window_size);

        // Calculate how many tokens are part of repetitive patterns
        let repetitive_tokens: usize = patterns
            .iter()
            .map(|(_, &count)| (count - 1) * window_size)
            .sum();

        let score = repetitive_tokens as f64 / tokens.len() as f64;
        max_score = max_score.max(score);
        total_repetitions += patterns.values().filter(|&&c| c > 2).count();
    }

    // Bonus for highly repetitive patterns (appearing 3+ times)
    let repetition_bonus = (total_repetitions as f64 / 10.0).min(0.2);

    (max_score + repetition_bonus).min(1.0)
}

/// Calculate token diversity score
/// Returns a value between 0 (all tokens same) and 1 (all tokens unique)
pub fn calculate_token_diversity<T: EntropyToken>(tokens: &[T]) -> f64 {
    if tokens.is_empty() {
        return 0.0;
    }

    let mut unique_tokens = HashMap::new();
    for token in tokens {
        *unique_tokens.entry(token.value()).or_insert(0) += 1;
    }

    unique_tokens.len() as f64 / tokens.len() as f64
}

/// Calculate weighted entropy considering both categories and individual tokens
pub fn calculate_weighted_entropy<T: EntropyToken>(tokens: &[T]) -> f64 {
    if tokens.is_empty() {
        return 0.0;
    }

    // Category-level entropy (existing behavior)
    let category_entropy = calculate_category_entropy(tokens);

    // Token-level entropy (new)
    let token_entropy = calculate_token_level_entropy(tokens);

    // Weighted combination
    category_entropy * 0.6 + token_entropy * 0.4
}

/// Calculate entropy at the token category level
fn calculate_category_entropy<T: EntropyToken>(tokens: &[T]) -> f64 {
    let mut frequency_map: HashMap<TokenCategory, f64> = HashMap::new();
    let total_weight: f64 = tokens.iter().map(|t| t.weight()).sum();

    if total_weight == 0.0 {
        return 0.0;
    }

    for token in tokens {
        let category = token.to_category();
        let weight = token.weight();
        *frequency_map.entry(category).or_insert(0.0) += weight;
    }

    let mut entropy = 0.0;
    for &freq in frequency_map.values() {
        if freq > 0.0 {
            let probability = freq / total_weight;
            entropy -= probability * probability.log2();
        }
    }

    // Normalize
    if frequency_map.len() > 1 {
        let max_entropy = (frequency_map.len() as f64).log2();
        entropy / max_entropy
    } else {
        0.0
    }
}

/// Calculate entropy at the individual token level
fn calculate_token_level_entropy<T: EntropyToken>(tokens: &[T]) -> f64 {
    let mut frequency_map: HashMap<String, usize> = HashMap::new();

    for token in tokens {
        *frequency_map.entry(token.value().to_string()).or_insert(0) += 1;
    }

    let total = tokens.len() as f64;
    let mut entropy = 0.0;

    for &count in frequency_map.values() {
        let probability = count as f64 / total;
        entropy -= probability * probability.log2();
    }

    // Normalize
    if frequency_map.len() > 1 {
        let max_entropy = total.log2();
        entropy / max_entropy
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::complexity::entropy_traits::GenericToken;

    #[test]
    fn test_detect_repetitive_sequences() {
        let tokens = vec![
            GenericToken::identifier("x".to_string()),
            GenericToken::operator("+".to_string()),
            GenericToken::literal("1".to_string()),
            GenericToken::identifier("x".to_string()),
            GenericToken::operator("+".to_string()),
            GenericToken::literal("1".to_string()),
        ];

        let patterns = detect_repetitive_sequences(&tokens, 3);
        assert_eq!(patterns.len(), 1, "Should detect one repeating pattern");

        let pattern = TokenPattern::from_tokens(&tokens[0..3]);
        assert_eq!(
            patterns.get(&pattern),
            Some(&2),
            "Pattern should appear twice"
        );
    }

    #[test]
    fn test_calculate_repetition_score() {
        // Highly repetitive tokens
        let repetitive = vec![
            GenericToken::identifier("x".to_string()),
            GenericToken::operator("+".to_string()),
            GenericToken::literal("1".to_string()),
            GenericToken::identifier("x".to_string()),
            GenericToken::operator("+".to_string()),
            GenericToken::literal("1".to_string()),
            GenericToken::identifier("x".to_string()),
            GenericToken::operator("+".to_string()),
            GenericToken::literal("1".to_string()),
        ];

        let score = calculate_repetition_score(&repetitive);
        assert!(
            score > 0.5,
            "Repetitive tokens should have high score: {}",
            score
        );

        // Diverse tokens
        let diverse = vec![
            GenericToken::identifier("a".to_string()),
            GenericToken::operator("+".to_string()),
            GenericToken::identifier("b".to_string()),
            GenericToken::operator("*".to_string()),
            GenericToken::identifier("c".to_string()),
            GenericToken::operator("-".to_string()),
            GenericToken::identifier("d".to_string()),
        ];

        let score = calculate_repetition_score(&diverse);
        assert!(
            score < 0.2,
            "Diverse tokens should have low score: {}",
            score
        );
    }

    #[test]
    fn test_token_diversity() {
        let repetitive = vec![
            GenericToken::operator("+".to_string()),
            GenericToken::operator("+".to_string()),
            GenericToken::operator("+".to_string()),
        ];

        let diversity = calculate_token_diversity(&repetitive);
        assert!(
            diversity < 0.5,
            "Same tokens should have low diversity: {}",
            diversity
        );

        let diverse = vec![
            GenericToken::operator("+".to_string()),
            GenericToken::operator("-".to_string()),
            GenericToken::operator("*".to_string()),
        ];

        let diversity = calculate_token_diversity(&diverse);
        assert_eq!(diversity, 1.0, "Unique tokens should have max diversity");
    }
}
