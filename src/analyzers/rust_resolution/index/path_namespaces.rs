//! Declaration membership in the type and value namespaces.
use super::*;

impl WorkspaceIndex {
    pub(in crate::analyzers::rust_resolution::index) fn type_candidates(
        &self,
        path: &[String],
        context: &Context,
    ) -> Vec<&TypeDeclaration> {
        let positions: HashSet<_> = self
            .resolve_paths(path, context)
            .iter()
            .filter_map(|path| self.type_paths.get(path))
            .flatten()
            .copied()
            .collect();
        let mut candidates: Vec<_> = positions
            .into_iter()
            .map(|position| &self.declarations[position])
            .filter(|decl| self.same_workspace(&context.file, &decl.id.file))
            .collect();
        candidates.sort_by(|left, right| left.id.cmp(&right.id));
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
            || path.last().is_some_and(|name| {
                self.named_callables(name).any(|call| {
                    call.kind == CallableKind::FreeFunction
                        && self.same_workspace(&context.file, &call.context.file)
                        && qualified(&call.context.module, name) == path
                })
            })
    }

    pub(super) fn path_is_known(&self, path: &[String], context: &Context) -> bool {
        self.has_binding(path, context, Namespace::Type)
            || self.has_binding(path, context, Namespace::Value)
    }
}
