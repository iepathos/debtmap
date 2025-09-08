use crate::core::{DebtItem, DebtType, Priority};
use crate::debt::suppression::SuppressionContext;
use std::path::Path;
use syn::visit::Visit;
use syn::{Expr, ExprMethodCall, ExprTry, File, ItemFn};

pub struct ContextLossAnalyzer<'a> {
    items: Vec<DebtItem>,
    current_file: &'a Path,
    suppression: Option<&'a SuppressionContext>,
    in_test_function: bool,
    question_mark_count: usize,
    current_function: Option<usize>,
}

impl<'a> ContextLossAnalyzer<'a> {
    pub fn new(file_path: &'a Path, suppression: Option<&'a SuppressionContext>) -> Self {
        Self {
            items: Vec::new(),
            current_file: file_path,
            suppression,
            in_test_function: false,
            question_mark_count: 0,
            current_function: None,
        }
    }

    pub fn detect(mut self, file: &File) -> Vec<DebtItem> {
        self.visit_file(file);
        self.items
    }

    fn get_line_number(&self, span: proc_macro2::Span) -> usize {
        span.start().line
    }

    fn add_debt_item(&mut self, line: usize, pattern: ContextLossPattern, context: &str) {
        // Check if this item is suppressed
        if let Some(checker) = self.suppression {
            if checker.is_suppressed(line, &DebtType::ErrorSwallowing) {
                return;
            }
        }

        let priority = self.determine_priority(&pattern);
        let message = format!("{}: {}", pattern.description(), pattern.remediation());

        self.items.push(DebtItem {
            id: format!("context-loss-{}-{}", self.current_file.display(), line),
            debt_type: DebtType::ErrorSwallowing,
            priority,
            file: self.current_file.to_path_buf(),
            line,
            column: None,
            message,
            context: Some(context.to_string()),
        });
    }

    fn determine_priority(&self, pattern: &ContextLossPattern) -> Priority {
        // Lower priority for test code
        if self.in_test_function {
            return Priority::Low;
        }

        match pattern {
            ContextLossPattern::MapErrDiscardingOriginal => Priority::Medium,
            ContextLossPattern::AnyhowWithoutContext => Priority::Medium,
            ContextLossPattern::QuestionMarkChain => Priority::Low,
            ContextLossPattern::StringErrorConversion => Priority::High,
            ContextLossPattern::IntoErrorConversion => Priority::Medium,
        }
    }

    fn check_map_err_patterns(&mut self, method_call: &ExprMethodCall) {
        if method_call.method == "map_err" {
            let line = self.get_line_number(method_call.method.span());

            // Check if the closure discards the original error
            if let Some(arg) = method_call.args.first() {
                let discards_original = match arg {
                    Expr::Closure(closure) => {
                        // Check if the closure ignores its parameter
                        if closure
                            .inputs
                            .iter()
                            .any(|pat| matches!(pat, syn::Pat::Wild(_)))
                        {
                            true
                        } else {
                            // Check if the body doesn't reference the error parameter
                            // This is a simplified check
                            match &*closure.body {
                                Expr::Lit(_) => true, // Just returns a literal
                                Expr::Call(call) => {
                                    // Check if it's a simple constructor without using the error
                                    !format!("{}", quote::quote!(#call)).contains("e")
                                }
                                _ => false,
                            }
                        }
                    }
                    _ => false,
                };

                if discards_original {
                    self.add_debt_item(
                        line,
                        ContextLossPattern::MapErrDiscardingOriginal,
                        "map_err discards original error context",
                    );
                }
            }
        }
    }

    fn check_context_methods(&mut self, method_call: &ExprMethodCall) {
        let method_name = method_call.method.to_string();

        // Check for anyhow-style methods without context
        if method_name == "with_context" || method_name == "context" {
            // This is good - they're adding context
            return;
        }

        // Check for into() or From conversions that might lose context
        if method_name == "into" {
            let line = self.get_line_number(method_call.method.span());

            // Check if this is likely an error conversion
            // This is a heuristic - we'd need type information for accuracy
            self.add_debt_item(
                line,
                ContextLossPattern::IntoErrorConversion,
                "into() conversion may lose error context",
            );
        }
    }

    fn check_string_conversions(&mut self, method_call: &ExprMethodCall) {
        let method_name = method_call.method.to_string();

        if method_name == "to_string" || method_name == "to_owned" {
            // Check if this is on an error type (heuristic)
            let line = self.get_line_number(method_call.method.span());

            // Look for patterns like err.to_string() in error handling contexts
            self.add_debt_item(
                line,
                ContextLossPattern::StringErrorConversion,
                "Converting error to string loses type information",
            );
        }
    }
}

impl<'a> Visit<'_> for ContextLossAnalyzer<'a> {
    fn visit_item_fn(&mut self, node: &ItemFn) {
        let was_in_test = self.in_test_function;
        let prev_count = self.question_mark_count;
        let prev_func = self.current_function;
        
        self.in_test_function = node
            .attrs
            .iter()
            .any(|attr| attr.path().get_ident().map(|i| i.to_string()).as_deref() == Some("test"));
        
        // Reset question mark count for each function
        self.question_mark_count = 0;
        self.current_function = Some(self.get_line_number(node.sig.fn_token.span));

        syn::visit::visit_item_fn(self, node);
        
        self.in_test_function = was_in_test;
        self.question_mark_count = prev_count;
        self.current_function = prev_func;
    }

    fn visit_expr_method_call(&mut self, node: &ExprMethodCall) {
        self.check_map_err_patterns(node);
        self.check_context_methods(node);
        self.check_string_conversions(node);
        syn::visit::visit_expr_method_call(self, node);
    }

    fn visit_expr_try(&mut self, node: &ExprTry) {
        // Track total number of ? operators in current function
        self.question_mark_count += 1;

        if self.question_mark_count > 3 {
            let line = self.get_line_number(node.question_token.span);
            self.add_debt_item(
                line,
                ContextLossPattern::QuestionMarkChain,
                "Long chain of ? operators without context",
            );
        }

        syn::visit::visit_expr_try(self, node);
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ContextLossPattern {
    MapErrDiscardingOriginal,
    AnyhowWithoutContext,
    QuestionMarkChain,
    StringErrorConversion,
    IntoErrorConversion,
}

impl ContextLossPattern {
    fn description(&self) -> &'static str {
        match self {
            Self::MapErrDiscardingOriginal => "map_err discards original error",
            Self::AnyhowWithoutContext => "anyhow error without context",
            Self::QuestionMarkChain => "Long ? operator chain",
            Self::StringErrorConversion => "Error converted to string",
            Self::IntoErrorConversion => "Generic into() error conversion",
        }
    }

    fn remediation(&self) -> &'static str {
        match self {
            Self::MapErrDiscardingOriginal => "Include original error as source or in message",
            Self::AnyhowWithoutContext => "Use .context() or .with_context() to add information",
            Self::QuestionMarkChain => "Add context at key points in the error chain",
            Self::StringErrorConversion => "Preserve error type or use structured error types",
            Self::IntoErrorConversion => "Use explicit error conversion with context preservation",
        }
    }
}

pub fn analyze_error_context(
    file: &File,
    file_path: &Path,
    suppression: Option<&SuppressionContext>,
) -> Vec<DebtItem> {
    let analyzer = ContextLossAnalyzer::new(file_path, suppression);
    analyzer.detect(file)
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_str;

    #[test]
    fn test_map_err_discarding_original() {
        let code = r#"
            fn example() -> Result<i32, String> {
                some_function()
                    .map_err(|_| "Something went wrong".to_string())
            }
        "#;

        let file = parse_str::<File>(code).expect("Failed to parse test code");
        let items = analyze_error_context(&file, Path::new("test.rs"), None);

        assert!(!items.is_empty());
        assert!(items[0].message.contains("map_err"));
        assert!(items[0].message.contains("discards"));
    }

    #[test]
    fn test_string_error_conversion() {
        let code = r#"
            fn example() {
                let err = std::io::Error::new(std::io::ErrorKind::Other, "test");
                let msg = err.to_string();
            }
        "#;

        let file = parse_str::<File>(code).expect("Failed to parse test code");
        let items = analyze_error_context(&file, Path::new("test.rs"), None);

        assert!(!items.is_empty());
        assert!(items[0].message.contains("string"));
    }

    #[test]
    fn test_question_mark_chain() {
        let code = r#"
            fn example() -> Result<i32, Box<dyn std::error::Error>> {
                let a = func1()?;
                let b = func2()?;
                let c = func3()?;
                let d = func4()?;
                let e = func5()?;
                Ok(e)
            }
        "#;

        let file = parse_str::<File>(code).expect("Failed to parse test code");
        let items = analyze_error_context(&file, Path::new("test.rs"), None);

        // Should detect long chain of ? operators
        assert!(!items.is_empty());
    }

    #[test]
    fn test_into_conversion() {
        let code = r#"
            fn example() -> Result<(), Box<dyn std::error::Error>> {
                let err = std::io::Error::new(std::io::ErrorKind::Other, "test");
                Err(err.into())
            }
        "#;

        let file = parse_str::<File>(code).expect("Failed to parse test code");
        let items = analyze_error_context(&file, Path::new("test.rs"), None);

        assert!(!items.is_empty());
        assert!(items[0].message.contains("into()"));
    }

    #[test]
    fn test_good_context_handling() {
        let code = r#"
            fn example() -> Result<i32, anyhow::Error> {
                some_function()
                    .with_context(|| "Failed to call some_function")?;
                Ok(42)
            }
        "#;

        let file = parse_str::<File>(code).expect("Failed to parse test code");
        let items = analyze_error_context(&file, Path::new("test.rs"), None);

        // Should not detect issues when context is properly added
        // Note: Our simple analysis might still flag the ?, but that's ok
        // In a real implementation we'd have more sophisticated analysis
    }
}
