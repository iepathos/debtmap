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
        match expr {
            Expr::If(_) => {
                self.complexity += 1 + self.nesting_level;
                self.nesting_level += 1;
                syn::visit::visit_expr(self, expr);
                self.nesting_level -= 1;
            }
            Expr::Match(expr_match) => {
                self.complexity += 1 + self.nesting_level;
                self.nesting_level += 1;
                for _arm in &expr_match.arms {
                    self.complexity += 1;
                }
                syn::visit::visit_expr(self, expr);
                self.nesting_level -= 1;
            }
            Expr::While(_) | Expr::ForLoop(_) | Expr::Loop(_) => {
                self.complexity += 1 + self.nesting_level;
                self.nesting_level += 1;
                syn::visit::visit_expr(self, expr);
                self.nesting_level -= 1;
            }
            Expr::Binary(binary) if is_logical_operator(&binary.op) => {
                self.complexity += 1;
                syn::visit::visit_expr(self, expr);
            }
            Expr::Try(_) => {
                self.complexity += 1 + self.nesting_level;
                self.nesting_level += 1;
                syn::visit::visit_expr(self, expr);
                self.nesting_level -= 1;
            }
            Expr::Closure(_) => {
                self.complexity += 1;
                syn::visit::visit_expr(self, expr);
            }
            _ => syn::visit::visit_expr(self, expr),
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
