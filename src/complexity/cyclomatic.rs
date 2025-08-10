use syn::{visit::Visit, Block, Expr, Stmt};

pub fn calculate_cyclomatic(block: &Block) -> u32 {
    let mut visitor = CyclomaticVisitor {
        complexity: 1,
        in_condition: false,
    };
    visitor.visit_block(block);
    visitor.complexity
}

struct CyclomaticVisitor {
    complexity: u32,
    in_condition: bool,
}

fn calculate_expr_complexity(expr: &Expr, in_condition: bool) -> u32 {
    match expr {
        Expr::If(expr_if) => {
            let mut count = 1;
            if expr_if.else_branch.is_some() {
                count += 1;
            }
            count
        }
        Expr::While(_) | Expr::ForLoop(_) | Expr::Loop(_) => 1,
        Expr::Try(_) => 1,
        Expr::Match(expr_match) => expr_match.arms.len().saturating_sub(1) as u32,
        Expr::Binary(binary) if is_logical_operator(&binary.op) && !in_condition => 1,
        _ => 0,
    }
}

impl<'ast> Visit<'ast> for CyclomaticVisitor {
    fn visit_expr(&mut self, expr: &'ast Expr) {
        let complexity_delta = calculate_expr_complexity(expr, self.in_condition);
        self.complexity += complexity_delta;

        let was_in_condition = self.in_condition;
        if matches!(expr, Expr::If(_) | Expr::While(_)) {
            self.in_condition = true;
        }

        syn::visit::visit_expr(self, expr);

        self.in_condition = was_in_condition;
    }

    fn visit_stmt(&mut self, stmt: &'ast Stmt) {
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
