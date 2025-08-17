use super::{
    BusinessCriticality, FunctionIntent, ModuleType, PatternContext, PerformanceSensitivity,
};
use crate::debt::Priority;
use crate::performance::PerformanceAntiPattern;

pub struct SeverityAdjuster {
    context_weights: ContextWeights,
}

#[derive(Debug, Clone)]
pub struct ContextWeights {
    pub module_type_weight: f64,
    pub function_intent_weight: f64,
    pub business_criticality_weight: f64,
    pub performance_sensitivity_weight: f64,
    pub architectural_pattern_weight: f64,
}

impl Default for ContextWeights {
    fn default() -> Self {
        Self {
            module_type_weight: 1.0,
            function_intent_weight: 0.8,
            business_criticality_weight: 1.2,
            performance_sensitivity_weight: 1.5,
            architectural_pattern_weight: 0.6,
        }
    }
}

impl Default for SeverityAdjuster {
    fn default() -> Self {
        Self::new()
    }
}

impl SeverityAdjuster {
    pub fn new() -> Self {
        Self {
            context_weights: ContextWeights::default(),
        }
    }

    pub fn with_weights(context_weights: ContextWeights) -> Self {
        Self { context_weights }
    }

    pub fn adjust_severity(
        &self,
        pattern: &PerformanceAntiPattern,
        context: &PatternContext,
        confidence: f64,
    ) -> Priority {
        let base_severity = self.get_base_severity(pattern);
        let context_adjustment = self.calculate_context_adjustment(context);
        let confidence_adjustment = self.calculate_confidence_adjustment(confidence);

        let base_score = Self::priority_to_score(&base_severity);
        let adjusted_score = base_score * context_adjustment * confidence_adjustment;

        self.score_to_priority(adjusted_score)
    }

    fn get_base_severity(&self, pattern: &PerformanceAntiPattern) -> Priority {
        match pattern {
            PerformanceAntiPattern::InefficientIO { .. } => Priority::High,
            PerformanceAntiPattern::NestedLoopComplexity { .. } => Priority::High,
            PerformanceAntiPattern::UnboundedAllocation { .. } => Priority::Critical,
            PerformanceAntiPattern::SynchronousBlocking { .. } => Priority::Medium,
            PerformanceAntiPattern::InefficientAlgorithm { .. } => Priority::High,
            PerformanceAntiPattern::ResourceLeak { .. } => Priority::Critical,
            PerformanceAntiPattern::NestedLoop { .. } => Priority::High,
            PerformanceAntiPattern::InefficientDataStructure { .. } => Priority::Medium,
            PerformanceAntiPattern::ExcessiveAllocation { .. } => Priority::Medium,
            PerformanceAntiPattern::StringProcessingAntiPattern { .. } => Priority::Low,
        }
    }

    fn calculate_context_adjustment(&self, context: &PatternContext) -> f64 {
        let mut adjustment = 1.0;

        // Module type adjustment
        adjustment *= match context.module_type {
            ModuleType::Production => 1.0,
            ModuleType::Test => 0.3,      // Significant reduction for tests
            ModuleType::Benchmark => 0.1, // Benchmarks are expected to stress-test
            ModuleType::Example => 0.2,   // Examples are for demonstration
            ModuleType::Documentation => 0.1, // Doc tests are simple
            ModuleType::Utility => 0.7,   // Utility functions matter but less critical
            ModuleType::Infrastructure => 0.5, // Infrastructure should be efficient but not critical
        } * self.context_weights.module_type_weight;

        // Function intent adjustment
        adjustment *= match context.function_intent {
            FunctionIntent::BusinessLogic => 1.0,
            FunctionIntent::Setup => 0.2, // Setup is typically one-time
            FunctionIntent::Teardown => 0.1, // Teardown even less critical
            FunctionIntent::Validation => 0.8, // Validation should be efficient
            FunctionIntent::DataTransformation => 0.9, // Data transformation is important
            FunctionIntent::IOWrapper => 0.4, // I/O wrappers expected to do I/O
            FunctionIntent::ErrorHandling => 0.6, // Error handling should be fast but not critical
            FunctionIntent::Configuration => 0.3, // Configuration is typically one-time
            FunctionIntent::Unknown => 0.8, // Be conservative when uncertain
        } * self.context_weights.function_intent_weight;

        // Performance sensitivity adjustment
        adjustment *= match context.performance_sensitivity {
            PerformanceSensitivity::High => 1.5, // Boost for hot paths
            PerformanceSensitivity::Medium => 1.0,
            PerformanceSensitivity::Low => 0.5,
            PerformanceSensitivity::Irrelevant => 0.1,
        } * self.context_weights.performance_sensitivity_weight;

        // Business criticality adjustment
        adjustment *= match context.business_criticality {
            BusinessCriticality::Critical => 1.3, // Boost for critical business logic
            BusinessCriticality::Important => 1.0,
            BusinessCriticality::Utility => 0.7,
            BusinessCriticality::Infrastructure => 0.6,
            BusinessCriticality::Development => 0.2, // Development code is less critical
        } * self.context_weights.business_criticality_weight;

        adjustment.max(0.01) // Never reduce to zero
    }

    fn calculate_confidence_adjustment(&self, confidence: f64) -> f64 {
        // Apply a non-linear confidence curve
        // High confidence (>0.8) has minimal impact
        // Medium confidence (0.5-0.8) has moderate impact
        // Low confidence (<0.5) significantly reduces severity
        if confidence >= 0.8 {
            0.9 + (confidence - 0.8) * 0.5 // 0.9 to 1.0
        } else if confidence >= 0.5 {
            0.5 + (confidence - 0.5) * 1.33 // 0.5 to 0.9
        } else {
            confidence // Linear reduction below 0.5
        }
    }

    fn priority_to_score(priority: &Priority) -> f64 {
        match priority {
            Priority::Critical => 4.0,
            Priority::High => 3.0,
            Priority::Medium => 2.0,
            Priority::Low => 1.0,
        }
    }

    fn score_to_priority(&self, score: f64) -> Priority {
        if score >= Self::priority_to_score(&Priority::Critical) * 0.8 {
            Priority::Critical
        } else if score >= Self::priority_to_score(&Priority::High) * 0.8 {
            Priority::High
        } else if score >= Self::priority_to_score(&Priority::Medium) * 0.8 {
            Priority::Medium
        } else {
            Priority::Low
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::performance::PerformanceAntiPattern;

    #[test]
    fn test_severity_adjustment_test_context() {
        let adjuster = SeverityAdjuster::new();

        let pattern = PerformanceAntiPattern::InefficientIO {
            io_pattern: crate::performance::IOPattern::SyncInLoop,
            batching_opportunity: true,
            async_opportunity: true,
            location: crate::core::SourceLocation::default(),
        };

        let test_context = PatternContext {
            module_type: ModuleType::Test,
            function_intent: FunctionIntent::Setup,
            performance_sensitivity: PerformanceSensitivity::Irrelevant,
            business_criticality: BusinessCriticality::Development,
            architectural_pattern: Some(super::super::ArchitecturalPattern::TestFixture),
            confidence: 0.9,
        };

        let severity = adjuster.adjust_severity(&pattern, &test_context, 0.9);
        assert!(
            severity <= Priority::Low,
            "Test context should reduce severity"
        );
    }

    #[test]
    fn test_severity_adjustment_production_context() {
        let adjuster = SeverityAdjuster::new();

        let pattern = PerformanceAntiPattern::InefficientIO {
            io_pattern: crate::performance::IOPattern::SyncInLoop,
            batching_opportunity: true,
            async_opportunity: true,
            location: crate::core::SourceLocation::default(),
        };

        let production_context = PatternContext {
            module_type: ModuleType::Production,
            function_intent: FunctionIntent::BusinessLogic,
            performance_sensitivity: PerformanceSensitivity::High,
            business_criticality: BusinessCriticality::Critical,
            architectural_pattern: None,
            confidence: 0.9,
        };

        let severity = adjuster.adjust_severity(&pattern, &production_context, 0.9);
        assert!(
            severity >= Priority::High,
            "Production context should maintain high severity"
        );
    }

    #[test]
    fn test_confidence_impact() {
        let adjuster = SeverityAdjuster::new();

        let pattern = PerformanceAntiPattern::InefficientIO {
            io_pattern: crate::performance::IOPattern::SyncInLoop,
            batching_opportunity: true,
            async_opportunity: true,
            location: crate::core::SourceLocation::default(),
        };

        let context = PatternContext {
            module_type: ModuleType::Production,
            function_intent: FunctionIntent::BusinessLogic,
            performance_sensitivity: PerformanceSensitivity::Medium,
            business_criticality: BusinessCriticality::Important,
            architectural_pattern: None,
            confidence: 0.9,
        };

        let high_confidence_severity = adjuster.adjust_severity(&pattern, &context, 0.9);
        let low_confidence_severity = adjuster.adjust_severity(&pattern, &context, 0.3);

        assert!(
            high_confidence_severity > low_confidence_severity,
            "Higher confidence should result in higher severity"
        );
    }
}
