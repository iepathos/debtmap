//! Declaration membership in the type and value namespaces.
use super::*;

impl WorkspaceIndex {
    pub(in crate::analyzers::rust_resolution::index) fn type_candidates(
        &self,
        path: &[String],
        context: &Context,
    ) -> Vec<&TypeDeclaration> {
        self.type_candidate_positions(path, context)
            .into_iter()
            .map(|position| &self.declarations[position])
            .collect()
    }

    pub(in crate::analyzers::rust_resolution::index) fn type_candidate_positions(
        &self,
        path: &[String],
        context: &Context,
    ) -> Vec<usize> {
        let positions: HashSet<_> = self
            .resolve_paths(path, context)
            .iter()
            .filter_map(|path| self.type_paths.get(path))
            .flatten()
            .copied()
            .collect();
        let mut candidates: Vec<_> = positions
            .into_iter()
            .filter(|position| {
                self.same_workspace(&context.file, &self.declarations[*position].id.file)
            })
            .collect();
        candidates.sort_by(|left, right| {
            self.declarations[*left]
                .id
                .cmp(&self.declarations[*right].id)
        });
        candidates
    }

    pub(super) fn has_binding(
        &self,
        path: &[String],
        context: &Context,
        namespace: Namespace,
    ) -> bool {
        match namespace {
            Namespace::Type => {
                self.type_paths
                    .get(path)
                    .into_iter()
                    .flatten()
                    .any(|position| {
                        self.same_workspace(&context.file, &self.declarations[*position].id.file)
                    })
                    || self
                        .module_files
                        .get(path)
                        .into_iter()
                        .flatten()
                        .any(|file| self.same_workspace(&context.file, file))
            }
            Namespace::Value => self.has_value_binding(path, context),
        }
    }

    fn has_value_binding(&self, path: &[String], context: &Context) -> bool {
        self.value_paths
            .get(path)
            .into_iter()
            .flatten()
            .any(|position| {
                self.same_workspace(&context.file, &self.values[*position].context.file)
            })
            || self
                .type_paths
                .get(path)
                .into_iter()
                .flatten()
                .any(|position| {
                    let declaration = &self.declarations[*position];
                    (declaration.unit || declaration.tuple_arity.is_some())
                        && self.same_workspace(&context.file, &declaration.id.file)
                })
            || self
                .free_callables_at(path)
                .any(|call| self.same_workspace(&context.file, &call.context.file))
    }
}
