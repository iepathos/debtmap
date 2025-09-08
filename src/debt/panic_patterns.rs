use crate::core::{DebtItem, DebtType, Priority};
use crate::debt::suppression::SuppressionContext;
use std::path::Path;
use syn::visit::Visit;
use syn::{Expr, ExprMacro, ExprMethodCall, File, ItemFn, Macro};

pub struct PanicPatternDetector<'a> {
    items: Vec<DebtItem>,
    current_file: &'a Path,
    suppression: Option<&'a SuppressionContext>,
    in_test_function: bool,
    in_test_module: bool,
}

impl<'a> PanicPatternDetector<'a> {
    pub fn new(file_path: &'a Path, suppression: Option<&'a SuppressionContext>) -> Self {
        Self {
            items: Vec::new(),
            current_file: file_path,
            suppression,
            in_test_function: false,
            in_test_module: false,
        }
    }

    pub fn detect(mut self, file: &File) -> Vec<DebtItem> {
        self.visit_file(file);
        self.items
    }

    fn get_line_number(&self, span: proc_macro2::Span) -> usize {
        span.start().line
    }

    fn add_debt_item(&mut self, line: usize, pattern: PanicPattern, context: &str) {
        // Check if this item is suppressed
        if let Some(checker) = self.suppression {
            if checker.is_suppressed(line, &DebtType::ErrorSwallowing) {
                return;
            }
        }

        let priority = self.determine_priority(&pattern);
        let message = format!("{}: {}", pattern.description(), pattern.remediation());

        self.items.push(DebtItem {
            id: format!("panic-pattern-{}-{}", self.current_file.display(), line),
            debt_type: DebtType::ErrorSwallowing,
            priority,
            file: self.current_file.to_path_buf(),
            line,
            column: None,
            message,
            context: Some(context.to_string()),
        });
    }

    fn determine_priority(&self, pattern: &PanicPattern) -> Priority {
        // Lower priority for test code
        if self.in_test_function || self.in_test_module {
            return Priority::Low;
        }

        match pattern {
            PanicPattern::UnwrapOnResult | PanicPattern::UnwrapOnOption => Priority::High,
            PanicPattern::ExpectWithGenericMessage => Priority::Medium,
            PanicPattern::PanicInNonTest => Priority::Critical,
            PanicPattern::UnreachableInReachable => Priority::High,
            PanicPattern::TodoInProduction => Priority::Medium,
        }
    }

    fn check_unwrap_patterns(&mut self, method_call: &ExprMethodCall) {
        let method_name = method_call.method.to_string();

        if method_name == "unwrap" {
            let line = self.get_line_number(method_call.method.span());

            // Try to determine if it's Result or Option based on context
            // For now, we'll assume it could be either
            self.add_debt_item(
                line,
                PanicPattern::UnwrapOnResult,
                ".unwrap() can panic in production",
            );
        } else if method_name == "expect" {
            let line = self.get_line_number(method_call.method.span());

            // Check if the expect message is generic
            let is_generic = method_call.args.iter().any(|arg| {
                if let Expr::Lit(lit) = arg {
                    if let syn::Lit::Str(s) = &lit.lit {
                        let msg = s.value().to_lowercase();
                        // Common generic messages
                        msg == "failed"
                            || msg == "error"
                            || msg == "should not happen"
                            || msg == "unexpected"
                            || msg.len() < 10 // Very short messages are likely generic
                    } else {
                        false
                    }
                } else {
                    false
                }
            });

            if is_generic {
                self.add_debt_item(
                    line,
                    PanicPattern::ExpectWithGenericMessage,
                    "expect() with generic error message",
                );
            }
        }
    }

    fn check_panic_macros(&mut self, mac: &Macro) {
        if let Some(ident) = mac.path.get_ident() {
            let macro_name = ident.to_string();
            let line = self.get_line_number(ident.span());

            match macro_name.as_str() {
                "panic" => {
                    self.add_debt_item(
                        line,
                        PanicPattern::PanicInNonTest,
                        "panic! macro in production code",
                    );
                }
                "unreachable" => {
                    self.add_debt_item(
                        line,
                        PanicPattern::UnreachableInReachable,
                        "unreachable! macro may be reachable",
                    );
                }
                "todo" => {
                    self.add_debt_item(
                        line,
                        PanicPattern::TodoInProduction,
                        "todo! macro in production code",
                    );
                }
                "unimplemented" => {
                    self.add_debt_item(
                        line,
                        PanicPattern::TodoInProduction,
                        "unimplemented! macro in production code",
                    );
                }
                _ => {}
            }
        }
    }
}

impl<'a> Visit<'_> for PanicPatternDetector<'a> {
    fn visit_item_fn(&mut self, node: &ItemFn) {
        let was_in_test = self.in_test_function;
        self.in_test_function = node
            .attrs
            .iter()
            .any(|attr| attr.path().get_ident().map(|i| i.to_string()).as_deref() == Some("test"));

        syn::visit::visit_item_fn(self, node);
        self.in_test_function = was_in_test;
    }

    fn visit_item_mod(&mut self, node: &syn::ItemMod) {
        let was_in_test = self.in_test_module;

        // Check for #[cfg(test)] attribute
        self.in_test_module = node.attrs.iter().any(|attr| {
            if attr.path().get_ident().map(|i| i.to_string()).as_deref() == Some("cfg") {
                // Parse the attribute to check if it's cfg(test)
                if let Ok(syn::Meta::List(list)) = attr.parse_args::<syn::Meta>() {
                    list.tokens.to_string().contains("test")
                } else {
                    false
                }
            } else {
                false
            }
        });

        syn::visit::visit_item_mod(self, node);
        self.in_test_module = was_in_test;
    }

    fn visit_expr_method_call(&mut self, node: &ExprMethodCall) {
        self.check_unwrap_patterns(node);
        syn::visit::visit_expr_method_call(self, node);
    }

    fn visit_expr_macro(&mut self, node: &ExprMacro) {
        self.check_panic_macros(&node.mac);
        syn::visit::visit_expr_macro(self, node);
    }

    fn visit_stmt(&mut self, node: &syn::Stmt) {
        // Also check for macros in statement position
        if let syn::Stmt::Macro(stmt_macro) = node {
            self.check_panic_macros(&stmt_macro.mac);
        }
        syn::visit::visit_stmt(self, node);
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PanicPattern {
    UnwrapOnResult,
    UnwrapOnOption,
    ExpectWithGenericMessage,
    PanicInNonTest,
    UnreachableInReachable,
    TodoInProduction,
}

impl PanicPattern {
    fn description(&self) -> &'static str {
        match self {
            Self::UnwrapOnResult => ".unwrap() on Result type",
            Self::UnwrapOnOption => ".unwrap() on Option type",
            Self::ExpectWithGenericMessage => ".expect() with generic message",
            Self::PanicInNonTest => "panic! in non-test code",
            Self::UnreachableInReachable => "unreachable! that may be reachable",
            Self::TodoInProduction => "todo!/unimplemented! in production",
        }
    }

    fn remediation(&self) -> &'static str {
        match self {
            Self::UnwrapOnResult | Self::UnwrapOnOption => {
                "Use ? operator, match, or unwrap_or_else with proper error handling"
            }
            Self::ExpectWithGenericMessage => "Provide descriptive context in expect() message",
            Self::PanicInNonTest => "Return Result or handle error gracefully",
            Self::UnreachableInReachable => "Verify code path is truly unreachable or handle case",
            Self::TodoInProduction => "Implement the functionality or return appropriate error",
        }
    }
}

pub fn detect_panic_patterns(
    file: &File,
    file_path: &Path,
    suppression: Option<&SuppressionContext>,
) -> Vec<DebtItem> {
    let detector = PanicPatternDetector::new(file_path, suppression);
    detector.detect(file)
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_str;

    #[test]
    fn test_unwrap_detection() {
        let code = r#"
            fn example() {
                let result: Result<i32, String> = Ok(42);
                let value = result.unwrap();
            }
        "#;

        let file = parse_str::<File>(code).expect("Failed to parse test code");
        let items = detect_panic_patterns(&file, Path::new("test.rs"), None);

        assert!(!items.is_empty());
        assert_eq!(items[0].debt_type, DebtType::ErrorSwallowing);
        assert!(items[0].message.contains("unwrap"));
    }

    #[test]
    fn test_expect_with_generic_message() {
        let code = r#"
            fn example() {
                let result: Result<i32, String> = Ok(42);
                let value = result.expect("failed");
            }
        "#;

        let file = parse_str::<File>(code).expect("Failed to parse test code");
        let items = detect_panic_patterns(&file, Path::new("test.rs"), None);

        assert!(!items.is_empty());
        assert!(items[0].message.contains("expect"));
        assert!(items[0].message.contains("generic"));
    }

    #[test]
    fn test_panic_in_production() {
        let code = r#"
            fn example() {
                if some_condition() {
                    panic!("This should not happen");
                }
            }
        "#;

        let file = parse_str::<File>(code).expect("Failed to parse test code");
        let items = detect_panic_patterns(&file, Path::new("test.rs"), None);

        assert!(!items.is_empty());
        assert_eq!(items[0].priority, Priority::Critical);
        assert!(items[0].message.contains("panic!"));
    }

    #[test]
    fn test_no_detection_in_tests() {
        let code = r#"
            #[test]
            fn test_example() {
                let result: Result<i32, String> = Ok(42);
                let value = result.unwrap();
                panic!("Test panic");
            }
        "#;

        let file = parse_str::<File>(code).expect("Failed to parse test code");
        let items = detect_panic_patterns(&file, Path::new("test.rs"), None);

        // Should still detect but with low priority
        assert!(!items.is_empty());
        assert_eq!(items[0].priority, Priority::Low);
    }

    #[test]
    fn test_todo_macro() {
        let code = r#"
            fn example() {
                todo!("Implement this later");
            }
        "#;

        let file = parse_str::<File>(code).expect("Failed to parse test code");
        let items = detect_panic_patterns(&file, Path::new("test.rs"), None);

        assert!(!items.is_empty());
        assert!(items[0].message.contains("todo"));
    }

    #[test]
    fn test_unreachable_macro() {
        let code = r#"
            fn example(x: i32) {
                match x {
                    1 => println!("one"),
                    2 => println!("two"),
                    _ => unreachable!(),
                }
            }
        "#;

        let file = parse_str::<File>(code).expect("Failed to parse test code");
        let items = detect_panic_patterns(&file, Path::new("test.rs"), None);

        assert!(!items.is_empty());
        assert!(items[0].message.contains("unreachable"));
    }
}
