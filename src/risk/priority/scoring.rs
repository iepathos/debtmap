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

        (base_score * dependency_factor * size_factor * debt_factor).clamp(0.0, 10.0)
    }

    fn pattern_match_score(&self, path: &Path) -> f64 {
        let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        match file_name {
            "main.rs" | "lib.rs" => return 10.0,
            _ => {}
        }

        let file_lower = file_name.to_lowercase();
        if let Some(score) = self
            .patterns
            .iter()
            .find(|(pattern, _)| file_lower.contains(*pattern))
            .map(|(_, score)| *score)
        {
            return score;
        }

        let path_str = path.to_string_lossy().to_lowercase();
        if let Some(score) = self
            .patterns
            .iter()
            .find(|(pattern, _)| path_str.contains(*pattern))
            .map(|(_, score)| *score * 0.8)
        {
            return score;
        }

        4.0
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
