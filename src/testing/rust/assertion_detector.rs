use super::RustAssertionType;
use syn::spanned::Spanned;
use syn::visit::Visit;
use syn::{Expr, ItemFn, Macro, Stmt};

/// Detects and counts assertions in Rust test functions
pub struct AssertionDetector {
    assertions: Vec<Assertion>,
}

#[derive(Debug, Clone)]
pub struct Assertion {
    pub assertion_type: RustAssertionType,
    pub line: usize,
}

impl AssertionDetector {
    pub fn new() -> Self {
        Self {
            assertions: Vec::new(),
        }
    }

    /// Analyze assertions in a test function
    pub fn analyze_assertions(&mut self, func: &ItemFn) -> Vec<Assertion> {
        self.assertions.clear();

        // Check for `#[should_panic]`
        if self.has_should_panic_attribute(func) {
            self.assertions.push(Assertion {
                assertion_type: RustAssertionType::ShouldPanic,
                line: func.sig.ident.span().start().line,
            });
        }

        // Check if function returns Result<()>
        if self.returns_result(&func.sig.output) {
            self.assertions.push(Assertion {
                assertion_type: RustAssertionType::ResultOk,
                line: func.sig.ident.span().start().line,
            });
        }

        // Visit function body to find assertions
        self.visit_block(&func.block);

        self.assertions.clone()
    }

    /// Count total assertions
    pub fn count_assertions(&self) -> usize {
        self.assertions.len()
    }

    /// Check if test has no assertions
    pub fn has_no_assertions(&self) -> bool {
        self.assertions.is_empty()
    }

    /// Check for `\[should_panic\]` attribute
    fn has_should_panic_attribute(&self, func: &ItemFn) -> bool {
        func.attrs.iter().any(|attr| {
            attr.path().is_ident("should_panic")
                || attr
                    .path()
                    .segments
                    .iter()
                    .any(|seg| seg.ident == "should_panic")
        })
    }

    /// Check if function returns Result<()> or Result<(), E>
    fn returns_result(&self, output: &syn::ReturnType) -> bool {
        if let syn::ReturnType::Type(_, ty) = output {
            let type_str = quote::quote!(#ty).to_string();
            type_str.starts_with("Result")
        } else {
            false
        }
    }

    /// Detect assertion macro by name
    fn detect_assertion_macro(&self, mac: &Macro) -> Option<RustAssertionType> {
        let path_str = mac
            .path
            .segments
            .iter()
            .map(|seg| seg.ident.to_string())
            .collect::<Vec<_>>()
            .join("::");

        match path_str.as_str() {
            "assert" => Some(RustAssertionType::Assert),
            "assert_eq" => Some(RustAssertionType::AssertEq),
            "assert_ne" => Some(RustAssertionType::AssertNe),
            "debug_assert" => Some(RustAssertionType::Assert),
            "debug_assert_eq" => Some(RustAssertionType::AssertEq),
            "debug_assert_ne" => Some(RustAssertionType::AssertNe),
            "matches" => Some(RustAssertionType::Matches),
            "assert_matches" => Some(RustAssertionType::Matches),
            _ if path_str.contains("assert") => Some(RustAssertionType::Custom(path_str)),
            _ => None,
        }
    }
}

impl Default for AssertionDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl<'ast> Visit<'ast> for AssertionDetector {
    fn visit_stmt(&mut self, stmt: &'ast Stmt) {
        // Check for macro invocations in statements
        if let Stmt::Macro(stmt_macro) = stmt {
            if let Some(assertion_type) = self.detect_assertion_macro(&stmt_macro.mac) {
                self.assertions.push(Assertion {
                    assertion_type,
                    line: stmt_macro.mac.path.span().start().line,
                });
            }
        }

        syn::visit::visit_stmt(self, stmt);
    }

    fn visit_expr(&mut self, expr: &'ast Expr) {
        // Check for macro invocations in expressions
        if let Expr::Macro(expr_macro) = expr {
            if let Some(assertion_type) = self.detect_assertion_macro(&expr_macro.mac) {
                self.assertions.push(Assertion {
                    assertion_type,
                    line: expr_macro.mac.path.span().start().line,
                });
            }
        }

        syn::visit::visit_expr(self, expr);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn test_detect_assert() {
        let func: ItemFn = parse_quote! {
            #[test]
            fn test_something() {
                assert!(true);
            }
        };

        let mut detector = AssertionDetector::new();
        let assertions = detector.analyze_assertions(&func);
        assert_eq!(assertions.len(), 1);
        assert!(matches!(
            assertions[0].assertion_type,
            RustAssertionType::Assert
        ));
    }

    #[test]
    fn test_detect_assert_eq() {
        let func: ItemFn = parse_quote! {
            #[test]
            fn test_equality() {
                assert_eq!(1, 1);
            }
        };

        let mut detector = AssertionDetector::new();
        let assertions = detector.analyze_assertions(&func);
        assert_eq!(assertions.len(), 1);
        assert!(matches!(
            assertions[0].assertion_type,
            RustAssertionType::AssertEq
        ));
    }

    #[test]
    fn test_detect_multiple_assertions() {
        let func: ItemFn = parse_quote! {
            #[test]
            fn test_multiple() {
                assert!(true);
                assert_eq!(1, 1);
                assert_ne!(1, 2);
            }
        };

        let mut detector = AssertionDetector::new();
        let assertions = detector.analyze_assertions(&func);
        assert_eq!(assertions.len(), 3);
    }

    #[test]
    fn test_detect_no_assertions() {
        let func: ItemFn = parse_quote! {
            #[test]
            fn test_no_assertions() {
                let x = 42;
                println!("{}", x);
            }
        };

        let mut detector = AssertionDetector::new();
        detector.analyze_assertions(&func);
        assert!(detector.has_no_assertions());
    }

    #[test]
    fn test_detect_should_panic() {
        let func: ItemFn = parse_quote! {
            #[test]
            #[should_panic]
            fn test_panic() {
                panic!("expected");
            }
        };

        let mut detector = AssertionDetector::new();
        let assertions = detector.analyze_assertions(&func);
        assert_eq!(assertions.len(), 1);
        assert!(matches!(
            assertions[0].assertion_type,
            RustAssertionType::ShouldPanic
        ));
    }

    #[test]
    fn test_detect_result_return() {
        let func: ItemFn = parse_quote! {
            #[test]
            fn test_result() -> Result<(), Box<dyn std::error::Error>> {
                Ok(())
            }
        };

        let mut detector = AssertionDetector::new();
        let assertions = detector.analyze_assertions(&func);
        assert_eq!(assertions.len(), 1);
        assert!(matches!(
            assertions[0].assertion_type,
            RustAssertionType::ResultOk
        ));
    }

    #[test]
    fn test_count_assertions() {
        let func: ItemFn = parse_quote! {
            #[test]
            fn test_count() {
                assert!(true);
                assert_eq!(1, 1);
            }
        };

        let mut detector = AssertionDetector::new();
        detector.analyze_assertions(&func);
        assert_eq!(detector.count_assertions(), 2);
    }
}
