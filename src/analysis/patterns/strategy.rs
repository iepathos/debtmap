//! Strategy pattern recognition
//!
//! Detects the Strategy pattern in Python code by identifying:
//! - Strategy interfaces (Protocol, ABC with abstract methods)
//! - Concrete strategy implementations
//! - Strategy injection through constructors or setters

use super::{
    find_class_implementations, Implementation, PatternInstance, PatternRecognizer, PatternType,
};
use crate::core::{ast::ClassDef, FileMetrics, FunctionMetrics};

/// Extract class and method names from a qualified function name (e.g., "ClassName.method_name")
fn parse_qualified_name(name: &str) -> Option<(&str, &str)> {
    let mut parts = name.split('.');
    let class_name = parts.next()?;
    let method_name = parts.next()?;
    Some((class_name, method_name))
}

/// Check if a class has an abstract method with the given name
fn has_abstract_method(class: &ClassDef, method_name: &str) -> bool {
    class
        .methods
        .iter()
        .any(|m| m.name == method_name && m.is_abstract)
}

/// Find the first strategy base class that the given class extends and has an abstract method
fn find_strategy_base<'a>(
    class: &ClassDef,
    method_name: &str,
    classes: &'a [ClassDef],
    is_strategy: impl Fn(&ClassDef) -> bool,
) -> Option<&'a ClassDef> {
    class.base_classes.iter().find_map(|base_name| {
        classes
            .iter()
            .find(|c| &c.name == base_name)
            .filter(|base| is_strategy(base))
            .filter(|base| has_abstract_method(base, method_name))
    })
}

/// Create a PatternInstance for a detected strategy implementation
fn make_strategy_instance(
    base_name: &str,
    class_name: &str,
    function: &FunctionMetrics,
    file_path: &std::path::Path,
) -> PatternInstance {
    PatternInstance {
        pattern_type: PatternType::Strategy,
        confidence: 0.8,
        base_class: Some(base_name.to_string()),
        implementations: vec![Implementation {
            file: file_path.to_path_buf(),
            class_name: Some(class_name.to_string()),
            function_name: function.name.clone(),
            line: function.line,
        }],
        usage_sites: Vec::new(),
        reasoning: format!(
            "Implements strategy method {} from {}",
            function.name, base_name
        ),
    }
}

pub struct StrategyPatternRecognizer;

impl StrategyPatternRecognizer {
    pub fn new() -> Self {
        Self
    }

    /// Check if class is a strategy interface
    fn is_strategy_interface(&self, class: &ClassDef) -> bool {
        // Strategy interfaces often use Protocol or ABC
        let has_protocol_base = class
            .base_classes
            .iter()
            .any(|b| b.contains("Protocol") || b.contains("ABC") || b.contains("Strategy"));

        // Strategy interfaces typically have abstract methods
        let has_abstract_methods = class.methods.iter().any(|m| m.is_abstract);

        has_protocol_base && has_abstract_methods
    }

    /// Find strategy implementations
    fn find_strategy_implementations(
        &self,
        interface: &ClassDef,
        file_metrics: &FileMetrics,
    ) -> Vec<Implementation> {
        find_class_implementations(interface, file_metrics)
            .into_iter()
            .flat_map(|class| {
                class
                    .methods
                    .iter()
                    .filter_map(|method| {
                        if interface
                            .methods
                            .iter()
                            .any(|m| m.name == method.name && m.is_abstract)
                        {
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

    /// Detect strategy injection patterns (heuristic-based)
    fn find_strategy_injection_hints(&self, interface_name: &str, class: &ClassDef) -> bool {
        // Check for __init__ methods that might inject strategy
        // This is a simplified heuristic - full implementation would need AST parsing
        class.methods.iter().any(|method| method.name == "__init__")
            && class.base_classes.iter().all(|base| base != interface_name)
    }
}

impl Default for StrategyPatternRecognizer {
    fn default() -> Self {
        Self::new()
    }
}

impl PatternRecognizer for StrategyPatternRecognizer {
    fn name(&self) -> &str {
        "Strategy"
    }

    fn detect(&self, file_metrics: &FileMetrics) -> Vec<PatternInstance> {
        let mut patterns = Vec::new();

        if let Some(classes) = &file_metrics.classes {
            for class in classes {
                if self.is_strategy_interface(class) {
                    let implementations = self.find_strategy_implementations(class, file_metrics);

                    // Check for potential injection sites
                    let has_injection_hints = classes
                        .iter()
                        .any(|c| self.find_strategy_injection_hints(&class.name, c));

                    let impl_count = implementations.len();
                    let has_impls = impl_count > 0;

                    if has_impls || has_injection_hints {
                        let confidence = if has_impls {
                            0.85 // Higher confidence with implementations
                        } else {
                            0.6 // Lower confidence if only hints
                        };

                        patterns.push(PatternInstance {
                            pattern_type: PatternType::Strategy,
                            confidence,
                            base_class: Some(class.name.clone()),
                            implementations,
                            usage_sites: Vec::new(), // Would need full AST parsing
                            reasoning: format!(
                                "Strategy interface {} with {} implementation(s)",
                                class.name,
                                if has_impls { "concrete" } else { "potential" }
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
        let (class_name, method_name) = parse_qualified_name(&function.name)?;
        let classes = file_metrics.classes.as_ref()?;
        let class = classes.iter().find(|c| c.name == class_name)?;

        let strategy_base = find_strategy_base(class, method_name, classes, |c| {
            self.is_strategy_interface(c)
        })?;

        Some(make_strategy_instance(
            &strategy_base.name,
            class_name,
            function,
            &file_metrics.path,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{ast::MethodDef, ComplexityMetrics, Language};
    use std::path::PathBuf;

    fn create_strategy_interface() -> ClassDef {
        ClassDef {
            name: "PaymentStrategy".to_string(),
            base_classes: vec!["ABC".to_string()],
            methods: vec![MethodDef {
                name: "process_payment".to_string(),
                is_abstract: true,
                decorators: vec!["abstractmethod".to_string()],
                overrides_base: false,
                line: 10,
            }],
            is_abstract: true,
            decorators: vec![],
            line: 5,
        }
    }

    fn create_strategy_implementation() -> ClassDef {
        ClassDef {
            name: "CreditCardStrategy".to_string(),
            base_classes: vec!["PaymentStrategy".to_string()],
            methods: vec![MethodDef {
                name: "process_payment".to_string(),
                is_abstract: false,
                decorators: vec![],
                overrides_base: true,
                line: 20,
            }],
            is_abstract: false,
            decorators: vec![],
            line: 18,
        }
    }

    #[test]
    fn test_is_strategy_interface() {
        let recognizer = StrategyPatternRecognizer::new();
        let strategy = create_strategy_interface();
        assert!(recognizer.is_strategy_interface(&strategy));
    }

    #[test]
    fn test_detect_strategy_pattern() {
        let recognizer = StrategyPatternRecognizer::new();
        let file_metrics = FileMetrics {
            path: PathBuf::from("payment.py"),
            language: Language::Python,
            complexity: ComplexityMetrics::default(),
            debt_items: vec![],
            dependencies: vec![],
            duplications: vec![],
            total_lines: 0,
            module_scope: None,
            classes: Some(vec![
                create_strategy_interface(),
                create_strategy_implementation(),
            ]),
        };

        let patterns = recognizer.detect(&file_metrics);
        assert_eq!(patterns.len(), 1);
        assert_eq!(patterns[0].pattern_type, PatternType::Strategy);
        assert_eq!(patterns[0].implementations.len(), 1);
        assert!(patterns[0].confidence >= 0.8);
    }

    #[test]
    fn test_is_function_used_by_pattern() {
        let recognizer = StrategyPatternRecognizer::new();
        let file_metrics = FileMetrics {
            path: PathBuf::from("payment.py"),
            language: Language::Python,
            complexity: ComplexityMetrics::default(),
            debt_items: vec![],
            dependencies: vec![],
            duplications: vec![],
            total_lines: 0,
            module_scope: None,
            classes: Some(vec![
                create_strategy_interface(),
                create_strategy_implementation(),
            ]),
        };

        let function = FunctionMetrics {
            name: "CreditCardStrategy.process_payment".to_string(),
            file: PathBuf::from("payment.py"),
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
            entropy_analysis: None,
        };

        let result = recognizer.is_function_used_by_pattern(&function, &file_metrics);
        assert!(result.is_some());
        let pattern = result.unwrap();
        assert_eq!(pattern.pattern_type, PatternType::Strategy);
    }

    fn create_function_metrics(name: &str) -> FunctionMetrics {
        FunctionMetrics {
            name: name.to_string(),
            file: PathBuf::from("payment.py"),
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
            entropy_analysis: None,
        }
    }

    #[test]
    fn test_is_function_used_by_pattern_no_dot_in_name() {
        let recognizer = StrategyPatternRecognizer::new();
        let file_metrics = FileMetrics {
            path: PathBuf::from("payment.py"),
            language: Language::Python,
            complexity: ComplexityMetrics::default(),
            debt_items: vec![],
            dependencies: vec![],
            duplications: vec![],
            total_lines: 0,
            module_scope: None,
            classes: Some(vec![
                create_strategy_interface(),
                create_strategy_implementation(),
            ]),
        };

        // Function name without dot returns None (no class.method format)
        let function = create_function_metrics("process_payment");
        let result = recognizer.is_function_used_by_pattern(&function, &file_metrics);
        assert!(result.is_none());
    }

    #[test]
    fn test_is_function_used_by_pattern_no_classes() {
        let recognizer = StrategyPatternRecognizer::new();
        let file_metrics = FileMetrics {
            path: PathBuf::from("payment.py"),
            language: Language::Python,
            complexity: ComplexityMetrics::default(),
            debt_items: vec![],
            dependencies: vec![],
            duplications: vec![],
            total_lines: 0,
            module_scope: None,
            classes: None, // No classes in file
        };

        let function = create_function_metrics("CreditCardStrategy.process_payment");
        let result = recognizer.is_function_used_by_pattern(&function, &file_metrics);
        assert!(result.is_none());
    }

    #[test]
    fn test_is_function_used_by_pattern_class_not_found() {
        let recognizer = StrategyPatternRecognizer::new();
        let file_metrics = FileMetrics {
            path: PathBuf::from("payment.py"),
            language: Language::Python,
            complexity: ComplexityMetrics::default(),
            debt_items: vec![],
            dependencies: vec![],
            duplications: vec![],
            total_lines: 0,
            module_scope: None,
            classes: Some(vec![create_strategy_interface()]), // Only interface, no implementation
        };

        // Function references a class that doesn't exist in file_metrics
        let function = create_function_metrics("NonExistentClass.process_payment");
        let result = recognizer.is_function_used_by_pattern(&function, &file_metrics);
        assert!(result.is_none());
    }

    #[test]
    fn test_is_function_used_by_pattern_base_class_not_found() {
        let recognizer = StrategyPatternRecognizer::new();

        // Create implementation that references a base class not in the file
        let orphan_impl = ClassDef {
            name: "OrphanStrategy".to_string(),
            base_classes: vec!["MissingBaseClass".to_string()],
            methods: vec![MethodDef {
                name: "execute".to_string(),
                is_abstract: false,
                decorators: vec![],
                overrides_base: true,
                line: 20,
            }],
            is_abstract: false,
            decorators: vec![],
            line: 18,
        };

        let file_metrics = FileMetrics {
            path: PathBuf::from("payment.py"),
            language: Language::Python,
            complexity: ComplexityMetrics::default(),
            debt_items: vec![],
            dependencies: vec![],
            duplications: vec![],
            total_lines: 0,
            module_scope: None,
            classes: Some(vec![orphan_impl]),
        };

        let function = create_function_metrics("OrphanStrategy.execute");
        let result = recognizer.is_function_used_by_pattern(&function, &file_metrics);
        assert!(result.is_none());
    }

    #[test]
    fn test_is_function_used_by_pattern_base_not_strategy_interface() {
        let recognizer = StrategyPatternRecognizer::new();

        // Regular base class (not a strategy interface)
        let regular_base = ClassDef {
            name: "RegularBase".to_string(),
            base_classes: vec![],
            methods: vec![MethodDef {
                name: "some_method".to_string(),
                is_abstract: false, // Not abstract
                decorators: vec![],
                overrides_base: false,
                line: 10,
            }],
            is_abstract: false,
            decorators: vec![],
            line: 5,
        };

        let derived = ClassDef {
            name: "DerivedClass".to_string(),
            base_classes: vec!["RegularBase".to_string()],
            methods: vec![MethodDef {
                name: "some_method".to_string(),
                is_abstract: false,
                decorators: vec![],
                overrides_base: true,
                line: 20,
            }],
            is_abstract: false,
            decorators: vec![],
            line: 18,
        };

        let file_metrics = FileMetrics {
            path: PathBuf::from("test.py"),
            language: Language::Python,
            complexity: ComplexityMetrics::default(),
            debt_items: vec![],
            dependencies: vec![],
            duplications: vec![],
            total_lines: 0,
            module_scope: None,
            classes: Some(vec![regular_base, derived]),
        };

        let function = create_function_metrics("DerivedClass.some_method");
        let result = recognizer.is_function_used_by_pattern(&function, &file_metrics);
        assert!(result.is_none());
    }

    #[test]
    fn test_is_function_used_by_pattern_method_not_abstract() {
        let recognizer = StrategyPatternRecognizer::new();
        let file_metrics = FileMetrics {
            path: PathBuf::from("payment.py"),
            language: Language::Python,
            complexity: ComplexityMetrics::default(),
            debt_items: vec![],
            dependencies: vec![],
            duplications: vec![],
            total_lines: 0,
            module_scope: None,
            classes: Some(vec![
                create_strategy_interface(),
                create_strategy_implementation(),
            ]),
        };

        // Function references a method that's not in the abstract interface
        let function = create_function_metrics("CreditCardStrategy.helper_method");
        let result = recognizer.is_function_used_by_pattern(&function, &file_metrics);
        assert!(result.is_none());
    }

    #[test]
    fn test_is_function_used_by_pattern_multiple_base_classes() {
        let recognizer = StrategyPatternRecognizer::new();

        let strategy_interface = create_strategy_interface();

        // Class with multiple base classes, one of which is the strategy interface
        let multi_inheritance = ClassDef {
            name: "MultiInheritance".to_string(),
            base_classes: vec![
                "SomeOtherBase".to_string(),
                "PaymentStrategy".to_string(), // Second base is strategy
            ],
            methods: vec![MethodDef {
                name: "process_payment".to_string(),
                is_abstract: false,
                decorators: vec![],
                overrides_base: true,
                line: 25,
            }],
            is_abstract: false,
            decorators: vec![],
            line: 22,
        };

        let file_metrics = FileMetrics {
            path: PathBuf::from("payment.py"),
            language: Language::Python,
            complexity: ComplexityMetrics::default(),
            debt_items: vec![],
            dependencies: vec![],
            duplications: vec![],
            total_lines: 0,
            module_scope: None,
            classes: Some(vec![strategy_interface, multi_inheritance]),
        };

        let function = create_function_metrics("MultiInheritance.process_payment");
        let result = recognizer.is_function_used_by_pattern(&function, &file_metrics);
        assert!(result.is_some());
        let pattern = result.unwrap();
        assert_eq!(pattern.pattern_type, PatternType::Strategy);
        assert_eq!(pattern.base_class, Some("PaymentStrategy".to_string()));
    }

    #[test]
    fn test_is_function_used_by_pattern_protocol_base() {
        let recognizer = StrategyPatternRecognizer::new();

        // Strategy using Protocol instead of ABC
        let protocol_strategy = ClassDef {
            name: "Serializer".to_string(),
            base_classes: vec!["Protocol".to_string()],
            methods: vec![MethodDef {
                name: "serialize".to_string(),
                is_abstract: true,
                decorators: vec!["abstractmethod".to_string()],
                overrides_base: false,
                line: 10,
            }],
            is_abstract: true,
            decorators: vec![],
            line: 5,
        };

        let json_serializer = ClassDef {
            name: "JsonSerializer".to_string(),
            base_classes: vec!["Serializer".to_string()],
            methods: vec![MethodDef {
                name: "serialize".to_string(),
                is_abstract: false,
                decorators: vec![],
                overrides_base: true,
                line: 20,
            }],
            is_abstract: false,
            decorators: vec![],
            line: 18,
        };

        let file_metrics = FileMetrics {
            path: PathBuf::from("serializer.py"),
            language: Language::Python,
            complexity: ComplexityMetrics::default(),
            debt_items: vec![],
            dependencies: vec![],
            duplications: vec![],
            total_lines: 0,
            module_scope: None,
            classes: Some(vec![protocol_strategy, json_serializer]),
        };

        let function = create_function_metrics("JsonSerializer.serialize");
        let result = recognizer.is_function_used_by_pattern(&function, &file_metrics);
        assert!(result.is_some());
        let pattern = result.unwrap();
        assert_eq!(pattern.confidence, 0.8);
        assert!(pattern.reasoning.contains("serialize"));
    }
}
