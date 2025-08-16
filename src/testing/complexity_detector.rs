use super::{
    is_test_function, ComplexitySource, TestQualityImpact,
    TestSimplification, TestingAntiPattern, TestingDetector,
};
use std::path::PathBuf;
use syn::visit::Visit;
use syn::{Block, Expr, ExprCall, ExprIf, ExprLoop, ExprMatch, ExprMethodCall, File, Item, ItemFn};

pub struct TestComplexityDetector {
    max_test_complexity: u32,
    max_mock_setups: usize,
    max_test_length: usize,
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
    fn detect_anti_patterns(&self, file: &File, path: &PathBuf) -> Vec<TestingAntiPattern> {
        let mut patterns = Vec::new();

        for item in &file.items {
            if let Item::Fn(function) = item {
                if is_test_function(function) {
                    let analysis = analyze_test_complexity(function);

                    if is_overly_complex(&analysis, self) {
                        let line = function
                            .sig
                            .ident
                            .span()
                            .start()
                            .line;

                        patterns.push(TestingAntiPattern::OverlyComplexTest {
                            test_name: function.sig.ident.to_string(),
                            file: path.clone(),
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
                                    let line = function
                                        .sig
                                        .ident
                                        .span()
                                        .start()
                                        .line;

                                    patterns.push(TestingAntiPattern::OverlyComplexTest {
                                        test_name: function.sig.ident.to_string(),
                                        file: path.clone(),
                                        line,
                                        complexity_score: analysis.total_complexity,
                                        complexity_sources: analysis.sources.clone(),
                                        suggested_simplification: suggest_simplification(&analysis, self),
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
            TestingAntiPattern::OverlyComplexTest { complexity_score, .. } => {
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
struct TestComplexityAnalysis {
    cyclomatic_complexity: u32,
    mock_setup_count: usize,
    line_count: usize,
    assertion_complexity: u32,
    total_complexity: u32,
    sources: Vec<ComplexitySource>,
    has_loops: bool,
    has_nested_conditionals: bool,
    assertion_count: usize,
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
            self.analysis.sources.push(ComplexitySource::ExcessiveMocking);
        }
        if self.analysis.has_nested_conditionals {
            self.analysis.sources.push(ComplexitySource::NestedConditionals);
        }
        if self.analysis.assertion_count > 5 {
            self.analysis.sources.push(ComplexitySource::MultipleAssertions);
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
        match node {
            Expr::Binary(binary) => {
                use syn::BinOp;
                match binary.op {
                    BinOp::And(_) | BinOp::Or(_) => {
                        self.analysis.assertion_complexity += 1;
                    }
                    _ => {}
                }
            }
            _ => {}
        }

        syn::visit::visit_expr(self, node);
    }
}

fn analyze_test_complexity(function: &ItemFn) -> TestComplexityAnalysis {
    let mut analyzer = ComplexityAnalyzer::new();
    analyzer.visit_item_fn(function);
    analyzer.analysis
}

fn count_lines_in_block(block: &Block) -> usize {
    // Simple line count based on statements
    // In real implementation, we'd use span information
    block.stmts.len()
}

fn is_mock_setup_call(name: &str) -> bool {
    let mock_patterns = [
        "mock", "when", "given", "expect", "stub", "fake",
        "with_return", "returns", "with_args", "times",
        "Mock", "Stub", "Fake", "Double"
    ];

    mock_patterns.iter().any(|pattern| name.contains(pattern))
}

fn is_mock_method_call(name: &str) -> bool {
    let mock_methods = [
        "expect", "times", "returning", "with", "withf",
        "return_once", "return_const", "never", "once"
    ];

    mock_methods.contains(&name.as_ref())
}

fn is_assertion_call(name: &str) -> bool {
    name.starts_with("assert") || name == "panic" || name == "expect"
}

fn calculate_total_complexity(analysis: &TestComplexityAnalysis) -> u32 {
    analysis.cyclomatic_complexity
        + (analysis.mock_setup_count as u32 * 2)
        + analysis.assertion_complexity
        + (analysis.line_count as u32 / 10) // Penalty for long tests
}

fn is_overly_complex(analysis: &TestComplexityAnalysis, detector: &TestComplexityDetector) -> bool {
    analysis.total_complexity > detector.max_test_complexity
        || analysis.mock_setup_count > detector.max_mock_setups
        || analysis.line_count > detector.max_test_length
}

fn suggest_simplification(
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