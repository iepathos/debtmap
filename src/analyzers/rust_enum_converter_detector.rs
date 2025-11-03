//! AST-based enum converter detection for Rust
//!
//! This module implements detection of simple enum-to-string converter functions
//! to reduce false positives in priority ranking (Spec 124).
//!
//! # Problem
//!
//! Simple enum converter functions (e.g., `FrameworkType::name()`) are flagged
//! as CRITICAL business logic, but they're just data accessors with exhaustive
//! match expressions returning literals.
//!
//! # Detection Strategy
//!
//! 1. **Name Pattern**: Matches common converter names (name, as_str, to_string, etc.)
//! 2. **Match Expression**: Single exhaustive match on self or parameter
//! 3. **Literal Returns**: All match arms return only literals (strings, numbers, bools)
//! 4. **Low Complexity**: Cognitive complexity ≤ 3
//!
//! # Examples Detected
//!
//! ```rust,ignore
//! // Detected as enum converter
//! impl FrameworkType {
//!     pub fn name(&self) -> &'static str {
//!         match self {
//!             FrameworkType::Django => "Django",
//!             FrameworkType::Flask => "Flask",
//!         }
//!     }
//! }
//!
//! // NOT detected (has function calls)
//! impl BuiltinException {
//!     pub fn message(&self) -> String {
//!         match self {
//!             Self::ValueError => format!("Invalid value"),
//!             Self::TypeError => format!("Type error"),
//!         }
//!     }
//! }
//! ```

use crate::config::get_constructor_detection_config;
use crate::core::FunctionMetrics;
use syn::{Arm, Expr, ExprMatch, ItemFn};

/// Detect if a function is a simple enum converter
///
/// Returns true if:
/// - Function name matches converter patterns
/// - Has low cognitive complexity (≤ max_cognitive from config)
/// - Body contains a single exhaustive match returning only literals
pub fn is_enum_converter(func: &FunctionMetrics, syn_func: &ItemFn) -> bool {
    // Get configuration
    let config = get_constructor_detection_config();

    // Check name pattern matches converter names
    if !matches_converter_name(&func.name) {
        return false;
    }

    // Check cognitive complexity is low (reuse constructor config)
    if func.cognitive > config.max_cognitive {
        return false;
    }

    // Analyze function body for exhaustive literal match
    if let Some(match_expr) = find_single_match_expr(&syn_func.block) {
        if is_exhaustive_literal_match(match_expr) {
            return true;
        }
    }

    false
}

/// Check if function name matches common converter patterns
fn matches_converter_name(name: &str) -> bool {
    let converter_patterns = [
        "name",
        "as_str",
        "as_",
        "to_str",
        "to_string",
        "to_",
        "is_",
        "value",
        "id",
        "kind",
        "variant",
    ];

    let name_lower = name.to_lowercase();
    converter_patterns
        .iter()
        .any(|pattern| name_lower == *pattern || name_lower.starts_with(pattern))
}

/// Find a single match expression in the function body
///
/// Returns Some if function body is a single match expression or
/// a single return statement containing a match.
fn find_single_match_expr(block: &syn::Block) -> Option<&ExprMatch> {
    // Check if block has exactly one statement
    if block.stmts.is_empty() || block.stmts.len() > 1 {
        return None;
    }

    // Get the single statement
    let stmt = &block.stmts[0];

    // Expression statement (implicit return or explicit)
    if let syn::Stmt::Expr(expr, _) = stmt {
        if let Expr::Match(match_expr) = expr {
            return Some(match_expr);
        }
        // Check for explicit return with match
        if let Expr::Return(ret_expr) = expr {
            if let Some(ret_value) = &ret_expr.expr {
                if let Expr::Match(match_expr) = ret_value.as_ref() {
                    return Some(match_expr);
                }
            }
        }
    }

    None
}

/// Check if match expression has only literal return values
///
/// Returns true if:
/// - Match target is simple (self or single variable)
/// - All arms have no guards
/// - All arms return literal expressions
fn is_exhaustive_literal_match(match_expr: &ExprMatch) -> bool {
    // Check match is on self or single param
    if !is_simple_match_target(&match_expr.expr) {
        return false;
    }

    // Check all arms return literals
    match_expr.arms.iter().all(is_literal_arm)
}

/// Check if match target is simple (self or single variable)
fn is_simple_match_target(expr: &Expr) -> bool {
    match expr {
        // Match on *self or self
        Expr::Unary(unary) if matches!(unary.op, syn::UnOp::Deref(_)) => {
            matches!(&*unary.expr, Expr::Path(_))
        }
        // Match on self or a variable
        Expr::Path(_) => true,
        _ => false,
    }
}

/// Check if match arm returns only a literal
fn is_literal_arm(arm: &Arm) -> bool {
    // Arm must not have guard
    if arm.guard.is_some() {
        return false;
    }

    // Arm body must be literal expression
    is_literal_expr(&arm.body)
}

/// Check if expression is a literal (string, number, bool)
fn is_literal_expr(expr: &Expr) -> bool {
    match expr {
        // Direct literals (strings, numbers, chars, bytes, bools)
        Expr::Lit(_) => true,
        // Path expressions for true/false/None
        Expr::Path(path) => {
            let path_str = quote::quote!(#path).to_string();
            matches!(path_str.as_str(), "true" | "false" | "None")
        }
        // Block with single literal expression
        Expr::Block(block) if block.block.stmts.len() == 1 => {
            if let Some(syn::Stmt::Expr(inner_expr, _)) = block.block.stmts.first() {
                is_literal_expr(inner_expr)
            } else {
                false
            }
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::FunctionMetrics;
    use std::path::PathBuf;
    use syn::parse_quote;

    fn create_test_metrics(name: &str, cyclomatic: u32, cognitive: u32) -> FunctionMetrics {
        FunctionMetrics {
            file: PathBuf::from("test.rs"),
            name: name.to_string(),
            line: 1,
            length: 10,
            cyclomatic,
            cognitive,
            nesting: 0,
            is_test: false,
            visibility: None,
            is_trait_method: false,
            in_test_module: false,
            entropy_score: None,
            is_pure: None,
            purity_confidence: None,
            purity_reason: None,
            call_dependencies: None,
            detected_patterns: None,
            upstream_callers: None,
            downstream_callees: None,
            mapping_pattern_result: None,
            adjusted_complexity: None,
            composition_metrics: None,
            language_specific: None,
            purity_level: None,
        }
    }

    #[test]
    fn test_framework_type_name_detected() {
        let code: ItemFn = parse_quote! {
            pub fn name(&self) -> &'static str {
                match self {
                    FrameworkType::Django => "Django",
                    FrameworkType::Flask => "Flask",
                }
            }
        };

        let metrics = create_test_metrics("name", 2, 0);
        assert!(
            is_enum_converter(&metrics, &code),
            "Should detect FrameworkType::name as enum converter"
        );
    }

    #[test]
    fn test_builtin_exception_as_str_detected() {
        let code: ItemFn = parse_quote! {
            fn as_str(&self) -> &str {
                match self {
                    Self::BaseException => "BaseException",
                    Self::ValueError => "ValueError",
                    Self::TypeError => "TypeError",
                }
            }
        };

        let metrics = create_test_metrics("as_str", 3, 0);
        assert!(
            is_enum_converter(&metrics, &code),
            "Should detect BuiltinException::as_str as enum converter"
        );
    }

    #[test]
    fn test_function_call_in_match_not_detected() {
        let code: ItemFn = parse_quote! {
            pub fn process(&self) -> String {
                match self {
                    Variant::A => format!("A"),
                    Variant::B => format!("B"),
                }
            }
        };

        let metrics = create_test_metrics("process", 2, 1);
        assert!(
            !is_enum_converter(&metrics, &code),
            "Should NOT detect converter with function calls"
        );
    }

    #[test]
    fn test_high_cognitive_complexity_rejected() {
        let code: ItemFn = parse_quote! {
            pub fn name(&self) -> &'static str {
                match self {
                    Type::A => "A",
                    Type::B => "B",
                }
            }
        };

        let metrics = create_test_metrics("name", 2, 5); // cognitive = 5
        assert!(
            !is_enum_converter(&metrics, &code),
            "Should reject high cognitive complexity"
        );
    }

    #[test]
    fn test_match_with_guard_not_detected() {
        let code: ItemFn = parse_quote! {
            pub fn name(&self) -> &'static str {
                match self {
                    Type::A if condition => "A",
                    Type::B => "B",
                    _ => "Unknown",
                }
            }
        };

        let metrics = create_test_metrics("name", 3, 1);
        assert!(
            !is_enum_converter(&metrics, &code),
            "Should NOT detect match with guards"
        );
    }

    #[test]
    fn test_nested_match_not_detected() {
        let code: ItemFn = parse_quote! {
            pub fn value(&self) -> i32 {
                match self {
                    Type::A => match inner {
                        Inner::X => 1,
                        Inner::Y => 2,
                    },
                    Type::B => 3,
                }
            }
        };

        let metrics = create_test_metrics("value", 4, 2);
        assert!(
            !is_enum_converter(&metrics, &code),
            "Should NOT detect nested match expressions"
        );
    }

    #[test]
    fn test_numeric_literal_converter() {
        let code: ItemFn = parse_quote! {
            pub fn id(&self) -> i32 {
                match self {
                    Status::Active => 1,
                    Status::Inactive => 0,
                    Status::Pending => 2,
                }
            }
        };

        let metrics = create_test_metrics("id", 3, 0);
        assert!(
            is_enum_converter(&metrics, &code),
            "Should detect numeric literal converter"
        );
    }

    #[test]
    fn test_boolean_literal_converter() {
        let code: ItemFn = parse_quote! {
            pub fn is_active(&self) -> bool {
                match self {
                    Status::Active => true,
                    Status::Inactive => false,
                    Status::Pending => false,
                }
            }
        };

        let metrics = create_test_metrics("is_active", 3, 0);
        assert!(
            is_enum_converter(&metrics, &code),
            "Should detect boolean literal converter"
        );
    }

    #[test]
    fn test_matches_converter_name() {
        // Exact matches
        assert!(matches_converter_name("name"));
        assert!(matches_converter_name("value"));
        assert!(matches_converter_name("id"));
        assert!(matches_converter_name("kind"));
        assert!(matches_converter_name("variant"));

        // Prefix matches
        assert!(matches_converter_name("as_str"));
        assert!(matches_converter_name("as_string"));
        assert!(matches_converter_name("to_str"));
        assert!(matches_converter_name("to_string"));

        // Non-matches
        assert!(!matches_converter_name("calculate"));
        assert!(!matches_converter_name("process"));
        assert!(!matches_converter_name("validate"));
    }

    #[test]
    fn test_is_literal_expr() {
        // String literal
        let expr: Expr = parse_quote! { "test" };
        assert!(is_literal_expr(&expr));

        // Numeric literal
        let expr: Expr = parse_quote! { 42 };
        assert!(is_literal_expr(&expr));

        // Boolean literal
        let expr: Expr = parse_quote! { true };
        assert!(is_literal_expr(&expr));

        // None
        let expr: Expr = parse_quote! { None };
        assert!(is_literal_expr(&expr));

        // Function call (not literal)
        let expr: Expr = parse_quote! { format!("test") };
        assert!(!is_literal_expr(&expr));

        // Variable reference (not literal)
        let expr: Expr = parse_quote! { some_var };
        assert!(!is_literal_expr(&expr));
    }

    #[test]
    fn test_is_simple_match_target() {
        // Match on self
        let expr: Expr = parse_quote! { self };
        assert!(is_simple_match_target(&expr));

        // Match on *self
        let expr: Expr = parse_quote! { *self };
        assert!(is_simple_match_target(&expr));

        // Match on variable
        let expr: Expr = parse_quote! { value };
        assert!(is_simple_match_target(&expr));

        // Match on complex expression (not simple)
        let expr: Expr = parse_quote! { self.field };
        assert!(!is_simple_match_target(&expr));

        // Match on function call (not simple)
        let expr: Expr = parse_quote! { get_value() };
        assert!(!is_simple_match_target(&expr));
    }

    #[test]
    fn test_find_single_match_expr() {
        // Single match expression (implicit return)
        let func: ItemFn = parse_quote! {
            fn name(&self) -> &str {
                match self {
                    Type::A => "A",
                }
            }
        };
        assert!(find_single_match_expr(&func.block).is_some());

        // Explicit return with match
        let func: ItemFn = parse_quote! {
            fn name(&self) -> &str {
                return match self {
                    Type::A => "A",
                };
            }
        };
        assert!(find_single_match_expr(&func.block).is_some());

        // Multiple statements (not single match)
        let func: ItemFn = parse_quote! {
            fn name(&self) -> &str {
                let x = 10;
                match self {
                    Type::A => "A",
                }
            }
        };
        assert!(find_single_match_expr(&func.block).is_none());
    }
}
