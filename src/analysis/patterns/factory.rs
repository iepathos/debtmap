//! Factory pattern recognition
//!
//! Detects the Factory pattern in code by identifying:
//! - Functions with factory-style names (create_, make_, build_, etc.)
//! - Factory method patterns
//! - Factory class patterns

use super::{Implementation, PatternInstance, PatternRecognizer, PatternType};
use crate::core::{FileMetrics, FunctionMetrics};

pub struct FactoryPatternRecognizer;

impl FactoryPatternRecognizer {
    pub fn new() -> Self {
        Self
    }

    /// Check if a function name suggests it's a factory function
    fn is_factory_function(&self, function: &FunctionMetrics) -> bool {
        let name_lower = function.name.to_lowercase();

        // Name-based heuristics for factory functions
        name_lower.contains("create")
            || name_lower.contains("make")
            || name_lower.contains("build")
            || name_lower.contains("factory")
            || name_lower.starts_with("get_")
            || name_lower.starts_with("new_")
    }

    /// Detect factory registries from module scope analysis
    ///
    /// Looks for module-level assignments that might be factory registries,
    /// such as dictionaries mapping type names to classes.
    ///
    /// NOTE: Full AST traversal for detecting dictionary-based registries
    /// is not yet implemented. This is a heuristic-based approach that looks
    /// for naming patterns like 'registry', 'factories', 'builders', etc.
    fn detect_factory_registries(&self, file_metrics: &FileMetrics) -> Vec<Implementation> {
        let module_scope = match &file_metrics.module_scope {
            Some(scope) => scope,
            None => return vec![],
        };

        module_scope
            .assignments
            .iter()
            .filter(|assignment| {
                let name_lower = assignment.name.to_lowercase();
                name_lower.contains("registry")
                    || name_lower.contains("factories")
                    || name_lower.contains("builders")
                    || name_lower.contains("creators")
            })
            .map(|assignment| Implementation {
                file: file_metrics.path.clone(),
                class_name: None,
                function_name: assignment.name.clone(),
                line: assignment.line,
            })
            .collect()
    }
}

impl Default for FactoryPatternRecognizer {
    fn default() -> Self {
        Self::new()
    }
}

impl PatternRecognizer for FactoryPatternRecognizer {
    fn name(&self) -> &str {
        "Factory"
    }

    fn detect(&self, file_metrics: &FileMetrics) -> Vec<PatternInstance> {
        let mut patterns = Vec::new();

        // Detect factory functions
        let factory_functions: Vec<PatternInstance> = file_metrics
            .complexity
            .functions
            .iter()
            .filter(|function| self.is_factory_function(function))
            .map(|function| PatternInstance {
                pattern_type: PatternType::Factory,
                confidence: 0.7,
                base_class: None,
                implementations: vec![Implementation {
                    file: file_metrics.path.clone(),
                    class_name: None,
                    function_name: function.name.clone(),
                    line: function.line,
                }],
                usage_sites: vec![],
                reasoning: format!("Factory function {} (name-based detection)", function.name),
            })
            .collect();

        patterns.extend(factory_functions);

        // Detect factory registries
        let registries = self.detect_factory_registries(file_metrics);
        if !registries.is_empty() {
            patterns.push(PatternInstance {
                pattern_type: PatternType::Factory,
                confidence: 0.8,
                base_class: None,
                implementations: registries.clone(),
                usage_sites: vec![],
                reasoning: format!(
                    "Factory registry pattern with {} registry definition(s)",
                    registries.len()
                ),
            });
        }

        patterns
    }

    fn is_function_used_by_pattern(
        &self,
        _function: &FunctionMetrics,
        _file_metrics: &FileMetrics,
    ) -> Option<PatternInstance> {
        // Factory pattern primarily affects the factory itself and instantiated classes
        // For single-file detection, we don't mark individual functions as used by factory
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{ComplexityMetrics, Language};
    use std::path::PathBuf;

    fn create_test_function(name: &str, line: usize) -> FunctionMetrics {
        FunctionMetrics {
            name: name.to_string(),
            file: PathBuf::from("test.py"),
            line,
            cyclomatic: 1,
            cognitive: 0,
            nesting: 0,
            length: 10,
            is_test: false,
            visibility: None,
            is_trait_method: false,
            in_test_module: false,
            entropy_score: None,
            is_pure: None,
            purity_confidence: None,
            detected_patterns: None,
            upstream_callers: None,
            downstream_callees: None,
            mapping_pattern_result: None,
            adjusted_complexity: None,
            composition_metrics: None,
        }
    }

    fn create_test_file_metrics_with_functions(functions: Vec<FunctionMetrics>) -> FileMetrics {
        FileMetrics {
            path: PathBuf::from("test.py"),
            language: Language::Python,
            complexity: ComplexityMetrics {
                functions,
                cyclomatic_complexity: 0,
                cognitive_complexity: 0,
            },
            debt_items: vec![],
            dependencies: vec![],
            duplications: vec![],
            module_scope: None,
            classes: None,
        }
    }

    #[test]
    fn test_is_factory_function_create() {
        let function = create_test_function("create_handler", 10);
        let recognizer = FactoryPatternRecognizer::new();
        assert!(recognizer.is_factory_function(&function));
    }

    #[test]
    fn test_is_factory_function_make() {
        let function = create_test_function("make_widget", 10);
        let recognizer = FactoryPatternRecognizer::new();
        assert!(recognizer.is_factory_function(&function));
    }

    #[test]
    fn test_is_factory_function_build() {
        let function = create_test_function("build_object", 10);
        let recognizer = FactoryPatternRecognizer::new();
        assert!(recognizer.is_factory_function(&function));
    }

    #[test]
    fn test_is_factory_function_get() {
        let function = create_test_function("get_instance", 10);
        let recognizer = FactoryPatternRecognizer::new();
        assert!(recognizer.is_factory_function(&function));
    }

    #[test]
    fn test_is_factory_function_new() {
        let function = create_test_function("new_connection", 10);
        let recognizer = FactoryPatternRecognizer::new();
        assert!(recognizer.is_factory_function(&function));
    }

    #[test]
    fn test_is_not_factory_function() {
        let function = create_test_function("process_data", 10);
        let recognizer = FactoryPatternRecognizer::new();
        assert!(!recognizer.is_factory_function(&function));
    }

    #[test]
    fn test_detect_factory_patterns() {
        let file_metrics = create_test_file_metrics_with_functions(vec![
            create_test_function("create_handler", 10),
            create_test_function("process_data", 20),
            create_test_function("make_widget", 30),
        ]);

        let recognizer = FactoryPatternRecognizer::new();
        let patterns = recognizer.detect(&file_metrics);

        assert_eq!(patterns.len(), 2);
        assert!(patterns
            .iter()
            .any(|p| p.implementations[0].function_name == "create_handler"));
        assert!(patterns
            .iter()
            .any(|p| p.implementations[0].function_name == "make_widget"));
    }

    #[test]
    fn test_factory_pattern_confidence() {
        let file_metrics =
            create_test_file_metrics_with_functions(vec![create_test_function("create_obj", 10)]);

        let recognizer = FactoryPatternRecognizer::new();
        let patterns = recognizer.detect(&file_metrics);

        assert_eq!(patterns.len(), 1);
        assert_eq!(patterns[0].confidence, 0.7);
        assert_eq!(patterns[0].pattern_type, PatternType::Factory);
    }

    #[test]
    fn test_factory_recognizer_name() {
        let recognizer = FactoryPatternRecognizer::new();
        assert_eq!(recognizer.name(), "Factory");
    }

    #[test]
    fn test_is_function_used_by_pattern_returns_none() {
        let function = create_test_function("create_handler", 10);
        let file_metrics = create_test_file_metrics_with_functions(vec![]);

        let recognizer = FactoryPatternRecognizer::new();
        let result = recognizer.is_function_used_by_pattern(&function, &file_metrics);

        assert!(result.is_none());
    }
}
