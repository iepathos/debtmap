//! Cache unambiguous workspace membership before resolving declaration types.
use super::*;

impl WorkspaceIndex {
    pub(super) fn index_workspace_membership(&mut self) {
        let mut roots: Vec<_> = self.roots.values().flatten().collect();
        roots.sort();
        roots.dedup();
        let root_ids: HashMap<_, _> = roots
            .into_iter()
            .enumerate()
            .map(|(id, root)| (root, id))
            .collect();
        self.workspace_membership = self
            .contexts
            .iter()
            .filter(|(_, contexts)| contexts.len() == 1)
            .filter_map(|(file, _)| {
                let roots = self.roots.get(file).filter(|roots| roots.len() == 1)?;
                let id = root_ids.get(roots.iter().next()?)?;
                Some((file.clone(), *id))
            })
            .collect();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn previous_membership(index: &WorkspaceIndex, left: &Path, right: &Path) -> bool {
        left == right
            || (index
                .contexts
                .get(left)
                .is_some_and(|contexts| contexts.len() == 1)
                && index
                    .contexts
                    .get(right)
                    .is_some_and(|contexts| contexts.len() == 1)
                && index
                    .roots
                    .get(left)
                    .zip(index.roots.get(right))
                    .is_some_and(|(left, right)| {
                        left.len() == 1 && right.len() == 1 && !left.is_disjoint(right)
                    }))
    }

    #[test]
    fn cached_membership_preserves_missing_ambiguous_and_distinct_roots() {
        let mut index = WorkspaceIndex::default();
        let root_sets = [
            vec![],
            vec!["root_a"],
            vec!["root_b"],
            vec!["root_a", "root_b"],
        ];
        let mut files = vec![PathBuf::from("absent")];
        for context_count in 0..=2 {
            for (root_case, roots) in root_sets.iter().enumerate() {
                let file = PathBuf::from(format!("file_{context_count}_{root_case}"));
                let contexts = (0..context_count)
                    .map(|i| Context {
                        file: file.clone(),
                        module: vec![i.to_string()],
                    })
                    .collect();
                index.contexts.insert(file.clone(), contexts);
                index
                    .roots
                    .insert(file.clone(), roots.iter().map(PathBuf::from).collect());
                files.push(file);
            }
        }
        let context_only = PathBuf::from("context_only");
        index.contexts.insert(
            context_only.clone(),
            vec![Context {
                file: context_only.clone(),
                module: vec![],
            }],
        );
        files.push(context_only);
        let root_only = PathBuf::from("root_only");
        index
            .roots
            .insert(root_only.clone(), HashSet::from([PathBuf::from("root_a")]));
        files.push(root_only);
        index.index_workspace_membership();
        for left in &files {
            for right in &files {
                assert_eq!(
                    index.same_workspace(left, right),
                    previous_membership(&index, left, right),
                    "{left:?} vs {right:?}"
                );
            }
        }
    }
}
