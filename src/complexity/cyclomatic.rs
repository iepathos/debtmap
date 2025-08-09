use syn::{visit::Visit, Block, Expr, Stmt};

pub fn calculate_cyclomatic(block: &Block) -> u32 {
    let mut visitor = CyclomaticVisitor { complexity: 1 };
    visitor.visit_block(block);
    visitor.complexity
}

struct CyclomaticVisitor {
    complexity: u32,
}

impl<'ast> Visit<'ast> for CyclomaticVisitor {
    fn visit_expr(&mut self, expr: &'ast Expr) {
        match expr {
            Expr::If(_) => self.complexity += 1,
            Expr::Match(expr_match) => {
                self.complexity += expr_match.arms.len() as u32;
            }
            Expr::While(_) | Expr::ForLoop(_) | Expr::Loop(_) => {
                self.complexity += 1;
            }
            Expr::Binary(binary) if is_logical_operator(&binary.op) => {
                self.complexity += 1;
            }
            Expr::Try(_) => self.complexity += 1,
            _ => {}
        }
        syn::visit::visit_expr(self, expr);
    }

    fn visit_stmt(&mut self, stmt: &'ast Stmt) {
        if let Stmt::Expr(Expr::If(_), _) = stmt {
            return;
        }
        syn::visit::visit_stmt(self, stmt);
    }
}

fn is_logical_operator(op: &syn::BinOp) -> bool {
    matches!(op, syn::BinOp::And(_) | syn::BinOp::Or(_))
}

pub fn calculate_cyclomatic_for_function(complexity: u32, params: usize) -> u32 {
    complexity + params.saturating_sub(1) as u32
}

pub fn combine_cyclomatic(branches: Vec<u32>) -> u32 {
    branches.iter().sum::<u32>() + 1
}
