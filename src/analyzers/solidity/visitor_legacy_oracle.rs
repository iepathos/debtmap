use tree_sitter::Node;

use super::*;
use crate::analyzers::solidity::effects::{modifier_bodies_by_name, state_variable_names};
use crate::analyzers::solidity::types::{ContractInfo, ContractKind};

pub(super) fn legacy_analyze_ast(
    ast: &SolidityAst,
    config: &SolidityLanguageConfig,
) -> SolidityAnalysis {
    let root = ast.tree.root_node();
    let mut analysis = SolidityAnalysis {
        is_test_file: crate::analyzers::solidity::test_detection::is_test_context(
            &ast.path,
            &ast.source,
            None,
        ),
        has_floating_pragma: crate::analyzers::solidity::test_detection::has_floating_pragma(
            &ast.source,
        ),
        ..Default::default()
    };
    collect_contracts(root, ast, &mut analysis.contracts);
    let modifiers = modifier_bodies_by_name(root, &ast.source);
    collect_callables(
        root,
        ast,
        config,
        None,
        &analysis.contracts,
        &modifiers,
        &mut analysis.functions,
    );
    analysis
        .functions
        .sort_by(|a, b| a.line.cmp(&b.line).then_with(|| a.name.cmp(&b.name)));
    analysis
}

pub(super) fn count_advisory_callables(node: Node) -> usize {
    let mut count = usize::from(matches!(
        node.kind(),
        "function_definition" | "modifier_definition" | "constructor" | "fallback" | "receive"
    ));
    walk_children(node, |child| count += count_advisory_callables(child));
    count
}

fn collect_contracts(node: Node, ast: &SolidityAst, contracts: &mut Vec<ContractInfo>) {
    if let Some(contract) = contract_from_node(node, ast) {
        contracts.push(contract);
    }
    walk_children(node, |child| collect_contracts(child, ast, contracts));
}

fn contract_from_node(node: Node, ast: &SolidityAst) -> Option<ContractInfo> {
    let (kind, name) = match node.kind() {
        "contract_declaration" => (ContractKind::Contract, node.child_by_field_name("name")?),
        "interface_declaration" => (ContractKind::Interface, node.child_by_field_name("name")?),
        "library_declaration" => (ContractKind::Library, node.child_by_field_name("name")?),
        _ => return None,
    };
    Some(ContractInfo {
        name: node_text(&name, &ast.source).to_string(),
        kind,
        base_classes: inheritance_names(node, ast),
        state_variable_count: count_nodes(node, "state_variable_declaration"),
        function_count: count_visitor_callables(node),
        state_variables: state_variable_names(node, &ast.source),
    })
}

fn collect_callables<'tree>(
    node: Node<'tree>,
    ast: &SolidityAst,
    config: &SolidityLanguageConfig,
    contract_name: Option<String>,
    contracts: &[ContractInfo],
    modifiers: &std::collections::HashMap<String, Node<'tree>>,
    functions: &mut Vec<SolidityFunction>,
) {
    let current_contract = contract_name.or_else(|| ancestor_contract_name(node, ast));
    if let Some(function) = callable_from_node(
        node,
        ast,
        config,
        current_contract.clone(),
        contracts,
        modifiers,
    ) {
        functions.push(function);
    }
    let next_contract = next_contract(node, ast, current_contract);
    walk_children(node, |child| {
        collect_callables(
            child,
            ast,
            config,
            next_contract.clone(),
            contracts,
            modifiers,
            functions,
        )
    });
}

fn next_contract(node: Node, ast: &SolidityAst, current: Option<String>) -> Option<String> {
    if !is_contract_node(node) {
        return current;
    }
    node.child_by_field_name("name")
        .map(|name| node_text(&name, &ast.source).to_string())
        .or(current)
}

fn ancestor_contract_name(node: Node, ast: &SolidityAst) -> Option<String> {
    let mut current = node.parent();
    while let Some(parent) = current {
        if is_contract_node(parent)
            && let Some(name) = parent.child_by_field_name("name")
        {
            return Some(node_text(&name, &ast.source).to_string());
        }
        current = parent.parent();
    }
    None
}

fn inheritance_names(node: Node, ast: &SolidityAst) -> Vec<String> {
    let mut names = Vec::new();
    walk_children(node, |child| {
        if child.kind() == "inheritance_specifier"
            && let Some(name) = child.child_by_field_name("name")
        {
            names.push(node_text(&name, &ast.source).to_string());
        }
    });
    names
}

fn count_nodes(node: Node, kind: &str) -> usize {
    let mut count = usize::from(node.kind() == kind);
    walk_children(node, |child| count += count_nodes(child, kind));
    count
}

fn count_visitor_callables(node: Node) -> usize {
    let mut count = usize::from(matches!(
        node.kind(),
        "function_definition"
            | "modifier_definition"
            | "constructor"
            | "constructor_definition"
            | "fallback"
            | "receive"
    ));
    walk_children(node, |child| count += count_visitor_callables(child));
    count
}

fn is_contract_node(node: Node) -> bool {
    matches!(
        node.kind(),
        "contract_declaration" | "interface_declaration" | "library_declaration"
    )
}

fn walk_children(node: Node, mut visit: impl FnMut(Node)) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        visit(child);
    }
}
