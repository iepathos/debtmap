use std::path::{Component, Path, PathBuf};

use tree_sitter::Node;

use crate::analyzers::solidity::parser::node_text;
use crate::core::ast::SolidityAst;
use crate::core::{Dependency, DependencyKind};

pub(super) fn import_dependencies(node: Node, ast: &SolidityAst) -> Vec<Dependency> {
    let mut paths = Vec::new();
    collect_import_paths(node, ast, &mut paths);
    paths
        .into_iter()
        .map(|name| resolve_import_name(&name, &ast.path))
        .map(|name| Dependency {
            name,
            kind: DependencyKind::Import,
        })
        .collect()
}

pub(super) fn inheritance_name(node: Node, ast: &SolidityAst) -> Option<String> {
    node.child_by_field_name("name")
        .map(|name| node_text(&name, &ast.source).to_string())
        .or_else(|| {
            node_text(&node, &ast.source)
                .split_whitespace()
                .last()
                .map(str::to_string)
        })
}

pub(super) fn sort_dependencies(dependencies: &mut Vec<Dependency>) {
    dependencies.sort_by(|a, b| {
        a.name
            .cmp(&b.name)
            .then_with(|| format!("{:?}", a.kind).cmp(&format!("{:?}", b.kind)))
    });
    dependencies.dedup_by(|a, b| a.name == b.name && a.kind == b.kind);
}

fn collect_import_paths(node: Node, ast: &SolidityAst, paths: &mut Vec<String>) {
    if matches!(node.kind(), "import_path" | "string") {
        let text = node_text(&node, &ast.source).trim_matches('"').to_string();
        if !text.is_empty() {
            paths.push(text);
        }
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_import_paths(child, ast, paths);
    }
}

fn resolve_import_name(import: &str, file_path: &Path) -> String {
    if !import.starts_with('.') {
        return import.to_string();
    }
    file_path
        .parent()
        .map(|parent| normalize_path(parent.join(import)))
        .unwrap_or_else(|| PathBuf::from(import))
        .to_string_lossy()
        .replace(std::path::MAIN_SEPARATOR, "/")
}

fn normalize_path(path: PathBuf) -> PathBuf {
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
