//! Design pattern detection module
//!
//! This module provides pattern detection capabilities for identifying
//! common design patterns in code, such as Observer, Factory, and Callback patterns.

pub mod callback;
pub mod config;
pub mod dependency_injection;
pub mod factory;
pub mod observer;
pub mod rust_traits;
pub mod singleton;
pub mod strategy;
pub mod template_method;

use crate::analysis::call_graph::TraitRegistry;
use crate::core::{ast::ClassDef, FileMetrics, FunctionMetrics};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;

// Stub for removed Python cross-module context
#[derive(Debug, Clone, Default)]
pub struct CrossModuleContext;

impl CrossModuleContext {
    pub fn new() -> Self {
        CrossModuleContext
    }
}

/// Types of design patterns that can be detected
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PatternType {
    Observer,
    Factory,
    Callback,
    Singleton,
    Strategy,
    TemplateMethod,
    DependencyInjection,
}

/// A detected instance of a design pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternInstance {
    pub pattern_type: PatternType,
    pub confidence: f32,
    pub base_class: Option<String>,
    pub implementations: Vec<Implementation>,
    pub usage_sites: Vec<UsageSite>,
    pub reasoning: String,
}

/// Implementation details for a pattern instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Implementation {
    pub file: PathBuf,
    pub class_name: Option<String>,
    pub function_name: String,
    pub line: usize,
}

/// Usage site information for a pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageSite {
    pub file: PathBuf,
    pub line: usize,
    pub context: String,
}

/// Trait for pattern recognition implementations
pub trait PatternRecognizer: Send + Sync {
    fn name(&self) -> &str;
    fn detect(&self, file_metrics: &FileMetrics) -> Vec<PatternInstance>;
    fn is_function_used_by_pattern(
        &self,
        function: &FunctionMetrics,
        file_metrics: &FileMetrics,
    ) -> Option<PatternInstance>;
}

/// Main pattern detector that coordinates all pattern recognizers
pub struct PatternDetector {
    recognizers: Vec<Box<dyn PatternRecognizer>>,
    cross_module_context: Option<Arc<CrossModuleContext>>,
    trait_registry: Option<Arc<TraitRegistry>>,
}

impl PatternDetector {
    pub fn new() -> Self {
        Self {
            recognizers: vec![
                Box::new(observer::ObserverPatternRecognizer::new()),
                Box::new(factory::FactoryPatternRecognizer::new()),
                Box::new(callback::CallbackPatternRecognizer::new()),
                Box::new(singleton::SingletonPatternRecognizer::new()),
                Box::new(strategy::StrategyPatternRecognizer::new()),
                Box::new(template_method::TemplateMethodPatternRecognizer::new()),
                Box::new(dependency_injection::DependencyInjectionRecognizer::new()),
            ],
            cross_module_context: None,
            trait_registry: None,
        }
    }

    /// Add cross-module context for cross-file pattern detection
    pub fn with_cross_module_context(mut self, context: Arc<CrossModuleContext>) -> Self {
        self.cross_module_context = Some(context);
        self
    }

    /// Add trait registry for Rust trait pattern detection
    pub fn with_trait_registry(mut self, registry: Arc<TraitRegistry>) -> Self {
        self.trait_registry = Some(registry);
        self
    }

    pub fn detect_all_patterns(&self, file_metrics: &FileMetrics) -> Vec<PatternInstance> {
        self.recognizers
            .iter()
            .flat_map(|recognizer| recognizer.detect(file_metrics))
            .collect()
    }

    pub fn is_function_used_by_pattern(
        &self,
        function: &FunctionMetrics,
        file_metrics: &FileMetrics,
    ) -> Option<PatternInstance> {
        self.recognizers
            .iter()
            .find_map(|recognizer| recognizer.is_function_used_by_pattern(function, file_metrics))
    }

    /// Detect patterns across multiple files using cross-module context
    pub fn detect_cross_file_patterns(&self, all_files: &[FileMetrics]) -> Vec<PatternInstance> {
        let mut patterns = Vec::new();

        if let Some(context) = &self.cross_module_context {
            patterns.extend(self.detect_cross_file_observer_patterns(all_files, context));
        }

        if let Some(registry) = &self.trait_registry {
            let rust_recognizer = rust_traits::RustTraitPatternRecognizer::new(registry.clone());
            patterns.extend(rust_recognizer.detect_trait_observer_patterns());
        }

        patterns
    }

    /// Detect observer patterns that span multiple files
    fn detect_cross_file_observer_patterns(
        &self,
        all_files: &[FileMetrics],
        context: &CrossModuleContext,
    ) -> Vec<PatternInstance> {
        let mut patterns = Vec::new();

        for file_metrics in all_files {
            if let Some(classes) = &file_metrics.classes {
                for interface in classes {
                    if !is_abstract_base(interface) {
                        continue;
                    }

                    let implementations = self.find_cross_file_implementations(
                        interface,
                        &file_metrics.path,
                        all_files,
                        context,
                    );

                    if !implementations.is_empty() {
                        patterns.push(PatternInstance {
                            pattern_type: PatternType::Observer,
                            confidence: 0.95,
                            base_class: Some(interface.name.clone()),
                            implementations,
                            usage_sites: Vec::new(),
                            reasoning: format!(
                                "Cross-file observer interface {} with implementations in other files",
                                interface.name
                            ),
                        });
                    }
                }
            }
        }

        patterns
    }

    /// Find implementations of an interface across all files.
    ///
    /// Uses functional iteration to find classes that inherit from the interface
    /// and collect their method implementations.
    fn find_cross_file_implementations(
        &self,
        interface: &ClassDef,
        interface_file: &std::path::Path,
        all_files: &[FileMetrics],
        context: &CrossModuleContext,
    ) -> Vec<Implementation> {
        all_files
            .iter()
            .flat_map(|file_metrics| {
                self.find_implementations_in_file(file_metrics, interface, interface_file, context)
            })
            .collect()
    }

    /// Find interface implementations within a single file.
    fn find_implementations_in_file(
        &self,
        file_metrics: &FileMetrics,
        interface: &ClassDef,
        interface_file: &std::path::Path,
        context: &CrossModuleContext,
    ) -> Vec<Implementation> {
        file_metrics
            .classes
            .as_ref()
            .map(|classes| {
                classes
                    .iter()
                    .filter(|class| {
                        self.inherits_from_interface(
                            class,
                            interface,
                            &file_metrics.path,
                            interface_file,
                            context,
                        )
                    })
                    .flat_map(|class| {
                        find_class_method_implementations(class, interface, &file_metrics.path)
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Check if a class inherits from an interface (possibly in a different file)
    fn inherits_from_interface(
        &self,
        class: &ClassDef,
        interface: &ClassDef,
        class_file: &std::path::Path,
        interface_file: &std::path::Path,
        _context: &CrossModuleContext,
    ) -> bool {
        // Simplified without Python cross-module support
        if class.base_classes.contains(&interface.name) && class_file == interface_file {
            return true;
        }

        false
    }
}

impl Default for PatternDetector {
    fn default() -> Self {
        Self::new()
    }
}

/// Check if a method name matches an abstract method in the interface.
///
/// This is a pure predicate function for filtering methods.
fn implements_abstract_method(method_name: &str, interface: &ClassDef) -> bool {
    interface
        .methods
        .iter()
        .any(|m| m.name == method_name && m.is_abstract)
}

/// Find all method implementations in a class that implement abstract methods from an interface.
///
/// Returns an iterator of Implementation structs for matching methods.
fn find_class_method_implementations<'a>(
    class: &'a ClassDef,
    interface: &'a ClassDef,
    file_path: &'a std::path::Path,
) -> impl Iterator<Item = Implementation> + 'a {
    class
        .methods
        .iter()
        .filter(move |method| implements_abstract_method(&method.name, interface))
        .map(move |method| Implementation {
            file: file_path.to_path_buf(),
            class_name: Some(class.name.clone()),
            function_name: method.name.clone(),
            line: method.line,
        })
}

/// Helper function to check if a class is an abstract base class
pub(crate) fn is_abstract_base(class: &ClassDef) -> bool {
    let has_abc_base = class
        .base_classes
        .iter()
        .any(|b| b.contains("ABC") || b.contains("Protocol") || b.contains("Interface"));

    let has_abstract_methods = class.methods.iter().any(|m| m.is_abstract);

    has_abc_base && has_abstract_methods
}

/// Helper function to find implementations of an interface
pub(crate) fn find_class_implementations<'a>(
    interface: &ClassDef,
    file_metrics: &'a FileMetrics,
) -> Vec<&'a ClassDef> {
    file_metrics
        .classes
        .as_ref()
        .map(|classes| {
            classes
                .iter()
                .filter(|class| class.base_classes.contains(&interface.name))
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{ast::MethodDef, ComplexityMetrics, Language};

    fn create_test_file_metrics() -> FileMetrics {
        FileMetrics {
            path: PathBuf::from("test.py"),
            language: Language::Python,
            complexity: ComplexityMetrics::default(),
            debt_items: vec![],
            dependencies: vec![],
            duplications: vec![],
            total_lines: 0,
            module_scope: None,
            classes: None,
        }
    }

    #[test]
    fn test_pattern_detector_creation() {
        let detector = PatternDetector::new();
        assert_eq!(detector.recognizers.len(), 7);
    }

    #[test]
    fn test_is_abstract_base_with_abc() {
        let class = ClassDef {
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
        };

        assert!(is_abstract_base(&class));
    }

    #[test]
    fn test_is_abstract_base_without_abc() {
        let class = ClassDef {
            name: "Regular".to_string(),
            base_classes: vec![],
            methods: vec![MethodDef {
                name: "method".to_string(),
                is_abstract: false,
                decorators: vec![],
                overrides_base: false,
                line: 10,
            }],
            is_abstract: false,
            decorators: vec![],
            line: 5,
        };

        assert!(!is_abstract_base(&class));
    }

    #[test]
    fn test_detect_all_patterns_empty() {
        let detector = PatternDetector::new();
        let metrics = create_test_file_metrics();
        let patterns = detector.detect_all_patterns(&metrics);
        assert_eq!(patterns.len(), 0);
    }

    #[test]
    fn test_pattern_detector_with_singleton() {
        let detector = PatternDetector::new();
        assert_eq!(detector.recognizers.len(), 7);
    }

    #[test]
    fn test_implements_abstract_method_matches() {
        let interface = ClassDef {
            name: "Observer".to_string(),
            base_classes: vec!["ABC".to_string()],
            methods: vec![
                MethodDef {
                    name: "on_event".to_string(),
                    is_abstract: true,
                    decorators: vec![],
                    overrides_base: false,
                    line: 10,
                },
                MethodDef {
                    name: "on_error".to_string(),
                    is_abstract: true,
                    decorators: vec![],
                    overrides_base: false,
                    line: 15,
                },
            ],
            is_abstract: true,
            decorators: vec![],
            line: 5,
        };

        assert!(implements_abstract_method("on_event", &interface));
        assert!(implements_abstract_method("on_error", &interface));
    }

    #[test]
    fn test_implements_abstract_method_no_match() {
        let interface = ClassDef {
            name: "Observer".to_string(),
            base_classes: vec!["ABC".to_string()],
            methods: vec![MethodDef {
                name: "on_event".to_string(),
                is_abstract: true,
                decorators: vec![],
                overrides_base: false,
                line: 10,
            }],
            is_abstract: true,
            decorators: vec![],
            line: 5,
        };

        assert!(!implements_abstract_method("other_method", &interface));
    }

    #[test]
    fn test_implements_abstract_method_non_abstract_not_matched() {
        let interface = ClassDef {
            name: "Observer".to_string(),
            base_classes: vec!["ABC".to_string()],
            methods: vec![MethodDef {
                name: "concrete_method".to_string(),
                is_abstract: false,
                decorators: vec![],
                overrides_base: false,
                line: 10,
            }],
            is_abstract: true,
            decorators: vec![],
            line: 5,
        };

        assert!(!implements_abstract_method("concrete_method", &interface));
    }

    #[test]
    fn test_find_class_method_implementations() {
        let interface = ClassDef {
            name: "Observer".to_string(),
            base_classes: vec!["ABC".to_string()],
            methods: vec![MethodDef {
                name: "on_event".to_string(),
                is_abstract: true,
                decorators: vec![],
                overrides_base: false,
                line: 10,
            }],
            is_abstract: true,
            decorators: vec![],
            line: 5,
        };

        let implementing_class = ClassDef {
            name: "ConcreteObserver".to_string(),
            base_classes: vec!["Observer".to_string()],
            methods: vec![
                MethodDef {
                    name: "on_event".to_string(),
                    is_abstract: false,
                    decorators: vec![],
                    overrides_base: true,
                    line: 20,
                },
                MethodDef {
                    name: "helper".to_string(),
                    is_abstract: false,
                    decorators: vec![],
                    overrides_base: false,
                    line: 25,
                },
            ],
            is_abstract: false,
            decorators: vec![],
            line: 15,
        };

        let file_path = PathBuf::from("test.py");
        let implementations: Vec<_> =
            find_class_method_implementations(&implementing_class, &interface, &file_path)
                .collect();

        assert_eq!(implementations.len(), 1);
        assert_eq!(implementations[0].function_name, "on_event");
        assert_eq!(
            implementations[0].class_name,
            Some("ConcreteObserver".to_string())
        );
        assert_eq!(implementations[0].line, 20);
    }

    #[test]
    fn test_find_class_method_implementations_empty() {
        let interface = ClassDef {
            name: "Observer".to_string(),
            base_classes: vec!["ABC".to_string()],
            methods: vec![MethodDef {
                name: "on_event".to_string(),
                is_abstract: true,
                decorators: vec![],
                overrides_base: false,
                line: 10,
            }],
            is_abstract: true,
            decorators: vec![],
            line: 5,
        };

        let unrelated_class = ClassDef {
            name: "Unrelated".to_string(),
            base_classes: vec![],
            methods: vec![MethodDef {
                name: "do_something".to_string(),
                is_abstract: false,
                decorators: vec![],
                overrides_base: false,
                line: 20,
            }],
            is_abstract: false,
            decorators: vec![],
            line: 15,
        };

        let file_path = PathBuf::from("test.py");
        let implementations: Vec<_> =
            find_class_method_implementations(&unrelated_class, &interface, &file_path).collect();

        assert!(implementations.is_empty());
    }
}
