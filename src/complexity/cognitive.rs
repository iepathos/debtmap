//! Cognitive complexity calculation for Rust code blocks.
//!
//! **Note**: For new code, prefer using [`super::pure::calculate_cognitive_pure`]
//! which operates directly on `syn::File` AST and is faster to test.
//! This module provides block-level complexity calculation with additional
//! features like semantic normalization and pattern detection.
//!
//! The pure functions in `pure.rs` are:
//! - Deterministic (same input = same output)
//! - Fast to test (no I/O overhead)
//! - Easier to compose
//!
//! This module provides more sophisticated analysis including:
//! - Pattern-based complexity adjustments
//! - Semantic normalization
//! - Recursive match detection
//!
//! See [`super::pure`] for simpler file-level pure functions.

use super::pattern_adjustments::calculate_cognitive_adjusted;
use super::patterns::{analyze_patterns, PatternComplexity};
use super::recursive_detector::RecursiveMatchDetector;
use super::rust_normalizer::RustSemanticNormalizer;
use super::semantic_normalizer::{
    NormalizedBlock, NormalizedExpression, NormalizedStatement, SemanticNormalizer,
};
use syn::{visit::Visit, Block, Expr};

/// Calculate cognitive complexity using the visitor-based algorithm.
///
/// This is the default implementation for debtmap, chosen for consistency
/// with historical scoring and prioritization. For new analysis that benefits
/// from semantic normalization (e.g., handling multiline formatting), use
/// [`calculate_cognitive_normalized`] instead.
///
/// # Algorithm
/// Uses AST visitor pattern to traverse code blocks and calculate complexity
/// based on control flow constructs, with pattern-based adjustments for
/// recursive structures and complex match expressions.
pub fn calculate_cognitive(block: &Block) -> u32 {
    calculate_cognitive_visitor_based(block)
}

/// Calculate cognitive complexity using semantic normalization
pub fn calculate_cognitive_normalized(block: &Block) -> u32 {
    let normalizer = RustSemanticNormalizer::new();
    let normalized = normalizer.normalize(block.clone());

    let complexity = calculate_from_normalized(&normalized);

    // Add pattern-based complexity (using original block for now)
    let patterns = analyze_patterns(block);
    let base_complexity = complexity + patterns.total_complexity();

    // Only apply pattern-specific adjustments if complexity is significant
    // This avoids false positives on simple blocks
    if base_complexity > 2 {
        calculate_cognitive_adjusted(block, base_complexity)
    } else {
        base_complexity
    }
}

/// Calculate cognitive complexity using visitor-based AST traversal.
///
/// This is the historical algorithm used by debtmap for cognitive complexity
/// calculation. It uses the visitor pattern to traverse the AST and accumulates
/// complexity based on control flow constructs and nesting levels.
///
/// For most use cases, prefer [`calculate_cognitive`] which is an alias to this
/// function. For semantic normalization features, use [`calculate_cognitive_normalized`].
pub fn calculate_cognitive_visitor_based(block: &Block) -> u32 {
    let mut visitor = CognitiveVisitor {
        complexity: 0,
        nesting_level: 0,
    };
    visitor.visit_block(block);

    // Add pattern-based complexity
    let patterns = analyze_patterns(block);
    let base_complexity = visitor.complexity + patterns.total_complexity();

    // Only apply pattern-specific adjustments if complexity is significant
    // This avoids false positives on simple blocks
    if base_complexity > 2 {
        calculate_cognitive_adjusted(block, base_complexity)
    } else {
        base_complexity
    }
}

pub fn calculate_cognitive_with_patterns(block: &Block) -> (u32, PatternComplexity) {
    let mut visitor = CognitiveVisitor {
        complexity: 0,
        nesting_level: 0,
    };
    visitor.visit_block(block);

    let patterns = analyze_patterns(block);
    let base_complexity = visitor.complexity + patterns.total_complexity();
    let adjusted_total = if base_complexity > 2 {
        calculate_cognitive_adjusted(block, base_complexity)
    } else {
        base_complexity
    };
    (adjusted_total, patterns)
}

/// Calculate cognitive complexity with recursive match detection
pub fn calculate_cognitive_with_recursive_matches(
    block: &Block,
) -> (u32, Vec<super::recursive_detector::MatchLocation>) {
    let mut visitor = CognitiveVisitor {
        complexity: 0,
        nesting_level: 0,
    };
    visitor.visit_block(block);

    // Use recursive detector to find all matches
    let mut detector = RecursiveMatchDetector::new();
    let matches = detector.find_matches_in_block(block);

    // Add pattern-based complexity
    let patterns = analyze_patterns(block);
    let base_complexity = visitor.complexity + patterns.total_complexity();

    // Apply adjustments
    let adjusted_total = if base_complexity > 2 {
        calculate_cognitive_adjusted(block, base_complexity)
    } else {
        base_complexity
    };

    (adjusted_total, matches)
}

struct CognitiveVisitor {
    complexity: u32,
    nesting_level: u32,
}

struct ExprMetrics {
    base_complexity: u32,
    extra_complexity: u32,
    increases_nesting: bool,
}

// Pure enum for expression classification
enum ExprClassification {
    ControlFlow,
    Match(u32), // Number of arms
    LogicalOp,
    Closure { is_async: bool },
    Await,
    Unsafe,
    Other,
}

impl CognitiveVisitor {
    fn calculate_expr_metrics(&self, expr: &Expr) -> ExprMetrics {
        // Extract classification logic as a pure function
        let classification = Self::classify_expr(expr);

        // Calculate metrics based on classification
        match classification {
            ExprClassification::ControlFlow => ExprMetrics {
                base_complexity: 1 + self.nesting_level,
                extra_complexity: 0,
                increases_nesting: true,
            },
            ExprClassification::Match(arm_count) => ExprMetrics {
                base_complexity: 1 + self.nesting_level,
                extra_complexity: arm_count,
                increases_nesting: true,
            },
            ExprClassification::LogicalOp => ExprMetrics {
                base_complexity: 1,
                extra_complexity: 0,
                increases_nesting: false,
            },
            ExprClassification::Closure { is_async } => {
                let base = if is_async { 2 } else { 1 };
                ExprMetrics {
                    base_complexity: base + self.nesting_level.min(1),
                    extra_complexity: 0,
                    increases_nesting: false,
                }
            }
            ExprClassification::Await => ExprMetrics {
                base_complexity: 1,
                extra_complexity: 0,
                increases_nesting: false,
            },
            ExprClassification::Unsafe => ExprMetrics {
                base_complexity: 2,
                extra_complexity: 0,
                increases_nesting: true,
            },
            ExprClassification::Other => ExprMetrics {
                base_complexity: 0,
                extra_complexity: 0,
                increases_nesting: false,
            },
        }
    }

    // Pure function for expression classification
    fn classify_expr(expr: &Expr) -> ExprClassification {
        match expr {
            Expr::If(_) | Expr::While(_) | Expr::ForLoop(_) | Expr::Loop(_) | Expr::Try(_) => {
                ExprClassification::ControlFlow
            }
            Expr::Match(expr_match) => ExprClassification::Match(expr_match.arms.len() as u32),
            Expr::Binary(binary) if is_logical_operator(&binary.op) => {
                ExprClassification::LogicalOp
            }
            Expr::Closure(closure) => ExprClassification::Closure {
                is_async: closure.asyncness.is_some(),
            },
            Expr::Await(_) => ExprClassification::Await,
            Expr::Unsafe(_) => ExprClassification::Unsafe,
            _ => ExprClassification::Other,
        }
    }

    fn visit_with_nesting(&mut self, expr: &Expr, increases_nesting: bool) {
        if increases_nesting {
            self.nesting_level += 1;
            syn::visit::visit_expr(self, expr);
            self.nesting_level -= 1;
        } else {
            syn::visit::visit_expr(self, expr);
        }
    }
}

impl<'ast> Visit<'ast> for CognitiveVisitor {
    fn visit_expr(&mut self, expr: &'ast Expr) {
        let metrics = self.calculate_expr_metrics(expr);
        self.complexity += metrics.base_complexity + metrics.extra_complexity;

        self.visit_with_nesting(expr, metrics.increases_nesting);
    }

    fn visit_block(&mut self, block: &'ast Block) {
        syn::visit::visit_block(self, block);
    }
}

fn is_logical_operator(op: &syn::BinOp) -> bool {
    matches!(op, syn::BinOp::And(_) | syn::BinOp::Or(_))
}

pub fn calculate_cognitive_penalty(nesting: u32) -> u32 {
    static PENALTY_TABLE: &[(u32, u32)] = &[(0, 0), (1, 1), (2, 2), (3, 4)];

    PENALTY_TABLE
        .iter()
        .find(|(level, _)| *level == nesting)
        .map(|(_, penalty)| *penalty)
        .unwrap_or(8)
}

pub fn combine_cognitive(complexities: Vec<u32>) -> u32 {
    complexities.iter().sum()
}

/// Calculate complexity from normalized AST
fn calculate_from_normalized(normalized: &NormalizedBlock) -> u32 {
    calculate_from_normalized_internal(normalized, 0)
}

fn calculate_from_normalized_internal(normalized: &NormalizedBlock, nesting_level: u32) -> u32 {
    let mut complexity = 0;

    for stmt in &normalized.statements {
        let (stmt_complexity, increases_nesting) =
            calculate_statement_complexity(stmt, nesting_level);
        complexity += stmt_complexity;

        if increases_nesting {
            // Process nested statements recursively with increased nesting
            if let NormalizedStatement::Control(control) = stmt {
                let nested_complexity =
                    calculate_from_normalized_internal(&control.body, nesting_level + 1);
                complexity += nested_complexity;
            }
        }
    }

    // Apply formatting artifact reduction only at the top level
    if nesting_level == 0
        && normalized
            .formatting_metadata
            .multiline_expressions_normalized
            > 0
    {
        let reduction = normalized
            .formatting_metadata
            .multiline_expressions_normalized
            .min(complexity / 8);
        complexity = complexity.saturating_sub(reduction);
    }

    complexity
}

/// Calculate complexity for a normalized statement
fn calculate_statement_complexity(stmt: &NormalizedStatement, nesting_level: u32) -> (u32, bool) {
    match stmt {
        NormalizedStatement::Expression(expr) => {
            let complexity = calculate_expression_complexity(expr, nesting_level);
            (complexity, false)
        }
        NormalizedStatement::Control(control) => {
            use super::semantic_normalizer::ControlType;
            let base_complexity = match control.control_type {
                ControlType::If | ControlType::While | ControlType::For | ControlType::Loop => {
                    1 + nesting_level
                }
                ControlType::Match => {
                    // Match expressions get base complexity plus arms
                    1 + nesting_level
                }
                ControlType::Try => 1 + nesting_level,
            };
            (base_complexity, true)
        }
        NormalizedStatement::Local(_) | NormalizedStatement::Declaration(_) => (0, false),
    }
}

/// Calculate complexity for a normalized expression
fn calculate_expression_complexity(expr: &NormalizedExpression, nesting_level: u32) -> u32 {
    use super::semantic_normalizer::ExprType;

    // Don't skip entirely for multiline artifacts, just reduce the penalty
    let artifact_reduction = if expr.is_multiline_artifact { 1 } else { 0 };

    let base_complexity = match &expr.expr_type {
        ExprType::ControlFlow => 1 + nesting_level,
        ExprType::Match { arm_count } => {
            // Base complexity plus number of arms
            1 + nesting_level + (*arm_count as u32)
        }
        ExprType::LogicalOp => 1,
        ExprType::Closure { is_async } => (if *is_async { 2 } else { 1 }) + nesting_level.min(1),
        ExprType::Await => 1,
        ExprType::Unsafe => 2,
        _ => 0,
    };

    // Add complexity from logical components (these are already normalized)
    let component_complexity: u32 = expr
        .logical_components
        .iter()
        .map(|c| c.complexity_contribution)
        .sum();

    (base_complexity + component_complexity).saturating_sub(artifact_reduction)
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn test_classify_expr_control_flow() {
        let if_expr: Expr = parse_quote! { if x > 0 { 1 } else { 0 } };
        assert!(matches!(
            CognitiveVisitor::classify_expr(&if_expr),
            ExprClassification::ControlFlow
        ));

        let while_expr: Expr = parse_quote! { while x > 0 { x -= 1; } };
        assert!(matches!(
            CognitiveVisitor::classify_expr(&while_expr),
            ExprClassification::ControlFlow
        ));

        let for_expr: Expr = parse_quote! { for i in 0..10 { println!("{}", i); } };
        assert!(matches!(
            CognitiveVisitor::classify_expr(&for_expr),
            ExprClassification::ControlFlow
        ));

        let loop_expr: Expr = parse_quote! { loop { break; } };
        assert!(matches!(
            CognitiveVisitor::classify_expr(&loop_expr),
            ExprClassification::ControlFlow
        ));

        let try_expr: Expr = parse_quote! { res? };
        assert!(matches!(
            CognitiveVisitor::classify_expr(&try_expr),
            ExprClassification::ControlFlow
        ));
    }

    #[test]
    fn test_classify_expr_match() {
        let match_expr: Expr = parse_quote! {
            match x {
                0 => "zero",
                1 => "one",
                _ => "many",
            }
        };

        if let ExprClassification::Match(arm_count) = CognitiveVisitor::classify_expr(&match_expr) {
            assert_eq!(arm_count, 3, "Match should have 3 arms");
        } else {
            panic!("Expected Match classification");
        }
    }

    #[test]
    fn test_classify_expr_logical_op() {
        let and_expr: Expr = parse_quote! { x && y };
        assert!(matches!(
            CognitiveVisitor::classify_expr(&and_expr),
            ExprClassification::LogicalOp
        ));

        let or_expr: Expr = parse_quote! { x || y };
        assert!(matches!(
            CognitiveVisitor::classify_expr(&or_expr),
            ExprClassification::LogicalOp
        ));
    }

    #[test]
    fn test_classify_expr_closure() {
        let sync_closure: Expr = parse_quote! { |x| x + 1 };
        assert!(matches!(
            CognitiveVisitor::classify_expr(&sync_closure),
            ExprClassification::Closure { is_async: false }
        ));

        let async_closure: Expr = parse_quote! { async |x| x + 1 };
        assert!(matches!(
            CognitiveVisitor::classify_expr(&async_closure),
            ExprClassification::Closure { is_async: true }
        ));
    }

    #[test]
    fn test_classify_expr_await() {
        let await_expr: Expr = parse_quote! { fut.await };
        assert!(matches!(
            CognitiveVisitor::classify_expr(&await_expr),
            ExprClassification::Await
        ));
    }

    #[test]
    fn test_classify_expr_unsafe() {
        let unsafe_expr: Expr = parse_quote! { unsafe { *ptr } };
        assert!(matches!(
            CognitiveVisitor::classify_expr(&unsafe_expr),
            ExprClassification::Unsafe
        ));
    }

    #[test]
    fn test_classify_expr_other() {
        let call_expr: Expr = parse_quote! { foo(x, y) };
        assert!(matches!(
            CognitiveVisitor::classify_expr(&call_expr),
            ExprClassification::Other
        ));

        let literal_expr: Expr = parse_quote! { 42 };
        assert!(matches!(
            CognitiveVisitor::classify_expr(&literal_expr),
            ExprClassification::Other
        ));
    }

    #[test]
    fn test_calculate_expr_metrics_nesting_effect() {
        let visitor = CognitiveVisitor {
            complexity: 0,
            nesting_level: 2,
        };

        let if_expr: Expr = parse_quote! { if x > 0 { 1 } else { 0 } };
        let metrics = visitor.calculate_expr_metrics(&if_expr);

        assert_eq!(
            metrics.base_complexity, 3,
            "Base complexity should be 1 + nesting_level"
        );
        assert_eq!(metrics.extra_complexity, 0);
        assert!(metrics.increases_nesting);
    }

    #[test]
    fn test_calculate_expr_metrics_match_arms() {
        let visitor = CognitiveVisitor {
            complexity: 0,
            nesting_level: 0,
        };

        let match_expr: Expr = parse_quote! {
            match x {
                0 => "zero",
                1 => "one",
                2 => "two",
                3 => "three",
                _ => "many",
            }
        };
        let metrics = visitor.calculate_expr_metrics(&match_expr);

        assert_eq!(metrics.base_complexity, 1);
        assert_eq!(
            metrics.extra_complexity, 5,
            "Should count number of match arms"
        );
        assert!(metrics.increases_nesting);
    }
}
