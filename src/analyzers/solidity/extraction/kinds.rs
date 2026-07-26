use tree_sitter::Node;

use crate::analyzers::solidity::types::ContractKind;

pub(super) fn contract_kind(kind: &str) -> Option<ContractKind> {
    match kind {
        "contract_declaration" => Some(ContractKind::Contract),
        "interface_declaration" => Some(ContractKind::Interface),
        "library_declaration" => Some(ContractKind::Library),
        _ => None,
    }
}

pub(super) fn is_callable_kind(kind: &str) -> bool {
    matches!(
        kind,
        "function_definition"
            | "modifier_definition"
            | "constructor"
            | "constructor_definition"
            | "fallback"
            | "receive"
    )
}

pub(super) fn is_contract_advisory_callable(kind: &str) -> bool {
    matches!(
        kind,
        "function_definition" | "modifier_definition" | "constructor" | "fallback" | "receive"
    )
}

pub(super) fn is_direct_contract_child(node: Node) -> bool {
    node.parent().is_some_and(|parent| {
        matches!(
            parent.kind(),
            "contract_declaration" | "interface_declaration" | "library_declaration"
        )
    })
}
