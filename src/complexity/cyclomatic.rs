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
///
/// When a pattern-like match expression is detected, its contribution to
/// complexity is adjusted using logarithmic scaling. Importantly, this
/// adjustment only affects the match's contribution - other control flow
/// in the block is preserved.
pub fn calculate_cyclomatic_adjusted(block: &Block) -> u32 {
    let base = calculate_cyclomatic(block);

    // Check for match expressions that should use logarithmic scaling
    for stmt in &block.stmts {
        if let Stmt::Expr(expr, _) = stmt {
            if let Some(info) = detect_match_expression(expr) {
                // Calculate the match's original contribution: (arms - 1)
                let original_match_contribution = info.condition_count.saturating_sub(1) as u32;

                // Calculate the adjusted contribution using logarithmic scaling
                let adjusted_match = (info.condition_count as f32).log2().ceil() as u32;
                let default_penalty = if !info.has_default { 1 } else { 0 };

                // Replace just the match's contribution, preserving other complexity
                return base - original_match_contribution + adjusted_match + default_penalty;
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
///
/// McCabe cyclomatic complexity counts decision points (predicates), not branches.
/// An if-else has ONE decision point regardless of whether there's an else branch.
/// The else branch is just the alternative path from the single decision.
///
/// Logical operators (&&, ||) always add complexity because they represent
/// additional predicates that can independently affect control flow.
/// `if a && b && c` has 3 predicates and is more complex than `if a`.
fn calculate_expr_complexity(expr: &Expr, _in_condition: bool) -> u32 {
    match expr {
        // If adds 1 decision point regardless of else branch presence.
        // The else is not a separate decision - it's the alternative path.
        Expr::If(_) => 1,
        Expr::While(_) | Expr::ForLoop(_) | Expr::Loop(_) => 1,
        Expr::Try(_) => 1,
        Expr::Match(expr_match) => expr_match.arms.len().saturating_sub(1) as u32,
        // Logical operators always add complexity - they represent additional predicates
        // that can independently affect control flow, regardless of context.
        Expr::Binary(binary) if is_logical_operator(&binary.op) => 1,
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

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    /// Bug fix test: if-else should count as 1 decision point, not 2.
    /// McCabe cyclomatic complexity counts decision points (predicates),
    /// not branches. An if-else has ONE decision point regardless of
    /// whether there's an else branch.
    #[test]
    fn test_if_else_counts_as_one_decision_point() {
        // Single if without else: complexity = 1 (base) + 1 (if) = 2
        let block_if_only: Block = parse_quote! {{
            if x > 0 {
                do_something();
            }
        }};
        assert_eq!(
            calculate_cyclomatic(&block_if_only),
            2,
            "if without else should add 1 to base complexity"
        );

        // Single if WITH else: complexity should ALSO be 1 (base) + 1 (if) = 2
        // The else is not a separate decision point - it's the alternative path
        let block_if_else: Block = parse_quote! {{
            if x > 0 {
                do_something();
            } else {
                do_other();
            }
        }};
        assert_eq!(
            calculate_cyclomatic(&block_if_else),
            2,
            "if-else should add 1 to base complexity, not 2 (else is not a decision point)"
        );
    }

    #[test]
    fn test_multiple_if_else_chains() {
        // 3 sequential if-else statements: complexity = 1 (base) + 3 (ifs) = 4
        let block: Block = parse_quote! {{
            if a { x(); } else { y(); }
            if b { x(); } else { y(); }
            if c { x(); } else { y(); }
        }};
        assert_eq!(
            calculate_cyclomatic(&block),
            4,
            "3 if-else statements should add 3 to base complexity"
        );
    }

    #[test]
    fn test_nested_if_else() {
        // Nested if-else: complexity = 1 (base) + 2 (two if decisions) = 3
        let block: Block = parse_quote! {{
            if a {
                if b {
                    x();
                } else {
                    y();
                }
            } else {
                z();
            }
        }};
        assert_eq!(
            calculate_cyclomatic(&block),
            3,
            "nested if-else should count 2 decisions (outer if + inner if)"
        );
    }

    #[test]
    fn test_match_complexity() {
        // Match with 3 arms: complexity = 1 (base) + 2 (arms - 1) = 3
        let block: Block = parse_quote! {{
            match x {
                A => 1,
                B => 2,
                _ => 3,
            }
        }};
        assert_eq!(
            calculate_cyclomatic(&block),
            3,
            "match with 3 arms should add 2 (arms - 1) to base complexity"
        );
    }

    #[test]
    fn test_loop_complexity() {
        let block: Block = parse_quote! {{
            while condition {
                do_work();
            }
            for i in items {
                process(i);
            }
            loop {
                if done { break; }
            }
        }};
        // 1 (base) + 1 (while) + 1 (for) + 1 (loop) + 1 (if inside loop) = 5
        assert_eq!(calculate_cyclomatic(&block), 5);
    }

    /// Bug fix test: Logical operators inside conditions SHOULD add complexity.
    /// `if a && b && c` has 3 predicates and should have higher complexity
    /// than `if a`.
    #[test]
    fn test_logical_operators_in_conditions_add_complexity() {
        // Single condition: complexity = 1 (base) + 1 (if) = 2
        let single_condition: Block = parse_quote! {{
            if a {
                do_something();
            }
        }};
        assert_eq!(
            calculate_cyclomatic(&single_condition),
            2,
            "Single condition should have complexity 2"
        );

        // Multiple conditions with &&: complexity = 1 (base) + 1 (if) + 2 (&& operators) = 4
        let three_conditions: Block = parse_quote! {{
            if a && b && c {
                do_something();
            }
        }};
        assert!(
            calculate_cyclomatic(&three_conditions) > 2,
            "if a && b && c should have higher complexity than if a (got {})",
            calculate_cyclomatic(&three_conditions)
        );

        // Mixed && and ||: should also add complexity
        let mixed_operators: Block = parse_quote! {{
            if a && b || c {
                do_something();
            }
        }};
        assert!(
            calculate_cyclomatic(&mixed_operators) > 2,
            "if a && b || c should have higher complexity than if a (got {})",
            calculate_cyclomatic(&mixed_operators)
        );
    }

    /// Bug fix test: calculate_cyclomatic_adjusted should NOT discard base complexity
    /// when a pattern match is detected. The adjustment should only affect the
    /// match expression's contribution, not the entire function's complexity.
    #[test]
    fn test_adjusted_preserves_other_complexity() {
        // Function with control flow BEFORE a match expression
        let block: Block = parse_quote! {{
            if condition {
                do_something();
            }
            for i in items {
                process(i);
            }
            match x {
                A => 1,
                B => 2,
                C => 3,
                D => 4,
                E => 5,
                F => 6,
                G => 7,
                _ => 8,
            }
        }};

        let base = calculate_cyclomatic(&block);
        let adjusted = calculate_cyclomatic_adjusted(&block);

        // Base should be: 1 (base) + 1 (if) + 1 (for) + 7 (match: 8 arms - 1) = 10
        assert_eq!(base, 10, "Base complexity should include all control flow");

        // Adjusted should NOT be just log2(8)=3 - it should preserve the if and for loop
        // The if and for contribute 2, and the match should be adjusted to ~3
        // So adjusted should be around 1 (base) + 1 (if) + 1 (for) + 3 (adjusted match) = 6
        // At minimum, it should be > 3 (the match-only adjustment)
        assert!(
            adjusted > 3,
            "Adjusted complexity ({}) should preserve non-match control flow, not just return match adjustment",
            adjusted
        );
    }
}
