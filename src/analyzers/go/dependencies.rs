use crate::analyzers::go::parser::node_text;
use crate::core::ast::GoAst;
use crate::core::{Dependency, DependencyKind};
use tree_sitter::Node;

pub fn extract_dependencies(ast: &GoAst) -> Vec<Dependency> {
    let mut dependencies = Vec::new();
    collect_imports(ast.tree.root_node(), ast, &mut dependencies);
    dependencies.sort_by(|a, b| a.name.cmp(&b.name));
    dependencies.dedup_by(|a, b| a.name == b.name);
    dependencies
}

fn collect_imports(node: Node, ast: &GoAst, dependencies: &mut Vec<Dependency>) {
    if node.kind() == "import_declaration" {
        dependencies.extend(import_paths(node, ast).into_iter().map(import_dependency));
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_imports(child, ast, dependencies);
    }
}

fn import_paths(node: Node, ast: &GoAst) -> Vec<String> {
    let mut paths = Vec::new();
    collect_string_literals(node, ast, &mut paths);
    paths
}

fn collect_string_literals(node: Node, ast: &GoAst, paths: &mut Vec<String>) {
    if matches!(
        node.kind(),
        "interpreted_string_literal" | "raw_string_literal"
    ) {
        paths.push(unquote(node_text(&node, &ast.source)));
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_string_literals(child, ast, paths);
    }
}

fn unquote(text: &str) -> String {
    text.trim_matches(|c| c == '"' || c == '`').to_string()
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
    use crate::analyzers::go::parser::parse_source;
    use std::path::PathBuf;

    #[test]
    fn test_extract_dependencies() {
        let source = r#"package main

import (
    "context"
    alias "net/http"
)
"#;
        let ast = parse_source(source, &PathBuf::from("main.go")).unwrap();
        let dependencies = extract_dependencies(&ast);

        assert_eq!(dependencies.len(), 2);
        assert!(dependencies.iter().any(|dep| dep.name == "context"));
        assert!(dependencies.iter().any(|dep| dep.name == "net/http"));
    }
}
