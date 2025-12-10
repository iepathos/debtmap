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
use syn::{Block, Expr, Stmt};

/// Calculate cyclomatic complexity using iterative AST traversal.
///
/// Uses an explicit stack instead of recursive visitor to avoid stack overflow
/// on deeply nested AST structures.
pub fn calculate_cyclomatic(block: &Block) -> u32 {
    let mut complexity = 1u32;

    // Use explicit stack for iterative traversal
    // Stack contains (expression, in_condition flag)
    let mut expr_stack: Vec<(&Expr, bool)> = Vec::new();
    let mut stmt_stack: Vec<&Stmt> = Vec::new();

    // Initialize with block statements
    for stmt in block.stmts.iter().rev() {
        stmt_stack.push(stmt);
    }

    while !stmt_stack.is_empty() || !expr_stack.is_empty() {
        // Process statements first
        if let Some(stmt) = stmt_stack.pop() {
            match stmt {
                Stmt::Local(local) => {
                    if let Some(init) = &local.init {
                        expr_stack.push((&init.expr, false));
                    }
                }
                Stmt::Expr(expr, _) => {
                    expr_stack.push((expr, false));
                }
                Stmt::Item(_) => {
                    // Items don't contribute to complexity
                }
                Stmt::Macro(_) => {
                    // Macros can't be analyzed without expansion
                }
            }
            continue;
        }

        // Process expressions
        if let Some((expr, in_condition)) = expr_stack.pop() {
            // Calculate complexity contribution for this expression
            complexity += calculate_expr_complexity(expr, in_condition);

            // Add child expressions to stack (in reverse order for correct traversal)
            match expr {
                Expr::If(expr_if) => {
                    // Add else branch
                    if let Some((_, else_expr)) = &expr_if.else_branch {
                        expr_stack.push((else_expr, false));
                    }
                    // Add then block statements
                    for stmt in expr_if.then_branch.stmts.iter().rev() {
                        stmt_stack.push(stmt);
                    }
                    // Add condition (mark as in_condition)
                    expr_stack.push((&expr_if.cond, true));
                }
                Expr::While(expr_while) => {
                    // Add body statements
                    for stmt in expr_while.body.stmts.iter().rev() {
                        stmt_stack.push(stmt);
                    }
                    // Add condition
                    expr_stack.push((&expr_while.cond, true));
                }
                Expr::ForLoop(expr_for) => {
                    // Add body statements
                    for stmt in expr_for.body.stmts.iter().rev() {
                        stmt_stack.push(stmt);
                    }
                    // Add iterator expression
                    expr_stack.push((&expr_for.expr, false));
                }
                Expr::Loop(expr_loop) => {
                    // Add body statements
                    for stmt in expr_loop.body.stmts.iter().rev() {
                        stmt_stack.push(stmt);
                    }
                }
                Expr::Match(expr_match) => {
                    // Add arm bodies
                    for arm in expr_match.arms.iter().rev() {
                        expr_stack.push((&arm.body, false));
                        if let Some((_, guard)) = &arm.guard {
                            expr_stack.push((guard, false));
                        }
                    }
                    // Add match expression
                    expr_stack.push((&expr_match.expr, false));
                }
                Expr::Block(expr_block) => {
                    for stmt in expr_block.block.stmts.iter().rev() {
                        stmt_stack.push(stmt);
                    }
                }
                Expr::Closure(closure) => {
                    expr_stack.push((&closure.body, false));
                }
                Expr::Async(async_block) => {
                    for stmt in async_block.block.stmts.iter().rev() {
                        stmt_stack.push(stmt);
                    }
                }
                Expr::Try(expr_try) => {
                    expr_stack.push((&expr_try.expr, false));
                }
                Expr::Binary(binary) => {
                    expr_stack.push((&binary.right, in_condition));
                    expr_stack.push((&binary.left, in_condition));
                }
                Expr::Unary(unary) => {
                    expr_stack.push((&unary.expr, in_condition));
                }
                Expr::Call(call) => {
                    for arg in call.args.iter().rev() {
                        expr_stack.push((arg, false));
                    }
                    expr_stack.push((&call.func, false));
                }
                Expr::MethodCall(method_call) => {
                    for arg in method_call.args.iter().rev() {
                        expr_stack.push((arg, false));
                    }
                    expr_stack.push((&method_call.receiver, false));
                }
                Expr::Field(field) => {
                    expr_stack.push((&field.base, false));
                }
                Expr::Index(index) => {
                    expr_stack.push((&index.index, false));
                    expr_stack.push((&index.expr, false));
                }
                Expr::Paren(paren) => {
                    expr_stack.push((&paren.expr, in_condition));
                }
                Expr::Reference(reference) => {
                    expr_stack.push((&reference.expr, false));
                }
                Expr::Await(await_expr) => {
                    expr_stack.push((&await_expr.base, false));
                }
                Expr::Cast(cast) => {
                    expr_stack.push((&cast.expr, false));
                }
                Expr::Assign(assign) => {
                    expr_stack.push((&assign.right, false));
                    expr_stack.push((&assign.left, false));
                }
                Expr::Return(ret) => {
                    if let Some(expr) = &ret.expr {
                        expr_stack.push((expr, false));
                    }
                }
                Expr::Break(brk) => {
                    if let Some(expr) = &brk.expr {
                        expr_stack.push((expr, false));
                    }
                }
                Expr::Tuple(tuple) => {
                    for elem in tuple.elems.iter().rev() {
                        expr_stack.push((elem, false));
                    }
                }
                Expr::Array(array) => {
                    for elem in array.elems.iter().rev() {
                        expr_stack.push((elem, false));
                    }
                }
                Expr::Struct(struct_expr) => {
                    if let Some(rest) = &struct_expr.rest {
                        expr_stack.push((rest, false));
                    }
                    for field in struct_expr.fields.iter().rev() {
                        expr_stack.push((&field.expr, false));
                    }
                }
                Expr::Repeat(repeat) => {
                    expr_stack.push((&repeat.len, false));
                    expr_stack.push((&repeat.expr, false));
                }
                Expr::Range(range) => {
                    if let Some(to) = &range.end {
                        expr_stack.push((to, false));
                    }
                    if let Some(from) = &range.start {
                        expr_stack.push((from, false));
                    }
                }
                Expr::Let(let_expr) => {
                    expr_stack.push((&let_expr.expr, false));
                }
                Expr::Yield(yield_expr) => {
                    if let Some(expr) = &yield_expr.expr {
                        expr_stack.push((expr, false));
                    }
                }
                // Leaf expressions - no children to process
                Expr::Lit(_)
                | Expr::Path(_)
                | Expr::Continue(_)
                | Expr::Infer(_)
                | Expr::Verbatim(_)
                | Expr::Const(_) => {}
                // Catch-all for any other expressions
                _ => {}
            }
        }
    }

    complexity
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

/// Calculate complexity contribution for a single expression.
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

fn is_logical_operator(op: &syn::BinOp) -> bool {
    matches!(op, syn::BinOp::And(_) | syn::BinOp::Or(_))
}

pub fn calculate_cyclomatic_for_function(complexity: u32, params: usize) -> u32 {
    complexity + params.saturating_sub(1) as u32
}

pub fn combine_cyclomatic(branches: Vec<u32>) -> u32 {
    branches.iter().sum::<u32>() + 1
}
