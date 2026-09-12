//! Lexical declaration paths and imports.

use super::*;

impl WorkspaceIndex {
    pub(super) fn type_candidates(
        &self,
        path: &[String],
        context: &Context,
    ) -> Vec<&TypeDeclaration> {
        let paths = self.resolve_paths(path, context);
        let positions: HashSet<_> = paths
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

    pub(super) fn same_workspace(&self, left: &Path, right: &Path) -> bool {
        left == right
            || (self
                .contexts
                .get(left)
                .is_some_and(|contexts| contexts.len() == 1)
                && self
                    .contexts
                    .get(right)
                    .is_some_and(|contexts| contexts.len() == 1)
                && self
                    .roots
                    .get(left)
                    .zip(self.roots.get(right))
                    .is_some_and(|(left, right)| {
                        left.len() == 1 && right.len() == 1 && !left.is_disjoint(right)
                    }))
    }

    pub(super) fn resolve_paths(&self, path: &[String], context: &Context) -> Vec<Vec<String>> {
        self.resolve_paths_bounded(path, context, 0)
            .iter()
            .flat_map(|path| self.expand_reexports(path, context, 0))
            .collect()
    }

    fn expand_reexports(
        &self,
        path: &[String],
        context: &Context,
        depth: usize,
    ) -> Vec<Vec<String>> {
        if depth >= EXPANSION_LIMIT {
            return Vec::new();
        }
        for position in 0..path.len() {
            let imports: Vec<_> = self
                .module_imports(&path[..position])
                .filter(|import| !import.glob && import.alias == path[position])
                .filter(|import| self.same_workspace(&context.file, &import.context.file))
                .collect();
            if !imports.is_empty() {
                return imports
                    .into_iter()
                    .flat_map(|import| {
                        self.resolve_paths_bounded(&import.path, &import.context, depth + 1)
                            .into_iter()
                            .map(|prefix| qualified_path(&prefix, &path[position + 1..]))
                            .filter(|expanded| expanded != path)
                            .flat_map(|expanded| {
                                self.expand_reexports(&expanded, context, depth + 1)
                            })
                            .collect::<Vec<_>>()
                    })
                    .collect();
            }
        }
        vec![path.to_vec()]
    }

    pub(super) fn resolve_paths_bounded(
        &self,
        path: &[String],
        context: &Context,
        depth: usize,
    ) -> Vec<Vec<String>> {
        if depth >= EXPANSION_LIMIT || path.is_empty() {
            return Vec::new();
        }
        if matches!(path[0].as_str(), "crate" | "self" | "super" | "::") {
            return vec![relative_path(path, &context.module)];
        }
        let imports: Vec<_> = self
            .context_imports(context)
            .filter(|import| import.alias == path[0] && !import.glob)
            .collect();
        if !imports.is_empty() {
            return imports
                .iter()
                .flat_map(|import| {
                    let expanded = import
                        .path
                        .iter()
                        .chain(path.iter().skip(1))
                        .cloned()
                        .collect::<Vec<_>>();
                    if expanded == path {
                        vec![relative_path(&expanded, &context.module)]
                    } else {
                        self.resolve_paths_bounded(&expanded, context, depth + 1)
                    }
                })
                .collect();
        }
        let local = qualified_path(&context.module, path);
        let mut paths = vec![local];
        for import in self.context_imports(context).filter(|import| import.glob) {
            let prefix = relative_path(&import.path, &context.module);
            paths.push(qualified_path(&prefix, path));
        }
        paths
    }
}
