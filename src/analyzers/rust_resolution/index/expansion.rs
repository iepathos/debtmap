//! Bounded type expansion and direct substitution.

use super::*;

impl WorkspaceIndex {
    pub fn type_from_syn(
        &self,
        ty: &syn::Type,
        context: &Context,
        substitutions: &Substitutions,
    ) -> TypeFact {
        self.expand_type(ty, context, substitutions, 0)
    }

    pub fn type_from_path(
        &self,
        path: &syn::Path,
        context: &Context,
        substitutions: &Substitutions,
    ) -> TypeFact {
        self.expand_path(path, context, substitutions, 0)
    }

    pub(super) fn expand_type(
        &self,
        ty: &syn::Type,
        context: &Context,
        substitutions: &Substitutions,
        depth: usize,
    ) -> TypeFact {
        if depth >= EXPANSION_LIMIT {
            return TypeFact::Unknown(UnknownReason::AnalysisLimit);
        }
        match ty {
            syn::Type::Path(path) if path.qself.is_none() => {
                self.expand_path(&path.path, context, substitutions, depth + 1)
            }
            syn::Type::Reference(reference) => TypeFact::Reference {
                mutable: reference.mutability.is_some(),
                inner: Box::new(self.expand_type(
                    &reference.elem,
                    context,
                    substitutions,
                    depth + 1,
                )),
            },
            syn::Type::Paren(paren) => {
                self.expand_type(&paren.elem, context, substitutions, depth + 1)
            }
            syn::Type::Group(group) => {
                self.expand_type(&group.elem, context, substitutions, depth + 1)
            }
            syn::Type::Tuple(tuple) => TypeFact::Tuple(
                tuple
                    .elems
                    .iter()
                    .map(|ty| self.expand_type(ty, context, substitutions, depth + 1))
                    .collect(),
            ),
            syn::Type::TraitObject(object) => {
                TypeFact::Dynamic(object.bounds.iter().filter_map(trait_bound_path).collect())
            }
            _ => TypeFact::Unknown(UnknownReason::UnsupportedTypeOperation),
        }
    }

    pub(super) fn expand_path(
        &self,
        path: &syn::Path,
        context: &Context,
        substitutions: &Substitutions,
        depth: usize,
    ) -> TypeFact {
        if depth >= EXPANSION_LIMIT {
            return TypeFact::Unknown(UnknownReason::AnalysisLimit);
        }
        let segments = resolution_segments(path);
        if segments.len() == 1 {
            if let Some(fact) = substitutions.get(&segments[0]) {
                return fact.clone();
            }
            if segments[0] == "Self" {
                return TypeFact::SelfType;
            }
            if segments[0] == "Unknown" {
                return TypeFact::unknown();
            }
        }
        let candidates = self.type_candidates(&segments, context);
        let arguments = path
            .segments
            .last()
            .map(|last| self.arguments(&last.arguments, context, substitutions, depth + 1))
            .unwrap_or_default();
        match candidates.as_slice() {
            [] => self.unavailable_type(&segments, context),
            [declaration] => self.expand_declaration(declaration, arguments, depth),
            declarations => TypeFact::Uncertain {
                constraint: Box::new(TypeFact::Ambiguous(
                    declarations
                        .iter()
                        .map(|declaration| {
                            self.expand_declaration(declaration, arguments.clone(), depth)
                        })
                        .collect(),
                )),
                reason: UnknownReason::AmbiguousDeclaration,
            },
        }
    }

    fn unavailable_type(&self, segments: &[String], context: &Context) -> TypeFact {
        let paths = self.resolve_paths(segments, context);
        let explicitly_qualified =
            segments.len() > 1 || paths != [relative_path(segments, &context.module)];
        if explicitly_qualified {
            TypeFact::Uncertain {
                constraint: Box::new(TypeFact::UnavailablePath(segments.to_vec())),
                reason: UnknownReason::UnavailableDefinition,
            }
        } else {
            TypeFact::Unknown(UnknownReason::UnavailableDefinition)
        }
    }

    fn expand_declaration(
        &self,
        declaration: &TypeDeclaration,
        arguments: Vec<TypeFact>,
        depth: usize,
    ) -> TypeFact {
        let arguments = complete_arguments(arguments, &declaration.generics);
        match &declaration.alias {
            Some(alias) => self.expand_type(
                alias,
                &declaration.context,
                &bind_generics(&declaration.generics, &arguments),
                depth + 1,
            ),
            None => TypeFact::Nominal {
                declaration: declaration.id.clone(),
                arguments,
            },
        }
    }

    pub(super) fn arguments(
        &self,
        args: &syn::PathArguments,
        context: &Context,
        substitutions: &Substitutions,
        depth: usize,
    ) -> Vec<TypeFact> {
        let syn::PathArguments::AngleBracketed(args) = args else {
            return Vec::new();
        };
        args.args
            .iter()
            .filter_map(|arg| match arg {
                syn::GenericArgument::Type(ty) => {
                    Some(self.expand_type(ty, context, substitutions, depth + 1))
                }
                syn::GenericArgument::Const(expr) => {
                    Some(TypeFact::Const(expr.to_token_stream().to_string()))
                }
                syn::GenericArgument::Lifetime(_) => None,
                _ => Some(TypeFact::Unknown(UnknownReason::UnsupportedTypeOperation)),
            })
            .collect()
    }

    pub fn return_type(
        &self,
        call: &Callable,
        receiver: Option<&TypeFact>,
        arguments: &[TypeFact],
    ) -> TypeFact {
        let mut substitutions = call.substitutions.clone();
        if let Some(receiver) = receiver {
            let receiver = strip_references(receiver);
            substitutions.insert("Self".to_string(), receiver.clone());
            if let Some(owner) = &call.owner {
                infer_substitutions(owner, receiver, &mut substitutions);
            }
        }
        for (name, argument) in generic_names(&call.signature.generics)
            .iter()
            .zip(arguments)
        {
            substitutions.insert(name.clone(), argument.clone());
        }
        let output = match &call.signature.output {
            syn::ReturnType::Default => TypeFact::Tuple(Vec::new()),
            syn::ReturnType::Type(_, ty) => self.type_from_syn(ty, &call.context, &substitutions),
        };
        let output = match receiver.and_then(TypeFact::uncertainty_reason) {
            Some(reason) => with_uncertainty(output, reason),
            None => output,
        };
        if call.signature.asyncness.is_some() {
            TypeFact::Future(Box::new(output))
        } else {
            output
        }
    }

    pub fn field_type(&self, receiver: &TypeFact, member: &syn::Member) -> TypeFact {
        match receiver {
            TypeFact::Reference { inner, .. } => self.field_type(inner, member),
            TypeFact::Uncertain { constraint, reason } => {
                with_uncertainty(self.field_type(constraint, member), reason.clone())
            }
            TypeFact::Ambiguous(owners) => with_uncertainty(
                TypeFact::Ambiguous(
                    owners
                        .iter()
                        .map(|owner| self.field_type(owner, member))
                        .collect(),
                ),
                UnknownReason::AmbiguousDeclaration,
            ),
            _ => self.known_field_type(receiver, member),
        }
    }

    fn known_field_type(&self, receiver: &TypeFact, member: &syn::Member) -> TypeFact {
        let receiver = strip_references(receiver);
        if let (TypeFact::Tuple(fields), syn::Member::Unnamed(member)) = (receiver, member) {
            return fields
                .get(member.index as usize)
                .cloned()
                .unwrap_or_else(TypeFact::unknown);
        }
        let Some((id, arguments)) = receiver.nominal() else {
            return TypeFact::unknown();
        };
        let Some(declaration) = self.declaration(id) else {
            return TypeFact::unknown();
        };
        let key = match member {
            syn::Member::Named(name) => name.to_string(),
            syn::Member::Unnamed(index) => index.index.to_string(),
        };
        let Some(ty) = declaration.fields.get(&key) else {
            return TypeFact::Unknown(UnknownReason::UnavailableDefinition);
        };
        self.type_from_syn(
            ty,
            &declaration.context,
            &bind_generics(&declaration.generics, arguments),
        )
    }
}
