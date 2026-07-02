//! Foundry remapping parsing and Solidity import path resolution.

use std::collections::{HashMap, HashSet};
use std::path::{Component, Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Remapping {
    pub context: String,
    pub target: String,
}

pub fn parse_remapping_line(line: &str) -> Option<Remapping> {
    let line = line.split('#').next()?.trim();
    if line.is_empty() {
        return None;
    }
    let (context, target) = line.split_once('=')?;
    let context = context.trim();
    let target = target.trim();
    if context.is_empty() || target.is_empty() {
        return None;
    }
    Some(Remapping {
        context: normalize_prefix(context),
        target: normalize_target(target),
    })
}

pub fn parse_remappings(source: &str) -> Vec<Remapping> {
    source.lines().filter_map(parse_remapping_line).collect()
}

pub fn apply_remappings(import: &str, remappings: &[Remapping]) -> String {
    let import = import.trim_matches('"');
    remappings
        .iter()
        .filter(|remapping| import.starts_with(&remapping.context))
        .max_by_key(|remapping| remapping.context.len())
        .map(|remapping| {
            let suffix = import.strip_prefix(&remapping.context).unwrap_or("");
            format!("{}{}", remapping.target, suffix)
        })
        .unwrap_or_else(|| import.to_string())
}

pub fn normalize_project_path(path: PathBuf) -> PathBuf {
    path.components()
        .fold(PathBuf::new(), |mut normalized, part| {
            match part {
                Component::ParentDir => {
                    normalized.pop();
                }
                Component::CurDir => {}
                _ => normalized.push(part.as_os_str()),
            }
            normalized
        })
}

pub fn find_nearest_project_root(file_path: &Path) -> Option<PathBuf> {
    file_path
        .parent()?
        .ancestors()
        .find(|directory| {
            directory.join("remappings.txt").is_file() || directory.join("foundry.toml").is_file()
        })
        .map(Path::to_path_buf)
}

pub fn load_remappings_from_root(project_root: &Path) -> Vec<Remapping> {
    let remappings_path = project_root.join("remappings.txt");
    std::fs::read_to_string(remappings_path)
        .map(|source| parse_remappings(&source))
        .unwrap_or_default()
}

pub fn resolve_relative_import(import: &str, file_path: &Path) -> PathBuf {
    let import = import.trim_matches('"');
    if import.starts_with('.') {
        file_path
            .parent()
            .map(|parent| normalize_project_path(parent.join(import)))
            .unwrap_or_else(|| PathBuf::from(import))
    } else {
        PathBuf::from(import)
    }
}

#[derive(Debug)]
pub struct SolidityImportResolver {
    roots: HashMap<PathBuf, ProjectContext>,
}

#[derive(Debug)]
struct ProjectContext {
    remappings: Vec<Remapping>,
    analyzed_files: HashSet<PathBuf>,
}

impl SolidityImportResolver {
    pub fn from_analyzed_files(files: &[PathBuf]) -> Self {
        let groups = group_files_by_project_root(files);
        let roots = groups
            .into_iter()
            .map(|(root, group_files)| {
                let analyzed_files = group_files
                    .iter()
                    .map(|file| relative_to_root(file, &root))
                    .collect();
                let context = ProjectContext {
                    remappings: load_remappings_from_root(&root),
                    analyzed_files,
                };
                (root, context)
            })
            .collect();

        Self { roots }
    }

    pub fn resolve(&self, import: &str, source_file: &Path) -> String {
        let import = import.trim();
        let root = self.project_root(source_file);
        let context = self.roots.get(&root);

        if let Some(resolved) = self.match_analyzed(import, context) {
            return resolved;
        }

        let candidates = import_candidates(import, source_file, &root, context);
        for candidate in candidates {
            if let Some(resolved) = self.match_analyzed_path(&candidate, context) {
                return resolved;
            }
        }

        import.to_string()
    }

    fn project_root(&self, source_file: &Path) -> PathBuf {
        self.roots
            .keys()
            .find(|root| source_file.starts_with(root))
            .cloned()
            .or_else(|| find_nearest_project_root(source_file))
            .unwrap_or_else(|| {
                source_file
                    .parent()
                    .map(Path::to_path_buf)
                    .unwrap_or_else(|| PathBuf::from("."))
            })
    }

    fn match_analyzed(&self, import: &str, context: Option<&ProjectContext>) -> Option<String> {
        let path = PathBuf::from(import);
        self.match_analyzed_path(&path, context)
    }

    fn match_analyzed_path(
        &self,
        candidate: &Path,
        context: Option<&ProjectContext>,
    ) -> Option<String> {
        let normalized = normalize_project_path(candidate.to_path_buf());
        context.and_then(|ctx| {
            ctx.analyzed_files
                .iter()
                .find(|analyzed| paths_equivalent(&normalized, analyzed, candidate))
                .map(|path| to_slash_path(path))
        })
    }
}

fn to_slash_path(path: &Path) -> String {
    path.to_string_lossy()
        .replace(std::path::MAIN_SEPARATOR, "/")
}

fn paths_equivalent(normalized: &Path, analyzed: &Path, candidate: &Path) -> bool {
    normalized == analyzed
        || normalized.ends_with(analyzed)
        || analyzed.ends_with(normalized)
        || candidate.ends_with(analyzed)
}

fn group_files_by_project_root(files: &[PathBuf]) -> HashMap<PathBuf, Vec<PathBuf>> {
    if files.is_empty() {
        return HashMap::new();
    }

    if let Some(root) = shared_configured_root(files) {
        return HashMap::from([(root, files.to_vec())]);
    }

    HashMap::from([(common_ancestor_root(files), files.to_vec())])
}

fn shared_configured_root(files: &[PathBuf]) -> Option<PathBuf> {
    let mut roots = HashSet::new();
    for file in files {
        if let Some(root) = find_nearest_project_root(file) {
            roots.insert(root);
        }
    }
    roots
        .into_iter()
        .find(|root| files.iter().all(|file| file.starts_with(root)))
}

fn common_ancestor_root(files: &[PathBuf]) -> PathBuf {
    let parents: Vec<PathBuf> = files
        .iter()
        .filter_map(|file| file.parent().map(Path::to_path_buf))
        .collect();

    if parents.is_empty() {
        return PathBuf::from(".");
    }

    let component_lists: Vec<Vec<_>> = parents
        .iter()
        .map(|path| path.components().collect::<Vec<_>>())
        .collect();

    let mut common = PathBuf::new();
    for index in 0..component_lists[0].len() {
        let part = &component_lists[0][index];
        if component_lists
            .iter()
            .all(|components| components.get(index) == Some(part))
        {
            common.push(part.as_os_str());
        } else {
            break;
        }
    }

    if common.as_os_str().is_empty() {
        PathBuf::from(".")
    } else {
        common
    }
}

fn relative_to_root(file: &Path, root: &Path) -> PathBuf {
    normalize_project_path(
        file.strip_prefix(root)
            .map(Path::to_path_buf)
            .unwrap_or_else(|_| file.to_path_buf()),
    )
}

fn import_candidates(
    import: &str,
    source_file: &Path,
    project_root: &Path,
    context: Option<&ProjectContext>,
) -> Vec<PathBuf> {
    let import = import.trim_matches('"');
    let remappings = context.map(|ctx| ctx.remappings.as_slice()).unwrap_or(&[]);
    let remapped = apply_remappings(import, remappings);

    let mut candidates = vec![
        relative_to_root(
            &normalize_project_path(project_root.join(&remapped)),
            project_root,
        ),
        relative_to_root(&resolve_relative_import(import, source_file), project_root),
    ];

    if !import.starts_with('.') {
        candidates.push(relative_to_root(
            &normalize_project_path(project_root.join("node_modules").join(import)),
            project_root,
        ));
        candidates.push(relative_to_root(
            &normalize_project_path(project_root.join(import)),
            project_root,
        ));
    }

    candidates.sort();
    candidates.dedup();
    candidates
}

fn normalize_prefix(prefix: &str) -> String {
    if prefix.ends_with('/') {
        prefix.to_string()
    } else {
        format!("{}/", prefix)
    }
}

fn normalize_target(target: &str) -> String {
    if target.ends_with('/') {
        target.to_string()
    } else {
        format!("{}/", target)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_parse_remapping_line() {
        assert_eq!(
            parse_remapping_line("@openzeppelin/contracts/=lib/openzeppelin-contracts/contracts/"),
            Some(Remapping {
                context: "@openzeppelin/contracts/".to_string(),
                target: "lib/openzeppelin-contracts/contracts/".to_string(),
            })
        );
    }

    #[test]
    fn test_parse_remapping_line_ignores_comments_and_blanks() {
        assert_eq!(parse_remapping_line("# comment"), None);
        assert_eq!(parse_remapping_line(""), None);
        assert_eq!(
            parse_remapping_line("forge-std/=lib/forge-std/src/ # trailing"),
            Some(Remapping {
                context: "forge-std/".to_string(),
                target: "lib/forge-std/src/".to_string(),
            })
        );
    }

    #[test]
    fn test_apply_remappings_longest_prefix_wins() {
        let remappings = parse_remappings(
            "@openzeppelin/=lib/openzeppelin/\n@openzeppelin/contracts/=lib/openzeppelin-contracts/contracts/\n",
        );
        assert_eq!(
            apply_remappings("@openzeppelin/contracts/token/ERC20/ERC20.sol", &remappings),
            "lib/openzeppelin-contracts/contracts/token/ERC20/ERC20.sol"
        );
    }

    #[test]
    fn test_apply_remappings_unmapped_import_unchanged() {
        let remappings = parse_remappings("forge-std/=lib/forge-std/src/\n");
        assert_eq!(
            apply_remappings("external/Unknown.sol", &remappings),
            "external/Unknown.sol"
        );
    }

    #[test]
    fn test_load_remappings_from_root() {
        let temp = TempDir::new().unwrap();
        fs::write(
            temp.path().join("remappings.txt"),
            "@openzeppelin/contracts/=lib/openzeppelin-contracts/contracts/\n",
        )
        .unwrap();

        let remappings = load_remappings_from_root(temp.path());
        assert_eq!(remappings.len(), 1);
        assert_eq!(remappings[0].context, "@openzeppelin/contracts/");
    }

    #[test]
    fn test_resolve_package_import_via_remapping() {
        let temp = TempDir::new().unwrap();
        fs::write(
            temp.path().join("remappings.txt"),
            "@openzeppelin/contracts/=lib/openzeppelin-contracts/contracts/\n",
        )
        .unwrap();
        let token_path = temp
            .path()
            .join("lib/openzeppelin-contracts/contracts/token/ERC20/ERC20.sol");
        fs::create_dir_all(token_path.parent().unwrap()).unwrap();
        fs::write(&token_path, "pragma solidity 0.8.20;").unwrap();

        let vault_path = temp.path().join("src/Vault.sol");
        fs::create_dir_all(vault_path.parent().unwrap()).unwrap();
        fs::write(&vault_path, "pragma solidity 0.8.20;").unwrap();

        let resolver =
            SolidityImportResolver::from_analyzed_files(&[token_path, vault_path.clone()]);
        let resolved =
            resolver.resolve("@openzeppelin/contracts/token/ERC20/ERC20.sol", &vault_path);
        assert!(resolved.ends_with("lib/openzeppelin-contracts/contracts/token/ERC20/ERC20.sol"));
    }

    #[test]
    fn test_unresolved_external_import_stays_raw() {
        let temp = TempDir::new().unwrap();
        let source = temp.path().join("src/Vault.sol");
        fs::create_dir_all(source.parent().unwrap()).unwrap();
        fs::write(&source, "pragma solidity 0.8.20;").unwrap();

        let resolver = SolidityImportResolver::from_analyzed_files(std::slice::from_ref(&source));
        assert_eq!(
            resolver.resolve("@thirdparty/contracts/Token.sol", &source),
            "@thirdparty/contracts/Token.sol"
        );
    }

    #[test]
    fn test_resolve_node_modules_fallback_without_remapping() {
        let temp = TempDir::new().unwrap();
        let token_path = temp
            .path()
            .join("node_modules/@openzeppelin/contracts/token/ERC20/ERC20.sol");
        fs::create_dir_all(token_path.parent().unwrap()).unwrap();
        fs::write(&token_path, "pragma solidity 0.8.20;").unwrap();

        let vault_path = temp.path().join("contracts/Vault.sol");
        fs::create_dir_all(vault_path.parent().unwrap()).unwrap();
        fs::write(&vault_path, "pragma solidity 0.8.20;").unwrap();

        let resolver =
            SolidityImportResolver::from_analyzed_files(&[token_path, vault_path.clone()]);
        let resolved =
            resolver.resolve("@openzeppelin/contracts/token/ERC20/ERC20.sol", &vault_path);
        assert!(resolved.contains("node_modules/@openzeppelin/contracts"));
    }
}
