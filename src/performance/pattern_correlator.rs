use crate::performance::context::{FunctionIntent, ModuleType, PatternContext};
use crate::performance::PerformanceAntiPattern;

pub struct PatternCorrelator {
    correlation_rules: Vec<CorrelationRule>,
}

#[derive(Debug, Clone)]
pub struct CorrelationRule {
    pub patterns: Vec<PatternType>,
    pub context_indicators: Vec<ContextIndicator>,
    pub confidence_adjustment: f64,
    pub severity_adjustment: f64,
    pub explanation: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PatternType {
    InefficientIO,
    NestedLoop,
    UnboundedAllocation,
    SynchronousBlocking,
    InefficientAlgorithm,
    ResourceLeak,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ContextIndicator {
    TestModule,
    SetupFunction,
    TeardownFunction,
    IOWrapper,
    ErrorHandling,
    BatchProcessing,
}

#[derive(Debug, Clone)]
pub struct PatternCorrelation {
    pub pattern_group: Vec<PerformanceAntiPattern>,
    pub correlation_type: CorrelationType,
    pub confidence_boost: f64,
    pub severity_reduction: f64,
    pub explanation: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CorrelationType {
    TestFixture,
    BatchProcessing,
    ErrorHandling,
    DataMigration,
    Initialization,
}

impl Default for PatternCorrelator {
    fn default() -> Self {
        Self::new()
    }
}

impl PatternCorrelator {
    pub fn new() -> Self {
        Self {
            correlation_rules: Self::default_rules(),
        }
    }

    fn default_rules() -> Vec<CorrelationRule> {
        vec![
            // Test fixture rule
            CorrelationRule {
                patterns: vec![PatternType::InefficientIO],
                context_indicators: vec![
                    ContextIndicator::TestModule,
                    ContextIndicator::SetupFunction,
                ],
                confidence_adjustment: 0.9,
                severity_adjustment: 0.2,
                explanation: "I/O operations in test setup/teardown context".to_string(),
            },
            // Batch processing rule
            CorrelationRule {
                patterns: vec![PatternType::InefficientIO, PatternType::NestedLoop],
                context_indicators: vec![ContextIndicator::BatchProcessing],
                confidence_adjustment: 0.8,
                severity_adjustment: 0.7,
                explanation: "I/O in batch processing context - expected pattern".to_string(),
            },
            // Error handling rule
            CorrelationRule {
                patterns: vec![PatternType::InefficientIO],
                context_indicators: vec![ContextIndicator::ErrorHandling],
                confidence_adjustment: 0.7,
                severity_adjustment: 0.6,
                explanation: "I/O in error handling context - often acceptable".to_string(),
            },
        ]
    }

    pub fn correlate_patterns(
        &self,
        patterns: &[PerformanceAntiPattern],
        contexts: &[PatternContext],
    ) -> Vec<PatternCorrelation> {
        let mut correlations = Vec::new();

        // Look for test fixture patterns
        if self.has_test_fixture_pattern(patterns, contexts) {
            correlations.push(PatternCorrelation {
                pattern_group: patterns.to_vec(),
                correlation_type: CorrelationType::TestFixture,
                confidence_boost: 0.9,
                severity_reduction: 0.2,
                explanation: "I/O operations in test setup/teardown context".to_string(),
            });
        }

        // Look for batch processing patterns
        if self.has_batch_processing_pattern(patterns, contexts) {
            correlations.push(PatternCorrelation {
                pattern_group: patterns.to_vec(),
                correlation_type: CorrelationType::BatchProcessing,
                confidence_boost: 0.8,
                severity_reduction: 0.7,
                explanation: "I/O in batch processing context - expected pattern".to_string(),
            });
        }

        // Look for error handling patterns
        if self.has_error_handling_pattern(patterns, contexts) {
            correlations.push(PatternCorrelation {
                pattern_group: patterns.to_vec(),
                correlation_type: CorrelationType::ErrorHandling,
                confidence_boost: 0.7,
                severity_reduction: 0.6,
                explanation: "I/O in error handling context - often acceptable".to_string(),
            });
        }

        // Look for initialization patterns
        if self.has_initialization_pattern(patterns, contexts) {
            correlations.push(PatternCorrelation {
                pattern_group: patterns.to_vec(),
                correlation_type: CorrelationType::Initialization,
                confidence_boost: 0.85,
                severity_reduction: 0.3,
                explanation: "I/O in initialization context - typically one-time operation"
                    .to_string(),
            });
        }

        correlations
    }

    fn has_test_fixture_pattern(
        &self,
        patterns: &[PerformanceAntiPattern],
        contexts: &[PatternContext],
    ) -> bool {
        // Check for I/O operations in test context with setup/teardown intent
        patterns
            .iter()
            .any(|p| matches!(p, PerformanceAntiPattern::InefficientIO { .. }))
            && contexts.iter().any(|c| {
                matches!(c.module_type, ModuleType::Test)
                    && matches!(
                        c.function_intent,
                        FunctionIntent::Setup | FunctionIntent::Teardown
                    )
            })
    }

    fn has_batch_processing_pattern(
        &self,
        patterns: &[PerformanceAntiPattern],
        contexts: &[PatternContext],
    ) -> bool {
        // Look for nested loops with I/O that might indicate batch processing
        let has_nested_loops = patterns
            .iter()
            .any(|p| matches!(p, PerformanceAntiPattern::NestedLoopComplexity { .. }));
        let has_io = patterns
            .iter()
            .any(|p| matches!(p, PerformanceAntiPattern::InefficientIO { .. }));

        has_nested_loops
            && has_io
            && contexts
                .iter()
                .any(|c| matches!(c.function_intent, FunctionIntent::DataTransformation))
    }

    fn has_error_handling_pattern(
        &self,
        patterns: &[PerformanceAntiPattern],
        contexts: &[PatternContext],
    ) -> bool {
        patterns
            .iter()
            .any(|p| matches!(p, PerformanceAntiPattern::InefficientIO { .. }))
            && contexts
                .iter()
                .any(|c| matches!(c.function_intent, FunctionIntent::ErrorHandling))
    }

    fn has_initialization_pattern(
        &self,
        _patterns: &[PerformanceAntiPattern],
        contexts: &[PatternContext],
    ) -> bool {
        contexts.iter().any(|c| {
            matches!(
                c.function_intent,
                FunctionIntent::Setup | FunctionIntent::Configuration
            ) && matches!(
                c.module_type,
                ModuleType::Production | ModuleType::Infrastructure
            )
        })
    }

    pub fn apply_correlation(
        &self,
        issue: &mut super::smart_detector::SmartPerformanceIssue,
        correlation: &PatternCorrelation,
    ) {
        // Apply confidence boost
        issue.confidence = (issue.confidence * correlation.confidence_boost).min(1.0);

        // Apply severity reduction
        let current_severity_value = Self::priority_to_score(&issue.adjusted_severity);
        let adjusted_value = current_severity_value * correlation.severity_reduction;
        issue.adjusted_severity = self.value_to_priority(adjusted_value);

        // Update reasoning
        if !issue.reasoning.is_empty() {
            issue.reasoning.push_str("\n");
        }
        issue
            .reasoning
            .push_str(&format!("Pattern correlation: {}", correlation.explanation));
    }

    fn priority_to_score(priority: &crate::debt::Priority) -> f64 {
        use crate::debt::Priority;
        match priority {
            Priority::Critical => 4.0,
            Priority::High => 3.0,
            Priority::Medium => 2.0,
            Priority::Low => 1.0,
        }
    }

    fn value_to_priority(&self, value: f64) -> crate::debt::Priority {
        use crate::debt::Priority;
        
        if value >= Self::priority_to_score(&Priority::Critical) * 0.8 {
            Priority::Critical
        } else if value >= Self::priority_to_score(&Priority::High) * 0.8 {
            Priority::High
        } else if value >= Self::priority_to_score(&Priority::Medium) * 0.8 {
            Priority::Medium
        } else {
            Priority::Low
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::performance::context::{BusinessCriticality, PerformanceSensitivity};

    #[test]
    fn test_test_fixture_correlation() {
        let correlator = PatternCorrelator::new();

        let patterns = vec![PerformanceAntiPattern::InefficientIO {
            io_pattern: crate::performance::IOPattern::SyncInLoop,
            batching_opportunity: true,
            async_opportunity: true,
            location: SourceLocation::default(),
        }];

        let contexts = vec![PatternContext {
            module_type: ModuleType::Test,
            function_intent: FunctionIntent::Setup,
            architectural_pattern: None,
            business_criticality: BusinessCriticality::Development,
            performance_sensitivity: PerformanceSensitivity::Irrelevant,
            confidence: 0.8,
        }];

        let correlations = correlator.correlate_patterns(&patterns, &contexts);

        assert_eq!(correlations.len(), 1);
        assert_eq!(
            correlations[0].correlation_type,
            CorrelationType::TestFixture
        );
        assert_eq!(correlations[0].severity_reduction, 0.2);
    }

    #[test]
    fn test_batch_processing_correlation() {
        let correlator = PatternCorrelator::new();

        let patterns = vec![
            PerformanceAntiPattern::InefficientIO {
                io_pattern: crate::performance::IOPattern::SyncInLoop,
                batching_opportunity: true,
                async_opportunity: true,
                location: SourceLocation::default(),
            },
            PerformanceAntiPattern::NestedLoopComplexity {
                depth: 3,
                complexity: 10,
                location: SourceLocation::default(),
            },
        ];

        let contexts = vec![PatternContext {
            module_type: ModuleType::Production,
            function_intent: FunctionIntent::DataTransformation,
            architectural_pattern: None,
            business_criticality: BusinessCriticality::Important,
            performance_sensitivity: PerformanceSensitivity::Medium,
            confidence: 0.7,
        }];

        let correlations = correlator.correlate_patterns(&patterns, &contexts);

        assert!(correlations
            .iter()
            .any(|c| c.correlation_type == CorrelationType::BatchProcessing));
    }

    #[test]
    fn test_error_handling_correlation() {
        let correlator = PatternCorrelator::new();

        let patterns = vec![PerformanceAntiPattern::InefficientIO {
            io_pattern: crate::performance::IOPattern::SingleSync,
            batching_opportunity: false,
            async_opportunity: true,
            location: SourceLocation::default(),
        }];

        let contexts = vec![PatternContext {
            module_type: ModuleType::Production,
            function_intent: FunctionIntent::ErrorHandling,
            architectural_pattern: None,
            business_criticality: BusinessCriticality::Important,
            performance_sensitivity: PerformanceSensitivity::Low,
            confidence: 0.6,
        }];

        let correlations = correlator.correlate_patterns(&patterns, &contexts);

        assert!(correlations
            .iter()
            .any(|c| c.correlation_type == CorrelationType::ErrorHandling));
    }
}
