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
        let declarations = self.type_candidates(&resolution_segments(path), context);
        match declarations.as_slice() {
            [declaration] if declaration.unit => {
                Some(self.type_from_path(path, context, substitutions))
            }
            _ => None,
        }
    }

    pub(super) fn declared_value_type(
        &self,
        path: &syn::Path,
        context: &Context,
    ) -> Option<TypeFact> {
        let paths = self.resolve_paths(&resolution_segments(path), context);
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
            [fact] => Some(fact.clone()),
            _ => Some(with_uncertainty(
                TypeFact::Ambiguous(facts),
                UnknownReason::AmbiguousDeclaration,
            )),
        }
    }
}
