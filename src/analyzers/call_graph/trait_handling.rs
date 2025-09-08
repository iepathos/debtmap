/// Trait handling and method resolution for call graph extraction
use crate::analyzers::type_tracker::{ScopeKind, TypeTracker};
use syn::{Expr, Local, Pat};

/// Handles trait resolution and method dispatch
pub struct TraitHandler<'a> {
    type_tracker: &'a mut TypeTracker,
    current_impl_type: Option<String>,
}

impl<'a> TraitHandler<'a> {
    pub fn new(type_tracker: &'a mut TypeTracker) -> Self {
        Self {
            type_tracker,
            current_impl_type: None,
        }
    }

    /// Set the current impl type
    pub fn set_current_impl_type(&mut self, impl_type: Option<String>) {
        self.current_impl_type = impl_type;
    }

    /// Get the current impl type
    pub fn current_impl_type(&self) -> &Option<String> {
        &self.current_impl_type
    }

    /// Enter a new scope
    pub fn enter_scope(&mut self, kind: ScopeKind) {
        self.type_tracker
            .enter_scope(kind, self.current_impl_type.clone());
    }

    /// Exit the current scope
    pub fn exit_scope(&mut self) {
        self.type_tracker.exit_scope();
    }

    /// Track a local variable binding
    pub fn track_local(&mut self, local: &Local) {
        if let Pat::Ident(pat_ident) = &local.pat {
            let var_name = pat_ident.ident.to_string();

            // Try to infer type from the initializer
            if let Some(init) = &local.init {
                if let Some(type_info) = self.type_tracker.resolve_expr_type(&init.expr) {
                    self.type_tracker.record_variable(var_name, type_info);
                }
            }
        }
    }

    /// Process an expression and track type information
    pub fn process_expression(&mut self, expr: &Expr) {
        match expr {
            Expr::Let(expr_let) => {
                if let Pat::Ident(pat_ident) = &*expr_let.pat {
                    let var_name = pat_ident.ident.to_string();
                    if let Some(type_info) = self.type_tracker.resolve_expr_type(&expr_let.expr) {
                        self.type_tracker.record_variable(var_name, type_info);
                    }
                }
            }
            _ => {
                // Process other expressions as needed
            }
        }
    }

    /// Get the type of a receiver expression
    pub fn get_receiver_type(&mut self, receiver: &Expr) -> Option<String> {
        self.type_tracker
            .resolve_expr_type(receiver)
            .map(|t| t.type_name)
    }

    /// Check if an expression is a method call on self
    pub fn is_self_method_call(receiver: &Expr) -> bool {
        match receiver {
            Expr::Path(path) => path.path.is_ident("self"),
            _ => false,
        }
    }

    /// Resolve a method name with trait information
    pub fn resolve_method_name(&mut self, receiver: &Expr, method_name: &str) -> String {
        if Self::is_self_method_call(receiver) {
            // If calling on self, use the current impl type
            if let Some(impl_type) = &self.current_impl_type {
                return format!("{}::{}", impl_type, method_name);
            }
        } else if let Some(receiver_type) = self.get_receiver_type(receiver) {
            // If we know the receiver type, qualify the method
            return format!("{}::{}", receiver_type, method_name);
        }

        // Fallback to unqualified name
        method_name.to_string()
    }

    /// Extract the impl type from an impl block
    pub fn extract_impl_type(impl_block: &syn::ItemImpl) -> Option<String> {
        match &*impl_block.self_ty {
            syn::Type::Path(type_path) => {
                let segments: Vec<String> = type_path
                    .path
                    .segments
                    .iter()
                    .map(|seg| seg.ident.to_string())
                    .collect();

                if !segments.is_empty() {
                    Some(segments.join("::"))
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Check if a function has a self parameter
    pub fn has_self_param(sig: &syn::Signature) -> bool {
        sig.inputs
            .iter()
            .any(|arg| matches!(arg, syn::FnArg::Receiver(_)))
    }

    /// Process function arguments and track parameter types
    pub fn process_function_params(&mut self, _sig: &syn::Signature) {
        // Track self parameter if present
        if let Some(_impl_type) = &self.current_impl_type {
            self.type_tracker.track_self_param(None, None);
        }
    }

    /// Clear all tracked type information
    pub fn clear_tracked_types(&mut self) {
        // Exit all scopes and re-enter module scope
        // Since we don't have a way to check if there are more scopes,
        // we'll just exit a reasonable number of times
        for _ in 0..10 {
            self.type_tracker.exit_scope();
        }
        self.type_tracker.enter_scope(ScopeKind::Module, None);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn test_is_self_method_call() {
        let self_expr: Expr = parse_quote! { self };
        assert!(TraitHandler::is_self_method_call(&self_expr));

        let other_expr: Expr = parse_quote! { other };
        assert!(!TraitHandler::is_self_method_call(&other_expr));

        let field_expr: Expr = parse_quote! { self.field };
        assert!(!TraitHandler::is_self_method_call(&field_expr));
    }

    #[test]
    fn test_extract_impl_type() {
        let impl_block: syn::ItemImpl = parse_quote! {
            impl MyStruct {
                fn method(&self) {}
            }
        };
        assert_eq!(
            TraitHandler::extract_impl_type(&impl_block),
            Some("MyStruct".to_string())
        );

        let impl_block_qualified: syn::ItemImpl = parse_quote! {
            impl module::MyStruct {
                fn method(&self) {}
            }
        };
        assert_eq!(
            TraitHandler::extract_impl_type(&impl_block_qualified),
            Some("module::MyStruct".to_string())
        );
    }

    #[test]
    fn test_has_self_param() {
        let sig_with_self: syn::Signature = parse_quote! {
            fn method(&self)
        };
        assert!(TraitHandler::has_self_param(&sig_with_self));

        let sig_with_mut_self: syn::Signature = parse_quote! {
            fn method(&mut self)
        };
        assert!(TraitHandler::has_self_param(&sig_with_mut_self));

        let sig_without_self: syn::Signature = parse_quote! {
            fn function(x: i32)
        };
        assert!(!TraitHandler::has_self_param(&sig_without_self));
    }
}
