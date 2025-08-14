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

impl PatternMatcher for PureIoMatcher {
    fn match_pattern(
        &self,
        function: &FunctionMetrics,
        _file: &FileMetrics,
    ) -> Option<DetectedPattern> {
        let has_io = function.name.contains("read")
            || function.name.contains("write")
            || function.name.contains("save")
            || function.name.contains("load");

        let is_simple = function.cyclomatic <= 2;

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
