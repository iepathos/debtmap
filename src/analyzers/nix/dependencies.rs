use crate::analyzers::nix::parser::node_text;
use crate::analyzers::nix::visitor::{binding_name, binding_value, unwrap_parens};
use crate::analyzers::syntax_tree::children;
use crate::core::ast::NixAst;
use crate::core::{Dependency, DependencyKind};
use tree_sitter::Node;

pub fn extract_dependencies(ast: &NixAst) -> Vec<Dependency> {
    let mut names = collect_dependencies(ast.tree.root_node(), ast, "");
    names.sort();
    names.dedup();
    names.into_iter().map(import_dependency).collect()
}

fn collect_dependencies(node: Node, ast: &NixAst, prefix: &str) -> Vec<String> {
    let current = path_dependency(node, ast).or_else(|| flake_input_url(node, ast, prefix));
    let next_prefix = binding_name(node, ast, prefix)
        .filter(|_| node.kind() == "binding")
        .unwrap_or_else(|| prefix.to_string());

    current
        .into_iter()
        .chain(
            children(node)
                .into_iter()
                .flat_map(|child| collect_dependencies(child, ast, &next_prefix)),
        )
        .collect()
}

fn path_dependency(node: Node, ast: &NixAst) -> Option<String> {
    if node.kind() != "apply_expression" {
        return None;
    }

    let function = node.child_by_field_name("function")?;
    if !is_import_function(function, ast) {
        return None;
    }

    let argument = unwrap_parens(node.child_by_field_name("argument")?);
    matches!(
        argument.kind(),
        "path_expression" | "spath_expression" | "hpath_expression"
    )
    .then(|| node_text(&argument, &ast.source).to_string())
}

fn is_import_function(function: Node, ast: &NixAst) -> bool {
    matches!(
        function_name(function, ast).as_deref(),
        Some("import" | "callPackage")
    )
}

fn function_name(function: Node, ast: &NixAst) -> Option<String> {
    match function.kind() {
        "variable_expression" => Some(node_text(&function, &ast.source).to_string()),
        "select_expression" => {
            let path = function.child_by_field_name("attrpath")?;
            node_text(&path, &ast.source)
                .rsplit('.')
                .next()
                .map(str::to_string)
        }
        _ => None,
    }
}

fn flake_input_url(node: Node, ast: &NixAst, prefix: &str) -> Option<String> {
    if node.kind() != "binding" {
        return None;
    }

    let path = binding_name(node, ast, prefix)?;
    let is_input_url = path.starts_with("inputs.") && path.ends_with(".url");
    let value = binding_value(node).filter(|_| is_input_url)?;
    plain_string(value, ast)
}

fn plain_string(node: Node, ast: &NixAst) -> Option<String> {
    let is_plain = node.kind() == "string_expression"
        && children(node)
            .iter()
            .all(|child| child.kind() != "interpolation");
    is_plain.then(|| node_text(&node, &ast.source).trim_matches('"').to_string())
}

fn import_dependency(name: String) -> Dependency {
    Dependency {
        name,
        kind: DependencyKind::Import,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzers::nix::parser::parse_source;
    use std::path::PathBuf;

    fn names(source: &str) -> Vec<String> {
        let ast = parse_source(source, &PathBuf::from("flake.nix")).unwrap();
        extract_dependencies(&ast)
            .into_iter()
            .map(|dep| dep.name)
            .collect()
    }

    #[test]
    fn test_import_paths() {
        let found = names("{ a = import ./x.nix; b = import <nixpkgs> {}; c = import (./y.nix); }");
        assert_eq!(found, vec!["./x.nix", "./y.nix", "<nixpkgs>"]);
    }

    #[test]
    fn test_import_home_path() {
        assert_eq!(names("import ~/foo.nix"), vec!["~/foo.nix"]);
    }

    #[test]
    fn test_call_package() {
        let found = names("{ a = callPackage ./pkg {}; b = pkgs.callPackage ./other { x = 1; }; }");
        assert_eq!(found, vec!["./other", "./pkg"]);
    }

    #[test]
    fn test_flake_input_urls() {
        let source = r#"{
  inputs.nixpkgs.url = "github:NixOS/nixpkgs";
  inputs = {
    utils = { url = "github:numtide/flake-utils"; };
    other.url = "git+https://example.com/repo";
  };
  outputs = { self }: { url = "ignored"; };
}"#;
        assert_eq!(
            names(source),
            vec![
                "git+https://example.com/repo",
                "github:NixOS/nixpkgs",
                "github:numtide/flake-utils",
            ]
        );
    }

    #[test]
    fn test_dedup_and_ignore_non_path_imports() {
        let found = names("{ a = import ./x.nix; b = import ./x.nix; c = import foo; }");
        assert_eq!(found, vec!["./x.nix"]);
    }

    #[test]
    fn test_dependency_kind_is_import() {
        let ast = parse_source("import ./x.nix", &PathBuf::from("a.nix")).unwrap();
        let deps = extract_dependencies(&ast);

        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].kind, DependencyKind::Import);
    }
}
