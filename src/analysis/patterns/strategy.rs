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
        // Extract class name and method name from function name (e.g., "ClassName.method_name")
        let mut parts = function.name.split('.');
        let class_name = parts.next()?;
        let method_name = parts.next()?;

        if let Some(classes) = &file_metrics.classes {
            let class = classes.iter().find(|c| c.name == class_name)?;

            // Check if class implements a strategy interface
            for base_name in &class.base_classes {
                if let Some(base_class) = classes.iter().find(|c| &c.name == base_name) {
                    if self.is_strategy_interface(base_class) {
                        // Check if function implements an abstract method
                        let implements_abstract = base_class
                            .methods
                            .iter()
                            .any(|m| m.name == method_name && m.is_abstract);

                        if implements_abstract {
                            return Some(PatternInstance {
                                pattern_type: PatternType::Strategy,
                                confidence: 0.8,
                                base_class: Some(base_name.clone()),
                                implementations: vec![Implementation {
                                    file: file_metrics.path.clone(),
                                    class_name: Some(class_name.to_string()),
                                    function_name: function.name.clone(),
                                    line: function.line,
                                }],
                                usage_sites: Vec::new(),
                                reasoning: format!(
                                    "Implements strategy method {} from {}",
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
            detected_patterns: None,
            upstream_callers: None,
            downstream_callees: None,
        };

        let result = recognizer.is_function_used_by_pattern(&function, &file_metrics);
        assert!(result.is_some());
        let pattern = result.unwrap();
        assert_eq!(pattern.pattern_type, PatternType::Strategy);
    }
}
