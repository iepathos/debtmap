//! Value-namespace declarations are independent of type aliases and enums.

use super::*;

impl WorkspaceIndex {
    pub fn value_from_path(
        &self,
        path: &syn::Path,
        context: &Context,
        substitutions: &Substitutions,
    ) -> Option<TypeFact> {
        if let Some(value) = self.declared_value_type(path, context) {
            return Some(value);
        }
        let declarations = self.constructor_declarations(path, context);
        match declarations.as_slice() {
            [declaration] if declaration.unit => {
                let fact = self.unit_constructor_fact(declaration, path, context, substitutions);
                Some(
                    if self.value_path_conflicts(&resolution_segments(path), context) {
                        with_uncertainty(fact, UnknownReason::AmbiguousDeclaration)
                    } else {
                        fact
                    },
                )
            }
            _ => None,
        }
    }

    fn unit_constructor_fact(
        &self,
        declaration: &TypeDeclaration,
        path: &syn::Path,
        context: &Context,
        substitutions: &Substitutions,
    ) -> TypeFact {
        let arguments = path
            .segments
            .last()
            .map(|segment| TypeSyntax::arguments(&segment.arguments))
            .unwrap_or_default()
            .iter()
            .map(|ty| self.type_from_owned(ty, context, substitutions))
            .collect();
        TypeFact::Nominal {
            declaration: declaration.id.clone(),
            arguments: complete_arguments(arguments, &declaration.generics),
        }
    }

    /// Tuple constructors provide a nominal result without being callable bodies.
    pub fn constructor_result(
        &self,
        path: &syn::Path,
        arity: usize,
        context: &Context,
        arguments: &[TypeFact],
    ) -> Option<TypeFact> {
        if self.declared_value_type(path, context).is_some() {
            return None;
        }
        let declarations = self.constructor_declarations(path, context);
        match declarations.as_slice() {
            [declaration]
                if declaration.tuple_arity == Some(arity)
                    && !self.value_path_conflicts(&resolution_segments(path), context) =>
            {
                Some(TypeFact::Nominal {
                    declaration: declaration.id.clone(),
                    arguments: complete_arguments(arguments.to_vec(), &declaration.generics),
                })
            }
            _ => None,
        }
    }

    fn constructor_declarations(
        &self,
        path: &syn::Path,
        context: &Context,
    ) -> Vec<&TypeDeclaration> {
        let positions: HashSet<_> = self
            .resolve_value_paths(&resolution_segments(path), context)
            .iter()
            .filter_map(|path| self.type_paths.get(path))
            .flatten()
            .copied()
            .collect();
        positions
            .into_iter()
            .map(|position| &self.declarations[position])
            .filter(|declaration| declaration.unit || declaration.tuple_arity.is_some())
            .filter(|declaration| self.same_workspace(&context.file, &declaration.id.file))
            .collect()
    }

    pub(super) fn declared_value_type(
        &self,
        path: &syn::Path,
        context: &Context,
    ) -> Option<TypeFact> {
        let paths = self.resolve_value_paths(&resolution_segments(path), context);
        let positions: HashSet<_> = paths
            .iter()
            .filter_map(|path| self.value_paths.get(path))
            .flatten()
            .copied()
            .collect();
        let mut values: Vec<_> = positions
            .into_iter()
            .map(|position| &self.values[position])
            .filter(|value| self.same_workspace(&value.context.file, &context.file))
            .collect();
        values.sort_by_key(|value| (value.context.file.clone(), value.line, value.column));
        let facts: Vec<_> = values
            .iter()
            .map(|value| self.type_from_owned(&value.ty, &value.context, &Substitutions::new()))
            .collect();
        match facts.as_slice() {
            [] => None,
            [fact] => Some(
                if self.value_path_conflicts(&resolution_segments(path), context) {
                    with_uncertainty(fact.clone(), UnknownReason::AmbiguousDeclaration)
                } else {
                    fact.clone()
                },
            ),
            _ => Some(with_uncertainty(
                TypeFact::Ambiguous(facts),
                UnknownReason::AmbiguousDeclaration,
            )),
        }
    }
}
