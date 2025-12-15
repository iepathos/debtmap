//! Pure complexity calculation functions for Rust code analysis.
//!
//! This module contains functions for calculating various complexity metrics
//! including cyclomatic complexity, cognitive complexity, nesting depth, and
//! line counts. All functions are pure and side-effect-free.

use crate::complexity::{
    cognitive::calculate_cognitive_with_patterns,
    visitor_detector::{PatternInfo, PatternType},
};
use syn::visit::Visit;

/// Calculate cyclomatic complexity with visitor pattern detection.
///
/// Returns the RAW cyclomatic complexity (before any dampening).
/// Pattern-based adjustments should be stored separately in adjusted_complexity field.
/// This ensures pattern detection logic can access the true complexity metrics.
pub fn calculate_cyclomatic_with_visitor(
    block: &syn::Block,
    _func: &syn::ItemFn,
    _file_ast: Option<&syn::File>,
) -> u32 {
    // ALWAYS return raw cyclomatic complexity
    // Pattern detection and dampening should happen separately
    use crate::complexity::cyclomatic::calculate_cyclomatic;
    calculate_cyclomatic(block)
}

/// Calculate cognitive complexity with visitor pattern detection.
///
/// If a visitor pattern is detected, applies pattern-specific scaling to the cognitive complexity.
/// Otherwise, falls back to standard cognitive complexity calculation.
pub fn calculate_cognitive_with_visitor(
    block: &syn::Block,
    func: &syn::ItemFn,
    file_ast: Option<&syn::File>,
) -> u32 {
    try_detect_visitor_pattern(func, file_ast)
        .map(|pattern_info| apply_cognitive_pattern_scaling(block, &pattern_info))
        .unwrap_or_else(|| calculate_cognitive_syn(block))
}

/// Apply pattern-specific scaling to cognitive complexity.
///
/// Different patterns get different complexity adjustments:
/// - Visitor: logarithmic scaling (encourages pattern usage)
/// - ExhaustiveMatch: square root scaling (moderate reduction)
/// - SimpleMapping: 20% of base (significant reduction)
/// - Others: no scaling
fn apply_cognitive_pattern_scaling(block: &syn::Block, pattern_info: &PatternInfo) -> u32 {
    let base_cognitive = calculate_cognitive_syn(block);

    match pattern_info.pattern_type {
        PatternType::Visitor => ((base_cognitive as f32).log2().ceil()).max(1.0) as u32,
        PatternType::ExhaustiveMatch => ((base_cognitive as f32).sqrt().ceil()).max(2.0) as u32,
        PatternType::SimpleMapping => ((base_cognitive as f32) * 0.2).max(1.0) as u32,
        _ => base_cognitive,
    }
}

/// Calculate cognitive complexity for a syn block.
///
/// Uses the enhanced version that includes pattern detection.
pub fn calculate_cognitive_syn(block: &syn::Block) -> u32 {
    let (total, _patterns) = calculate_cognitive_with_patterns(block);
    total
}

/// Try to detect visitor pattern in a function.
///
/// Returns pattern info if detected, None otherwise.
fn try_detect_visitor_pattern(
    func: &syn::ItemFn,
    file_ast: Option<&syn::File>,
) -> Option<PatternInfo> {
    use crate::complexity::visitor_detector::detect_visitor_pattern;

    file_ast.and_then(|ast| detect_visitor_pattern(ast, func))
}

/// Calculate maximum nesting depth in a block.
///
/// Counts nesting levels of control flow structures (if, while, for, loop, match).
/// Returns the maximum depth encountered.
pub fn calculate_nesting(block: &syn::Block) -> u32 {
    struct NestingVisitor {
        current_depth: u32,
        max_depth: u32,
    }

    impl NestingVisitor {
        fn visit_nested<F>(&mut self, f: F)
        where
            F: FnOnce(&mut Self),
        {
            self.current_depth += 1;
            self.max_depth = self.max_depth.max(self.current_depth);
            f(self);
            self.current_depth -= 1;
        }
    }

    impl<'ast> Visit<'ast> for NestingVisitor {
        fn visit_expr_if(&mut self, i: &'ast syn::ExprIf) {
            // Increment nesting for the if itself
            self.current_depth += 1;
            self.max_depth = self.max_depth.max(self.current_depth);

            // Visit condition (no nesting change)
            self.visit_expr(&i.cond);

            // Visit then branch (already at incremented depth)
            self.visit_block(&i.then_branch);

            // Decrement before visiting else branch so else-if stays flat
            self.current_depth -= 1;

            // Visit else branch at original nesting level
            // This handles else-if chains correctly
            if let Some((_, else_expr)) = &i.else_branch {
                self.visit_expr(else_expr);
            }
        }

        fn visit_expr_while(&mut self, i: &'ast syn::ExprWhile) {
            self.visit_nested(|v| syn::visit::visit_expr_while(v, i));
        }

        fn visit_expr_for_loop(&mut self, i: &'ast syn::ExprForLoop) {
            self.visit_nested(|v| syn::visit::visit_expr_for_loop(v, i));
        }

        fn visit_expr_loop(&mut self, i: &'ast syn::ExprLoop) {
            self.visit_nested(|v| syn::visit::visit_expr_loop(v, i));
        }

        fn visit_expr_match(&mut self, i: &'ast syn::ExprMatch) {
            self.current_depth += 1;
            self.max_depth = self.max_depth.max(self.current_depth);

            self.visit_expr(&i.expr);

            for arm in &i.arms {
                self.visit_arm(arm);
            }

            self.current_depth -= 1;
        }

        fn visit_arm(&mut self, i: &'ast syn::Arm) {
            for attr in &i.attrs {
                self.visit_attribute(attr);
            }
            self.visit_pat(&i.pat);
            if let Some((_, guard)) = &i.guard {
                self.visit_expr(guard);
            }
            self.visit_expr(&i.body);
        }

        fn visit_block(&mut self, block: &'ast syn::Block) {
            syn::visit::visit_block(self, block);
        }
    }

    let mut visitor = NestingVisitor {
        current_depth: 0,
        max_depth: 0,
    };

    for stmt in &block.stmts {
        visitor.visit_stmt(stmt);
    }

    visitor.max_depth
}

/// Count the number of source lines in a block.
///
/// Returns the span of lines from start to end of the block.
pub fn count_lines(block: &syn::Block) -> usize {
    use syn::spanned::Spanned;

    let span = block.span();
    let start_line = span.start().line;
    let end_line = span.end().line;

    if end_line >= start_line {
        end_line - start_line + 1
    } else {
        1
    }
}

/// Count the number of source lines in a function.
///
/// Returns the span of lines from the function signature to the end of its body.
pub fn count_function_lines(item_fn: &syn::ItemFn) -> usize {
    use syn::spanned::Spanned;

    let span = item_fn.span();
    let start_line = span.start().line;
    let end_line = span.end().line;

    if end_line >= start_line {
        end_line - start_line + 1
    } else {
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_count_lines_simple_block() {
        let code = r#"
        fn test() {
            let x = 1;
            let y = 2;
        }
        "#;
        let file: syn::File = syn::parse_str(code).unwrap();
        if let syn::Item::Fn(item_fn) = &file.items[0] {
            let lines = count_lines(&item_fn.block);
            assert!(lines > 0);
        }
    }

    #[test]
    fn test_calculate_nesting_simple() {
        let code = r#"
        {
            let x = 1;
        }
        "#;
        let block: syn::Block = syn::parse_str(code).unwrap();
        let nesting = calculate_nesting(&block);
        assert_eq!(nesting, 0);
    }

    #[test]
    fn test_calculate_nesting_with_if() {
        let code = r#"
        {
            if true {
                let x = 1;
            }
        }
        "#;
        let block: syn::Block = syn::parse_str(code).unwrap();
        let nesting = calculate_nesting(&block);
        assert_eq!(nesting, 1);
    }

    #[test]
    fn test_calculate_nesting_nested() {
        let code = r#"
        {
            if true {
                for i in 0..10 {
                    let x = 1;
                }
            }
        }
        "#;
        let block: syn::Block = syn::parse_str(code).unwrap();
        let nesting = calculate_nesting(&block);
        assert_eq!(nesting, 2);
    }

    #[test]
    fn test_else_if_chain_flat_nesting() {
        let code = r#"
        {
            if a {
                x
            } else if b {
                y
            } else if c {
                z
            } else {
                w
            }
        }
        "#;
        let block: syn::Block = syn::parse_str(code).unwrap();
        assert_eq!(
            calculate_nesting(&block),
            1,
            "else-if chain should have nesting 1"
        );
    }

    #[test]
    fn test_nested_if_inside_then() {
        let code = r#"
        {
            if a {
                if b {
                    x
                }
            }
        }
        "#;
        let block: syn::Block = syn::parse_str(code).unwrap();
        assert_eq!(
            calculate_nesting(&block),
            2,
            "if inside then should have nesting 2"
        );
    }

    #[test]
    fn test_match_with_else_if_chain() {
        let code = r#"
        {
            match x {
                A => {
                    if a {
                    } else if b {
                    } else if c {
                    }
                }
                _ => {}
            }
        }
        "#;
        let block: syn::Block = syn::parse_str(code).unwrap();
        assert_eq!(
            calculate_nesting(&block),
            2,
            "match + else-if chain should have nesting 2"
        );
    }

    #[test]
    fn test_long_else_if_chain_nesting() {
        let code = r#"
        {
            if a {
            } else if b {
            } else if c {
            } else if d {
            } else if e {
            } else if f {
            } else if g {
            } else if h {
            }
        }
        "#;
        let block: syn::Block = syn::parse_str(code).unwrap();
        assert_eq!(
            calculate_nesting(&block),
            1,
            "long else-if chain should still have nesting 1"
        );
    }
}
