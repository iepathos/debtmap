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
        if let Some((receiver_id, _)) = receiver.nominal() {
            if matches!(
                call.owner,
                Some(TypeFact::Generic(_) | TypeFact::BoundedGeneric { .. })
            ) {
                return self.trait_in_scope(call, context);
            }
            if let Some(TypeFact::Ambiguous(owners)) = &call.owner {
                return owners.iter().any(|owner| owner_compatible(owner, receiver))
                    && self.trait_in_scope(call, context);
            }
            return call
                .owner
                .as_ref()
                .and_then(TypeFact::nominal)
                .is_some_and(|(owner_id, _)| owner_id == receiver_id)
                && call
                    .owner
                    .as_ref()
                    .is_some_and(|owner| owner_compatible(owner, receiver))
                && self.trait_in_scope(call, context);
        }
        if let TypeFact::Reference { inner, .. } = receiver {
            return self.receiver_admits(inner, call, context);
        }
        if let TypeFact::Dynamic(bounds) | TypeFact::BoundedGeneric { traits: bounds, .. } =
            receiver
        {
            return call.trait_path.as_ref().is_some_and(|trait_path| {
                bounds
                    .iter()
                    .any(|bound| self.same_trait(bound, context, trait_path, &call.context))
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

    pub(super) fn same_trait(
        &self,
        left: &[String],
        left_context: &Context,
        right: &[String],
        right_context: &Context,
    ) -> bool {
        let left = self.type_candidates(left, left_context);
        let right = self.type_candidates(right, right_context);
        left.len() == 1 && right.len() == 1 && left[0].is_trait && left[0].id == right[0].id
    }

    pub(super) fn trait_in_scope(&self, call: &Callable, context: &Context) -> bool {
        let Some(path) = &call.trait_path else {
            return true;
        };
        let targets = self.type_candidates(path, &call.context);
        let [target] = targets.as_slice() else {
            return false;
        };
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

    pub fn lookup_call(
        &self,
        path: &syn::Path,
        qself: Option<&syn::QSelf>,
        context: &Context,
    ) -> Lookup<'_> {
        self.lookup_call_with_substitutions(path, qself, context, &Substitutions::new())
    }

    pub fn lookup_call_with_substitutions(
        &self,
        path: &syn::Path,
        qself: Option<&syn::QSelf>,
        context: &Context,
        substitutions: &Substitutions,
    ) -> Lookup<'_> {
        if let Some(qself) = qself {
            return self.lookup_qualified(path, qself, context, substitutions);
        }
        if self.declared_value_type(path, context).is_some() {
            return Lookup::default();
        }
        let segments = resolution_segments(path);
        if segments.len() > 1 {
            let mut owner_path = path.clone();
            owner_path.segments.pop();
            let owner = self.type_from_path(&owner_path, context, substitutions);
            if owner.has_nominal_candidates() {
                return self.lookup_associated(&owner, &segments[segments.len() - 1], context);
            }
        }
        let paths = self.resolve_paths(&segments, context);
        let candidates = self
            .named_callables(segments.last().map(String::as_str).unwrap_or_default())
            .filter(|call| call.kind == CallableKind::FreeFunction)
            .filter(|call| self.same_workspace(&call.context.file, &context.file))
            .filter(|call| {
                paths.contains(&qualified(
                    &call.context.module,
                    &call.signature.ident.clone(),
                ))
            })
            .collect::<Vec<_>>();
        let justified = candidates.len() == 1;
        let provenance = if candidates.first().is_some_and(|call| {
            qualified(&call.context.module, &call.signature.ident.clone())
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
        }
    }

    pub(super) fn lookup_qualified(
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
        let trait_path = &segments[..qself.position];
        let qualified_trait = syn::Path {
            leading_colon: path.leading_colon,
            segments: path.segments.iter().take(qself.position).cloned().collect(),
        };
        let trait_type = self.type_from_path(&qualified_trait, context, substitutions);
        let candidates = self
            .named_callables(name)
            .filter(|call| call.kind != CallableKind::TraitDeclaration)
            .filter(|call| {
                call.trait_type
                    .as_ref()
                    .is_some_and(|candidate| owner_compatible(candidate, &trait_type))
            })
            .filter(|call| {
                call.owner
                    .as_ref()
                    .is_some_and(|candidate| owner_compatible(candidate, &owner))
            })
            .filter(|call| {
                call.trait_path.as_ref().is_some_and(|candidate| {
                    self.same_trait(trait_path, context, candidate, &call.context)
                })
            })
            .collect::<Vec<_>>();
        let justified = candidates.len() == 1
            && owner.nominal().is_some()
            && candidates[0].requirements_known
            && candidates[0]
                .owner
                .as_ref()
                .is_some_and(|candidate| owner_match_known(candidate, &owner));
        Lookup {
            candidates,
            justified,
            provenance: CallEdgeProvenance::TypeResolution,
        }
    }
}
