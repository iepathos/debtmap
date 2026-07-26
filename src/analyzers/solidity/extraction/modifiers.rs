use std::collections::HashMap;

use tree_sitter::Node;

use crate::analyzers::solidity::parser::node_text;
use crate::core::ast::SolidityAst;

pub(super) fn collect_modifier<'tree>(
    node: Node<'tree>,
    ast: &SolidityAst,
    modifiers: &mut HashMap<String, Node<'tree>>,
) {
    if node.kind() != "modifier_definition" {
        return;
    }
    if let (Some(name), Some(body)) = (
        node.child_by_field_name("name"),
        node.child_by_field_name("body"),
    ) {
        modifiers.insert(node_text(&name, &ast.source).to_string(), body);
    }
}
