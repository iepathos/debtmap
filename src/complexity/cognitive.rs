use syn::{visit::Visit, Block, Expr};

pub fn calculate_cognitive(block: &Block) -> u32 {
    let mut visitor = CognitiveVisitor {
        complexity: 0,
        nesting_level: 0,
    };
    visitor.visit_block(block);
    visitor.complexity
}

struct CognitiveVisitor {
    complexity: u32,
    nesting_level: u32,
}

impl<'ast> Visit<'ast> for CognitiveVisitor {
    fn visit_expr(&mut self, expr: &'ast Expr) {
        let (base_complexity, extra_complexity, increases_nesting) = match expr {
            Expr::If(_) | Expr::While(_) | Expr::ForLoop(_) | Expr::Loop(_) | Expr::Try(_) => {
                (1 + self.nesting_level, 0, true)
            }
            Expr::Match(expr_match) => (1 + self.nesting_level, expr_match.arms.len() as u32, true),
            Expr::Binary(binary) if is_logical_operator(&binary.op) => (1, 0, false),
            Expr::Closure(_) => (1, 0, false),
            _ => (0, 0, false),
        };

        self.complexity += base_complexity + extra_complexity;

        if increases_nesting {
            self.nesting_level += 1;
            syn::visit::visit_expr(self, expr);
            self.nesting_level -= 1;
        } else {
            syn::visit::visit_expr(self, expr);
        }
    }
}

fn is_logical_operator(op: &syn::BinOp) -> bool {
    matches!(op, syn::BinOp::And(_) | syn::BinOp::Or(_))
}

pub fn calculate_cognitive_penalty(nesting: u32) -> u32 {
    match nesting {
        0 => 0,
        1 => 1,
        2 => 2,
        3 => 4,
        _ => 8,
    }
}

pub fn combine_cognitive(complexities: Vec<u32>) -> u32 {
    complexities.iter().sum()
}
