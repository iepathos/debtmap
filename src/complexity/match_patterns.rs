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
        // Check if matching against string literals specifically
        match_expr.arms.iter().any(|arm| {
            if let Pat::Lit(pat_lit) = &arm.pat {
                matches!(pat_lit.lit, syn::Lit::Str(_))
            } else {
                false
            }
        })
    }
}

impl PatternRecognizer for MatchExpressionRecognizer {
    fn detect(&self, block: &syn::Block) -> Option<PatternMatchInfo> {
        block.stmts
            .iter()
            .find_map(|stmt| self.analyze_statement(stmt))
    }

    fn adjust_complexity(&self, info: &PatternMatchInfo, _base: u32) -> u32 {
        // Logarithmic scaling for match expressions
        let adjusted = (info.condition_count as f32).log2().ceil() as u32;

        // Small penalty for missing default case
        let default_penalty = if !info.has_default { 1 } else { 0 };

        adjusted + default_penalty
    }
}

impl MatchExpressionRecognizer {
    /// Analyze a single statement for match expression patterns
    fn analyze_statement(&self, stmt: &Stmt) -> Option<PatternMatchInfo> {
        match stmt {
            Stmt::Expr(Expr::Match(match_expr), _) => self.analyze_match_expression(match_expr),
            _ => None,
        }
    }

    /// Analyze a match expression and extract pattern information
    fn analyze_match_expression(&self, match_expr: &ExprMatch) -> Option<PatternMatchInfo> {
        // Check if match qualifies for pattern extraction
        if !self.is_pattern_extractable(match_expr) {
            return None;
        }

        let pattern_type = self.determine_pattern_type(match_expr);

        Some(PatternMatchInfo {
            variable_name: "match_expr".to_string(),
            condition_count: match_expr.arms.len(),
            has_default: self.has_wildcard_arm(match_expr),
            pattern_type,
        })
    }

    /// Check if match expression is suitable for pattern extraction
    fn is_pattern_extractable(&self, match_expr: &ExprMatch) -> bool {
        match_expr.arms.len() >= 3
            && match_expr
                .arms
                .iter()
                .all(|arm| self.is_simple_arm(&arm.body))
    }

    /// Determine the type of pattern matching being used
    fn determine_pattern_type(&self, match_expr: &ExprMatch) -> PatternType {
        if self.detect_enum_matching(match_expr) {
            PatternType::EnumMatching
        } else if self.detect_string_matching(match_expr) {
            PatternType::StringMatching
        } else {
            PatternType::SimpleComparison
        }
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

    #[test]
    fn test_detect_enum_matching_comprehensive() {
        let recognizer = MatchExpressionRecognizer::new();

        // High enum variant ratio
        let match_expr: ExprMatch = parse_quote! {
            match value {
                Enum::Variant1 => 1,
                Enum::Variant2(x) => x,
                Enum::Variant3 { field } => field,
                SomeType::Other => 0,
                _ => -1,
            }
        };

        assert!(recognizer.detect_enum_matching(&match_expr));
    }

    #[test]
    fn test_detect_enum_matching_low_ratio() {
        let recognizer = MatchExpressionRecognizer::new();

        // Low enum variant ratio (only 1 out of 5)
        let match_expr: ExprMatch = parse_quote! {
            match value {
                1 => "one",
                2 => "two",
                3 => "three",
                Enum::Variant => "enum",
                _ => "other",
            }
        };

        assert!(!recognizer.detect_enum_matching(&match_expr));
    }

    #[test]
    fn test_detect_string_matching() {
        let recognizer = MatchExpressionRecognizer::new();

        let match_expr: ExprMatch = parse_quote! {
            match input {
                "hello" => 1,
                "world" => 2,
                Enum::Variant => 3,
                _ => 0,
            }
        };

        assert!(recognizer.detect_string_matching(&match_expr));
    }

    #[test]
    fn test_detect_no_string_matching() {
        let recognizer = MatchExpressionRecognizer::new();

        let match_expr: ExprMatch = parse_quote! {
            match value {
                1 => "one",
                2 => "two",
                _ => "other",
            }
        };

        assert!(!recognizer.detect_string_matching(&match_expr));
    }

    #[test]
    fn test_wildcard_arm_detection() {
        let recognizer = MatchExpressionRecognizer::new();

        // With wildcard
        let match_expr: ExprMatch = parse_quote! {
            match value {
                1 => "one",
                2 => "two",
                _ => "other",
            }
        };

        assert!(recognizer.has_wildcard_arm(&match_expr));

        // Without wildcard
        let match_expr: ExprMatch = parse_quote! {
            match value {
                1 => "one",
                2 => "two",
            }
        };

        assert!(!recognizer.has_wildcard_arm(&match_expr));
    }

    #[test]
    fn test_is_simple_arm_edge_cases() {
        let recognizer = MatchExpressionRecognizer::new();

        // Simple block with single expression
        let expr: Expr = parse_quote!({
            42
        });
        assert!(recognizer.is_simple_arm(&expr));

        // Block with single return statement
        let expr: Expr = parse_quote!({
            return 42;
        });
        assert!(recognizer.is_simple_arm(&expr));

        // Block with statement plus return
        let expr: Expr = parse_quote!({
            let x = 42;
            return x;
        });
        assert!(recognizer.is_simple_arm(&expr));

        // Complex block
        let expr: Expr = parse_quote!({
            let x = 42;
            let y = 43;
            return x + y;
        });
        assert!(!recognizer.is_simple_arm(&expr));

        // Method call
        let expr: Expr = parse_quote!(obj.method());
        assert!(recognizer.is_simple_arm(&expr));

        // Field access
        let expr: Expr = parse_quote!(obj.field);
        assert!(recognizer.is_simple_arm(&expr));

        // Simple constructor
        let expr: Expr = parse_quote!(SomeType::new());
        assert!(recognizer.is_simple_arm(&expr));
    }

    #[test]
    fn test_detect_insufficient_arms() {
        let recognizer = MatchExpressionRecognizer::new();

        // Only 2 arms (below threshold)
        let block: syn::Block = parse_quote! {{
            match value {
                1 => "one",
                _ => "other",
            }
        }};

        let info = recognizer.detect(&block);
        assert!(info.is_none());
    }

    #[test]
    fn test_adjust_complexity_with_default() {
        let recognizer = MatchExpressionRecognizer::new();

        let info = PatternMatchInfo {
            variable_name: "test".to_string(),
            condition_count: 8,
            has_default: true,
            pattern_type: PatternType::EnumMatching,
        };

        let adjusted = recognizer.adjust_complexity(&info, 8);

        // log2(8) = 3, no penalty for default case
        assert_eq!(adjusted, 3);
    }

    #[test]
    fn test_adjust_complexity_without_default() {
        let recognizer = MatchExpressionRecognizer::new();

        let info = PatternMatchInfo {
            variable_name: "test".to_string(),
            condition_count: 8,
            has_default: false,
            pattern_type: PatternType::EnumMatching,
        };

        let adjusted = recognizer.adjust_complexity(&info, 8);

        // log2(8) = 3, plus 1 penalty for missing default
        assert_eq!(adjusted, 4);
    }

    #[test]
    fn test_direct_match_expression_detection() {
        // Test the helper function for direct expression analysis
        let expr: Expr = parse_quote! {
            match status {
                Status::Active => "active",
                Status::Inactive => "inactive",
                Status::Pending => "pending",
            }
        };

        let info = detect_match_expression(&expr);
        assert!(info.is_some());

        let info = info.unwrap();
        assert_eq!(info.condition_count, 3);
        assert_eq!(info.pattern_type, PatternType::EnumMatching);
        assert!(!info.has_default);
    }

    #[test]
    fn test_direct_match_expression_with_complex_arms() {
        // Test that complex arms are not recognized
        let expr: Expr = parse_quote! {
            match value {
                1 => {
                    let x = complex_computation();
                    if x > 0 {
                        x * 2
                    } else {
                        0
                    }
                },
                2 => simple_value(),
                _ => 0,
            }
        };

        let info = detect_match_expression(&expr);
        // Should return None because not all arms are simple
        assert!(info.is_none());
    }
}
