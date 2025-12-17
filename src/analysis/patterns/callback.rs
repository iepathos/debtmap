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
    pub(crate) fn has_callback_decorator(
        &self,
        function: &FunctionMetrics,
        file_metrics: &FileMetrics,
    ) -> bool {
        // Check method decorators from AST classes if available
        if let Some(classes) = &file_metrics.classes {
            for class in classes {
                for method in &class.methods {
                    // Match method by name and approximate line number
                    if method.name == function.name || function.name.ends_with(&method.name) {
                        // Check for callback-style decorators
                        for decorator in &method.decorators {
                            let dec_lower = decorator.to_lowercase();
                            if dec_lower.contains("route")
                                || dec_lower.contains("handler")
                                || dec_lower.contains("callback")
                                || dec_lower.contains("listener")
                                || dec_lower.contains("event")
                                || dec_lower.contains("on_")
                            {
                                return true;
                            }
                        }
                    }
                }
            }
        }

        // Fallback to naming conventions as a heuristic
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
            .filter(|function| self.has_callback_decorator(function, file_metrics))
            .map(|function| {
                // Check if detected via decorator or naming convention
                let has_decorator = file_metrics.classes.as_ref().is_some_and(|classes| {
                    classes.iter().any(|class| {
                        class.methods.iter().any(|method| {
                            (method.name == function.name || function.name.ends_with(&method.name))
                                && !method.decorators.is_empty()
                        })
                    })
                });

                let (confidence, reasoning) = if has_decorator {
                    (
                        0.9,
                        format!(
                            "Callback handler {} (decorator-based detection)",
                            function.name
                        ),
                    )
                } else {
                    (
                        0.6,
                        format!("Callback handler {} (name-based detection)", function.name),
                    )
                };

                PatternInstance {
                    pattern_type: PatternType::Callback,
                    confidence,
                    base_class: None,
                    implementations: vec![Implementation {
                        file: file_metrics.path.clone(),
                        class_name: None,
                        function_name: function.name.clone(),
                        line: function.line,
                    }],
                    usage_sites: vec![],
                    reasoning,
                }
            })
            .collect()
    }

    fn is_function_used_by_pattern(
        &self,
        function: &FunctionMetrics,
        file_metrics: &FileMetrics,
    ) -> Option<PatternInstance> {
        if self.has_callback_decorator(function, file_metrics) {
            // Check if detected via decorator or naming convention
            let has_decorator = file_metrics.classes.as_ref().is_some_and(|classes| {
                classes.iter().any(|class| {
                    class.methods.iter().any(|method| {
                        (method.name == function.name || function.name.ends_with(&method.name))
                            && !method.decorators.is_empty()
                    })
                })
            });

            let confidence = if has_decorator { 0.9 } else { 0.6 };

            Some(PatternInstance {
                pattern_type: PatternType::Callback,
                confidence,
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
            total_lines: 0,
            module_scope: None,
            classes: None,
        }
    }

    #[test]
    fn test_has_callback_decorator_on_prefix() {
        let function = create_test_function("on_click", 10);
        let file_metrics = create_test_file_metrics_with_functions(vec![]);
        let recognizer = CallbackPatternRecognizer::new();
        assert!(recognizer.has_callback_decorator(&function, &file_metrics));
    }

    #[test]
    fn test_has_callback_decorator_handle_prefix() {
        let function = create_test_function("handle_request", 10);
        let file_metrics = create_test_file_metrics_with_functions(vec![]);
        let recognizer = CallbackPatternRecognizer::new();
        assert!(recognizer.has_callback_decorator(&function, &file_metrics));
    }

    #[test]
    fn test_has_callback_decorator_callback_prefix() {
        let function = create_test_function("callback_success", 10);
        let file_metrics = create_test_file_metrics_with_functions(vec![]);
        let recognizer = CallbackPatternRecognizer::new();
        assert!(recognizer.has_callback_decorator(&function, &file_metrics));
    }

    #[test]
    fn test_has_callback_decorator_handler_suffix() {
        let function = create_test_function("request_handler", 10);
        let file_metrics = create_test_file_metrics_with_functions(vec![]);
        let recognizer = CallbackPatternRecognizer::new();
        assert!(recognizer.has_callback_decorator(&function, &file_metrics));
    }

    #[test]
    fn test_has_callback_decorator_listener() {
        let function = create_test_function("event_listener", 10);
        let file_metrics = create_test_file_metrics_with_functions(vec![]);
        let recognizer = CallbackPatternRecognizer::new();
        assert!(recognizer.has_callback_decorator(&function, &file_metrics));
    }

    #[test]
    fn test_not_callback() {
        let function = create_test_function("process_data", 10);
        let file_metrics = create_test_file_metrics_with_functions(vec![]);
        let recognizer = CallbackPatternRecognizer::new();
        assert!(!recognizer.has_callback_decorator(&function, &file_metrics));
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
        assert_eq!(patterns[0].confidence, 0.6); // Name-based detection
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
        assert_eq!(pattern.confidence, 0.6); // Name-based detection
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
