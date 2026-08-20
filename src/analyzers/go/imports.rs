//! Go module and import syntax used by cross-file call resolution.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub(crate) fn package_import_path(directory: &Path) -> Option<String> {
    let module = nearest_module(directory)?;
    let relative = directory.strip_prefix(&module.root).ok()?;
    Some(join_import_path(&module.path, relative))
}

pub(crate) fn import_aliases_for_file(path: &Path) -> HashMap<String, String> {
    std::fs::read_to_string(path)
        .map(|source| import_aliases(&source))
        .unwrap_or_default()
}

pub(crate) fn import_aliases(source: &str) -> HashMap<String, String> {
    import_specs(source)
        .into_iter()
        .filter_map(|spec| alias_and_path(&spec))
        .filter(|(alias, _)| alias != "." && alias != "_")
        .collect()
}

#[derive(Debug, Clone)]
struct GoModule {
    root: PathBuf,
    path: String,
}

fn nearest_module(directory: &Path) -> Option<GoModule> {
    directory.ancestors().find_map(module_at)
}

fn module_at(directory: &Path) -> Option<GoModule> {
    let source = std::fs::read_to_string(directory.join("go.mod")).ok()?;
    parse_module_path(&source).map(|path| GoModule {
        root: directory.to_path_buf(),
        path,
    })
}

pub(crate) fn parse_module_path(source: &str) -> Option<String> {
    source.lines().find_map(|line| {
        let line = line.split("//").next().unwrap_or("").trim();
        line.strip_prefix("module ")
            .map(str::trim)
            .filter(|path| !path.is_empty())
            .map(str::to_string)
    })
}

pub(crate) fn join_import_path(module_path: &str, relative: &Path) -> String {
    let relative = relative
        .components()
        .filter_map(|component| component.as_os_str().to_str())
        .collect::<Vec<_>>()
        .join("/");
    match relative.is_empty() {
        true => module_path.to_string(),
        false => format!("{module_path}/{relative}"),
    }
}

fn alias_and_path(spec: &str) -> Option<(String, String)> {
    let path = import_path(spec)?;
    let quote_index = spec.find('"').or_else(|| spec.find('`'))?;
    let prefix = spec[..quote_index].trim();
    let alias = prefix
        .split_whitespace()
        .last()
        .map(str::to_string)
        .or_else(|| path.rsplit('/').next().map(str::to_string))?;
    Some((alias, path))
}

fn import_path(spec: &str) -> Option<String> {
    let start = spec.find('"').or_else(|| spec.find('`'))?;
    let quote = spec.as_bytes()[start] as char;
    let rest = &spec[start + 1..];
    let end = rest.find(quote)?;
    Some(rest[..end].to_string())
}

fn import_specs(source: &str) -> Vec<String> {
    source
        .lines()
        .fold(ImportScan::default(), |scan, line| scan.next(line))
        .specs
}

#[derive(Debug, Clone, Default)]
struct ImportScan {
    in_block: bool,
    specs: Vec<String>,
}

impl ImportScan {
    fn next(mut self, line: &str) -> Self {
        let trimmed = line.trim();
        if self.in_block {
            self.in_block = !trimmed.starts_with(')');
            if self.in_block {
                self.specs.push(trimmed.to_string());
            }
        } else if let Some(rest) = trimmed.strip_prefix("import ") {
            self.in_block = rest.trim_start().starts_with('(');
            if !self.in_block {
                self.specs.push(rest.trim().to_string());
            }
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_default_and_explicit_aliases() {
        let source = r#"
            import "example.com/project/lib"
            import (
                helper "example.com/project/helpers"
                _ "example.com/project/sideeffect"
            )
        "#;

        let aliases = import_aliases(source);

        assert_eq!(aliases.get("lib"), Some(&"example.com/project/lib".into()));
        assert_eq!(
            aliases.get("helper"),
            Some(&"example.com/project/helpers".into())
        );
        assert!(!aliases.contains_key("_"));
    }
}
