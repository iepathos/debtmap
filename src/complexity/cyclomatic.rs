//! Cyclomatic complexity calculation for Rust code blocks.
//!
//! **Note**: For new code, prefer using [`super::pure::calculate_cyclomatic_pure`]
//! which operates directly on `syn::File` AST and is faster to test.
//! This module provides block-level complexity calculation which is still
//! useful for analyzing individual function bodies.
//!
//! The pure functions in `pure.rs` are:
//! - Deterministic (same input = same output)
//! - Fast to test (no I/O overhead)
//! - Easier to compose
//!
//! See [`super::pure`] for file-level pure functions.

use super::match_patterns::detect_match_expression;
use syn::{visit::Visit, Block, Expr, Stmt};

pub fn calculate_cyclomatic(block: &Block) -> u32 {
    let mut visitor = CyclomaticVisitor {
        complexity: 1,
        in_condition: false,
    };
    visitor.visit_block(block);
    visitor.complexity
}

/// Calculate cyclomatic complexity with pattern adjustments
pub fn calculate_cyclomatic_adjusted(block: &Block) -> u32 {
    let base = calculate_cyclomatic(block);

    // Check for match expressions that should use logarithmic scaling
    for stmt in &block.stmts {
        if let Stmt::Expr(expr, _) = stmt {
            if let Some(info) = detect_match_expression(expr) {
                // Apply logarithmic scaling for pattern-based match expressions
                let adjusted = (info.condition_count as f32).log2().ceil() as u32;
                let default_penalty = if !info.has_default { 1 } else { 0 };
                return adjusted + default_penalty;
            }
        }
    }

    // Check for if-else pattern matching using existing pattern recognizers
    use super::pattern_adjustments::{PatternMatchRecognizer, PatternRecognizer};
    let recognizer = PatternMatchRecognizer::new();
    if let Some(info) = recognizer.detect(block) {
        return recognizer.adjust_complexity(&info, base);
    }

    base
}

struct CyclomaticVisitor {
    complexity: u32,
    in_condition: bool,
}

fn calculate_expr_complexity(expr: &Expr, in_condition: bool) -> u32 {
    match expr {
        Expr::If(expr_if) => {
            let mut count = 1;
            if expr_if.else_branch.is_some() {
                count += 1;
            }
            count
        }
        Expr::While(_) | Expr::ForLoop(_) | Expr::Loop(_) => 1,
        Expr::Try(_) => 1,
        Expr::Match(expr_match) => expr_match.arms.len().saturating_sub(1) as u32,
        Expr::Binary(binary) if is_logical_operator(&binary.op) && !in_condition => 1,
        _ => 0,
    }
}

impl<'ast> Visit<'ast> for CyclomaticVisitor {
    fn visit_expr(&mut self, expr: &'ast Expr) {
        let complexity_delta = calculate_expr_complexity(expr, self.in_condition);
        self.complexity += complexity_delta;

        let was_in_condition = self.in_condition;
        if matches!(expr, Expr::If(_) | Expr::While(_)) {
            self.in_condition = true;
        }

        syn::visit::visit_expr(self, expr);

        self.in_condition = was_in_condition;
    }

    fn visit_stmt(&mut self, stmt: &'ast Stmt) {
        syn::visit::visit_stmt(self, stmt);
    }
}

fn is_logical_operator(op: &syn::BinOp) -> bool {
    matches!(op, syn::BinOp::And(_) | syn::BinOp::Or(_))
}

pub fn calculate_cyclomatic_for_function(complexity: u32, params: usize) -> u32 {
    complexity + params.saturating_sub(1) as u32
}

pub fn combine_cyclomatic(branches: Vec<u32>) -> u32 {
    branches.iter().sum::<u32>() + 1
}
