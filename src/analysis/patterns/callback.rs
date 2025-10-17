//! Callback pattern recognition
//!
//! Detects the Callback pattern in code by identifying:
//! - Functions with callback-style decorators (@app.route, @handler, etc.)
//! - Event handler decorators
//! - Callback registration patterns

use super::{Implementation, PatternInstance, PatternRecognizer, PatternType};
use crate::core::{FileMetrics, FunctionMetrics};

pub struct CallbackPatternRecognizer;

impl CallbackPatternRecognizer {
    pub fn new() -> Self {
        Self
    }

    /// Check if a function has decorators that indicate it's a callback
    pub(crate) fn has_callback_decorator(&self, function: &FunctionMetrics) -> bool {
        // In the current implementation, FunctionMetrics doesn't have a decorators field
        // We would need to enhance the parser to extract decorators
        // For now, we use naming conventions as a heuristic
        let name_lower = function.name.to_lowercase();

        name_lower.starts_with("on_")
            || name_lower.starts_with("handle_")
            || name_lower.starts_with("callback_")
            || name_lower.contains("handler")
            || name_lower.contains("listener")
    }
}

impl Default for CallbackPatternRecognizer {
    fn default() -> Self {
        Self::new()
    }
}

impl PatternRecognizer for CallbackPatternRecognizer {
    fn name(&self) -> &str {
        "Callback"
    }

    fn detect(&self, file_metrics: &FileMetrics) -> Vec<PatternInstance> {
        file_metrics
            .complexity
            .functions
            .iter()
            .filter(|function| self.has_callback_decorator(function))
            .map(|function| PatternInstance {
                pattern_type: PatternType::Callback,
                confidence: 0.75,
                base_class: None,
                implementations: vec![Implementation {
                    file: file_metrics.path.clone(),
                    class_name: None,
                    function_name: function.name.clone(),
                    line: function.line,
                }],
                usage_sites: vec![],
                reasoning: format!("Callback handler {} (name-based detection)", function.name),
            })
            .collect()
    }

    fn is_function_used_by_pattern(
        &self,
        function: &FunctionMetrics,
        _file_metrics: &FileMetrics,
    ) -> Option<PatternInstance> {
        if self.has_callback_decorator(function) {
            Some(PatternInstance {
                pattern_type: PatternType::Callback,
                confidence: 0.75,
                base_class: None,
                implementations: vec![Implementation {
                    file: function.file.clone(),
                    class_name: None,
                    function_name: function.name.clone(),
                    line: function.line,
                }],
                usage_sites: vec![],
                reasoning: format!("Callback handler {}", function.name),
            })
        } else {
            None
        }
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
    fn test_has_callback_decorator_on_prefix() {
        let function = create_test_function("on_click", 10);
        let recognizer = CallbackPatternRecognizer::new();
        assert!(recognizer.has_callback_decorator(&function));
    }

    #[test]
    fn test_has_callback_decorator_handle_prefix() {
        let function = create_test_function("handle_request", 10);
        let recognizer = CallbackPatternRecognizer::new();
        assert!(recognizer.has_callback_decorator(&function));
    }

    #[test]
    fn test_has_callback_decorator_callback_prefix() {
        let function = create_test_function("callback_success", 10);
        let recognizer = CallbackPatternRecognizer::new();
        assert!(recognizer.has_callback_decorator(&function));
    }

    #[test]
    fn test_has_callback_decorator_handler_suffix() {
        let function = create_test_function("request_handler", 10);
        let recognizer = CallbackPatternRecognizer::new();
        assert!(recognizer.has_callback_decorator(&function));
    }

    #[test]
    fn test_has_callback_decorator_listener() {
        let function = create_test_function("event_listener", 10);
        let recognizer = CallbackPatternRecognizer::new();
        assert!(recognizer.has_callback_decorator(&function));
    }

    #[test]
    fn test_not_callback() {
        let function = create_test_function("process_data", 10);
        let recognizer = CallbackPatternRecognizer::new();
        assert!(!recognizer.has_callback_decorator(&function));
    }

    #[test]
    fn test_detect_callback_patterns() {
        let file_metrics = create_test_file_metrics_with_functions(vec![
            create_test_function("on_click", 10),
            create_test_function("process_data", 20),
            create_test_function("handle_request", 30),
        ]);

        let recognizer = CallbackPatternRecognizer::new();
        let patterns = recognizer.detect(&file_metrics);

        assert_eq!(patterns.len(), 2);
        assert!(patterns
            .iter()
            .any(|p| p.implementations[0].function_name == "on_click"));
        assert!(patterns
            .iter()
            .any(|p| p.implementations[0].function_name == "handle_request"));
    }

    #[test]
    fn test_callback_pattern_confidence() {
        let file_metrics =
            create_test_file_metrics_with_functions(vec![create_test_function("on_event", 10)]);

        let recognizer = CallbackPatternRecognizer::new();
        let patterns = recognizer.detect(&file_metrics);

        assert_eq!(patterns.len(), 1);
        assert_eq!(patterns[0].confidence, 0.75);
        assert_eq!(patterns[0].pattern_type, PatternType::Callback);
    }

    #[test]
    fn test_callback_recognizer_name() {
        let recognizer = CallbackPatternRecognizer::new();
        assert_eq!(recognizer.name(), "Callback");
    }

    #[test]
    fn test_is_function_used_by_pattern() {
        let function = create_test_function("handle_event", 10);
        let file_metrics = create_test_file_metrics_with_functions(vec![]);

        let recognizer = CallbackPatternRecognizer::new();
        let result = recognizer.is_function_used_by_pattern(&function, &file_metrics);

        assert!(result.is_some());
        let pattern = result.unwrap();
        assert_eq!(pattern.pattern_type, PatternType::Callback);
        assert_eq!(pattern.confidence, 0.75);
    }

    #[test]
    fn test_is_function_not_used_by_pattern() {
        let function = create_test_function("regular_function", 10);
        let file_metrics = create_test_file_metrics_with_functions(vec![]);

        let recognizer = CallbackPatternRecognizer::new();
        let result = recognizer.is_function_used_by_pattern(&function, &file_metrics);

        assert!(result.is_none());
    }
}
