//! Dependency Injection pattern recognition
//!
//! Detects the Dependency Injection pattern in Python code by identifying:
//! - Constructor injection (dependencies passed via __init__)
//! - Decorator-based injection (e.g., @inject, @autowired)
//! - Setter injection
//! - Field injection via decorators

use super::{Implementation, PatternInstance, PatternRecognizer, PatternType};
use crate::core::{ast::ClassDef, FileMetrics, FunctionMetrics};
use std::path::Path;

pub struct DependencyInjectionRecognizer;

impl DependencyInjectionRecognizer {
    pub fn new() -> Self {
        Self
    }

    /// Check if decorator indicates dependency injection
    fn is_injection_decorator(decorator: &str) -> bool {
        let decorator_lower = decorator.to_lowercase();
        decorator_lower.contains("inject")
            || decorator_lower.contains("autowired")
            || decorator_lower.contains("dependency")
            || decorator_lower.contains("provide")
    }

    /// Check if class uses constructor injection
    fn uses_constructor_injection(&self, class: &ClassDef) -> bool {
        class.methods.iter().any(|method| {
            if method.name == "__init__" {
                // Constructor injection typically has multiple parameters
                // This is a heuristic - we can't see actual parameters without full AST
                // But we can check for decorators or other hints
                !method.decorators.is_empty()
                    || method
                        .decorators
                        .iter()
                        .any(|d| Self::is_injection_decorator(d))
            } else {
                false
            }
        })
    }

    /// Check if class uses decorator-based injection
    fn uses_decorator_injection(&self, class: &ClassDef) -> bool {
        // Check class-level decorators
        class
            .decorators
            .iter()
            .any(|d| Self::is_injection_decorator(d))
            || class.methods.iter().any(|method| {
                method
                    .decorators
                    .iter()
                    .any(|d| Self::is_injection_decorator(d))
            })
    }

    /// Check if class uses setter injection
    fn uses_setter_injection(&self, class: &ClassDef) -> bool {
        class.methods.iter().any(|method| {
            (method.name.starts_with("set_") || method.name == "inject")
                && method
                    .decorators
                    .iter()
                    .any(|d| Self::is_injection_decorator(d))
        })
    }

    /// Count the number of injection decorators in a class
    ///
    /// This includes both class-level decorators and method-level decorators.
    /// Returns the total count of decorators that indicate dependency injection.
    fn count_injection_decorators(class: &ClassDef) -> usize {
        class
            .decorators
            .iter()
            .filter(|d| Self::is_injection_decorator(d))
            .count()
            + class
                .methods
                .iter()
                .flat_map(|m| &m.decorators)
                .filter(|d| Self::is_injection_decorator(d))
                .count()
    }

    /// Collect implementation details for methods that use dependency injection
    ///
    /// Returns a vector of implementations for methods that either:
    /// - Are the `__init__` constructor
    /// - Have injection decorators
    fn collect_injection_implementations(
        class: &ClassDef,
        file_path: &Path,
    ) -> Vec<Implementation> {
        class
            .methods
            .iter()
            .filter(|method| {
                method.name == "__init__"
                    || method
                        .decorators
                        .iter()
                        .any(|d| Self::is_injection_decorator(d))
            })
            .map(|method| Implementation {
                file: file_path.to_path_buf(),
                class_name: Some(class.name.clone()),
                function_name: method.name.clone(),
                line: method.line,
            })
            .collect()
    }

    /// Build a PatternInstance for a dependency injection pattern
    ///
    /// Creates a pattern instance with the appropriate confidence level,
    /// implementations, and reasoning based on the injection types detected.
    fn build_pattern_instance(
        class: &ClassDef,
        has_constructor: bool,
        has_decorator: bool,
        has_setter: bool,
        confidence: f32,
        implementations: Vec<Implementation>,
    ) -> PatternInstance {
        let mut injection_types = Vec::new();
        if has_constructor {
            injection_types.push("constructor");
        }
        if has_decorator {
            injection_types.push("decorator");
        }
        if has_setter {
            injection_types.push("setter");
        }

        PatternInstance {
            pattern_type: PatternType::DependencyInjection,
            confidence,
            base_class: Some(class.name.clone()),
            implementations,
            usage_sites: Vec::new(),
            reasoning: format!(
                "Class {} uses dependency injection via {}",
                class.name,
                injection_types.join(" and ")
            ),
        }
    }

    /// Calculate confidence based on evidence strength
    fn calculate_confidence(
        &self,
        has_constructor: bool,
        has_decorator: bool,
        has_setter: bool,
        decorator_count: usize,
    ) -> f32 {
        let mut confidence = 0.0;

        // Constructor injection is moderate evidence (could be normal initialization)
        if has_constructor {
            confidence += 0.4;
        }

        // Decorator injection is strong evidence
        if has_decorator {
            confidence += 0.5;
            // Multiple injection decorators increase confidence
            confidence += (decorator_count.saturating_sub(1) as f32) * 0.1;
        }

        // Setter injection is moderate evidence
        if has_setter {
            confidence += 0.3;
        }

        // Cap at 0.95 (leave room for uncertainty)
        confidence.min(0.95)
    }
}

impl Default for DependencyInjectionRecognizer {
    fn default() -> Self {
        Self::new()
    }
}

impl PatternRecognizer for DependencyInjectionRecognizer {
    fn name(&self) -> &str {
        "DependencyInjection"
    }

    fn detect(&self, file_metrics: &FileMetrics) -> Vec<PatternInstance> {
        let mut patterns = Vec::new();

        // Guard clause: early return if no classes
        let Some(classes) = &file_metrics.classes else {
            return patterns;
        };

        for class in classes {
            let has_constructor = self.uses_constructor_injection(class);
            let has_decorator = self.uses_decorator_injection(class);
            let has_setter = self.uses_setter_injection(class);

            // Guard clause: skip if no injection detected
            if !has_constructor && !has_decorator && !has_setter {
                continue;
            }

            // Count injection decorators
            let decorator_count = Self::count_injection_decorators(class);

            let confidence = self.calculate_confidence(
                has_constructor,
                has_decorator,
                has_setter,
                decorator_count,
            );

            // Collect implementations (methods with injection decorators)
            let implementations =
                Self::collect_injection_implementations(class, &file_metrics.path);

            patterns.push(Self::build_pattern_instance(
                class,
                has_constructor,
                has_decorator,
                has_setter,
                confidence,
                implementations,
            ));
        }

        patterns
    }

    fn is_function_used_by_pattern(
        &self,
        function: &FunctionMetrics,
        file_metrics: &FileMetrics,
    ) -> Option<PatternInstance> {
        // Extract class name and method name from function name
        let mut parts = function.name.split('.');
        let class_name = parts.next()?;
        let method_name = parts.next()?;

        if let Some(classes) = &file_metrics.classes {
            let class = classes.iter().find(|c| c.name == class_name)?;

            // Check if this method has injection decorators
            let method = class.methods.iter().find(|m| m.name == method_name)?;

            let has_injection_decorator = method
                .decorators
                .iter()
                .any(|d| Self::is_injection_decorator(d));

            let is_constructor = method_name == "__init__"
                && (self.uses_decorator_injection(class) || self.uses_setter_injection(class));

            if has_injection_decorator || is_constructor {
                return Some(PatternInstance {
                    pattern_type: PatternType::DependencyInjection,
                    confidence: if has_injection_decorator { 0.9 } else { 0.7 },
                    base_class: Some(class_name.to_string()),
                    implementations: vec![Implementation {
                        file: file_metrics.path.clone(),
                        class_name: Some(class_name.to_string()),
                        function_name: function.name.clone(),
                        line: function.line,
                    }],
                    usage_sites: Vec::new(),
                    reasoning: format!("Method {} uses dependency injection", function.name),
                });
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

    fn create_class_with_constructor_injection() -> ClassDef {
        ClassDef {
            name: "UserService".to_string(),
            base_classes: vec![],
            methods: vec![MethodDef {
                name: "__init__".to_string(),
                is_abstract: false,
                decorators: vec!["inject".to_string()],
                overrides_base: false,
                line: 10,
            }],
            is_abstract: false,
            decorators: vec![],
            line: 5,
        }
    }

    fn create_class_with_decorator_injection() -> ClassDef {
        ClassDef {
            name: "PaymentProcessor".to_string(),
            base_classes: vec![],
            methods: vec![MethodDef {
                name: "process".to_string(),
                is_abstract: false,
                decorators: vec!["autowired".to_string()],
                overrides_base: false,
                line: 20,
            }],
            is_abstract: false,
            decorators: vec!["injectable".to_string()],
            line: 18,
        }
    }

    #[test]
    fn test_is_injection_decorator() {
        assert!(DependencyInjectionRecognizer::is_injection_decorator(
            "inject"
        ));
        assert!(DependencyInjectionRecognizer::is_injection_decorator(
            "Autowired"
        ));
        assert!(DependencyInjectionRecognizer::is_injection_decorator(
            "dependency"
        ));
        assert!(!DependencyInjectionRecognizer::is_injection_decorator(
            "property"
        ));
    }

    #[test]
    fn test_count_injection_decorators_none() {
        let class = ClassDef {
            name: "PlainClass".to_string(),
            base_classes: vec![],
            methods: vec![],
            is_abstract: false,
            decorators: vec![],
            line: 1,
        };
        assert_eq!(
            DependencyInjectionRecognizer::count_injection_decorators(&class),
            0
        );
    }

    #[test]
    fn test_count_injection_decorators_class_level_only() {
        let class = ClassDef {
            name: "ServiceClass".to_string(),
            base_classes: vec![],
            methods: vec![],
            is_abstract: false,
            decorators: vec!["inject".to_string(), "autowired".to_string()],
            line: 1,
        };
        assert_eq!(
            DependencyInjectionRecognizer::count_injection_decorators(&class),
            2
        );
    }

    #[test]
    fn test_count_injection_decorators_method_level_only() {
        let class = ClassDef {
            name: "ServiceClass".to_string(),
            base_classes: vec![],
            methods: vec![
                MethodDef {
                    name: "method1".to_string(),
                    is_abstract: false,
                    decorators: vec!["inject".to_string()],
                    overrides_base: false,
                    line: 5,
                },
                MethodDef {
                    name: "method2".to_string(),
                    is_abstract: false,
                    decorators: vec!["provide".to_string()],
                    overrides_base: false,
                    line: 10,
                },
            ],
            is_abstract: false,
            decorators: vec![],
            line: 1,
        };
        assert_eq!(
            DependencyInjectionRecognizer::count_injection_decorators(&class),
            2
        );
    }

    #[test]
    fn test_count_injection_decorators_both_levels() {
        let class = ClassDef {
            name: "ServiceClass".to_string(),
            base_classes: vec![],
            methods: vec![MethodDef {
                name: "method1".to_string(),
                is_abstract: false,
                decorators: vec!["inject".to_string()],
                overrides_base: false,
                line: 5,
            }],
            is_abstract: false,
            decorators: vec!["autowired".to_string()],
            line: 1,
        };
        assert_eq!(
            DependencyInjectionRecognizer::count_injection_decorators(&class),
            2
        );
    }

    #[test]
    fn test_collect_injection_implementations_with_init() {
        let class = ClassDef {
            name: "UserService".to_string(),
            base_classes: vec![],
            methods: vec![MethodDef {
                name: "__init__".to_string(),
                is_abstract: false,
                decorators: vec![],
                overrides_base: false,
                line: 10,
            }],
            is_abstract: false,
            decorators: vec![],
            line: 5,
        };
        let path = PathBuf::from("test.py");
        let implementations =
            DependencyInjectionRecognizer::collect_injection_implementations(&class, &path);
        assert_eq!(implementations.len(), 1);
        assert_eq!(implementations[0].function_name, "__init__");
        assert_eq!(implementations[0].line, 10);
    }

    #[test]
    fn test_collect_injection_implementations_with_decorators() {
        let class = ClassDef {
            name: "PaymentProcessor".to_string(),
            base_classes: vec![],
            methods: vec![
                MethodDef {
                    name: "process".to_string(),
                    is_abstract: false,
                    decorators: vec!["inject".to_string()],
                    overrides_base: false,
                    line: 20,
                },
                MethodDef {
                    name: "validate".to_string(),
                    is_abstract: false,
                    decorators: vec!["autowired".to_string()],
                    overrides_base: false,
                    line: 30,
                },
            ],
            is_abstract: false,
            decorators: vec![],
            line: 15,
        };
        let path = PathBuf::from("test.py");
        let implementations =
            DependencyInjectionRecognizer::collect_injection_implementations(&class, &path);
        assert_eq!(implementations.len(), 2);
        assert_eq!(implementations[0].function_name, "process");
        assert_eq!(implementations[1].function_name, "validate");
    }

    #[test]
    fn test_collect_injection_implementations_with_both() {
        let class = create_class_with_constructor_injection();
        let path = PathBuf::from("test.py");
        let implementations =
            DependencyInjectionRecognizer::collect_injection_implementations(&class, &path);
        assert_eq!(implementations.len(), 1);
        assert_eq!(implementations[0].function_name, "__init__");
    }

    #[test]
    fn test_collect_injection_implementations_with_neither() {
        let class = ClassDef {
            name: "PlainClass".to_string(),
            base_classes: vec![],
            methods: vec![MethodDef {
                name: "do_something".to_string(),
                is_abstract: false,
                decorators: vec!["property".to_string()],
                overrides_base: false,
                line: 10,
            }],
            is_abstract: false,
            decorators: vec![],
            line: 5,
        };
        let path = PathBuf::from("test.py");
        let implementations =
            DependencyInjectionRecognizer::collect_injection_implementations(&class, &path);
        assert_eq!(implementations.len(), 0);
    }

    #[test]
    fn test_build_pattern_instance_constructor_only() {
        let class = create_class_with_constructor_injection();
        let implementations = vec![];
        let pattern = DependencyInjectionRecognizer::build_pattern_instance(
            &class,
            true,
            false,
            false,
            0.8,
            implementations,
        );
        assert_eq!(pattern.pattern_type, PatternType::DependencyInjection);
        assert_eq!(pattern.confidence, 0.8);
        assert!(pattern.reasoning.contains("constructor"));
        assert!(!pattern.reasoning.contains("decorator"));
    }

    #[test]
    fn test_build_pattern_instance_decorator_only() {
        let class = create_class_with_decorator_injection();
        let implementations = vec![];
        let pattern = DependencyInjectionRecognizer::build_pattern_instance(
            &class,
            false,
            true,
            false,
            0.9,
            implementations,
        );
        assert_eq!(pattern.pattern_type, PatternType::DependencyInjection);
        assert_eq!(pattern.confidence, 0.9);
        assert!(pattern.reasoning.contains("decorator"));
        assert!(!pattern.reasoning.contains("constructor"));
    }

    #[test]
    fn test_build_pattern_instance_setter_only() {
        let class = ClassDef {
            name: "ServiceClass".to_string(),
            base_classes: vec![],
            methods: vec![],
            is_abstract: false,
            decorators: vec![],
            line: 1,
        };
        let implementations = vec![];
        let pattern = DependencyInjectionRecognizer::build_pattern_instance(
            &class,
            false,
            false,
            true,
            0.7,
            implementations,
        );
        assert_eq!(pattern.pattern_type, PatternType::DependencyInjection);
        assert_eq!(pattern.confidence, 0.7);
        assert!(pattern.reasoning.contains("setter"));
        assert!(!pattern.reasoning.contains("constructor"));
    }

    #[test]
    fn test_build_pattern_instance_multiple_types() {
        let class = ClassDef {
            name: "FullServiceClass".to_string(),
            base_classes: vec![],
            methods: vec![],
            is_abstract: false,
            decorators: vec![],
            line: 1,
        };
        let implementations = vec![];
        let pattern = DependencyInjectionRecognizer::build_pattern_instance(
            &class,
            true,
            true,
            true,
            0.95,
            implementations,
        );
        assert_eq!(pattern.pattern_type, PatternType::DependencyInjection);
        assert_eq!(pattern.confidence, 0.95);
        assert!(pattern.reasoning.contains("constructor"));
        assert!(pattern.reasoning.contains("decorator"));
        assert!(pattern.reasoning.contains("setter"));
    }

    #[test]
    fn test_detect_constructor_injection() {
        let recognizer = DependencyInjectionRecognizer::new();
        let file_metrics = FileMetrics {
            path: PathBuf::from("services.py"),
            language: Language::Python,
            complexity: ComplexityMetrics::default(),
            debt_items: vec![],
            dependencies: vec![],
            duplications: vec![],
            total_lines: 0,
            module_scope: None,
            classes: Some(vec![create_class_with_constructor_injection()]),
        };

        let patterns = recognizer.detect(&file_metrics);
        assert_eq!(patterns.len(), 1);
        assert_eq!(patterns[0].pattern_type, PatternType::DependencyInjection);
        assert!(patterns[0].confidence >= 0.5);
        assert!(patterns[0].reasoning.contains("constructor"));
    }

    #[test]
    fn test_detect_decorator_injection() {
        let recognizer = DependencyInjectionRecognizer::new();
        let file_metrics = FileMetrics {
            path: PathBuf::from("services.py"),
            language: Language::Python,
            complexity: ComplexityMetrics::default(),
            debt_items: vec![],
            dependencies: vec![],
            duplications: vec![],
            total_lines: 0,
            module_scope: None,
            classes: Some(vec![create_class_with_decorator_injection()]),
        };

        let patterns = recognizer.detect(&file_metrics);
        assert_eq!(patterns.len(), 1);
        assert_eq!(patterns[0].pattern_type, PatternType::DependencyInjection);
        assert!(patterns[0].confidence >= 0.5);
        assert!(patterns[0].reasoning.contains("decorator"));
    }

    #[test]
    fn test_is_function_used_by_pattern() {
        let recognizer = DependencyInjectionRecognizer::new();
        let file_metrics = FileMetrics {
            path: PathBuf::from("services.py"),
            language: Language::Python,
            complexity: ComplexityMetrics::default(),
            debt_items: vec![],
            dependencies: vec![],
            duplications: vec![],
            total_lines: 0,
            module_scope: None,
            classes: Some(vec![create_class_with_decorator_injection()]),
        };

        let function = FunctionMetrics {
            name: "PaymentProcessor.process".to_string(),
            file: PathBuf::from("services.py"),
            line: 20,
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
            error_swallowing_count: None,
            error_swallowing_patterns: None,
        };

        let result = recognizer.is_function_used_by_pattern(&function, &file_metrics);
        assert!(result.is_some());
        let pattern = result.unwrap();
        assert_eq!(pattern.pattern_type, PatternType::DependencyInjection);
        assert!(pattern.confidence >= 0.7);
    }
}
