//! Mutation scope analysis
//!
//! Functions for determining whether mutations affect local or external state.

use super::types::MutationScope;
use crate::analyzers::scope_tracker::{ScopeTracker, SelfKind};
use syn::{Expr, ExprField};

/// Determine the scope of a mutation from an expression
pub fn determine_mutation_scope(expr: &Expr, scope: &ScopeTracker) -> MutationScope {
    match expr {
        // Simple identifier: x = value
        Expr::Path(path) => {
            let ident = path
                .path
                .get_ident()
                .map(|i| i.to_string())
                .unwrap_or_default();

            if scope.is_local(&ident) {
                MutationScope::Local
            } else {
                // Conservative: assume external
                MutationScope::External
            }
        }

        // Field access: obj.field = value
        Expr::Field(field) => determine_field_mutation_scope(field, scope),

        // Index: arr[i] = value
        Expr::Index(index) => {
            if let Expr::Path(path) = &*index.expr {
                if let Some(ident) = path.path.get_ident() {
                    if scope.is_local(&ident.to_string()) {
                        return MutationScope::Local;
                    }
                }
            }
            MutationScope::External
        }

        // Pointer dereference: *ptr = value
        Expr::Unary(unary) if matches!(unary.op, syn::UnOp::Deref(_)) => {
            // Conservative: assume external
            MutationScope::External
        }

        _ => MutationScope::External,
    }
}

/// Determine mutation scope for field access expressions
pub fn determine_field_mutation_scope(field: &ExprField, scope: &ScopeTracker) -> MutationScope {
    match &*field.base {
        // self.field = value
        Expr::Path(path)
            if scope.is_self(
                &path
                    .path
                    .get_ident()
                    .map(|i| i.to_string())
                    .unwrap_or_default(),
            ) =>
        {
            // Check self kind
            if let Some(self_kind) = scope.get_self_kind() {
                match self_kind {
                    SelfKind::MutRef => MutationScope::External, // &mut self
                    SelfKind::Owned | SelfKind::MutOwned => {
                        // mut self or self (owned) - local mutation
                        MutationScope::Local
                    }
                    _ => MutationScope::External,
                }
            } else {
                MutationScope::External
            }
        }

        // local_var.field = value
        Expr::Path(path) => {
            let ident = path
                .path
                .get_ident()
                .map(|i| i.to_string())
                .unwrap_or_default();

            if scope.is_local(&ident) {
                MutationScope::Local
            } else {
                MutationScope::External
            }
        }

        _ => MutationScope::External,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn test_local_var_mutation() {
        let scope = ScopeTracker::new();
        let mut scope = scope;
        scope.add_local_var("x".to_string());

        let expr: Expr = parse_quote!(x);
        assert_eq!(determine_mutation_scope(&expr, &scope), MutationScope::Local);
    }

    #[test]
    fn test_external_var_mutation() {
        let scope = ScopeTracker::new();
        let expr: Expr = parse_quote!(external);
        assert_eq!(
            determine_mutation_scope(&expr, &scope),
            MutationScope::External
        );
    }

    #[test]
    fn test_pointer_dereference_external() {
        let scope = ScopeTracker::new();
        let expr: Expr = parse_quote!(*ptr);
        assert_eq!(
            determine_mutation_scope(&expr, &scope),
            MutationScope::External
        );
    }
}
