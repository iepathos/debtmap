//! Constrained callable candidate lookup.

use super::*;

impl WorkspaceIndex {
    pub fn lookup_methods(&self, receiver: &TypeFact, name: &str, context: &Context) -> Lookup<'_> {
        self.lookup_members(receiver, name, context, true)
    }

    pub fn lookup_associated(
        &self,
        receiver: &TypeFact,
        name: &str,
        context: &Context,
    ) -> Lookup<'_> {
        self.lookup_members(receiver, name, context, false)
    }

    pub(super) fn lookup_members(
        &self,
        receiver: &TypeFact,
        name: &str,
        context: &Context,
        dotted: bool,
    ) -> Lookup<'_> {
        let candidates = self
            .named_callables(name)
            .filter(|call| call.owner.is_some() && (!dotted || call.has_receiver()))
            .filter(|call| call.kind != CallableKind::TraitDeclaration)
            .filter(|call| self.same_workspace(&context.file, &call.context.file))
            .filter(|call| self.receiver_admits(receiver, call, context))
            .collect::<Vec<_>>();
        let justified = candidates.len() == 1
            && !receiver.is_uncertain()
            && receiver.nominal().is_some()
            && candidates[0].requirements_known
            && receiver_adjustment_known(receiver, candidates[0], dotted)
            && candidates[0]
                .owner
                .as_ref()
                .is_some_and(|owner| owner_match_known(owner, receiver));
        Lookup {
            candidates,
            justified,
            provenance: CallEdgeProvenance::TypeResolution,
            reason: None,
        }
    }

    pub(super) fn receiver_admits(
        &self,
        receiver: &TypeFact,
        call: &Callable,
        context: &Context,
    ) -> bool {
        match receiver {
            TypeFact::Uncertain { constraint, .. } => {
                return self.receiver_admits(constraint, call, context);
            }
            TypeFact::Ambiguous(owners) => {
                return owners
                    .iter()
                    .any(|owner| self.receiver_admits(owner, call, context));
            }
            _ => {}
        }
        if receiver.has_receiver_identity() {
            return call
                .owner
                .as_ref()
                .is_some_and(|owner| constrained_owner_compatible(owner, receiver))
                && self.trait_in_scope(call, context);
        }
        if let TypeFact::Reference { inner, .. } = receiver {
            return self.receiver_admits(inner, call, context);
        }
        if let TypeFact::Dynamic(bounds) | TypeFact::BoundedGeneric { traits: bounds, .. } =
            receiver
        {
            return self.callable_traits(call).any(|declaration| {
                declaration.is_trait && bounds.iter().any(|bound| bound.admits(&declaration.id))
            });
        }
        !matches!(
            receiver,
            TypeFact::Tuple(_)
                | TypeFact::Future(_)
                | TypeFact::Const(_)
                | TypeFact::UnavailablePath(_)
        ) && self.trait_in_scope(call, context)
    }

    pub(super) fn trait_in_scope(&self, call: &Callable, context: &Context) -> bool {
        if call.trait_path.is_none() {
            return true;
        }
        let [position] = call.trait_candidates.as_slice() else {
            return false;
        };
        let target = &self.declarations[*position];
        if target.context.module == context.module
            && self.same_workspace(&target.id.file, &context.file)
        {
            return true;
        }
        self.context_imports(context).any(|import| {
            if import.glob {
                self.resolve_paths(&import.path, context)
                    .contains(&target.context.module)
            } else {
                self.type_candidates(std::slice::from_ref(&import.alias), context)
                    .iter()
                    .any(|decl| decl.id == target.id)
            }
        })
    }

    pub(in crate::analyzers::rust_resolution) fn lookup_free_value(
        &self,
        path: &syn::Path,
        context: &Context,
    ) -> Lookup<'_> {
        if self.declared_value_type(path, context).is_some() {
            return Lookup::default();
        }
        let segments = resolution_segments(path);
        let paths = self.resolve_value_paths(&segments, context);
        let candidates = self.free_candidates(
            &paths,
            context,
            segments.last().map(String::as_str).unwrap_or_default(),
        );
        let ambiguous = candidates.len() == 1 && self.value_path_conflicts(&segments, context);
        let justified = candidates.len() == 1 && !ambiguous;
        let provenance = if candidates.first().is_some_and(|call| {
            qualified(&call.context.module, &call.signature.ident.to_string())
                != relative_path(&segments, &context.module)
        }) {
            CallEdgeProvenance::ImportResolution
        } else {
            CallEdgeProvenance::AstDirect
        };
        Lookup {
            candidates,
            justified,
            provenance,
            reason: ambiguous.then_some(UncertaintyReason::AmbiguousDeclaration),
        }
    }

    pub(super) fn lookup_trait_associated(
        &self,
        owner: &TypeFact,
        trait_type: &TypeFact,
        name: &str,
    ) -> Lookup<'_> {
        let candidates = self
            .named_callables(name)
            .filter(|call| call.kind != CallableKind::TraitDeclaration)
            .filter(|call| {
                call.trait_type
                    .as_ref()
                    .is_some_and(|candidate| owner_compatible(candidate, trait_type))
            })
            .filter(|call| {
                call.owner
                    .as_ref()
                    .is_some_and(|candidate| owner_compatible(candidate, owner))
            })
            .filter(|call| self.has_same_trait_fact(trait_type, call))
            .collect::<Vec<_>>();
        let justified = candidates.len() == 1
            && owner.has_receiver_identity()
            && !trait_type.is_uncertain()
            && candidates[0].requirements_known
            && candidates[0]
                .owner
                .as_ref()
                .is_some_and(|candidate| owner_match_known(candidate, owner));
        Lookup {
            candidates,
            justified,
            provenance: CallEdgeProvenance::TypeResolution,
            reason: matches!(
                trait_type.uncertainty_reason(),
                Some(UnknownReason::AmbiguousDeclaration)
            )
            .then_some(UncertaintyReason::AmbiguousDeclaration),
        }
    }

    fn has_same_trait_fact(&self, fact: &TypeFact, call: &Callable) -> bool {
        match (fact.nominal(), call.trait_candidates.as_slice()) {
            (Some((id, _)), [position]) => {
                let candidate = &self.declarations[*position];
                candidate.is_trait && candidate.id == *id
            }
            _ => false,
        }
    }
}
