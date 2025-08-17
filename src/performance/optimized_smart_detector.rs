use crate::performance::context::{
    BusinessCriticality, FunctionIntent, IntentClassifier, ModuleClassifier, ModuleType,
    PatternContext, PerformanceSensitivity, SeverityAdjuster,
};
use crate::performance::{
    CollectedPerformanceData, OptimizedAllocationDetector, OptimizedDataStructureDetector,
    OptimizedIODetector, OptimizedNestedLoopDetector, OptimizedPerformanceDetector,
    OptimizedStringDetector, PerformanceAntiPattern, SmartPerformanceConfig,
    SmartPerformanceIssue,
};
use crate::priority::call_graph::CallGraph;
use std::path::Path;

/// Smart performance detector that works with pre-collected data
pub struct OptimizedSmartDetector {
    optimized_detectors: Vec<Box<dyn OptimizedPerformanceDetector>>,
    module_classifier: ModuleClassifier,
    _intent_classifier: IntentClassifier,
    severity_adjuster: SeverityAdjuster,
    config: SmartPerformanceConfig,
}

impl OptimizedSmartDetector {
    pub fn new() -> Self {
        let optimized_detectors: Vec<Box<dyn OptimizedPerformanceDetector>> = vec![
            Box::new(OptimizedNestedLoopDetector::new()),
            Box::new(OptimizedIODetector::new()),
            Box::new(OptimizedAllocationDetector::new()),
            Box::new(OptimizedStringDetector::new()),
            Box::new(OptimizedDataStructureDetector::new()),
        ];

        Self {
            optimized_detectors,
            module_classifier: ModuleClassifier::new(),
            _intent_classifier: IntentClassifier::new(),
            severity_adjuster: SeverityAdjuster::new(),
            config: SmartPerformanceConfig::default(),
        }
    }

    pub fn with_config(mut self, config: SmartPerformanceConfig) -> Self {
        self.config = config;
        self
    }

    /// Analyze collected data with smart context analysis and pattern correlation
    pub fn analyze_with_context(
        &self,
        data: &CollectedPerformanceData,
        path: &Path,
        call_graph: Option<&CallGraph>,
    ) -> Vec<SmartPerformanceIssue> {
        if !self.config.enabled {
            return self.analyze_without_context(data, path);
        }

        // Step 1: Run optimized detection on collected data
        let mut raw_patterns = Vec::new();
        for detector in &self.optimized_detectors {
            raw_patterns.extend(detector.analyze_collected_data(data, path));
        }

        if raw_patterns.is_empty() {
            return Vec::new();
        }

        // Step 2: Apply pattern correlation if enabled
        let patterns = if self.config.pattern_correlation_enabled {
            self.apply_pattern_correlation(raw_patterns, data)
        } else {
            raw_patterns
        };

        // Step 3: Analyze module context
        let module_context = self.analyze_module_context(path);

        // Step 4: Analyze each pattern with context
        let mut smart_issues = Vec::new();
        for pattern in patterns {
            let function_context = self.analyze_pattern_context_from_data(
                &pattern,
                data,
                &module_context,
                call_graph,
            );
            let confidence = self.calculate_pattern_confidence(&pattern, &function_context);

            // Step 5: Apply smart filtering
            if self.should_report_pattern(&pattern, &function_context, confidence) {
                let adjusted_severity = self.severity_adjuster.adjust_severity(
                    &pattern,
                    &function_context,
                    confidence,
                );

                smart_issues.push(SmartPerformanceIssue {
                    original_pattern: pattern.clone(),
                    context: function_context.clone(),
                    adjusted_severity,
                    confidence,
                    reasoning: self.generate_reasoning(&pattern, &function_context),
                    recommendation: self
                        .generate_contextual_recommendation(&pattern, &function_context),
                });
            }
        }

        smart_issues
    }

    /// Apply pattern correlation to reduce false positives
    fn apply_pattern_correlation(
        &self,
        patterns: Vec<PerformanceAntiPattern>,
        data: &CollectedPerformanceData,
    ) -> Vec<PerformanceAntiPattern> {
        let mut filtered_patterns = Vec::new();

        for pattern in patterns {
            // Check if this pattern should be filtered based on correlations
            if !self.should_filter_by_correlation(&pattern, data) {
                filtered_patterns.push(pattern);
            }
        }

        // For now, return filtered patterns directly
        // Full correlation would require context for each pattern
        filtered_patterns
    }

    /// Check if a pattern should be filtered based on correlations
    fn should_filter_by_correlation(
        &self,
        pattern: &PerformanceAntiPattern,
        data: &CollectedPerformanceData,
    ) -> bool {
        // Example correlation rules:

        // 1. Filter I/O in test setup loops
        if let PerformanceAntiPattern::InefficientIO { .. } = pattern {
            let location = pattern.location();
            // Check if this I/O is in a test function
            for func in &data.functions {
                if func.is_test
                    && location.line >= func.span.0
                    && location.line <= func.span.1
                {
                    return true; // Filter test I/O
                }
            }
        }

        // 2. Filter allocations in iterator chains (often optimized by compiler)
        if let PerformanceAntiPattern::ExcessiveAllocation { .. } = pattern {
            // Check if allocation is in an iterator chain
            for loop_info in &data.loops {
                if loop_info.is_iterator_chain
                    && loop_info.location.line == pattern.location().line
                {
                    return true; // Filter iterator allocations
                }
            }
        }

        false
    }

    fn analyze_without_context(
        &self,
        data: &CollectedPerformanceData,
        path: &Path,
    ) -> Vec<SmartPerformanceIssue> {
        let mut issues = Vec::new();
        for detector in &self.optimized_detectors {
            for pattern in detector.analyze_collected_data(data, path) {
                issues.push(SmartPerformanceIssue {
                    original_pattern: pattern.clone(),
                    context: PatternContext::default(),
                    adjusted_severity: self.get_base_severity(&pattern),
                    confidence: 1.0,
                    reasoning: "No context analysis performed".to_string(),
                    recommendation: self.generate_default_recommendation(&pattern),
                });
            }
        }
        issues
    }

    fn analyze_module_context(&self, path: &Path) -> PatternContext {
        let module_type = self.module_classifier.classify_module(path);

        PatternContext {
            module_type: module_type.clone(),
            function_intent: FunctionIntent::Unknown,
            architectural_pattern: None,
            business_criticality: self.infer_business_criticality(&module_type),
            performance_sensitivity: self.infer_performance_sensitivity(&module_type),
            confidence: 0.8,
        }
    }

    fn analyze_pattern_context_from_data(
        &self,
        pattern: &PerformanceAntiPattern,
        data: &CollectedPerformanceData,
        module_context: &PatternContext,
        _call_graph: Option<&CallGraph>,
    ) -> PatternContext {
        // Find the function containing this pattern
        let pattern_line = pattern.location().line;
        
        for func in &data.functions {
            if pattern_line >= func.span.0 && pattern_line <= func.span.1 {
                // Determine function intent based on name and characteristics
                let function_intent = if func.is_test {
                    if func.name.contains("setup") || func.name.contains("init") {
                        FunctionIntent::Setup
                    } else if func.name.contains("teardown") || func.name.contains("cleanup") {
                        FunctionIntent::Teardown
                    } else {
                        FunctionIntent::BusinessLogic
                    }
                } else if func.name.contains("handle") || func.name.contains("process") {
                    FunctionIntent::IOWrapper
                } else {
                    FunctionIntent::BusinessLogic
                };

                return PatternContext {
                    module_type: module_context.module_type.clone(),
                    function_intent: function_intent.clone(),
                    architectural_pattern: None,
                    business_criticality: self.refine_business_criticality(
                        &function_intent,
                        &module_context.business_criticality,
                    ),
                    performance_sensitivity: self.refine_performance_sensitivity(
                        &function_intent,
                        &module_context.performance_sensitivity,
                    ),
                    confidence: 0.9, // Higher confidence since we have function data
                };
            }
        }

        module_context.clone()
    }

    fn should_report_pattern(
        &self,
        _pattern: &PerformanceAntiPattern,
        context: &PatternContext,
        confidence: f64,
    ) -> bool {
        // Always report if confidence is very high
        if confidence >= self.config.high_confidence_threshold {
            return true;
        }

        // Filter based on context
        match (
            &context.module_type,
            &context.function_intent,
            &context.performance_sensitivity,
        ) {
            // Never report test fixture setup/teardown
            (ModuleType::Test, FunctionIntent::Setup | FunctionIntent::Teardown, _)
                if self.config.ignore_test_fixtures =>
            {
                false
            }

            // Report test business logic with reduced threshold
            (ModuleType::Test, FunctionIntent::BusinessLogic, _) => {
                confidence >= self.config.test_confidence_threshold
            }

            // Report utility functions only if high confidence
            (_, _, PerformanceSensitivity::Irrelevant) => {
                confidence >= self.config.utility_confidence_threshold
            }

            // Report production code with normal thresholds
            (
                ModuleType::Production,
                _,
                PerformanceSensitivity::High | PerformanceSensitivity::Medium,
            ) => confidence >= self.config.production_confidence_threshold,

            // Default: report if above base threshold
            _ => confidence >= self.config.base_confidence_threshold,
        }
    }

    fn calculate_pattern_confidence(
        &self,
        _pattern: &PerformanceAntiPattern,
        context: &PatternContext,
    ) -> f64 {
        let mut confidence = context.confidence;

        // Adjust confidence based on context clarity
        match context.function_intent {
            FunctionIntent::Unknown => confidence *= 0.8,
            FunctionIntent::Setup | FunctionIntent::Teardown
                if context.module_type == ModuleType::Test =>
            {
                confidence *= 0.95 // High confidence this is a test fixture
            }
            _ => {}
        }

        confidence.min(1.0).max(0.0)
    }

    fn get_base_severity(&self, pattern: &PerformanceAntiPattern) -> crate::debt::Priority {
        use crate::performance::PerformanceImpact;

        // Get impact from first detector that recognizes the pattern
        let impact = self
            .optimized_detectors
            .iter()
            .find_map(|d| {
                let test_data = CollectedPerformanceData::new(String::new(), std::path::PathBuf::new());
                let test_patterns = d.analyze_collected_data(&test_data, Path::new(""));
                if test_patterns.iter().any(|p| {
                    std::mem::discriminant(p) == std::mem::discriminant(pattern)
                }) {
                    Some(d.estimate_impact(pattern))
                } else {
                    None
                }
            })
            .unwrap_or(PerformanceImpact::Low);

        crate::performance::impact_to_priority(impact)
    }

    fn generate_reasoning(
        &self,
        _pattern: &PerformanceAntiPattern,
        context: &PatternContext,
    ) -> String {
        format!(
            "Pattern detected in {:?} module with {:?} function intent. Confidence: {:.2}",
            context.module_type, context.function_intent, context.confidence
        )
    }

    fn generate_contextual_recommendation(
        &self,
        pattern: &PerformanceAntiPattern,
        context: &PatternContext,
    ) -> String {
        match (&context.module_type, &context.function_intent) {
            (ModuleType::Test, _) => {
                format!("In test code: Consider if optimization is worth the complexity")
            }
            (_, FunctionIntent::IOWrapper) => {
                format!("I/O-bound function: Focus on batching and async operations")
            }
            _ => self.generate_default_recommendation(pattern),
        }
    }

    fn generate_default_recommendation(&self, _pattern: &PerformanceAntiPattern) -> String {
        "Consider optimizing this performance pattern".to_string()
    }

    fn infer_business_criticality(&self, module_type: &ModuleType) -> BusinessCriticality {
        use crate::performance::context::BusinessCriticality;
        match module_type {
            ModuleType::Production => BusinessCriticality::Important,
            ModuleType::Test => BusinessCriticality::Development,
            ModuleType::Utility => BusinessCriticality::Utility,
            ModuleType::Benchmark => BusinessCriticality::Development,
            ModuleType::Example => BusinessCriticality::Development,
            _ => BusinessCriticality::Utility,
        }
    }

    fn infer_performance_sensitivity(&self, module_type: &ModuleType) -> PerformanceSensitivity {
        match module_type {
            ModuleType::Production => PerformanceSensitivity::High,
            ModuleType::Test => PerformanceSensitivity::Low,
            ModuleType::Utility => PerformanceSensitivity::Medium,
            ModuleType::Benchmark => PerformanceSensitivity::High,
            ModuleType::Example => PerformanceSensitivity::Irrelevant,
            _ => PerformanceSensitivity::Medium,
        }
    }

    fn refine_business_criticality(&self, intent: &FunctionIntent, base: &BusinessCriticality) -> BusinessCriticality {
        use crate::performance::context::BusinessCriticality;
        match (intent, base) {
            (FunctionIntent::BusinessLogic, BusinessCriticality::Utility) => BusinessCriticality::Important,
            (FunctionIntent::BusinessLogic, BusinessCriticality::Important) => BusinessCriticality::Critical,
            (FunctionIntent::Setup | FunctionIntent::Teardown, _) => BusinessCriticality::Development,
            _ => base.clone(),
        }
    }

    fn refine_performance_sensitivity(
        &self,
        intent: &FunctionIntent,
        base: &PerformanceSensitivity,
    ) -> PerformanceSensitivity {
        match (intent, base) {
            (FunctionIntent::Setup | FunctionIntent::Teardown, _) => {
                PerformanceSensitivity::Irrelevant
            }
            _ => base.clone(),
        }
    }
}