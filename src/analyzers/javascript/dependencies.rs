use crate::core::{Dependency, DependencyKind};
use tree_sitter::Node;

pub fn extract_dependencies(node: Node, source: &str) -> Vec<Dependency> {
    let mut dependencies = Vec::new();
    visit_node_for_dependencies(node, source, &mut dependencies);
    dependencies
}

fn visit_node_for_dependencies(node: Node, source: &str, dependencies: &mut Vec<Dependency>) {
    type ExtractorFn = fn(&Node, &str, &mut Vec<Dependency>);

    static NODE_EXTRACTORS: &[(&str, ExtractorFn)] = &[
        ("import_statement", extract_import_statement),
        ("call_expression", extract_call_expression),
        ("import", extract_dynamic_import),
    ];

    if let Some((_, extractor)) = NODE_EXTRACTORS
        .iter()
        .find(|(kind, _)| *kind == node.kind())
    {
        extractor(&node, source, dependencies);
    }

    node.children(&mut node.walk())
        .for_each(|child| visit_node_for_dependencies(child, source, dependencies));
}

fn extract_import_statement(node: &Node, source: &str, dependencies: &mut Vec<Dependency>) {
    if let Some(module_name) = node
        .child_by_field_name("source")
        .and_then(|source_node| source_node.utf8_text(source.as_bytes()).ok())
    {
        dependencies.push(Dependency {
            name: clean_module_name(module_name),
            kind: DependencyKind::Import,
        });
    }
}

fn extract_call_expression(node: &Node, source: &str, dependencies: &mut Vec<Dependency>) {
    let is_require = node
        .child_by_field_name("function")
        .and_then(|func| func.utf8_text(source.as_bytes()).ok())
        .map(|name| name == "require")
        .unwrap_or(false);

    if !is_require {
        return;
    }

    if let Some(args) = node.child_by_field_name("arguments") {
        extract_string_arguments(&args, source, dependencies);
    }
}

fn extract_dynamic_import(node: &Node, source: &str, dependencies: &mut Vec<Dependency>) {
    if let Some(args) = node
        .parent()
        .filter(|parent| parent.kind() == "call_expression")
        .and_then(|parent| parent.child_by_field_name("arguments"))
    {
        extract_string_arguments(&args, source, dependencies);
    }
}

fn extract_string_arguments(args_node: &Node, source: &str, dependencies: &mut Vec<Dependency>) {
    args_node
        .children(&mut args_node.walk())
        .filter(|child| child.kind() == "string")
        .filter_map(|child| child.utf8_text(source.as_bytes()).ok())
        .for_each(|module_name| {
            dependencies.push(Dependency {
                name: clean_module_name(module_name),
                kind: DependencyKind::Import,
            })
        });
}

fn clean_module_name(module_name: &str) -> String {
    module_name
        .trim_matches(|c| c == '"' || c == '\'' || c == '`')
        .to_string()
}
