//! Singleton pattern recognition
//!
//! Detects the Singleton pattern in code by identifying:
//! - Module-level class instances
//! - Class-level static instances
//! - Singleton instance methods being called

use super::{Implementation, PatternInstance, PatternRecognizer, PatternType};
use crate::core::{FileMetrics, FunctionMetrics};

pub struct SingletonPatternRecognizer;

impl SingletonPatternRecognizer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SingletonPatternRecognizer {
    fn default() -> Self {
        Self::new()
    }
}

impl PatternRecognizer for SingletonPatternRecognizer {
    fn name(&self) -> &str {
        "Singleton"
    }

    fn detect(&self, file_metrics: &FileMetrics) -> Vec<PatternInstance> {
        let mut patterns = Vec::new();

        if let Some(module_scope) = &file_metrics.module_scope {
            for singleton in &module_scope.singleton_instances {
                patterns.push(PatternInstance {
                    pattern_type: PatternType::Singleton,
                    confidence: 0.9,
                    base_class: Some(singleton.class_name.clone()),
                    implementations: vec![Implementation {
                        file: file_metrics.path.clone(),
                        class_name: Some(singleton.class_name.clone()),
                        function_name: singleton.variable_name.clone(),
                        line: singleton.line,
                    }],
                    usage_sites: Vec::new(),
                    reasoning: format!(
                        "Module-level singleton: {} = {}()",
                        singleton.variable_name, singleton.class_name
                    ),
                });
            }
        }

        patterns
    }

    fn is_function_used_by_pattern(
        &self,
        function: &FunctionMetrics,
        file_metrics: &FileMetrics,
    ) -> Option<PatternInstance> {
        let parts: Vec<&str> = function.name.split("::").collect();
        let class_name = if parts.len() >= 2 {
            parts[0]
        } else {
            return None;
        };

        if let Some(module_scope) = &file_metrics.module_scope {
            if module_scope
                .singleton_instances
                .iter()
                .any(|s| s.class_name == class_name)
            {
                return Some(PatternInstance {
                    pattern_type: PatternType::Singleton,
                    confidence: 0.85,
                    base_class: Some(class_name.to_string()),
                    implementations: vec![Implementation {
                        file: file_metrics.path.clone(),
                        class_name: Some(class_name.to_string()),
                        function_name: function.name.clone(),
                        line: function.line,
                    }],
                    usage_sites: Vec::new(),
                    reasoning: format!("Method on singleton class {}", class_name),
                });
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{
        ast::ModuleScopeAnalysis, ast::SingletonInstance, ComplexityMetrics, Language,
    };
    use std::path::PathBuf;

    fn create_test_file_metrics_with_singleton(singleton: SingletonInstance) -> FileMetrics {
        FileMetrics {
            path: PathBuf::from("test.py"),
            language: Language::Python,
            complexity: ComplexityMetrics::default(),
            debt_items: vec![],
            dependencies: vec![],
            duplications: vec![],
            module_scope: Some(ModuleScopeAnalysis {
                assignments: vec![],
                singleton_instances: vec![singleton],
            }),
            classes: None,
        }
    }

    #[test]
    fn test_singleton_detection() {
        let singleton = SingletonInstance {
            variable_name: "manager".to_string(),
            class_name: "Manager".to_string(),
            line: 10,
        };

        let file_metrics = create_test_file_metrics_with_singleton(singleton);
        let recognizer = SingletonPatternRecognizer::new();
        let patterns = recognizer.detect(&file_metrics);

        assert_eq!(patterns.len(), 1);
        assert_eq!(patterns[0].pattern_type, PatternType::Singleton);
        assert_eq!(patterns[0].confidence, 0.9);
        assert_eq!(patterns[0].base_class, Some("Manager".to_string()));
    }

    #[test]
    fn test_is_function_used_by_singleton() {
        let singleton = SingletonInstance {
            variable_name: "manager".to_string(),
            class_name: "Manager".to_string(),
            line: 10,
        };

        let file_metrics = create_test_file_metrics_with_singleton(singleton);

        let function = FunctionMetrics {
            name: "Manager::process".to_string(),
            file: PathBuf::from("test.py"),
            line: 15,
            cyclomatic: 1,
            cognitive: 0,
            nesting: 0,
            length: 5,
            is_test: false,
            visibility: None,
            is_trait_method: false,
            in_test_module: false,
            entropy_score: None,
            is_pure: None,
            purity_confidence: None,
            purity_reason: None,
            call_dependencies: None,
            detected_patterns: None,
            upstream_callers: None,
            downstream_callees: None,
            mapping_pattern_result: None,
            adjusted_complexity: None,
            composition_metrics: None,
            language_specific: None,
            purity_level: None,
        };

        let recognizer = SingletonPatternRecognizer::new();
        let result = recognizer.is_function_used_by_pattern(&function, &file_metrics);

        assert!(result.is_some());
        let pattern = result.unwrap();
        assert_eq!(pattern.pattern_type, PatternType::Singleton);
        assert_eq!(pattern.confidence, 0.85);
    }

    #[test]
    fn test_singleton_name() {
        let recognizer = SingletonPatternRecognizer::new();
        assert_eq!(recognizer.name(), "Singleton");
    }
}
