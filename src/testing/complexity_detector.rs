use super::{
    is_test_function, ComplexitySource, TestQualityImpact, TestSimplification, TestingAntiPattern,
    TestingDetector,
};
use std::path::Path;
use syn::visit::Visit;
use syn::{Block, Expr, ExprCall, ExprIf, ExprLoop, ExprMatch, ExprMethodCall, File, Item, ItemFn};

pub struct TestComplexityDetector {
    pub(crate) max_test_complexity: u32,
    pub(crate) max_mock_setups: usize,
    pub(crate) max_test_length: usize,
}

impl Default for TestComplexityDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl TestComplexityDetector {
    pub fn new() -> Self {
        Self {
            max_test_complexity: 10,
            max_mock_setups: 5,
            max_test_length: 50,
        }
    }
}

impl TestingDetector for TestComplexityDetector {
    fn detect_anti_patterns(&self, file: &File, path: &Path) -> Vec<TestingAntiPattern> {
        let mut patterns = Vec::new();

        for item in &file.items {
            if let Item::Fn(function) = item {
                if is_test_function(function) {
                    let analysis = analyze_test_complexity(function);

                    if is_overly_complex(&analysis, self) {
                        let line = function.sig.ident.span().start().line;

                        patterns.push(TestingAntiPattern::OverlyComplexTest {
                            test_name: function.sig.ident.to_string(),
                            file: path.to_path_buf(),
                            line,
                            complexity_score: analysis.total_complexity,
                            complexity_sources: analysis.sources.clone(),
                            suggested_simplification: suggest_simplification(&analysis, self),
                        });
                    }
                }
            }

            // Also check test modules
            if let Item::Mod(module) = item {
                if let Some((_, items)) = &module.content {
                    for mod_item in items {
                        if let Item::Fn(function) = mod_item {
                            if is_test_function(function) {
                                let analysis = analyze_test_complexity(function);

                                if is_overly_complex(&analysis, self) {
                                    let line = function.sig.ident.span().start().line;

                                    patterns.push(TestingAntiPattern::OverlyComplexTest {
                                        test_name: function.sig.ident.to_string(),
                                        file: path.to_path_buf(),
                                        line,
                                        complexity_score: analysis.total_complexity,
                                        complexity_sources: analysis.sources.clone(),
                                        suggested_simplification: suggest_simplification(
                                            &analysis, self,
                                        ),
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }

        patterns
    }

    fn detector_name(&self) -> &'static str {
        "TestComplexityDetector"
    }

    fn assess_test_quality_impact(&self, pattern: &TestingAntiPattern) -> TestQualityImpact {
        match pattern {
            TestingAntiPattern::OverlyComplexTest {
                complexity_score, ..
            } => {
                if *complexity_score > 20 {
                    TestQualityImpact::High
                } else {
                    TestQualityImpact::Medium
                }
            }
            _ => TestQualityImpact::Medium,
        }
    }
}

#[derive(Debug, Default, Clone)]
pub(crate) struct TestComplexityAnalysis {
    pub(crate) cyclomatic_complexity: u32,
    pub(crate) mock_setup_count: usize,
    pub(crate) line_count: usize,
    pub(crate) assertion_complexity: u32,
    pub(crate) total_complexity: u32,
    pub(crate) sources: Vec<ComplexitySource>,
    pub(crate) has_loops: bool,
    pub(crate) has_nested_conditionals: bool,
    pub(crate) assertion_count: usize,
}

struct ComplexityAnalyzer {
    analysis: TestComplexityAnalysis,
    nesting_level: usize,
}

impl ComplexityAnalyzer {
    fn new() -> Self {
        Self {
            analysis: TestComplexityAnalysis::default(),
            nesting_level: 0,
        }
    }
}

impl<'ast> Visit<'ast> for ComplexityAnalyzer {
    fn visit_item_fn(&mut self, node: &'ast ItemFn) {
        // Count lines in the function
        self.analysis.line_count = count_lines_in_block(&node.block);

        // Visit the function body
        syn::visit::visit_item_fn(self, node);

        // Calculate total complexity after visiting
        self.analysis.total_complexity = calculate_total_complexity(&self.analysis);

        // Identify complexity sources
        if self.analysis.mock_setup_count > 3 {
            self.analysis
                .sources
                .push(ComplexitySource::ExcessiveMocking);
        }
        if self.analysis.has_nested_conditionals {
            self.analysis
                .sources
                .push(ComplexitySource::NestedConditionals);
        }
        if self.analysis.assertion_count > 5 {
            self.analysis
                .sources
                .push(ComplexitySource::MultipleAssertions);
        }
        if self.analysis.has_loops {
            self.analysis.sources.push(ComplexitySource::LoopInTest);
        }
        if self.analysis.line_count > 30 {
            self.analysis.sources.push(ComplexitySource::ExcessiveSetup);
        }
    }

    fn visit_expr_if(&mut self, node: &'ast ExprIf) {
        self.analysis.cyclomatic_complexity += 1;

        self.nesting_level += 1;
        if self.nesting_level > 1 {
            self.analysis.has_nested_conditionals = true;
        }

        syn::visit::visit_expr_if(self, node);
        self.nesting_level -= 1;
    }

    fn visit_expr_match(&mut self, node: &'ast ExprMatch) {
        // Match adds complexity based on number of arms
        let arms_count = node.arms.len();
        if arms_count > 1 {
            self.analysis.cyclomatic_complexity += (arms_count - 1) as u32;
        }

        self.nesting_level += 1;
        if self.nesting_level > 1 {
            self.analysis.has_nested_conditionals = true;
        }

        syn::visit::visit_expr_match(self, node);
        self.nesting_level -= 1;
    }

    fn visit_expr_loop(&mut self, node: &'ast ExprLoop) {
        self.analysis.cyclomatic_complexity += 1;
        self.analysis.has_loops = true;

        syn::visit::visit_expr_loop(self, node);
    }

    fn visit_expr_call(&mut self, node: &'ast ExprCall) {
        if let Expr::Path(path) = &*node.func {
            let func_name = path
                .path
                .segments
                .last()
                .map(|seg| seg.ident.to_string())
                .unwrap_or_default();

            if is_mock_setup_call(&func_name) {
                self.analysis.mock_setup_count += 1;
            }

            if is_assertion_call(&func_name) {
                self.analysis.assertion_count += 1;
            }
        }

        syn::visit::visit_expr_call(self, node);
    }

    fn visit_expr_method_call(&mut self, node: &'ast ExprMethodCall) {
        let method_name = node.method.to_string();

        if is_mock_method_call(&method_name) {
            self.analysis.mock_setup_count += 1;
        }

        syn::visit::visit_expr_method_call(self, node);
    }

    fn visit_expr(&mut self, node: &'ast Expr) {
        // Check for complex boolean expressions in assertions
        if let Expr::Binary(binary) = node {
            use syn::BinOp;
            match binary.op {
                BinOp::And(_) | BinOp::Or(_) => {
                    self.analysis.assertion_complexity += 1;
                }
                _ => {}
            }
        }

        syn::visit::visit_expr(self, node);
    }
}

pub(crate) fn analyze_test_complexity(function: &ItemFn) -> TestComplexityAnalysis {
    let mut analyzer = ComplexityAnalyzer::new();
    analyzer.visit_item_fn(function);
    analyzer.analysis
}

pub(crate) fn count_lines_in_block(block: &Block) -> usize {
    // Simple line count based on statements
    // In real implementation, we'd use span information
    block.stmts.len()
}

pub(crate) fn is_mock_setup_call(name: &str) -> bool {
    let mock_patterns = [
        "mock",
        "when",
        "given",
        "expect",
        "stub",
        "fake",
        "with_return",
        "returns",
        "with_args",
        "times",
        "Mock",
        "Stub",
        "Fake",
        "Double",
    ];

    mock_patterns.iter().any(|pattern| name.contains(pattern))
}

pub(crate) fn is_mock_method_call(name: &str) -> bool {
    let mock_methods = [
        "expect",
        "times",
        "returning",
        "with",
        "withf",
        "return_once",
        "return_const",
        "never",
        "once",
    ];

    mock_methods.contains(&name)
}

pub(crate) fn is_assertion_call(name: &str) -> bool {
    name.starts_with("assert") || name == "panic" || name == "expect"
}

pub(crate) fn calculate_total_complexity(analysis: &TestComplexityAnalysis) -> u32 {
    analysis.cyclomatic_complexity
        + (analysis.mock_setup_count as u32 * 2)
        + analysis.assertion_complexity
        + (analysis.line_count as u32 / 10) // Penalty for long tests
}

pub(crate) fn is_overly_complex(
    analysis: &TestComplexityAnalysis,
    detector: &TestComplexityDetector,
) -> bool {
    analysis.total_complexity > detector.max_test_complexity
        || analysis.mock_setup_count > detector.max_mock_setups
        || analysis.line_count > detector.max_test_length
}

pub(crate) fn suggest_simplification(
    analysis: &TestComplexityAnalysis,
    detector: &TestComplexityDetector,
) -> TestSimplification {
    if analysis.mock_setup_count > detector.max_mock_setups {
        TestSimplification::ReduceMocking
    } else if analysis.line_count > detector.max_test_length {
        if analysis.assertion_count > 3 && analysis.mock_setup_count > 3 {
            TestSimplification::SplitTest
        } else {
            TestSimplification::ExtractHelper
        }
    } else if analysis.cyclomatic_complexity > 5 {
        TestSimplification::ParameterizeTest
    } else {
        TestSimplification::SimplifySetup
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn test_is_mock_setup_call_positive_cases() {
        assert!(is_mock_setup_call("mock_service"));
        assert!(is_mock_setup_call("when_called"));
        assert!(is_mock_setup_call("given_input"));
        assert!(is_mock_setup_call("expect_call"));
        assert!(is_mock_setup_call("stub_response"));
        assert!(is_mock_setup_call("fake_impl"));
        assert!(is_mock_setup_call("with_return_value"));
        assert!(is_mock_setup_call("returns_value"));
        assert!(is_mock_setup_call("with_args_matching"));
        assert!(is_mock_setup_call("times_called"));
        assert!(is_mock_setup_call("MockService"));
        assert!(is_mock_setup_call("StubRepository"));
        assert!(is_mock_setup_call("FakeDatabase"));
        assert!(is_mock_setup_call("DoubleClient"));
    }

    #[test]
    fn test_is_mock_setup_call_negative_cases() {
        assert!(!is_mock_setup_call("regular_function"));
        assert!(!is_mock_setup_call("process_data"));
        assert!(!is_mock_setup_call("calculate_result"));
        assert!(!is_mock_setup_call("validate_input"));
        assert!(!is_mock_setup_call("transform_output"));
    }

    #[test]
    fn test_is_mock_setup_call_edge_cases() {
        assert!(is_mock_setup_call("mockito"));
        assert!(!is_mock_setup_call("MOCK_CONSTANT")); // Doesn't match - all uppercase
        assert!(is_mock_setup_call("create_mock"));
        assert!(!is_mock_setup_call(""));
        assert!(!is_mock_setup_call("m"));
    }

    #[test]
    fn test_is_mock_method_call_positive_cases() {
        assert!(is_mock_method_call("expect"));
        assert!(is_mock_method_call("times"));
        assert!(is_mock_method_call("returning"));
        assert!(is_mock_method_call("with"));
        assert!(is_mock_method_call("withf"));
        assert!(is_mock_method_call("return_once"));
        assert!(is_mock_method_call("return_const"));
        assert!(is_mock_method_call("never"));
        assert!(is_mock_method_call("once"));
    }

    #[test]
    fn test_is_mock_method_call_negative_cases() {
        assert!(!is_mock_method_call("execute"));
        assert!(!is_mock_method_call("process"));
        assert!(!is_mock_method_call("validate"));
        assert!(!is_mock_method_call("transform"));
        assert!(!is_mock_method_call("calculate"));
        assert!(!is_mock_method_call(""));
    }

    #[test]
    fn test_is_assertion_call_positive_cases() {
        assert!(is_assertion_call("assert"));
        assert!(is_assertion_call("assert_eq"));
        assert!(is_assertion_call("assert_ne"));
        assert!(is_assertion_call("assert_matches"));
        assert!(is_assertion_call("panic"));
        assert!(is_assertion_call("expect"));
    }

    #[test]
    fn test_is_assertion_call_negative_cases() {
        assert!(!is_assertion_call("process"));
        assert!(!is_assertion_call("validate"));
        assert!(!is_assertion_call("transform"));
        assert!(!is_assertion_call("execute"));
        assert!(!is_assertion_call(""));
    }

    #[test]
    fn test_calculate_total_complexity_basic() {
        let analysis = TestComplexityAnalysis {
            cyclomatic_complexity: 5,
            mock_setup_count: 0,
            assertion_complexity: 0,
            line_count: 10,
            ..Default::default()
        };

        assert_eq!(calculate_total_complexity(&analysis), 6); // 5 + 0 + 0 + 1
    }

    #[test]
    fn test_calculate_total_complexity_with_mocks() {
        let analysis = TestComplexityAnalysis {
            cyclomatic_complexity: 3,
            mock_setup_count: 2,
            assertion_complexity: 1,
            line_count: 15,
            ..Default::default()
        };

        assert_eq!(calculate_total_complexity(&analysis), 9); // 3 + 4 + 1 + 1
    }

    #[test]
    fn test_calculate_total_complexity_long_test() {
        let analysis = TestComplexityAnalysis {
            cyclomatic_complexity: 2,
            mock_setup_count: 1,
            assertion_complexity: 0,
            line_count: 50,
            ..Default::default()
        };

        assert_eq!(calculate_total_complexity(&analysis), 9); // 2 + 2 + 0 + 5
    }

    #[test]
    fn test_is_overly_complex_by_total_complexity() {
        let detector = TestComplexityDetector::new();
        let analysis = TestComplexityAnalysis {
            total_complexity: 11,
            mock_setup_count: 3,
            line_count: 30,
            ..Default::default()
        };

        assert!(is_overly_complex(&analysis, &detector));
    }

    #[test]
    fn test_is_overly_complex_by_mock_count() {
        let detector = TestComplexityDetector::new();
        let analysis = TestComplexityAnalysis {
            total_complexity: 5,
            mock_setup_count: 6,
            line_count: 30,
            ..Default::default()
        };

        assert!(is_overly_complex(&analysis, &detector));
    }

    #[test]
    fn test_is_overly_complex_by_line_count() {
        let detector = TestComplexityDetector::new();
        let analysis = TestComplexityAnalysis {
            total_complexity: 5,
            mock_setup_count: 3,
            line_count: 51,
            ..Default::default()
        };

        assert!(is_overly_complex(&analysis, &detector));
    }

    #[test]
    fn test_is_not_overly_complex() {
        let detector = TestComplexityDetector::new();
        let analysis = TestComplexityAnalysis {
            total_complexity: 8,
            mock_setup_count: 3,
            line_count: 30,
            ..Default::default()
        };

        assert!(!is_overly_complex(&analysis, &detector));
    }

    #[test]
    fn test_suggest_simplification_reduce_mocking() {
        let detector = TestComplexityDetector::new();
        let analysis = TestComplexityAnalysis {
            mock_setup_count: 6,
            line_count: 30,
            assertion_count: 2,
            cyclomatic_complexity: 3,
            ..Default::default()
        };

        assert!(matches!(
            suggest_simplification(&analysis, &detector),
            TestSimplification::ReduceMocking
        ));
    }

    #[test]
    fn test_suggest_simplification_split_test() {
        let detector = TestComplexityDetector::new();
        let analysis = TestComplexityAnalysis {
            mock_setup_count: 4,
            line_count: 60,
            assertion_count: 5,
            cyclomatic_complexity: 3,
            ..Default::default()
        };

        assert!(matches!(
            suggest_simplification(&analysis, &detector),
            TestSimplification::SplitTest
        ));
    }

    #[test]
    fn test_suggest_simplification_extract_helper() {
        let detector = TestComplexityDetector::new();
        let analysis = TestComplexityAnalysis {
            mock_setup_count: 2,
            line_count: 55,
            assertion_count: 2,
            cyclomatic_complexity: 3,
            ..Default::default()
        };

        assert!(matches!(
            suggest_simplification(&analysis, &detector),
            TestSimplification::ExtractHelper
        ));
    }

    #[test]
    fn test_suggest_simplification_parameterize() {
        let detector = TestComplexityDetector::new();
        let analysis = TestComplexityAnalysis {
            mock_setup_count: 3,
            line_count: 40,
            assertion_count: 2,
            cyclomatic_complexity: 6,
            ..Default::default()
        };

        assert!(matches!(
            suggest_simplification(&analysis, &detector),
            TestSimplification::ParameterizeTest
        ));
    }

    #[test]
    fn test_suggest_simplification_simplify_setup() {
        let detector = TestComplexityDetector::new();
        let analysis = TestComplexityAnalysis {
            mock_setup_count: 3,
            line_count: 40,
            assertion_count: 2,
            cyclomatic_complexity: 3,
            ..Default::default()
        };

        assert!(matches!(
            suggest_simplification(&analysis, &detector),
            TestSimplification::SimplifySetup
        ));
    }

    #[test]
    fn test_count_lines_in_block() {
        let block: syn::Block = parse_quote! {{
            let x = 1;
            let y = 2;
            assert_eq!(x + y, 3);
        }};

        assert_eq!(count_lines_in_block(&block), 3);
    }

    #[test]
    fn test_count_lines_in_empty_block() {
        let block: syn::Block = parse_quote! {{}};

        assert_eq!(count_lines_in_block(&block), 0);
    }

    #[test]
    fn test_test_complexity_detector_default() {
        let detector = TestComplexityDetector::default();
        assert_eq!(detector.max_test_complexity, 10);
        assert_eq!(detector.max_mock_setups, 5);
        assert_eq!(detector.max_test_length, 50);
    }
}
