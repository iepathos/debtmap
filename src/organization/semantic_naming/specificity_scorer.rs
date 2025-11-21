//! Specificity scoring for module names.
//!
//! Evaluates how specific and descriptive a module name is,
//! rejecting generic names like "unknown", "misc", "utils", etc.

use std::collections::HashSet;

/// Scores module names based on specificity and descriptiveness
pub struct SpecificityScorer {
    generic_terms: HashSet<String>,
    specific_verbs: Vec<String>,
    domain_terms: Vec<String>,
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
            // Additional type-based generic terms (Spec 193)
            "transformations",
            "computation",
            "item",
            "formatter",
            "shared",
            "operations",
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

        let domain_terms = vec![
            "coverage",
            "complexity",
            "validation",
            "formatting",
            "parsing",
            "computation",
        ]
        .into_iter()
        .map(String::from)
        .collect();

        Self {
            generic_terms,
            specific_verbs,
            domain_terms,
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

        // Bonus: Domain-specific terms get a boost
        if self.domain_terms.contains(&name_lower) {
            score += 0.12; // Direct domain term match
        } else if self
            .domain_terms
            .iter()
            .any(|term| name_lower.contains(term))
        {
            score += 0.05; // Contains domain term
        }

        // Bonus: Longer names are typically more specific (up to a point)
        let name_len = name.len();
        if name_len > 8 {
            score += 0.04;
        }
        if name_len > 12 {
            score += 0.02;
        }

        // Bonus: Compound names (with underscore) are more specific
        if name.contains('_') {
            score += 0.10;
        }

        // Bonus: Contains specific action verbs or their gerund forms
        let has_verb = self
            .specific_verbs
            .iter()
            .any(|verb| name_lower.contains(verb) || name_lower.contains(&format!("{}ing", verb)));
        if has_verb {
            score += 0.10;
        }

        // Penalty: Contains generic terms as part of name
        // BUT: Don't penalize compound names where generic is just a suffix
        // (e.g., "validation_operations" is OK, but "operations_handler" is penalized)
        for generic in &self.generic_terms {
            if name_lower.contains(generic) && name_lower != *generic {
                // Skip penalty if this is a compound name with the generic as suffix
                if name_lower.ends_with(generic) && name_lower.len() > generic.len() + 1 {
                    // This is like "validation_operations" - acceptable
                    continue;
                }
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

        if score >= 0.85 {
            "Excellent"
        } else if score >= 0.6 {
            "Good"
        } else if score >= 0.4 {
            "Acceptable"
        } else {
            "Poor"
        }
    }

    /// Calculate specificity score with type-awareness (Spec 193 Phase 2)
    ///
    /// Type-based splits get stricter penalties for generic names because they
    /// originate from Rust type names which are often too vague for module names.
    ///
    /// # Arguments
    ///
    /// * `name` - Module name to score (without .rs extension)
    /// * `is_type_based` - true if this name comes from type-based clustering
    ///
    /// # Returns
    ///
    /// Specificity score in range [0.0, 1.0]
    pub fn calculate_specificity_type_aware(&self, name: &str, is_type_based: bool) -> f64 {
        let mut score = self.calculate_specificity(name);

        // Apply stricter penalties for type-based splits
        if is_type_based {
            // Heavy penalty for generic type names
            if is_generic_type_name(name) {
                score *= 0.2; // Very low score for generic names
            } else if score < 0.6 {
                // Moderate penalty for borderline names (only if not already good)
                score *= 0.85;
            }
        }

        score.clamp(0.0, 1.0)
    }

    /// Check if name is acceptable for type-based splits (stricter threshold)
    ///
    /// Type-based splits require a higher quality bar (0.65) because they
    /// originate from type names which are often too generic.
    pub fn is_acceptable_for_type_based(&self, name: &str) -> bool {
        self.calculate_specificity_type_aware(name, true) >= 0.65
    }
}

/// Detect if a module name is too generic to be useful (Spec 193 Phase 1).
///
/// Generic names include:
/// - "unknown", "self", "transformations" (provide no semantic info)
/// - Type names like "Item", "Data", "Formatter" (too vague)
/// - Short names (<5 chars) (likely abbreviations or type vars)
///
/// # Examples
/// ```
/// # use debtmap::organization::semantic_naming::specificity_scorer::is_generic_type_name;
/// assert!(is_generic_type_name("unknown"));
/// assert!(is_generic_type_name("self"));
/// assert!(!is_generic_type_name("validation_rules"));
/// ```
pub fn is_generic_type_name(name: &str) -> bool {
    let normalized = name.to_lowercase();

    // Check against known generic patterns (Spec 193)
    // These patterns are too vague to be useful module names
    const GENERIC_TYPE_PATTERNS: &[&str] = &[
        "unknown",
        "self",
        "transformations",
        "computation",
        "item",
        "data",
        "utils",
        "helpers",
        "misc",
        "other",
        "common",
        "shared",
        "base",
    ];

    // Reject if name equals a generic pattern
    if GENERIC_TYPE_PATTERNS.iter().any(|p| normalized == *p) {
        return true;
    }

    // Reject if name is just generic + suffix (e.g., "operations", "formatting")
    // but allow compound names (e.g., "validation_operations", "terminal_formatting")
    const GENERIC_SUFFIXES: &[&str] = &["operations", "formatting"];

    for suffix in GENERIC_SUFFIXES {
        if normalized == *suffix {
            return true;
        }
    }

    // Too short to be meaningful
    if name.len() < 5 {
        return true;
    }

    // All caps or numbers (e.g., "T", "U", "X123")
    if name.chars().all(|c| c.is_uppercase() || c.is_numeric()) {
        return true;
    }

    false
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
        // Note: "computation" is now considered generic (Spec 193)
        // Use more specific compound terms instead
        assert!(scorer.calculate_specificity("metric_calculation") > 0.6);
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
        let with_generic_suffix = scorer.calculate_specificity("coverage_utils");
        let with_generic_prefix = scorer.calculate_specificity("utils_coverage");

        // Spec 193: Generic as suffix is acceptable in compound names
        // Both should score similarly since "coverage" provides specificity
        assert!(without_generic > 0.6);
        assert!(with_generic_suffix > 0.6);

        // But generic as prefix should be penalized
        assert!(without_generic > with_generic_prefix);
    }

    // Spec 193: Generic type name detection tests
    #[test]
    fn test_is_generic_type_name_detects_unknown() {
        assert!(is_generic_type_name("unknown"));
        assert!(is_generic_type_name("Unknown"));
        assert!(is_generic_type_name("UNKNOWN"));
    }

    #[test]
    fn test_is_generic_type_name_detects_transformations() {
        assert!(is_generic_type_name("transformations"));
        assert!(is_generic_type_name("Transformations"));
    }

    #[test]
    fn test_is_generic_type_name_detects_computation() {
        assert!(is_generic_type_name("computation"));
        assert!(is_generic_type_name("Computation"));
    }

    #[test]
    fn test_is_generic_type_name_detects_self() {
        assert!(is_generic_type_name("self"));
        assert!(is_generic_type_name("Self"));
    }

    #[test]
    fn test_is_generic_type_name_accepts_specific() {
        assert!(!is_generic_type_name("validation_rules"));
        assert!(!is_generic_type_name("responsibility_classifier"));
        assert!(!is_generic_type_name("scoring_calculations"));
    }

    #[test]
    fn test_is_generic_type_name_rejects_short() {
        assert!(is_generic_type_name("io"));
        assert!(is_generic_type_name("abc"));
        assert!(is_generic_type_name("T"));
    }

    #[test]
    fn test_type_aware_scoring_penalizes_generic() {
        let scorer = SpecificityScorer::new();

        // Test with "unknown" - should be 0.0 for both (it's fully generic)
        assert_eq!(
            scorer.calculate_specificity_type_aware("unknown", false),
            0.0
        );
        assert_eq!(
            scorer.calculate_specificity_type_aware("unknown", true),
            0.0
        );

        // Test with somewhat generic term "operations" to see penalty difference
        let behavioral_score = scorer.calculate_specificity_type_aware("operations", false);
        let type_based_score = scorer.calculate_specificity_type_aware("operations", true);

        // "operations" is generic, so type-based should heavily penalize it
        assert!(type_based_score < 0.2); // Should be heavily penalized
        assert!(behavioral_score > type_based_score || behavioral_score < 0.2); // Either no penalty or both low
    }

    #[test]
    fn test_type_aware_scoring_stricter_threshold() {
        let scorer = SpecificityScorer::new();

        // "formatting" is somewhat generic but not in the worst category
        let behavioral_score = scorer.calculate_specificity_type_aware("formatting", false);
        let type_based_score = scorer.calculate_specificity_type_aware("formatting", true);

        // Type-based should get additional penalty
        assert!(behavioral_score > type_based_score);
    }

    #[test]
    fn test_is_acceptable_for_type_based_rejects_generic() {
        let scorer = SpecificityScorer::new();

        assert!(!scorer.is_acceptable_for_type_based("unknown"));
        assert!(!scorer.is_acceptable_for_type_based("transformations"));
        assert!(!scorer.is_acceptable_for_type_based("computation"));
        assert!(!scorer.is_acceptable_for_type_based("self"));
    }

    #[test]
    fn test_is_acceptable_for_type_based_accepts_specific() {
        let scorer = SpecificityScorer::new();

        assert!(scorer.is_acceptable_for_type_based("validation_operations"));
        assert!(scorer.is_acceptable_for_type_based("responsibility_classification"));
    }
}
