//! Lower caller-written syntax without leaking its block scope into declarations.
use super::*;
use crate::analyzers::rust_resolution::types::TraitBoundFact;

impl Body<'_> {
    pub fn declared_type(&self, ty: &syn::Type) -> TypeFact {
        self.scoped_type(ty, 0)
    }

    fn scoped_type(&self, ty: &syn::Type, depth: usize) -> TypeFact {
        if depth >= 32 {
            return TypeFact::Unknown(UnknownReason::AnalysisLimit);
        }
        match ty {
            syn::Type::Reference(reference) => TypeFact::Reference {
                mutable: reference.mutability.is_some(),
                inner: Box::new(self.scoped_type(&reference.elem, depth + 1)),
            },
            syn::Type::Tuple(tuple) => TypeFact::Tuple(
                tuple
                    .elems
                    .iter()
                    .map(|ty| self.scoped_type(ty, depth + 1))
                    .collect(),
            ),
            syn::Type::Paren(paren) => self.scoped_type(&paren.elem, depth + 1),
            syn::Type::Group(group) => self.scoped_type(&group.elem, depth + 1),
            syn::Type::Path(path) if path.qself.is_none() => self.scoped_path(&path.path, depth),
            syn::Type::TraitObject(object) => TypeFact::Dynamic(
                object
                    .bounds
                    .iter()
                    .filter_map(|bound| match bound {
                        syn::TypeParamBound::Trait(bound) => Some(self.scoped_bound(&bound.path)),
                        _ => None,
                    })
                    .collect(),
            ),
            _ => type_shadows::find(&self.bindings, ty)
                .map(unavailable)
                .unwrap_or_else(unknown),
        }
    }

    fn scoped_path(&self, path: &syn::Path, depth: usize) -> TypeFact {
        if self.type_path_shadowed(path) {
            return unavailable(path_names(path));
        }
        if path.is_ident("Self") {
            return self.callable.owner.clone().unwrap_or(TypeFact::SelfType);
        }
        let arguments = path
            .segments
            .last()
            .map(|segment| self.scoped_arguments(&segment.arguments, depth + 1))
            .unwrap_or_default();
        let project = self.index.type_from_path_arguments(
            path,
            arguments.clone(),
            &self.callable.context,
            &self.substitutions,
        );
        if !permits_external_model(&project) {
            return project;
        }
        let external = self.index.external_path(path, &self.callable.context);
        super::super::models::modeled_type(&external, arguments).unwrap_or(project)
    }

    pub(in crate::analyzers::rust_resolution) fn scoped_arguments(
        &self,
        arguments: &syn::PathArguments,
        depth: usize,
    ) -> Vec<TypeFact> {
        let syn::PathArguments::AngleBracketed(arguments) = arguments else {
            return Vec::new();
        };
        arguments
            .args
            .iter()
            .filter_map(|argument| match argument {
                syn::GenericArgument::Type(ty) => Some(self.scoped_type(ty, depth)),
                syn::GenericArgument::Const(value) => {
                    Some(TypeFact::Const(quote::quote!(#value).to_string()))
                }
                syn::GenericArgument::Lifetime(_) => None,
                _ => Some(unknown()),
            })
            .collect()
    }

    fn scoped_bound(&self, path: &syn::Path) -> TraitBoundFact {
        if type_shadows::find_path(&self.bindings, path).is_some() {
            return TraitBoundFact::Unresolved {
                path: path_names(path),
                file: self.callable.context.file.clone(),
                module: self.callable.context.module.clone(),
                candidates: Vec::new(),
                reason: UnknownReason::UnsupportedTypeOperation,
            };
        }
        self.index
            .resolve_trait_bound(&path_names(path), &self.callable.context)
    }

    fn type_path_shadowed(&self, path: &syn::Path) -> bool {
        path.leading_colon.is_none()
            && path
                .segments
                .first()
                .is_some_and(|segment| self.bindings.type_shadowed(&segment.ident.to_string()))
    }
}

fn permits_external_model(fact: &TypeFact) -> bool {
    match fact {
        TypeFact::Unknown(UnknownReason::UnavailableDefinition) => true,
        TypeFact::Uncertain { constraint, reason } => {
            matches!(reason, UnknownReason::UnavailableDefinition)
                && matches!(constraint.as_ref(), TypeFact::UnavailablePath(_))
        }
        _ => false,
    }
}

fn unavailable(path: Vec<String>) -> TypeFact {
    TypeFact::Uncertain {
        constraint: Box::new(TypeFact::UnavailablePath(path)),
        reason: UnknownReason::UnsupportedTypeOperation,
    }
}

fn path_names(path: &syn::Path) -> Vec<String> {
    let mut names: Vec<_> = path
        .segments
        .iter()
        .map(|segment| segment.ident.to_string())
        .collect();
    if path.leading_colon.is_some() {
        names.insert(0, "::".into());
    }
    names
}

#[cfg(test)]
#[path = "scoped_types_tests.rs"]
mod tests;
