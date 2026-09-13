//! Lexical type and value paths within a function body.
use super::{
    body::{Body, unknown},
    index::Lookup,
    types::TypeFact,
};

impl<'a> Body<'a> {
    pub(super) fn path_fact(&self, path: &syn::Path) -> TypeFact {
        if let Some(ident) = path.get_ident() {
            if self
                .callable
                .const_parameters
                .iter()
                .any(|name| ident == name)
            {
                return unknown();
            }
            if let Some(fact) = self.bindings.get(&ident.to_string()) {
                return fact;
            }
            if ident == "Self" {
                return self.callable.owner.clone().unwrap_or_else(unknown);
            }
        }
        if self.path_shadowed(path) {
            return unknown();
        }
        self.index
            .value_from_path(path, &self.callable.context, &self.substitutions)
            .unwrap_or_else(unknown)
    }

    pub(super) fn lookup_path(&self, path: &syn::ExprPath) -> Lookup<'a> {
        if self.path_shadowed(&path.path) {
            return Lookup::default();
        }
        if path.path.segments.len() == 1
            && path
                .path
                .get_ident()
                .is_some_and(|i| self.bindings.get(&i.to_string()).is_some())
        {
            return Lookup::default();
        }
        if path
            .path
            .segments
            .first()
            .is_some_and(|s| s.ident == "Self")
            && let (Some(owner), Some(method)) = (&self.callable.owner, path.path.segments.last())
        {
            return self.index.lookup_associated(
                owner,
                &method.ident.to_string(),
                &self.callable.context,
            );
        }
        if self.substitutions.is_empty() {
            self.index
                .lookup_call(&path.path, path.qself.as_ref(), &self.callable.context)
        } else {
            self.index.lookup_call_with_substitutions(
                &path.path,
                path.qself.as_ref(),
                &self.callable.context,
                &self.substitutions,
            )
        }
    }

    pub(super) fn path_shadowed(&self, path: &syn::Path) -> bool {
        path.leading_colon.is_none()
            && path.segments.first().is_some_and(|s| {
                if path.segments.len() > 1 {
                    self.bindings.type_shadowed(&s.ident.to_string())
                } else {
                    self.bindings.value_shadowed(&s.ident.to_string())
                }
            })
    }

    pub(super) fn path_type(&self, path: &syn::Path) -> TypeFact {
        self.declared_type(&syn::Type::Path(syn::TypePath {
            qself: None,
            path: path.clone(),
        }))
    }
}
