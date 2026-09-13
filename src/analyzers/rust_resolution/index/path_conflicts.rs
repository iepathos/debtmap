//! Explicit binding conflicts survive when one imported declaration is unavailable.
use super::*;

impl WorkspaceIndex {
    pub(in crate::analyzers::rust_resolution::index) fn type_path_conflicts(
        &self,
        path: &[String],
        context: &Context,
    ) -> bool {
        self.namespace_path_conflicts(path, context, Namespace::Type)
    }

    pub(in crate::analyzers::rust_resolution::index) fn value_path_conflicts(
        &self,
        path: &[String],
        context: &Context,
    ) -> bool {
        self.namespace_path_conflicts(path, context, Namespace::Value)
    }

    fn namespace_path_conflicts(
        &self,
        path: &[String],
        context: &Context,
        namespace: Namespace,
    ) -> bool {
        let paths = self.resolved_namespace_paths(path, context, namespace);
        if paths.len() < 2 {
            return false;
        }
        std::iter::once(relative_path(path, &context.module))
            .chain(paths)
            .any(|path| {
                (0..path.len()).any(|position| {
                    let namespace = if position + 1 < path.len() {
                        Namespace::Type
                    } else {
                        namespace
                    };
                    self.explicit_segment_conflicts(&path[..=position], context, namespace)
                })
            })
    }

    fn explicit_segment_conflicts(
        &self,
        path: &[String],
        context: &Context,
        namespace: Namespace,
    ) -> bool {
        let Some((name, module)) = path.split_last() else {
            return false;
        };
        let local = self
            .has_binding(path, context, namespace)
            .then(|| path.to_vec());
        let targets: HashSet<_> = local
            .into_iter()
            .chain(
                self.module_imports(module)
                    .filter(|import| !import.glob && import.alias == *name)
                    .filter(|import| self.same_workspace(&context.file, &import.context.file))
                    .flat_map(|import| self.import_targets(import, context, 0, namespace)),
            )
            .collect();
        targets.len() > 1
    }
}
