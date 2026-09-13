//! Trait bounds are resolved in their declaring context before propagation.

use super::super::types::TraitBoundFact;
use super::*;

impl WorkspaceIndex {
    pub(super) fn resolve_trait_bound(&self, path: &[String], context: &Context) -> TraitBoundFact {
        let candidates: Vec<_> = self
            .type_candidates(path, context)
            .into_iter()
            .filter(|declaration| declaration.is_trait)
            .map(|declaration| declaration.id.clone())
            .collect();
        match candidates.as_slice() {
            [declaration] => TraitBoundFact::Resolved(declaration.clone()),
            _ => TraitBoundFact::Unresolved {
                path: path.to_vec(),
                file: context.file.clone(),
                module: context.module.clone(),
                reason: if candidates.is_empty() {
                    UnknownReason::UnavailableDefinition
                } else {
                    UnknownReason::AmbiguousDeclaration
                },
                candidates,
            },
        }
    }

    pub(super) fn resolve_substitutions(&self, substitutions: &Substitutions) -> Substitutions {
        substitutions
            .iter()
            .map(|(name, fact)| (name.clone(), self.resolve_bound_fact(fact)))
            .collect()
    }

    fn resolve_bound_fact(&self, fact: &TypeFact) -> TypeFact {
        match fact {
            TypeFact::BoundedGeneric { name, traits } => TypeFact::BoundedGeneric {
                name: name.clone(),
                traits: self.resolve_bounds(traits),
            },
            TypeFact::Dynamic(traits) => TypeFact::Dynamic(self.resolve_bounds(traits)),
            _ => fact.clone(),
        }
    }

    fn resolve_bounds(&self, bounds: &[TraitBoundFact]) -> Vec<TraitBoundFact> {
        bounds
            .iter()
            .map(|bound| match bound {
                TraitBoundFact::Resolved(_) => bound.clone(),
                TraitBoundFact::Unresolved {
                    path, file, module, ..
                } => self.resolve_trait_bound(
                    path,
                    &Context {
                        file: file.clone(),
                        module: module.clone(),
                    },
                ),
            })
            .collect()
    }
}
