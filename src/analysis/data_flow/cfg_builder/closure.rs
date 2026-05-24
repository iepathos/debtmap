//! Closure capture analysis for CFG construction.
//!
//! This module provides the visitor and helpers for detecting which variables
//! are captured by closures and how (by value, by reference, or by mutable reference).

use std::collections::HashSet;

use syn::visit::Visit;
use syn::{Expr, ExprAssign, ExprBinary, ExprClosure, ExprMethodCall, Pat};

use super::super::types::CaptureMode;

/// Information about a capture detected during closure body analysis.
#[derive(Debug, Clone)]
pub(super) struct CaptureInfo {
    /// Name of the captured variable
    pub var_name: String,
    /// Inferred capture mode
    pub mode: CaptureMode,
    /// Whether the variable is mutated in the closure body
    pub is_mutated: bool,
}

/// Visitor to detect captured variables in closure body.
///
/// Walks the closure body AST and identifies variables that:
/// 1. Are referenced in the closure body
/// 2. Are defined in the outer scope (not closure parameters)
/// 3. Are not special names like `self` or `Self`
pub(super) struct ClosureCaptureVisitor<'a> {
    /// Variables available in outer scope (potential captures)
    outer_scope: &'a HashSet<String>,
    /// Closure parameters (not captures)
    closure_params: &'a HashSet<String>,
    /// Detected captures
    captures: Vec<CaptureInfo>,
    /// Variables mutated in closure body
    mutated_vars: HashSet<String>,
    /// Whether this is a move closure
    is_move: bool,
}

impl<'a> ClosureCaptureVisitor<'a> {
    pub fn new(
        outer_scope: &'a HashSet<String>,
        closure_params: &'a HashSet<String>,
        is_move: bool,
    ) -> Self {
        Self {
            outer_scope,
            closure_params,
            captures: Vec::new(),
            mutated_vars: HashSet::new(),
            is_move,
        }
    }

    /// Finalize capture detection by updating capture modes based on mutation info.
    pub fn finalize_captures(mut self) -> Vec<CaptureInfo> {
        for capture in &mut self.captures {
            if self.mutated_vars.contains(&capture.var_name) {
                capture.is_mutated = true;
                if !self.is_move {
                    capture.mode = CaptureMode::ByMutRef;
                }
            }
        }
        self.captures
    }

    /// Record a variable as mutated.
    fn record_mutation(&mut self, name: &str) {
        self.mutated_vars.insert(name.to_string());
    }

    /// Try to record a variable reference as a capture.
    fn try_record_capture(&mut self, name: String) {
        // Skip special names
        if name == "self" || name == "Self" {
            return;
        }

        // Check if it's from outer scope (not a closure param)
        if self.outer_scope.contains(&name) && !self.closure_params.contains(&name) {
            // Check if already captured
            if !self.captures.iter().any(|c| c.var_name == name) {
                self.captures.push(CaptureInfo {
                    var_name: name,
                    mode: if self.is_move {
                        CaptureMode::ByValue
                    } else {
                        CaptureMode::ByRef
                    },
                    is_mutated: false,
                });
            }
        }
    }

    fn try_record_path_capture(&mut self, expr: &Expr) {
        if let Some(name) = path_ident(expr) {
            self.try_record_capture(name);
        }
    }

    fn record_path_mutation(&mut self, expr: &Expr) {
        if let Some(name) = path_ident(expr) {
            self.record_mutation(&name);
            self.try_record_capture(name);
        }
    }

    fn visit_method_call(&mut self, method_call: &ExprMethodCall) {
        self.visit_expr(&method_call.receiver);
        self.record_mutating_method_receiver(method_call);
        for arg in &method_call.args {
            self.visit_expr(arg);
        }
    }

    fn record_mutating_method_receiver(&mut self, method_call: &ExprMethodCall) {
        let method_name = method_call.method.to_string();
        if is_mutating_method(&method_name) {
            self.record_path_mutation(&method_call.receiver);
        }
    }

    fn visit_assign(&mut self, assign: &ExprAssign) {
        self.record_path_mutation(&assign.left);
        self.visit_expr(&assign.right);
    }

    fn visit_binary(&mut self, binary: &ExprBinary) {
        if is_compound_assignment(&binary.op) {
            self.record_path_mutation(&binary.left);
        }
        self.visit_expr(&binary.left);
        self.visit_expr(&binary.right);
    }

    fn visit_nested_closure(&mut self, nested_closure: &ExprClosure) {
        let nested_params = nested_closure_params(nested_closure);
        let nested_is_move = nested_closure.capture.is_some();
        let mut nested_visitor =
            ClosureCaptureVisitor::new(self.outer_scope, &nested_params, nested_is_move);
        nested_visitor.visit_expr(&nested_closure.body);
        for capture in nested_visitor.finalize_captures() {
            self.push_unique_capture(capture);
        }
    }

    fn push_unique_capture(&mut self, capture: CaptureInfo) {
        if !self
            .captures
            .iter()
            .any(|existing| existing.var_name == capture.var_name)
        {
            self.captures.push(capture);
        }
    }
}

impl<'ast, 'a> Visit<'ast> for ClosureCaptureVisitor<'a> {
    fn visit_expr(&mut self, expr: &'ast Expr) {
        match expr {
            // Variable reference - potential capture
            Expr::Path(_) => self.try_record_path_capture(expr),
            // Method call - check receiver
            Expr::MethodCall(method_call) => self.visit_method_call(method_call),
            // Assignment - track mutation
            Expr::Assign(assign) => self.visit_assign(assign),
            // Binary operation that might be compound assignment (+=, -=, etc.)
            Expr::Binary(binary) => self.visit_binary(binary),
            // Nested closure - recurse with combined scope
            Expr::Closure(nested_closure) => self.visit_nested_closure(nested_closure),
            // Default: recurse into children
            _ => {
                syn::visit::visit_expr(self, expr);
            }
        }
    }
}

fn path_ident(expr: &Expr) -> Option<String> {
    match expr {
        Expr::Path(path) => path.path.get_ident().map(ToString::to_string),
        _ => None,
    }
}

fn nested_closure_params(closure: &ExprClosure) -> HashSet<String> {
    closure
        .inputs
        .iter()
        .filter_map(extract_pattern_name)
        .collect()
}

/// Check if a binary operator is a compound assignment.
fn is_compound_assignment(op: &syn::BinOp) -> bool {
    matches!(
        op,
        syn::BinOp::AddAssign(_)
            | syn::BinOp::SubAssign(_)
            | syn::BinOp::MulAssign(_)
            | syn::BinOp::DivAssign(_)
            | syn::BinOp::RemAssign(_)
            | syn::BinOp::BitAndAssign(_)
            | syn::BinOp::BitOrAssign(_)
            | syn::BinOp::BitXorAssign(_)
            | syn::BinOp::ShlAssign(_)
            | syn::BinOp::ShrAssign(_)
    )
}

/// Check if a method name indicates mutation.
pub(super) fn is_mutating_method(name: &str) -> bool {
    matches!(
        name,
        "push"
            | "pop"
            | "insert"
            | "remove"
            | "clear"
            | "extend"
            | "drain"
            | "append"
            | "truncate"
            | "reserve"
            | "shrink_to_fit"
            | "set"
            | "swap"
            | "sort"
            | "sort_by"
            | "sort_by_key"
            | "dedup"
            | "retain"
            | "resize"
    )
}

/// Extract the variable name from a pattern.
pub(super) fn extract_pattern_name(pat: &Pat) -> Option<String> {
    match pat {
        Pat::Ident(pat_ident) => Some(pat_ident.ident.to_string()),
        Pat::Type(pat_type) => extract_pattern_name(&pat_type.pat),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    fn names(names: &[&str]) -> HashSet<String> {
        names.iter().map(ToString::to_string).collect()
    }

    fn captures_for(
        expr: Expr,
        outer: &[&str],
        params: &[&str],
        is_move: bool,
    ) -> Vec<CaptureInfo> {
        let outer_scope = names(outer);
        let closure_params = names(params);
        let mut visitor = ClosureCaptureVisitor::new(&outer_scope, &closure_params, is_move);
        visitor.visit_expr(&expr);
        visitor.finalize_captures()
    }

    fn capture<'a>(captures: &'a [CaptureInfo], name: &str) -> &'a CaptureInfo {
        captures
            .iter()
            .find(|capture| capture.var_name == name)
            .unwrap_or_else(|| panic!("missing capture for {name}"))
    }

    #[test]
    fn path_records_outer_variable_capture() {
        let captures = captures_for(parse_quote!(outer_value), &["outer_value"], &[], false);

        assert_eq!(capture(&captures, "outer_value").mode, CaptureMode::ByRef);
    }

    #[test]
    fn path_skips_closure_parameters_and_special_names() {
        let expr = parse_quote!((outer_value, self, Self, item));
        let captures = captures_for(
            expr,
            &["outer_value", "self", "Self", "item"],
            &["item"],
            false,
        );

        assert_eq!(captures.len(), 1);
        assert_eq!(captures[0].var_name, "outer_value");
    }

    #[test]
    fn assignment_records_mutable_capture() {
        let captures = captures_for(parse_quote!(counter = 1), &["counter"], &[], false);
        let counter = capture(&captures, "counter");

        assert_eq!(counter.mode, CaptureMode::ByMutRef);
        assert!(counter.is_mutated);
    }

    #[test]
    fn move_assignment_stays_by_value_and_mutated() {
        let captures = captures_for(parse_quote!(counter = 1), &["counter"], &[], true);
        let counter = capture(&captures, "counter");

        assert_eq!(counter.mode, CaptureMode::ByValue);
        assert!(counter.is_mutated);
    }

    #[test]
    fn mutating_method_records_mutable_receiver_capture() {
        let captures = captures_for(
            parse_quote!(items.push(value)),
            &["items", "value"],
            &[],
            false,
        );

        let items = capture(&captures, "items");
        assert_eq!(items.mode, CaptureMode::ByMutRef);
        assert!(items.is_mutated);
        assert_eq!(capture(&captures, "value").mode, CaptureMode::ByRef);
    }

    #[test]
    fn non_mutating_method_records_receiver_and_args_by_ref() {
        let captures = captures_for(
            parse_quote!(items.contains(value)),
            &["items", "value"],
            &[],
            false,
        );

        assert_eq!(capture(&captures, "items").mode, CaptureMode::ByRef);
        assert!(!capture(&captures, "items").is_mutated);
        assert_eq!(capture(&captures, "value").mode, CaptureMode::ByRef);
    }

    #[test]
    fn compound_assignment_records_mutable_capture() {
        let captures = captures_for(
            parse_quote!(total += amount),
            &["total", "amount"],
            &[],
            false,
        );

        let total = capture(&captures, "total");
        assert_eq!(total.mode, CaptureMode::ByMutRef);
        assert!(total.is_mutated);
        assert_eq!(capture(&captures, "amount").mode, CaptureMode::ByRef);
    }

    #[test]
    fn nested_closure_propagates_unique_outer_captures() {
        let captures = captures_for(parse_quote!(|| value + value), &["value"], &[], false);

        assert_eq!(captures.len(), 1);
        assert_eq!(capture(&captures, "value").mode, CaptureMode::ByRef);
    }

    #[test]
    fn typed_pattern_name_is_extracted() {
        let closure: ExprClosure = parse_quote!(|item: usize| item);

        assert_eq!(
            extract_pattern_name(&closure.inputs[0]),
            Some("item".to_string())
        );
    }
}
