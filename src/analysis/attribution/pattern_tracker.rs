use super::{
    AttributedComplexity, CodeLocation, ComplexityComponent, ComplexitySourceType,
    RecognizedPattern,
};
use crate::core::FunctionMetrics;
use std::collections::HashMap;

/// Pattern analysis and tracking
pub struct PatternTracker {
    pattern_detectors: Vec<Box<dyn PatternDetector>>,
}

impl Default for PatternTracker {
    fn default() -> Self {
        Self {
            pattern_detectors: vec![
                Box::new(ErrorHandlingDetector::new()),
                Box::new(ValidationDetector::new()),
                Box::new(DataTransformationDetector::new()),
                Box::new(StateManagementDetector::new()),
                Box::new(IteratorDetector::new()),
            ],
        }
    }
}

impl PatternTracker {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn analyze_patterns(&self, functions: &[FunctionMetrics]) -> AttributedComplexity {
        let mut total = 0u32;
        let mut breakdown = Vec::new();
        let mut pattern_counts = HashMap::new();

        for func in functions {
            for detector in &self.pattern_detectors {
                if let Some(pattern_info) = detector.detect(func) {
                    *pattern_counts
                        .entry(pattern_info.pattern_type.clone())
                        .or_insert(0) += 1;

                    // Patterns typically reduce perceived complexity
                    let adjustment =
                        (func.cyclomatic as f32 * pattern_info.adjustment_factor) as u32;
                    let reduction = func.cyclomatic.saturating_sub(adjustment);

                    if reduction > 0 {
                        breakdown.push(ComplexityComponent {
                            source_type: ComplexitySourceType::PatternRecognition {
                                pattern_type: pattern_info.pattern_type,
                                adjustment_factor: pattern_info.adjustment_factor,
                            },
                            contribution: reduction,
                            location: CodeLocation {
                                file: func.file.to_string_lossy().to_string(),
                                line: func.line as u32,
                                column: 0,
                                span: Some((func.line as u32, (func.line + func.length) as u32)),
                            },
                            description: format!("{} pattern in {}", pattern_info.name, func.name),
                            suggestions: pattern_info.optimization_suggestions,
                        });

                        total += reduction;
                    }
                }
            }
        }

        // Calculate confidence based on pattern consistency
        let confidence = if !pattern_counts.is_empty() {
            let max_count = *pattern_counts.values().max().unwrap_or(&0);
            let pattern_variety = pattern_counts.len();

            // Higher confidence when we see consistent patterns
            let consistency_score = (max_count as f32 / functions.len().max(1) as f32).min(1.0);
            let variety_score = (pattern_variety as f32 / 5.0).min(1.0);

            (consistency_score * 0.6 + variety_score * 0.4).min(0.9)
        } else {
            0.3 // Low confidence when no patterns detected
        };

        AttributedComplexity {
            total,
            breakdown,
            confidence,
        }
    }
}

/// Trait for pattern detection
trait PatternDetector: Send + Sync {
    fn detect(&self, func: &FunctionMetrics) -> Option<PatternInfo>;
}

/// Information about a detected pattern
struct PatternInfo {
    name: String,
    pattern_type: RecognizedPattern,
    adjustment_factor: f32,
    optimization_suggestions: Vec<String>,
}

/// Detector for error handling patterns
struct ErrorHandlingDetector;

impl ErrorHandlingDetector {
    fn new() -> Self {
        Self
    }
}

impl PatternDetector for ErrorHandlingDetector {
    fn detect(&self, func: &FunctionMetrics) -> Option<PatternInfo> {
        // Simple heuristic: functions with "error", "handle", "catch" in name
        let name_lower = func.name.to_lowercase();
        if name_lower.contains("error")
            || name_lower.contains("handle")
            || name_lower.contains("catch")
        {
            Some(PatternInfo {
                name: "Error Handling".to_string(),
                pattern_type: RecognizedPattern::ErrorHandling,
                adjustment_factor: 0.8, // Error handling is expected complexity
                optimization_suggestions: vec![
                    "Consider using Result type consistently".to_string(),
                    "Extract error transformation logic".to_string(),
                ],
            })
        } else {
            None
        }
    }
}

/// Detector for validation patterns
struct ValidationDetector;

impl ValidationDetector {
    fn new() -> Self {
        Self
    }
}

impl PatternDetector for ValidationDetector {
    fn detect(&self, func: &FunctionMetrics) -> Option<PatternInfo> {
        let name_lower = func.name.to_lowercase();
        if name_lower.contains("valid")
            || name_lower.contains("check")
            || name_lower.contains("verify")
        {
            Some(PatternInfo {
                name: "Validation".to_string(),
                pattern_type: RecognizedPattern::Validation,
                adjustment_factor: 0.85,
                optimization_suggestions: vec![
                    "Consider using validation libraries".to_string(),
                    "Extract validation rules into constants".to_string(),
                ],
            })
        } else {
            None
        }
    }
}

/// Detector for data transformation patterns
struct DataTransformationDetector;

impl DataTransformationDetector {
    fn new() -> Self {
        Self
    }
}

impl PatternDetector for DataTransformationDetector {
    fn detect(&self, func: &FunctionMetrics) -> Option<PatternInfo> {
        let name_lower = func.name.to_lowercase();
        if name_lower.contains("transform")
            || name_lower.contains("convert")
            || name_lower.contains("map")
            || name_lower.contains("parse")
        {
            Some(PatternInfo {
                name: "Data Transformation".to_string(),
                pattern_type: RecognizedPattern::DataTransformation,
                adjustment_factor: 0.9,
                optimization_suggestions: vec![
                    "Consider using functional transformations".to_string(),
                    "Extract transformation logic into pure functions".to_string(),
                ],
            })
        } else {
            None
        }
    }
}

/// Detector for state management patterns
struct StateManagementDetector;

impl StateManagementDetector {
    fn new() -> Self {
        Self
    }
}

impl PatternDetector for StateManagementDetector {
    fn detect(&self, func: &FunctionMetrics) -> Option<PatternInfo> {
        let name_lower = func.name.to_lowercase();
        if name_lower.contains("state")
            || name_lower.contains("update")
            || name_lower.contains("sync")
        {
            Some(PatternInfo {
                name: "State Management".to_string(),
                pattern_type: RecognizedPattern::StateManagement,
                adjustment_factor: 0.75, // State management can be complex
                optimization_suggestions: vec![
                    "Consider immutable state updates".to_string(),
                    "Use state machines for complex state logic".to_string(),
                ],
            })
        } else {
            None
        }
    }
}

/// Detector for iterator patterns
struct IteratorDetector;

impl IteratorDetector {
    fn new() -> Self {
        Self
    }
}

impl PatternDetector for IteratorDetector {
    fn detect(&self, func: &FunctionMetrics) -> Option<PatternInfo> {
        let name_lower = func.name.to_lowercase();
        if name_lower.contains("iter")
            || name_lower.contains("next")
            || name_lower.contains("collect")
            || name_lower.contains("fold")
        {
            Some(PatternInfo {
                name: "Iterator".to_string(),
                pattern_type: RecognizedPattern::Iterator,
                adjustment_factor: 0.95,
                optimization_suggestions: vec![
                    "Consider using iterator combinators".to_string(),
                    "Avoid unnecessary intermediate collections".to_string(),
                ],
            })
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_pattern_tracker_new() {
        let tracker = PatternTracker::new();
        assert!(!tracker.pattern_detectors.is_empty());
    }

    #[test]
    fn test_error_handling_detection() {
        let detector = ErrorHandlingDetector::new();

        let func = FunctionMetrics {
            name: "handle_error".to_string(),
            file: PathBuf::from("test.rs"),
            line: 10,
            cyclomatic: 5,
            cognitive: 3,
            nesting: 1,
            length: 20,
            is_test: false,
            visibility: None,
            is_trait_method: false,
            in_test_module: false,
            entropy_score: None,
            is_pure: None,
            purity_confidence: None,
            detected_patterns: None,
        };

        let pattern = detector.detect(&func);
        assert!(pattern.is_some());

        let info = pattern.unwrap();
        assert_eq!(info.pattern_type, RecognizedPattern::ErrorHandling);
        assert_eq!(info.adjustment_factor, 0.8);
    }

    #[test]
    fn test_validation_detection() {
        let detector = ValidationDetector::new();

        let func = FunctionMetrics {
            name: "validate_input".to_string(),
            file: PathBuf::from("test.rs"),
            line: 20,
            cyclomatic: 8,
            cognitive: 5,
            nesting: 2,
            length: 30,
            is_test: false,
            visibility: None,
            is_trait_method: false,
            in_test_module: false,
            entropy_score: None,
            is_pure: None,
            purity_confidence: None,
            detected_patterns: None,
        };

        let pattern = detector.detect(&func);
        assert!(pattern.is_some());

        let info = pattern.unwrap();
        assert_eq!(info.pattern_type, RecognizedPattern::Validation);
    }

    #[test]
    fn test_no_pattern_detection() {
        let detector = ErrorHandlingDetector::new();

        let func = FunctionMetrics {
            name: "calculate_sum".to_string(),
            file: PathBuf::from("test.rs"),
            line: 30,
            cyclomatic: 2,
            cognitive: 1,
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
        };

        let pattern = detector.detect(&func);
        assert!(pattern.is_none());
    }

    #[test]
    fn test_analyze_patterns_empty() {
        let tracker = PatternTracker::new();
        let functions = vec![];

        let result = tracker.analyze_patterns(&functions);
        assert_eq!(result.total, 0);
        assert_eq!(result.confidence, 0.3);
    }

    #[test]
    fn test_analyze_patterns_with_functions() {
        let tracker = PatternTracker::new();
        let functions = vec![FunctionMetrics {
            name: "handle_request_error".to_string(),
            file: PathBuf::from("test.rs"),
            line: 10,
            cyclomatic: 10,
            cognitive: 8,
            nesting: 2,
            length: 40,
            is_test: false,
            visibility: None,
            is_trait_method: false,
            in_test_module: false,
            entropy_score: None,
            is_pure: None,
            purity_confidence: None,
            detected_patterns: None,
        }];

        let result = tracker.analyze_patterns(&functions);
        assert!(result.total > 0);
        assert!(!result.breakdown.is_empty());
    }
}
