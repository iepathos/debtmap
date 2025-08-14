use crate::core::{FileMetrics, FunctionMetrics};
use crate::refactoring::{
    DetectedPattern, PatternAssessment, PatternEvidence, PatternMatcher, PatternType,
};

pub struct ForLoopMatcher;

impl PatternMatcher for ForLoopMatcher {
    fn match_pattern(
        &self,
        function: &FunctionMetrics,
        _file: &FileMetrics,
    ) -> Option<DetectedPattern> {
        // High cyclomatic complexity often indicates loops
        if function.cyclomatic > 6
            && !function.name.contains("map")
            && !function.name.contains("filter")
            && !function.name.contains("fold")
        {
            Some(DetectedPattern {
                pattern_type: PatternType::ImperativeLoop,
                confidence: 0.7,
                evidence: PatternEvidence {
                    code_snippets: vec![],
                    line_numbers: vec![function.line as u32],
                    confidence_factors: vec![
                        format!(
                            "Cyclomatic complexity {} suggests loops",
                            function.cyclomatic
                        ),
                        "No functional pattern indicators in name".to_string(),
                    ],
                },
                assessment: PatternAssessment::ImprovementOpportunity {
                    current_issues: vec![
                        "Imperative loops are harder to test".to_string(),
                        "Mutable state management is error-prone".to_string(),
                        "Loop logic is not composable".to_string(),
                    ],
                    potential_benefits: vec![
                        "Convert to map for transformations".to_string(),
                        "Use filter for selections".to_string(),
                        "Apply fold for aggregations".to_string(),
                        "Functional patterns are more testable".to_string(),
                    ],
                    refactoring_suggestions: vec![],
                },
            })
        } else {
            None
        }
    }
}

pub struct WhileLoopMatcher;

impl PatternMatcher for WhileLoopMatcher {
    fn match_pattern(
        &self,
        function: &FunctionMetrics,
        _file: &FileMetrics,
    ) -> Option<DetectedPattern> {
        // Functions with "while" or "until" patterns
        if function.name.contains("while")
            || function.name.contains("until")
            || function.name.contains("loop")
        {
            Some(DetectedPattern {
                pattern_type: PatternType::ImperativeLoop,
                confidence: 0.8,
                evidence: PatternEvidence {
                    code_snippets: vec![],
                    line_numbers: vec![function.line as u32],
                    confidence_factors: vec!["Function name indicates while/until loop".to_string()],
                },
                assessment: PatternAssessment::ImprovementOpportunity {
                    current_issues: vec![
                        "While loops often have complex termination conditions".to_string(),
                        "Potential for infinite loops".to_string(),
                        "State management is complex".to_string(),
                    ],
                    potential_benefits: vec![
                        "Consider recursion with clear base case".to_string(),
                        "Use iterators with take_while".to_string(),
                        "Apply functional patterns for clearer logic".to_string(),
                    ],
                    refactoring_suggestions: vec![],
                },
            })
        } else {
            None
        }
    }
}

pub struct NestedLoopMatcher;

impl PatternMatcher for NestedLoopMatcher {
    fn match_pattern(
        &self,
        function: &FunctionMetrics,
        _file: &FileMetrics,
    ) -> Option<DetectedPattern> {
        // Very high complexity suggests nested loops
        if function.cyclomatic > 10 {
            Some(DetectedPattern {
                pattern_type: PatternType::ImperativeLoop,
                confidence: 0.75,
                evidence: PatternEvidence {
                    code_snippets: vec![],
                    line_numbers: vec![function.line as u32],
                    confidence_factors: vec![format!(
                        "Very high complexity {} suggests nested loops",
                        function.cyclomatic
                    )],
                },
                assessment: PatternAssessment::ImprovementOpportunity {
                    current_issues: vec![
                        "Nested loops have O(n²) or worse complexity".to_string(),
                        "Very difficult to test all paths".to_string(),
                        "Hard to understand and maintain".to_string(),
                    ],
                    potential_benefits: vec![
                        "Flatten with flat_map operations".to_string(),
                        "Extract inner loop to separate function".to_string(),
                        "Consider different data structures".to_string(),
                        "Use functional composition".to_string(),
                    ],
                    refactoring_suggestions: vec![],
                },
            })
        } else {
            None
        }
    }
}
