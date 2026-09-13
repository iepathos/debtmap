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

    pub fn type_from_owned(
        &self,
        ty: &TypeSyntax,
        context: &Context,
        substitutions: &Substitutions,
    ) -> TypeFact {
        self.expand_owned(ty, context, substitutions, 0)
    }

    pub(super) fn expand_type(
        &self,
        ty: &syn::Type,
        context: &Context,
        substitutions: &Substitutions,
        depth: usize,
    ) -> TypeFact {
        self.expand_owned(&TypeSyntax::from_syn(ty), context, substitutions, depth)
    }

    pub(super) fn expand_path(
        &self,
        path: &syn::Path,
        context: &Context,
        substitutions: &Substitutions,
        depth: usize,
    ) -> TypeFact {
        self.expand_owned(&TypeSyntax::from_path(path), context, substitutions, depth)
    }

    fn expand_owned(
        &self,
        ty: &TypeSyntax,
        context: &Context,
        substitutions: &Substitutions,
        depth: usize,
    ) -> TypeFact {
        if depth >= EXPANSION_LIMIT {
            return TypeFact::Unknown(UnknownReason::AnalysisLimit);
        }
        match ty {
            TypeSyntax::Path {
                segments,
                arguments,
            } => self.expand_owned_path(segments, arguments, context, substitutions, depth + 1),
            TypeSyntax::Reference { mutable, inner } => TypeFact::Reference {
                mutable: *mutable,
                inner: Box::new(self.expand_owned(inner, context, substitutions, depth + 1)),
            },
            TypeSyntax::Tuple(fields) => TypeFact::Tuple(
                fields
                    .iter()
                    .map(|ty| self.expand_owned(ty, context, substitutions, depth + 1))
                    .collect(),
            ),
            TypeSyntax::Dynamic(bounds) => TypeFact::Dynamic(
                bounds
                    .iter()
                    .map(|path| self.resolve_trait_bound(path, context))
                    .collect(),
            ),
            TypeSyntax::Const(value) => TypeFact::Const(value.clone()),
            TypeSyntax::Unsupported => TypeFact::Unknown(UnknownReason::UnsupportedTypeOperation),
        }
    }

    fn expand_owned_path(
        &self,
        segments: &[String],
        arguments: &[TypeSyntax],
        context: &Context,
        substitutions: &Substitutions,
        depth: usize,
    ) -> TypeFact {
        if depth >= EXPANSION_LIMIT {
            return TypeFact::Unknown(UnknownReason::AnalysisLimit);
        }
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
        let candidates = self.type_candidates(segments, context);
        let arguments = arguments
            .iter()
            .map(|ty| self.expand_owned(ty, context, substitutions, depth + 1))
            .collect();
        match candidates.as_slice() {
            [] => self.unavailable_type(segments, context),
            [declaration] => {
                let fact = self.expand_declaration(declaration, arguments, depth);
                if self.type_path_conflicts(segments, context) {
                    with_uncertainty(fact, UnknownReason::AmbiguousDeclaration)
                } else {
                    fact
                }
            }
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
        if segments.len() == 1
            && !self.explicitly_bound_type(&segments[0], context)
            && let Some(primitive) = super::super::types::PrimitiveType::from_name(&segments[0])
        {
            return TypeFact::Primitive(primitive);
        }
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
            Some(alias) => self.expand_owned(
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
}
