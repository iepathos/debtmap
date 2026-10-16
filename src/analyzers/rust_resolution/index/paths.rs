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

struct Walk {
    paths: Vec<Vec<String>>,
    complete: bool,
}

impl Walk {
    fn done(paths: Vec<Vec<String>>) -> Self {
        Self {
            paths,
            complete: true,
        }
    }

    fn cycle() -> Self {
        Self {
            paths: Vec::new(),
            complete: false,
        }
    }
}

fn segment_namespace(position: usize, len: usize, namespace: Namespace) -> Namespace {
    if position + 1 < len {
        Namespace::Type
    } else {
        namespace
    }
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
            .chain(self.exported_glob_paths(path, context))
            .collect()
    }

    fn exported_glob_paths(&self, path: &[String], context: &Context) -> Vec<Vec<String>> {
        self.context_imports(context)
            .filter(|import| import.glob)
            .map(|import| relative_path(&import.path, &context.module))
            .filter(|module| self.exports_name(module, &path[0], context))
            .map(|module| qualified_path(&module, path))
            .collect()
    }

    fn exports_name(&self, module: &[String], name: &str, context: &Context) -> bool {
        let mut seen = HashSet::new();
        self.exports_from(module, name, context, &mut seen)
    }

    fn exports_from(
        &self,
        module: &[String],
        name: &str,
        context: &Context,
        seen: &mut HashSet<Vec<String>>,
    ) -> bool {
        if !seen.insert(module.to_vec()) {
            return false;
        }
        let candidate = qualified(module, name);
        self.has_binding(&candidate, context, Namespace::Type)
            || self.has_binding(&candidate, context, Namespace::Value)
            || self.reexports_name(module, name, context, seen)
    }

    fn reexports_name(
        &self,
        module: &[String],
        name: &str,
        context: &Context,
        seen: &mut HashSet<Vec<String>>,
    ) -> bool {
        self.module_imports(module)
            .filter(|import| self.same_workspace(&context.file, &import.context.file))
            .any(|import| self.import_exports_name(import, module, name, context, seen))
    }

    fn import_exports_name(
        &self,
        import: &Import,
        module: &[String],
        name: &str,
        context: &Context,
        seen: &mut HashSet<Vec<String>>,
    ) -> bool {
        if !import.glob {
            return import.alias == name;
        }
        let target = relative_path(&import.path, module);
        self.exports_from(&target, name, context, seen)
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
        self.expand_cached(path, context, depth, namespace, &mut Vec::new())
            .paths
    }

    fn expand_cached(
        &self,
        path: &[String],
        context: &Context,
        depth: usize,
        namespace: Namespace,
        stack: &mut Vec<Vec<String>>,
    ) -> Walk {
        if depth >= EXPANSION_LIMIT {
            return Walk::done(Vec::new());
        }
        let key = self.reexport_key(path, context, depth, namespace);
        if let Some(paths) = self.reexports.get(&key) {
            return Walk::done(paths.clone());
        }
        if stack.iter().any(|seen| seen.as_slice() == path) {
            return Walk::cycle();
        }
        stack.push(path.to_vec());
        let walk = self.expand_step(path, context, depth, namespace, stack);
        stack.pop();
        if walk.complete {
            self.reexports.insert(key, walk.paths.clone());
        }
        walk
    }

    fn reexport_key(
        &self,
        path: &[String],
        context: &Context,
        depth: usize,
        namespace: Namespace,
    ) -> ReexportKey {
        let workspace = self.workspace_of(&context.file);
        let file = workspace.is_none().then(|| context.file.clone());
        (path.to_vec(), workspace, file, namespace, depth as u8)
    }

    fn expand_step(
        &self,
        path: &[String],
        context: &Context,
        depth: usize,
        namespace: Namespace,
        stack: &mut Vec<Vec<String>>,
    ) -> Walk {
        for position in 0..path.len() {
            let targets = self.reexport_targets(path, position, context, depth, namespace);
            if !targets.is_empty() {
                return self
                    .follow_reexports(path, position, &targets, context, depth, namespace, stack);
            }
        }
        Walk::done(vec![path.to_vec()])
    }

    fn reexport_targets(
        &self,
        path: &[String],
        position: usize,
        context: &Context,
        depth: usize,
        namespace: Namespace,
    ) -> Vec<Vec<String>> {
        let segment_namespace = segment_namespace(position, path.len(), namespace);
        self.module_imports(&path[..position])
            .filter(|import| !import.glob && import.alias == path[position])
            .filter(|import| self.same_workspace(&context.file, &import.context.file))
            .flat_map(|import| self.import_targets(import, context, depth, segment_namespace))
            .map(|prefix| qualified_path(&prefix, &path[position + 1..]))
            .filter(|expanded| expanded != path)
            .collect()
    }

    #[allow(clippy::too_many_arguments)]
    fn follow_reexports(
        &self,
        path: &[String],
        position: usize,
        targets: &[Vec<String>],
        context: &Context,
        depth: usize,
        namespace: Namespace,
        stack: &mut Vec<Vec<String>>,
    ) -> Walk {
        let segment_namespace = segment_namespace(position, path.len(), namespace);
        let local = self
            .has_binding(&path[..=position], context, segment_namespace)
            .then(|| path.to_vec());
        let mut walk = Walk::done(local.into_iter().collect());
        for target in targets {
            let child = self.expand_cached(target, context, depth + 1, namespace, stack);
            walk.complete &= child.complete;
            walk.paths.extend(child.paths);
        }
        walk
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
