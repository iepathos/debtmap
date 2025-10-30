//! Utility functions for Python type tracking
//!
//! This module contains pure helper functions for extracting information
//! from Python AST nodes.

use rustpython_parser::ast;

/// Helper function to extract callback expression as a string
pub(crate) fn extract_callback_expr_impl(expr: &ast::Expr) -> String {
    match expr {
        ast::Expr::Name(name) => name.id.to_string(),
        ast::Expr::Attribute(attr) => {
            if let ast::Expr::Name(obj) = &*attr.value {
                format!("{}.{}", obj.id, attr.attr)
            } else {
                attr.attr.to_string()
            }
        }
        ast::Expr::Call(call) => {
            // For functools.partial(func, ...) extract func
            if let ast::Expr::Attribute(attr) = &*call.func {
                if attr.attr.as_str() == "partial" {
                    if let Some(first_arg) = call.args.first() {
                        let func_name = extract_callback_expr_impl(first_arg);
                        return format!("partial({})", func_name);
                    }
                }
            }
            "<lambda>".to_string()
        }
        ast::Expr::Lambda(_) => "<lambda>".to_string(),
        _ => "<unknown>".to_string(),
    }
}

/// Helper function to extract full attribute name recursively
pub(crate) fn extract_attribute_name_recursive(expr: &ast::Expr) -> String {
    match expr {
        ast::Expr::Name(name) => name.id.to_string(),
        ast::Expr::Attribute(attr) => {
            let base = extract_attribute_name_recursive(&attr.value);
            format!("{}.{}", base, attr.attr)
        }
        _ => "<unknown>".to_string(),
    }
}
