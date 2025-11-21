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
    /// are already used, disambiguates the best candidate with a numeric suffix.
    ///
    /// # Arguments
    ///
    /// * `parent_path` - Parent directory where module will be created
    /// * `candidates` - List of name candidates (should be sorted by confidence)
    ///
    /// # Returns
    ///
    /// A unique name candidate, possibly disambiguated
    pub fn ensure_unique_name(
        &mut self,
        parent_path: &Path,
        mut candidates: Vec<NameCandidate>,
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

        // All candidates are used, disambiguate the best one
        let base_candidate = candidates.remove(0);
        let unique_name = Self::disambiguate_name_static(used, &base_candidate.module_name);

        used.insert(unique_name.clone());

        NameCandidate {
            module_name: unique_name.clone(),
            confidence: base_candidate.confidence * 0.8, // Lower confidence for disambiguated
            specificity_score: base_candidate.specificity_score,
            reasoning: format!(
                "{} (disambiguated to avoid collision)",
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

        let result = validator.ensure_unique_name(parent, candidates);

        assert_eq!(result.module_name, "metrics");
        assert_eq!(result.confidence, 0.9);
    }

    #[test]
    fn test_collision_disambiguates() {
        let mut validator = NameUniquenessValidator::new();
        let parent = Path::new("src/organization");

        let candidates1 = vec![create_test_candidate("metrics", 0.9)];
        let candidates2 = vec![create_test_candidate("metrics", 0.85)];

        let name1 = validator.ensure_unique_name(parent, candidates1);
        let name2 = validator.ensure_unique_name(parent, candidates2);

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
        validator.ensure_unique_name(parent, candidates1);

        // Provide multiple candidates, first is taken
        let candidates2 = vec![
            create_test_candidate("metrics", 0.85),     // Already used
            create_test_candidate("computation", 0.80), // Available
            create_test_candidate("analysis", 0.75),
        ];

        let result = validator.ensure_unique_name(parent, candidates2);

        assert_eq!(result.module_name, "computation");
        assert_eq!(result.confidence, 0.80);
    }

    #[test]
    fn test_multiple_disambiguations() {
        let mut validator = NameUniquenessValidator::new();
        let parent = Path::new("src/organization");

        let name1 =
            validator.ensure_unique_name(parent, vec![create_test_candidate("metrics", 0.9)]);
        let name2 =
            validator.ensure_unique_name(parent, vec![create_test_candidate("metrics", 0.85)]);
        let name3 =
            validator.ensure_unique_name(parent, vec![create_test_candidate("metrics", 0.80)]);

        assert_eq!(name1.module_name, "metrics");
        assert_eq!(name2.module_name, "metrics_2");
        assert_eq!(name3.module_name, "metrics_3");
    }

    #[test]
    fn test_different_directories_independent() {
        let mut validator = NameUniquenessValidator::new();
        let parent1 = Path::new("src/organization");
        let parent2 = Path::new("src/analysis");

        let name1 =
            validator.ensure_unique_name(parent1, vec![create_test_candidate("metrics", 0.9)]);
        let name2 =
            validator.ensure_unique_name(parent2, vec![create_test_candidate("metrics", 0.9)]);

        // Same name can be used in different directories
        assert_eq!(name1.module_name, "metrics");
        assert_eq!(name2.module_name, "metrics");
    }

    #[test]
    fn test_is_used() {
        let mut validator = NameUniquenessValidator::new();
        let parent = Path::new("src/organization");

        assert!(!validator.is_used(parent, "metrics"));

        validator.ensure_unique_name(parent, vec![create_test_candidate("metrics", 0.9)]);

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
        let result = validator
            .ensure_unique_name(parent, vec![create_test_candidate("existing_module", 0.9)]);

        assert_eq!(result.module_name, "existing_module_2");
    }

    #[test]
    fn test_clear() {
        let mut validator = NameUniquenessValidator::new();
        let parent = Path::new("src/organization");

        validator.ensure_unique_name(parent, vec![create_test_candidate("metrics", 0.9)]);
        assert!(validator.is_used(parent, "metrics"));

        validator.clear();
        assert!(!validator.is_used(parent, "metrics"));
    }

    #[test]
    fn test_clear_directory() {
        let mut validator = NameUniquenessValidator::new();
        let parent1 = Path::new("src/organization");
        let parent2 = Path::new("src/analysis");

        validator.ensure_unique_name(parent1, vec![create_test_candidate("metrics", 0.9)]);
        validator.ensure_unique_name(parent2, vec![create_test_candidate("coverage", 0.9)]);

        validator.clear_directory(parent1);

        assert!(!validator.is_used(parent1, "metrics"));
        assert!(validator.is_used(parent2, "coverage"));
    }

    #[test]
    fn test_empty_candidates_generates_fallback() {
        let mut validator = NameUniquenessValidator::new();
        let parent = Path::new("src/organization");

        let result = validator.ensure_unique_name(parent, vec![]);

        assert_eq!(result.module_name, "needs_review_1");
        assert!(result.confidence < 0.5);
    }
}
