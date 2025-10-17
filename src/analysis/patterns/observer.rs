//! Observer pattern recognition
//!
//! Detects the Observer pattern in Python code by identifying:
//! - Abstract base classes with @abstractmethod decorators
//! - Concrete implementations inheriting from observer interfaces
//! - Observer notification loops

use super::{
    find_class_implementations, is_abstract_base, Implementation, PatternInstance,
    PatternRecognizer, PatternType,
};
use crate::core::{ast::ClassDef, FileMetrics, FunctionMetrics};

pub struct ObserverPatternRecognizer;

impl ObserverPatternRecognizer {
    pub fn new() -> Self {
        Self
    }

    /// Find concrete implementations of observer interface in the same file
    fn find_implementations(
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

    /// Detect observer invocation loops by checking method implementations
    ///
    /// This checks for patterns where collections named 'observers', 'callbacks',
    /// 'listeners', etc. are likely iterated over to call observer methods.
    ///
    /// NOTE: Full AST traversal for loop detection is not yet implemented.
    /// This is a heuristic-based approach that looks for methods with names
    /// like 'notify', 'notify_all', 'trigger', etc. that likely contain
    /// observer invocation loops.
    fn has_observer_invocation_patterns(&self, class: &ClassDef) -> bool {
        // Check for common notification method names
        let notification_methods = ["notify", "notify_all", "trigger", "fire", "emit", "broadcast"];

        class.methods.iter().any(|method| {
            let method_lower = method.name.to_lowercase();
            notification_methods.iter().any(|pattern| method_lower.contains(pattern))
        })
    }
}

impl Default for ObserverPatternRecognizer {
    fn default() -> Self {
        Self::new()
    }
}

impl PatternRecognizer for ObserverPatternRecognizer {
    fn name(&self) -> &str {
        "Observer"
    }

    fn detect(&self, file_metrics: &FileMetrics) -> Vec<PatternInstance> {
        let classes = match &file_metrics.classes {
            Some(classes) => classes,
            None => return vec![],
        };

        classes
            .iter()
            .filter(|class| is_abstract_base(class))
            .filter_map(|interface| {
                let implementations = self.find_implementations(interface, file_metrics);

                if implementations.is_empty() {
                    None
                } else {
                    // Check if any implementation class has observer invocation patterns
                    let has_invocation = classes.iter().any(|class| {
                        class.base_classes.contains(&interface.name)
                            || self.has_observer_invocation_patterns(class)
                    });

                    let confidence = if has_invocation { 0.95 } else { 0.85 };

                    let reasoning = if has_invocation {
                        format!(
                            "Observer interface {} with {} concrete implementation(s) and invocation pattern detected",
                            interface.name,
                            implementations.len()
                        )
                    } else {
                        format!(
                            "Observer interface {} with {} concrete implementation(s)",
                            interface.name,
                            implementations.len()
                        )
                    };

                    Some(PatternInstance {
                        pattern_type: PatternType::Observer,
                        confidence,
                        base_class: Some(interface.name.clone()),
                        implementations: implementations.clone(),
                        usage_sites: vec![],
                        reasoning,
                    })
                }
            })
            .collect()
    }

    fn is_function_used_by_pattern(
        &self,
        function: &FunctionMetrics,
        file_metrics: &FileMetrics,
    ) -> Option<PatternInstance> {
        let classes = file_metrics.classes.as_ref()?;

        // Extract class name from function name if it exists
        // Function names in metrics might be in format "ClassName::method_name"
        let parts: Vec<&str> = function.name.split("::").collect();
        let (class_name, method_name) = if parts.len() == 2 {
            (parts[0], parts[1])
        } else {
            // If no :: separator, use the entire name as method name
            return None;
        };

        // Find the class containing this function
        let class = classes.iter().find(|c| c.name == class_name)?;

        // Check if class implements an observer interface
        for base_class_name in &class.base_classes {
            if let Some(base_class) = classes.iter().find(|c| &c.name == base_class_name) {
                if is_abstract_base(base_class) {
                    // Check if this method overrides an abstract method
                    if base_class
                        .methods
                        .iter()
                        .any(|m| m.name == method_name && m.is_abstract)
                    {
                        return Some(PatternInstance {
                            pattern_type: PatternType::Observer,
                            confidence: 0.85,
                            base_class: Some(base_class.name.clone()),
                            implementations: vec![Implementation {
                                file: file_metrics.path.clone(),
                                class_name: Some(class_name.to_string()),
                                function_name: method_name.to_string(),
                                line: function.line,
                            }],
                            usage_sites: vec![],
                            reasoning: format!(
                                "Implements abstract method {} from observer interface {}",
                                method_name, base_class.name
                            ),
                        });
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

    fn create_test_file_metrics_with_classes(classes: Vec<ClassDef>) -> FileMetrics {
        FileMetrics {
            path: PathBuf::from("test.py"),
            language: Language::Python,
            complexity: ComplexityMetrics::default(),
            debt_items: vec![],
            dependencies: vec![],
            duplications: vec![],
            module_scope: None,
            classes: Some(classes),
        }
    }

    #[test]
    fn test_observer_interface_detection_no_implementations() {
        let file_metrics = create_test_file_metrics_with_classes(vec![ClassDef {
            name: "Observer".to_string(),
            base_classes: vec!["ABC".to_string()],
            methods: vec![MethodDef {
                name: "on_event".to_string(),
                is_abstract: true,
                decorators: vec!["abstractmethod".to_string()],
                overrides_base: false,
                line: 10,
            }],
            is_abstract: true,
            decorators: vec![],
            line: 5,
        }]);

        let recognizer = ObserverPatternRecognizer::new();
        let patterns = recognizer.detect(&file_metrics);

        // No patterns because there are no implementations
        assert_eq!(patterns.len(), 0);
    }

    #[test]
    fn test_observer_implementation_detection() {
        let file_metrics = create_test_file_metrics_with_classes(vec![
            ClassDef {
                name: "Observer".to_string(),
                base_classes: vec!["ABC".to_string()],
                methods: vec![MethodDef {
                    name: "on_event".to_string(),
                    is_abstract: true,
                    decorators: vec!["abstractmethod".to_string()],
                    overrides_base: false,
                    line: 10,
                }],
                is_abstract: true,
                decorators: vec![],
                line: 5,
            },
            ClassDef {
                name: "ConcreteObserver".to_string(),
                base_classes: vec!["Observer".to_string()],
                methods: vec![MethodDef {
                    name: "on_event".to_string(),
                    is_abstract: false,
                    decorators: vec![],
                    overrides_base: true,
                    line: 20,
                }],
                is_abstract: false,
                decorators: vec![],
                line: 18,
            },
        ]);

        let recognizer = ObserverPatternRecognizer::new();
        let patterns = recognizer.detect(&file_metrics);

        assert_eq!(patterns.len(), 1);
        assert_eq!(patterns[0].pattern_type, PatternType::Observer);
        assert_eq!(patterns[0].implementations.len(), 1);
        assert_eq!(patterns[0].implementations[0].function_name, "on_event");
        assert_eq!(
            patterns[0].implementations[0].class_name,
            Some("ConcreteObserver".to_string())
        );
    }

    #[test]
    fn test_observer_name() {
        let recognizer = ObserverPatternRecognizer::new();
        assert_eq!(recognizer.name(), "Observer");
    }

    #[test]
    fn test_is_function_used_by_pattern() {
        let file_metrics = create_test_file_metrics_with_classes(vec![
            ClassDef {
                name: "Observer".to_string(),
                base_classes: vec!["ABC".to_string()],
                methods: vec![MethodDef {
                    name: "on_event".to_string(),
                    is_abstract: true,
                    decorators: vec!["abstractmethod".to_string()],
                    overrides_base: false,
                    line: 10,
                }],
                is_abstract: true,
                decorators: vec![],
                line: 5,
            },
            ClassDef {
                name: "ConcreteObserver".to_string(),
                base_classes: vec!["Observer".to_string()],
                methods: vec![MethodDef {
                    name: "on_event".to_string(),
                    is_abstract: false,
                    decorators: vec![],
                    overrides_base: true,
                    line: 20,
                }],
                is_abstract: false,
                decorators: vec![],
                line: 18,
            },
        ]);

        let function = FunctionMetrics {
            name: "ConcreteObserver::on_event".to_string(),
            file: PathBuf::from("test.py"),
            line: 20,
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
            detected_patterns: None,
            upstream_callers: None,
            downstream_callees: None,
        };

        let recognizer = ObserverPatternRecognizer::new();
        let result = recognizer.is_function_used_by_pattern(&function, &file_metrics);

        assert!(result.is_some());
        let pattern = result.unwrap();
        assert_eq!(pattern.pattern_type, PatternType::Observer);
        assert_eq!(pattern.confidence, 0.85);
    }
}
