use super::{is_test_function, TestQualityImpact, TestingAntiPattern, TestingDetector};
use std::path::Path;
use syn::visit::Visit;
use syn::{Expr, ExprCall, ExprMacro, ExprMethodCall, File, Item, ItemFn, Stmt};

pub struct AssertionDetector {}

impl Default for AssertionDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl AssertionDetector {
    pub fn new() -> Self {
        Self {}
    }
}

impl TestingDetector for AssertionDetector {
    fn detect_anti_patterns(&self, file: &File, path: &Path) -> Vec<TestingAntiPattern> {
        let mut patterns = Vec::new();

        for item in &file.items {
            if let Item::Fn(function) = item {
                if is_test_function(function) {
                    let analysis = analyze_test_structure(function);

                    if !analysis.has_assertions {
                        let line = function.sig.ident.span().start().line;

                        patterns.push(TestingAntiPattern::TestWithoutAssertions {
                            test_name: function.sig.ident.to_string(),
                            file: path.to_path_buf(),
                            line,
                            has_setup: analysis.has_setup,
                            has_action: analysis.has_action,
                            suggested_assertions: suggest_assertions(&analysis),
                        });
                    }
                }
            }

            // Also check for test modules
            if let Item::Mod(module) = item {
                if let Some((_, items)) = &module.content {
                    for mod_item in items {
                        if let Item::Fn(function) = mod_item {
                            if is_test_function(function) {
                                let analysis = analyze_test_structure(function);

                                if !analysis.has_assertions {
                                    let line = function.sig.ident.span().start().line;

                                    patterns.push(TestingAntiPattern::TestWithoutAssertions {
                                        test_name: function.sig.ident.to_string(),
                                        file: path.to_path_buf(),
                                        line,
                                        has_setup: analysis.has_setup,
                                        has_action: analysis.has_action,
                                        suggested_assertions: suggest_assertions(&analysis),
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
        "AssertionDetector"
    }

    fn assess_test_quality_impact(&self, pattern: &TestingAntiPattern) -> TestQualityImpact {
        match pattern {
            TestingAntiPattern::TestWithoutAssertions { .. } => TestQualityImpact::Critical,
            _ => TestQualityImpact::Medium,
        }
    }
}

#[derive(Debug, Default)]
struct TestStructureAnalysis {
    has_setup: bool,
    has_action: bool,
    has_assertions: bool,
    assertion_count: usize,
    has_panic: bool,
    has_expect: bool,
    has_unwrap: bool,
}

struct TestAnalyzer {
    analysis: TestStructureAnalysis,
}

impl TestAnalyzer {
    fn new() -> Self {
        Self {
            analysis: TestStructureAnalysis::default(),
        }
    }
}

impl<'ast> Visit<'ast> for TestAnalyzer {
    fn visit_macro(&mut self, node: &'ast syn::Macro) {
        let macro_name = node
            .path
            .segments
            .last()
            .map(|seg| seg.ident.to_string())
            .unwrap_or_default();

        if is_assertion_macro(&macro_name) {
            self.analysis.has_assertions = true;
            self.analysis.assertion_count += 1;
        }

        if macro_name == "panic" {
            self.analysis.has_panic = true;
        }

        syn::visit::visit_macro(self, node);
    }

    fn visit_expr_macro(&mut self, node: &'ast ExprMacro) {
        let macro_name = node
            .mac
            .path
            .segments
            .last()
            .map(|seg| seg.ident.to_string())
            .unwrap_or_default();

        if is_assertion_macro(&macro_name) {
            self.analysis.has_assertions = true;
            self.analysis.assertion_count += 1;
        }

        if macro_name == "panic" {
            self.analysis.has_panic = true;
        }

        syn::visit::visit_expr_macro(self, node);
    }

    fn visit_expr_call(&mut self, node: &'ast ExprCall) {
        if let Expr::Path(path) = &*node.func {
            let func_name = path
                .path
                .segments
                .last()
                .map(|seg| seg.ident.to_string())
                .unwrap_or_default();

            if is_assertion_function(&func_name) {
                self.analysis.has_assertions = true;
                self.analysis.assertion_count += 1;
            }

            // Check for setup patterns
            if is_setup_function(&func_name) {
                self.analysis.has_setup = true;
            }
        }

        syn::visit::visit_expr_call(self, node);
    }

    fn visit_expr_method_call(&mut self, node: &'ast ExprMethodCall) {
        let method_name = node.method.to_string();

        // Check for assertion methods
        if method_name == "is_ok"
            || method_name == "is_err"
            || method_name == "is_some"
            || method_name == "is_none"
        {
            // Only count as assertion if used with assert!
            // This is a simplification - we'd need more context to be sure
        }

        // Check for expect/unwrap which can act as implicit assertions
        if method_name == "expect" {
            self.analysis.has_expect = true;
            self.analysis.has_assertions = true;
        }

        if method_name == "unwrap" {
            self.analysis.has_unwrap = true;
            // unwrap can act as an implicit assertion in tests
            self.analysis.has_assertions = true;
        }

        // Detect action patterns
        if !self.analysis.has_action {
            self.analysis.has_action = true;
        }

        syn::visit::visit_expr_method_call(self, node);
    }

    fn visit_stmt(&mut self, node: &'ast Stmt) {
        if let Stmt::Local(_) = node {
            if !self.analysis.has_setup {
                self.analysis.has_setup = true;
            }
        }

        syn::visit::visit_stmt(self, node);
    }
}

fn analyze_test_structure(function: &ItemFn) -> TestStructureAnalysis {
    let mut analyzer = TestAnalyzer::new();

    // Use the visitor trait to properly visit the entire function
    syn::visit::visit_item_fn(&mut analyzer, function);

    analyzer.analysis
}

fn is_assertion_macro(name: &str) -> bool {
    matches!(
        name,
        "assert"
            | "assert_eq"
            | "assert_ne"
            | "assert_matches"
            | "debug_assert"
            | "debug_assert_eq"
            | "debug_assert_ne"
    )
}

fn is_assertion_function(name: &str) -> bool {
    // Some test frameworks use functions instead of macros
    matches!(
        name,
        "assert" | "assert_eq" | "assert_ne" | "assert_that" | "expect"
    )
}

fn is_setup_function(name: &str) -> bool {
    name.starts_with("create_")
        || name.starts_with("new_")
        || name.starts_with("setup_")
        || name.starts_with("build_")
        || name == "new"
        || name == "default"
}

fn suggest_assertions(analysis: &TestStructureAnalysis) -> Vec<String> {
    let mut suggestions = Vec::new();

    if analysis.has_action && !analysis.has_assertions {
        suggestions.push("Add assertions to verify the behavior".to_string());
        suggestions.push("Consider using assert!, assert_eq!, or assert_ne!".to_string());
    }

    if analysis.has_setup && !analysis.has_action {
        suggestions.push("Add action phase - call the method under test".to_string());
    }

    if !analysis.has_setup && !analysis.has_action && !analysis.has_assertions {
        suggestions
            .push("Implement complete test structure: setup -> action -> assert".to_string());
    }

    if suggestions.is_empty() {
        suggestions.push("Verify that the test is checking expected behavior".to_string());
    }

    suggestions
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_suggest_assertions_for_action_without_assertions() {
        let analysis = TestStructureAnalysis {
            has_setup: false,
            has_action: true,
            has_assertions: false,
            assertion_count: 0,
            has_panic: false,
            has_expect: false,
            has_unwrap: false,
        };

        let suggestions = suggest_assertions(&analysis);

        assert_eq!(suggestions.len(), 2);
        assert_eq!(suggestions[0], "Add assertions to verify the behavior");
        assert_eq!(
            suggestions[1],
            "Consider using assert!, assert_eq!, or assert_ne!"
        );
    }

    #[test]
    fn test_suggest_assertions_for_setup_without_action() {
        let analysis = TestStructureAnalysis {
            has_setup: true,
            has_action: false,
            has_assertions: false,
            assertion_count: 0,
            has_panic: false,
            has_expect: false,
            has_unwrap: false,
        };

        let suggestions = suggest_assertions(&analysis);

        assert_eq!(suggestions.len(), 1);
        assert_eq!(
            suggestions[0],
            "Add action phase - call the method under test"
        );
    }

    #[test]
    fn test_suggest_assertions_for_empty_test() {
        let analysis = TestStructureAnalysis {
            has_setup: false,
            has_action: false,
            has_assertions: false,
            assertion_count: 0,
            has_panic: false,
            has_expect: false,
            has_unwrap: false,
        };

        let suggestions = suggest_assertions(&analysis);

        assert_eq!(suggestions.len(), 1);
        assert_eq!(
            suggestions[0],
            "Implement complete test structure: setup -> action -> assert"
        );
    }

    #[test]
    fn test_suggest_assertions_for_complete_test() {
        let analysis = TestStructureAnalysis {
            has_setup: true,
            has_action: true,
            has_assertions: true,
            assertion_count: 1,
            has_panic: false,
            has_expect: false,
            has_unwrap: false,
        };

        let suggestions = suggest_assertions(&analysis);

        assert_eq!(suggestions.len(), 1);
        assert_eq!(
            suggestions[0],
            "Verify that the test is checking expected behavior"
        );
    }

    #[test]
    fn test_suggest_assertions_for_setup_and_action_without_assertions() {
        let analysis = TestStructureAnalysis {
            has_setup: true,
            has_action: true,
            has_assertions: false,
            assertion_count: 0,
            has_panic: false,
            has_expect: false,
            has_unwrap: false,
        };

        let suggestions = suggest_assertions(&analysis);

        assert_eq!(suggestions.len(), 2);
        assert_eq!(suggestions[0], "Add assertions to verify the behavior");
        assert_eq!(
            suggestions[1],
            "Consider using assert!, assert_eq!, or assert_ne!"
        );
    }

    #[test]
    fn test_suggest_assertions_for_test_with_panic() {
        let analysis = TestStructureAnalysis {
            has_setup: false,
            has_action: true,
            has_assertions: false,
            assertion_count: 0,
            has_panic: true,
            has_expect: false,
            has_unwrap: false,
        };

        let suggestions = suggest_assertions(&analysis);

        assert_eq!(suggestions.len(), 2);
        assert_eq!(suggestions[0], "Add assertions to verify the behavior");
        assert_eq!(
            suggestions[1],
            "Consider using assert!, assert_eq!, or assert_ne!"
        );
    }

    #[test]
    fn test_suggest_assertions_for_test_with_expect() {
        let analysis = TestStructureAnalysis {
            has_setup: true,
            has_action: true,
            has_assertions: true,
            assertion_count: 0,
            has_panic: false,
            has_expect: true,
            has_unwrap: false,
        };

        let suggestions = suggest_assertions(&analysis);

        assert_eq!(suggestions.len(), 1);
        assert_eq!(
            suggestions[0],
            "Verify that the test is checking expected behavior"
        );
    }

    #[test]
    fn test_suggest_assertions_prioritizes_action_without_assertions() {
        let analysis = TestStructureAnalysis {
            has_setup: true,
            has_action: true,
            has_assertions: false,
            assertion_count: 0,
            has_panic: false,
            has_expect: false,
            has_unwrap: false,
        };

        let suggestions = suggest_assertions(&analysis);

        // Should only get the action without assertions suggestions, not the setup without action suggestion
        assert_eq!(suggestions.len(), 2);
        assert!(suggestions[0].contains("Add assertions"));
        assert!(!suggestions.iter().any(|s| s.contains("Add action phase")));
    }

    #[test]
    fn test_is_assertion_macro_recognizes_assert() {
        assert!(is_assertion_macro("assert"));
        assert!(is_assertion_macro("assert_eq"));
        assert!(is_assertion_macro("assert_ne"));
        assert!(is_assertion_macro("assert_matches"));
        assert!(is_assertion_macro("debug_assert"));
        assert!(is_assertion_macro("debug_assert_eq"));
        assert!(is_assertion_macro("debug_assert_ne"));
    }

    #[test]
    fn test_is_assertion_macro_rejects_non_assertions() {
        assert!(!is_assertion_macro("println"));
        assert!(!is_assertion_macro("panic"));
        assert!(!is_assertion_macro("format"));
        assert!(!is_assertion_macro("vec"));
        assert!(!is_assertion_macro(""));
        assert!(!is_assertion_macro("assert_foo"));
        assert!(!is_assertion_macro("my_assert"));
    }

    #[test]
    fn test_is_assertion_function_recognizes_assertions() {
        assert!(is_assertion_function("assert"));
        assert!(is_assertion_function("assert_eq"));
        assert!(is_assertion_function("assert_ne"));
        assert!(is_assertion_function("assert_that"));
        assert!(is_assertion_function("expect"));
    }

    #[test]
    fn test_is_assertion_function_rejects_non_assertions() {
        assert!(!is_assertion_function("test"));
        assert!(!is_assertion_function("run"));
        assert!(!is_assertion_function("check"));
        assert!(!is_assertion_function("verify"));
        assert!(!is_assertion_function(""));
        assert!(!is_assertion_function("assert_something"));
        assert!(!is_assertion_function("my_expect"));
    }

    #[test]
    fn test_is_setup_function_recognizes_setup_patterns() {
        assert!(is_setup_function("create_test_data"));
        assert!(is_setup_function("new_instance"));
        assert!(is_setup_function("setup_environment"));
        assert!(is_setup_function("build_context"));
        assert!(is_setup_function("new"));
        assert!(is_setup_function("default"));
    }

    #[test]
    fn test_is_setup_function_rejects_non_setup_functions() {
        assert!(!is_setup_function("test_something"));
        assert!(!is_setup_function("assert_equal"));
        assert!(!is_setup_function("run"));
        assert!(!is_setup_function("execute"));
        assert!(!is_setup_function(""));
        assert!(!is_setup_function("newer"));
        assert!(!is_setup_function("my_default"));
    }

    #[test]
    fn test_analyze_test_structure_simple_test() {
        use syn::parse_quote;

        let function: ItemFn = parse_quote! {
            #[test]
            fn test_simple() {
                let value = 42;
                assert_eq!(value, 42);
            }
        };

        let analysis = analyze_test_structure(&function);

        assert!(analysis.has_setup);
        assert!(analysis.has_assertions);
        assert_eq!(analysis.assertion_count, 1);
        assert!(!analysis.has_panic);
        assert!(!analysis.has_expect);
        assert!(!analysis.has_unwrap);
    }

    #[test]
    fn test_analyze_test_structure_test_without_assertions() {
        use syn::parse_quote;

        let function: ItemFn = parse_quote! {
            #[test]
            fn test_no_assertions() {
                let value = 42;
                let result = value * 2;
            }
        };

        let analysis = analyze_test_structure(&function);

        assert!(analysis.has_setup);
        assert!(!analysis.has_assertions);
        assert_eq!(analysis.assertion_count, 0);
        assert!(!analysis.has_panic);
        assert!(!analysis.has_expect);
        assert!(!analysis.has_unwrap);
    }

    #[test]
    fn test_analyze_test_structure_with_expect() {
        use syn::parse_quote;

        let function: ItemFn = parse_quote! {
            #[test]
            fn test_with_expect() {
                let result: Result<i32, &str> = Ok(42);
                let value = result.expect("Should have value");
            }
        };

        let analysis = analyze_test_structure(&function);

        assert!(analysis.has_setup);
        assert!(analysis.has_action);
        assert!(analysis.has_assertions);
        assert!(analysis.has_expect);
        assert!(!analysis.has_panic);
        assert!(!analysis.has_unwrap);
    }

    #[test]
    fn test_analyze_test_structure_with_unwrap() {
        use syn::parse_quote;

        let function: ItemFn = parse_quote! {
            #[test]
            fn test_with_unwrap() {
                let result: Result<i32, &str> = Ok(42);
                let value = result.unwrap();
            }
        };

        let analysis = analyze_test_structure(&function);

        assert!(analysis.has_setup);
        assert!(analysis.has_action);
        assert!(analysis.has_assertions);
        assert!(analysis.has_unwrap);
        assert!(!analysis.has_panic);
        assert!(!analysis.has_expect);
    }

    #[test]
    fn test_analyze_test_structure_with_multiple_assertions() {
        use syn::parse_quote;

        let function: ItemFn = parse_quote! {
            #[test]
            fn test_multiple_assertions() {
                let x = 10;
                let y = 20;
                assert!(x < y);
                assert_eq!(x + y, 30);
                assert_ne!(x, y);
            }
        };

        let analysis = analyze_test_structure(&function);

        assert!(analysis.has_setup);
        assert!(analysis.has_assertions);
        assert_eq!(analysis.assertion_count, 3);
        assert!(!analysis.has_panic);
        assert!(!analysis.has_expect);
        assert!(!analysis.has_unwrap);
    }

    #[test]
    fn test_analyze_test_structure_empty_test() {
        use syn::parse_quote;

        let function: ItemFn = parse_quote! {
            #[test]
            fn test_empty() {
            }
        };

        let analysis = analyze_test_structure(&function);

        assert!(!analysis.has_setup);
        assert!(!analysis.has_action);
        assert!(!analysis.has_assertions);
        assert_eq!(analysis.assertion_count, 0);
        assert!(!analysis.has_panic);
        assert!(!analysis.has_expect);
        assert!(!analysis.has_unwrap);
    }
}
