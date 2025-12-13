//! Variable extraction from expressions and patterns.
//!
//! This module handles extracting variable references from syn expressions
//! and pattern bindings for CFG construction.

use syn::{Expr, Pat, Stmt};

use super::super::types::VarId;
use super::CfgBuilder;

impl CfgBuilder {
    /// Extract all variables referenced in an expression.
    /// Returns a list of VarIds for variables that appear in the expression.
    pub(super) fn extract_vars_from_expr(&mut self, expr: &Expr) -> Vec<VarId> {
        match expr {
            // Path: x, foo::bar
            Expr::Path(path) => self.extract_path_vars(path),

            // Field access: x.field, x.y.z
            Expr::Field(field) => self.extract_vars_from_expr(&field.base),

            // Method call: receiver.method(args)
            Expr::MethodCall(method) => self.extract_method_call_vars(method),

            // Binary: a + b, x && y
            Expr::Binary(binary) => {
                let mut vars = self.extract_vars_from_expr(&binary.left);
                vars.extend(self.extract_vars_from_expr(&binary.right));
                vars
            }

            // Unary: !x, *ptr, -n
            Expr::Unary(unary) => self.extract_vars_from_expr(&unary.expr),

            // Index: arr[i]
            Expr::Index(index) => {
                let mut vars = self.extract_vars_from_expr(&index.expr);
                vars.extend(self.extract_vars_from_expr(&index.index));
                vars
            }

            // Call: f(a, b, c)
            Expr::Call(call) => self.extract_call_vars(call),

            // Reference: &x, &mut x
            Expr::Reference(reference) => self.extract_vars_from_expr(&reference.expr),

            // Paren: (expr)
            Expr::Paren(paren) => self.extract_vars_from_expr(&paren.expr),

            // Block: { expr }
            Expr::Block(block) => self.extract_block_vars(block),

            // Tuple: (a, b, c)
            Expr::Tuple(tuple) => tuple
                .elems
                .iter()
                .flat_map(|e| self.extract_vars_from_expr(e))
                .collect(),

            // Cast: x as T
            Expr::Cast(cast) => self.extract_vars_from_expr(&cast.expr),

            // Array: [a, b, c]
            Expr::Array(array) => array
                .elems
                .iter()
                .flat_map(|e| self.extract_vars_from_expr(e))
                .collect(),

            // Repeat: [x; N]
            Expr::Repeat(repeat) => self.extract_vars_from_expr(&repeat.expr),

            // Struct: Foo { field: value }
            Expr::Struct(expr_struct) => expr_struct
                .fields
                .iter()
                .flat_map(|f| self.extract_vars_from_expr(&f.expr))
                .collect(),

            // Range: a..b, a..=b
            Expr::Range(range) => self.extract_range_vars(range),

            // Try: expr?
            Expr::Try(try_expr) => self.extract_vars_from_expr(&try_expr.expr),

            // Await: expr.await
            Expr::Await(await_expr) => self.extract_vars_from_expr(&await_expr.base),

            // Literals and other non-variable expressions
            Expr::Lit(_) => vec![],

            // Default: return empty for unsupported expressions
            _ => vec![],
        }
    }

    /// Extract variables from a path expression.
    fn extract_path_vars(&mut self, path: &syn::ExprPath) -> Vec<VarId> {
        if let Some(ident) = path.path.get_ident() {
            vec![self.get_or_create_var(&ident.to_string())]
        } else if let Some(seg) = path.path.segments.last() {
            vec![self.get_or_create_var(&seg.ident.to_string())]
        } else {
            vec![]
        }
    }

    /// Extract variables from a method call expression.
    fn extract_method_call_vars(&mut self, method: &syn::ExprMethodCall) -> Vec<VarId> {
        let mut vars = self.extract_vars_from_expr(&method.receiver);
        for arg in &method.args {
            vars.extend(self.extract_vars_from_expr(arg));
        }
        vars
    }

    /// Extract variables from a function call expression.
    fn extract_call_vars(&mut self, call: &syn::ExprCall) -> Vec<VarId> {
        let mut vars = self.extract_vars_from_expr(&call.func);
        for arg in &call.args {
            vars.extend(self.extract_vars_from_expr(arg));
        }
        vars
    }

    /// Extract variables from a block expression.
    fn extract_block_vars(&mut self, block: &syn::ExprBlock) -> Vec<VarId> {
        block
            .block
            .stmts
            .last()
            .and_then(|stmt| {
                if let Stmt::Expr(expr, _) = stmt {
                    Some(self.extract_vars_from_expr(expr))
                } else {
                    None
                }
            })
            .unwrap_or_default()
    }

    /// Extract variables from a range expression.
    fn extract_range_vars(&mut self, range: &syn::ExprRange) -> Vec<VarId> {
        let mut vars = Vec::new();
        if let Some(start) = &range.start {
            vars.extend(self.extract_vars_from_expr(start));
        }
        if let Some(end) = &range.end {
            vars.extend(self.extract_vars_from_expr(end));
        }
        vars
    }

    /// Extract the primary variable from an expression (for assignment targets, returns).
    /// Returns the first/main variable, or None if expression has no variable.
    pub(super) fn extract_primary_var(&mut self, expr: &Expr) -> Option<VarId> {
        self.extract_vars_from_expr(expr).into_iter().next()
    }

    /// Extract variable bindings from a pattern.
    pub(super) fn extract_vars_from_pattern(&mut self, pat: &Pat) -> Vec<VarId> {
        match pat {
            // Simple identifier: let x = ...
            Pat::Ident(pat_ident) => {
                vec![self.get_or_create_var(&pat_ident.ident.to_string())]
            }

            // Tuple: let (a, b) = ...
            Pat::Tuple(tuple) => tuple
                .elems
                .iter()
                .flat_map(|p| self.extract_vars_from_pattern(p))
                .collect(),

            // Struct: let Point { x, y } = ...
            Pat::Struct(pat_struct) => pat_struct
                .fields
                .iter()
                .flat_map(|field| self.extract_vars_from_pattern(&field.pat))
                .collect(),

            // TupleStruct: let Some(x) = ...
            Pat::TupleStruct(tuple_struct) => tuple_struct
                .elems
                .iter()
                .flat_map(|p| self.extract_vars_from_pattern(p))
                .collect(),

            // Slice: let [first, rest @ ..] = ...
            Pat::Slice(slice) => slice
                .elems
                .iter()
                .flat_map(|p| self.extract_vars_from_pattern(p))
                .collect(),

            // Reference: let &x = ... or let &mut x = ...
            Pat::Reference(reference) => self.extract_vars_from_pattern(&reference.pat),

            // Or: let A | B = ...
            Pat::Or(or) => or
                .cases
                .first()
                .map(|p| self.extract_vars_from_pattern(p))
                .unwrap_or_default(),

            // Type: let x: T = ...
            Pat::Type(pat_type) => self.extract_vars_from_pattern(&pat_type.pat),

            // Wildcard: let _ = ...
            Pat::Wild(_) => vec![],

            // Literal patterns: match on literal
            Pat::Lit(_) => vec![],

            // Rest: ..
            Pat::Rest(_) => vec![],

            // Range pattern: 1..=10
            Pat::Range(_) => vec![],

            // Path pattern: None, MyEnum::Variant
            Pat::Path(_) => vec![],

            // Const pattern
            Pat::Const(_) => vec![],

            // Paren pattern: (pat)
            Pat::Paren(paren) => self.extract_vars_from_pattern(&paren.pat),

            _ => vec![],
        }
    }
}
