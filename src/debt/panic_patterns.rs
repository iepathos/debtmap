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

        // Check for #[cfg(test)] attribute or test module name
        self.in_test_module = node.attrs.iter().any(|attr| {
            if attr.path().get_ident().map(|i| i.to_string()).as_deref() == Some("cfg") {
                // Check if it's cfg(test) by converting to string
                let attr_str = format!("{}", quote::quote!(#attr));
                attr_str.contains("test")
            } else {
                false
            }
        }) || node.ident == "tests";

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

    #[test]
    fn test_expect_with_descriptive_message() {
        let code = r#"
            fn example() {
                let config = std::fs::read_to_string("config.toml")
                    .expect("Failed to read config.toml - check file exists and permissions");
            }
        "#;

        let file = parse_str::<File>(code).expect("Failed to parse test code");
        let items = detect_panic_patterns(&file, Path::new("test.rs"), None);

        // Should not detect issues for descriptive expect messages
        assert!(items.is_empty() || !items[0].message.contains("generic"));
    }

    #[test]
    fn test_unimplemented_macro() {
        let code = r#"
            fn example() {
                unimplemented!("This feature is not ready yet");
            }
        "#;

        let file = parse_str::<File>(code).expect("Failed to parse test code");
        let items = detect_panic_patterns(&file, Path::new("test.rs"), None);

        assert!(!items.is_empty());
        assert!(items[0].message.contains("unimplemented"));
        assert_eq!(items[0].debt_type, DebtType::ErrorSwallowing);
    }

    #[test]
    fn test_multiple_panic_patterns() {
        let code = r#"
            fn complex_example() {
                let result = risky_operation().unwrap();
                let option = maybe_value().expect("error");

                if impossible_condition() {
                    panic!("This should never happen");
                }

                todo!("Implement error handling");
                unreachable!();
            }
        "#;

        let file = parse_str::<File>(code).expect("Failed to parse test code");
        let items = detect_panic_patterns(&file, Path::new("test.rs"), None);

        // Should detect multiple patterns
        assert!(items.len() >= 4);

        let has_unwrap = items.iter().any(|item| item.message.contains("unwrap"));
        let has_expect = items.iter().any(|item| item.message.contains("expect"));
        let has_panic = items.iter().any(|item| item.message.contains("panic!"));
        let has_todo = items.iter().any(|item| item.message.contains("todo"));

        assert!(has_unwrap);
        assert!(has_expect);
        assert!(has_panic);
        assert!(has_todo);
    }

    #[test]
    fn test_priority_determination() {
        let detector = PanicPatternDetector::new(Path::new("test.rs"), None);

        // Test production code priorities
        assert_eq!(
            detector.determine_priority(&PanicPattern::PanicInNonTest),
            Priority::Critical
        );
        assert_eq!(
            detector.determine_priority(&PanicPattern::UnwrapOnResult),
            Priority::High
        );
        assert_eq!(
            detector.determine_priority(&PanicPattern::ExpectWithGenericMessage),
            Priority::Medium
        );
        assert_eq!(
            detector.determine_priority(&PanicPattern::TodoInProduction),
            Priority::Medium
        );
    }

    #[test]
    fn test_pattern_descriptions_and_remediations() {
        use PanicPattern::*;

        // Test descriptions
        assert_eq!(UnwrapOnResult.description(), ".unwrap() on Result type");
        assert_eq!(UnwrapOnOption.description(), ".unwrap() on Option type");
        assert_eq!(
            ExpectWithGenericMessage.description(),
            ".expect() with generic message"
        );
        assert_eq!(PanicInNonTest.description(), "panic! in non-test code");
        assert_eq!(
            UnreachableInReachable.description(),
            "unreachable! that may be reachable"
        );
        assert_eq!(
            TodoInProduction.description(),
            "todo!/unimplemented! in production"
        );

        // Test remediations
        assert!(UnwrapOnResult.remediation().contains("? operator"));
        assert!(ExpectWithGenericMessage
            .remediation()
            .contains("descriptive context"));
        assert!(PanicInNonTest.remediation().contains("Return Result"));
        assert!(UnreachableInReachable
            .remediation()
            .contains("Verify code path"));
        assert!(TodoInProduction
            .remediation()
            .contains("Implement the functionality"));
    }

    #[test]
    fn test_check_unwrap_patterns_edge_cases() {
        let code = r#"
            fn example() {
                // Chained unwraps
                let value = some_result().unwrap().unwrap();

                // Unwrap in various contexts
                let items: Vec<_> = vec![Some(1), Some(2), None]
                    .into_iter()
                    .map(|x| x.unwrap())
                    .collect();

                // Expect with very short message
                let result = operation().expect("err");

                // Expect with common generic messages
                let data = fetch_data().expect("should not happen");
                let config = load_config().expect("unexpected");
            }
        "#;

        let file = parse_str::<File>(code).expect("Failed to parse test code");
        let items = detect_panic_patterns(&file, Path::new("test.rs"), None);

        // Should detect multiple unwrap/expect issues
        assert!(items.len() >= 5);

        let unwrap_count = items
            .iter()
            .filter(|item| item.message.contains("unwrap"))
            .count();
        let expect_count = items
            .iter()
            .filter(|item| item.message.contains("expect"))
            .count();

        assert!(unwrap_count >= 3); // Multiple unwraps
        assert!(expect_count >= 3); // Multiple generic expects
    }

    #[test]
    fn test_test_module_detection() {
        let code = r#"
            #[cfg(test)]
            mod tests {
                use super::*;

                fn helper_function() {
                    let result = operation().unwrap();
                    panic!("Test helper panic");
                }

                #[test]
                fn test_something() {
                    helper_function();
                }
            }
        "#;

        let file = parse_str::<File>(code).expect("Failed to parse test code");
        let items = detect_panic_patterns(&file, Path::new("test.rs"), None);

        // Should detect issues but with low priority due to test module
        if !items.is_empty() {
            assert_eq!(items[0].priority, Priority::Low);
        }
    }

    #[test]
    fn test_macro_in_statement_position() {
        let code = r#"
            fn example() {
                if condition {
                    todo!();
                } else {
                    unreachable!();
                }

                panic!("Statement panic");
            }
        "#;

        let file = parse_str::<File>(code).expect("Failed to parse test code");
        let items = detect_panic_patterns(&file, Path::new("test.rs"), None);

        // Should detect macros in both expression and statement positions
        assert!(items.len() >= 3);

        let has_todo = items.iter().any(|item| item.message.contains("todo"));
        let has_unreachable = items
            .iter()
            .any(|item| item.message.contains("unreachable"));
        let has_panic = items.iter().any(|item| item.message.contains("panic!"));

        assert!(has_todo);
        assert!(has_unreachable);
        assert!(has_panic);
    }
}
