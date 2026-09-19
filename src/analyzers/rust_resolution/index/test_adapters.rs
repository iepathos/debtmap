//! Syntax conveniences for unit tests of the shared fact-based lookup core.
use super::*;

impl WorkspaceIndex {
    pub(super) fn type_from_syn(
        &self,
        ty: &syn::Type,
        context: &Context,
        substitutions: &Substitutions,
    ) -> TypeFact {
        self.type_from_owned(&TypeSyntax::from_syn(ty), context, substitutions)
    }

    fn type_from_path(
        &self,
        path: &syn::Path,
        context: &Context,
        substitutions: &Substitutions,
    ) -> TypeFact {
        self.type_from_owned(&TypeSyntax::from_path(path), context, substitutions)
    }

    pub(in crate::analyzers::rust_resolution) fn lookup_call(
        &self,
        path: &syn::Path,
        qself: Option<&syn::QSelf>,
        context: &Context,
    ) -> Lookup<'_> {
        let substitutions = Substitutions::new();
        if let Some(qself) = qself {
            return self.lookup_qualified(path, qself, context, &substitutions);
        }
        let segments = resolution_segments(path);
        if segments.len() > 1 {
            let mut owner_path = path.clone();
            owner_path.segments.pop();
            let owner = self.type_from_path(&owner_path, context, &substitutions);
            if owner.has_nominal_candidates() || owner.has_receiver_identity() {
                return self.lookup_associated(&owner, &segments[segments.len() - 1], context);
            }
        }
        self.lookup_free_value(path, context)
    }

    fn lookup_qualified(
        &self,
        path: &syn::Path,
        qself: &syn::QSelf,
        context: &Context,
        substitutions: &Substitutions,
    ) -> Lookup<'_> {
        let owner = self.type_from_syn(&qself.ty, context, substitutions);
        let segments = path_segments(path);
        let Some(name) = segments.last() else {
            return Lookup::default();
        };
        if qself.position == 0 {
            return self.lookup_associated(&owner, name, context);
        }
        let qualified_trait = syn::Path {
            leading_colon: path.leading_colon,
            segments: path.segments.iter().take(qself.position).cloned().collect(),
        };
        let trait_type = self.type_from_path(&qualified_trait, context, substitutions);
        self.lookup_trait_associated(&owner, &trait_type, name)
    }
}
