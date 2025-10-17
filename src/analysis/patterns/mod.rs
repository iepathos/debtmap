//! Design pattern detection module
//!
//! This module provides pattern detection capabilities for identifying
//! common design patterns in code, such as Observer, Factory, and Callback patterns.

pub mod callback;
pub mod factory;
pub mod observer;

use crate::core::{ast::ClassDef, FileMetrics, FunctionMetrics};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Types of design patterns that can be detected
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PatternType {
    Observer,
    Factory,
    Callback,
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
}

impl PatternDetector {
    pub fn new() -> Self {
        Self {
            recognizers: vec![
                Box::new(observer::ObserverPatternRecognizer::new()),
                Box::new(factory::FactoryPatternRecognizer::new()),
                Box::new(callback::CallbackPatternRecognizer::new()),
            ],
        }
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
}

impl Default for PatternDetector {
    fn default() -> Self {
        Self::new()
    }
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
            module_scope: None,
            classes: None,
        }
    }

    #[test]
    fn test_pattern_detector_creation() {
        let detector = PatternDetector::new();
        assert_eq!(detector.recognizers.len(), 3);
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
}
