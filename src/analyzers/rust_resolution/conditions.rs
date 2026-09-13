//! Successful and unsuccessful condition paths retain separate lexical facts.
use super::{bindings::Bindings, body::Body};
use syn::{Expr, visit::Visit};

pub(super) struct ConditionStates {
    pub success: Bindings,
    pub failure: Bindings,
}

impl Body<'_> {
    pub(super) fn condition(&mut self, expr: &Expr) -> ConditionStates {
        match expr {
            Expr::Let(binding) => self.let_condition(binding),
            Expr::Binary(binary) if matches!(binary.op, syn::BinOp::And(_) | syn::BinOp::Or(_)) => {
                self.binary_condition(binary)
            }
            Expr::Paren(paren) => self.condition(&paren.expr),
            Expr::Group(group) => self.condition(&group.expr),
            Expr::Unary(unary) if matches!(unary.op, syn::UnOp::Not(_)) => {
                let states = self.condition(&unary.expr);
                ConditionStates {
                    success: states.failure,
                    failure: states.success,
                }
            }
            _ => {
                self.visit_expr(expr);
                ConditionStates {
                    success: self.bindings.clone(),
                    failure: self.bindings.clone(),
                }
            }
        }
    }

    fn let_condition(&mut self, binding: &syn::ExprLet) -> ConditionStates {
        self.visit_expr(&binding.expr);
        let fact = self.infer(&binding.expr);
        let failure = self.bindings.clone();
        self.bind_pattern(&binding.pat, fact, false);
        ConditionStates {
            success: self.bindings.clone(),
            failure,
        }
    }

    fn binary_condition(&mut self, binary: &syn::ExprBinary) -> ConditionStates {
        let left = self.condition(&binary.left);
        let is_and = matches!(binary.op, syn::BinOp::And(_));
        self.bindings = if is_and {
            left.success.clone()
        } else {
            left.failure.clone()
        };
        let right = self.condition(&binary.right);
        if is_and {
            ConditionStates {
                success: right.success,
                failure: left.failure.join(&[left.failure.clone(), right.failure]),
            }
        } else {
            ConditionStates {
                success: left.success.join(&[left.success.clone(), right.success]),
                failure: right.failure,
            }
        }
    }
}

impl Body<'_> {
    pub(super) fn visit_short_circuit(&mut self, expr: &Expr) {
        let before = self.bindings.clone();
        let states = self.condition(expr);
        self.bindings = before.join(&[states.success, states.failure]);
    }

    pub(super) fn visit_if(&mut self, branch: &syn::ExprIf) {
        let before = self.bindings.clone();
        self.bindings.push();
        let states = self.condition(&branch.cond);
        self.bindings = states.success;
        self.visit_block(&branch.then_branch);
        self.bindings.pop();
        let left = self.bindings.clone();
        self.bindings = states.failure;
        self.bindings.pop();
        if let Some((_, expr)) = &branch.else_branch {
            self.visit_expr(expr);
        }
        self.bindings = before.join(&[left, self.bindings.clone()]);
    }

    pub(super) fn visit_match(&mut self, branch: &syn::ExprMatch) {
        self.visit_expr(&branch.expr);
        let fact = self.infer(&branch.expr);
        let before = self.bindings.clone();
        let mut outcomes = Vec::new();
        for arm in &branch.arms {
            let skipped = self.bindings.clone();
            self.bindings.push();
            self.bind_pattern(&arm.pat, fact.clone(), false);
            if let Some((_, guard)) = &arm.guard {
                self.visit_expr(guard);
            }
            let mut guarded = self.bindings.clone();
            guarded.pop();
            self.visit_expr(&arm.body);
            self.bindings.pop();
            outcomes.push(self.bindings.clone());
            self.bindings = skipped.join(&[skipped.clone(), guarded]);
        }
        self.bindings = before.join(&outcomes);
    }

    pub(super) fn visit_while(&mut self, expr: &syn::ExprWhile) {
        let writes = super::flow::loop_writes(
            Some(&expr.cond),
            &expr.body,
            self.index,
            &self.callable.context,
        );
        self.bindings.invalidate_writes(&writes);
        let before = self.bindings.clone();
        self.bindings.push();
        let states = self.condition(&expr.cond);
        self.bindings = states.success;
        self.visit_block(&expr.body);
        self.bindings.pop();
        let body = self.bindings.clone();
        self.bindings = states.failure;
        self.bindings.pop();
        self.bindings = before.join(&[body, self.bindings.clone()]);
        self.bindings.invalidate_writes(&writes);
    }
}
