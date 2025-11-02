use crate::analysis::multi_signal_aggregation::ResponsibilityCategory;
use crate::analysis::rust_patterns::context::RustFunctionContext;
use serde::{Deserialize, Serialize};
use syn::{visit::Visit, Expr, ExprTry, ReturnType, Type};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErrorPatternType {
    QuestionMarkOperator,
    CustomErrorType,
    ErrorConversion,
    UnwrapUsage,
    PanicUsage,
    ExpectUsage,
    UnreachableUsage,
    UnwrapOrDefaultUsage,
    OkUnwrapChain,
    ExpectErrUsage,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ErrorPattern {
    pub pattern_type: ErrorPatternType,
    pub count: usize,
    pub evidence: String,
}

/// AST visitor for error handling patterns
#[derive(Default)]
pub struct ErrorPatternVisitor {
    pub question_mark_count: usize,
    pub unwrap_count: usize,
    pub expect_count: usize,
    pub panic_count: usize,
    pub unreachable_count: usize,
    pub unwrap_or_default_count: usize,
    pub ok_unwrap_chain_count: usize,
    pub expect_err_count: usize,
}

impl ErrorPatternVisitor {
    pub fn new() -> Self {
        Self::default()
    }
}

impl<'ast> Visit<'ast> for ErrorPatternVisitor {
    fn visit_expr_try(&mut self, try_expr: &'ast ExprTry) {
        // The `?` operator
        self.question_mark_count += 1;
        syn::visit::visit_expr_try(self, try_expr);
    }

    fn visit_expr_method_call(&mut self, method: &'ast syn::ExprMethodCall) {
        let method_name = method.method.to_string();

        match method_name.as_str() {
            "unwrap" => {
                // Check if this is part of .ok().unwrap() chain
                if let Expr::MethodCall(inner) = &*method.receiver {
                    if inner.method == "ok" {
                        self.ok_unwrap_chain_count += 1;
                    }
                }
                self.unwrap_count += 1;
            }
            "expect" => self.expect_count += 1,
            "expect_err" => self.expect_err_count += 1,
            "unwrap_or_default" => self.unwrap_or_default_count += 1,
            _ => {}
        }

        syn::visit::visit_expr_method_call(self, method);
    }

    fn visit_macro(&mut self, mac: &'ast syn::Macro) {
        let macro_name = mac
            .path
            .segments
            .last()
            .map(|s| s.ident.to_string())
            .unwrap_or_default();

        match macro_name.as_str() {
            "panic" => self.panic_count += 1,
            "unreachable" => self.unreachable_count += 1,
            _ => {}
        }

        syn::visit::visit_macro(self, mac);
    }
}

pub struct RustErrorDetector;

impl RustErrorDetector {
    pub fn new() -> Self {
        Self
    }

    pub fn detect_error_patterns(&self, context: &RustFunctionContext) -> Vec<ErrorPattern> {
        let mut patterns = Vec::new();

        // Traverse AST for error handling
        let mut visitor = ErrorPatternVisitor::new();
        visitor.visit_block(context.body());

        // Question mark operator usage
        if visitor.question_mark_count > 0 {
            patterns.push(ErrorPattern {
                pattern_type: ErrorPatternType::QuestionMarkOperator,
                count: visitor.question_mark_count,
                evidence: format!("Uses ? operator {} times", visitor.question_mark_count),
            });
        }

        // Unwrap usage (anti-pattern)
        if visitor.unwrap_count > 0 {
            patterns.push(ErrorPattern {
                pattern_type: ErrorPatternType::UnwrapUsage,
                count: visitor.unwrap_count,
                evidence: format!(
                    "Uses unwrap() {} times (anti-pattern)",
                    visitor.unwrap_count
                ),
            });
        }

        // Expect usage (better than unwrap)
        if visitor.expect_count > 0 {
            patterns.push(ErrorPattern {
                pattern_type: ErrorPatternType::ExpectUsage,
                count: visitor.expect_count,
                evidence: format!("Uses expect() {} times", visitor.expect_count),
            });
        }

        // Panic usage (anti-pattern)
        if visitor.panic_count > 0 {
            patterns.push(ErrorPattern {
                pattern_type: ErrorPatternType::PanicUsage,
                count: visitor.panic_count,
                evidence: format!("Uses panic!() {} times (anti-pattern)", visitor.panic_count),
            });
        }

        // Unreachable usage (anti-pattern in production code)
        if visitor.unreachable_count > 0 {
            patterns.push(ErrorPattern {
                pattern_type: ErrorPatternType::UnreachableUsage,
                count: visitor.unreachable_count,
                evidence: format!("Uses unreachable!() {} times", visitor.unreachable_count),
            });
        }

        // Unwrap or default (silent error suppression)
        if visitor.unwrap_or_default_count > 0 {
            patterns.push(ErrorPattern {
                pattern_type: ErrorPatternType::UnwrapOrDefaultUsage,
                count: visitor.unwrap_or_default_count,
                evidence: format!(
                    "Uses unwrap_or_default() {} times (may hide errors)",
                    visitor.unwrap_or_default_count
                ),
            });
        }

        // Ok().unwrap() chain (particularly bad anti-pattern)
        if visitor.ok_unwrap_chain_count > 0 {
            patterns.push(ErrorPattern {
                pattern_type: ErrorPatternType::OkUnwrapChain,
                count: visitor.ok_unwrap_chain_count,
                evidence: format!(
                    "Uses .ok().unwrap() {} times (severe anti-pattern)",
                    visitor.ok_unwrap_chain_count
                ),
            });
        }

        // Expect_err usage (uncommon, may indicate test code)
        if visitor.expect_err_count > 0 {
            patterns.push(ErrorPattern {
                pattern_type: ErrorPatternType::ExpectErrUsage,
                count: visitor.expect_err_count,
                evidence: format!("Uses expect_err() {} times", visitor.expect_err_count),
            });
        }

        // Check return type for Result
        if let ReturnType::Type(_, ty) = &context.item_fn.sig.output {
            if Self::is_result_type(ty) {
                patterns.push(ErrorPattern {
                    pattern_type: ErrorPatternType::CustomErrorType,
                    count: 1,
                    evidence: "Returns Result type".into(),
                });
            }
        }

        patterns
    }

    fn is_result_type(ty: &Type) -> bool {
        if let Type::Path(type_path) = ty {
            if let Some(segment) = type_path.path.segments.last() {
                return segment.ident == "Result";
            }
        }
        false
    }

    pub fn classify_from_error_patterns(
        &self,
        patterns: &[ErrorPattern],
    ) -> Option<ResponsibilityCategory> {
        // High ? usage = Error Propagation & Handling
        let question_mark_count: usize = patterns
            .iter()
            .filter(|p| p.pattern_type == ErrorPatternType::QuestionMarkOperator)
            .map(|p| p.count)
            .sum();

        if question_mark_count >= 3 {
            return Some(ResponsibilityCategory::ErrorHandling);
        }

        // Custom error type = Error Handling
        if patterns
            .iter()
            .any(|p| p.pattern_type == ErrorPatternType::CustomErrorType)
        {
            return Some(ResponsibilityCategory::ErrorHandling);
        }

        None
    }
}

impl Default for RustErrorDetector {
    fn default() -> Self {
        Self::new()
    }
}
