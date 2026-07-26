use tree_sitter::Node;

use crate::analyzers::solidity::parser::node_text;
use crate::analyzers::solidity::types::SolidityFunctionSignature;

pub(super) fn signature_from_function(node: Node, source: &str) -> SolidityFunctionSignature {
    let return_type = node.child_by_field_name("return_type");
    SolidityFunctionSignature {
        params: parameter_names(node, source, return_type),
        return_slots: return_slot_count(return_type),
    }
}

fn parameter_names(function: Node, source: &str, return_type: Option<Node>) -> Vec<String> {
    let mut names = Vec::new();
    let excluded = [function.child_by_field_name("body"), return_type]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
    collect_parameter_names(function, source, &excluded, &mut names);
    names
}

fn collect_parameter_names(node: Node, source: &str, excluded: &[Node], names: &mut Vec<String>) {
    if excluded.contains(&node) {
        return;
    }
    if node.kind() == "parameter"
        && let Some(name) = node.child_by_field_name("name")
    {
        names.push(node_text(&name, source).to_string());
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_parameter_names(child, source, excluded, names);
    }
}

fn return_slot_count(return_type: Option<Node>) -> usize {
    let Some(return_type) = return_type else {
        return 0;
    };
    let count = count_return_parameters(return_type);
    if count > 0 {
        count
    } else {
        usize::from(has_child_kind(return_type, "type_name"))
    }
}

fn count_return_parameters(node: Node) -> usize {
    let own = usize::from(matches!(node.kind(), "parameter" | "return_parameter"));
    let mut cursor = node.walk();
    own + node
        .children(&mut cursor)
        .map(count_return_parameters)
        .sum::<usize>()
}

fn has_child_kind(node: Node, kind: &str) -> bool {
    let mut cursor = node.walk();
    node.children(&mut cursor).any(|child| child.kind() == kind)
}
