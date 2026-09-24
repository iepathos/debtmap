//! Body traversal. Initializers run before bindings; branches join outer facts.
use super::body::{Body, unknown};
use super::types::TypeFact;
use crate::analysis::effect_evidence::ObservedEffectKind;
use syn::{
    Expr,
    spanned::Spanned,
    visit::{self, Visit},
};

impl<'ast> Visit<'ast> for Body<'_> {
    fn visit_item(&mut self, _: &'ast syn::Item) {} // Indexed bodies have their own caller.

    fn visit_stmt(&mut self, stmt: &'ast syn::Stmt) {
        if let syn::Stmt::Macro(statement) = stmt {
            let expression = Expr::Macro(syn::ExprMacro {
                attrs: statement.attrs.clone(),
                mac: statement.mac.clone(),
            });
            self.visit_expr(&expression);
        } else {
            visit::visit_stmt(self, stmt);
        }
    }

    fn visit_block(&mut self, block: &'ast syn::Block) {
        self.bindings.push();
        for statement in &block.stmts {
            if let syn::Stmt::Item(syn::Item::Macro(item)) = statement
                && let Some(name) = &item.ident
            {
                self.bindings
                    .insert(format!("macro:{name}"), unknown(), false);
            }
        }
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
        let callable = local
            .init
            .as_ref()
            .and_then(|init| callable_path_expression(&init.expr))
            .and_then(|path| {
                let (lookup, _) = self.lookup_invocation(path);
                (lookup.justified && lookup.candidates.len() == 1)
                    .then(|| lookup.candidates[0].id.clone())
            });
        self.record_owned_drop(&fact, local.pat.span());
        let closure = local
            .init
            .as_ref()
            .and_then(|init| closure_expression(&init.expr))
            .map(|closure| self.capture_closure(closure));
        self.bind_pattern(&local.pat, fact, false);
        if let (syn::Pat::Ident(pattern), Some(target)) = (&local.pat, callable)
            && pattern.mutability.is_none()
            && pattern.by_ref.is_none()
        {
            self.bindings
                .bind_callable(&pattern.ident.to_string(), target);
        }
        if let (syn::Pat::Ident(pattern), Some(closure)) = (&local.pat, closure)
            && pattern.mutability.is_none()
            && pattern.by_ref.is_none()
        {
            self.bindings
                .bind_closure(&pattern.ident.to_string(), closure);
        }
    }

    fn visit_expr(&mut self, expr: &'ast Expr) {
        match expr {
            Expr::Call(call) => self.visit_call(call, expr),
            Expr::MethodCall(method) => self.visit_method(method, expr),
            Expr::Assign(assign) => {
                self.visit_expr(&assign.right);
                let value = self.infer(&assign.right);
                self.visit_expr(&assign.left);
                self.record_assignment_effect(expr, &assign.left);
                self.assign_target(&assign.left, value);
            }
            Expr::Binary(binary) if matches!(binary.op, syn::BinOp::And(_) | syn::BinOp::Or(_)) => {
                self.visit_short_circuit(expr)
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
            Expr::Async(async_expr) => self.visit_lazy_block(&async_expr.block),
            Expr::Field(field) => {
                self.visit_expr(&field.base);
                if !self.infer(expr).is_known() {
                    self.record_unsupported_syntax(
                        expr,
                        "unresolved field access may invoke custom Deref",
                    );
                }
            }
            Expr::Binary(binary) => self.visit_binary_effect(binary, expr),
            Expr::Unary(unary) => {
                self.visit_expr(&unary.expr);
                if matches!(unary.op, syn::UnOp::Deref(_)) || !self.primitive_operand(&unary.expr) {
                    self.record_unsupported_syntax(
                        expr,
                        "unmodeled unary operation or dereference",
                    );
                }
            }
            Expr::Index(_) | Expr::Await(_) | Expr::Try(_) | Expr::Verbatim(_) => {
                self.record_unsupported_syntax(expr, "unmodeled implicit operation");
                visit::visit_expr(self, expr);
            }
            Expr::Macro(expr_macro) => {
                let name = expr_macro
                    .mac
                    .path
                    .segments
                    .last()
                    .map(|segment| segment.ident.to_string())
                    .unwrap_or_default();
                let provenance = self.provenance(expr, format!("macro:{name}"));
                let canonical = self
                    .index
                    .standard_macro(&expr_macro.mac.path, &self.callable.context);
                let local_shadow = self.bindings.value_shadowed(&format!("macro:{name}"));
                let modeled = canonical
                    .filter(|_| !local_shadow)
                    .and_then(|canonical| super::models::console_macro(&canonical, provenance));
                let is_modeled = modeled.is_some();
                if let Some(evidence) = modeled {
                    self.join_assessment(evidence);
                }
                let mut expander = crate::analyzers::call_graph::MacroExpander::new();
                let expanded = expander.handle_macro_expression(expr_macro);
                if !is_modeled {
                    self.record_unsupported_syntax(expr, &format!("unexpanded macro {name}!"));
                }
                for expanded in expanded {
                    if is_modeled {
                        self.visit_expr(&expanded);
                    } else {
                        self.visit_lazy_expression(&expanded);
                    }
                }
            }
            // Taking a callable value is not an invocation. Callable references
            // remain available to dedicated reachability analysis.
            Expr::Path(path) => {
                if !self.path_shadowed(&path.path)
                    && self
                        .index
                        .has_static_candidate(&path.path, &self.callable.context)
                {
                    self.record_unsupported_syntax(
                        expr,
                        "static value access requires external-state analysis",
                    );
                }
                let lookup = self.lookup_invocation(path).0;
                self.record_reference(lookup, expr);
            }
            _ => visit::visit_expr(self, expr),
        }
    }
}

impl Body<'_> {
    fn visit_call(&mut self, call: &syn::ExprCall, expr: &Expr) {
        if let Some(path) = callable_path_expression(&call.func) {
            self.visit_path_call(path, call, expr);
        } else if let Some(closure) = closure_expression(&call.func) {
            self.visit_invoked_closure(closure);
        } else {
            self.visit_expr(&call.func);
            self.record_indirect_invocation(expr);
        }
        for arg in &call.args {
            self.visit_expr(arg);
        }
    }

    fn visit_path_call(&mut self, path: &syn::ExprPath, call: &syn::ExprCall, expr: &Expr) {
        if self.invoke_bound_closure(path, expr) {
            return;
        }
        let (lookup, constructor_only) = self.lookup_invocation(path);
        if constructor_only {
            return;
        }
        let query = quote::quote!(#path).to_string();
        if lookup.justified || !lookup.candidates.is_empty() {
            if lookup.justified
                && lookup.candidates.len() == 1
                && lookup.candidates[0].signature.asyncness
            {
                self.record_reference(lookup, expr);
            } else {
                self.visit_project_callback(&lookup, call.args.iter(), 0);
                self.record(lookup, expr, query, None);
            }
        } else if self.path_shadowed(&path.path) {
            self.record_indirect_invocation(expr);
        } else {
            self.visit_external_function(path, lookup, expr, &call.args);
        }
    }

    fn visit_external_function(
        &mut self,
        path: &syn::ExprPath,
        lookup: super::index::Lookup<'_>,
        expr: &Expr,
        arguments: &syn::punctuated::Punctuated<Expr, syn::Token![,]>,
    ) {
        let external = self.index.external_path(&path.path, &self.callable.context);
        let provenance = self.provenance(expr, external.join("::"));
        if let Some(evidence) = super::models::function(&external, provenance) {
            self.join_assessment(evidence);
            if arguments
                .iter()
                .any(|argument| !super::models::supported_argument(&self.infer(argument)))
            {
                self.record_unsupported_syntax(
                    expr,
                    "modeled standard-library argument conversion may dispatch user code",
                );
            }
        } else {
            self.record(lookup, expr, quote::quote!(#path).to_string(), None);
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
        let method_name = method.method.to_string();
        if lookup.justified || !lookup.candidates.is_empty() {
            if lookup.justified
                && lookup.candidates.len() == 1
                && lookup.candidates[0].signature.asyncness
            {
                self.record_reference(lookup, expr);
            } else {
                self.visit_project_callback(&lookup, method.args.iter(), 1);
                self.record(lookup, expr, method_name, Some(receiver));
            }
        } else {
            let provenance = self.provenance(expr, method_name.clone());
            let callback_resolved = super::models::executes_callback(&receiver, &method_name)
                && method
                    .args
                    .first()
                    .is_some_and(|argument| self.record_callback_invocation(argument));
            if let Some(evidence) =
                super::models::method(&receiver, &method_name, callback_resolved, provenance)
            {
                self.join_assessment(evidence);
            } else {
                self.record(lookup, expr, method_name, Some(receiver));
            }
        }
        for arg in &method.args {
            self.visit_expr(arg);
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

    fn record_assignment_effect(&mut self, expr: &Expr, target: &Expr) {
        match target {
            Expr::Path(path)
                if path
                    .path
                    .get_ident()
                    .is_some_and(|name| self.bindings.get(&name.to_string()).is_some()) =>
            {
                self.record_observed_effect(
                    expr,
                    ObservedEffectKind::LocalMutation,
                    format!("assignment to {}", quote::quote!(#path)),
                )
            }
            Expr::Field(field) if matches!(&*field.base, Expr::Path(path) if path.path.is_ident("self")) => {
                self.record_observed_effect(
                    expr,
                    if matches!(self.bindings.get("self"), Some(TypeFact::Reference { .. })) {
                        ObservedEffectKind::ExternalWrite
                    } else {
                        ObservedEffectKind::LocalMutation
                    },
                    format!("assignment to {}", quote::quote!(#field)),
                )
            }
            Expr::Paren(paren) => self.record_assignment_effect(expr, &paren.expr),
            Expr::Tuple(tuple) => {
                for target in &tuple.elems {
                    self.record_assignment_effect(expr, target);
                }
            }
            _ => self.record_unsupported_syntax(expr, "assignment target ownership is unresolved"),
        }
    }

    fn visit_for(&mut self, expr: &syn::ExprForLoop) {
        self.record_unsupported_syntax(&Expr::ForLoop(expr.clone()), "unmodeled iterator dispatch");
        self.visit_expr(&expr.expr);
        self.bindings.push();
        self.bind_pattern(&expr.pat, unknown(), false);
        self.visit_loop_body(&expr.body);
        self.bindings.pop();
    }

    fn visit_loop_body(&mut self, block: &syn::Block) {
        let writes = super::flow::loop_writes(None, block, self.index, &self.callable.context);
        self.bindings.invalidate_writes(&writes);
        self.visit_block(block);
        self.bindings.invalidate_writes(&writes);
    }

    fn visit_closure(&mut self, closure: &syn::ExprClosure) {
        // Preserve reachability without merging construction-time body effects.
        let before_bindings = self.bindings.clone();
        let before_assessment = self.assessment.clone();
        let before_mode = self.reachability_only;
        self.reachability_only = true;
        self.bindings.push();
        for pat in &closure.inputs {
            self.bind_pattern(pat, unknown(), false);
        }
        self.visit_expr(&closure.body);
        self.bindings = before_bindings;
        self.assessment = before_assessment;
        self.reachability_only = before_mode;
    }

    fn visit_lazy_expression(&mut self, expr: &Expr) {
        let assessment = self.assessment.clone();
        let bindings = self.bindings.clone();
        let previous = self.reachability_only;
        self.reachability_only = true;
        self.visit_expr(expr);
        self.reachability_only = previous;
        self.assessment = assessment;
        self.bindings = bindings;
    }

    fn visit_lazy_block(&mut self, block: &syn::Block) {
        self.visit_lazy_expression(&Expr::Block(syn::ExprBlock {
            attrs: Vec::new(),
            label: None,
            block: block.clone(),
        }));
    }

    fn primitive_operand(&self, expr: &Expr) -> bool {
        matches!(self.infer(expr), TypeFact::Primitive(_))
            || matches!(expr, Expr::Lit(literal) if matches!(literal.lit, syn::Lit::Int(_) | syn::Lit::Float(_) | syn::Lit::Bool(_) | syn::Lit::Char(_) | syn::Lit::Byte(_)))
    }

    fn visit_binary_effect(&mut self, binary: &syn::ExprBinary, expr: &Expr) {
        self.visit_expr(&binary.left);
        self.visit_expr(&binary.right);
        if !self.primitive_operand(&binary.left) || !self.primitive_operand(&binary.right) {
            self.record_unsupported_syntax(expr, "unmodeled operator dispatch");
        }
        if is_assignment_operator(&binary.op) {
            self.record_assignment_effect(expr, &binary.left);
        }
    }

    pub(super) fn visit_invoked_closure(&mut self, closure: &syn::ExprClosure) {
        if closure.asyncness.is_some() {
            self.visit_closure(closure);
            return;
        }
        let before = self.bindings.clone();
        self.bindings.push();
        for pat in &closure.inputs {
            self.bind_pattern(pat, unknown(), false);
        }
        self.visit_expr(&closure.body);
        self.bindings = before;
    }

    fn record_callback_invocation(&mut self, expr: &Expr) -> bool {
        if let Some(closure) = closure_expression(expr) {
            self.visit_invoked_closure(closure);
            return true;
        }
        let Expr::Path(path) = expr else {
            return false;
        };
        if self.invoke_bound_closure(path, expr) {
            return true;
        }
        let (lookup, constructor_only) = self.lookup_invocation(path);
        if constructor_only || !lookup.justified || lookup.candidates.len() != 1 {
            return false;
        }
        self.record(lookup, expr, quote::quote!(#path).to_string(), None);
        true
    }

    fn visit_project_callback<'a>(
        &mut self,
        lookup: &super::index::Lookup<'_>,
        mut arguments: impl Iterator<Item = &'a Expr>,
        receiver_offset: usize,
    ) {
        let [target] = lookup.candidates.as_slice() else {
            return;
        };
        if !lookup.justified || target.signature.asyncness {
            return;
        }
        if let Some(argument) = target
            .invoked_parameter
            .and_then(|index| index.checked_sub(receiver_offset))
            .and_then(|index| arguments.nth(index))
        {
            self.record_callback_invocation(argument);
        }
    }
}

fn is_assignment_operator(op: &syn::BinOp) -> bool {
    matches!(
        op,
        syn::BinOp::AddAssign(_)
            | syn::BinOp::SubAssign(_)
            | syn::BinOp::MulAssign(_)
            | syn::BinOp::DivAssign(_)
            | syn::BinOp::RemAssign(_)
            | syn::BinOp::BitXorAssign(_)
            | syn::BinOp::BitAndAssign(_)
            | syn::BinOp::BitOrAssign(_)
            | syn::BinOp::ShlAssign(_)
            | syn::BinOp::ShrAssign(_)
    )
}

fn closure_expression(expr: &Expr) -> Option<&syn::ExprClosure> {
    match expr {
        Expr::Closure(closure) => Some(closure),
        Expr::Paren(paren) => closure_expression(&paren.expr),
        Expr::Group(group) => closure_expression(&group.expr),
        _ => None,
    }
}

fn callable_path_expression(expr: &Expr) -> Option<&syn::ExprPath> {
    match expr {
        Expr::Path(path) => Some(path),
        Expr::Paren(paren) => callable_path_expression(&paren.expr),
        Expr::Group(group) => callable_path_expression(&group.expr),
        _ => None,
    }
}
