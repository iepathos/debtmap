use crate::core::{DebtItem, DebtType, Priority};
use crate::debt::suppression::SuppressionContext;
use std::path::Path;
use syn::visit::Visit;
use syn::{Expr, ExprIf, ExprLet, ExprMatch, ExprMethodCall, File, ItemFn, Pat, Stmt};

pub struct ErrorSwallowingDetector<'a> {
    items: Vec<DebtItem>,
    current_file: &'a Path,
    suppression: Option<&'a SuppressionContext>,
    in_test_function: bool,
}

impl<'a> ErrorSwallowingDetector<'a> {
    pub fn new(file_path: &'a Path, suppression: Option<&'a SuppressionContext>) -> Self {
        Self {
            items: Vec::new(),
            current_file: file_path,
            suppression,
            in_test_function: false,
        }
    }

    pub fn detect(mut self, file: &File) -> Vec<DebtItem> {
        self.visit_file(file);
        self.items
    }

    fn add_debt_item(&mut self, line: usize, pattern: ErrorSwallowingPattern, context: &str) {
        // Check if this item is suppressed
        if let Some(checker) = self.suppression {
            if checker.is_suppressed(line, &DebtType::ErrorSwallowing) {
                return;
            }
        }

        let priority = self.determine_priority(&pattern);

        let message = format!("{}: {}", pattern.description(), pattern.remediation());

        self.items.push(DebtItem {
            id: format!("error-swallow-{}-{}", self.current_file.display(), line),
            debt_type: DebtType::ErrorSwallowing,
            priority,
            file: self.current_file.to_path_buf(),
            line,
            message,
            context: Some(context.to_string()),
        });
    }

    fn determine_priority(&self, pattern: &ErrorSwallowingPattern) -> Priority {
        // Lower priority for test functions
        if self.in_test_function {
            return Priority::Low;
        }

        match pattern {
            ErrorSwallowingPattern::IfLetOkNoElse | ErrorSwallowingPattern::IfLetOkEmptyElse => {
                Priority::Medium
            }
            ErrorSwallowingPattern::LetUnderscoreResult => Priority::High,
            ErrorSwallowingPattern::OkMethodDiscard => Priority::Medium,
            ErrorSwallowingPattern::MatchIgnoredErr => Priority::Medium,
            ErrorSwallowingPattern::UnwrapOrNoLog
            | ErrorSwallowingPattern::UnwrapOrDefaultNoLog => Priority::Low,
        }
    }

    fn check_if_let_ok(&mut self, expr_if: &ExprIf) {
        if !Self::is_if_let_ok_pattern(&expr_if.cond) {
            return;
        }

        let line = 1; // Placeholder line number
        if let Some((pattern, description)) = Self::classify_error_handling(&expr_if.else_branch) {
            self.add_debt_item(line, pattern, description);
        }
    }

    // Pure function to check if expression is an if-let Ok pattern
    fn is_if_let_ok_pattern(cond: &Expr) -> bool {
        match cond {
            Expr::Let(ExprLet { pat, .. }) => Self::is_ok_pattern(pat),
            _ => false,
        }
    }

    // Pure function to check if pattern matches Ok(...)
    fn is_ok_pattern(pat: &Pat) -> bool {
        match pat {
            Pat::TupleStruct(pat_tuple) => pat_tuple
                .path
                .get_ident()
                .is_some_and(|ident| ident == "Ok"),
            _ => false,
        }
    }

    // Pure function to classify error handling pattern based on else branch
    fn classify_error_handling(
        else_branch: &Option<(syn::token::Else, Box<Expr>)>,
    ) -> Option<(ErrorSwallowingPattern, &'static str)> {
        match else_branch {
            Some((_, expr)) if is_empty_block(expr) => Some((
                ErrorSwallowingPattern::IfLetOkEmptyElse,
                "Empty else branch for Result handling",
            )),
            None => Some((
                ErrorSwallowingPattern::IfLetOkNoElse,
                "No error handling for Result",
            )),
            _ => None, // Proper error handling, no debt
        }
    }

    fn check_let_underscore(&mut self, stmt: &Stmt) {
        if let Stmt::Local(local) = stmt {
            if let Pat::Wild(_) = &local.pat {
                if let Some(init) = &local.init {
                    if is_result_type(&init.expr) {
                        // Use a placeholder line number
                        let line = 1;
                        self.add_debt_item(
                            line,
                            ErrorSwallowingPattern::LetUnderscoreResult,
                            "Result discarded with let _ pattern",
                        );
                    }
                }
            }
        }
    }

    fn check_ok_method(&mut self, method_call: &ExprMethodCall) {
        let ident = &method_call.method;
        if ident == "ok" && method_call.args.is_empty() {
            // Check if the result of .ok() is used meaningfully
            let line = 1;
            self.add_debt_item(
                line,
                ErrorSwallowingPattern::OkMethodDiscard,
                "Error information discarded with .ok()",
            );
        }
    }

    fn check_unwrap_or_methods(&mut self, method_call: &ExprMethodCall) {
        let ident = &method_call.method;
        let method_name = ident.to_string();
        if method_name == "unwrap_or" || method_name == "unwrap_or_default" {
            let line = 1;
            let pattern = if method_name == "unwrap_or" {
                ErrorSwallowingPattern::UnwrapOrNoLog
            } else {
                ErrorSwallowingPattern::UnwrapOrDefaultNoLog
            };
            self.add_debt_item(
                line,
                pattern,
                &format!("Error swallowed by {} without logging", method_name),
            );
        }
    }

    fn check_match_expr(&mut self, expr_match: &ExprMatch) {
        for arm in &expr_match.arms {
            if let Pat::TupleStruct(pat_tuple) = &arm.pat {
                if let Some(path) = pat_tuple.path.get_ident() {
                    if path == "Err" {
                        // Check if the Err arm body is effectively empty
                        if is_empty_expr(&arm.body) {
                            let line = 1;
                            self.add_debt_item(
                                line,
                                ErrorSwallowingPattern::MatchIgnoredErr,
                                "Error variant ignored in match expression",
                            );
                        }
                    }
                }
            }
        }
    }
}

impl<'a> Visit<'_> for ErrorSwallowingDetector<'a> {
    fn visit_item_fn(&mut self, node: &ItemFn) {
        // Check if this is a test function
        let was_in_test = self.in_test_function;
        self.in_test_function = node
            .attrs
            .iter()
            .any(|attr| attr.path().get_ident().map(|i| i.to_string()).as_deref() == Some("test"));

        syn::visit::visit_item_fn(self, node);
        self.in_test_function = was_in_test;
    }

    fn visit_expr_if(&mut self, node: &ExprIf) {
        self.check_if_let_ok(node);
        syn::visit::visit_expr_if(self, node);
    }

    fn visit_stmt(&mut self, node: &Stmt) {
        self.check_let_underscore(node);
        syn::visit::visit_stmt(self, node);
    }

    fn visit_expr_method_call(&mut self, node: &ExprMethodCall) {
        self.check_ok_method(node);
        self.check_unwrap_or_methods(node);
        syn::visit::visit_expr_method_call(self, node);
    }

    fn visit_expr_match(&mut self, node: &ExprMatch) {
        self.check_match_expr(node);
        syn::visit::visit_expr_match(self, node);
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ErrorSwallowingPattern {
    IfLetOkNoElse,
    IfLetOkEmptyElse,
    LetUnderscoreResult,
    OkMethodDiscard,
    MatchIgnoredErr,
    UnwrapOrNoLog,
    UnwrapOrDefaultNoLog,
}

impl ErrorSwallowingPattern {
    fn description(&self) -> &'static str {
        match self {
            Self::IfLetOkNoElse => "if let Ok(...) without else branch",
            Self::IfLetOkEmptyElse => "if let Ok(...) with empty else branch",
            Self::LetUnderscoreResult => "let _ = discarding Result",
            Self::OkMethodDiscard => ".ok() discarding error information",
            Self::MatchIgnoredErr => "match with ignored Err variant",
            Self::UnwrapOrNoLog => "unwrap_or without error logging",
            Self::UnwrapOrDefaultNoLog => "unwrap_or_default without error logging",
        }
    }

    fn remediation(&self) -> &'static str {
        match self {
            Self::IfLetOkNoElse | Self::IfLetOkEmptyElse => {
                "Use ? operator or handle error case explicitly"
            }
            Self::LetUnderscoreResult => "Add error handling or logging before discarding",
            Self::OkMethodDiscard => "Use map_err to log before converting to Option",
            Self::MatchIgnoredErr => "Handle or log the error in the Err arm",
            Self::UnwrapOrNoLog | Self::UnwrapOrDefaultNoLog => {
                "Use unwrap_or_else with error logging"
            }
        }
    }
}

fn is_empty_block(expr: &Expr) -> bool {
    match expr {
        Expr::Block(block) => block.block.stmts.is_empty(),
        _ => false,
    }
}

fn is_empty_expr(expr: &Expr) -> bool {
    match expr {
        Expr::Block(block) => block.block.stmts.is_empty(),
        Expr::Tuple(tuple) => tuple.elems.is_empty(),
        _ => false,
    }
}

fn is_result_type(expr: &Expr) -> bool {
    // This is a simplified check - in a real implementation,
    // we'd want to use type information to be more accurate
    match expr {
        Expr::Call(_call) => {
            // Check for function calls that likely return Result
            true // Simplified for now
        }
        Expr::MethodCall(_method) => {
            // Check for method calls that likely return Result
            true // Simplified for now
        }
        Expr::Try(_) => true,
        _ => false,
    }
}

pub fn detect_error_swallowing(
    file: &File,
    file_path: &Path,
    suppression: Option<&SuppressionContext>,
) -> Vec<DebtItem> {
    let detector = ErrorSwallowingDetector::new(file_path, suppression);
    detector.detect(file)
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::{parse_quote, parse_str, Pat};

    #[test]
    fn test_if_let_ok_no_else() {
        let code = r#"
            fn example() {
                if let Ok(value) = some_function() {
                    println!("{}", value);
                }
            }
        "#;

        let file = parse_str::<File>(code).expect("Failed to parse test code");
        let items = detect_error_swallowing(&file, Path::new("test.rs"), None);

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].debt_type, DebtType::ErrorSwallowing);
        assert!(items[0].message.contains("if let Ok"));
    }

    #[test]
    fn test_let_underscore_result() {
        let code = r#"
            fn example() {
                let _ = function_returning_result();
            }
        "#;

        let file = parse_str::<File>(code).expect("Failed to parse test code");
        let items = detect_error_swallowing(&file, Path::new("test.rs"), None);

        assert!(!items.is_empty());
        assert_eq!(items[0].debt_type, DebtType::ErrorSwallowing);
    }

    #[test]
    fn test_ok_method_discard() {
        let code = r#"
            fn example() {
                some_result.ok();
            }
        "#;

        let file = parse_str::<File>(code).expect("Failed to parse test code");
        let items = detect_error_swallowing(&file, Path::new("test.rs"), None);

        assert!(!items.is_empty());
        assert_eq!(items[0].debt_type, DebtType::ErrorSwallowing);
        assert!(items[0].message.contains(".ok()"));
    }

    #[test]
    fn test_match_ignored_err() {
        let code = r#"
            fn example() {
                match some_result {
                    Ok(v) => println!("{}", v),
                    Err(_) => {},
                }
            }
        "#;

        let file = parse_str::<File>(code).expect("Failed to parse test code");
        let items = detect_error_swallowing(&file, Path::new("test.rs"), None);

        assert!(!items.is_empty());
        assert_eq!(items[0].debt_type, DebtType::ErrorSwallowing);
        assert!(items[0].message.contains("match"));
    }

    #[test]
    fn test_unwrap_or_no_log() {
        let code = r#"
            fn example() {
                let value = some_result.unwrap_or(0);
            }
        "#;

        let file = parse_str::<File>(code).expect("Failed to parse test code");
        let items = detect_error_swallowing(&file, Path::new("test.rs"), None);

        assert!(!items.is_empty());
        assert_eq!(items[0].debt_type, DebtType::ErrorSwallowing);
        assert!(items[0].message.contains("unwrap_or"));
    }

    #[test]
    fn test_is_if_let_ok_pattern() {
        // Test positive case: if let Ok pattern
        let code = "if let Ok(value) = some_function() { }";
        let expr: syn::Expr = parse_str(code).expect("Failed to parse");
        if let syn::Expr::If(expr_if) = expr {
            assert!(
                ErrorSwallowingDetector::is_if_let_ok_pattern(&expr_if.cond),
                "Should recognize if let Ok pattern"
            );
        }

        // Test negative case: regular if condition
        let code = "if true { }";
        let expr: syn::Expr = parse_str(code).expect("Failed to parse");
        if let syn::Expr::If(expr_if) = expr {
            assert!(
                !ErrorSwallowingDetector::is_if_let_ok_pattern(&expr_if.cond),
                "Should not recognize regular if as Ok pattern"
            );
        }

        // Test negative case: if let Err pattern
        let code = "if let Err(e) = some_function() { }";
        let expr: syn::Expr = parse_str(code).expect("Failed to parse");
        if let syn::Expr::If(expr_if) = expr {
            assert!(
                !ErrorSwallowingDetector::is_if_let_ok_pattern(&expr_if.cond),
                "Should not recognize if let Err as Ok pattern"
            );
        }
    }

    #[test]
    fn test_is_ok_pattern() {
        // Test positive case: Ok pattern
        let pat: Pat = parse_quote!(Ok(value));
        assert!(
            ErrorSwallowingDetector::is_ok_pattern(&pat),
            "Should recognize Ok pattern"
        );

        // Test negative case: Err pattern
        let pat: Pat = parse_quote!(Err(e));
        assert!(
            !ErrorSwallowingDetector::is_ok_pattern(&pat),
            "Should not recognize Err as Ok pattern"
        );

        // Test negative case: Some pattern
        let pat: Pat = parse_quote!(Some(x));
        assert!(
            !ErrorSwallowingDetector::is_ok_pattern(&pat),
            "Should not recognize Some as Ok pattern"
        );

        // Test negative case: wildcard pattern
        let pat: Pat = parse_quote!(_);
        assert!(
            !ErrorSwallowingDetector::is_ok_pattern(&pat),
            "Should not recognize wildcard as Ok pattern"
        );
    }

    #[test]
    fn test_classify_error_handling() {
        use syn::{parse_quote, Expr};

        // Test case: No else branch
        let else_branch = None;
        let result = ErrorSwallowingDetector::classify_error_handling(&else_branch);
        assert!(result.is_some());
        let (pattern, desc) = result.unwrap();
        assert_eq!(pattern, ErrorSwallowingPattern::IfLetOkNoElse);
        assert_eq!(desc, "No error handling for Result");

        // Test case: Empty else block
        let empty_block: Expr = parse_quote! { {} };
        let else_branch = Some((syn::token::Else::default(), Box::new(empty_block)));
        let result = ErrorSwallowingDetector::classify_error_handling(&else_branch);
        assert!(result.is_some());
        let (pattern, desc) = result.unwrap();
        assert_eq!(pattern, ErrorSwallowingPattern::IfLetOkEmptyElse);
        assert_eq!(desc, "Empty else branch for Result handling");

        // Test case: Non-empty else block (proper handling)
        let non_empty_block: Expr = parse_quote! { { println!("error"); } };
        let else_branch = Some((syn::token::Else::default(), Box::new(non_empty_block)));
        let result = ErrorSwallowingDetector::classify_error_handling(&else_branch);
        assert!(
            result.is_none(),
            "Should not flag debt for proper error handling"
        );
    }

    #[test]
    fn test_if_let_ok_with_empty_else() {
        let code = r#"
            fn example() {
                if let Ok(value) = some_function() {
                    println!("{}", value);
                } else {
                    // Empty else block
                }
            }
        "#;

        let file = parse_str::<File>(code).expect("Failed to parse test code");
        let items = detect_error_swallowing(&file, Path::new("test.rs"), None);

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].debt_type, DebtType::ErrorSwallowing);
        assert!(items[0].message.contains("empty else branch"));
    }

    #[test]
    fn test_if_let_ok_with_proper_handling() {
        let code = r#"
            fn example() {
                if let Ok(value) = some_function() {
                    println!("{}", value);
                } else {
                    eprintln!("Error occurred");
                }
            }
        "#;

        let file = parse_str::<File>(code).expect("Failed to parse test code");
        let items = detect_error_swallowing(&file, Path::new("test.rs"), None);

        // Should not detect debt when else branch has proper handling
        assert_eq!(
            items.len(),
            0,
            "Should not detect debt for proper error handling"
        );
    }

    #[test]
    fn test_lower_priority_in_tests() {
        let code = r#"
            #[test]
            fn test_example() {
                if let Ok(value) = some_function() {
                    assert_eq!(value, 42);
                }
            }
        "#;

        let file = parse_str::<File>(code).expect("Failed to parse test code");
        let items = detect_error_swallowing(&file, Path::new("test.rs"), None);

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].priority, Priority::Low);
    }
}
