//! Detect unindexed block declarations in nominal types and trait bounds alike.
use super::Bindings;
use syn::visit::Visit;

pub(super) fn find(bindings: &Bindings, ty: &syn::Type) -> Option<Vec<String>> {
    let mut shadow = TypeShadows {
        bindings,
        found: None,
    };
    shadow.visit_type(ty);
    shadow.found
}

struct TypeShadows<'a> {
    bindings: &'a Bindings,
    found: Option<Vec<String>>,
}

impl TypeShadows<'_> {
    fn check_path(&mut self, path: &syn::Path) {
        if path.leading_colon.is_none()
            && path
                .segments
                .first()
                .is_some_and(|s| self.bindings.type_shadowed(&s.ident.to_string()))
        {
            self.found = Some(path.segments.iter().map(|s| s.ident.to_string()).collect());
        }
    }
}

impl<'ast> Visit<'ast> for TypeShadows<'_> {
    fn visit_type_path(&mut self, ty: &'ast syn::TypePath) {
        self.check_path(&ty.path);
        syn::visit::visit_type_path(self, ty);
    }

    fn visit_trait_bound(&mut self, bound: &'ast syn::TraitBound) {
        self.check_path(&bound.path);
        syn::visit::visit_trait_bound(self, bound);
    }
}
