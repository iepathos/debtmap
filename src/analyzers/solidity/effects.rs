//! EVM-relevant state and purity effect analysis for Solidity functions.

use std::collections::{HashMap, HashSet};

use tree_sitter::Node;

use crate::analyzers::solidity::parser::node_text;
use crate::core::PurityLevel;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SolidityEffectSummary {
    pub reads_state: bool,
    pub writes_state: bool,
    pub external_call: bool,
    pub value_transfer: bool,
}

impl SolidityEffectSummary {
    pub fn merge(mut self, other: Self) -> Self {
        self.reads_state |= other.reads_state;
        self.writes_state |= other.writes_state;
        self.external_call |= other.external_call;
        self.value_transfer |= other.value_transfer;
        self
    }

    pub fn purity_level(&self) -> PurityLevel {
        if self.writes_state || self.external_call || self.value_transfer {
            PurityLevel::Impure
        } else if self.reads_state {
            PurityLevel::ReadOnly
        } else {
            PurityLevel::StrictlyPure
        }
    }

    pub fn is_strictly_pure(&self) -> bool {
        !self.reads_state && !self.writes_state && !self.external_call && !self.value_transfer
    }
}

pub fn analyze_callable_effects(
    node: Node,
    source: &str,
    state_variables: &[String],
    modifiers: &HashMap<String, Node<'_>>,
) -> SolidityEffectSummary {
    let Some(body) = node.child_by_field_name("body") else {
        return SolidityEffectSummary::default();
    };

    let state = state_variables.iter().cloned().collect::<HashSet<_>>();
    let mut summary = analyze_body_effects(body, source, &state);
    for modifier_name in modifier_invocations(node, source) {
        if let Some(modifier_body) = modifiers.get(&modifier_name) {
            summary = summary.merge(analyze_body_effects(*modifier_body, source, &state));
        }
    }
    summary
}

pub fn mutability_mismatch_patterns(
    declared: Option<&str>,
    effects: &SolidityEffectSummary,
) -> Vec<String> {
    let mismatched = match declared {
        Some("pure") => {
            effects.reads_state
                || effects.writes_state
                || effects.external_call
                || effects.value_transfer
        }
        Some("view") => effects.writes_state || effects.external_call || effects.value_transfer,
        _ => false,
    };

    mismatched
        .then(|| "mutability-mismatch".to_string())
        .into_iter()
        .collect()
}

pub fn state_variable_names(contract: Node, source: &str) -> Vec<String> {
    let mut names = Vec::new();
    walk_nodes(contract, &mut |node| {
        if node.kind() != "state_variable_declaration" {
            return;
        }
        if let Some(name) = node.child_by_field_name("name") {
            names.push(node_text(&name, source).to_string());
        }
    });
    names
}

pub fn modifier_bodies_by_name<'a>(root: Node<'a>, source: &str) -> HashMap<String, Node<'a>> {
    let mut modifiers = HashMap::new();
    collect_modifier_bodies(root, source, &mut modifiers);
    modifiers
}

fn collect_modifier_bodies<'a>(
    node: Node<'a>,
    source: &str,
    modifiers: &mut HashMap<String, Node<'a>>,
) {
    if node.kind() == "modifier_definition"
        && let (Some(name), Some(body)) = (
            node.child_by_field_name("name"),
            node.child_by_field_name("body"),
        )
    {
        modifiers.insert(node_text(&name, source).to_string(), body);
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_modifier_bodies(child, source, modifiers);
    }
}

fn analyze_body_effects(
    body: Node,
    source: &str,
    state: &HashSet<String>,
) -> SolidityEffectSummary {
    let mut summary = SolidityEffectSummary::default();
    walk_nodes(body, &mut |node| {
        if node.kind() == "identifier" && state.contains(node_text(&node, source)) {
            summary.reads_state = true;
        }
        if is_state_write(node, source, state) {
            summary.writes_state = true;
        }
        if is_external_call(node, source) {
            summary.external_call = true;
        }
        if is_value_transfer(node, source) {
            summary.value_transfer = true;
        }
    });
    summary
}

fn modifier_invocations(node: Node, source: &str) -> Vec<String> {
    let mut names = Vec::new();
    walk_nodes(node, &mut |current| {
        if current.kind() != "modifier_invocation" {
            return;
        }
        if let Some(name) = current.child_by_field_name("name").or_else(|| {
            current
                .children(&mut current.walk())
                .find(|child| child.kind() == "identifier")
        }) {
            names.push(node_text(&name, source).to_string());
        } else {
            names.push(node_text(&current, source).to_string());
        }
    });
    names
}

fn is_state_write(node: Node, source: &str, state: &HashSet<String>) -> bool {
    match node.kind() {
        "assignment_expression" | "augmented_assignment_expression" => node
            .child_by_field_name("left")
            .is_some_and(|left| targets_state(&left, source, state)),
        "call_expression" => is_state_mutating_call(node, source, state),
        _ => false,
    }
}

fn is_state_mutating_call(node: Node, source: &str, state: &HashSet<String>) -> bool {
    let Some(function) = node.child_by_field_name("function") else {
        return false;
    };
    if function.kind() != "member_expression" {
        return false;
    }

    let Some(property) = function.child_by_field_name("property") else {
        return false;
    };
    if !matches!(
        node_text(&property, source),
        "push" | "pop" | "delete" | "pushBack" | "pushFront"
    ) {
        return false;
    }

    function
        .child_by_field_name("object")
        .is_some_and(|object| targets_state(&object, source, state))
}

fn targets_state(node: &Node, source: &str, state: &HashSet<String>) -> bool {
    match node.kind() {
        "identifier" => state.contains(node_text(node, source)),
        "member_expression" | "subscript_expression" => node
            .child_by_field_name("object")
            .or_else(|| node.child_by_field_name("base"))
            .is_some_and(|object| targets_state(&object, source, state)),
        _ => node
            .children(&mut node.walk())
            .any(|child| targets_state(&child, source, state)),
    }
}

fn is_external_call(node: Node, source: &str) -> bool {
    if node.kind() != "call_expression" {
        return false;
    }

    let Some(function) = node.child_by_field_name("function") else {
        return node_text(&node, source).contains(".call");
    };

    match function.kind() {
        "member_expression" => {
            let Some(property) = function.child_by_field_name("property") else {
                return false;
            };
            matches!(
                node_text(&property, source),
                "call" | "delegatecall" | "staticcall" | "transfer" | "send"
            )
        }
        _ => {
            let text = node_text(&node, source);
            text.contains(".call") || text.contains(".transfer(") || text.contains(".send(")
        }
    }
}

fn is_value_transfer(node: Node, source: &str) -> bool {
    if node.kind() != "call_expression" {
        return false;
    }

    let text = node_text(&node, source);
    if text.contains(".transfer(") || text.contains(".send(") || text.contains("call{value:") {
        return true;
    }

    node.child_by_field_name("function")
        .and_then(|function| function.child_by_field_name("property"))
        .is_some_and(|property| matches!(node_text(&property, source), "transfer" | "send"))
}

fn walk_nodes(node: Node, visit: &mut impl FnMut(Node)) {
    visit(node);
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_nodes(child, visit);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzers::solidity::parser::parse_source;
    use std::path::Path;

    fn parse_fixture(source: &str) -> crate::core::ast::SolidityAst {
        parse_source(source, Path::new("Effects.sol")).expect("parse")
    }

    fn find_function<'a>(
        node: tree_sitter::Node<'a>,
        source: &'a str,
        name: &str,
    ) -> Option<tree_sitter::Node<'a>> {
        if node.kind() == "function_definition" && node_text(&node, source).contains(name) {
            return Some(node);
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if let Some(found) = find_function(child, source, name) {
                return Some(found);
            }
        }
        None
    }

    fn find_contract(node: tree_sitter::Node<'_>) -> Option<tree_sitter::Node<'_>> {
        if node.kind() == "contract_declaration" {
            return Some(node);
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if let Some(found) = find_contract(child) {
                return Some(found);
            }
        }
        None
    }

    #[test]
    fn test_detects_state_write_and_read() {
        let source = r#"pragma solidity 0.8.20;
contract C {
    uint256 public value;
    function setValue(uint256 next) public { value = next; }
}
"#;
        let ast = parse_fixture(source);
        let contract = find_contract(ast.tree.root_node()).expect("contract");
        let state = state_variable_names(contract, source);
        let function = find_function(ast.tree.root_node(), source, "setValue").expect("function");
        let effects = analyze_callable_effects(function, source, &state, &HashMap::new());
        assert!(effects.reads_state);
        assert!(effects.writes_state);
        assert_eq!(effects.purity_level(), PurityLevel::Impure);
    }

    #[test]
    fn test_local_assignment_does_not_count_as_state_write() {
        let source = r#"pragma solidity 0.8.20;
contract C {
    function locals() public pure {
        uint256 local = 1;
        local = 2;
    }
}
"#;
        let ast = parse_fixture(source);
        let function = find_function(ast.tree.root_node(), source, "locals").expect("function");
        let effects = analyze_callable_effects(function, source, &[], &HashMap::new());
        assert!(!effects.reads_state);
        assert!(!effects.writes_state);
        assert!(effects.is_strictly_pure());
    }

    #[test]
    fn test_view_read_only_state_access() {
        let source = r#"pragma solidity 0.8.20;
contract C {
    uint256 public value;
    function readValue() public view returns (uint256) { return value; }
}
"#;
        let ast = parse_fixture(source);
        let contract = find_contract(ast.tree.root_node()).expect("contract");
        let state = state_variable_names(contract, source);
        let function = find_function(ast.tree.root_node(), source, "readValue").expect("function");
        let effects = analyze_callable_effects(function, source, &state, &HashMap::new());
        assert!(effects.reads_state);
        assert!(!effects.writes_state);
        assert_eq!(effects.purity_level(), PurityLevel::ReadOnly);
    }

    #[test]
    fn test_modifier_state_write_contributes_to_function() {
        let source = r#"pragma solidity 0.8.20;
contract C {
    uint256 public value;
    modifier updatesValue() { value = 1; _; }
    function touch() public updatesValue {}
}
"#;
        let ast = parse_source(source, Path::new("Effects.sol")).expect("parse");
        let contract = find_contract(ast.tree.root_node()).expect("contract");
        let state = state_variable_names(contract, source);
        let modifiers = modifier_bodies_by_name(ast.tree.root_node(), source);
        let function = find_function(ast.tree.root_node(), source, "touch").unwrap();
        let effects = analyze_callable_effects(function, source, &state, &modifiers);
        assert!(effects.writes_state);
    }

    #[test]
    fn test_mutability_mismatch_for_pure_with_state_read() {
        let effects = SolidityEffectSummary {
            reads_state: true,
            ..Default::default()
        };
        assert_eq!(
            mutability_mismatch_patterns(Some("pure"), &effects),
            vec!["mutability-mismatch".to_string()]
        );
    }
}
