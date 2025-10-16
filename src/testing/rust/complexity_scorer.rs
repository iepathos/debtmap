use syn::spanned::Spanned;
use syn::visit::Visit;
use syn::ItemFn;

/// Scores test complexity using multiple factors
pub struct ComplexityScorer {
    conditionals: u32,
    loops: u32,
    assertions: u32,
    nesting_depth: u32,
    max_nesting: u32,
    line_count: usize,
}

#[derive(Debug, Clone)]
pub struct TestComplexityScore {
    pub total_score: f32,
    pub factors: ComplexityFactors,
    pub maintainability_index: f32,
}

#[derive(Debug, Clone)]
pub struct ComplexityFactors {
    pub conditionals: u32,
    pub loops: u32,
    pub assertions: u32,
    pub nesting_depth: u32,
    pub line_count: usize,
}

impl ComplexityScorer {
    /// Default complexity threshold for tests
    pub const DEFAULT_THRESHOLD: f32 = 10.0;

    pub fn new() -> Self {
        Self {
            conditionals: 0,
            loops: 0,
            assertions: 0,
            nesting_depth: 0,
            max_nesting: 0,
            line_count: 0,
        }
    }

    /// Calculate complexity score for a test function
    pub fn calculate_complexity(
        &mut self,
        func: &ItemFn,
        assertion_count: usize,
    ) -> TestComplexityScore {
        self.reset();
        self.assertions = assertion_count as u32;
        self.line_count = self.count_lines(func);

        // Visit the function body to count complexity factors
        self.visit_block(&func.block);

        let total_score = self.compute_total_score();
        let maintainability_index = self.compute_maintainability_index(total_score);

        TestComplexityScore {
            total_score,
            factors: ComplexityFactors {
                conditionals: self.conditionals,
                loops: self.loops,
                assertions: self.assertions,
                nesting_depth: self.max_nesting,
                line_count: self.line_count,
            },
            maintainability_index,
        }
    }

    /// Reset all counters
    fn reset(&mut self) {
        self.conditionals = 0;
        self.loops = 0;
        self.assertions = 0;
        self.nesting_depth = 0;
        self.max_nesting = 0;
        self.line_count = 0;
    }

    /// Compute total complexity score based on all factors
    fn compute_total_score(&self) -> f32 {
        let mut score = 0.0;

        // Conditionals: +2 per if/match
        score += self.conditionals as f32 * 2.0;

        // Loops: +3 per loop
        score += self.loops as f32 * 3.0;

        // Assertions: +1 per assertion beyond 5
        if self.assertions > 5 {
            score += (self.assertions - 5) as f32;
        }

        // Nesting depth: +2 per level > 2
        if self.max_nesting > 2 {
            score += (self.max_nesting - 2) as f32 * 2.0;
        }

        // Line count: +(lines-30)/10 for tests > 30 lines
        if self.line_count > 30 {
            score += ((self.line_count - 30) as f32) / 10.0;
        }

        score
    }

    /// Compute maintainability index (inverse of complexity)
    fn compute_maintainability_index(&self, total_score: f32) -> f32 {
        100.0 - (total_score * 2.0).min(100.0)
    }

    /// Count lines in function
    fn count_lines(&self, func: &ItemFn) -> usize {
        let span = func.span();
        let start_line = span.start().line;
        let end_line = span.end().line;

        if end_line >= start_line {
            end_line - start_line + 1
        } else {
            1
        }
    }

    /// Track nesting depth
    fn enter_nested(&mut self) {
        self.nesting_depth += 1;
        if self.nesting_depth > self.max_nesting {
            self.max_nesting = self.nesting_depth;
        }
    }

    fn exit_nested(&mut self) {
        if self.nesting_depth > 0 {
            self.nesting_depth -= 1;
        }
    }
}

impl Default for ComplexityScorer {
    fn default() -> Self {
        Self::new()
    }
}

impl<'ast> Visit<'ast> for ComplexityScorer {
    fn visit_expr_if(&mut self, expr: &'ast syn::ExprIf) {
        self.conditionals += 1;
        self.enter_nested();
        syn::visit::visit_expr_if(self, expr);
        self.exit_nested();
    }

    fn visit_expr_match(&mut self, expr: &'ast syn::ExprMatch) {
        self.conditionals += 1;
        self.enter_nested();
        syn::visit::visit_expr_match(self, expr);
        self.exit_nested();
    }

    fn visit_expr_while(&mut self, expr: &'ast syn::ExprWhile) {
        self.loops += 1;
        self.enter_nested();
        syn::visit::visit_expr_while(self, expr);
        self.exit_nested();
    }

    fn visit_expr_for_loop(&mut self, expr: &'ast syn::ExprForLoop) {
        self.loops += 1;
        self.enter_nested();
        syn::visit::visit_expr_for_loop(self, expr);
        self.exit_nested();
    }

    fn visit_expr_loop(&mut self, expr: &'ast syn::ExprLoop) {
        self.loops += 1;
        self.enter_nested();
        syn::visit::visit_expr_loop(self, expr);
        self.exit_nested();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn test_simple_test_low_complexity() {
        let func: ItemFn = parse_quote! {
            #[test]
            fn test_simple() {
                let x = 42;
                assert_eq!(x, 42);
            }
        };

        let mut scorer = ComplexityScorer::new();
        let score = scorer.calculate_complexity(&func, 1);
        assert!(score.total_score < 5.0);
    }

    #[test]
    fn test_conditional_increases_complexity() {
        let func: ItemFn = parse_quote! {
            #[test]
            fn test_conditional() {
                if true {
                    assert!(true);
                }
            }
        };

        let mut scorer = ComplexityScorer::new();
        let score = scorer.calculate_complexity(&func, 1);
        assert_eq!(score.factors.conditionals, 1);
        assert!(score.total_score >= 2.0);
    }

    #[test]
    fn test_loop_increases_complexity() {
        let func: ItemFn = parse_quote! {
            #[test]
            fn test_loop() {
                for i in 0..10 {
                    assert!(i < 10);
                }
            }
        };

        let mut scorer = ComplexityScorer::new();
        let score = scorer.calculate_complexity(&func, 1);
        assert_eq!(score.factors.loops, 1);
        assert!(score.total_score >= 3.0);
    }

    #[test]
    fn test_excessive_assertions() {
        let func: ItemFn = parse_quote! {
            #[test]
            fn test_many_assertions() {
                assert!(true);
                assert!(true);
                assert!(true);
                assert!(true);
                assert!(true);
                assert!(true);
                assert!(true);
            }
        };

        let mut scorer = ComplexityScorer::new();
        let score = scorer.calculate_complexity(&func, 7);
        assert_eq!(score.factors.assertions, 7);
        // Should add 2 points for assertions beyond 5
        assert!(score.total_score >= 2.0);
    }

    #[test]
    fn test_nested_complexity() {
        let func: ItemFn = parse_quote! {
            #[test]
            fn test_nested() {
                if true {
                    for i in 0..10 {
                        if i % 2 == 0 {
                            assert!(true);
                        }
                    }
                }
            }
        };

        let mut scorer = ComplexityScorer::new();
        let score = scorer.calculate_complexity(&func, 1);
        assert!(score.factors.nesting_depth >= 3);
        assert!(score.total_score > 5.0);
    }

    #[test]
    fn test_maintainability_index() {
        let func: ItemFn = parse_quote! {
            #[test]
            fn test_simple() {
                assert!(true);
            }
        };

        let mut scorer = ComplexityScorer::new();
        let score = scorer.calculate_complexity(&func, 1);
        // Simple tests should have high maintainability
        assert!(score.maintainability_index > 90.0);
    }

    #[test]
    fn test_complex_test_low_maintainability() {
        let func: ItemFn = parse_quote! {
            #[test]
            fn test_complex() {
                for i in 0..10 {
                    if i % 2 == 0 {
                        for j in 0..5 {
                            if j > i {
                                assert!(true);
                            }
                        }
                    }
                }
            }
        };

        let mut scorer = ComplexityScorer::new();
        let score = scorer.calculate_complexity(&func, 1);
        // Complex tests should have low maintainability
        assert!(score.maintainability_index < 80.0);
    }
}
