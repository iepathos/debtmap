//! Pure functions for complexity calculation.
//!
//! This module contains pure functions that operate directly on AST structures
//! without any I/O operations. Pure functions have several advantages:
//!
//! - **Deterministic**: Same input always produces the same output
//! - **No side effects**: No I/O, no mutation of external state
//! - **Fast to test**: Tests run in microseconds instead of milliseconds
//! - **Easy to reason about**: Clear input → output relationship
//! - **Highly composable**: Can be combined with other pure functions
//!
//! # Design Principles
//!
//! 1. **No `Result` types**: Pure functions cannot fail - they always produce output
//! 2. **No I/O**: Pure functions never read files, access network, etc.
//! 3. **Referentially transparent**: Can be replaced with their return value
//! 4. **Immutable**: Don't modify their arguments
//!
//! # Usage
//!
//! ```rust
//! use syn::parse_str;
//!
//! let code = "fn example(x: i32) { if x > 0 { println!(\"positive\"); } }";
//! let ast: syn::File = syn::parse_str(code).unwrap();
//!
//! // Pure functions operate on parsed AST
//! let cyclomatic = debtmap::complexity::pure::calculate_cyclomatic_pure(&ast);
//! let cognitive = debtmap::complexity::pure::calculate_cognitive_pure(&ast);
//! ```
//!
//! # Relationship with Effect System
//!
//! For operations that require I/O (reading files), use the effect wrappers
//! in `src/complexity/effects_wrappers.rs` which compose these pure functions
//! with file reading.

use syn::visit::Visit;
use syn::{Block, Expr, File, ImplItem, Item, Stmt};

/// A detected code pattern that may indicate complexity issues.
///
/// These patterns are commonly associated with technical debt or
/// maintainability concerns.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Pattern {
    /// A struct with many fields (potential god object)
    GodObject {
        /// Name of the struct
        name: String,
        /// Number of fields
        field_count: usize,
    },
    /// A function with many lines
    LongFunction {
        /// Name of the function
        name: String,
        /// Approximate line count
        lines: usize,
    },
    /// A function with many parameters
    ManyParameters {
        /// Name of the function
        name: String,
        /// Number of parameters
        param_count: usize,
    },
    /// Deeply nested control flow
    DeepNesting {
        /// Name of the containing function
        function_name: String,
        /// Maximum nesting depth found
        depth: u32,
    },
    /// Complex match expression
    ComplexMatch {
        /// Number of match arms
        arm_count: usize,
    },
}

// ============================================================================
// Cyclomatic Complexity - Pure Functions
// ============================================================================

/// Calculate cyclomatic complexity from a parsed AST.
///
/// This is a pure function - deterministic, no I/O, no `Result`.
///
/// Cyclomatic complexity measures the number of linearly independent paths
/// through a program. Higher values indicate more complex code that may
/// need more tests to achieve coverage.
///
/// # Algorithm
///
/// Base complexity of 1, plus:
/// - Each `if` statement adds 1
/// - Each `else` branch adds 1
/// - Each loop (`while`, `for`, `loop`) adds 1
/// - Each `match` arm adds 1 (minus 1 for the first arm)
/// - Each `?` operator adds 1
/// - Each logical operator (`&&`, `||`) outside conditions adds 1
///
/// # Example
///
/// ```rust
/// let ast: syn::File = syn::parse_str("fn foo() {}").unwrap();
/// let complexity = debtmap::complexity::pure::calculate_cyclomatic_pure(&ast);
/// assert_eq!(complexity, 1); // Simple function has complexity 1
/// ```
pub fn calculate_cyclomatic_pure(file: &File) -> u32 {
    file.items.iter().map(count_item_branches).sum()
}

/// Count branches in a single AST item.
pub fn count_item_branches(item: &Item) -> u32 {
    match item {
        Item::Fn(func) => count_function_branches(&func.block),
        Item::Impl(impl_block) => impl_block
            .items
            .iter()
            .filter_map(|item| {
                if let ImplItem::Fn(method) = item {
                    Some(count_function_branches(&method.block))
                } else {
                    None
                }
            })
            .sum(),
        _ => 0,
    }
}

/// Count branches in a function body.
///
/// Returns base complexity of 1 plus additional complexity from control flow.
pub fn count_function_branches(block: &Block) -> u32 {
    1 + block.stmts.iter().map(count_stmt_branches).sum::<u32>()
}

fn count_stmt_branches(stmt: &Stmt) -> u32 {
    match stmt {
        Stmt::Expr(expr, _) => count_expr_branches(expr),
        Stmt::Local(local) => local
            .init
            .as_ref()
            .map(|init| count_expr_branches(&init.expr))
            .unwrap_or(0),
        _ => 0,
    }
}

/// Count branches in an expression.
///
/// This is a pure function that recursively counts control flow constructs.
pub fn count_expr_branches(expr: &Expr) -> u32 {
    match expr {
        Expr::If(expr_if) => {
            let mut count = 1; // if itself
                               // Count condition complexity
            count += count_expr_branches(&expr_if.cond);
            // Count then branch
            count += expr_if
                .then_branch
                .stmts
                .iter()
                .map(count_stmt_branches)
                .sum::<u32>();
            // Count else branch
            if let Some((_, else_expr)) = &expr_if.else_branch {
                count += 1; // else adds complexity
                count += count_expr_branches(else_expr);
            }
            count
        }
        Expr::While(expr_while) => {
            1 + count_expr_branches(&expr_while.cond)
                + expr_while
                    .body
                    .stmts
                    .iter()
                    .map(count_stmt_branches)
                    .sum::<u32>()
        }
        Expr::ForLoop(expr_for) => {
            1 + count_expr_branches(&expr_for.expr)
                + expr_for
                    .body
                    .stmts
                    .iter()
                    .map(count_stmt_branches)
                    .sum::<u32>()
        }
        Expr::Loop(expr_loop) => {
            1 + expr_loop
                .body
                .stmts
                .iter()
                .map(count_stmt_branches)
                .sum::<u32>()
        }
        Expr::Match(expr_match) => {
            let arms_count = expr_match.arms.len().saturating_sub(1) as u32;
            arms_count
                + count_expr_branches(&expr_match.expr)
                + expr_match
                    .arms
                    .iter()
                    .map(|arm| count_expr_branches(&arm.body))
                    .sum::<u32>()
        }
        Expr::Try(_) => 1,
        Expr::Binary(binary) if is_logical_operator(&binary.op) => {
            1 + count_expr_branches(&binary.left) + count_expr_branches(&binary.right)
        }
        Expr::Block(expr_block) => expr_block.block.stmts.iter().map(count_stmt_branches).sum(),
        Expr::Closure(closure) => count_expr_branches(&closure.body),
        Expr::Call(call) => {
            count_expr_branches(&call.func) + call.args.iter().map(count_expr_branches).sum::<u32>()
        }
        Expr::MethodCall(method_call) => {
            count_expr_branches(&method_call.receiver)
                + method_call
                    .args
                    .iter()
                    .map(count_expr_branches)
                    .sum::<u32>()
        }
        _ => 0,
    }
}

fn is_logical_operator(op: &syn::BinOp) -> bool {
    matches!(op, syn::BinOp::And(_) | syn::BinOp::Or(_))
}

// ============================================================================
// Cognitive Complexity - Pure Functions
// ============================================================================

/// Calculate cognitive complexity from a parsed AST.
///
/// This is a pure function - deterministic, no I/O, no `Result`.
///
/// Cognitive complexity measures how difficult code is to understand.
/// Unlike cyclomatic complexity, it accounts for nesting levels and
/// other factors that make code harder to comprehend.
///
/// # Algorithm
///
/// - Control flow statements add 1 + current nesting level
/// - Nesting increases for control structures (if, while, match, etc.)
/// - Logical operators add 1 (regardless of nesting)
/// - Closures and async blocks add complexity
/// - Unsafe blocks add 2 (higher mental burden)
///
/// # Example
///
/// ```rust
/// let ast: syn::File = syn::parse_str("fn foo() { if true { if false { } } }").unwrap();
/// let complexity = debtmap::complexity::pure::calculate_cognitive_pure(&ast);
/// // Nested if statements have higher cognitive complexity
/// ```
pub fn calculate_cognitive_pure(file: &File) -> u32 {
    file.items
        .iter()
        .map(|item| calculate_item_cognitive(item, 0))
        .sum()
}

fn calculate_item_cognitive(item: &Item, nesting: u32) -> u32 {
    match item {
        Item::Fn(func) => calculate_block_cognitive(&func.block, nesting),
        Item::Impl(impl_block) => impl_block
            .items
            .iter()
            .filter_map(|item| {
                if let ImplItem::Fn(method) = item {
                    Some(calculate_block_cognitive(&method.block, nesting))
                } else {
                    None
                }
            })
            .sum(),
        _ => 0,
    }
}

fn calculate_block_cognitive(block: &Block, nesting: u32) -> u32 {
    block
        .stmts
        .iter()
        .map(|stmt| calculate_stmt_cognitive(stmt, nesting))
        .sum()
}

fn calculate_stmt_cognitive(stmt: &Stmt, nesting: u32) -> u32 {
    match stmt {
        Stmt::Expr(expr, _) => calculate_expr_cognitive(expr, nesting),
        Stmt::Local(local) => local
            .init
            .as_ref()
            .map(|init| calculate_expr_cognitive(&init.expr, nesting))
            .unwrap_or(0),
        _ => 0,
    }
}

fn calculate_expr_cognitive(expr: &Expr, nesting: u32) -> u32 {
    match expr {
        Expr::If(if_expr) => {
            // If adds 1 + nesting
            let cost = 1 + nesting;
            // Recursively calculate nested complexity
            let cond_cost = calculate_expr_cognitive(&if_expr.cond, nesting);
            let then_cost = calculate_block_cognitive(&if_expr.then_branch, nesting + 1);
            let else_cost = if_expr
                .else_branch
                .as_ref()
                .map(|(_, else_expr)| {
                    // else if doesn't add nesting increment
                    if matches!(**else_expr, Expr::If(_)) {
                        calculate_expr_cognitive(else_expr, nesting)
                    } else {
                        1 + calculate_expr_cognitive(else_expr, nesting + 1)
                    }
                })
                .unwrap_or(0);
            cost + cond_cost + then_cost + else_cost
        }
        Expr::While(while_expr) => {
            1 + nesting
                + calculate_expr_cognitive(&while_expr.cond, nesting)
                + calculate_block_cognitive(&while_expr.body, nesting + 1)
        }
        Expr::ForLoop(for_expr) => {
            1 + nesting
                + calculate_expr_cognitive(&for_expr.expr, nesting)
                + calculate_block_cognitive(&for_expr.body, nesting + 1)
        }
        Expr::Loop(loop_expr) => {
            1 + nesting + calculate_block_cognitive(&loop_expr.body, nesting + 1)
        }
        Expr::Match(match_expr) => {
            let match_cost = 1 + nesting;
            let expr_cost = calculate_expr_cognitive(&match_expr.expr, nesting);
            let arms_cost: u32 = match_expr
                .arms
                .iter()
                .map(|arm| calculate_expr_cognitive(&arm.body, nesting + 1))
                .sum();
            match_cost + expr_cost + arms_cost
        }
        Expr::Try(_) => 1,
        Expr::Binary(binary) if is_logical_operator(&binary.op) => {
            1 + calculate_expr_cognitive(&binary.left, nesting)
                + calculate_expr_cognitive(&binary.right, nesting)
        }
        Expr::Closure(closure) => {
            let base = if closure.asyncness.is_some() { 2 } else { 1 };
            base + nesting.min(1) + calculate_expr_cognitive(&closure.body, nesting + 1)
        }
        Expr::Await(_) => 1,
        Expr::Unsafe(unsafe_expr) => 2 + calculate_block_cognitive(&unsafe_expr.block, nesting + 1),
        Expr::Block(block_expr) => calculate_block_cognitive(&block_expr.block, nesting),
        Expr::Call(call) => {
            calculate_expr_cognitive(&call.func, nesting)
                + call
                    .args
                    .iter()
                    .map(|arg| calculate_expr_cognitive(arg, nesting))
                    .sum::<u32>()
        }
        Expr::MethodCall(method_call) => {
            calculate_expr_cognitive(&method_call.receiver, nesting)
                + method_call
                    .args
                    .iter()
                    .map(|arg| calculate_expr_cognitive(arg, nesting))
                    .sum::<u32>()
        }
        _ => 0,
    }
}

// ============================================================================
// Pattern Detection - Pure Functions
// ============================================================================

/// Detect code patterns in a parsed AST.
///
/// This is a pure function - deterministic, no I/O, no `Result`.
///
/// Patterns detected:
/// - God objects (structs with many fields)
/// - Long functions (functions with many statements)
/// - Many parameters (functions with many arguments)
/// - Deep nesting (deeply nested control flow)
/// - Complex matches (match expressions with many arms)
///
/// # Example
///
/// ```rust
/// let ast: syn::File = syn::parse_str(r#"
///     struct BigStruct {
///         field1: i32, field2: i32, field3: i32,
///         field4: i32, field5: i32, field6: i32,
///     }
/// "#).unwrap();
/// let patterns = debtmap::complexity::pure::detect_patterns_pure(&ast);
/// assert!(!patterns.is_empty()); // Should detect god object
/// ```
pub fn detect_patterns_pure(file: &File) -> Vec<Pattern> {
    let mut patterns = Vec::new();

    for item in &file.items {
        patterns.extend(detect_item_patterns(item));
    }

    patterns
}

fn detect_item_patterns(item: &Item) -> Vec<Pattern> {
    match item {
        Item::Struct(s) => detect_struct_patterns(s),
        Item::Fn(f) => detect_function_patterns(f),
        Item::Impl(i) => detect_impl_patterns(i),
        _ => vec![],
    }
}

fn detect_struct_patterns(s: &syn::ItemStruct) -> Vec<Pattern> {
    let mut patterns = Vec::new();

    // God object detection (structs with more than 5 fields)
    let field_count = match &s.fields {
        syn::Fields::Named(named) => named.named.len(),
        syn::Fields::Unnamed(unnamed) => unnamed.unnamed.len(),
        syn::Fields::Unit => 0,
    };

    if field_count > 5 {
        patterns.push(Pattern::GodObject {
            name: s.ident.to_string(),
            field_count,
        });
    }

    patterns
}

fn detect_function_patterns(f: &syn::ItemFn) -> Vec<Pattern> {
    let mut patterns = Vec::new();
    let name = f.sig.ident.to_string();

    // Long function detection
    let line_count = count_stmts_recursive(&f.block);
    if line_count > 50 {
        patterns.push(Pattern::LongFunction {
            name: name.clone(),
            lines: line_count,
        });
    }

    // Many parameters detection
    let param_count = f.sig.inputs.len();
    if param_count > 5 {
        patterns.push(Pattern::ManyParameters {
            name: name.clone(),
            param_count,
        });
    }

    // Deep nesting detection
    let max_depth = calculate_max_nesting_depth(&f.block);
    if max_depth > 4 {
        patterns.push(Pattern::DeepNesting {
            function_name: name,
            depth: max_depth,
        });
    }

    patterns
}

fn detect_impl_patterns(i: &syn::ItemImpl) -> Vec<Pattern> {
    i.items
        .iter()
        .filter_map(|item| {
            if let ImplItem::Fn(method) = item {
                Some(detect_method_patterns(method))
            } else {
                None
            }
        })
        .flatten()
        .collect()
}

fn detect_method_patterns(method: &syn::ImplItemFn) -> Vec<Pattern> {
    let mut patterns = Vec::new();
    let name = method.sig.ident.to_string();

    // Long function detection
    let line_count = count_stmts_recursive(&method.block);
    if line_count > 50 {
        patterns.push(Pattern::LongFunction {
            name: name.clone(),
            lines: line_count,
        });
    }

    // Many parameters detection (excluding self)
    let param_count = method
        .sig
        .inputs
        .iter()
        .filter(|arg| !matches!(arg, syn::FnArg::Receiver(_)))
        .count();
    if param_count > 5 {
        patterns.push(Pattern::ManyParameters {
            name: name.clone(),
            param_count,
        });
    }

    // Deep nesting detection
    let max_depth = calculate_max_nesting_depth(&method.block);
    if max_depth > 4 {
        patterns.push(Pattern::DeepNesting {
            function_name: name,
            depth: max_depth,
        });
    }

    patterns
}

fn count_stmts_recursive(block: &Block) -> usize {
    let mut count = block.stmts.len();

    for stmt in &block.stmts {
        if let Stmt::Expr(expr, _) = stmt {
            count += count_expr_stmts(expr);
        }
    }

    count
}

fn count_expr_stmts(expr: &Expr) -> usize {
    match expr {
        Expr::Block(b) => count_stmts_recursive(&b.block),
        Expr::If(if_expr) => {
            let then_count = count_stmts_recursive(&if_expr.then_branch);
            let else_count = if_expr
                .else_branch
                .as_ref()
                .map(|(_, e)| count_expr_stmts(e))
                .unwrap_or(0);
            then_count + else_count
        }
        Expr::While(while_expr) => count_stmts_recursive(&while_expr.body),
        Expr::ForLoop(for_expr) => count_stmts_recursive(&for_expr.body),
        Expr::Loop(loop_expr) => count_stmts_recursive(&loop_expr.body),
        Expr::Match(match_expr) => match_expr
            .arms
            .iter()
            .map(|arm| count_expr_stmts(&arm.body))
            .sum(),
        _ => 0,
    }
}

/// Calculate the maximum nesting depth in a block.
pub fn calculate_max_nesting_depth(block: &Block) -> u32 {
    calculate_block_nesting_depth(block, 0)
}

fn calculate_block_nesting_depth(block: &Block, current_depth: u32) -> u32 {
    let mut max_depth = current_depth;

    for stmt in &block.stmts {
        if let Stmt::Expr(expr, _) = stmt {
            let depth = calculate_expr_nesting_depth(expr, current_depth);
            max_depth = max_depth.max(depth);
        }
    }

    max_depth
}

fn calculate_expr_nesting_depth(expr: &Expr, current_depth: u32) -> u32 {
    match expr {
        Expr::If(if_expr) => {
            let new_depth = current_depth + 1;
            let then_depth = calculate_block_nesting_depth(&if_expr.then_branch, new_depth);
            let else_depth = if_expr
                .else_branch
                .as_ref()
                .map(|(_, e)| calculate_expr_nesting_depth(e, current_depth))
                .unwrap_or(current_depth);
            then_depth.max(else_depth)
        }
        Expr::While(while_expr) => {
            calculate_block_nesting_depth(&while_expr.body, current_depth + 1)
        }
        Expr::ForLoop(for_expr) => calculate_block_nesting_depth(&for_expr.body, current_depth + 1),
        Expr::Loop(loop_expr) => calculate_block_nesting_depth(&loop_expr.body, current_depth + 1),
        Expr::Match(match_expr) => {
            let new_depth = current_depth + 1;
            match_expr
                .arms
                .iter()
                .map(|arm| calculate_expr_nesting_depth(&arm.body, new_depth))
                .max()
                .unwrap_or(new_depth)
        }
        Expr::Block(block_expr) => calculate_block_nesting_depth(&block_expr.block, current_depth),
        _ => current_depth,
    }
}

// ============================================================================
// Additional Pure Helper Functions
// ============================================================================

/// Check if a match expression represents pure mapping (no side effects).
///
/// A pure mapping match has:
/// - Simple pattern matching (no guards)
/// - All arms return values (no side effects)
/// - No early returns or breaks
pub fn is_pure_mapping_match(match_expr: &syn::ExprMatch) -> bool {
    match_expr.arms.iter().all(|arm| {
        // No guard conditions
        arm.guard.is_none()
        // Body is a simple expression (not a block with statements)
        && !matches!(&*arm.body, Expr::Block(b) if b.block.stmts.len() > 1)
    })
}

/// Calculate the nesting depth of a specific expression.
pub fn calculate_nesting_depth(block: &Block) -> u32 {
    calculate_max_nesting_depth(block)
}

/// Count the number of function/method branches.
///
/// This is useful for determining test coverage requirements.
pub fn count_branches(block: &Block) -> u32 {
    count_function_branches(block).saturating_sub(1)
}

// ============================================================================
// Visitor for Complex Pattern Detection
// ============================================================================

struct ComplexMatchVisitor {
    patterns: Vec<Pattern>,
    threshold: usize,
}

impl<'ast> Visit<'ast> for ComplexMatchVisitor {
    fn visit_expr_match(&mut self, node: &'ast syn::ExprMatch) {
        if node.arms.len() > self.threshold {
            self.patterns.push(Pattern::ComplexMatch {
                arm_count: node.arms.len(),
            });
        }
        syn::visit::visit_expr_match(self, node);
    }
}

/// Detect complex match expressions in a file.
///
/// A match is considered complex if it has more arms than the threshold.
pub fn detect_complex_matches(file: &File, threshold: usize) -> Vec<Pattern> {
    let mut visitor = ComplexMatchVisitor {
        patterns: Vec::new(),
        threshold,
    };
    visitor.visit_file(file);
    visitor.patterns
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // Cyclomatic Complexity Tests
    // ========================================================================

    #[test]
    fn test_cyclomatic_empty_function() {
        let ast: File = syn::parse_str("fn foo() {}").unwrap();
        assert_eq!(calculate_cyclomatic_pure(&ast), 1);
    }

    #[test]
    fn test_cyclomatic_simple_if() {
        let ast: File = syn::parse_str(
            r#"
            fn foo(x: bool) {
                if x {
                    println!("yes");
                }
            }
        "#,
        )
        .unwrap();
        assert_eq!(calculate_cyclomatic_pure(&ast), 2);
    }

    #[test]
    fn test_cyclomatic_if_else() {
        let ast: File = syn::parse_str(
            r#"
            fn foo(x: bool) {
                if x {
                    println!("yes");
                } else {
                    println!("no");
                }
            }
        "#,
        )
        .unwrap();
        assert_eq!(calculate_cyclomatic_pure(&ast), 3);
    }

    #[test]
    fn test_cyclomatic_match() {
        let ast: File = syn::parse_str(
            r#"
            fn foo(x: Option<i32>) {
                match x {
                    Some(v) => println!("{}", v),
                    None => println!("none"),
                }
            }
        "#,
        )
        .unwrap();
        // Match with 2 arms: base 1 + (2-1) arms = 2
        assert_eq!(calculate_cyclomatic_pure(&ast), 2);
    }

    #[test]
    fn test_cyclomatic_while_loop() {
        let ast: File = syn::parse_str(
            r#"
            fn foo(mut x: i32) {
                while x > 0 {
                    x -= 1;
                }
            }
        "#,
        )
        .unwrap();
        assert_eq!(calculate_cyclomatic_pure(&ast), 2);
    }

    #[test]
    fn test_cyclomatic_for_loop() {
        let ast: File = syn::parse_str(
            r#"
            fn foo() {
                for i in 0..10 {
                    println!("{}", i);
                }
            }
        "#,
        )
        .unwrap();
        assert_eq!(calculate_cyclomatic_pure(&ast), 2);
    }

    #[test]
    fn test_cyclomatic_nested() {
        let ast: File = syn::parse_str(
            r#"
            fn foo(x: bool, y: bool) {
                if x {
                    if y {
                        println!("both");
                    }
                }
            }
        "#,
        )
        .unwrap();
        // Two if statements: base 1 + 2 = 3
        assert_eq!(calculate_cyclomatic_pure(&ast), 3);
    }

    #[test]
    fn test_cyclomatic_try_operator() {
        let ast: File = syn::parse_str(
            r#"
            fn foo(x: Result<i32, ()>) -> Result<i32, ()> {
                let v = x?;
                Ok(v + 1)
            }
        "#,
        )
        .unwrap();
        // Try operator adds 1: base 1 + 1 = 2
        assert_eq!(calculate_cyclomatic_pure(&ast), 2);
    }

    // ========================================================================
    // Cognitive Complexity Tests
    // ========================================================================

    #[test]
    fn test_cognitive_empty_function() {
        let ast: File = syn::parse_str("fn foo() {}").unwrap();
        assert_eq!(calculate_cognitive_pure(&ast), 0);
    }

    #[test]
    fn test_cognitive_simple_if() {
        let ast: File = syn::parse_str(
            r#"
            fn foo(x: bool) {
                if x {
                    println!("yes");
                }
            }
        "#,
        )
        .unwrap();
        // if at nesting 0: 1 + 0 = 1
        assert_eq!(calculate_cognitive_pure(&ast), 1);
    }

    #[test]
    fn test_cognitive_nested_if() {
        let ast: File = syn::parse_str(
            r#"
            fn foo(x: bool, y: bool) {
                if x {
                    if y {
                        println!("both");
                    }
                }
            }
        "#,
        )
        .unwrap();
        // Outer if: 1 + 0 = 1
        // Inner if: 1 + 1 = 2
        // Total: 3
        assert_eq!(calculate_cognitive_pure(&ast), 3);
    }

    #[test]
    fn test_cognitive_deeply_nested() {
        let ast: File = syn::parse_str(
            r#"
            fn foo(a: bool, b: bool, c: bool) {
                if a {
                    if b {
                        if c {
                            println!("all");
                        }
                    }
                }
            }
        "#,
        )
        .unwrap();
        // Level 0: 1
        // Level 1: 2
        // Level 2: 3
        // Total: 6
        assert_eq!(calculate_cognitive_pure(&ast), 6);
    }

    #[test]
    fn test_cognitive_logical_operators() {
        let ast: File = syn::parse_str(
            r#"
            fn foo(a: bool, b: bool, c: bool) {
                if a && b || c {
                    println!("complex");
                }
            }
        "#,
        )
        .unwrap();
        // if: 1, && : 1, ||: 1 = 3
        assert_eq!(calculate_cognitive_pure(&ast), 3);
    }

    #[test]
    fn test_cognitive_else_if_chain() {
        let ast: File = syn::parse_str(
            r#"
            fn foo(x: i32) {
                if x < 0 {
                    println!("negative");
                } else if x == 0 {
                    println!("zero");
                } else {
                    println!("positive");
                }
            }
        "#,
        )
        .unwrap();
        // First if: 1
        // else if (doesn't add nesting): 1
        // else: 1
        // Total: 3
        assert_eq!(calculate_cognitive_pure(&ast), 3);
    }

    // ========================================================================
    // Pattern Detection Tests
    // ========================================================================

    #[test]
    fn test_detect_god_object() {
        let ast: File = syn::parse_str(
            r#"
            struct BigStruct {
                field1: i32,
                field2: i32,
                field3: i32,
                field4: i32,
                field5: i32,
                field6: i32,
            }
        "#,
        )
        .unwrap();
        let patterns = detect_patterns_pure(&ast);
        assert_eq!(patterns.len(), 1);
        assert!(matches!(
            &patterns[0],
            Pattern::GodObject {
                name,
                field_count: 6
            } if name == "BigStruct"
        ));
    }

    #[test]
    fn test_detect_many_parameters() {
        let ast: File = syn::parse_str(
            r#"
            fn many_params(a: i32, b: i32, c: i32, d: i32, e: i32, f: i32) {}
        "#,
        )
        .unwrap();
        let patterns = detect_patterns_pure(&ast);
        assert_eq!(patterns.len(), 1);
        assert!(matches!(
            &patterns[0],
            Pattern::ManyParameters {
                name,
                param_count: 6
            } if name == "many_params"
        ));
    }

    #[test]
    fn test_detect_deep_nesting() {
        let ast: File = syn::parse_str(
            r#"
            fn deep() {
                if true {
                    if true {
                        if true {
                            if true {
                                if true {
                                    println!("deep");
                                }
                            }
                        }
                    }
                }
            }
        "#,
        )
        .unwrap();
        let patterns = detect_patterns_pure(&ast);
        assert!(patterns.iter().any(|p| matches!(
            p,
            Pattern::DeepNesting { depth, .. } if *depth > 4
        )));
    }

    #[test]
    fn test_detect_complex_match() {
        let ast: File = syn::parse_str(
            r#"
            fn foo(x: i32) {
                match x {
                    0 => println!("zero"),
                    1 => println!("one"),
                    2 => println!("two"),
                    3 => println!("three"),
                    4 => println!("four"),
                    5 => println!("five"),
                    _ => println!("other"),
                }
            }
        "#,
        )
        .unwrap();
        let patterns = detect_complex_matches(&ast, 5);
        assert_eq!(patterns.len(), 1);
        assert!(matches!(
            &patterns[0],
            Pattern::ComplexMatch { arm_count: 7 }
        ));
    }

    #[test]
    fn test_is_pure_mapping_match_true() {
        let code = r#"
            match x {
                Some(v) => v,
                None => 0,
            }
        "#;
        let expr: syn::ExprMatch = syn::parse_str(code).unwrap();
        assert!(is_pure_mapping_match(&expr));
    }

    #[test]
    fn test_is_pure_mapping_match_false_guard() {
        let code = r#"
            match x {
                Some(v) if v > 0 => v,
                _ => 0,
            }
        "#;
        let expr: syn::ExprMatch = syn::parse_str(code).unwrap();
        assert!(!is_pure_mapping_match(&expr));
    }

    // ========================================================================
    // Determinism Tests (Pure Function Property)
    // ========================================================================

    #[test]
    fn test_cyclomatic_deterministic() {
        let code = r#"
            fn example(x: i32) {
                if x > 0 {
                    while x > 10 {
                        println!("big");
                    }
                } else {
                    println!("small");
                }
            }
        "#;
        let ast: File = syn::parse_str(code).unwrap();

        // Run multiple times - should always get same result
        let results: Vec<u32> = (0..10).map(|_| calculate_cyclomatic_pure(&ast)).collect();
        assert!(results.iter().all(|&r| r == results[0]));
    }

    #[test]
    fn test_cognitive_deterministic() {
        let code = r#"
            fn example(a: bool, b: bool) {
                if a && b {
                    for i in 0..10 {
                        if i % 2 == 0 {
                            println!("{}", i);
                        }
                    }
                }
            }
        "#;
        let ast: File = syn::parse_str(code).unwrap();

        // Run multiple times - should always get same result
        let results: Vec<u32> = (0..10).map(|_| calculate_cognitive_pure(&ast)).collect();
        assert!(results.iter().all(|&r| r == results[0]));
    }

    #[test]
    fn test_pattern_detection_deterministic() {
        let code = r#"
            struct Big { a: i32, b: i32, c: i32, d: i32, e: i32, f: i32 }
            fn many(a: i32, b: i32, c: i32, d: i32, e: i32, f: i32) {}
        "#;
        let ast: File = syn::parse_str(code).unwrap();

        // Run multiple times - should always get same patterns
        let results: Vec<Vec<Pattern>> = (0..10).map(|_| detect_patterns_pure(&ast)).collect();
        let first = &results[0];
        assert!(results.iter().all(|r| r.len() == first.len()));
    }

    // ========================================================================
    // Helper Function Tests
    // ========================================================================

    #[test]
    fn test_calculate_nesting_depth() {
        let block: Block = syn::parse_str(
            r#"{
            if true {
                if true {
                    if true {}
                }
            }
        }"#,
        )
        .unwrap();
        assert_eq!(calculate_nesting_depth(&block), 3);
    }

    #[test]
    fn test_count_branches() {
        let block: Block = syn::parse_str(
            r#"{
            if a { }
            if b { }
        }"#,
        )
        .unwrap();
        // Two if statements = 2 extra branches
        assert_eq!(count_branches(&block), 2);
    }

    #[test]
    fn test_impl_method_detection() {
        let ast: File = syn::parse_str(
            r#"
            struct Foo;
            impl Foo {
                fn method(&self, a: i32, b: i32, c: i32, d: i32, e: i32, f: i32) {}
            }
        "#,
        )
        .unwrap();
        let patterns = detect_patterns_pure(&ast);
        assert!(patterns.iter().any(|p| matches!(
            p,
            Pattern::ManyParameters { name, param_count: 6 } if name == "method"
        )));
    }
}
