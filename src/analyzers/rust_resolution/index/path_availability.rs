//! Distinguish missing explicit bindings from optional namespace search paths.
use super::*;

impl WorkspaceIndex {
    pub(in crate::analyzers::rust_resolution::index) fn value_path_has_unresolved_import(
        &self,
        path: &[String],
        context: &Context,
    ) -> bool {
        std::iter::once(relative_path(path, &context.module))
            .chain(self.resolve_namespace_paths(path, context, 0, Namespace::Value))
            .any(|path| self.path_has_unresolved_import(&path, context, 0))
    }

    fn path_has_unresolved_import(&self, path: &[String], context: &Context, depth: usize) -> bool {
        if depth >= EXPANSION_LIMIT {
            return true;
        }
        (0..path.len())
            .any(|position| self.segment_has_unresolved_import(path, position, context, depth))
    }

    fn segment_has_unresolved_import(
        &self,
        path: &[String],
        position: usize,
        context: &Context,
        depth: usize,
    ) -> bool {
        let namespace = if position + 1 < path.len() {
            Namespace::Type
        } else {
            Namespace::Value
        };
        self.module_imports(&path[..position])
            .filter(|import| !import.glob && import.alias == path[position])
            .filter(|import| self.same_workspace(&context.file, &import.context.file))
            .any(|import| {
                let targets: Vec<_> = self
                    .import_targets(import, context, depth, namespace)
                    .iter()
                    .map(|prefix| qualified_path(prefix, &path[position + 1..]))
                    .collect();
                self.import_group_is_unresolved(&targets, path, context, depth)
            })
    }

    fn import_group_is_unresolved(
        &self,
        targets: &[Vec<String>],
        path: &[String],
        context: &Context,
        depth: usize,
    ) -> bool {
        if targets.is_empty() {
            return false;
        }
        !targets
            .iter()
            .any(|target| self.expanded_path_has_binding(target, context, depth + 1))
            || targets
                .iter()
                .filter(|target| target.as_slice() != path)
                .any(|target| self.path_has_unresolved_import(target, context, depth + 1))
    }

    // Search alternatives belong to one import. A known type-only reexport also
    // establishes that binding; it is not an unavailable value competitor.
    fn expanded_path_has_binding(&self, path: &[String], context: &Context, depth: usize) -> bool {
        [Namespace::Value, Namespace::Type]
            .into_iter()
            .any(|namespace| {
                self.expand_reexports(path, context, depth, namespace)
                    .iter()
                    .any(|target| self.has_binding(target, context, namespace))
            })
    }
}
