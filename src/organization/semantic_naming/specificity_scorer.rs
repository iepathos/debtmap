//! Specificity scoring for module names.
//!
//! Evaluates how specific and descriptive a module name is,
//! rejecting generic names like "unknown", "misc", "utils", etc.

use std::collections::HashSet;

/// Scores module names based on specificity and descriptiveness
pub struct SpecificityScorer {
    generic_terms: HashSet<String>,
    specific_verbs: Vec<String>,
}

impl SpecificityScorer {
    /// Create a new specificity scorer with default configuration
    pub fn new() -> Self {
        let generic_terms: HashSet<String> = [
            "unknown",
            "self",
            "misc",
            "utils",
            "common",
            "helpers",
            "data",
            "types",
            "structs",
            "impl",
            "methods",
            "functions",
            "module",
            "base",
            "core",
            "main",
            "other",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();

        let specific_verbs = vec![
            "format",
            "parse",
            "validate",
            "calculate",
            "analyze",
            "serialize",
            "deserialize",
            "transform",
            "convert",
            "compute",
            "evaluate",
        ]
        .into_iter()
        .map(String::from)
        .collect();

        Self {
            generic_terms,
            specific_verbs,
        }
    }

    /// Calculate specificity score for a module name
    ///
    /// Returns a score from 0.0 (completely generic) to 1.0 (highly specific).
    ///
    /// # Arguments
    ///
    /// * `name` - Module name to score (without .rs extension)
    ///
    /// # Returns
    ///
    /// Specificity score in range [0.0, 1.0]
    pub fn calculate_specificity(&self, name: &str) -> f64 {
        let name_lower = name.to_lowercase();

        // Generic terms get zero score
        if self.generic_terms.contains(&name_lower) {
            return 0.0;
        }

        // Check if entire name is a single generic term
        for generic in &self.generic_terms {
            if name_lower == *generic {
                return 0.0;
            }
        }

        let mut score: f64 = 0.5; // Base score for non-generic names

        // Bonus: Longer names are typically more specific (up to a point)
        let name_len = name.len();
        if name_len > 8 {
            score += 0.1;
        }
        if name_len > 12 {
            score += 0.05;
        }

        // Bonus: Compound names (with underscore) are more specific
        if name.contains('_') {
            score += 0.15;
        }

        // Bonus: Contains specific action verbs
        if self
            .specific_verbs
            .iter()
            .any(|verb| name_lower.contains(verb))
        {
            score += 0.15;
        }

        // Penalty: Contains generic terms as part of name
        for generic in &self.generic_terms {
            if name_lower.contains(generic) && name_lower != *generic {
                score -= 0.1;
                break;
            }
        }

        // Penalty: Very short names tend to be abbreviations or generic
        if name_len < 4 {
            score -= 0.15;
        }

        // Penalty: Starts with "needs_review" (fallback naming)
        if name_lower.starts_with("needs_review") {
            score = 0.4; // Just above threshold
        }

        // Clamp to [0.0, 1.0]
        score.clamp(0.0_f64, 1.0)
    }

    /// Check if a name is acceptable (above minimum threshold)
    ///
    /// # Arguments
    ///
    /// * `name` - Module name to check
    /// * `min_threshold` - Minimum acceptable specificity (default: 0.4)
    ///
    /// # Returns
    ///
    /// true if name meets minimum specificity, false otherwise
    pub fn is_acceptable(&self, name: &str, min_threshold: f64) -> bool {
        self.calculate_specificity(name) >= min_threshold
    }

    /// Get a human-readable assessment of name quality
    pub fn assess_quality(&self, name: &str) -> &'static str {
        let score = self.calculate_specificity(name);

        if score >= 0.8 {
            "Excellent"
        } else if score >= 0.6 {
            "Good"
        } else if score >= 0.4 {
            "Acceptable"
        } else {
            "Poor"
        }
    }
}

impl Default for SpecificityScorer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rejects_generic_names() {
        let scorer = SpecificityScorer::new();

        assert_eq!(scorer.calculate_specificity("unknown"), 0.0);
        assert_eq!(scorer.calculate_specificity("self"), 0.0);
        assert_eq!(scorer.calculate_specificity("misc"), 0.0);
        assert_eq!(scorer.calculate_specificity("utils"), 0.0);
        assert_eq!(scorer.calculate_specificity("common"), 0.0);
        assert_eq!(scorer.calculate_specificity("helpers"), 0.0);
    }

    #[test]
    fn test_scores_specific_names_high() {
        let scorer = SpecificityScorer::new();

        // Domain-specific terms should score well
        assert!(scorer.calculate_specificity("coverage") > 0.5);
        assert!(scorer.calculate_specificity("complexity") > 0.5);
        assert!(scorer.calculate_specificity("validation") > 0.5);
    }

    #[test]
    fn test_compound_names_score_higher() {
        let scorer = SpecificityScorer::new();

        let single = scorer.calculate_specificity("format");
        let compound = scorer.calculate_specificity("format_coverage");

        assert!(compound > single);
        assert!(compound > 0.6);
    }

    #[test]
    fn test_specific_verbs_boost_score() {
        let scorer = SpecificityScorer::new();

        assert!(scorer.calculate_specificity("formatting") > 0.6);
        assert!(scorer.calculate_specificity("parsing") > 0.6);
        assert!(scorer.calculate_specificity("validation") > 0.6);
        assert!(scorer.calculate_specificity("computation") > 0.6);
    }

    #[test]
    fn test_short_names_penalized() {
        let scorer = SpecificityScorer::new();

        let short = scorer.calculate_specificity("io");
        let long = scorer.calculate_specificity("input_output");

        assert!(long > short);
    }

    #[test]
    fn test_needs_review_fallback() {
        let scorer = SpecificityScorer::new();

        let score = scorer.calculate_specificity("needs_review_group_1");

        // Should be just above threshold (0.4)
        assert!(score >= 0.4);
        assert!(score < 0.5);
    }

    #[test]
    fn test_is_acceptable_threshold() {
        let scorer = SpecificityScorer::new();

        assert!(scorer.is_acceptable("format_coverage", 0.4));
        assert!(!scorer.is_acceptable("unknown", 0.4));
        assert!(!scorer.is_acceptable("misc", 0.4));
    }

    #[test]
    fn test_quality_assessment() {
        let scorer = SpecificityScorer::new();

        assert_eq!(scorer.assess_quality("format_coverage"), "Good");
        assert_eq!(scorer.assess_quality("unknown"), "Poor");
        assert_eq!(scorer.assess_quality("validation"), "Good");
    }

    #[test]
    fn test_case_insensitive() {
        let scorer = SpecificityScorer::new();

        let lower = scorer.calculate_specificity("unknown");
        let upper = scorer.calculate_specificity("Unknown");
        let mixed = scorer.calculate_specificity("UnKnOwN");

        assert_eq!(lower, upper);
        assert_eq!(lower, mixed);
    }

    #[test]
    fn test_contains_generic_penalty() {
        let scorer = SpecificityScorer::new();

        let without_generic = scorer.calculate_specificity("coverage_analysis");
        let with_generic = scorer.calculate_specificity("coverage_utils");

        assert!(without_generic > with_generic);
    }
}
