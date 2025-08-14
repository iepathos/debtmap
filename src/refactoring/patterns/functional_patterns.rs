use crate::core::{FileMetrics, FunctionMetrics};
use crate::refactoring::{
    DetectedPattern, PatternAssessment, PatternEvidence, PatternMatcher, PatternType,
};

pub struct MapPatternMatcher;

impl PatternMatcher for MapPatternMatcher {
    fn match_pattern(
        &self,
        function: &FunctionMetrics,
        _file: &FileMetrics,
    ) -> Option<DetectedPattern> {
        if function.name.contains("map") || function.name.contains("transform") {
            Some(DetectedPattern {
                pattern_type: PatternType::FunctionalComposition,
                confidence: 0.9,
                evidence: PatternEvidence {
                    code_snippets: vec![],
                    line_numbers: vec![function.line as u32],
                    confidence_factors: vec![
                        "Function uses map pattern for transformation".to_string()
                    ],
                },
                assessment: PatternAssessment::GoodExample {
                    strengths: vec![
                        "Declarative transformation pattern".to_string(),
                        "Immutable data processing".to_string(),
                        "Easily testable and composable".to_string(),
                    ],
                    why_good: "Map patterns are functional programming best practices".to_string(),
                },
            })
        } else {
            None
        }
    }
}

pub struct FilterPatternMatcher;

impl PatternMatcher for FilterPatternMatcher {
    fn match_pattern(
        &self,
        function: &FunctionMetrics,
        _file: &FileMetrics,
    ) -> Option<DetectedPattern> {
        if function.name.contains("filter") || function.name.contains("select") {
            Some(DetectedPattern {
                pattern_type: PatternType::FunctionalComposition,
                confidence: 0.9,
                evidence: PatternEvidence {
                    code_snippets: vec![],
                    line_numbers: vec![function.line as u32],
                    confidence_factors: vec![
                        "Function uses filter pattern for selection".to_string()
                    ],
                },
                assessment: PatternAssessment::GoodExample {
                    strengths: vec![
                        "Declarative filtering logic".to_string(),
                        "Pure predicate function".to_string(),
                        "Composable with other functional patterns".to_string(),
                    ],
                    why_good: "Filter patterns promote clean, testable code".to_string(),
                },
            })
        } else {
            None
        }
    }
}

pub struct FoldPatternMatcher;

impl PatternMatcher for FoldPatternMatcher {
    fn match_pattern(
        &self,
        function: &FunctionMetrics,
        _file: &FileMetrics,
    ) -> Option<DetectedPattern> {
        if function.name.contains("fold")
            || function.name.contains("reduce")
            || function.name.contains("aggregate")
        {
            Some(DetectedPattern {
                pattern_type: PatternType::FunctionalComposition,
                confidence: 0.9,
                evidence: PatternEvidence {
                    code_snippets: vec![],
                    line_numbers: vec![function.line as u32],
                    confidence_factors: vec![
                        "Function uses fold/reduce pattern for aggregation".to_string()
                    ],
                },
                assessment: PatternAssessment::GoodExample {
                    strengths: vec![
                        "Functional aggregation pattern".to_string(),
                        "No mutable accumulator needed".to_string(),
                        "Clear data flow and transformation".to_string(),
                    ],
                    why_good: "Fold patterns eliminate mutable state in aggregations".to_string(),
                },
            })
        } else {
            None
        }
    }
}

pub struct PipelinePatternMatcher;

impl PatternMatcher for PipelinePatternMatcher {
    fn match_pattern(
        &self,
        function: &FunctionMetrics,
        _file: &FileMetrics,
    ) -> Option<DetectedPattern> {
        if function.name.contains("pipeline")
            || function.name.contains("chain")
            || function.name.contains("compose")
        {
            Some(DetectedPattern {
                pattern_type: PatternType::FunctionalComposition,
                confidence: 0.85,
                evidence: PatternEvidence {
                    code_snippets: vec![],
                    line_numbers: vec![function.line as u32],
                    confidence_factors: vec![
                        "Function uses pipeline/composition pattern".to_string()
                    ],
                },
                assessment: PatternAssessment::GoodExample {
                    strengths: vec![
                        "Clear data transformation pipeline".to_string(),
                        "Composable function chain".to_string(),
                        "Easy to understand data flow".to_string(),
                    ],
                    why_good: "Pipeline patterns make complex transformations readable".to_string(),
                },
            })
        } else {
            None
        }
    }
}
