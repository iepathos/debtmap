//! Value bindings are resolved together before projecting callable or type facts.

use super::*;

struct ValueBindings<'a> {
    functions: Vec<&'a Callable>,
    constructors: Vec<&'a TypeDeclaration>,
    values: Vec<&'a ValueDeclaration>,
    explicit_conflict: bool,
    unresolved_competitor: bool,
}

impl ValueBindings<'_> {
    fn constructor_only(&self) -> bool {
        !self.unresolved_competitor
            && !self.constructors.is_empty()
            && self.functions.is_empty()
            && self.values.is_empty()
    }

    fn ambiguous(&self) -> bool {
        self.explicit_conflict
            || self.unresolved_competitor
            || self.functions.len() + self.constructors.len() + self.values.len() > 1
    }

    fn fact(&self, facts: impl IntoIterator<Item = TypeFact>) -> Option<TypeFact> {
        let mut facts: Vec<_> = facts.into_iter().collect();
        let fact = match facts.len() {
            0 => return None,
            1 => facts.pop()?,
            _ => TypeFact::Ambiguous(facts),
        };
        Some(if self.ambiguous() {
            with_uncertainty(fact, UnknownReason::AmbiguousDeclaration)
        } else {
            fact
        })
    }
}

impl WorkspaceIndex {
    fn value_bindings(&self, path: &syn::Path, context: &Context) -> ValueBindings<'_> {
        let segments = resolution_segments(path);
        let paths = self.resolve_value_paths(&segments, context);
        let mut bindings = ValueBindings {
            functions: self.free_candidates(&paths, context),
            constructors: indexed_positions(&paths, &self.type_paths)
                .into_iter()
                .map(|position| &self.declarations[position])
                .filter(|declaration| declaration.unit || declaration.tuple_arity.is_some())
                .filter(|declaration| self.same_workspace(&context.file, &declaration.id.file))
                .collect(),
            values: indexed_positions(&paths, &self.value_paths)
                .into_iter()
                .map(|position| &self.values[position])
                .filter(|value| self.same_workspace(&context.file, &value.context.file))
                .collect(),
            explicit_conflict: self.value_path_conflicts(&segments, context),
            unresolved_competitor: false,
        };
        // Only otherwise-constructor-only bindings could discard the call record.
        bindings.unresolved_competitor = bindings.constructor_only()
            && self.value_path_has_unresolved_import(&segments, context);
        bindings.functions.sort_by_key(|callable| &callable.id);
        bindings
            .functions
            .dedup_by(|left, right| left.id == right.id);
        bindings
            .constructors
            .sort_by_key(|declaration| &declaration.id);
        bindings
            .constructors
            .dedup_by(|left, right| left.id == right.id);
        bindings
            .values
            .sort_by_key(|value| (&value.context.file, value.line, value.column));
        bindings.values.dedup_by(|left, right| {
            left.context == right.context && left.line == right.line && left.column == right.column
        });
        bindings
    }

    #[cfg(test)]
    pub(super) fn lookup_free_value(&self, path: &syn::Path, context: &Context) -> Lookup<'_> {
        self.value_call_lookup(path, context).0
    }

    pub fn value_call_lookup(&self, path: &syn::Path, context: &Context) -> (Lookup<'_>, bool) {
        let bindings = self.value_bindings(path, context);
        let constructor_only = bindings.constructor_only();
        let ambiguous = bindings.ambiguous();
        let provenance = if bindings.functions.first().is_some_and(|call| {
            qualified(&call.context.module, &call.signature.ident)
                != relative_path(&resolution_segments(path), &context.module)
        }) {
            CallEdgeProvenance::ImportResolution
        } else {
            CallEdgeProvenance::AstDirect
        };
        (
            Lookup {
                justified: bindings.functions.len() == 1 && !ambiguous,
                candidates: bindings.functions,
                provenance,
                reason: ambiguous.then_some(UncertaintyReason::AmbiguousDeclaration),
            },
            constructor_only,
        )
    }

    pub fn value_from_path(
        &self,
        path: &syn::Path,
        context: &Context,
        arguments: &[TypeFact],
    ) -> Option<TypeFact> {
        let bindings = self.value_bindings(path, context);
        let values = bindings
            .values
            .iter()
            .map(|value| self.type_from_owned(&value.ty, &value.context, &Substitutions::new()));
        let constructors = bindings
            .constructors
            .iter()
            .filter(|declaration| declaration.unit)
            .map(|declaration| constructor_fact(declaration, arguments));
        bindings.fact(values.chain(constructors))
    }

    /// Invocation facts retain all represented alternatives, including constructors.
    pub fn value_call_result(
        &self,
        path: &syn::Path,
        arity: usize,
        context: &Context,
        arguments: &[TypeFact],
    ) -> Option<TypeFact> {
        let bindings = self.value_bindings(path, context);
        let functions = bindings
            .functions
            .iter()
            .map(|function| self.return_type(function, None, arguments));
        let constructors = bindings
            .constructors
            .iter()
            .filter(|declaration| declaration.tuple_arity == Some(arity))
            .map(|declaration| constructor_fact(declaration, arguments));
        bindings.fact(functions.chain(constructors))
    }
}

fn constructor_fact(declaration: &TypeDeclaration, arguments: &[TypeFact]) -> TypeFact {
    TypeFact::Nominal {
        declaration: declaration.id.clone(),
        arguments: complete_arguments(arguments.to_vec(), &declaration.generics),
    }
}

fn indexed_positions(
    paths: &[Vec<String>],
    index: &HashMap<Vec<String>, Vec<usize>>,
) -> Vec<usize> {
    let mut positions: Vec<_> = paths
        .iter()
        .filter_map(|path| index.get(path))
        .flatten()
        .copied()
        .collect();
    positions.sort_unstable();
    positions.dedup();
    positions
}

#[cfg(test)]
#[path = "values/tests.rs"]
mod tests;
