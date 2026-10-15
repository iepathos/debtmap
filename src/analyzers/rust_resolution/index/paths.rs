//! Lexical declaration paths and namespace-specific import precedence.
use super::*;
#[path = "path_availability.rs"]
mod availability;
#[path = "path_conflicts.rs"]
mod conflicts;
#[path = "path_namespaces.rs"]
mod namespaces;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum Namespace {
    Type,
    Value,
}

impl WorkspaceIndex {
    pub(super) fn same_workspace(&self, left: &Path, right: &Path) -> bool {
        self.workspace_peer(left)(right, self.workspace_of(right))
    }

    pub(super) fn workspace_of(&self, file: &Path) -> Option<usize> {
        self.workspace_membership.get(file).copied()
    }

    /// Checks candidates, given their file and workspace, against `file`.
    pub(super) fn workspace_peer<'a>(
        &self,
        file: &'a Path,
    ) -> impl Fn(&Path, Option<usize>) -> bool + 'a {
        let id = self.workspace_of(file);
        move |other, other_id| match id {
            Some(id) => other_id == Some(id),
            None => file == other,
        }
    }

    pub(super) fn resolve_paths(&self, path: &[String], context: &Context) -> Vec<Vec<String>> {
        self.resolved_namespace_paths(path, context, Namespace::Type)
    }

    pub(super) fn resolve_value_paths(
        &self,
        path: &[String],
        context: &Context,
    ) -> Vec<Vec<String>> {
        self.resolved_namespace_paths(path, context, Namespace::Value)
    }

    fn resolved_namespace_paths(
        &self,
        path: &[String],
        context: &Context,
        namespace: Namespace,
    ) -> Vec<Vec<String>> {
        let key = (path.to_vec(), context.clone(), namespace);
        if let Some(paths) = self.resolved_paths.get(&key) {
            return paths.clone();
        }
        let paths: Vec<_> = self
            .resolve_namespace_paths(path, context, 0, namespace)
            .iter()
            .flat_map(|path| self.expand_reexports(path, context, 0, namespace))
            .collect();
        self.resolved_paths.insert(key, paths.clone());
        paths
    }

    fn resolve_namespace_paths(
        &self,
        path: &[String],
        context: &Context,
        depth: usize,
        namespace: Namespace,
    ) -> Vec<Vec<String>> {
        if depth >= EXPANSION_LIMIT || path.is_empty() {
            return Vec::new();
        }
        if matches!(path[0].as_str(), "crate" | "self" | "super" | "::") {
            return vec![relative_path(path, &context.module)];
        }
        let first_namespace = if path.len() > 1 {
            Namespace::Type
        } else {
            namespace
        };
        let first = qualified(&context.module, &path[0]);
        let local = qualified_path(&context.module, path);
        let mut explicit = self
            .has_binding(&first, context, first_namespace)
            .then(|| local.clone())
            .into_iter()
            .collect::<Vec<_>>();
        explicit.extend(
            self.context_imports(context)
                .filter(|import| !import.glob && import.alias == path[0])
                .flat_map(|import| self.import_targets(import, context, depth, first_namespace))
                .map(|prefix| qualified_path(&prefix, &path[1..])),
        );
        if !explicit.is_empty() {
            return explicit;
        }
        std::iter::once(local)
            .chain(
                self.context_imports(context)
                    .filter(|import| import.glob)
                    .map(|import| {
                        qualified_path(&relative_path(&import.path, &context.module), path)
                    }),
            )
            .collect()
    }

    fn import_targets(
        &self,
        import: &Import,
        context: &Context,
        depth: usize,
        namespace: Namespace,
    ) -> Vec<Vec<String>> {
        if self.declaration_shadows(import, context, namespace) {
            return Vec::new();
        }
        let targets = if import.path == [import.alias.clone()] {
            vec![relative_path(&import.path, &import.context.module)]
        } else {
            self.resolve_namespace_paths(&import.path, &import.context, depth + 1, namespace)
        };
        targets
            .into_iter()
            .filter(|path| {
                let other = match namespace {
                    Namespace::Type => Namespace::Value,
                    Namespace::Value => Namespace::Type,
                };
                self.has_binding(path, context, namespace)
                    || !self.has_binding(path, context, other)
            })
            .collect()
    }

    /// Type declarations bind path prefixes, so following a same-named import back
    /// through one only grows the path. Value bindings may be conditional alternatives.
    fn declaration_shadows(
        &self,
        import: &Import,
        context: &Context,
        namespace: Namespace,
    ) -> bool {
        let bound = qualified(&import.context.module, &import.alias);
        matches!(namespace, Namespace::Type)
            && self.has_binding(&bound, context, namespace)
            && relative_path(&import.path, &import.context.module).starts_with(&bound)
    }

    fn expand_reexports(
        &self,
        path: &[String],
        context: &Context,
        depth: usize,
        namespace: Namespace,
    ) -> Vec<Vec<String>> {
        if depth >= EXPANSION_LIMIT {
            return Vec::new();
        }
        for position in 0..path.len() {
            let segment_namespace = if position + 1 < path.len() {
                Namespace::Type
            } else {
                namespace
            };
            let imports: Vec<_> = self
                .module_imports(&path[..position])
                .filter(|import| !import.glob && import.alias == path[position])
                .filter(|import| self.same_workspace(&context.file, &import.context.file))
                .flat_map(|import| self.import_targets(import, context, depth, segment_namespace))
                .map(|prefix| qualified_path(&prefix, &path[position + 1..]))
                .filter(|expanded| expanded != path)
                .collect();
            if !imports.is_empty() {
                let local = self
                    .has_binding(&path[..=position], context, segment_namespace)
                    .then(|| path.to_vec())
                    .into_iter();
                return local
                    .chain(imports.iter().flat_map(|expanded| {
                        self.expand_reexports(expanded, context, depth + 1, namespace)
                    }))
                    .collect();
            }
        }
        vec![path.to_vec()]
    }

    /// Identify established import namespaces; unavailable imports remain conservative.
    pub fn path_namespaces(&self, path: &[String], context: &Context) -> Option<(bool, bool)> {
        let types = self
            .resolve_paths(path, context)
            .iter()
            .any(|path| self.has_binding(path, context, Namespace::Type));
        let values = self
            .resolve_value_paths(path, context)
            .iter()
            .any(|path| self.has_binding(path, context, Namespace::Value));
        (types || values).then_some((types, values))
    }

    pub(super) fn explicitly_bound_type(&self, name: &str, context: &Context) -> bool {
        self.has_binding(&qualified(&context.module, name), context, Namespace::Type)
            || self
                .context_imports(context)
                .filter(|import| !import.glob && import.alias == name)
                .any(|import| {
                    !self
                        .import_targets(import, context, 0, Namespace::Type)
                        .is_empty()
                })
    }
}
