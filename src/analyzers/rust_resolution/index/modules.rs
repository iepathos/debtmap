//! Source-established out-of-line module relationships.

use super::*;

impl WorkspaceIndex {
    pub(super) fn establish_modules(
        &mut self,
        known: HashSet<PathBuf>,
        edges: Vec<(PathBuf, PathBuf, Vec<String>)>,
    ) {
        let children: HashSet<_> = edges.iter().map(|(_, child, _)| child.clone()).collect();
        let mut roots = known
            .iter()
            .filter(|file| !children.contains(*file))
            .cloned()
            .collect::<Vec<_>>();
        roots.sort();
        for root in roots {
            self.assign_module(&root, &root, &[], &edges, &mut HashSet::new());
        }
        for file in known {
            self.contexts.entry(file.clone()).or_insert_with(|| {
                vec![Context {
                    file: file.clone(),
                    module: Vec::new(),
                }]
            });
            self.roots
                .entry(file.clone())
                .or_insert_with(|| HashSet::from([file]));
        }
    }

    pub(super) fn assign_module(
        &mut self,
        root: &Path,
        file: &Path,
        module: &[String],
        edges: &[(PathBuf, PathBuf, Vec<String>)],
        seen: &mut HashSet<PathBuf>,
    ) {
        if seen.len() >= EXPANSION_LIMIT || !seen.insert(file.to_path_buf()) {
            return;
        }
        let context = Context {
            file: file.to_path_buf(),
            module: module.to_vec(),
        };
        let contexts = self.contexts.entry(file.to_path_buf()).or_default();
        if !contexts.contains(&context) {
            contexts.push(context);
        }
        self.roots
            .entry(file.to_path_buf())
            .or_default()
            .insert(root.to_path_buf());
        for (_, child, relative) in edges.iter().filter(|(parent, _, _)| parent == file) {
            self.assign_module(root, child, &qualified_path(module, relative), edges, seen);
        }
        seen.remove(file);
    }
}

pub(super) fn module_edges(
    file: &Path,
    items: &[syn::Item],
    known: &HashMap<PathBuf, Vec<PathBuf>>,
) -> Vec<(PathBuf, PathBuf, Vec<String>)> {
    let parent = file.parent().unwrap_or_else(|| Path::new(""));
    let base = match file.file_stem().and_then(|name| name.to_str()) {
        Some("lib" | "main" | "mod") => parent.to_path_buf(),
        Some(stem) => parent.join(stem),
        None => parent.to_path_buf(),
    };
    let mut edges = Vec::new();
    collect_module_edges(file, items, &base, &[], known, &mut edges);
    edges
}

fn collect_module_edges(
    file: &Path,
    items: &[syn::Item],
    directory: &Path,
    prefix: &[String],
    known: &HashMap<PathBuf, Vec<PathBuf>>,
    edges: &mut Vec<(PathBuf, PathBuf, Vec<String>)>,
) {
    for item in items {
        let syn::Item::Mod(module) = item else {
            continue;
        };
        let name = module.ident.to_string();
        let relative = qualified(prefix, &name);
        if let Some((_, items)) = &module.content {
            collect_module_edges(file, items, &directory.join(&name), &relative, known, edges);
        } else {
            let custom = module.attrs.iter().find_map(module_path_attribute);
            let candidates = custom
                .map(|path| {
                    vec![if prefix.is_empty() {
                        file.parent().unwrap_or_else(|| Path::new("")).join(path)
                    } else {
                        directory.join(path)
                    }]
                })
                .unwrap_or_else(|| {
                    vec![
                        directory.join(format!("{name}.rs")),
                        directory.join(&name).join("mod.rs"),
                    ]
                });
            for candidate in candidates {
                if let Some([original]) = known
                    .get(&normalize_module_path(&candidate))
                    .map(Vec::as_slice)
                {
                    edges.push((file.to_path_buf(), original.clone(), relative.clone()));
                }
            }
        }
    }
}

pub(super) fn normalize_module_path(path: &Path) -> PathBuf {
    use std::path::Component;
    path.components()
        .fold(PathBuf::new(), |mut normalized, component| {
            match component {
                Component::CurDir => {}
                Component::ParentDir if normalized.file_name().is_some_and(|name| name != "..") => {
                    normalized.pop();
                }
                Component::ParentDir if normalized.has_root() => {}
                other => normalized.push(other.as_os_str()),
            }
            normalized
        })
}

fn module_path_attribute(attribute: &syn::Attribute) -> Option<String> {
    if !attribute.path().is_ident("path") {
        return None;
    }
    let syn::Meta::NameValue(value) = &attribute.meta else {
        return None;
    };
    let syn::Expr::Lit(value) = &value.value else {
        return None;
    };
    let syn::Lit::Str(value) = &value.lit else {
        return None;
    };
    Some(value.value())
}
