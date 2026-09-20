//! Closure syntax lives only for the current body parse, never in workspace records.
use super::{Bindings, Body};
use syn::Expr;

#[derive(Clone)]
pub(super) struct BoundClosure {
    expression: syn::ExprClosure,
    captures: Bindings,
}

impl Body<'_> {
    pub(in crate::analyzers::rust_resolution) fn capture_closure(
        &mut self,
        closure: &syn::ExprClosure,
    ) -> usize {
        let index = self.bound_closures.len();
        self.bound_closures.push(BoundClosure {
            expression: closure.clone(),
            captures: self.bindings.clone(),
        });
        index
    }

    pub(in crate::analyzers::rust_resolution) fn invoke_bound_closure(
        &mut self,
        path: &syn::ExprPath,
        expression: &Expr,
    ) -> bool {
        let Some(index) = path
            .path
            .get_ident()
            .and_then(|name| self.bindings.closure(&name.to_string()))
        else {
            return false;
        };
        let closure = self.bound_closures[index].clone();
        if !closure.captures.captures_unchanged(&self.bindings) {
            self.record_indirect_invocation(expression);
            return true;
        }
        let current = std::mem::replace(&mut self.bindings, closure.captures);
        self.visit_invoked_closure(&closure.expression);
        self.bindings = current;
        true
    }
}
