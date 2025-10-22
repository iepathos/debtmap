//! AST-based constructor detection for Rust
//!
//! This module enhances name-based constructor detection (Spec 117)
//! with AST analysis to catch non-standard patterns.
//!
//! # Detection Strategy
//!
//! 1. **Return Type**: Function returns `Self` (or `Result<Self>`, `Option<Self>`)
//! 2. **Body Pattern**: Struct initialization or simple field assignments
//! 3. **Complexity**: Low cyclomatic (≤5), no loops, minimal branching
//!
//! # Examples Caught
//!
//! ```rust,ignore
//! // Non-standard name (missed by name-based)
//! pub fn create_default_client() -> Self {
//!     Self { timeout: Duration::from_secs(30) }
//! }
//!
//! // Builder method (different role)
//! pub fn set_timeout(mut self, timeout: Duration) -> Self {
//!     self.timeout = timeout;
//!     self
//! }
//! ```
//!
//! # Fallback
//!
//! If AST unavailable (syntax errors, unsupported language):
//! - Falls back to name-based detection (Spec 117)
//! - Graceful degradation, no failures

use syn::{visit::Visit, Expr, ItemFn, ReturnType as SynReturnType, Type};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConstructorReturnType {
    OwnedSelf,  // -> Self
    ResultSelf, // -> Result<Self, E>
    OptionSelf, // -> Option<Self>
    RefSelf,    // -> &Self or &mut Self (builder pattern)
    Other,      // Other types
}

/// Extract return type from function signature using syn
pub fn extract_return_type(func: &ItemFn) -> Option<ConstructorReturnType> {
    match &func.sig.output {
        SynReturnType::Default => None, // No return type
        SynReturnType::Type(_, ty) => classify_return_type(ty),
    }
}

/// Classify return type for constructor detection
fn classify_return_type(ty: &Type) -> Option<ConstructorReturnType> {
    match ty {
        Type::Path(type_path) => {
            let path_str = quote::quote!(#type_path).to_string();

            if path_str == "Self" {
                Some(ConstructorReturnType::OwnedSelf)
            } else if path_str.starts_with("Result < Self") {
                Some(ConstructorReturnType::ResultSelf)
            } else if path_str.starts_with("Option < Self") {
                Some(ConstructorReturnType::OptionSelf)
            } else {
                Some(ConstructorReturnType::Other)
            }
        }
        Type::Reference(type_ref) => {
            if let Type::Path(path) = &*type_ref.elem {
                let path_str = quote::quote!(#path).to_string();
                if path_str == "Self" {
                    return Some(ConstructorReturnType::RefSelf);
                }
            }
            Some(ConstructorReturnType::Other)
        }
        _ => Some(ConstructorReturnType::Other),
    }
}

/// Visitor to detect constructor patterns in function body
pub struct ConstructorPatternVisitor {
    pattern: BodyPattern,
}

impl ConstructorPatternVisitor {
    pub fn new() -> Self {
        Self {
            pattern: BodyPattern::default(),
        }
    }

    pub fn into_pattern(self) -> BodyPattern {
        self.pattern
    }
}

impl Default for ConstructorPatternVisitor {
    fn default() -> Self {
        Self::new()
    }
}

impl<'ast> Visit<'ast> for ConstructorPatternVisitor {
    fn visit_expr(&mut self, expr: &'ast Expr) {
        match expr {
            Expr::Struct(_) => {
                self.pattern.struct_init_count += 1;
            }
            Expr::Path(path) => {
                // Check for Self references
                let path_str = quote::quote!(#path).to_string();
                if path_str.starts_with("Self") {
                    self.pattern.self_refs += 1;
                }
            }
            Expr::If(_) => self.pattern.has_if = true,
            Expr::Match(_) => self.pattern.has_match = true,
            Expr::Loop(_) | Expr::While(_) | Expr::ForLoop(_) => {
                self.pattern.has_loop = true;
            }
            Expr::Return(_) => self.pattern.early_returns += 1,
            Expr::Field(_) | Expr::Assign(_) => {
                self.pattern.field_assignments += 1;
            }
            _ => {}
        }
        syn::visit::visit_expr(self, expr);
    }
}

/// Analyze function body for constructor patterns
pub fn analyze_function_body(func: &ItemFn) -> BodyPattern {
    let mut visitor = ConstructorPatternVisitor::new();
    visitor.visit_block(&func.block);
    visitor.into_pattern()
}

#[derive(Debug, Clone, Default)]
pub struct BodyPattern {
    pub struct_init_count: usize,
    pub self_refs: usize,
    pub field_assignments: usize,
    pub has_if: bool,
    pub has_match: bool,
    pub has_loop: bool,
    pub early_returns: usize,
}

impl BodyPattern {
    /// Does this look like a constructor body?
    pub fn is_constructor_like(&self) -> bool {
        // Has struct initialization and no loops
        (self.struct_init_count > 0 && !self.has_loop)
        ||
        // Or minimal logic (≤1 if/match) with Self refs
        (self.self_refs > 0 && !self.has_loop && !self.has_match && self.field_assignments == 0)
    }

    /// Does this look like a builder method? (Phase 3 - not implemented yet)
    #[allow(dead_code)]
    pub fn is_builder_like(&self) -> bool {
        // Modifies fields and returns self
        self.field_assignments > 0 && self.early_returns <= 1 && !self.has_loop
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn test_extract_return_type_owned_self() {
        let func: ItemFn = parse_quote! {
            fn new() -> Self {
                Self { field: 0 }
            }
        };
        assert_eq!(
            extract_return_type(&func),
            Some(ConstructorReturnType::OwnedSelf)
        );
    }

    #[test]
    fn test_extract_return_type_result_self() {
        let func: ItemFn = parse_quote! {
            fn try_new() -> Result<Self, Error> {
                Ok(Self { field: 0 })
            }
        };
        assert_eq!(
            extract_return_type(&func),
            Some(ConstructorReturnType::ResultSelf)
        );
    }

    #[test]
    fn test_extract_return_type_option_self() {
        let func: ItemFn = parse_quote! {
            fn maybe_new() -> Option<Self> {
                Some(Self { field: 0 })
            }
        };
        assert_eq!(
            extract_return_type(&func),
            Some(ConstructorReturnType::OptionSelf)
        );
    }

    #[test]
    fn test_extract_return_type_ref_self() {
        let func: ItemFn = parse_quote! {
            fn get_self(&self) -> &Self {
                self
            }
        };
        assert_eq!(
            extract_return_type(&func),
            Some(ConstructorReturnType::RefSelf)
        );
    }

    #[test]
    fn test_extract_return_type_other() {
        let func: ItemFn = parse_quote! {
            fn get_value() -> i32 {
                42
            }
        };
        assert_eq!(
            extract_return_type(&func),
            Some(ConstructorReturnType::Other)
        );
    }

    #[test]
    fn test_extract_return_type_none() {
        let func: ItemFn = parse_quote! {
            fn do_something() {
                println!("Hello");
            }
        };
        assert_eq!(extract_return_type(&func), None);
    }

    #[test]
    fn test_analyze_function_body_struct_init() {
        let func: ItemFn = parse_quote! {
            fn new() -> Self {
                Self { field: 0 }
            }
        };
        let pattern = analyze_function_body(&func);
        assert_eq!(pattern.struct_init_count, 1);
        assert!(pattern.is_constructor_like());
    }

    #[test]
    fn test_analyze_function_body_with_loop() {
        let func: ItemFn = parse_quote! {
            fn process_items() -> Self {
                let mut result = Self::new();
                for item in items {
                    result.add(item);
                }
                result
            }
        };
        let pattern = analyze_function_body(&func);
        assert!(pattern.has_loop);
        assert!(!pattern.is_constructor_like());
    }

    #[test]
    fn test_analyze_function_body_self_refs() {
        let func: ItemFn = parse_quote! {
            fn default() -> Self {
                Self::new()
            }
        };
        let pattern = analyze_function_body(&func);
        assert!(pattern.self_refs > 0);
        assert!(pattern.is_constructor_like());
    }

    #[test]
    fn test_body_pattern_is_builder_like() {
        let func: ItemFn = parse_quote! {
            fn set_timeout(mut self, timeout: Duration) -> Self {
                self.timeout = timeout;
                self
            }
        };
        let pattern = analyze_function_body(&func);
        assert!(pattern.field_assignments > 0);
        assert!(pattern.is_builder_like());
    }
}
