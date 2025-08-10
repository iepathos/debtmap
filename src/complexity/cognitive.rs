use super::patterns::{analyze_patterns, PatternComplexity};
use syn::{visit::Visit, Block, Expr};

pub fn calculate_cognitive(block: &Block) -> u32 {
    let mut visitor = CognitiveVisitor {
        complexity: 0,
        nesting_level: 0,
    };
    visitor.visit_block(block);

    // Add pattern-based complexity
    let patterns = analyze_patterns(block);
    visitor.complexity + patterns.total_complexity()
}

pub fn calculate_cognitive_with_patterns(block: &Block) -> (u32, PatternComplexity) {
    let mut visitor = CognitiveVisitor {
        complexity: 0,
        nesting_level: 0,
    };
    visitor.visit_block(block);

    let patterns = analyze_patterns(block);
    let total = visitor.complexity + patterns.total_complexity();
    (total, patterns)
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

impl CognitiveVisitor {
    fn calculate_expr_metrics(&self, expr: &Expr) -> ExprMetrics {
        match expr {
            Expr::If(_) | Expr::While(_) | Expr::ForLoop(_) | Expr::Loop(_) | Expr::Try(_) => {
                ExprMetrics {
                    base_complexity: 1 + self.nesting_level,
                    extra_complexity: 0,
                    increases_nesting: true,
                }
            }
            Expr::Match(expr_match) => ExprMetrics {
                base_complexity: 1 + self.nesting_level,
                extra_complexity: expr_match.arms.len() as u32,
                increases_nesting: true,
            },
            Expr::Binary(binary) if is_logical_operator(&binary.op) => ExprMetrics {
                base_complexity: 1,
                extra_complexity: 0,
                increases_nesting: false,
            },
            Expr::Closure(closure) => {
                // Closures add more complexity if they're async or nested
                let base = if closure.asyncness.is_some() { 2 } else { 1 };
                ExprMetrics {
                    base_complexity: base + self.nesting_level.min(1),
                    extra_complexity: 0,
                    increases_nesting: false,
                }
            }
            Expr::Await(_) => ExprMetrics {
                base_complexity: 1,
                extra_complexity: 0,
                increases_nesting: false,
            },
            Expr::Unsafe(_) => ExprMetrics {
                base_complexity: 2,
                extra_complexity: 0,
                increases_nesting: true,
            },
            _ => ExprMetrics {
                base_complexity: 0,
                extra_complexity: 0,
                increases_nesting: false,
            },
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
