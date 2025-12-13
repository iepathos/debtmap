//! Closure capture analysis for CFG construction.
//!
//! This module provides the visitor and helpers for detecting which variables
//! are captured by closures and how (by value, by reference, or by mutable reference).

use std::collections::HashSet;

use syn::visit::Visit;
use syn::{Expr, Pat};

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
}

impl<'ast, 'a> Visit<'ast> for ClosureCaptureVisitor<'a> {
    fn visit_expr(&mut self, expr: &'ast Expr) {
        match expr {
            // Variable reference - potential capture
            Expr::Path(path) => {
                if let Some(ident) = path.path.get_ident() {
                    self.try_record_capture(ident.to_string());
                }
            }
            // Method call - check receiver
            Expr::MethodCall(method_call) => {
                // Visit receiver separately to detect captures
                self.visit_expr(&method_call.receiver);
                // Check if method is mutating
                let method_name = method_call.method.to_string();
                if is_mutating_method(&method_name) {
                    if let Expr::Path(path) = &*method_call.receiver {
                        if let Some(ident) = path.path.get_ident() {
                            self.record_mutation(&ident.to_string());
                        }
                    }
                }
                // Visit args
                for arg in &method_call.args {
                    self.visit_expr(arg);
                }
            }
            // Assignment - track mutation
            Expr::Assign(assign) => {
                if let Expr::Path(path) = &*assign.left {
                    if let Some(ident) = path.path.get_ident() {
                        self.record_mutation(&ident.to_string());
                    }
                }
                // Visit RHS
                self.visit_expr(&assign.right);
            }
            // Binary operation that might be compound assignment (+=, -=, etc.)
            Expr::Binary(binary) => {
                if is_compound_assignment(&binary.op) {
                    if let Expr::Path(path) = &*binary.left {
                        if let Some(ident) = path.path.get_ident() {
                            self.record_mutation(&ident.to_string());
                        }
                    }
                }
                self.visit_expr(&binary.left);
                self.visit_expr(&binary.right);
            }
            // Nested closure - recurse with combined scope
            Expr::Closure(nested_closure) => {
                // Extract nested closure params
                let nested_params: HashSet<String> = nested_closure
                    .inputs
                    .iter()
                    .filter_map(extract_pattern_name)
                    .collect();

                let nested_is_move = nested_closure.capture.is_some();
                let mut nested_visitor =
                    ClosureCaptureVisitor::new(self.outer_scope, &nested_params, nested_is_move);
                nested_visitor.visit_expr(&nested_closure.body);

                // Propagate captures from nested closure
                for capture in nested_visitor.finalize_captures() {
                    if !self.captures.iter().any(|c| c.var_name == capture.var_name) {
                        self.captures.push(capture);
                    }
                }
            }
            // Default: recurse into children
            _ => {
                syn::visit::visit_expr(self, expr);
            }
        }
    }
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
