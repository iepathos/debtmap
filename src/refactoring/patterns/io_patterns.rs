use crate::core::{FileMetrics, FunctionMetrics};
use crate::refactoring::{
    ConcernMixingPattern, DetectedPattern, PatternAssessment, PatternEvidence, PatternMatcher,
    PatternType, SeparationDifficulty, Urgency,
};

pub struct IoWithLogicMatcher;

impl IoWithLogicMatcher {
    /// Pure function to determine if a function name indicates I/O operations
    fn indicates_io_operation(name: &str) -> bool {
        const IO_INDICATORS: &[&str] = &["read", "write", "save", "load", "fetch", "send"];
        IO_INDICATORS
            .iter()
            .any(|indicator| name.contains(indicator))
    }

    /// Pure function to determine separation difficulty based on complexity
    fn determine_separation_difficulty(cyclomatic: u32) -> SeparationDifficulty {
        if cyclomatic > 10 {
            SeparationDifficulty::High
        } else {
            SeparationDifficulty::Medium
        }
    }

    /// Pure function to determine urgency based on complexity
    fn determine_urgency(cyclomatic: u32) -> Urgency {
        if cyclomatic > 10 {
            Urgency::High
        } else {
            Urgency::Medium
        }
    }
}

impl PatternMatcher for IoWithLogicMatcher {
    fn match_pattern(
        &self,
        function: &FunctionMetrics,
        _file: &FileMetrics,
    ) -> Option<DetectedPattern> {
        let has_io = Self::indicates_io_operation(&function.name);
        let has_complex_logic = function.cyclomatic > 5;

        if has_io && has_complex_logic {
            Some(DetectedPattern {
                pattern_type: PatternType::MixedConcerns(ConcernMixingPattern {
                    concerns: vec!["I/O Operations".to_string(), "Business Logic".to_string()],
                    separation_difficulty: Self::determine_separation_difficulty(
                        function.cyclomatic,
                    ),
                }),
                confidence: 0.85,
                evidence: PatternEvidence {
                    code_snippets: vec![],
                    line_numbers: vec![function.line as u32],
                    confidence_factors: vec![
                        "Function name indicates I/O operations".to_string(),
                        format!(
                            "High complexity {} suggests business logic",
                            function.cyclomatic
                        ),
                    ],
                },
                assessment: PatternAssessment::AntiPattern {
                    problems: vec![
                        "Mixing I/O with business logic".to_string(),
                        "Cannot test logic without mocking I/O".to_string(),
                        "Violates functional core / imperative shell".to_string(),
                    ],
                    recommended_patterns: vec![PatternType::FunctionalComposition],
                    urgency: Self::determine_urgency(function.cyclomatic),
                },
            })
        } else {
            None
        }
    }
}

pub struct PureIoMatcher;

impl PureIoMatcher {
    /// Classifies whether a function name indicates I/O operations
    fn is_io_function(name: &str) -> bool {
        const IO_INDICATORS: &[&str] = &["read", "write", "save", "load"];
        IO_INDICATORS
            .iter()
            .any(|indicator| name.contains(indicator))
    }

    /// Determines if complexity is simple enough for pure I/O pattern
    fn is_simple_complexity(cyclomatic: u32) -> bool {
        cyclomatic <= 2
    }
}

impl PatternMatcher for PureIoMatcher {
    fn match_pattern(
        &self,
        function: &FunctionMetrics,
        _file: &FileMetrics,
    ) -> Option<DetectedPattern> {
        let has_io = Self::is_io_function(&function.name);
        let is_simple = Self::is_simple_complexity(function.cyclomatic);

        if has_io && is_simple {
            Some(DetectedPattern {
                pattern_type: PatternType::IOOrchestration(
                    crate::refactoring::OrchestrationPattern {
                        pattern_type: "PureIO".to_string(),
                        description: "Simple I/O without business logic".to_string(),
                    },
                ),
                confidence: 0.9,
                evidence: PatternEvidence {
                    code_snippets: vec![],
                    line_numbers: vec![function.line as u32],
                    confidence_factors: vec![
                        "I/O operation with minimal complexity".to_string(),
                        "Follows imperative shell pattern".to_string(),
                    ],
                },
                assessment: PatternAssessment::GoodExample {
                    strengths: vec![
                        "Clean separation of I/O from logic".to_string(),
                        "Simple, focused responsibility".to_string(),
                        "Easy to mock for testing".to_string(),
                    ],
                    why_good: "I/O functions should be simple wrappers at boundaries".to_string(),
                },
            })
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{ComplexityMetrics, FileMetrics, FunctionMetrics, Language};
    use crate::refactoring::patterns::PatternMatcher;
    use std::path::PathBuf;

    fn create_test_function(name: &str, cyclomatic: u32) -> FunctionMetrics {
        FunctionMetrics {
            name: name.to_string(),
            file: PathBuf::from("test.rs"),
            line: 1,
            cyclomatic,
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
        }
    }

    fn create_test_file() -> FileMetrics {
        FileMetrics {
            path: PathBuf::from("test.rs"),
            language: Language::Rust,
            complexity: ComplexityMetrics::default(),
            debt_items: vec![],
            dependencies: vec![],
            duplications: vec![],
        }
    }

    #[test]
    fn test_is_io_function_detects_read() {
        assert!(PureIoMatcher::is_io_function("read_file"));
        assert!(PureIoMatcher::is_io_function("file_reader"));
        assert!(PureIoMatcher::is_io_function("async_read"));
    }

    #[test]
    fn test_is_io_function_detects_write() {
        assert!(PureIoMatcher::is_io_function("write_data"));
        assert!(PureIoMatcher::is_io_function("file_writer"));
        assert!(PureIoMatcher::is_io_function("buffer_write"));
    }

    #[test]
    fn test_is_io_function_detects_save_and_load() {
        assert!(PureIoMatcher::is_io_function("save_config"));
        assert!(PureIoMatcher::is_io_function("load_settings"));
        assert!(PureIoMatcher::is_io_function("autosave"));
        assert!(PureIoMatcher::is_io_function("preload_cache"));
    }

    #[test]
    fn test_is_io_function_rejects_non_io() {
        assert!(!PureIoMatcher::is_io_function("calculate_sum"));
        assert!(!PureIoMatcher::is_io_function("process_data"));
        assert!(!PureIoMatcher::is_io_function("validate_input"));
        assert!(!PureIoMatcher::is_io_function("transform_result"));
    }

    #[test]
    fn test_is_simple_complexity() {
        assert!(PureIoMatcher::is_simple_complexity(0));
        assert!(PureIoMatcher::is_simple_complexity(1));
        assert!(PureIoMatcher::is_simple_complexity(2));
        assert!(!PureIoMatcher::is_simple_complexity(3));
        assert!(!PureIoMatcher::is_simple_complexity(10));
    }

    #[test]
    fn test_match_pattern_detects_pure_io() {
        let matcher = PureIoMatcher;
        let function = create_test_function("read_file", 1);
        let file = create_test_file();

        let result = matcher.match_pattern(&function, &file);
        assert!(result.is_some());

        let pattern = result.unwrap();
        assert_eq!(pattern.confidence, 0.9);

        if let crate::refactoring::PatternType::IOOrchestration(orch) = pattern.pattern_type {
            assert_eq!(orch.pattern_type, "PureIO");
        } else {
            panic!("Expected IOOrchestration pattern");
        }
    }

    #[test]
    fn test_match_pattern_rejects_complex_io() {
        let matcher = PureIoMatcher;
        let function = create_test_function("read_and_process_file", 5);
        let file = create_test_file();

        let result = matcher.match_pattern(&function, &file);
        assert!(result.is_none());
    }

    #[test]
    fn test_match_pattern_rejects_non_io_simple() {
        let matcher = PureIoMatcher;
        let function = create_test_function("calculate_sum", 1);
        let file = create_test_file();

        let result = matcher.match_pattern(&function, &file);
        assert!(result.is_none());
    }

    #[test]
    fn test_match_pattern_boundary_complexity() {
        let matcher = PureIoMatcher;
        let file = create_test_file();

        // Test at boundary (complexity = 2)
        let function = create_test_function("write_buffer", 2);
        let result = matcher.match_pattern(&function, &file);
        assert!(result.is_some());

        // Test just over boundary (complexity = 3)
        let function = create_test_function("write_buffer", 3);
        let result = matcher.match_pattern(&function, &file);
        assert!(result.is_none());
    }
}
