use super::entropy_core::{EntropyToken, TokenCategory};

/// Re-export core traits for convenience
pub use super::entropy_core::{EntropyScore, UniversalEntropyCalculator};

/// Token wrapper for bridging existing token types with the new framework
#[derive(Debug, Clone)]
pub struct GenericToken {
    category: TokenCategory,
    weight: f64,
    value: String,
}

impl GenericToken {
    pub fn new(category: TokenCategory, weight: f64, value: String) -> Self {
        Self {
            category,
            weight,
            value,
        }
    }

    pub fn keyword(value: String) -> Self {
        Self::new(TokenCategory::Keyword, 1.0, value)
    }

    pub fn operator(value: String) -> Self {
        Self::new(TokenCategory::Operator, 0.8, value)
    }

    pub fn identifier(value: String) -> Self {
        Self::new(TokenCategory::Identifier, 0.5, value)
    }

    pub fn literal(value: String) -> Self {
        Self::new(TokenCategory::Literal, 0.3, value)
    }

    pub fn control_flow(value: String) -> Self {
        Self::new(TokenCategory::ControlFlow, 1.2, value)
    }

    pub fn function_call(value: String) -> Self {
        Self::new(TokenCategory::FunctionCall, 0.9, value)
    }

    pub fn custom(value: String) -> Self {
        Self::new(TokenCategory::Custom(value.clone()), 1.0, value)
    }

    pub fn value(&self) -> &str {
        &self.value
    }
}

impl std::hash::Hash for GenericToken {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.category.hash(state);
        self.value.hash(state);
    }
}

impl PartialEq for GenericToken {
    fn eq(&self, other: &Self) -> bool {
        self.category == other.category && self.value == other.value
    }
}

impl Eq for GenericToken {}

impl EntropyToken for GenericToken {
    fn to_category(&self) -> TokenCategory {
        self.category.clone()
    }

    fn weight(&self) -> f64 {
        self.weight
    }

    fn value(&self) -> &str {
        &self.value
    }
}

/// Base implementation helpers for language analyzers
pub trait AnalyzerHelpers {
    /// Calculate sequence similarity between two token sequences
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

    /// Calculate Levenshtein distance for pattern matching
    fn levenshtein_distance(&self, s1: &str, s2: &str) -> usize {
        let len1 = s1.len();
        let len2 = s2.len();
        let mut matrix = vec![vec![0; len2 + 1]; len1 + 1];

        #[allow(clippy::needless_range_loop)]
        for i in 0..=len1 {
            matrix[i][0] = i;
        }
        for j in 0..=len2 {
            matrix[0][j] = j;
        }

        for (i, c1) in s1.chars().enumerate() {
            for (j, c2) in s2.chars().enumerate() {
                let cost = if c1 == c2 { 0 } else { 1 };
                matrix[i + 1][j + 1] =
                    matrix[i][j + 1].min(matrix[i + 1][j]).min(matrix[i][j]) + cost;
            }
        }

        matrix[len1][len2]
    }

    /// Calculate pattern diversity score
    fn pattern_diversity(&self, patterns: &[String]) -> f64 {
        if patterns.is_empty() {
            return 0.0;
        }

        let unique_patterns: std::collections::HashSet<_> = patterns.iter().collect();
        unique_patterns.len() as f64 / patterns.len() as f64
    }
}
