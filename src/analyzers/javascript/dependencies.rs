use crate::core::{Dependency, DependencyKind};
use tree_sitter::Node;

pub fn extract_dependencies(node: Node, source: &str) -> Vec<Dependency> {
    let mut dependencies = Vec::new();
    visit_node_for_dependencies(node, source, &mut dependencies);
    dependencies
}

fn visit_node_for_dependencies(node: Node, source: &str, dependencies: &mut Vec<Dependency>) {
    match node.kind() {
        // ES6 imports
        "import_statement" => {
            if let Some(source_node) = node.child_by_field_name("source") {
                if let Ok(module_name) = source_node.utf8_text(source.as_bytes()) {
                    dependencies.push(Dependency {
                        name: module_name
                            .trim_matches(|c| c == '"' || c == '\'' || c == '`')
                            .to_string(),
                        kind: DependencyKind::Import,
                    });
                }
            }
        }
        // CommonJS require
        "call_expression" => {
            if let Some(function_node) = node.child_by_field_name("function") {
                if let Ok(func_name) = function_node.utf8_text(source.as_bytes()) {
                    if func_name == "require" {
                        if let Some(args_node) = node.child_by_field_name("arguments") {
                            for child in args_node.children(&mut args_node.walk()) {
                                if child.kind() == "string" {
                                    if let Ok(module_name) = child.utf8_text(source.as_bytes()) {
                                        dependencies.push(Dependency {
                                            name: module_name
                                                .trim_matches(|c| c == '"' || c == '\'' || c == '`')
                                                .to_string(),
                                            kind: DependencyKind::Import,
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        // Dynamic imports
        "import" => {
            // Handle dynamic import() expressions
            if let Some(parent) = node.parent() {
                if parent.kind() == "call_expression" {
                    if let Some(args_node) = parent.child_by_field_name("arguments") {
                        for child in args_node.children(&mut args_node.walk()) {
                            if child.kind() == "string" {
                                if let Ok(module_name) = child.utf8_text(source.as_bytes()) {
                                    dependencies.push(Dependency {
                                        name: module_name
                                            .trim_matches(|c| c == '"' || c == '\'' || c == '`')
                                            .to_string(),
                                        kind: DependencyKind::Import,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
        _ => {}
    }

    for child in node.children(&mut node.walk()) {
        visit_node_for_dependencies(child, source, dependencies);
    }
}
