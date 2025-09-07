use super::{module_detection::ModuleType, TestTarget};
use crate::core::ComplexityMetrics;
use im::HashMap;
use std::path::Path;

pub struct CriticalityScorer {
    patterns: HashMap<String, f64>,
}

impl Default for CriticalityScorer {
    fn default() -> Self {
        Self::new()
    }
}

impl CriticalityScorer {
    pub fn new() -> Self {
        let mut patterns = HashMap::new();
        patterns.insert("main".to_string(), 10.0);
        patterns.insert("lib".to_string(), 10.0);
        patterns.insert("core".to_string(), 8.0);
        patterns.insert("api".to_string(), 7.0);
        patterns.insert("service".to_string(), 6.0);
        patterns.insert("model".to_string(), 5.0);
        patterns.insert("handler".to_string(), 6.0);
        patterns.insert("controller".to_string(), 6.0);
        patterns.insert("repository".to_string(), 5.0);
        patterns.insert("util".to_string(), 3.0);
        patterns.insert("helper".to_string(), 3.0);
        patterns.insert("test".to_string(), 1.0);

        Self { patterns }
    }

    pub fn score(&self, target: &TestTarget) -> f64 {
        let base_score = self.pattern_match_score(&target.path);
        let dependency_factor = self.dependency_score(target);
        let size_factor = (target.lines as f64).ln() / 10.0;
        let debt_factor = 1.0 + (target.debt_items as f64 * 0.1);

        (base_score * dependency_factor * size_factor * debt_factor).max(0.0)
    }

    fn pattern_match_score(&self, path: &Path) -> f64 {
        let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        // Extract classification logic as pure function
        Self::classify_file_score(
            file_name,
            &path.to_string_lossy().to_lowercase(),
            &self.patterns,
        )
    }

    // Pure function for file classification
    fn classify_file_score(
        file_name: &str,
        path_str: &str,
        patterns: &HashMap<String, f64>,
    ) -> f64 {
        match file_name {
            "main.rs" | "lib.rs" => 10.0,
            _ => {
                let file_lower = file_name.to_lowercase();

                // Use functional chain instead of imperative if-let
                patterns
                    .iter()
                    .find(|(pattern, _)| file_lower.contains(*pattern))
                    .map(|(_, score)| *score)
                    .or_else(|| {
                        // Check path only if filename didn't match
                        patterns
                            .iter()
                            .find(|(pattern, _)| path_str.contains(*pattern))
                            .map(|(_, score)| *score * 0.8)
                    })
                    .unwrap_or(4.0)
            }
        }
    }

    fn dependency_score(&self, target: &TestTarget) -> f64 {
        let dependent_count = target.dependents.len() as f64;
        let dependency_count = target.dependencies.len() as f64;

        let dependent_factor = (1.0 + dependent_count / 10.0).clamp(1.0, 2.0);
        let dependency_factor = (1.0 + dependency_count / 20.0).clamp(1.0, 1.5);

        dependent_factor * dependency_factor
    }
}

pub struct EffortEstimator;

impl Default for EffortEstimator {
    fn default() -> Self {
        Self::new()
    }
}

impl EffortEstimator {
    pub fn new() -> Self {
        Self
    }

    pub fn estimate(&self, target: &TestTarget) -> f64 {
        let base_effort = self.complexity_to_test_cases(&target.complexity);
        let setup_effort = self.estimate_setup_complexity(target);
        let mock_effort = self.estimate_mocking_needs(target);

        base_effort + setup_effort + mock_effort
    }

    fn complexity_to_test_cases(&self, complexity: &ComplexityMetrics) -> f64 {
        let min_cases = complexity.cyclomatic_complexity as f64 + 1.0;
        let cognitive_factor = (complexity.cognitive_complexity as f64 / 10.0).max(1.0);
        min_cases * cognitive_factor
    }

    fn estimate_setup_complexity(&self, target: &TestTarget) -> f64 {
        match target.module_type {
            ModuleType::EntryPoint => 5.0,
            ModuleType::IO => 3.0,
            ModuleType::Api => 2.0,
            ModuleType::Core => 1.0,
            _ => 0.5,
        }
    }

    fn estimate_mocking_needs(&self, target: &TestTarget) -> f64 {
        let dep_count = target.dependencies.len() as f64;
        dep_count * 0.5
    }

    pub fn explain(&self, target: &TestTarget) -> String {
        let base = self.complexity_to_test_cases(&target.complexity);
        let setup = self.estimate_setup_complexity(target);
        let mocking = self.estimate_mocking_needs(target);

        format!(
            "Estimated effort: {:.0} (base: {:.0}, setup: {:.0}, mocking: {:.0})",
            base + setup + mocking,
            base,
            setup,
            mocking
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_classify_file_score_main_rs() {
        let patterns = HashMap::new();
        let score = CriticalityScorer::classify_file_score("main.rs", "src/main.rs", &patterns);
        assert_eq!(score, 10.0, "main.rs should have maximum score");
    }

    #[test]
    fn test_classify_file_score_lib_rs() {
        let patterns = HashMap::new();
        let score = CriticalityScorer::classify_file_score("lib.rs", "src/lib.rs", &patterns);
        assert_eq!(score, 10.0, "lib.rs should have maximum score");
    }

    #[test]
    fn test_classify_file_score_filename_pattern_match() {
        let mut patterns = HashMap::new();
        patterns.insert("api".to_string(), 7.0);
        patterns.insert("service".to_string(), 6.0);

        let score = CriticalityScorer::classify_file_score(
            "api_handler.rs",
            "src/api_handler.rs",
            &patterns,
        );
        assert_eq!(score, 7.0, "Should match 'api' pattern in filename");

        let score = CriticalityScorer::classify_file_score(
            "user_service.rs",
            "src/user_service.rs",
            &patterns,
        );
        assert_eq!(score, 6.0, "Should match 'service' pattern in filename");
    }

    #[test]
    fn test_classify_file_score_path_pattern_match() {
        let mut patterns = HashMap::new();
        patterns.insert("core".to_string(), 8.0);

        // Pattern in path but not filename should be scored at 80%
        let score =
            CriticalityScorer::classify_file_score("utils.rs", "src/core/utils.rs", &patterns);
        assert_eq!(
            score,
            8.0 * 0.8,
            "Should match 'core' pattern in path with 0.8 factor"
        );
    }

    #[test]
    fn test_classify_file_score_filename_priority_over_path() {
        let mut patterns = HashMap::new();
        patterns.insert("test".to_string(), 1.0);
        patterns.insert("core".to_string(), 8.0);

        // Filename match should take priority over path match
        let score = CriticalityScorer::classify_file_score(
            "test_utils.rs",
            "src/core/test_utils.rs",
            &patterns,
        );
        assert_eq!(
            score, 1.0,
            "Filename pattern should take priority over path pattern"
        );
    }

    #[test]
    fn test_classify_file_score_default() {
        let patterns = HashMap::new();
        let score =
            CriticalityScorer::classify_file_score("random.rs", "src/misc/random.rs", &patterns);
        assert_eq!(
            score, 4.0,
            "Should return default score when no patterns match"
        );
    }

    #[test]
    fn test_classify_file_score_case_insensitive() {
        let mut patterns = HashMap::new();
        patterns.insert("api".to_string(), 7.0);

        let score = CriticalityScorer::classify_file_score(
            "API_Handler.rs",
            "src/API_Handler.rs",
            &patterns,
        );
        assert_eq!(score, 7.0, "Should match patterns case-insensitively");
    }

    #[test]
    fn test_pattern_match_score_integration() {
        let scorer = CriticalityScorer::new();

        // Test with real path
        let path = PathBuf::from("src/main.rs");
        let score = scorer.pattern_match_score(&path);
        assert_eq!(score, 10.0, "main.rs should score 10.0");

        // Test with API path
        let path = PathBuf::from("src/api/handler.rs");
        let score = scorer.pattern_match_score(&path);
        assert!(score > 4.0, "API handler should score higher than default");
    }

    // Helper function for creating test targets
    fn create_test_target() -> TestTarget {
        TestTarget {
            id: "test".to_string(),
            path: PathBuf::from("test.rs"),
            function: None,
            line: 0,
            module_type: ModuleType::Unknown,
            current_coverage: 0.0,
            current_risk: 0.0,
            complexity: ComplexityMetrics::default(),
            dependencies: vec![],
            dependents: vec![],
            lines: 0,
            priority_score: 0.0,
            debt_items: 0,
        }
    }

    // Tests for EffortEstimator
    #[test]
    fn test_effort_estimator_new() {
        let estimator = EffortEstimator::new();
        // Just verify it can be created
        assert!(
            std::mem::size_of_val(&estimator) == 0,
            "EffortEstimator should be zero-sized"
        );
    }

    #[test]
    fn test_effort_estimator_default() {
        let estimator = EffortEstimator;
        assert!(
            std::mem::size_of_val(&estimator) == 0,
            "Default should create zero-sized type"
        );
    }

    #[test]
    fn test_complexity_to_test_cases_simple() {
        let estimator = EffortEstimator::new();
        let complexity = ComplexityMetrics {
            cyclomatic_complexity: 5,
            cognitive_complexity: 10,
            ..Default::default()
        };

        // Expected: (5 + 1) * max(10/10, 1.0) = 6 * 1 = 6
        let cases = estimator.complexity_to_test_cases(&complexity);
        assert_eq!(
            cases, 6.0,
            "Should calculate correct test cases for simple complexity"
        );
    }

    #[test]
    fn test_complexity_to_test_cases_high_cognitive() {
        let estimator = EffortEstimator::new();
        let complexity = ComplexityMetrics {
            cyclomatic_complexity: 3,
            cognitive_complexity: 30,
            ..Default::default()
        };

        // Expected: (3 + 1) * max(30/10, 1.0) = 4 * 3 = 12
        let cases = estimator.complexity_to_test_cases(&complexity);
        assert_eq!(cases, 12.0, "Should scale with cognitive complexity");
    }

    #[test]
    fn test_complexity_to_test_cases_minimal() {
        let estimator = EffortEstimator::new();
        let complexity = ComplexityMetrics {
            cyclomatic_complexity: 1,
            cognitive_complexity: 0,
            ..Default::default()
        };

        // Expected: (1 + 1) * max(0/10, 1.0) = 2 * 1 = 2
        let cases = estimator.complexity_to_test_cases(&complexity);
        assert_eq!(cases, 2.0, "Should handle minimal complexity");
    }

    #[test]
    fn test_estimate_setup_complexity_entry_point() {
        let estimator = EffortEstimator::new();
        let mut target = create_test_target();
        target.module_type = ModuleType::EntryPoint;

        let setup = estimator.estimate_setup_complexity(&target);
        assert_eq!(setup, 5.0, "EntryPoint should have setup effort of 5.0");
    }

    #[test]
    fn test_estimate_setup_complexity_io() {
        let estimator = EffortEstimator::new();
        let mut target = create_test_target();
        target.module_type = ModuleType::IO;

        let setup = estimator.estimate_setup_complexity(&target);
        assert_eq!(setup, 3.0, "IO should have setup effort of 3.0");
    }

    #[test]
    fn test_estimate_setup_complexity_api() {
        let estimator = EffortEstimator::new();
        let mut target = create_test_target();
        target.module_type = ModuleType::Api;

        let setup = estimator.estimate_setup_complexity(&target);
        assert_eq!(setup, 2.0, "Api should have setup effort of 2.0");
    }

    #[test]
    fn test_estimate_setup_complexity_core() {
        let estimator = EffortEstimator::new();
        let mut target = create_test_target();
        target.module_type = ModuleType::Core;

        let setup = estimator.estimate_setup_complexity(&target);
        assert_eq!(setup, 1.0, "Core should have setup effort of 1.0");
    }

    #[test]
    fn test_estimate_setup_complexity_other() {
        let estimator = EffortEstimator::new();
        let mut target = create_test_target();
        target.module_type = ModuleType::Utility;

        let setup = estimator.estimate_setup_complexity(&target);
        assert_eq!(
            setup, 0.5,
            "Other module types should have setup effort of 0.5"
        );
    }

    #[test]
    fn test_estimate_mocking_needs_no_dependencies() {
        let estimator = EffortEstimator::new();
        let target = create_test_target();

        let mocking = estimator.estimate_mocking_needs(&target);
        assert_eq!(
            mocking, 0.0,
            "No dependencies should mean no mocking effort"
        );
    }

    #[test]
    fn test_estimate_mocking_needs_with_dependencies() {
        let estimator = EffortEstimator::new();
        let mut target = create_test_target();
        target.dependencies = vec!["dep1".to_string(), "dep2".to_string(), "dep3".to_string()];

        // Expected: 3 * 0.5 = 1.5
        let mocking = estimator.estimate_mocking_needs(&target);
        assert_eq!(mocking, 1.5, "Should calculate 0.5 effort per dependency");
    }

    #[test]
    fn test_estimate_full_calculation() {
        let estimator = EffortEstimator::new();
        let mut target = create_test_target();
        target.complexity = ComplexityMetrics {
            cyclomatic_complexity: 4,
            cognitive_complexity: 20,
            ..Default::default()
        };
        target.module_type = ModuleType::Api;
        target.dependencies = vec!["http".to_string(), "database".to_string()];

        // Base: (4 + 1) * max(20/10, 1.0) = 5 * 2 = 10
        // Setup: Api = 2.0
        // Mocking: 2 * 0.5 = 1.0
        // Total: 10 + 2 + 1 = 13
        let effort = estimator.estimate(&target);
        assert_eq!(effort, 13.0, "Should correctly sum all effort components");
    }

    #[test]
    fn test_estimate_edge_case_zero_complexity() {
        let estimator = EffortEstimator::new();
        let mut target = create_test_target();
        target.complexity = ComplexityMetrics {
            cyclomatic_complexity: 0,
            cognitive_complexity: 0,
            ..Default::default()
        };
        target.module_type = ModuleType::Core;

        // Base: (0 + 1) * max(0/10, 1.0) = 1 * 1 = 1
        // Setup: Core = 1.0
        // Mocking: 0 * 0.5 = 0
        // Total: 1 + 1 + 0 = 2
        let effort = estimator.estimate(&target);
        assert_eq!(effort, 2.0, "Should handle zero complexity gracefully");
    }

    #[test]
    fn test_explain_format() {
        let estimator = EffortEstimator::new();
        let mut target = create_test_target();
        target.complexity = ComplexityMetrics {
            cyclomatic_complexity: 3,
            cognitive_complexity: 15,
            ..Default::default()
        };
        target.module_type = ModuleType::IO;
        target.dependencies = vec!["fs".to_string()];

        let explanation = estimator.explain(&target);

        // Base: (3 + 1) * max(15/10, 1.0) = 4 * 1.5 = 6
        // Setup: IO = 3.0
        // Mocking: 1 * 0.5 = 0.5
        // Total: 6 + 3 + 0.5 = 9.5, rounded to 10

        assert!(
            explanation.contains("10"),
            "Should include total effort rounded"
        );
        assert!(
            explanation.contains("base: 6"),
            "Should include base effort"
        );
        assert!(
            explanation.contains("setup: 3"),
            "Should include setup effort"
        );
        assert!(
            explanation.contains("mocking: 0"),
            "Should include mocking effort"
        );
    }

    #[test]
    fn test_explain_complex_scenario() {
        let estimator = EffortEstimator::new();
        let mut target = create_test_target();
        target.complexity = ComplexityMetrics {
            cyclomatic_complexity: 10,
            cognitive_complexity: 50,
            ..Default::default()
        };
        target.module_type = ModuleType::EntryPoint;
        target.dependencies = vec![
            "a".to_string(),
            "b".to_string(),
            "c".to_string(),
            "d".to_string(),
        ];

        let explanation = estimator.explain(&target);

        // Base: (10 + 1) * max(50/10, 1.0) = 11 * 5 = 55
        // Setup: EntryPoint = 5.0
        // Mocking: 4 * 0.5 = 2.0
        // Total: 55 + 5 + 2 = 62

        assert!(
            explanation.contains("62"),
            "Should calculate high effort for complex function"
        );
        assert!(
            explanation.contains("base: 55"),
            "Should show high base effort"
        );
        assert!(
            explanation.contains("setup: 5"),
            "Should show entry point setup"
        );
        assert!(
            explanation.contains("mocking: 2"),
            "Should show mocking for 4 dependencies"
        );
    }
}
