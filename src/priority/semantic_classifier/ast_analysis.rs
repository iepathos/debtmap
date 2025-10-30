//! AST analysis helpers for semantic classification
//!
//! This module contains functions that analyze Rust AST structures
//! to determine function characteristics.

/// Check if function body is simple accessor pattern (AST analysis)
pub(crate) fn is_simple_accessor_body(syn_func: &syn::ItemFn) -> bool {
    // Function should take &self (not &mut self)
    if !has_immutable_self_receiver(syn_func) {
        return false;
    }

    // Single statement or expression
    let stmts = &syn_func.block.stmts;
    if stmts.is_empty() {
        return false;
    }

    // Check for simple patterns
    match stmts.len() {
        1 => {
            // Single expression: self.field, &self.field, self.field.clone()
            match &stmts[0] {
                syn::Stmt::Expr(expr, _) => is_simple_accessor_expr(expr),
                _ => false,
            }
        }
        2 => {
            // Let binding + return: let x = self.field; x
            // This is acceptable for accessors
            is_simple_binding_pattern(stmts)
        }
        _ => false, // Multiple statements - too complex
    }
}

/// Check if expression is simple accessor pattern
fn is_simple_accessor_expr(expr: &syn::Expr) -> bool {
    match expr {
        // Direct field access: self.field
        syn::Expr::Field(field_expr) => {
            matches!(&*field_expr.base, syn::Expr::Path(path)
                if path.path.is_ident("self"))
        }

        // Reference to field: &self.field
        syn::Expr::Reference(ref_expr) => is_simple_accessor_expr(&ref_expr.expr),

        // Method call on field: self.field.clone()
        syn::Expr::MethodCall(method_call) => {
            // Must be called on self.field
            is_simple_accessor_expr(&method_call.receiver)
                // Common accessor methods
                && is_simple_accessor_method(&method_call.method)
        }

        // Simple match or if (for bool accessors)
        syn::Expr::Match(_) | syn::Expr::If(_) => {
            // Already validated by complexity metrics
            // If cognitive ≤ 1, it's simple enough
            true
        }

        _ => false,
    }
}

/// Check if method is a simple accessor method
fn is_simple_accessor_method(method: &syn::Ident) -> bool {
    matches!(
        method.to_string().as_str(),
        "clone" | "to_string" | "as_ref" | "as_str" | "as_bytes" | "copied"
    )
}

/// Check if function has immutable self receiver
pub(crate) fn has_immutable_self_receiver(syn_func: &syn::ItemFn) -> bool {
    if let Some(syn::FnArg::Receiver(receiver)) = syn_func.sig.inputs.first() {
        receiver.mutability.is_none()
    } else {
        false
    }
}

/// Check if statements follow simple binding pattern
fn is_simple_binding_pattern(stmts: &[syn::Stmt]) -> bool {
    if stmts.len() != 2 {
        return false;
    }

    // First statement should be a let binding
    let _binding = match &stmts[0] {
        syn::Stmt::Local(_) => true,
        _ => return false,
    };

    // Second statement should be an expression (return value)
    matches!(&stmts[1], syn::Stmt::Expr(_, _))
}
