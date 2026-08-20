//! Stable finding identifiers for JSON v4.

use super::types::UnifiedDebtItemOutput;
use crate::priority::call_graph::FunctionId;
use sha2::{Digest, Sha256};
use std::path::{Component, Path, PathBuf};

pub(super) fn assign_finding_ids(items: &mut [UnifiedDebtItemOutput], root: Option<&Path>) {
    for item in items {
        match item {
            UnifiedDebtItemOutput::Function(function) => {
                function.finding_id = Some(stable_finding_id(
                    root,
                    &function.location.file,
                    function.location.function.as_deref(),
                    function.debt_type.display_name(),
                ));
            }
            UnifiedDebtItemOutput::File(file) => {
                file.finding_id = Some(stable_finding_id(
                    root,
                    &file.location.file,
                    None,
                    file.debt_type
                        .as_ref()
                        .map(|debt| debt.display_name())
                        .unwrap_or("file"),
                ));
            }
        }
    }
}

fn stable_finding_id(
    root: Option<&Path>,
    file: &str,
    function: Option<&str>,
    debt_kind: &str,
) -> String {
    let path = stable_path(root, Path::new(file));
    let symbol =
        function.map(|name| FunctionId::new(path.clone(), name.to_string(), 0).canonical_symbol());
    let mut digest = Sha256::new();
    hash_part(&mut digest, path.to_string_lossy().as_bytes());
    if let Some(symbol) = symbol {
        hash_part(&mut digest, symbol.language.to_string().as_bytes());
        hash_part(&mut digest, symbol.module.as_bytes());
        hash_part(
            &mut digest,
            symbol.owner.as_deref().unwrap_or("").as_bytes(),
        );
        hash_part(&mut digest, symbol.name.as_bytes());
        hash_part(
            &mut digest,
            symbol.signature.as_deref().unwrap_or("").as_bytes(),
        );
    }
    hash_part(&mut digest, debt_kind.as_bytes());
    format!("dm4_{:x}", digest.finalize())
}

fn hash_part(digest: &mut Sha256, value: &[u8]) {
    digest.update(value.len().to_le_bytes());
    digest.update(value);
}

fn stable_path(root: Option<&Path>, file: &Path) -> PathBuf {
    let relative = root
        .and_then(|root| file.strip_prefix(root).ok())
        .unwrap_or(file);
    relative
        .components()
        .fold(PathBuf::new(), |mut path, component| {
            match component {
                Component::CurDir => {}
                Component::ParentDir => {
                    path.pop();
                }
                _ => path.push(component.as_os_str()),
            }
            path
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finding_id_is_independent_of_project_root() {
        let first = stable_finding_id(
            Some(Path::new("/first/project")),
            "/first/project/src/lib.rs",
            Some("Worker::run"),
            "Complexity",
        );
        let second = stable_finding_id(
            Some(Path::new("/other/project")),
            "/other/project/src/lib.rs",
            Some("Worker::run"),
            "Complexity",
        );

        assert_eq!(first, second);
    }

    #[test]
    fn debt_kind_disambiguates_findings_for_one_symbol() {
        let complexity = stable_finding_id(None, "src/lib.rs", Some("run"), "Complexity");
        let dead_code = stable_finding_id(None, "src/lib.rs", Some("run"), "Dead Code");

        assert_ne!(complexity, dead_code);
    }
}
