pub mod assertion_detector;
pub mod complexity_detector;
pub mod flaky_detector;

use crate::core::{DebtItem, DebtType, Priority};
use std::path::PathBuf;
use syn::{File, ItemFn};

#[derive(Debug, Clone, PartialEq)]
pub enum TestingAntiPattern {
    TestWithoutAssertions {
        test_name: String,
        file: PathBuf,
        line: usize,
        has_setup: bool,
        has_action: bool,
        suggested_assertions: Vec<String>,
    },
    OverlyComplexTest {
        test_name: String,
        file: PathBuf,
        line: usize,
        complexity_score: u32,
        complexity_sources: Vec<ComplexitySource>,
        suggested_simplification: TestSimplification,
    },
    FlakyTestPattern {
        test_name: String,
        file: PathBuf,
        line: usize,
        flakiness_type: FlakinessType,
        reliability_impact: ReliabilityImpact,
        stabilization_suggestion: String,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum ComplexitySource {
    ExcessiveMocking,
    NestedConditionals,
    MultipleAssertions,
    LoopInTest,
    ExcessiveSetup,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TestSimplification {
    ExtractHelper,
    SplitTest,
    ParameterizeTest,
    SimplifySetup,
    ReduceMocking,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FlakinessType {
    TimingDependency,
    RandomValues,
    ExternalDependency,
    FilesystemDependency,
    NetworkDependency,
    ThreadingIssue,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ReliabilityImpact {
    Critical,
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TestQualityImpact {
    Critical,
    High,
    Medium,
    Low,
}

pub trait TestingDetector {
    fn detect_anti_patterns(&self, file: &File, path: &PathBuf) -> Vec<TestingAntiPattern>;
    fn detector_name(&self) -> &'static str;
    fn assess_test_quality_impact(&self, pattern: &TestingAntiPattern) -> TestQualityImpact;
}

pub fn is_test_function(function: &ItemFn) -> bool {
    function.attrs.iter().any(|attr| {
        // Check if it's a test attribute
        let path_str = attr.path().segments.iter()
            .map(|seg| seg.ident.to_string())
            .collect::<Vec<_>>()
            .join("::");
        
        // Match common test attributes
        path_str == "test" 
            || path_str == "tokio::test"
            || path_str == "async_std::test" 
            || path_str == "bench"
            || path_str.ends_with("::test")
    }) || function.sig.ident.to_string().starts_with("test_")
        || function.sig.ident.to_string().ends_with("_test")
}

pub fn analyze_testing_patterns(file: &File, path: &PathBuf) -> Vec<DebtItem> {
    let detectors: Vec<Box<dyn TestingDetector>> = vec![
        Box::new(assertion_detector::AssertionDetector::new()),
        Box::new(complexity_detector::TestComplexityDetector::new()),
        Box::new(flaky_detector::FlakyTestDetector::new()),
    ];

    let mut testing_items = Vec::new();

    for detector in detectors {
        let anti_patterns = detector.detect_anti_patterns(file, path);

        for pattern in anti_patterns {
            let impact = detector.assess_test_quality_impact(&pattern);
            let debt_item = convert_testing_pattern_to_debt_item(pattern, impact, path);
            testing_items.push(debt_item);
        }
    }

    testing_items
}

fn convert_testing_pattern_to_debt_item(
    pattern: TestingAntiPattern,
    _impact: TestQualityImpact,
    path: &PathBuf,
) -> DebtItem {
    let (priority, message, context, line, debt_type) = match pattern {
        TestingAntiPattern::TestWithoutAssertions {
            test_name,
            suggested_assertions,
            line,
            ..
        } => (
            Priority::High,
            format!("Test '{}' has no assertions", test_name),
            Some(format!(
                "Add assertions: {}",
                suggested_assertions.join(", ")
            )),
            line,
            DebtType::TestQuality,
        ),
        TestingAntiPattern::OverlyComplexTest {
            test_name,
            complexity_score,
            suggested_simplification,
            line,
            ..
        } => (
            Priority::Medium,
            format!(
                "Test '{}' is overly complex (score: {})",
                test_name, complexity_score
            ),
            Some(format!("Consider: {:?}", suggested_simplification)),
            line,
            DebtType::TestComplexity,
        ),
        TestingAntiPattern::FlakyTestPattern {
            test_name,
            flakiness_type,
            stabilization_suggestion,
            line,
            ..
        } => (
            Priority::High,
            format!(
                "Test '{}' has flaky pattern: {:?}",
                test_name, flakiness_type
            ),
            Some(stabilization_suggestion),
            line,
            DebtType::TestQuality,
        ),
    };

    DebtItem {
        id: format!("testing-{}-{}", path.display(), line),
        debt_type,
        priority,
        file: path.clone(),
        line,
        message,
        context,
    }
}
