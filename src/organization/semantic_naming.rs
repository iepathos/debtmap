//! Semantic module naming for god object splits.
//!
//! This module provides intelligent naming for module split recommendations by analyzing
//! method names, behavioral patterns, and domain terminology to generate descriptive,
//! unique, and actionable module names.
//!
//! # Naming Strategies
//!
//! 1. **Domain Terms**: Extracts common terms from method names (e.g., "coverage", "metrics")
//! 2. **Behavioral Patterns**: Recognizes common behaviors (e.g., "formatting", "validation")
//! 3. **Specificity Scoring**: Ensures names are descriptive, not generic
//! 4. **Uniqueness Validation**: Guarantees no filename collisions
//!
//! # Example
//!
//! ```
//! use debtmap::organization::semantic_naming::SemanticNameGenerator;
//!
//! let generator = SemanticNameGenerator::new();
//! let methods = vec![
//!     "format_coverage_status".to_string(),
//!     "format_coverage_factor".to_string(),
//!     "calculate_coverage_percentage".to_string(),
//! ];
//!
//! let candidates = generator.generate_names(&methods, None);
//! // Returns candidates like: "coverage" (0.85 confidence), "formatting" (0.75 confidence)
//! ```

mod domain_extractor;
mod pattern_recognizer;
mod specificity_scorer;
mod uniqueness_validator;

pub use domain_extractor::DomainTermExtractor;
pub use pattern_recognizer::PatternRecognizer;
pub use specificity_scorer::SpecificityScorer;
pub use uniqueness_validator::NameUniquenessValidator;

use serde::{Deserialize, Serialize};
use std::path::Path;

/// Naming strategy used to generate a module name
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NamingStrategy {
    /// Extracted from dominant domain terms in method names
    DomainTerms,
    /// Recognized behavioral pattern (e.g., formatting, validation)
    BehavioralPattern,
    /// Fallback with descriptive placeholder
    DescriptiveFallback,
}

/// A candidate module name with confidence and reasoning
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NameCandidate {
    /// Proposed module name (without .rs extension)
    pub module_name: String,
    /// Confidence score (0.0-1.0)
    pub confidence: f64,
    /// Specificity score (0.0-1.0) - how descriptive/specific the name is
    pub specificity_score: f64,
    /// Human-readable explanation of how this name was derived
    pub reasoning: String,
    /// Strategy used to generate this name
    pub strategy: NamingStrategy,
}

impl Default for NameCandidate {
    fn default() -> Self {
        Self {
            module_name: String::new(),
            confidence: 0.0,
            specificity_score: 0.0,
            reasoning: String::new(),
            strategy: NamingStrategy::DescriptiveFallback,
        }
    }
}

/// Semantic name generator that combines multiple naming strategies
pub struct SemanticNameGenerator {
    domain_extractor: DomainTermExtractor,
    pattern_recognizer: PatternRecognizer,
    #[allow(dead_code)] // Reserved for future use in advanced scoring
    specificity_scorer: SpecificityScorer,
    uniqueness_validator: NameUniquenessValidator,
}

impl SemanticNameGenerator {
    /// Create a new semantic name generator with default configuration
    pub fn new() -> Self {
        Self {
            domain_extractor: DomainTermExtractor::new(),
            pattern_recognizer: PatternRecognizer::new(),
            specificity_scorer: SpecificityScorer::new(),
            uniqueness_validator: NameUniquenessValidator::new(),
        }
    }

    /// Generate name candidates for a split based on its methods
    ///
    /// Returns up to 3 name candidates, ranked by confidence.
    ///
    /// # Arguments
    ///
    /// * `methods` - List of method names in the split
    /// * `responsibility` - Optional responsibility description for context
    ///
    /// # Returns
    ///
    /// Vector of name candidates (1-3 items), sorted by confidence descending
    pub fn generate_names(
        &self,
        methods: &[String],
        responsibility: Option<&str>,
    ) -> Vec<NameCandidate> {
        let mut candidates = Vec::new();

        // Strategy 1: Domain terms from method names
        if let Some(domain_name) = self.domain_extractor.generate_domain_name(methods) {
            if self.is_valid_candidate(&domain_name) {
                candidates.push(domain_name);
            }
        }

        // Strategy 2: Behavioral patterns
        if let Some(behavior_name) = self.pattern_recognizer.recognize_pattern(methods) {
            if self.is_valid_candidate(&behavior_name) {
                candidates.push(behavior_name);
            }
        }

        // Strategy 3: Extract from responsibility if provided and high quality
        if let Some(resp) = responsibility {
            if let Some(resp_name) = self.domain_extractor.extract_from_description(resp) {
                if self.is_valid_candidate(&resp_name) {
                    candidates.push(resp_name);
                }
            }
        }

        // If we have no good candidates, generate descriptive fallback
        if candidates.is_empty() {
            candidates.push(self.generate_fallback_name(methods));
        }

        // Sort by confidence (descending) and take top 3
        candidates.sort_by(|a, b| {
            b.confidence
                .partial_cmp(&a.confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        candidates.truncate(3);

        candidates
    }

    /// Generate a unique name for a split, ensuring no collisions within parent directory
    ///
    /// # Arguments
    ///
    /// * `parent_path` - Parent directory path where module will be created
    /// * `methods` - List of method names in the split
    /// * `responsibility` - Optional responsibility description
    ///
    /// # Returns
    ///
    /// The best name candidate, disambiguated if necessary for uniqueness
    pub fn generate_unique_name(
        &mut self,
        parent_path: &Path,
        methods: &[String],
        responsibility: Option<&str>,
    ) -> NameCandidate {
        let candidates = self.generate_names(methods, responsibility);
        self.uniqueness_validator
            .ensure_unique_name(parent_path, candidates)
    }

    /// Check if a candidate is valid (passes specificity threshold)
    fn is_valid_candidate(&self, candidate: &NameCandidate) -> bool {
        candidate.specificity_score >= 0.4
    }

    /// Generate a descriptive fallback name when no good semantic name is found
    fn generate_fallback_name(&self, methods: &[String]) -> NameCandidate {
        // Take up to 3 method names as hints
        let method_hints: Vec<_> = methods.iter().take(3).cloned().collect();
        let reasoning = if method_hints.is_empty() {
            "Auto-generated fallback (no methods to analyze)".to_string()
        } else {
            format!(
                "Auto-generated fallback - review needed. Contains: {}",
                method_hints.join(", ")
            )
        };

        NameCandidate {
            module_name: "needs_review".to_string(),
            confidence: 0.3,
            specificity_score: 0.4, // Just above threshold
            reasoning,
            strategy: NamingStrategy::DescriptiveFallback,
        }
    }
}

impl Default for SemanticNameGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generates_multiple_candidates() {
        let generator = SemanticNameGenerator::new();
        let methods = vec![
            "format_coverage_status".to_string(),
            "format_coverage_factor".to_string(),
            "calculate_coverage_percentage".to_string(),
        ];

        let candidates = generator.generate_names(&methods, None);

        assert!(!candidates.is_empty());
        assert!(candidates.len() <= 3);
        // Should be sorted by confidence descending
        if candidates.len() > 1 {
            assert!(candidates[0].confidence >= candidates[1].confidence);
        }
    }

    #[test]
    fn test_fallback_for_empty_methods() {
        let generator = SemanticNameGenerator::new();
        let methods: Vec<String> = vec![];

        let candidates = generator.generate_names(&methods, None);

        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].module_name, "needs_review");
        assert!(candidates[0].confidence < 0.5);
    }

    #[test]
    fn test_confidence_ordering() {
        let generator = SemanticNameGenerator::new();
        let methods = vec![
            "validate_input".to_string(),
            "validate_output".to_string(),
            "check_constraints".to_string(),
        ];

        let candidates = generator.generate_names(&methods, None);

        // All candidates should have confidence in valid range
        for candidate in &candidates {
            assert!(candidate.confidence >= 0.0 && candidate.confidence <= 1.0);
            assert!(candidate.specificity_score >= 0.0 && candidate.specificity_score <= 1.0);
        }
    }
}
