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
        match expr {
            // Simple variable use
            Expr::Path(path) => {
                if let Some(var) = self.extract_primary_var(&Expr::Path(path.clone())) {
                    Rvalue::Use(var)
                } else {
                    Rvalue::Constant
                }
            }

            // Binary operation
            Expr::Binary(binary) => self.convert_binary_to_rvalue(binary),

            // Unary operation
            Expr::Unary(unary) => {
                if let Some(operand) = self.extract_primary_var(&unary.expr) {
                    Rvalue::UnaryOp {
                        op: convert_un_op(&unary.op),
                        operand,
                    }
                } else {
                    Rvalue::Constant
                }
            }

            // Field access
            Expr::Field(field) => self.convert_field_to_rvalue(field),

            // Reference
            Expr::Reference(reference) => {
                if let Some(var) = self.extract_primary_var(&reference.expr) {
                    Rvalue::Ref {
                        var,
                        mutable: reference.mutability.is_some(),
                    }
                } else {
                    Rvalue::Constant
                }
            }

            // Function call
            Expr::Call(call) => {
                let func_name = extract_func_name(&call.func);
                let args = call
                    .args
                    .iter()
                    .filter_map(|arg| self.extract_primary_var(arg))
                    .collect();
                Rvalue::Call {
                    func: func_name,
                    args,
                }
            }

            // Method call
            Expr::MethodCall(method) => self.convert_method_call_to_rvalue(method),

            // Paren: (expr) - unwrap
            Expr::Paren(paren) => self.expr_to_rvalue(&paren.expr),

            // Cast: x as T - preserve the variable
            Expr::Cast(cast) => self.expr_to_rvalue(&cast.expr),

            // Block: { expr } - use final expression
            Expr::Block(block) => {
                if let Some(Stmt::Expr(expr, _)) = block.block.stmts.last() {
                    self.expr_to_rvalue(expr)
                } else {
                    Rvalue::Constant
                }
            }

            // Index: arr[i]
            Expr::Index(index) => {
                if let Some(base) = self.extract_primary_var(&index.expr) {
                    Rvalue::FieldAccess {
                        base,
                        field: "[index]".to_string(),
                    }
                } else {
                    Rvalue::Constant
                }
            }

            // Literals and other constant expressions
            Expr::Lit(_) => Rvalue::Constant,

            // Default fallback
            _ => Rvalue::Constant,
        }
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
