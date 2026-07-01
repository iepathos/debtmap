use crate::analyzers::solidity::parser::node_text;
use crate::core::ast::SolidityAst;
use crate::core::{Dependency, DependencyKind};
use std::path::{Component, Path, PathBuf};
use tree_sitter::Node;

pub fn extract_dependencies(ast: &SolidityAst) -> Vec<Dependency> {
    let mut dependencies = Vec::new();
    collect_dependencies(ast.tree.root_node(), ast, &mut dependencies);
    dependencies.sort_by(|a, b| {
        a.name
            .cmp(&b.name)
            .then_with(|| format!("{:?}", a.kind).cmp(&format!("{:?}", b.kind)))
    });
    dependencies.dedup_by(|a, b| a.name == b.name && a.kind == b.kind);
    dependencies
}

fn collect_dependencies(node: Node, ast: &SolidityAst, dependencies: &mut Vec<Dependency>) {
    match node.kind() {
        "import_directive" => dependencies.extend(import_dependencies(node, ast)),
        "inheritance_specifier" => {
            if let Some(name) = inheritance_name(node, ast) {
                dependencies.push(Dependency {
                    name,
                    kind: DependencyKind::Inheritance,
                });
            }
        }
        _ => {}
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_dependencies(child, ast, dependencies);
    }
}

fn import_dependencies(node: Node, ast: &SolidityAst) -> Vec<Dependency> {
    let mut imports = Vec::new();
    collect_import_paths(node, ast, &mut imports);
    imports
        .into_iter()
        .map(|name| resolve_import_name(&name, &ast.path))
        .map(|name| Dependency {
            name,
            kind: DependencyKind::Import,
        })
        .collect()
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
        .to_string()
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

fn collect_import_paths(node: Node, ast: &SolidityAst, paths: &mut Vec<String>) {
    if node.kind() == "import_path" || node.kind() == "string" {
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

fn inheritance_name(node: Node, ast: &SolidityAst) -> Option<String> {
    node.child_by_field_name("name")
        .map(|name| node_text(&name, &ast.source).to_string())
        .or_else(|| {
            let text = node_text(&node, &ast.source);
            text.split_whitespace().last().map(str::to_string)
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzers::solidity::parser::parse_source;
    use std::path::PathBuf;

    #[test]
    fn test_extract_imports_and_inheritance() {
        let source = r#"pragma solidity ^0.8.0;
import "./Token.sol";
import {SafeMath} from "lib/SafeMath.sol";

contract Vault is Token, Ownable {
    function deposit() public {}
}
"#;
        let ast = parse_source(source, &PathBuf::from("Vault.sol")).unwrap();
        let deps = extract_dependencies(&ast);

        assert!(
            deps.iter()
                .any(|dep| dep.kind == DependencyKind::Import && dep.name.contains("Token.sol"))
        );
        assert!(
            deps.iter()
                .any(|dep| dep.kind == DependencyKind::Inheritance && dep.name == "Token")
        );
    }

    #[test]
    fn test_resolves_relative_imports_against_file_path() {
        let source = r#"pragma solidity ^0.8.0;
import "../interfaces/IERC20.sol";
contract Vault {}
"#;
        let ast = parse_source(source, &PathBuf::from("contracts/vault/Vault.sol")).unwrap();
        let deps = extract_dependencies(&ast);

        assert!(deps.iter().any(|dep| {
            dep.kind == DependencyKind::Import
                && dep.name.ends_with("contracts/interfaces/IERC20.sol")
        }));
    }

    #[test]
    fn test_extracts_package_imports() {
        let source = r#"pragma solidity 0.8.20;
import "@thirdparty/contracts/Token.sol";
contract Vault {}
"#;
        let ast = parse_source(source, &PathBuf::from("src/Vault.sol")).unwrap();
        let deps = extract_dependencies(&ast);

        assert!(deps.iter().any(|dep| {
            dep.kind == DependencyKind::Import && dep.name == "@thirdparty/contracts/Token.sol"
        }));
    }
}
