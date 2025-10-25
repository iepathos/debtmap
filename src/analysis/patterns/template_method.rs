//! Template Method pattern recognition
//!
//! Detects the Template Method pattern in Python code by identifying:
//! - Template method base classes with non-abstract template methods
//! - Overridden hook methods in subclasses
//! - Abstract methods meant to be implemented by subclasses

use super::{
    find_class_implementations, Implementation, PatternInstance, PatternRecognizer, PatternType,
};
use crate::core::{ast::ClassDef, FileMetrics, FunctionMetrics};

pub struct TemplateMethodPatternRecognizer;

impl TemplateMethodPatternRecognizer {
    pub fn new() -> Self {
        Self
    }

    /// Check if method is a template method
    /// Template methods are typically non-abstract methods that coordinate
    /// calls to other methods (hooks) that can be overridden
    fn is_template_method(&self, class: &ClassDef) -> bool {
        // Heuristic: Class has both abstract and non-abstract methods
        let has_abstract = class.methods.iter().any(|m| m.is_abstract);
        let has_non_abstract = class.methods.iter().any(|m| !m.is_abstract);

        has_abstract && has_non_abstract
    }

    /// Find overridden template methods in implementations
    fn find_overridden_methods(
        &self,
        base_class: &ClassDef,
        file_metrics: &FileMetrics,
    ) -> Vec<Implementation> {
        find_class_implementations(base_class, file_metrics)
            .into_iter()
            .flat_map(|class| {
                class
                    .methods
                    .iter()
                    .filter_map(|method| {
                        // Check if method overrides a base class method
                        if base_class.methods.iter().any(|m| m.name == method.name) {
                            Some(Implementation {
                                file: file_metrics.path.clone(),
                                class_name: Some(class.name.clone()),
                                function_name: method.name.clone(),
                                line: method.line,
                            })
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>()
            })
            .collect()
    }
}

impl Default for TemplateMethodPatternRecognizer {
    fn default() -> Self {
        Self::new()
    }
}

impl PatternRecognizer for TemplateMethodPatternRecognizer {
    fn name(&self) -> &str {
        "TemplateMethod"
    }

    fn detect(&self, file_metrics: &FileMetrics) -> Vec<PatternInstance> {
        let mut patterns = Vec::new();

        if let Some(classes) = &file_metrics.classes {
            for class in classes {
                // Look for classes with template methods
                if self.is_template_method(class) {
                    let overridden = self.find_overridden_methods(class, file_metrics);

                    if !overridden.is_empty() {
                        patterns.push(PatternInstance {
                            pattern_type: PatternType::TemplateMethod,
                            confidence: 0.75, // Lower confidence (heuristic-based)
                            base_class: Some(class.name.clone()),
                            implementations: overridden,
                            usage_sites: Vec::new(),
                            reasoning: format!(
                                "Template method pattern in {} with {} overridden method(s)",
                                class.name,
                                patterns.len()
                            ),
                        });
                    }
                }
            }
        }

        patterns
    }

    fn is_function_used_by_pattern(
        &self,
        function: &FunctionMetrics,
        file_metrics: &FileMetrics,
    ) -> Option<PatternInstance> {
        // Extract class name and method name from function name (e.g., "ClassName.method_name")
        let mut parts = function.name.split('.');
        let class_name = parts.next()?;
        let method_name = parts.next()?;

        if let Some(classes) = &file_metrics.classes {
            let class = classes.iter().find(|c| c.name == class_name)?;

            // Check if method overrides a template method
            for base_name in &class.base_classes {
                if let Some(base_class) = classes.iter().find(|c| &c.name == base_name) {
                    // Check if base class has template pattern
                    if self.is_template_method(base_class) {
                        // Check if this function overrides a base class method
                        if base_class.methods.iter().any(|m| m.name == method_name) {
                            return Some(PatternInstance {
                                pattern_type: PatternType::TemplateMethod,
                                confidence: 0.7,
                                base_class: Some(base_name.clone()),
                                implementations: vec![Implementation {
                                    file: file_metrics.path.clone(),
                                    class_name: Some(class_name.to_string()),
                                    function_name: function.name.clone(),
                                    line: function.line,
                                }],
                                usage_sites: Vec::new(),
                                reasoning: format!(
                                    "Overrides template method {} from {}",
                                    function.name, base_name
                                ),
                            });
                        }
                    }
                }
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{ast::MethodDef, ComplexityMetrics, Language};
    use std::path::PathBuf;

    fn create_template_base_class() -> ClassDef {
        ClassDef {
            name: "DataProcessor".to_string(),
            base_classes: vec!["ABC".to_string()],
            methods: vec![
                MethodDef {
                    name: "process".to_string(),
                    is_abstract: false, // Template method (non-abstract)
                    decorators: vec![],
                    overrides_base: false,
                    line: 10,
                },
                MethodDef {
                    name: "load_data".to_string(),
                    is_abstract: true, // Hook method (abstract)
                    decorators: vec!["abstractmethod".to_string()],
                    overrides_base: false,
                    line: 15,
                },
                MethodDef {
                    name: "transform_data".to_string(),
                    is_abstract: true, // Hook method (abstract)
                    decorators: vec!["abstractmethod".to_string()],
                    overrides_base: false,
                    line: 20,
                },
            ],
            is_abstract: true,
            decorators: vec![],
            line: 5,
        }
    }

    fn create_template_implementation() -> ClassDef {
        ClassDef {
            name: "CSVProcessor".to_string(),
            base_classes: vec!["DataProcessor".to_string()],
            methods: vec![
                MethodDef {
                    name: "load_data".to_string(),
                    is_abstract: false,
                    decorators: vec![],
                    overrides_base: true,
                    line: 30,
                },
                MethodDef {
                    name: "transform_data".to_string(),
                    is_abstract: false,
                    decorators: vec![],
                    overrides_base: true,
                    line: 35,
                },
            ],
            is_abstract: false,
            decorators: vec![],
            line: 28,
        }
    }

    #[test]
    fn test_is_template_method() {
        let recognizer = TemplateMethodPatternRecognizer::new();
        let template_class = create_template_base_class();
        assert!(recognizer.is_template_method(&template_class));
    }

    #[test]
    fn test_detect_template_method_pattern() {
        let recognizer = TemplateMethodPatternRecognizer::new();
        let file_metrics = FileMetrics {
            path: PathBuf::from("processor.py"),
            language: Language::Python,
            complexity: ComplexityMetrics::default(),
            debt_items: vec![],
            dependencies: vec![],
            duplications: vec![],
            module_scope: None,
            classes: Some(vec![
                create_template_base_class(),
                create_template_implementation(),
            ]),
        };

        let patterns = recognizer.detect(&file_metrics);
        assert_eq!(patterns.len(), 1);
        assert_eq!(patterns[0].pattern_type, PatternType::TemplateMethod);
        assert_eq!(patterns[0].implementations.len(), 2); // load_data and transform_data
    }

    #[test]
    fn test_is_function_used_by_pattern() {
        let recognizer = TemplateMethodPatternRecognizer::new();
        let file_metrics = FileMetrics {
            path: PathBuf::from("processor.py"),
            language: Language::Python,
            complexity: ComplexityMetrics::default(),
            debt_items: vec![],
            dependencies: vec![],
            duplications: vec![],
            module_scope: None,
            classes: Some(vec![
                create_template_base_class(),
                create_template_implementation(),
            ]),
        };

        let function = FunctionMetrics {
            name: "CSVProcessor.load_data".to_string(),
            file: PathBuf::from("processor.py"),
            line: 30,
            cyclomatic: 1,
            cognitive: 1,
            nesting: 0,
            length: 5,
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
        };

        let result = recognizer.is_function_used_by_pattern(&function, &file_metrics);
        assert!(result.is_some());
        let pattern = result.unwrap();
        assert_eq!(pattern.pattern_type, PatternType::TemplateMethod);
    }
}
