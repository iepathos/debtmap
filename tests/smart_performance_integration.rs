use debtmap::performance::{
    IOPattern, IOPerformanceDetector, NestedLoopDetector, PerformanceAntiPattern,
    PerformanceDetector, SmartPerformanceConfig, SmartPerformanceDetector,
};
use std::path::Path;
use syn;

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

    let file = syn::parse_str::<syn::File>(source).unwrap();
    let detectors: Vec<Box<dyn PerformanceDetector>> = vec![
        Box::new(IOPerformanceDetector::new()),
        Box::new(NestedLoopDetector::new()),
    ];
    let detector = SmartPerformanceDetector::new(detectors);
    let issues = detector.detect_with_context(&file, Path::new("tests/file_test.rs"), None);

    // Should detect the I/O pattern but classify it as test fixture with low severity
    for issue in &issues {
        assert!(
            issue.adjusted_severity <= debtmap::core::Priority::Low,
            "Test context should reduce severity"
        );
        assert!(
            issue.reasoning.to_lowercase().contains("test"),
            "Reasoning should mention test context"
        );
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

    let file = syn::parse_str::<syn::File>(source).unwrap();
    let detectors: Vec<Box<dyn PerformanceDetector>> = vec![Box::new(IOPerformanceDetector::new())];
    let detector = SmartPerformanceDetector::new(detectors);
    let issues = detector.detect_with_context(&file, Path::new("src/request_processor.rs"), None);

    // Should detect high-severity performance issue in production code
    for issue in &issues {
        assert!(
            issue.adjusted_severity >= debtmap::core::Priority::Medium,
            "Production context should maintain at least medium severity"
        );
        assert!(
            issue.reasoning.to_lowercase().contains("production"),
            "Reasoning should mention production context"
        );
    }
}

#[test]
fn test_smart_detection_with_custom_config() {
    let source = r#"
        fn utility_function() {
            for i in 0..100 {
                std::fs::write(&format!("file_{}.txt", i), "data").unwrap();
            }
        }
    "#;

    let file = syn::parse_str::<syn::File>(source).unwrap();

    // Test with lenient config
    let lenient_config = SmartPerformanceConfig {
        enabled: true,
        context_analysis_enabled: true,
        pattern_correlation_enabled: true,
        high_confidence_threshold: 0.95,
        production_confidence_threshold: 0.8,
        test_confidence_threshold: 0.3,
        utility_confidence_threshold: 0.9,
        base_confidence_threshold: 0.7,
        ignore_test_fixtures: true,
        reduce_test_severity: true,
        boost_critical_paths: true,
    };

    let detectors: Vec<Box<dyn PerformanceDetector>> = vec![Box::new(IOPerformanceDetector::new())];
    let detector = SmartPerformanceDetector::new(detectors).with_config(lenient_config);
    let lenient_issues =
        detector.detect_with_context(&file, Path::new("src/utils/helper.rs"), None);

    // Test with strict config
    let strict_config = SmartPerformanceConfig {
        enabled: true,
        context_analysis_enabled: true,
        pattern_correlation_enabled: true,
        high_confidence_threshold: 0.8,
        production_confidence_threshold: 0.5,
        test_confidence_threshold: 0.5,
        utility_confidence_threshold: 0.6,
        base_confidence_threshold: 0.4,
        ignore_test_fixtures: false,
        reduce_test_severity: false,
        boost_critical_paths: true,
    };

    let detectors: Vec<Box<dyn PerformanceDetector>> = vec![Box::new(IOPerformanceDetector::new())];
    let detector = SmartPerformanceDetector::new(detectors).with_config(strict_config);
    let strict_issues = detector.detect_with_context(&file, Path::new("src/utils/helper.rs"), None);

    // Strict config should be more likely to report issues
    assert!(
        strict_issues.len() >= lenient_issues.len(),
        "Strict config should report same or more issues"
    );
}

#[test]
fn test_context_classification() {
    use debtmap::performance::context::{ModuleClassifier, ModuleType};

    let classifier = ModuleClassifier::new();

    // Test files
    assert_eq!(
        classifier.classify_module(Path::new("tests/integration_test.rs")),
        ModuleType::Test
    );
    assert_eq!(
        classifier.classify_module(Path::new("src/lib_test.rs")),
        ModuleType::Test
    );

    // Benchmark files
    assert_eq!(
        classifier.classify_module(Path::new("benches/performance.rs")),
        ModuleType::Benchmark
    );

    // Production files
    assert_eq!(
        classifier.classify_module(Path::new("src/main.rs")),
        ModuleType::Production
    );
    assert_eq!(
        classifier.classify_module(Path::new("src/lib.rs")),
        ModuleType::Production
    );

    // Utility files
    assert_eq!(
        classifier.classify_module(Path::new("src/utils/helpers.rs")),
        ModuleType::Utility
    );

    // Example files
    assert_eq!(
        classifier.classify_module(Path::new("examples/demo.rs")),
        ModuleType::Example
    );
}

#[test]
fn test_function_intent_classification() {
    use debtmap::performance::context::{FunctionIntent, IntentClassifier};

    let classifier = IntentClassifier::new();

    // Setup function
    let setup_function = syn::parse_quote! {
        fn setup_test_environment() {
            // setup code
        }
    };
    assert_eq!(
        classifier.classify_function_intent(&setup_function, None),
        FunctionIntent::Setup
    );

    // Teardown function
    let teardown_function = syn::parse_quote! {
        fn cleanup_resources() {
            // cleanup code
        }
    };
    assert_eq!(
        classifier.classify_function_intent(&teardown_function, None),
        FunctionIntent::Teardown
    );

    // Business logic function
    let business_function = syn::parse_quote! {
        fn process_user_request(request: Request) -> Response {
            // business logic
            Response::new()
        }
    };
    assert_eq!(
        classifier.classify_function_intent(&business_function, None),
        FunctionIntent::BusinessLogic
    );

    // I/O wrapper function
    let io_function = syn::parse_quote! {
        fn read_config_file(path: &str) -> Config {
            std::fs::read_to_string(path).unwrap()
        }
    };
    assert_eq!(
        classifier.classify_function_intent(&io_function, None),
        FunctionIntent::IOWrapper
    );
}

#[test]
fn test_severity_adjustment() {
    use debtmap::common::SourceLocation;
    use debtmap::core::Priority;
    use debtmap::performance::context::{
        BusinessCriticality, FunctionIntent, ModuleType, PatternContext, PerformanceSensitivity,
        SeverityAdjuster,
    };

    let adjuster = SeverityAdjuster::new();

    let pattern = PerformanceAntiPattern::InefficientIO {
        io_pattern: IOPattern::SyncInLoop,
        batching_opportunity: true,
        async_opportunity: true,
        location: SourceLocation::default(),
    };

    // Test context (should reduce severity)
    let test_context = PatternContext {
        module_type: ModuleType::Test,
        function_intent: FunctionIntent::Setup,
        performance_sensitivity: PerformanceSensitivity::Irrelevant,
        business_criticality: BusinessCriticality::Development,
        architectural_pattern: None,
        confidence: 0.9,
    };

    let test_severity = adjuster.adjust_severity(&pattern, &test_context, 0.9);
    assert!(
        test_severity <= Priority::Low,
        "Test context should significantly reduce severity"
    );

    // Production context (should maintain high severity)
    let production_context = PatternContext {
        module_type: ModuleType::Production,
        function_intent: FunctionIntent::BusinessLogic,
        performance_sensitivity: PerformanceSensitivity::High,
        business_criticality: BusinessCriticality::Critical,
        architectural_pattern: None,
        confidence: 0.9,
    };

    let production_severity = adjuster.adjust_severity(&pattern, &production_context, 0.9);
    assert!(
        production_severity >= Priority::High,
        "Production context should maintain high severity"
    );

    // Utility context (should have medium severity)
    let utility_context = PatternContext {
        module_type: ModuleType::Utility,
        function_intent: FunctionIntent::Unknown,
        performance_sensitivity: PerformanceSensitivity::Medium,
        business_criticality: BusinessCriticality::Utility,
        architectural_pattern: None,
        confidence: 0.7,
    };

    let utility_severity = adjuster.adjust_severity(&pattern, &utility_context, 0.7);
    assert!(
        utility_severity > test_severity && utility_severity < production_severity,
        "Utility context should have intermediate severity. test={:?}, utility={:?}, production={:?}",
        test_severity, utility_severity, production_severity
    );
}

#[test]
fn test_pattern_correlation() {
    use debtmap::common::SourceLocation;
    use debtmap::performance::context::{
        BusinessCriticality, FunctionIntent, ModuleType, PatternContext, PerformanceSensitivity,
    };
    use debtmap::performance::pattern_correlator::{CorrelationType, PatternCorrelator};

    let correlator = PatternCorrelator::new();

    // Test fixture pattern
    let test_patterns = vec![PerformanceAntiPattern::InefficientIO {
        io_pattern: IOPattern::SyncInLoop,
        batching_opportunity: true,
        async_opportunity: true,
        location: SourceLocation::default(),
    }];

    let test_contexts = vec![PatternContext {
        module_type: ModuleType::Test,
        function_intent: FunctionIntent::Setup,
        architectural_pattern: None,
        business_criticality: BusinessCriticality::Development,
        performance_sensitivity: PerformanceSensitivity::Irrelevant,
        confidence: 0.8,
    }];

    let correlations = correlator.correlate_patterns(&test_patterns, &test_contexts);

    assert!(
        !correlations.is_empty(),
        "Should find test fixture correlation"
    );
    assert_eq!(
        correlations[0].correlation_type,
        CorrelationType::TestFixture
    );
    assert_eq!(correlations[0].severity_reduction, 0.2);
}
