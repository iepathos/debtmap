//! Single-pass structural extraction for Solidity source files.

use std::collections::HashMap;
use tree_sitter::Node;

use crate::analyzers::solidity::parser::{node_line, node_text};
use crate::analyzers::solidity::types::{ContractInfo, SolidityFunctionSignatures};
use crate::core::ast::SolidityAst;
use crate::core::{Dependency, DependencyKind};

mod dependencies;
mod kinds;
mod modifiers;
mod signatures;
#[cfg(test)]
mod tests;

use dependencies::{import_dependencies, inheritance_name, sort_dependencies};
use kinds::{
    contract_kind, is_callable_kind, is_contract_advisory_callable, is_direct_contract_child,
};
use modifiers::collect_modifier;
use signatures::signature_from_function;

#[derive(Debug, Clone)]
pub struct ExtractedContract<'tree> {
    pub node: Node<'tree>,
    pub info: ContractInfo,
    pub advisory_function_count: usize,
}

#[derive(Debug, Clone)]
pub struct ExtractedCallable<'tree> {
    pub node: Node<'tree>,
    pub contract_name: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct SolidityExtraction<'tree> {
    pub contracts: Vec<ExtractedContract<'tree>>,
    pub callables: Vec<ExtractedCallable<'tree>>,
    pub dependencies: Vec<Dependency>,
    pub modifier_bodies: HashMap<String, Node<'tree>>,
    pub function_signatures: SolidityFunctionSignatures,
}

pub fn extract_solidity(ast: &SolidityAst) -> SolidityExtraction<'_> {
    let mut extraction = SolidityExtraction::default();
    visit_node(ast.tree.root_node(), ast, None, &mut extraction);
    sort_dependencies(&mut extraction.dependencies);
    extraction
}

fn visit_node<'tree>(
    node: Node<'tree>,
    ast: &'tree SolidityAst,
    contract_index: Option<usize>,
    extraction: &mut SolidityExtraction<'tree>,
) {
    let active_contract = register_contract(node, ast, contract_index, extraction);
    collect_node_data(node, ast, active_contract, extraction);

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        visit_node(child, ast, active_contract, extraction);
    }
}

fn register_contract<'tree>(
    node: Node<'tree>,
    ast: &SolidityAst,
    current: Option<usize>,
    extraction: &mut SolidityExtraction<'tree>,
) -> Option<usize> {
    let Some(kind) = contract_kind(node.kind()) else {
        return current;
    };
    let Some(name) = node.child_by_field_name("name") else {
        return current;
    };

    extraction.contracts.push(ExtractedContract {
        node,
        info: ContractInfo {
            name: node_text(&name, &ast.source).to_string(),
            kind,
            ..Default::default()
        },
        advisory_function_count: 0,
    });
    Some(extraction.contracts.len() - 1)
}

fn collect_node_data<'tree>(
    node: Node<'tree>,
    ast: &SolidityAst,
    contract_index: Option<usize>,
    extraction: &mut SolidityExtraction<'tree>,
) {
    match node.kind() {
        "import_directive" => {
            extraction
                .dependencies
                .extend(import_dependencies(node, ast));
        }
        "inheritance_specifier" => collect_inheritance(node, ast, contract_index, extraction),
        "state_variable_declaration" => {
            collect_state_variable(node, ast, contract_index, extraction)
        }
        kind if is_callable_kind(kind) => collect_callable(node, ast, contract_index, extraction),
        _ => {}
    }
}

fn collect_inheritance(
    node: Node,
    ast: &SolidityAst,
    contract_index: Option<usize>,
    extraction: &mut SolidityExtraction<'_>,
) {
    let Some(name) = inheritance_name(node, ast) else {
        return;
    };
    extraction.dependencies.push(Dependency {
        name,
        kind: DependencyKind::Inheritance,
    });
    if is_direct_contract_child(node)
        && let Some(base_name) = node.child_by_field_name("name")
        && let Some(contract) = contract_index.and_then(|index| extraction.contracts.get_mut(index))
    {
        contract
            .info
            .base_classes
            .push(node_text(&base_name, &ast.source).to_string());
    }
}

fn collect_state_variable(
    node: Node,
    ast: &SolidityAst,
    contract_index: Option<usize>,
    extraction: &mut SolidityExtraction<'_>,
) {
    let Some(contract) = contract_index.and_then(|index| extraction.contracts.get_mut(index))
    else {
        return;
    };
    contract.info.state_variable_count += 1;
    if let Some(name) = node.child_by_field_name("name") {
        contract
            .info
            .state_variables
            .push(node_text(&name, &ast.source).to_string());
    }
}

fn collect_callable<'tree>(
    node: Node<'tree>,
    ast: &SolidityAst,
    contract_index: Option<usize>,
    extraction: &mut SolidityExtraction<'tree>,
) {
    let contract_name = contract_index
        .and_then(|index| extraction.contracts.get_mut(index))
        .map(|contract| {
            contract.info.function_count += 1;
            if is_contract_advisory_callable(node.kind()) {
                contract.advisory_function_count += 1;
            }
            contract.info.name.clone()
        });
    extraction.callables.push(ExtractedCallable {
        node,
        contract_name,
    });
    collect_modifier(node, ast, &mut extraction.modifier_bodies);
    collect_signature(node, ast, extraction);
}

fn collect_signature(node: Node, ast: &SolidityAst, extraction: &mut SolidityExtraction<'_>) {
    if node.kind() != "function_definition" {
        return;
    }
    extraction
        .function_signatures
        .insert(node_line(&node), signature_from_function(node, &ast.source));
}
