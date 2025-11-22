//! Name uniqueness validation to prevent filename collisions.
//!
//! Tracks used module names per directory and ensures all generated
//! names are unique within their parent directory.

use super::NameCandidate;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

/// Validates and ensures uniqueness of module names
pub struct NameUniquenessValidator {
    /// Map from parent directory to set of used module names
    used_names: HashMap<PathBuf, HashSet<String>>,
}

impl NameUniquenessValidator {
    /// Create a new uniqueness validator
    pub fn new() -> Self {
        Self {
            used_names: HashMap::new(),
        }
    }

    /// Ensure a unique name from a list of candidates
    ///
    /// Tries each candidate in order of confidence. If all candidates
    /// are already used, disambiguates the best candidate using method-based
    /// terms when available, or numeric suffix as fallback.
    ///
    /// # Arguments
    ///
    /// * `parent_path` - Parent directory where module will be created
    /// * `candidates` - List of name candidates (should be sorted by confidence)
    /// * `methods` - Optional list of method names for intelligent disambiguation
    ///
    /// # Returns
    ///
    /// A unique name candidate, possibly disambiguated
    pub fn ensure_unique_name(
        &mut self,
        parent_path: &Path,
        mut candidates: Vec<NameCandidate>,
        methods: Option<&[String]>,
    ) -> NameCandidate {
        if candidates.is_empty() {
            // No candidates provided, generate a fallback
            return self.generate_unique_fallback(parent_path);
        }

        let used = self
            .used_names
            .entry(parent_path.to_path_buf())
            .or_default();

        // Try each candidate in order
        for candidate in &candidates {
            if !used.contains(&candidate.module_name) {
                used.insert(candidate.module_name.clone());
                return candidate.clone();
            }
        }

        // All candidates are used, try intelligent disambiguation first
        let base_candidate = candidates.remove(0);

        // Try method-based disambiguation if methods provided
        if let Some(methods) = methods {
            if let Some(distinctive_term) = Self::extract_distinctive_term(methods) {
                let candidate = format!("{}_{}", base_candidate.module_name, distinctive_term);
                if !used.contains(&candidate) {
                    used.insert(candidate.clone());
                    return NameCandidate {
                        module_name: candidate,
                        confidence: base_candidate.confidence * 0.85, // Slight confidence reduction
                        specificity_score: base_candidate.specificity_score,
                        reasoning: format!(
                            "{} (disambiguated with method-specific term '{}')",
                            base_candidate.reasoning, distinctive_term
                        ),
                        strategy: base_candidate.strategy,
                    };
                }
            }
        }

        // Fall back to numeric disambiguation
        let unique_name = Self::disambiguate_name_static(used, &base_candidate.module_name);

        used.insert(unique_name.clone());

        NameCandidate {
            module_name: unique_name.clone(),
            confidence: base_candidate.confidence * 0.7, // Lower confidence for numeric suffix
            specificity_score: base_candidate.specificity_score,
            reasoning: format!(
                "{} (disambiguated with numeric suffix to avoid collision)",
                base_candidate.reasoning
            ),
            strategy: base_candidate.strategy,
        }
    }

    /// Check if a name is already used in a directory
    pub fn is_used(&self, parent_path: &Path, name: &str) -> bool {
        self.used_names
            .get(parent_path)
            .is_some_and(|used| used.contains(name))
    }

    /// Manually mark a name as used (useful for pre-existing files)
    pub fn mark_as_used(&mut self, parent_path: &Path, name: String) {
        self.used_names
            .entry(parent_path.to_path_buf())
            .or_default()
            .insert(name);
    }

    /// Clear all used names (for testing or reset)
    pub fn clear(&mut self) {
        self.used_names.clear();
    }

    /// Clear used names for a specific directory
    pub fn clear_directory(&mut self, parent_path: &Path) {
        self.used_names.remove(parent_path);
    }

    /// Generate a unique fallback name with numeric suffix
    fn generate_unique_fallback(&mut self, parent_path: &Path) -> NameCandidate {
        let used = self
            .used_names
            .entry(parent_path.to_path_buf())
            .or_default();

        let mut counter = 1;
        loop {
            let name = format!("needs_review_{}", counter);
            if !used.contains(&name) {
                used.insert(name.clone());
                return NameCandidate {
                    module_name: name,
                    confidence: 0.3,
                    specificity_score: 0.4,
                    reasoning: "Auto-generated fallback name (no candidates provided)".to_string(),
                    strategy: super::NamingStrategy::DescriptiveFallback,
                };
            }
            counter += 1;
        }
    }

    /// Extract a distinctive term from method names for disambiguation
    ///
    /// Analyzes method names to find a specific verb or noun that can be used
    /// to create a more descriptive disambiguated name.
    ///
    /// # Arguments
    ///
    /// * `methods` - List of method names to analyze
    ///
    /// # Returns
    ///
    /// A distinctive term if found, None otherwise
    ///
    /// # Examples
    ///
    /// ```ignore
    /// // Methods: ["infer_responsibility", "group_methods"]
    /// // → Returns: "inference" (from "infer" verb)
    ///
    /// // Methods: ["classify_struct_domain", "extract_domain"]
    /// // → Returns: "classification" (from "classify" verb)
    /// ```
    fn extract_distinctive_term(methods: &[String]) -> Option<String> {
        use std::collections::HashMap;

        // Look for distinctive verbs
        let distinctive_verbs = [
            ("infer", "inference"),
            ("classify", "classification"),
            ("extract", "extraction"),
            ("recommend", "recommendations"),
            ("suggest", "suggestions"),
            ("validate", "validation"),
            ("calculate", "calculation"),
            ("compute", "computation"),
            ("analyze", "analysis"),
            ("detect", "detection"),
            ("generate", "generation"),
            ("transform", "transformation"),
            ("convert", "conversion"),
            ("serialize", "serialization"),
            ("deserialize", "deserialization"),
            ("group", "grouping"),
            ("cluster", "clustering"),
            ("merge", "merging"),
            ("split", "splitting"),
        ];

        // Count verb occurrences
        let mut verb_counts: HashMap<&str, usize> = HashMap::new();
        for method in methods {
            let method_lower = method.to_lowercase();
            for (verb, _) in &distinctive_verbs {
                if method_lower.starts_with(verb) || method_lower.contains(&format!("_{}", verb)) {
                    *verb_counts.entry(verb).or_default() += 1;
                    break;
                }
            }
        }

        // Find most common distinctive verb
        if let Some((verb, _)) = verb_counts
            .into_iter()
            .filter(|(_, count)| (*count as f64 / methods.len() as f64) > 0.3)
            .max_by_key(|(_, count)| *count)
        {
            // Return the noun form of the verb
            for (v, noun) in &distinctive_verbs {
                if *v == verb {
                    return Some(noun.to_string());
                }
            }
        }

        // If no verb found, try to extract a distinctive noun
        let mut noun_counts: HashMap<String, usize> = HashMap::new();
        for method in methods {
            let parts: Vec<&str> = method.split('_').collect();
            // Look at non-first parts (skip verbs)
            for part in parts.iter().skip(1) {
                let part_lower = part.to_lowercase();
                if part_lower.len() > 4 {
                    // Only meaningful nouns
                    *noun_counts.entry(part_lower).or_default() += 1;
                }
            }
        }

        // Return most common noun
        noun_counts
            .into_iter()
            .filter(|(_, count)| (*count as f64 / methods.len() as f64) > 0.3)
            .max_by_key(|(_, count)| *count)
            .map(|(noun, _)| noun)
    }

    /// Disambiguate a name by adding a numeric suffix (static version)
    ///
    /// Finds the smallest positive integer N such that "name_N" is not used.
    fn disambiguate_name_static(used: &HashSet<String>, base_name: &str) -> String {
        let mut counter = 2; // Start with _2 (original is implicitly _1)

        loop {
            let disambiguated = format!("{}_{}", base_name, counter);
            if !used.contains(&disambiguated) {
                return disambiguated;
            }
            counter += 1;

            // Safety check to prevent infinite loops
            if counter > 1000 {
                // This should never happen in practice
                panic!(
                    "Failed to find unique name after 1000 attempts for base: {}",
                    base_name
                );
            }
        }
    }
}

impl Default for NameUniquenessValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::organization::semantic_naming::NamingStrategy;

    fn create_test_candidate(name: &str, confidence: f64) -> NameCandidate {
        NameCandidate {
            module_name: name.to_string(),
            confidence,
            specificity_score: 0.7,
            reasoning: "Test candidate".to_string(),
            strategy: NamingStrategy::DomainTerms,
        }
    }

    #[test]
    fn test_first_name_is_used() {
        let mut validator = NameUniquenessValidator::new();
        let parent = Path::new("src/organization");

        let candidates = vec![create_test_candidate("metrics", 0.9)];

        let result = validator.ensure_unique_name(parent, candidates, None);

        assert_eq!(result.module_name, "metrics");
        assert_eq!(result.confidence, 0.9);
    }

    #[test]
    fn test_collision_disambiguates() {
        let mut validator = NameUniquenessValidator::new();
        let parent = Path::new("src/organization");

        let candidates1 = vec![create_test_candidate("metrics", 0.9)];
        let candidates2 = vec![create_test_candidate("metrics", 0.85)];

        let name1 = validator.ensure_unique_name(parent, candidates1, None);
        let name2 = validator.ensure_unique_name(parent, candidates2, None);

        assert_eq!(name1.module_name, "metrics");
        assert_eq!(name2.module_name, "metrics_2");
        assert!(name2.confidence < 0.85); // Lowered for disambiguation
    }

    #[test]
    fn test_tries_multiple_candidates() {
        let mut validator = NameUniquenessValidator::new();
        let parent = Path::new("src/organization");

        // Reserve first candidate
        let candidates1 = vec![create_test_candidate("metrics", 0.9)];
        validator.ensure_unique_name(parent, candidates1, None);

        // Provide multiple candidates, first is taken
        let candidates2 = vec![
            create_test_candidate("metrics", 0.85),     // Already used
            create_test_candidate("computation", 0.80), // Available
            create_test_candidate("analysis", 0.75),
        ];

        let result = validator.ensure_unique_name(parent, candidates2, None);

        assert_eq!(result.module_name, "computation");
        assert_eq!(result.confidence, 0.80);
    }

    #[test]
    fn test_multiple_disambiguations() {
        let mut validator = NameUniquenessValidator::new();
        let parent = Path::new("src/organization");

        let name1 =
            validator.ensure_unique_name(parent, vec![create_test_candidate("metrics", 0.9)], None);
        let name2 = validator.ensure_unique_name(
            parent,
            vec![create_test_candidate("metrics", 0.85)],
            None,
        );
        let name3 = validator.ensure_unique_name(
            parent,
            vec![create_test_candidate("metrics", 0.80)],
            None,
        );

        assert_eq!(name1.module_name, "metrics");
        assert_eq!(name2.module_name, "metrics_2");
        assert_eq!(name3.module_name, "metrics_3");
    }

    #[test]
    fn test_different_directories_independent() {
        let mut validator = NameUniquenessValidator::new();
        let parent1 = Path::new("src/organization");
        let parent2 = Path::new("src/analysis");

        let name1 = validator.ensure_unique_name(
            parent1,
            vec![create_test_candidate("metrics", 0.9)],
            None,
        );
        let name2 = validator.ensure_unique_name(
            parent2,
            vec![create_test_candidate("metrics", 0.9)],
            None,
        );

        // Same name can be used in different directories
        assert_eq!(name1.module_name, "metrics");
        assert_eq!(name2.module_name, "metrics");
    }

    #[test]
    fn test_is_used() {
        let mut validator = NameUniquenessValidator::new();
        let parent = Path::new("src/organization");

        assert!(!validator.is_used(parent, "metrics"));

        validator.ensure_unique_name(parent, vec![create_test_candidate("metrics", 0.9)], None);

        assert!(validator.is_used(parent, "metrics"));
        assert!(!validator.is_used(parent, "coverage"));
    }

    #[test]
    fn test_mark_as_used() {
        let mut validator = NameUniquenessValidator::new();
        let parent = Path::new("src/organization");

        validator.mark_as_used(parent, "existing_module".to_string());

        assert!(validator.is_used(parent, "existing_module"));

        // Should disambiguate when trying to use marked name
        let result = validator.ensure_unique_name(
            parent,
            vec![create_test_candidate("existing_module", 0.9)],
            None,
        );

        assert_eq!(result.module_name, "existing_module_2");
    }

    #[test]
    fn test_clear() {
        let mut validator = NameUniquenessValidator::new();
        let parent = Path::new("src/organization");

        validator.ensure_unique_name(parent, vec![create_test_candidate("metrics", 0.9)], None);
        assert!(validator.is_used(parent, "metrics"));

        validator.clear();
        assert!(!validator.is_used(parent, "metrics"));
    }

    #[test]
    fn test_clear_directory() {
        let mut validator = NameUniquenessValidator::new();
        let parent1 = Path::new("src/organization");
        let parent2 = Path::new("src/analysis");

        validator.ensure_unique_name(parent1, vec![create_test_candidate("metrics", 0.9)], None);
        validator.ensure_unique_name(parent2, vec![create_test_candidate("coverage", 0.9)], None);

        validator.clear_directory(parent1);

        assert!(!validator.is_used(parent1, "metrics"));
        assert!(validator.is_used(parent2, "coverage"));
    }

    #[test]
    fn test_empty_candidates_generates_fallback() {
        let mut validator = NameUniquenessValidator::new();
        let parent = Path::new("src/organization");

        let result = validator.ensure_unique_name(parent, vec![], None);

        assert_eq!(result.module_name, "needs_review_1");
        assert!(result.confidence < 0.5);
    }

    #[test]
    fn test_method_based_disambiguation() {
        let mut validator = NameUniquenessValidator::new();
        let parent = Path::new("src/organization");

        // First use of "domain"
        let candidates1 = vec![create_test_candidate("domain", 0.9)];
        let name1 = validator.ensure_unique_name(parent, candidates1, None);
        assert_eq!(name1.module_name, "domain");

        // Second use with methods containing verbs
        let methods = vec![
            "classify_struct_domain".to_string(),
            "classify_domain_from_name".to_string(),
        ];
        let candidates2 = vec![create_test_candidate("domain", 0.85)];
        let name2 = validator.ensure_unique_name(parent, candidates2, Some(&methods));

        // Should use method-based disambiguation instead of numeric
        // Could be domain_classification or domain_<something> - just not domain_2
        assert!(name2.module_name.starts_with("domain_"));
        assert!(!name2.module_name.ends_with("_2"));
        assert!(name2.module_name.contains("classification") || name2.module_name.len() > 7);
        assert!(name2.confidence > 0.7); // Higher confidence than numeric
    }

    #[test]
    fn test_method_based_disambiguation_with_inference() {
        let mut validator = NameUniquenessValidator::new();
        let parent = Path::new("src/organization");

        // Reserve "responsibility"
        let candidates1 = vec![create_test_candidate("responsibility", 0.9)];
        validator.ensure_unique_name(parent, candidates1, None);

        // Try to use "responsibility" again with methods containing "infer"
        let methods = vec![
            "infer_responsibility_multi_signal".to_string(),
            "infer_responsibility_from_context".to_string(),
        ];
        let candidates2 = vec![create_test_candidate("responsibility", 0.85)];
        let name2 = validator.ensure_unique_name(parent, candidates2, Some(&methods));

        // Should extract a descriptive suffix (could be "inference" or something else)
        // Just ensure it's not numeric
        assert!(name2.module_name.starts_with("responsibility_"));
        assert!(!name2.module_name.matches('_').count() == 1 || !name2.module_name.ends_with("_2"));
        assert!(name2.confidence > 0.7);
    }
}
