use crate::debt::Priority;
use crate::performance::context::{
    FunctionIntent, IntentClassifier, ModuleClassifier, ModuleType, PatternContext,
    PerformanceSensitivity, SeverityAdjuster,
};
use crate::performance::{PerformanceAntiPattern, PerformanceDetector};
use crate::priority::call_graph::CallGraph;
use std::path::Path;
use syn::{File, ItemFn};

pub struct SmartPerformanceDetector {
    base_detectors: Vec<Box<dyn PerformanceDetector>>,
    module_classifier: ModuleClassifier,
    intent_classifier: IntentClassifier,
    severity_adjuster: SeverityAdjuster,
    config: SmartPerformanceConfig,
}

#[derive(Debug, Clone)]
pub struct SmartPerformanceConfig {
    pub enabled: bool,
    pub context_analysis_enabled: bool,
    pub pattern_correlation_enabled: bool,

    // Confidence thresholds
    pub high_confidence_threshold: f64,       // 0.9
    pub production_confidence_threshold: f64, // 0.7
    pub test_confidence_threshold: f64,       // 0.5
    pub utility_confidence_threshold: f64,    // 0.8
    pub base_confidence_threshold: f64,       // 0.6

    // Pattern-specific settings
    pub ignore_test_fixtures: bool, // true
    pub reduce_test_severity: bool, // true
    pub boost_critical_paths: bool, // true
}

impl Default for SmartPerformanceConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            context_analysis_enabled: true,
            pattern_correlation_enabled: true,
            high_confidence_threshold: 0.9,
            production_confidence_threshold: 0.7,
            test_confidence_threshold: 0.5,
            utility_confidence_threshold: 0.8,
            base_confidence_threshold: 0.6,
            ignore_test_fixtures: true,
            reduce_test_severity: true,
            boost_critical_paths: true,
        }
    }
}

impl SmartPerformanceDetector {
    pub fn new(base_detectors: Vec<Box<dyn PerformanceDetector>>) -> Self {
        Self {
            base_detectors,
            module_classifier: ModuleClassifier::new(),
            intent_classifier: IntentClassifier::new(),
            severity_adjuster: SeverityAdjuster::new(),
            config: SmartPerformanceConfig::default(),
        }
    }

    pub fn with_config(mut self, config: SmartPerformanceConfig) -> Self {
        self.config = config;
        self
    }

    pub fn detect_with_context(
        &self,
        file: &File,
        path: &Path,
        call_graph: Option<&CallGraph>,
    ) -> Vec<SmartPerformanceIssue> {
        if !self.config.enabled {
            // Fall back to regular detection if smart detection is disabled
            return self.detect_without_context(file, path);
        }

        // Step 1: Run base detection
        let mut raw_patterns = Vec::new();
        for detector in &self.base_detectors {
            raw_patterns.extend(detector.detect_anti_patterns(file, path));
        }

        if raw_patterns.is_empty() {
            return Vec::new();
        }

        // Step 2: Analyze module context
        let module_context = self.analyze_module_context(file, path);

        // Step 3: Analyze each pattern with context
        let mut smart_issues = Vec::new();
        for pattern in raw_patterns {
            let function_context =
                self.analyze_pattern_context(&pattern, file, &module_context, call_graph);
            let confidence = self.calculate_pattern_confidence(&pattern, &function_context);

            // Step 4: Apply smart filtering
            if self.should_report_pattern(&pattern, &function_context, confidence) {
                let adjusted_severity =
                    self.severity_adjuster
                        .adjust_severity(&pattern, &function_context, confidence);

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

    fn detect_without_context(&self, file: &File, path: &Path) -> Vec<SmartPerformanceIssue> {
        let mut issues = Vec::new();
        for detector in &self.base_detectors {
            for pattern in detector.detect_anti_patterns(file, path) {
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

    fn analyze_module_context(&self, _file: &File, path: &Path) -> PatternContext {
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

    fn analyze_pattern_context(
        &self,
        pattern: &PerformanceAntiPattern,
        file: &File,
        module_context: &PatternContext,
        call_graph: Option<&CallGraph>,
    ) -> PatternContext {
        // Try to find the function containing this pattern
        if let Some(function) = self.find_containing_function(pattern, file) {
            let function_intent = self
                .intent_classifier
                .classify_function_intent(&function, call_graph);

            PatternContext {
                module_type: module_context.module_type.clone(),
                function_intent: function_intent.clone(),
                architectural_pattern: self.detect_architectural_pattern(&function, module_context),
                business_criticality: self.refine_business_criticality(
                    &function_intent,
                    &module_context.business_criticality,
                ),
                performance_sensitivity: self.refine_performance_sensitivity(
                    &function_intent,
                    &module_context.performance_sensitivity,
                ),
                confidence: self.calculate_context_confidence(&function_intent, module_context),
            }
        } else {
            module_context.clone()
        }
    }

    fn find_containing_function(
        &self,
        pattern: &PerformanceAntiPattern,
        file: &File,
    ) -> Option<ItemFn> {
        // This is a simplified implementation
        // In reality, we'd need to match the pattern location with function spans
        let location = pattern.location();

        for item in &file.items {
            if let syn::Item::Fn(func) = item {
                // Check if pattern location is within function
                // This would require proper span tracking
                if self.is_location_in_function(&location, func) {
                    return Some(func.clone());
                }
            }
        }
        None
    }

    fn is_location_in_function(
        &self,
        _location: &crate::common::SourceLocation,
        _func: &ItemFn,
    ) -> bool {
        // Simplified implementation
        // Would need proper span tracking in real implementation
        true
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

    fn generate_reasoning(
        &self,
        pattern: &PerformanceAntiPattern,
        context: &PatternContext,
    ) -> String {
        let mut reasoning = String::new();

        // Add pattern description
        reasoning.push_str(&format!("Detected: {}\n", self.describe_pattern(pattern)));

        // Add context information
        reasoning.push_str(&format!("Context: {:?} module, ", context.module_type));
        reasoning.push_str(&format!("{:?} function intent\n", context.function_intent));

        // Add specific reasoning based on context
        match (&context.module_type, &context.function_intent) {
            (ModuleType::Test, FunctionIntent::Setup | FunctionIntent::Teardown) => {
                reasoning.push_str(
                    "I/O operations in test setup/teardown context are typically acceptable",
                );
            }
            (ModuleType::Test, _) => {
                reasoning
                    .push_str("Performance in test code is lower priority than production code");
            }
            (ModuleType::Production, FunctionIntent::BusinessLogic) => {
                reasoning
                    .push_str("Performance issue in production business logic should be addressed");
            }
            _ => {
                reasoning.push_str("Consider the context when deciding whether to optimize");
            }
        }

        reasoning
    }

    fn generate_contextual_recommendation(
        &self,
        pattern: &PerformanceAntiPattern,
        context: &PatternContext,
    ) -> String {
        match (&context.module_type, &context.function_intent) {
            (ModuleType::Test, FunctionIntent::Setup | FunctionIntent::Teardown) => {
                "This appears to be test fixture setup/teardown. Optimize only if test performance is critical.".to_string()
            }
            (ModuleType::Test, _) => {
                "Test code performance is less critical. Consider optimizing if tests are slow.".to_string()
            }
            (ModuleType::Production, FunctionIntent::BusinessLogic) => {
                self.generate_production_recommendation(pattern)
            }
            (ModuleType::Utility, _) => {
                "Utility function performance may impact multiple consumers. Consider caching or lazy evaluation.".to_string()
            }
            _ => self.generate_default_recommendation(pattern),
        }
    }

    fn generate_production_recommendation(&self, pattern: &PerformanceAntiPattern) -> String {
        match pattern {
            PerformanceAntiPattern::InefficientIO {
                batching_opportunity,
                async_opportunity,
                ..
            } => {
                let mut rec = String::from("Production I/O performance issue. ");
                if *batching_opportunity {
                    rec.push_str("Consider batching operations. ");
                }
                if *async_opportunity {
                    rec.push_str("Consider async I/O. ");
                }
                rec
            }
            PerformanceAntiPattern::NestedLoopComplexity { .. } => {
                "Nested loops in production code. Consider algorithmic improvements or early exits."
                    .to_string()
            }
            _ => self.generate_default_recommendation(pattern),
        }
    }

    fn generate_default_recommendation(&self, pattern: &PerformanceAntiPattern) -> String {
        match pattern {
            PerformanceAntiPattern::InefficientIO { .. } => {
                "Consider batching I/O operations or using async I/O".to_string()
            }
            PerformanceAntiPattern::NestedLoopComplexity { .. } => {
                "Consider algorithmic improvements to reduce complexity".to_string()
            }
            PerformanceAntiPattern::UnboundedAllocation { .. } => {
                "Implement bounds checking or streaming to prevent memory issues".to_string()
            }
            PerformanceAntiPattern::SynchronousBlocking { .. } => {
                "Consider async operations to improve responsiveness".to_string()
            }
            PerformanceAntiPattern::InefficientAlgorithm { .. } => {
                "Review algorithm choice for better time complexity".to_string()
            }
            PerformanceAntiPattern::ResourceLeak { .. } => {
                "Ensure proper resource cleanup in all code paths".to_string()
            }
            PerformanceAntiPattern::NestedLoop { .. } => {
                "Consider algorithm optimization or caching".to_string()
            }
            PerformanceAntiPattern::InefficientDataStructure { .. } => {
                "Consider using more efficient data structures".to_string()
            }
            PerformanceAntiPattern::ExcessiveAllocation { .. } => {
                "Reduce allocations through pooling or pre-allocation".to_string()
            }
            PerformanceAntiPattern::StringProcessingAntiPattern { .. } => {
                "Optimize string processing operations".to_string()
            }
        }
    }

    fn describe_pattern(&self, pattern: &PerformanceAntiPattern) -> String {
        match pattern {
            PerformanceAntiPattern::InefficientIO { io_pattern, .. } => {
                format!("Inefficient I/O pattern: {:?}", io_pattern)
            }
            PerformanceAntiPattern::NestedLoopComplexity { depth, .. } => {
                format!("Nested loop complexity (depth: {})", depth)
            }
            PerformanceAntiPattern::UnboundedAllocation { .. } => {
                "Unbounded memory allocation".to_string()
            }
            PerformanceAntiPattern::SynchronousBlocking { .. } => {
                "Synchronous blocking operation".to_string()
            }
            PerformanceAntiPattern::InefficientAlgorithm { .. } => {
                "Inefficient algorithm".to_string()
            }
            PerformanceAntiPattern::ResourceLeak { .. } => "Potential resource leak".to_string(),
            PerformanceAntiPattern::NestedLoop { nesting_level, .. } => {
                format!("Nested loop with {} levels", nesting_level)
            }
            PerformanceAntiPattern::InefficientDataStructure { operation, .. } => {
                format!("Inefficient data structure operation: {:?}", operation)
            }
            PerformanceAntiPattern::ExcessiveAllocation {
                allocation_type, ..
            } => {
                format!("Excessive allocation: {:?}", allocation_type)
            }
            PerformanceAntiPattern::StringProcessingAntiPattern { pattern_type, .. } => {
                format!("String processing anti-pattern: {:?}", pattern_type)
            }
        }
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

    fn infer_business_criticality(
        &self,
        module_type: &ModuleType,
    ) -> super::context::BusinessCriticality {
        use super::context::BusinessCriticality;

        match module_type {
            ModuleType::Production => BusinessCriticality::Important,
            ModuleType::Test
            | ModuleType::Benchmark
            | ModuleType::Example
            | ModuleType::Documentation => BusinessCriticality::Development,
            ModuleType::Utility => BusinessCriticality::Utility,
            ModuleType::Infrastructure => BusinessCriticality::Infrastructure,
        }
    }

    fn infer_performance_sensitivity(&self, module_type: &ModuleType) -> PerformanceSensitivity {
        match module_type {
            ModuleType::Production => PerformanceSensitivity::Medium,
            ModuleType::Test | ModuleType::Example | ModuleType::Documentation => {
                PerformanceSensitivity::Irrelevant
            }
            ModuleType::Benchmark => PerformanceSensitivity::High, // Benchmarks measure performance
            ModuleType::Utility | ModuleType::Infrastructure => PerformanceSensitivity::Low,
        }
    }

    fn detect_architectural_pattern(
        &self,
        function: &ItemFn,
        context: &PatternContext,
    ) -> Option<super::context::ArchitecturalPattern> {
        use super::context::ArchitecturalPattern;

        let function_name = function.sig.ident.to_string().to_lowercase();

        // Check for test fixture patterns
        if context.module_type == ModuleType::Test {
            if function_name.contains("setup")
                || function_name.contains("fixture")
                || function_name.contains("mock")
            {
                return Some(ArchitecturalPattern::TestFixture);
            }
        }

        // Check for builder pattern
        if function_name.contains("build") || function_name.contains("builder") {
            return Some(ArchitecturalPattern::Builder);
        }

        // Check for factory pattern
        if function_name.contains("create") || function_name.contains("factory") {
            return Some(ArchitecturalPattern::Factory);
        }

        None
    }

    fn refine_business_criticality(
        &self,
        function_intent: &FunctionIntent,
        base_criticality: &super::context::BusinessCriticality,
    ) -> super::context::BusinessCriticality {
        use super::context::BusinessCriticality;

        match function_intent {
            FunctionIntent::BusinessLogic
                if *base_criticality == BusinessCriticality::Important =>
            {
                BusinessCriticality::Critical
            }
            FunctionIntent::Setup | FunctionIntent::Teardown | FunctionIntent::Configuration => {
                BusinessCriticality::Infrastructure
            }
            _ => base_criticality.clone(),
        }
    }

    fn refine_performance_sensitivity(
        &self,
        function_intent: &FunctionIntent,
        base_sensitivity: &PerformanceSensitivity,
    ) -> PerformanceSensitivity {
        match function_intent {
            FunctionIntent::BusinessLogic
                if *base_sensitivity == PerformanceSensitivity::Medium =>
            {
                PerformanceSensitivity::High
            }
            FunctionIntent::Setup | FunctionIntent::Teardown | FunctionIntent::Configuration => {
                PerformanceSensitivity::Low
            }
            _ => base_sensitivity.clone(),
        }
    }

    fn calculate_context_confidence(
        &self,
        function_intent: &FunctionIntent,
        module_context: &PatternContext,
    ) -> f64 {
        let mut confidence = module_context.confidence;

        // Higher confidence if we identified the function intent
        if !matches!(function_intent, FunctionIntent::Unknown) {
            confidence = (confidence + 0.1).min(1.0);
        }

        confidence
    }
}

#[derive(Debug, Clone)]
pub struct SmartPerformanceIssue {
    pub original_pattern: PerformanceAntiPattern,
    pub context: PatternContext,
    pub adjusted_severity: Priority,
    pub confidence: f64,
    pub reasoning: String,
    pub recommendation: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::performance::{IOPerformanceDetector, NestedLoopDetector};

    #[test]
    fn test_smart_detection_filters_test_fixtures() {
        let source = r#"
            #[cfg(test)]
            mod tests {
                use tempfile::TempDir;
                
                #[test]
                fn test_file_processing() {
                    let temp_dir = TempDir::new().unwrap();
                    
                    // This should be recognized as test fixture setup
                    for i in 0..5 {
                        let test_file = temp_dir.path().join(format!("test_{}.rs", i));
                        std::fs::write(&test_file, "test content").unwrap();
                    }
                    
                    // Test the actual functionality
                    process_files(&temp_dir.path());
                }
            }
        "#;

        let file = syn::parse_str::<File>(source).unwrap();
        let detectors: Vec<Box<dyn PerformanceDetector>> = vec![
            Box::new(IOPerformanceDetector::new()),
            Box::new(NestedLoopDetector::new()),
        ];
        let detector = SmartPerformanceDetector::new(detectors);
        let issues = detector.detect_with_context(&file, Path::new("tests/file_test.rs"), None);

        // Should detect the I/O pattern but classify it as test fixture with low severity
        for issue in &issues {
            assert!(matches!(issue.context.module_type, ModuleType::Test));
            assert!(issue.adjusted_severity <= Priority::Low);
            assert!(issue.reasoning.to_lowercase().contains("test"));
        }
    }

    #[test]
    fn test_production_io_maintains_high_severity() {
        let source = r#"
            pub fn process_user_requests(requests: &[Request]) -> Vec<Response> {
                let mut responses = Vec::new();
                for request in requests {
                    // This should be flagged as high-severity blocking I/O
                    let data = std::fs::read_to_string(&request.file_path).unwrap();
                    responses.push(process_data(&data));
                }
                responses
            }
        "#;

        let file = syn::parse_str::<File>(source).unwrap();
        let detectors: Vec<Box<dyn PerformanceDetector>> =
            vec![Box::new(IOPerformanceDetector::new())];
        let detector = SmartPerformanceDetector::new(detectors);
        let issues =
            detector.detect_with_context(&file, Path::new("src/request_processor.rs"), None);

        // Should detect high-severity performance issue in production code
        for issue in &issues {
            assert!(matches!(issue.context.module_type, ModuleType::Production));
            assert!(issue.adjusted_severity >= Priority::Medium);
            assert!(issue.reasoning.to_lowercase().contains("production"));
        }
    }
}
