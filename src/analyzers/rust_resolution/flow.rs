//! Side-effect discovery for conservative loop-carried facts, respecting local shadows.
use super::index::{Context, WorkspaceIndex};
use std::collections::HashSet;
use syn::{
    Expr, Pat,
    visit::{self, Visit},
};

pub(super) fn loop_writes(
    condition: Option<&Expr>,
    block: &syn::Block,
    index: &WorkspaceIndex,
    context: &Context,
) -> HashSet<String> {
    let mut collector = Writes {
        index,
        context,
        scopes: vec![HashSet::new()],
        names: HashSet::new(),
    };
    if let Some(condition) = condition {
        collector.visit_expr(condition);
    }
    collector.visit_block(block);
    collector.names
}

struct Writes<'a> {
    index: &'a WorkspaceIndex,
    context: &'a Context,
    scopes: Vec<HashSet<String>>,
    names: HashSet<String>,
}

impl Writes<'_> {
    fn bind(&mut self, pat: &Pat) {
        let mut collector = Names::default();
        collector.visit_pat(pat);
        if let Some(scope) = self.scopes.last_mut() {
            scope.extend(collector.0);
        }
    }

    fn assign(&mut self, expr: &Expr) {
        match expr {
            Expr::Path(path) if path.path.get_ident().is_some() => {
                let name = path
                    .path
                    .get_ident()
                    .map(ToString::to_string)
                    .unwrap_or_default();
                if !self.scopes.iter().any(|scope| scope.contains(&name)) {
                    self.names.insert(name);
                }
            }
            Expr::Paren(p) => self.assign(&p.expr),
            Expr::Tuple(tuple) => {
                for expr in &tuple.elems {
                    self.assign(expr);
                }
            }
            _ => {}
        }
    }
}

impl<'ast> Visit<'ast> for Writes<'_> {
    fn visit_item(&mut self, _: &'ast syn::Item) {}
    fn visit_expr_closure(&mut self, _: &'ast syn::ExprClosure) {}

    fn visit_block(&mut self, block: &'ast syn::Block) {
        self.scopes.push(
            super::block_items::block_items(block)
                .into_iter()
                .filter(|item| {
                    item.import
                        .as_ref()
                        .and_then(|path| self.index.path_namespaces(path, self.context))
                        .map(|(_, values)| values)
                        .unwrap_or(item.values)
                })
                .map(|item| item.name)
                .collect(),
        );
        for statement in &block.stmts {
            self.visit_stmt(statement);
        }
        self.scopes.pop();
    }

    fn visit_local(&mut self, local: &'ast syn::Local) {
        if let Some(init) = &local.init {
            self.visit_expr(&init.expr);
        }
        self.bind(&local.pat);
    }

    fn visit_expr_assign(&mut self, assign: &'ast syn::ExprAssign) {
        self.assign(&assign.left);
        visit::visit_expr_assign(self, assign);
    }

    fn visit_expr_let(&mut self, expr: &'ast syn::ExprLet) {
        self.visit_expr(&expr.expr);
        self.bind(&expr.pat);
    }

    fn visit_expr_for_loop(&mut self, expr: &'ast syn::ExprForLoop) {
        self.visit_expr(&expr.expr);
        self.scopes.push(HashSet::new());
        self.bind(&expr.pat);
        self.visit_block(&expr.body);
        self.scopes.pop();
    }

    fn visit_expr_if(&mut self, expr: &'ast syn::ExprIf) {
        self.scopes.push(HashSet::new());
        self.visit_expr(&expr.cond);
        self.visit_block(&expr.then_branch);
        self.scopes.pop();
        if let Some((_, otherwise)) = &expr.else_branch {
            self.visit_expr(otherwise);
        }
    }

    fn visit_expr_while(&mut self, expr: &'ast syn::ExprWhile) {
        self.scopes.push(HashSet::new());
        self.visit_expr(&expr.cond);
        self.visit_block(&expr.body);
        self.scopes.pop();
    }

    fn visit_arm(&mut self, arm: &'ast syn::Arm) {
        self.scopes.push(HashSet::new());
        self.bind(&arm.pat);
        if let Some((_, guard)) = &arm.guard {
            self.visit_expr(guard);
        }
        self.visit_expr(&arm.body);
        self.scopes.pop();
    }
}

#[derive(Default)]
struct Names(HashSet<String>);
impl<'ast> Visit<'ast> for Names {
    fn visit_pat_ident(&mut self, pat: &'ast syn::PatIdent) {
        self.0.insert(pat.ident.to_string());
        visit::visit_pat_ident(self, pat);
    }
}
