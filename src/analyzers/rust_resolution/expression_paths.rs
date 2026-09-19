//! Lexical type and value paths within a function body.
use super::{
    body::{Body, unknown},
    index::{AssociatedQuery, Lookup},
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
        if let Some(query) = self.associated_query(path) {
            return self
                .index
                .lookup_associated_query(&query, &self.callable.context);
        }
        self.index
            .lookup_free_value(&path.path, &self.callable.context)
    }

    pub(super) fn associated_query(&self, path: &syn::ExprPath) -> Option<AssociatedQuery> {
        let member = path.path.segments.last()?;
        let (owner, trait_type) = if let Some(qself) = &path.qself {
            let trait_type = (qself.position > 0).then(|| {
                self.path_type(&syn::Path {
                    leading_colon: path.path.leading_colon,
                    segments: path
                        .path
                        .segments
                        .iter()
                        .take(qself.position)
                        .cloned()
                        .collect(),
                })
            });
            (self.declared_type(&qself.ty), trait_type)
        } else {
            let mut owner_path = path.path.clone();
            owner_path.segments.pop();
            owner_path.segments.pop_punct();
            if owner_path.segments.is_empty() {
                return None;
            }
            let owner = self.path_type(&owner_path);
            if !owner.has_nominal_candidates() && !owner.has_receiver_identity() {
                return None;
            }
            (owner, None)
        };
        Some(AssociatedQuery {
            owner,
            trait_type,
            member: member.ident.to_string(),
            arguments: self.arguments(&member.arguments),
        })
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
