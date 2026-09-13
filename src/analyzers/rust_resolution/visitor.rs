//! Body traversal. Initializers run before bindings; branches join outer facts.
use super::body::{Body, unknown};
use syn::{
    Expr,
    visit::{self, Visit},
};

impl<'ast> Visit<'ast> for Body<'_> {
    fn visit_item(&mut self, _: &'ast syn::Item) {} // Indexed bodies have their own caller.

    fn visit_block(&mut self, block: &'ast syn::Block) {
        self.bindings.push();
        for item in super::block_items::block_items(block) {
            let namespaces = item
                .import
                .as_ref()
                .and_then(|path| self.index.path_namespaces(path, &self.callable.context));
            let (types, values) = namespaces.unwrap_or((item.types, item.values));
            self.bindings.shadow_item(item.name, types, values);
        }
        for stmt in &block.stmts {
            self.visit_stmt(stmt);
        }
        self.bindings.pop();
    }

    fn visit_local(&mut self, local: &'ast syn::Local) {
        if let Some(init) = &local.init {
            self.visit_expr(&init.expr);
            if let Some((_, diverge)) = &init.diverge {
                self.visit_expr(diverge);
            }
        }
        let fact = local
            .init
            .as_ref()
            .map(|i| self.infer(&i.expr))
            .unwrap_or_else(unknown);
        self.bind_pattern(&local.pat, fact, false);
    }

    fn visit_expr(&mut self, expr: &'ast Expr) {
        match expr {
            Expr::Call(call) => self.visit_call(call, expr),
            Expr::MethodCall(method) => self.visit_method(method, expr),
            Expr::Assign(assign) => {
                self.visit_expr(&assign.right);
                let value = self.infer(&assign.right);
                self.visit_expr(&assign.left);
                self.assign_target(&assign.left, value);
            }
            Expr::Binary(binary) if matches!(binary.op, syn::BinOp::And(_) | syn::BinOp::Or(_)) => {
                self.visit_short_circuit(binary)
            }
            Expr::If(branch) => self.visit_if(branch),
            Expr::Match(branch) => self.visit_match(branch),
            Expr::ForLoop(loop_expr) => self.visit_for(loop_expr),
            Expr::While(loop_expr) => self.visit_while(loop_expr),
            Expr::Loop(loop_expr) => self.visit_loop_body(&loop_expr.body),
            Expr::Let(binding) => {
                self.visit_expr(&binding.expr);
                let fact = self.infer(&binding.expr);
                self.bind_pattern(&binding.pat, fact, false);
            }
            Expr::Closure(closure) => self.visit_closure(closure),
            Expr::Macro(expr_macro) => {
                let mut expander = crate::analyzers::call_graph::MacroExpander::new();
                for expanded in expander.handle_macro_expression(expr_macro) {
                    self.visit_expr(&expanded);
                }
            }
            Expr::Path(path) => self.visit_reference(path, expr),
            _ => visit::visit_expr(self, expr),
        }
    }
}

impl Body<'_> {
    fn visit_call(&mut self, call: &syn::ExprCall, expr: &Expr) {
        if let Expr::Path(path) = &*call.func {
            let lookup = self.lookup_path(path);
            let query = quote::quote!(#path).to_string();
            self.record(lookup, expr, query, None);
        } else {
            self.visit_expr(&call.func);
        }
        for arg in &call.args {
            self.visit_expr(arg);
        }
    }

    fn visit_method(&mut self, method: &syn::ExprMethodCall, expr: &Expr) {
        self.visit_expr(&method.receiver);
        let receiver = self.infer(&method.receiver);
        let index = self.index;
        let lookup = index.lookup_methods(
            &receiver,
            &method.method.to_string(),
            &self.callable.context,
        );
        self.record(lookup, expr, method.method.to_string(), Some(receiver));
        for arg in &method.args {
            self.visit_expr(arg);
        }
    }

    fn visit_reference(&mut self, path: &syn::ExprPath, expr: &Expr) {
        let lookup = self.lookup_path(path);
        if !lookup.candidates.is_empty() {
            self.record(lookup, expr, quote::quote!(#path).to_string(), None);
        }
    }

    fn assign_target(&mut self, target: &Expr, fact: super::types::TypeFact) {
        match target {
            Expr::Path(path) if path.path.get_ident().is_some() => {
                if let Some(name) = path.path.get_ident() {
                    self.bindings.assign(&name.to_string(), fact);
                }
            }
            Expr::Paren(p) => self.assign_target(&p.expr, fact),
            Expr::Tuple(tuple) => {
                for elem in &tuple.elems {
                    self.assign_target(elem, unknown());
                }
            }
            _ => {}
        }
    }

    fn visit_short_circuit(&mut self, binary: &syn::ExprBinary) {
        self.visit_expr(&binary.left);
        let before = self.bindings.clone();
        self.visit_expr(&binary.right);
        self.bindings = before.join(&[before.clone(), self.bindings.clone()]);
    }

    fn visit_if(&mut self, branch: &syn::ExprIf) {
        self.bindings.push();
        self.visit_expr(&branch.cond);
        let condition = self.bindings.clone();
        self.visit_block(&branch.then_branch);
        self.bindings.pop();
        let left = self.bindings.clone();
        self.bindings = condition;
        self.bindings.pop();
        let before = self.bindings.clone();
        if let Some((_, expr)) = &branch.else_branch {
            self.visit_expr(expr);
        }
        self.bindings = before.join(&[left, self.bindings.clone()]);
    }

    fn visit_match(&mut self, branch: &syn::ExprMatch) {
        self.visit_expr(&branch.expr);
        let fact = self.infer(&branch.expr);
        let before = self.bindings.clone();
        let mut outcomes = Vec::new();
        for arm in &branch.arms {
            let skipped = self.bindings.clone();
            self.bindings.push();
            self.bind_pattern(&arm.pat, fact.clone(), false);
            if let Some((_, guard)) = &arm.guard {
                self.visit_expr(guard);
            }
            let mut guarded = self.bindings.clone();
            guarded.pop();
            self.visit_expr(&arm.body);
            self.bindings.pop();
            outcomes.push(self.bindings.clone());
            self.bindings = skipped.join(&[skipped.clone(), guarded]);
        }
        self.bindings = before.join(&outcomes);
    }

    fn visit_for(&mut self, expr: &syn::ExprForLoop) {
        self.visit_expr(&expr.expr);
        self.bindings.push();
        self.bind_pattern(&expr.pat, unknown(), false);
        self.visit_loop_body(&expr.body);
        self.bindings.pop();
    }

    fn visit_while(&mut self, expr: &syn::ExprWhile) {
        let writes = super::flow::loop_writes(
            Some(&expr.cond),
            &expr.body,
            self.index,
            &self.callable.context,
        );
        self.bindings.invalidate_writes(&writes);
        self.bindings.push();
        self.visit_expr(&expr.cond);
        self.visit_block(&expr.body);
        self.bindings.pop();
        self.bindings.invalidate_writes(&writes);
    }

    fn visit_loop_body(&mut self, block: &syn::Block) {
        let writes = super::flow::loop_writes(None, block, self.index, &self.callable.context);
        self.bindings.invalidate_writes(&writes);
        self.visit_block(block);
        self.bindings.invalidate_writes(&writes);
    }

    fn visit_closure(&mut self, closure: &syn::ExprClosure) {
        let before = self.bindings.clone();
        self.bindings.push();
        for pat in &closure.inputs {
            self.bind_pattern(pat, unknown(), false);
        }
        self.visit_expr(&closure.body);
        self.bindings = before;
    }
}
