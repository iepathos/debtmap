//! Control flow processing for CFG construction.
//!
//! This module handles if/while/return/match expressions and other
//! control flow constructs during CFG construction.

use std::collections::HashSet;

use syn::visit::Visit;
use syn::{Expr, ExprAssign, ExprClosure, ExprIf, ExprMatch, ExprReturn, ExprWhile, Pat};

use super::super::types::{
    BlockId, CapturedVar, ExprKind, MatchArm, Rvalue, Statement, Terminator, VarId,
};
use super::closure::{CaptureInfo, ClosureCaptureVisitor};
use super::CfgBuilder;

impl CfgBuilder {
    /// Process an if expression, creating branch blocks.
    pub(super) fn process_if(&mut self, expr_if: &ExprIf) {
        let condition = self
            .extract_primary_var(&expr_if.cond)
            .unwrap_or_else(|| self.get_or_create_var("_cond"));

        let then_block = BlockId(self.block_counter + 1);
        let else_block = BlockId(self.block_counter + 2);

        self.finalize_current_block(Terminator::Branch {
            condition,
            then_block,
            else_block,
        });
    }

    /// Process a while expression, creating loop blocks.
    pub(super) fn process_while(&mut self, _expr_while: &ExprWhile) {
        let loop_head = BlockId(self.block_counter + 1);
        self.finalize_current_block(Terminator::Goto { target: loop_head });
    }

    /// Process a return expression.
    pub(super) fn process_return(&mut self, expr_return: &ExprReturn) {
        let value = expr_return
            .expr
            .as_ref()
            .and_then(|e| self.extract_primary_var(e));

        self.finalize_current_block(Terminator::Return { value });
    }

    /// Process an assignment expression.
    pub(super) fn process_assign(&mut self, assign: &ExprAssign) {
        let target = self
            .extract_primary_var(&assign.left)
            .unwrap_or_else(|| self.get_or_create_var("_unknown"));

        let source = self.expr_to_rvalue(&assign.right);

        self.current_block.push(Statement::Assign {
            target,
            source,
            line: None,
        });
    }

    /// Process a match expression, creating proper CFG structure.
    ///
    /// This creates:
    /// 1. A block ending with Match terminator that branches to arm blocks
    /// 2. One block per arm for pattern bindings and arm body
    /// 3. A join block where all arms converge
    pub(super) fn process_match(&mut self, expr_match: &ExprMatch) {
        let scrutinee_var = self.process_scrutinee(&expr_match.expr);

        let arm_count = expr_match.arms.len();
        let arm_start_id = self.block_counter + 1;
        let join_block_id = BlockId(arm_start_id + arm_count);

        let mut cfg_arms = Vec::with_capacity(arm_count);
        for i in 0..arm_count {
            cfg_arms.push(MatchArm {
                block: BlockId(arm_start_id + i),
                guard: None,
                bindings: Vec::new(),
            });
        }

        self.finalize_current_block(Terminator::Match {
            scrutinee: scrutinee_var,
            arms: cfg_arms.clone(),
            join_block: join_block_id,
        });

        for (i, arm) in expr_match.arms.iter().enumerate() {
            self.process_match_arm(arm, scrutinee_var, join_block_id, i);
        }

        // Create the join block
        self.current_block = Vec::new();
    }

    /// Process the scrutinee expression and return its VarId.
    fn process_scrutinee(&mut self, expr: &Expr) -> VarId {
        if let Some(var) = self.extract_primary_var(expr) {
            return var;
        }

        let temp_var = self.get_or_create_var("_scrutinee");
        let rvalue = self.expr_to_rvalue(expr);

        self.current_block.push(Statement::Assign {
            target: temp_var,
            source: rvalue,
            line: None,
        });

        temp_var
    }

    /// Process a single match arm, creating its basic block.
    fn process_match_arm(
        &mut self,
        arm: &syn::Arm,
        scrutinee: VarId,
        join_block: BlockId,
        _arm_index: usize,
    ) {
        self.current_block = Vec::new();

        // Bind pattern variables from scrutinee
        let bindings = self.bind_pattern_vars(&arm.pat, scrutinee);

        // Process guard if present
        let guard_var = arm
            .guard
            .as_ref()
            .map(|(_, guard_expr)| self.process_guard(guard_expr));

        // Process arm body
        self.process_expr(&arm.body);

        // Record bindings and guard (already tracked through Declare statements)
        let _ = (bindings, guard_var);

        self.finalize_current_block(Terminator::Goto { target: join_block });
    }

    /// Bind pattern variables and return their VarIds.
    fn bind_pattern_vars(&mut self, pat: &Pat, scrutinee: VarId) -> Vec<VarId> {
        let binding_names = self.extract_vars_from_pattern(pat);

        for (i, var) in binding_names.iter().enumerate() {
            let init = if i == 0 {
                Rvalue::Use(scrutinee)
            } else {
                Rvalue::FieldAccess {
                    base: scrutinee,
                    field: i.to_string(),
                }
            };

            self.current_block.push(Statement::Declare {
                var: *var,
                init: Some(init),
                line: None,
            });
        }

        binding_names
    }

    /// Process a guard expression and return condition VarId.
    fn process_guard(&mut self, guard_expr: &Expr) -> VarId {
        if let Some(var) = self.extract_primary_var(guard_expr) {
            return var;
        }

        let guard_var = self.get_or_create_var("_guard");
        let rvalue = self.expr_to_rvalue(guard_expr);

        self.current_block.push(Statement::Assign {
            target: guard_var,
            source: rvalue,
            line: None,
        });

        guard_var
    }

    /// Process a closure expression, extracting captures and body information.
    pub(super) fn process_closure(&mut self, closure: &ExprClosure) {
        // Record outer scope variables before entering closure
        let outer_scope_vars = self.current_scope_vars();

        // Create closure parameter scope
        let closure_params: HashSet<String> = closure
            .inputs
            .iter()
            .filter_map(|input| {
                if let Pat::Ident(pat_ident) = input {
                    Some(pat_ident.ident.to_string())
                } else {
                    None
                }
            })
            .collect();

        // Visit closure body to find captures
        let is_move = closure.capture.is_some();
        let mut capture_visitor =
            ClosureCaptureVisitor::new(&outer_scope_vars, &closure_params, is_move);
        capture_visitor.visit_expr(&closure.body);

        // Finalize and record captured variables
        let captures: Vec<CaptureInfo> = capture_visitor.finalize_captures();

        let capture_var_ids: Vec<VarId> = captures
            .iter()
            .map(|c| {
                let var_id = self.get_or_create_var(&c.var_name);
                self.captured_vars.push(CapturedVar {
                    var_id,
                    capture_mode: c.mode,
                    is_mutated: c.is_mutated,
                });
                var_id
            })
            .collect();

        // Emit closure expression statement
        self.current_block.push(Statement::Expr {
            expr: ExprKind::Closure {
                captures: capture_var_ids,
                is_move,
            },
            line: None,
        });
    }
}
