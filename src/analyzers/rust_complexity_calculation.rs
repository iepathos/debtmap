//! Pure complexity calculation functions for Rust code analysis.
//!
//! This module contains functions for calculating various complexity metrics
//! including cyclomatic complexity, cognitive complexity, nesting depth, and
//! line counts. All functions are pure and side-effect-free.

use crate::complexity::{
    calculate_cognitive_for_block,
    visitor_detector::{PatternInfo, PatternType},
};

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
/// Uses the pure implementation for consistent, spec-compliant behavior.
pub fn calculate_cognitive_syn(block: &syn::Block) -> u32 {
    calculate_cognitive_for_block(block)
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
///
/// Uses the pure implementation from `complexity::pure::calculate_max_nesting_depth`
/// as the single source of truth for consistent nesting calculations.
pub fn calculate_nesting(block: &syn::Block) -> u32 {
    crate::complexity::pure::calculate_max_nesting_depth(block)
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
/// Matches `FunctionMetrics::line`: the identifier's line through the closing
/// body brace, inclusive. Attributes and signature prefixes before the identifier
/// must not extend the computed end past the actual body.
pub fn count_function_lines(item_fn: &syn::ItemFn) -> usize {
    let start_line = item_fn.sig.ident.span().start().line;
    let end_line = item_fn.block.brace_token.span.close().start().line;
    end_line.saturating_sub(start_line) + 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rust_metric_bounds_start_at_identity_and_end_at_body() {
        use crate::analyzers::{Analyzer, rust::RustAnalyzer};
        use crate::extraction::{UnifiedFileExtractor, adapters};
        let source = r#"
/// Free function documentation.
#[inline]
pub
fn
free(
    value: u32,
) -> u32 {
    value
} // end free
fn after_free() {}
struct Widget;
impl Widget {
    /// Inherent method documentation.
    #[inline]
    pub fn method(
        &self,
    ) -> u32 {
        1
    } // end method
    fn after_method(&self) {}
}
trait Api { fn operation(&self) -> u32; }
impl Api for Widget {
    /// Trait implementation documentation.
    #[inline]
    fn operation(&self) -> u32 {
        2
    } // end operation
}
fn after_impl() {}
"#;
        let path = std::path::PathBuf::from("src/bounds.rs");
        let analyzer = RustAnalyzer::new();
        let ast = analyzer.parse(source, path.clone()).unwrap();
        let direct = analyzer.analyze(&ast);
        let cached = UnifiedFileExtractor::extract(&path, source).unwrap();
        let cached_metrics = adapters::metrics::all_function_metrics(&cached);
        for metrics in [&direct.complexity.functions, &cached_metrics] {
            for (name, start, end) in [
                ("free", "free(", "} // end free"),
                (
                    "Widget::method",
                    "    pub fn method(",
                    "    } // end method",
                ),
                (
                    "Widget::operation",
                    "    fn operation(&self) -> u32 {",
                    "    } // end operation",
                ),
            ] {
                let metric = metrics.iter().find(|metric| metric.name == name).unwrap();
                let line_of = |text| source.lines().position(|line| line == text).unwrap() + 1;
                assert_eq!(metric.line, line_of(start), "{name}: identity line");
                assert_eq!(
                    metric.line + metric.length - 1,
                    line_of(end),
                    "{name}: body end"
                );
            }
        }
    }

    #[test]
    fn expression_closure_metrics_end_at_the_original_body_span() {
        use crate::analyzers::{Analyzer, rust::RustAnalyzer};
        let source = "fn outer() {\n    let closure = |value: bool|\n        if value {\n            1\n        } else {\n            2\n        };\n    closure(true);\n}\n";
        let analyzer = RustAnalyzer::new();
        let ast = analyzer.parse(source, "src/closure.rs".into()).unwrap();
        let metrics = analyzer.analyze(&ast);
        let closure = metrics
            .complexity
            .functions
            .iter()
            .find(|metric| metric.name.contains("::<closure@"))
            .unwrap();
        assert_eq!(closure.line, 3);
        assert_eq!(closure.line + closure.length - 1, 7);
    }

    #[test]
    fn trait_default_body_length_excludes_attributes_and_multiline_prefix() {
        let source = "trait T {\n/// Default body.\n#[inline]\nfn\nprovided(\n &self,\n) {\n}\n}\n";
        let ast = syn::parse_file(source).unwrap();
        let syn::Item::Trait(item) = &ast.items[0] else {
            panic!("trait fixture")
        };
        let syn::TraitItem::Fn(method) = &item.items[0] else {
            panic!("method fixture")
        };
        let function = syn::ItemFn {
            attrs: method.attrs.clone(),
            vis: syn::Visibility::Inherited,
            sig: method.sig.clone(),
            block: Box::new(method.default.clone().unwrap()),
        };
        assert_eq!(count_function_lines(&function), 4);
    }

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
