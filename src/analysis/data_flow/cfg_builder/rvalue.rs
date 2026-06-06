//! Rvalue conversion for CFG construction.
//!
//! This module handles converting syn expressions into CFG Rvalue types,
//! including operator conversion and function name extraction.

use syn::{Expr, Stmt};

use super::super::types::{BinOp, Rvalue, UnOp, VarId};
use super::CfgBuilder;

impl CfgBuilder {
    /// Convert an expression to an Rvalue, extracting actual variables.
    pub(super) fn expr_to_rvalue(&mut self, expr: &Expr) -> Rvalue {
        self.convert_wrapped_expr_to_rvalue(expr)
            .or_else(|| self.convert_leaf_expr_to_rvalue(expr))
            .unwrap_or(Rvalue::Constant)
    }

    fn convert_wrapped_expr_to_rvalue(&mut self, expr: &Expr) -> Option<Rvalue> {
        match expr {
            Expr::Paren(paren) => Some(self.expr_to_rvalue(&paren.expr)),
            Expr::Cast(cast) => Some(self.expr_to_rvalue(&cast.expr)),
            Expr::Block(block) => Some(self.convert_block_to_rvalue(block)),
            _ => None,
        }
    }

    fn convert_leaf_expr_to_rvalue(&mut self, expr: &Expr) -> Option<Rvalue> {
        match expr {
            Expr::Path(path) => self.convert_path_to_rvalue(path),
            Expr::Binary(binary) => Some(self.convert_binary_to_rvalue(binary)),
            Expr::Unary(unary) => Some(self.convert_unary_to_rvalue(unary)),
            Expr::Field(field) => Some(self.convert_field_to_rvalue(field)),
            Expr::Reference(reference) => Some(self.convert_reference_to_rvalue(reference)),
            Expr::Call(call) => Some(self.convert_call_to_rvalue(call)),
            Expr::MethodCall(method) => Some(self.convert_method_call_to_rvalue(method)),
            Expr::Index(index) => Some(self.convert_index_to_rvalue(index)),
            Expr::Lit(_) => Some(Rvalue::Constant),
            _ => None,
        }
    }

    fn convert_path_to_rvalue(&mut self, path: &syn::ExprPath) -> Option<Rvalue> {
        self.extract_primary_var(&Expr::Path(path.clone()))
            .map(Rvalue::Use)
    }

    /// Convert a binary expression to an Rvalue.
    fn convert_binary_to_rvalue(&mut self, binary: &syn::ExprBinary) -> Rvalue {
        let left = self.extract_primary_var(&binary.left);
        let right = self.extract_primary_var(&binary.right);

        match (left, right) {
            (Some(l), Some(r)) => Rvalue::BinaryOp {
                op: convert_bin_op(&binary.op),
                left: l,
                right: r,
            },
            (Some(l), None) => Rvalue::Use(l),
            (None, Some(r)) => Rvalue::Use(r),
            (None, None) => Rvalue::Constant,
        }
    }

    fn convert_unary_to_rvalue(&mut self, unary: &syn::ExprUnary) -> Rvalue {
        self.extract_primary_var(&unary.expr)
            .map(|operand| Rvalue::UnaryOp {
                op: convert_un_op(&unary.op),
                operand,
            })
            .unwrap_or(Rvalue::Constant)
    }

    /// Convert a field access expression to an Rvalue.
    fn convert_field_to_rvalue(&mut self, field: &syn::ExprField) -> Rvalue {
        if let Some(base) = self.extract_primary_var(&field.base) {
            let field_name = match &field.member {
                syn::Member::Named(ident) => ident.to_string(),
                syn::Member::Unnamed(index) => index.index.to_string(),
            };
            Rvalue::FieldAccess {
                base,
                field: field_name,
            }
        } else {
            Rvalue::Constant
        }
    }

    fn convert_reference_to_rvalue(&mut self, reference: &syn::ExprReference) -> Rvalue {
        self.extract_primary_var(&reference.expr)
            .map(|var| Rvalue::Ref {
                var,
                mutable: reference.mutability.is_some(),
            })
            .unwrap_or(Rvalue::Constant)
    }

    fn convert_call_to_rvalue(&mut self, call: &syn::ExprCall) -> Rvalue {
        Rvalue::Call {
            func: extract_func_name(&call.func),
            args: self.extract_call_args(call.args.iter()),
        }
    }

    /// Convert a method call expression to an Rvalue.
    fn convert_method_call_to_rvalue(&mut self, method: &syn::ExprMethodCall) -> Rvalue {
        let func_name = method.method.to_string();
        let mut args: Vec<VarId> = vec![];

        if let Some(recv) = self.extract_primary_var(&method.receiver) {
            args.push(recv);
        }

        args.extend(
            method
                .args
                .iter()
                .filter_map(|a| self.extract_primary_var(a)),
        );

        Rvalue::Call {
            func: func_name,
            args,
        }
    }

    fn convert_block_to_rvalue(&mut self, block: &syn::ExprBlock) -> Rvalue {
        block
            .block
            .stmts
            .last()
            .and_then(final_block_expr)
            .map(|expr| self.expr_to_rvalue(expr))
            .unwrap_or(Rvalue::Constant)
    }

    fn convert_index_to_rvalue(&mut self, index: &syn::ExprIndex) -> Rvalue {
        self.extract_primary_var(&index.expr)
            .map(|base| Rvalue::FieldAccess {
                base,
                field: "[index]".to_string(),
            })
            .unwrap_or(Rvalue::Constant)
    }

    fn extract_call_args<'a>(&mut self, args: impl Iterator<Item = &'a Expr>) -> Vec<VarId> {
        args.filter_map(|arg| self.extract_primary_var(arg))
            .collect()
    }
}

fn final_block_expr(stmt: &Stmt) -> Option<&Expr> {
    match stmt {
        Stmt::Expr(expr, _) => Some(expr),
        _ => None,
    }
}

/// Extract function name from a call expression.
pub(super) fn extract_func_name(func: &Expr) -> String {
    match func {
        Expr::Path(path) => path
            .path
            .segments
            .iter()
            .map(|s| s.ident.to_string())
            .collect::<Vec<_>>()
            .join("::"),
        _ => "unknown".to_string(),
    }
}

/// Convert syn binary operator to CFG BinOp.
pub(super) fn convert_bin_op(op: &syn::BinOp) -> BinOp {
    match op {
        syn::BinOp::Add(_) | syn::BinOp::AddAssign(_) => BinOp::Add,
        syn::BinOp::Sub(_) | syn::BinOp::SubAssign(_) => BinOp::Sub,
        syn::BinOp::Mul(_) | syn::BinOp::MulAssign(_) => BinOp::Mul,
        syn::BinOp::Div(_) | syn::BinOp::DivAssign(_) => BinOp::Div,
        syn::BinOp::Eq(_) => BinOp::Eq,
        syn::BinOp::Ne(_) => BinOp::Ne,
        syn::BinOp::Lt(_) => BinOp::Lt,
        syn::BinOp::Gt(_) => BinOp::Gt,
        syn::BinOp::Le(_) => BinOp::Le,
        syn::BinOp::Ge(_) => BinOp::Ge,
        syn::BinOp::And(_) => BinOp::And,
        syn::BinOp::Or(_) => BinOp::Or,
        _ => BinOp::Add, // Fallback for bitwise ops, rem, shl, shr
    }
}

/// Convert syn unary operator to CFG UnOp.
pub(super) fn convert_un_op(op: &syn::UnOp) -> UnOp {
    match op {
        syn::UnOp::Neg(_) => UnOp::Neg,
        syn::UnOp::Not(_) => UnOp::Not,
        syn::UnOp::Deref(_) => UnOp::Deref,
        _ => UnOp::Not, // Fallback for unknown ops
    }
}
