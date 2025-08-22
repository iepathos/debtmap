use super::pattern_adjustments::{PatternMatchInfo, PatternRecognizer, PatternType};
use syn::{Expr, ExprMatch, Pat, Stmt};

/// Recognizes match expression patterns
pub struct MatchExpressionRecognizer;

impl Default for MatchExpressionRecognizer {
    fn default() -> Self {
        Self
    }
}

impl MatchExpressionRecognizer {
    pub fn new() -> Self {
        Self
    }

    /// Check if a match arm is simple (return, break, single expression)
    #[allow(clippy::only_used_in_recursion)]
    pub fn is_simple_arm(&self, body: &Expr) -> bool {
        match body {
            // Direct return, break, continue
            Expr::Return(_) | Expr::Break(_) | Expr::Continue(_) => true,
            // Single literal or path
            Expr::Lit(_) | Expr::Path(_) => true,
            // Simple method call or field access
            Expr::MethodCall(_) | Expr::Field(_) => true,
            // Simple constructor call
            Expr::Call(call) => {
                // Check if it's a simple enum variant or struct constructor
                matches!(&*call.func, Expr::Path(_))
            }
            // Block with single return or expression
            Expr::Block(block) => {
                let block = &block.block;
                if block.stmts.len() == 1 {
                    match &block.stmts[0] {
                        Stmt::Expr(expr, _) => self.is_simple_arm(expr),
                        _ => false,
                    }
                } else if block.stmts.len() == 2 {
                    // Allow one statement plus return
                    matches!(&block.stmts[1], Stmt::Expr(Expr::Return(_), _))
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    /// Check if a match has a wildcard/default arm
    fn has_wildcard_arm(&self, match_expr: &ExprMatch) -> bool {
        match_expr
            .arms
            .iter()
            .any(|arm| matches!(&arm.pat, Pat::Wild(_) | Pat::Ident(_)))
    }

    /// Detect if this is an enum matching pattern
    fn detect_enum_matching(&self, match_expr: &ExprMatch) -> bool {
        // Check if most arms are matching against enum variants
        let variant_count = match_expr
            .arms
            .iter()
            .filter(|arm| {
                matches!(
                    &arm.pat,
                    Pat::Path(_) | Pat::TupleStruct(_) | Pat::Struct(_)
                )
            })
            .count();

        variant_count as f32 / match_expr.arms.len() as f32 > 0.5
    }

    /// Detect if this is string matching pattern  
    fn detect_string_matching(&self, match_expr: &ExprMatch) -> bool {
        // Check if matching against string literals
        match_expr
            .arms
            .iter()
            .any(|arm| matches!(&arm.pat, Pat::Lit(_)))
    }
}

impl PatternRecognizer for MatchExpressionRecognizer {
    fn detect(&self, block: &syn::Block) -> Option<PatternMatchInfo> {
        // Look for match expressions in the block
        for stmt in &block.stmts {
            if let Stmt::Expr(Expr::Match(match_expr), _) = stmt {
                // Check if all arms are simple
                let simple_arms = match_expr
                    .arms
                    .iter()
                    .all(|arm| self.is_simple_arm(&arm.body));

                if simple_arms && match_expr.arms.len() >= 3 {
                    // Determine pattern type
                    let pattern_type = if self.detect_enum_matching(match_expr) {
                        PatternType::EnumMatching
                    } else if self.detect_string_matching(match_expr) {
                        PatternType::StringMatching
                    } else {
                        PatternType::SimpleComparison
                    };

                    return Some(PatternMatchInfo {
                        variable_name: "match_expr".to_string(),
                        condition_count: match_expr.arms.len(),
                        has_default: self.has_wildcard_arm(match_expr),
                        pattern_type,
                    });
                }
            }
        }

        None
    }

    fn adjust_complexity(&self, info: &PatternMatchInfo, _base: u32) -> u32 {
        // Logarithmic scaling for match expressions
        let adjusted = (info.condition_count as f32).log2().ceil() as u32;

        // Small penalty for missing default case
        let default_penalty = if !info.has_default { 1 } else { 0 };

        adjusted + default_penalty
    }
}

/// Helper function to detect match expressions directly
pub fn detect_match_expression(expr: &Expr) -> Option<PatternMatchInfo> {
    if let Expr::Match(match_expr) = expr {
        let recognizer = MatchExpressionRecognizer::new();

        // Check if all arms are simple
        let simple_arms = match_expr
            .arms
            .iter()
            .all(|arm| recognizer.is_simple_arm(&arm.body));

        if simple_arms && match_expr.arms.len() >= 2 {
            // Determine pattern type
            let pattern_type = if recognizer.detect_enum_matching(match_expr) {
                PatternType::EnumMatching
            } else if recognizer.detect_string_matching(match_expr) {
                PatternType::StringMatching
            } else {
                PatternType::SimpleComparison
            };

            return Some(PatternMatchInfo {
                variable_name: "match_expr".to_string(),
                condition_count: match_expr.arms.len(),
                has_default: recognizer.has_wildcard_arm(match_expr),
                pattern_type,
            });
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn test_match_expression_detection() {
        let block: syn::Block = parse_quote! {{
            match file_type {
                FileType::Rust => "rust",
                FileType::Python => "python",
                FileType::JavaScript => "javascript",
                FileType::TypeScript => "typescript",
                _ => "unknown",
            }
        }};

        let recognizer = MatchExpressionRecognizer::new();
        let info = recognizer.detect(&block);

        assert!(info.is_some());
        let info = info.unwrap();
        assert_eq!(info.condition_count, 5);
        assert_eq!(info.pattern_type, PatternType::EnumMatching);
        assert!(info.has_default);
    }

    #[test]
    fn test_logarithmic_scaling_for_match() {
        let info = PatternMatchInfo {
            variable_name: "test".to_string(),
            condition_count: 16,
            has_default: true,
            pattern_type: PatternType::EnumMatching,
        };

        let recognizer = MatchExpressionRecognizer::new();
        let adjusted = recognizer.adjust_complexity(&info, 16);

        // log2(16) = 4, so adjusted should be 4
        assert_eq!(adjusted, 4);
    }

    #[test]
    fn test_simple_arm_detection() {
        let recognizer = MatchExpressionRecognizer::new();

        // Simple return
        let expr: Expr = parse_quote!(return 42);
        assert!(recognizer.is_simple_arm(&expr));

        // Simple literal
        let expr: Expr = parse_quote!(42);
        assert!(recognizer.is_simple_arm(&expr));

        // Simple path
        let expr: Expr = parse_quote!(FileType::Rust);
        assert!(recognizer.is_simple_arm(&expr));

        // Complex expression
        let expr: Expr = parse_quote!(if x > 0 { foo() } else { bar() });
        assert!(!recognizer.is_simple_arm(&expr));
    }
}
