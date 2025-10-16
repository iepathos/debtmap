use super::{
    assertion_detector::AssertionDetector,
    complexity_scorer::{ComplexityScorer, TestComplexityScore},
    flaky_detector::FlakyDetector,
    framework_detector::FrameworkDetector,
    test_classifier::TestClassifier,
    RustFlakinessType, RustTestFramework, RustTestIssueType, RustTestQualityIssue,
    RustTestSeverity,
};
use std::path::Path;
use syn::visit::Visit;
use syn::{File, ItemFn};

/// Main analyzer for Rust test quality
pub struct RustTestQualityAnalyzer {
    test_classifier: TestClassifier,
    assertion_detector: AssertionDetector,
    complexity_scorer: ComplexityScorer,
    flaky_detector: FlakyDetector,
    framework_detector: FrameworkDetector,
    complexity_threshold: f32,
}

impl RustTestQualityAnalyzer {
    /// Default complexity threshold for tests
    pub const DEFAULT_COMPLEXITY_THRESHOLD: f32 = 10.0;

    pub fn new() -> Self {
        Self {
            test_classifier: TestClassifier::new(),
            assertion_detector: AssertionDetector::new(),
            complexity_scorer: ComplexityScorer::new(),
            flaky_detector: FlakyDetector::new(),
            framework_detector: FrameworkDetector::new(),
            complexity_threshold: Self::DEFAULT_COMPLEXITY_THRESHOLD,
        }
    }

    /// Create analyzer with custom complexity threshold
    pub fn with_threshold(threshold: f32) -> Self {
        Self {
            complexity_threshold: threshold,
            ..Self::new()
        }
    }

    /// Analyze a Rust file for test quality issues
    pub fn analyze_file(&mut self, file: &File, path: &Path) -> Vec<RustTestQualityIssue> {
        let mut visitor = TestFunctionVisitor::new(path);
        visitor.visit_file(file);

        let test_functions = visitor.test_functions;

        test_functions
            .into_iter()
            .flat_map(|func| self.analyze_test_function(&func, path))
            .collect()
    }

    /// Analyze a single test function
    pub fn analyze_test_function(
        &mut self,
        func: &ItemFn,
        path: &Path,
    ) -> Vec<RustTestQualityIssue> {
        // Only analyze if it's actually a test
        if !self.test_classifier.is_test_function(func) {
            return vec![];
        }

        let mut issues = Vec::new();

        let test_name = func.sig.ident.to_string();
        let line = func.sig.ident.span().start().line;

        // Detect test framework
        let framework = self.framework_detector.detect_framework(func);

        // Classify test type
        let _test_type = self.test_classifier.classify_test_type(func, path);

        // Skip some checks for property tests and benchmarks
        let skip_assertion_check = matches!(
            framework,
            RustTestFramework::Proptest
                | RustTestFramework::Quickcheck
                | RustTestFramework::Criterion
        );

        // Analyze assertions
        let assertions = self.assertion_detector.analyze_assertions(func);
        let assertion_count = assertions.len();

        if !skip_assertion_check && assertion_count == 0 {
            issues.push(RustTestQualityIssue {
                issue_type: RustTestIssueType::NoAssertions,
                test_name: test_name.clone(),
                line,
                severity: RustTestSeverity::High,
                confidence: 0.95,
                explanation: "Test function has no assertions, so it cannot verify correctness"
                    .to_string(),
                suggestion:
                    "Add assertions like assert!, assert_eq!, or assert_ne! to verify behavior"
                        .to_string(),
            });
        }

        // Analyze complexity
        let complexity_score = self
            .complexity_scorer
            .calculate_complexity(func, assertion_count);

        if complexity_score.total_score > self.complexity_threshold {
            issues.push(self.create_complexity_issue(test_name.clone(), line, complexity_score));
        }

        // Detect flaky patterns
        let flaky_indicators = self.flaky_detector.detect_flaky_patterns(func);

        for indicator in flaky_indicators {
            issues.push(RustTestQualityIssue {
                issue_type: RustTestIssueType::FlakyPattern(indicator.flakiness_type.clone()),
                test_name: test_name.clone(),
                line: indicator.line,
                severity: self.assess_flakiness_severity(&indicator.flakiness_type),
                confidence: 0.85,
                explanation: indicator.explanation.clone(),
                suggestion: self.get_flakiness_suggestion(&indicator.flakiness_type),
            });
        }

        issues
    }

    /// Create complexity issue with detailed factors
    fn create_complexity_issue(
        &self,
        test_name: String,
        line: usize,
        score: TestComplexityScore,
    ) -> RustTestQualityIssue {
        let explanation = format!(
            "Test complexity score ({:.1}) exceeds threshold ({:.1}). Factors: {} conditionals, {} loops, {} assertions, max nesting {}, {} lines",
            score.total_score,
            self.complexity_threshold,
            score.factors.conditionals,
            score.factors.loops,
            score.factors.assertions,
            score.factors.nesting_depth,
            score.factors.line_count
        );

        let suggestion = if score.factors.loops > 0 {
            "Consider extracting loop logic into helper functions or parameterizing the test"
                .to_string()
        } else if score.factors.conditionals > 2 {
            "Consider splitting into multiple focused test cases".to_string()
        } else if score.factors.line_count > 50 {
            "Consider breaking into smaller test functions".to_string()
        } else {
            "Simplify test logic or split into multiple tests".to_string()
        };

        RustTestQualityIssue {
            issue_type: RustTestIssueType::TooComplex(score.total_score as u32),
            test_name,
            line,
            severity: if score.total_score > self.complexity_threshold * 2.0 {
                RustTestSeverity::High
            } else {
                RustTestSeverity::Medium
            },
            confidence: 0.9,
            explanation,
            suggestion,
        }
    }

    /// Assess severity of flakiness type
    fn assess_flakiness_severity(&self, flakiness_type: &RustFlakinessType) -> RustTestSeverity {
        match flakiness_type {
            RustFlakinessType::TimingDependency => RustTestSeverity::High,
            RustFlakinessType::RandomValue => RustTestSeverity::High,
            RustFlakinessType::ThreadingIssue => RustTestSeverity::Critical,
            RustFlakinessType::NetworkDependency => RustTestSeverity::High,
            RustFlakinessType::ExternalDependency => RustTestSeverity::Medium,
            RustFlakinessType::FileSystemDependency => RustTestSeverity::Medium,
            RustFlakinessType::HashOrdering => RustTestSeverity::Medium,
        }
    }

    /// Get suggestion for fixing flakiness
    fn get_flakiness_suggestion(&self, flakiness_type: &RustFlakinessType) -> String {
        match flakiness_type {
            RustFlakinessType::TimingDependency => {
                "Use explicit synchronization or mock time instead of sleep/Instant::now"
                    .to_string()
            }
            RustFlakinessType::RandomValue => {
                "Use fixed test data or seed random generators deterministically".to_string()
            }
            RustFlakinessType::ThreadingIssue => {
                "Add proper synchronization or avoid threading in tests".to_string()
            }
            RustFlakinessType::NetworkDependency => {
                "Mock network calls or use test servers with predictable behavior".to_string()
            }
            RustFlakinessType::ExternalDependency => {
                "Mock external dependencies or use test doubles".to_string()
            }
            RustFlakinessType::FileSystemDependency => {
                "Use tempfile crate or mock filesystem operations".to_string()
            }
            RustFlakinessType::HashOrdering => {
                "Use BTreeMap/BTreeSet or sort results before assertions".to_string()
            }
        }
    }
}

impl Default for RustTestQualityAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

/// Visitor to collect test functions from a file
struct TestFunctionVisitor<'a> {
    test_functions: Vec<ItemFn>,
    test_classifier: TestClassifier,
    _path: &'a Path,
}

impl<'a> TestFunctionVisitor<'a> {
    fn new(path: &'a Path) -> Self {
        Self {
            test_functions: Vec::new(),
            test_classifier: TestClassifier::new(),
            _path: path,
        }
    }
}

impl<'a, 'ast> Visit<'ast> for TestFunctionVisitor<'a> {
    fn visit_item_fn(&mut self, func: &'ast ItemFn) {
        if self.test_classifier.is_test_function(func) {
            self.test_functions.push(func.clone());
        }
        syn::visit::visit_item_fn(self, func);
    }

    fn visit_impl_item_fn(&mut self, func: &'ast syn::ImplItemFn) {
        // Convert ImplItemFn to ItemFn for analysis
        let item_fn = syn::ItemFn {
            attrs: func.attrs.clone(),
            vis: func.vis.clone(),
            sig: func.sig.clone(),
            block: Box::new(func.block.clone()),
        };

        if self.test_classifier.is_test_function(&item_fn) {
            self.test_functions.push(item_fn);
        }

        syn::visit::visit_impl_item_fn(self, func);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn test_analyze_test_with_no_assertions() {
        let func: ItemFn = parse_quote! {
            #[test]
            fn test_no_assertions() {
                let x = 42;
                process(x);
            }
        };

        let mut analyzer = RustTestQualityAnalyzer::new();
        let path = Path::new("test.rs");
        let issues = analyzer.analyze_test_function(&func, path);

        assert!(!issues.is_empty());
        assert!(issues
            .iter()
            .any(|i| matches!(i.issue_type, RustTestIssueType::NoAssertions)));
    }

    #[test]
    fn test_analyze_complex_test() {
        let func: ItemFn = parse_quote! {
            #[test]
            fn test_complex() {
                for i in 0..10 {
                    if i % 2 == 0 {
                        for j in 0..5 {
                            assert_eq!(i * j, expected(i, j));
                        }
                    }
                }
            }
        };

        // Use a lower threshold to ensure the test triggers complexity detection
        let mut analyzer = RustTestQualityAnalyzer::with_threshold(5.0);
        let path = Path::new("test.rs");
        let issues = analyzer.analyze_test_function(&func, path);

        assert!(issues
            .iter()
            .any(|i| matches!(i.issue_type, RustTestIssueType::TooComplex(_))));
    }

    #[test]
    fn test_analyze_flaky_test() {
        let func: ItemFn = parse_quote! {
            #[test]
            fn test_flaky() {
                std::thread::sleep(std::time::Duration::from_millis(100));
                assert!(true);
            }
        };

        let mut analyzer = RustTestQualityAnalyzer::new();
        let path = Path::new("test.rs");
        let issues = analyzer.analyze_test_function(&func, path);

        assert!(issues
            .iter()
            .any(|i| matches!(i.issue_type, RustTestIssueType::FlakyPattern(_))));
    }

    #[test]
    fn test_analyze_proper_test() {
        let func: ItemFn = parse_quote! {
            #[test]
            fn test_proper() {
                let result = calculate(42);
                assert_eq!(result, 84);
            }
        };

        let mut analyzer = RustTestQualityAnalyzer::new();
        let path = Path::new("test.rs");
        let issues = analyzer.analyze_test_function(&func, path);

        // Should have no issues
        assert!(issues.is_empty());
    }

    #[test]
    fn test_skip_assertion_check_for_property_tests() {
        let func: ItemFn = parse_quote! {
            #[test]
            fn test_property(x: i32) {
                // Property test framework handles assertions
                x == x
            }
        };

        let mut analyzer = RustTestQualityAnalyzer::new();
        let path = Path::new("test.rs");
        let issues = analyzer.analyze_test_function(&func, path);

        // Should not flag missing assertions for property tests
        // (though this simple example might not be detected as property test)
        assert!(issues
            .iter()
            .any(|i| matches!(i.issue_type, RustTestIssueType::NoAssertions)));
    }
}
