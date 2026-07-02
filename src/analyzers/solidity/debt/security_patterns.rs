//! Solidity-specific security and quality pattern detection.

use tree_sitter::Node;

use crate::analyzers::solidity::parser::node_text;
use crate::config::SolidityLanguageConfig;

const SENTINEL_ADDRESSES: [&str; 4] = [
    "0x0000000000000000000000000000000000000000",
    "0x0000000000000000000000000000000000000001",
    "0x000000000000000000000000000000000000dEaD",
    "0xEeeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE",
];

pub fn detect_function_patterns(
    node: Node,
    source: &str,
    visibility: Option<&str>,
    is_test: bool,
    config: &SolidityLanguageConfig,
) -> Vec<String> {
    if is_test {
        return Vec::new();
    }

    let body = node.child_by_field_name("body").unwrap_or(node);
    let body_text = node_text(&body, source);
    let mut patterns = Vec::new();

    if config.security.tx_origin && body_text.contains("tx.origin") {
        patterns.push("tx-origin-usage".to_string());
    }
    if config.security.delegatecall && body_text.contains("delegatecall") {
        patterns.push("delegatecall-usage".to_string());
    }
    if config.security.selfdestruct && body_text.contains("selfdestruct") {
        patterns.push("selfdestruct-usage".to_string());
    }
    if config.security.assembly_blocks
        && (contains_assembly(body) || body_text.contains("assembly"))
    {
        patterns.push("assembly-block".to_string());
    }
    if config.security.hardcoded_addresses && has_hardcoded_address(body_text) {
        patterns.push("hardcoded-address".to_string());
    }
    if config.security.unchecked_calls && has_unchecked_low_level_call(body, source) {
        patterns.push("unchecked-low-level-call".to_string());
    }
    if config.security.unbounded_loops && has_unbounded_loop(node, body, source) {
        patterns.push("unbounded-loop".to_string());
    }
    if config.security.reentrancy_heuristic && has_external_call_before_state_update(body, source) {
        patterns.push("external-call-before-state-update".to_string());
    }
    if config.security.missing_access_control && is_missing_access_control(node, source, visibility)
    {
        patterns.push("missing-access-control".to_string());
    }

    patterns.sort();
    patterns.dedup();
    patterns
}

pub fn detect_contract_patterns(
    contract: Node,
    source: &str,
    function_count: usize,
    config: &SolidityLanguageConfig,
) -> Vec<String> {
    let mut patterns = Vec::new();
    let state_count = count_kind(contract, "state_variable_declaration");

    if config.security.large_contracts
        && (function_count > config.large_contract_threshold
            || state_count > config.large_contract_threshold)
    {
        patterns.push("large-contract".to_string());
    }

    if config.security.floating_pragma && has_floating_pragma(source) {
        patterns.push("floating-pragma".to_string());
    }

    patterns
}

fn has_floating_pragma(source: &str) -> bool {
    crate::analyzers::solidity::test_detection::has_floating_pragma(source)
}

fn contains_assembly(node: Node) -> bool {
    if node.kind() == "inline_assembly" {
        return true;
    }

    let mut cursor = node.walk();
    node.children(&mut cursor)
        .any(|child| contains_assembly(child))
}

fn has_hardcoded_address(text: &str) -> bool {
    text.split_whitespace().any(|token| {
        let token = token.trim_matches(|ch: char| {
            matches!(ch, ';' | ',' | ')' | '(' | '[' | ']' | '{' | '}' | '"')
        });
        is_non_sentinel_address_literal(token)
    })
}

fn is_non_sentinel_address_literal(token: &str) -> bool {
    if !token.starts_with("0x") || token.len() != 42 {
        return false;
    }

    token[2..].chars().all(|ch| ch.is_ascii_hexdigit())
        && !SENTINEL_ADDRESSES
            .iter()
            .any(|sentinel| token.eq_ignore_ascii_case(sentinel))
}

fn has_unchecked_low_level_call(node: Node, source: &str) -> bool {
    let mut found = false;
    walk_nodes(node, &mut |current| {
        if !is_low_level_call_expression(current, source)
            || is_checked_low_level_call(current, source)
        {
            return;
        }
        found = true;
    });
    found
}

fn is_low_level_call_expression(node: Node, source: &str) -> bool {
    if node.kind() != "call_expression" {
        return false;
    }

    low_level_call_method(node, source).is_some()
}

fn low_level_call_method(node: Node, source: &str) -> Option<&'static str> {
    let function = node.child_by_field_name("function")?;
    member_call_method(function, source)
        .or_else(|| text_low_level_call_method(node_text(&function, source)))
}

fn member_call_method(node: Node, source: &str) -> Option<&'static str> {
    if node.kind() == "member_expression" {
        let property = node.child_by_field_name("property")?;
        return match node_text(&property, source) {
            "call" => Some("call"),
            "delegatecall" => Some("delegatecall"),
            "staticcall" => Some("staticcall"),
            _ => None,
        };
    }

    node.children(&mut node.walk())
        .find_map(|child| member_call_method(child, source))
}

fn text_low_level_call_method(text: &str) -> Option<&'static str> {
    if text.contains(".call{") || text.ends_with(".call") || text.contains(".call(") {
        Some("call")
    } else if text.contains(".delegatecall") {
        Some("delegatecall")
    } else if text.contains(".staticcall") {
        Some("staticcall")
    } else {
        None
    }
}

fn is_checked_low_level_call(node: Node, source: &str) -> bool {
    if is_guarded_by_require_or_if(node, source) {
        return true;
    }

    success_binding_for_call(node, source)
        .is_some_and(|success| success_variable_is_checked(node, source, &success))
}

fn is_guarded_by_require_or_if(node: Node, source: &str) -> bool {
    let mut current = Some(node);
    while let Some(parent) = current {
        match parent.kind() {
            "call_expression" if is_require_or_assert_call(parent, source) => return true,
            "if_statement" if if_statement_checks_call_failure(parent, node, source) => {
                return true;
            }
            "expression_statement" | "parenthesized_expression" | "return_statement" => {}
            _ => break,
        }
        current = parent.parent();
    }
    false
}

fn is_require_or_assert_call(node: Node, source: &str) -> bool {
    if node.kind() != "call_expression" {
        return false;
    }

    let text = node_text(&node, source);
    text.starts_with("require(") || text.starts_with("assert(")
}

fn if_statement_checks_call_failure(if_node: Node, call_node: Node, source: &str) -> bool {
    if !if_contains_call(if_node, call_node) {
        return false;
    }

    if_node
        .child_by_field_name("condition")
        .is_some_and(|condition| condition_denies_success(&condition, source))
}

fn if_contains_call(if_node: Node, call_node: Node) -> bool {
    if_node.start_byte() <= call_node.start_byte() && call_node.end_byte() <= if_node.end_byte()
}

fn condition_denies_success(condition: &Node, source: &str) -> bool {
    let text = node_text(condition, source);
    text.contains('!')
        && (text.contains("success")
            || text.contains("sent")
            || text.contains("ok")
            || text.contains("result"))
}

fn success_binding_for_call(node: Node, source: &str) -> Option<String> {
    let declaration = ancestor_of_kind(node, "variable_declaration_statement")?;
    let declarations = first_named_child(declaration, "variable_declaration_tuple")
        .or_else(|| first_named_child(declaration, "variable_declaration"))?;
    first_bool_binding_name(declarations, source)
}

fn ancestor_of_kind<'a>(mut node: Node<'a>, kind: &str) -> Option<Node<'a>> {
    while node.kind() != kind {
        node = node.parent()?;
    }
    Some(node)
}

fn first_bool_binding_name(node: Node, source: &str) -> Option<String> {
    if node.kind() == "variable_declaration" {
        let type_node = node.child_by_field_name("type")?;
        if !node_text(&type_node, source).contains("bool") {
            return None;
        }
        let name = node.child_by_field_name("name")?;
        return Some(node_text(&name, source).to_string());
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if let Some(name) = first_bool_binding_name(child, source) {
            return Some(name);
        }
    }
    None
}

fn success_variable_is_checked(call_node: Node, source: &str, success: &str) -> bool {
    let Some(body) = function_body_ancestor(call_node) else {
        return false;
    };
    let mut checked = false;
    walk_nodes(body, &mut |current| {
        if checked || current.start_byte() <= call_node.end_byte() {
            return;
        }
        if references_success_in_guard(current, source, success) {
            checked = true;
        }
    });
    checked
}

fn references_success_in_guard(node: Node, source: &str, success: &str) -> bool {
    match node.kind() {
        "call_expression" if is_require_or_assert_call(node, source) => {
            node_text(&node, source).contains(success)
        }
        "if_statement" => node
            .child_by_field_name("condition")
            .is_some_and(|condition| {
                let text = node_text(&condition, source);
                text.contains(success) && text.contains('!')
            }),
        "revert_statement" => node_text(&node, source).contains(success),
        _ => false,
    }
}

fn call_arguments_text<'a>(node: Node, source: &'a str) -> Option<&'a str> {
    let mut cursor = node.walk();
    node.children(&mut cursor)
        .find(|child| child.kind() == "call_arguments" || child.kind() == "argument_list")
        .map(|args| node_text(&args, source))
}

fn has_unbounded_loop(function_node: Node, body: Node, source: &str) -> bool {
    let storage_arrays = contract_ancestor(function_node)
        .map(|contract| storage_dynamic_array_names(contract, source))
        .unwrap_or_default();
    let calldata_params = calldata_array_params(function_node, source);
    let mut found = false;

    walk_nodes(body, &mut |current| {
        if found || !matches!(current.kind(), "for_statement" | "while_statement") {
            return;
        }

        if has_explicit_constant_bound(current, source) {
            return;
        }

        let Some(target) = loop_length_target(current, source) else {
            return;
        };

        if calldata_params.iter().any(|param| param == &target) {
            return;
        }

        if storage_arrays.iter().any(|name| name == &target) {
            found = true;
        }
    });
    found
}

fn has_explicit_constant_bound(loop_node: Node, source: &str) -> bool {
    loop_node
        .child_by_field_name("condition")
        .is_some_and(|condition| subtree_contains_numeric_bound(&condition, source))
}

fn subtree_contains_numeric_bound(node: &Node, source: &str) -> bool {
    if node.kind() == "number_literal" {
        return true;
    }

    let text = node_text(node, source);
    if text.contains('<') {
        return text.split('<').nth(1).is_some_and(|rhs| {
            rhs.trim()
                .chars()
                .next()
                .is_some_and(|ch| ch.is_ascii_digit())
        });
    }

    node.children(&mut node.walk())
        .any(|child| subtree_contains_numeric_bound(&child, source))
}

fn loop_length_target(loop_node: Node, source: &str) -> Option<String> {
    loop_node
        .child_by_field_name("condition")
        .and_then(|condition| member_length_target(condition, source))
}

fn member_length_target(node: Node, source: &str) -> Option<String> {
    if node.kind() == "member_expression" {
        let property = node.child_by_field_name("property")?;
        if node_text(&property, source) != "length" {
            return None;
        }
        let object = node.child_by_field_name("object")?;
        if object.kind() == "identifier" {
            return Some(node_text(&object, source).to_string());
        }
    }

    node.children(&mut node.walk())
        .find_map(|child| member_length_target(child, source))
}

fn storage_dynamic_array_names(contract: Node, source: &str) -> Vec<String> {
    let mut names = Vec::new();
    walk_nodes(contract, &mut |node| {
        if node.kind() != "state_variable_declaration" {
            return;
        }
        let Some(type_node) = node.child_by_field_name("type") else {
            return;
        };
        if !is_dynamic_array_type(type_node, source) {
            return;
        }
        if let Some(name) = node.child_by_field_name("name") {
            names.push(node_text(&name, source).to_string());
        }
    });
    names
}

fn is_dynamic_array_type(type_node: Node, source: &str) -> bool {
    let text = node_text(&type_node, source);
    if !text.contains('[') {
        return false;
    }

    text.contains("[]")
}

fn calldata_array_params(function_node: Node, source: &str) -> Vec<String> {
    let mut params = Vec::new();
    walk_nodes(function_node, &mut |node| {
        if node.kind() != "parameter" {
            return;
        }
        let text = node_text(&node, source);
        if !text.contains("calldata") || !text.contains('[') {
            return;
        }
        if let Some(name) = node.child_by_field_name("name") {
            params.push(node_text(&name, source).to_string());
        }
    });
    params
}

fn has_external_call_before_state_update(body: Node, source: &str) -> bool {
    block_has_cei_violation(body, source)
}

fn block_has_cei_violation(node: Node, source: &str) -> bool {
    let mut saw_external = false;
    for statement in statement_container(node) {
        if compound_statement_has_cei_violation(statement, source) {
            return true;
        }
        if statement_may_invoke_external(statement, source) {
            saw_external = true;
        } else if saw_external && statement_updates_state(statement, source) {
            return true;
        }
    }
    false
}

fn compound_statement_has_cei_violation(statement: Node, source: &str) -> bool {
    match statement.kind() {
        "if_statement" => {
            statement_body(statement, "consequence")
                .is_some_and(|body| block_has_cei_violation(body, source))
                || statement
                    .child_by_field_name("alternative")
                    .is_some_and(|body| block_has_cei_violation(body, source))
        }
        "for_statement" | "while_statement" | "do_while_statement" => statement
            .child_by_field_name("body")
            .is_some_and(|body| block_has_cei_violation(body, source)),
        "try_statement" => {
            statement_body(statement, "body")
                .is_some_and(|body| block_has_cei_violation(body, source))
                || catch_bodies(statement)
                    .into_iter()
                    .any(|body| block_has_cei_violation(body, source))
        }
        "block_statement" => block_has_cei_violation(statement, source),
        _ => false,
    }
}

fn statement_container(node: Node) -> Vec<Node> {
    if matches!(node.kind(), "function_body" | "block_statement") {
        return named_children(node);
    }

    if let Some(body) = node.child_by_field_name("body") {
        return statement_container(body);
    }

    named_children(node)
}

fn named_children(node: Node) -> Vec<Node> {
    node.children(&mut node.walk())
        .filter(|child| child.is_named())
        .collect()
}

fn statement_body<'a>(statement: Node<'a>, field: &str) -> Option<Node<'a>> {
    statement.child_by_field_name(field)
}

fn catch_bodies(statement: Node) -> Vec<Node> {
    statement
        .children(&mut statement.walk())
        .filter(|child| child.kind() == "catch_clause")
        .filter_map(|catch| catch.child_by_field_name("body"))
        .collect()
}

fn statement_may_invoke_external(statement: Node, source: &str) -> bool {
    if statement_invokes_external_call(statement, source) {
        return true;
    }

    match statement.kind() {
        "if_statement" => {
            statement_body(statement, "consequence")
                .is_some_and(|body| block_may_invoke_external(body, source))
                || statement
                    .child_by_field_name("alternative")
                    .is_some_and(|body| block_may_invoke_external(body, source))
        }
        "for_statement" | "while_statement" | "do_while_statement" => statement
            .child_by_field_name("body")
            .is_some_and(|body| block_may_invoke_external(body, source)),
        "try_statement" => {
            statement_body(statement, "body")
                .is_some_and(|body| block_may_invoke_external(body, source))
                || catch_bodies(statement)
                    .into_iter()
                    .any(|body| block_may_invoke_external(body, source))
        }
        "block_statement" => block_may_invoke_external(statement, source),
        _ => false,
    }
}

fn block_may_invoke_external(node: Node, source: &str) -> bool {
    statement_container(node)
        .iter()
        .any(|statement| statement_may_invoke_external(*statement, source))
}

fn statement_invokes_external_call(statement: Node, source: &str) -> bool {
    let mut found = false;
    walk_nodes(statement, &mut |node| {
        if found || node.kind() != "call_expression" {
            return;
        }
        if is_external_interaction_call(node, source) {
            found = true;
        }
    });
    found
}

fn is_external_interaction_call(node: Node, source: &str) -> bool {
    if node.kind() != "call_expression" {
        return false;
    }

    let Some(function) = node.child_by_field_name("function") else {
        let text = node_text(&node, source);
        return text.contains(".transfer(") || text.contains(".send(");
    };

    match function.kind() {
        "member_expression" => {
            let Some(property) = function.child_by_field_name("property") else {
                return false;
            };
            matches!(
                node_text(&property, source),
                "call" | "delegatecall" | "transfer" | "send"
            )
        }
        "identifier" => node_text(&function, source) == "transfer",
        _ => {
            let text = node_text(&node, source);
            text.contains(".transfer(") || text.contains(".send(")
        }
    }
}

fn statement_updates_state(statement: Node, source: &str) -> bool {
    let mut found = false;
    walk_nodes(statement, &mut |node| {
        if found {
            return;
        }
        if !matches!(
            node.kind(),
            "assignment_expression" | "augmented_assignment_expression"
        ) {
            return;
        }
        if assignment_targets_state(node, source) {
            found = true;
        }
    });
    found
}

fn assignment_targets_state(node: Node, source: &str) -> bool {
    let Some(left) = node.child_by_field_name("left") else {
        return false;
    };
    if left.kind() == "tuple_expression" {
        return false;
    }

    let text = node_text(&left, source);
    !text.starts_with('(') && !text.starts_with("bool ")
}

fn is_missing_access_control(node: Node, source: &str, visibility: Option<&str>) -> bool {
    if !matches!(visibility, Some("public") | Some("external")) {
        return false;
    }

    if node.kind() != "function_definition" {
        return false;
    }

    if has_modifier(node) || has_internal_access_guard(node, source) {
        return false;
    }

    true
}

fn has_modifier(node: Node) -> bool {
    node.children(&mut node.walk())
        .any(|child| child.kind() == "modifier_invocation")
}

fn has_internal_access_guard(function_node: Node, source: &str) -> bool {
    let body = function_node
        .child_by_field_name("body")
        .unwrap_or(function_node);
    let mut found = false;
    walk_nodes(body, &mut |node| {
        if found || node.kind() != "call_expression" {
            return;
        }
        if is_access_guard_call(node, source) {
            found = true;
        }
    });
    found || body_contains_sender_require(body, source)
}

fn is_access_guard_call(node: Node, source: &str) -> bool {
    let Some(function) = node.child_by_field_name("function") else {
        return false;
    };
    let name = match function.kind() {
        "identifier" | "_identifier_path" => node_text(&function, source),
        "member_expression" => function
            .child_by_field_name("property")
            .map(|property| node_text(&property, source))
            .unwrap_or(""),
        _ => node_text(&function, source),
    };

    name.starts_with("_check")
        || name.starts_with("_authorize")
        || name.starts_with("_only")
        || matches!(name, "onlyOwner" | "onlyRole" | "onlyAdmin" | "onlyProxy")
}

fn body_contains_sender_require(body: Node, source: &str) -> bool {
    let mut found = false;
    walk_nodes(body, &mut |node| {
        if found || node.kind() != "call_expression" || !is_require_or_assert_call(node, source) {
            return;
        }
        let args = call_arguments_text(node, source).unwrap_or("");
        if args.contains("msg.sender") {
            found = true;
        }
    });
    found
}

fn contract_ancestor(node: Node) -> Option<Node> {
    let mut current = node.parent();
    while let Some(parent) = current {
        if matches!(
            parent.kind(),
            "contract_declaration" | "interface_declaration" | "library_declaration"
        ) {
            return Some(parent);
        }
        current = parent.parent();
    }
    None
}

fn function_body_ancestor(node: Node) -> Option<Node> {
    let mut current = Some(node);
    while let Some(parent) = current {
        if parent.kind() == "function_body" {
            return Some(parent);
        }
        current = parent.parent();
    }
    None
}

fn first_named_child<'a>(node: Node<'a>, kind: &str) -> Option<Node<'a>> {
    node.children(&mut node.walk())
        .find(|child| child.kind() == kind)
}

fn count_kind(node: Node, kind: &str) -> usize {
    let mut count = usize::from(node.kind() == kind);
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        count += count_kind(child, kind);
    }
    count
}

fn walk_nodes(node: Node, f: &mut impl FnMut(Node)) {
    f(node);
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_nodes(child, f);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzers::solidity::parser::parse_source;
    use std::path::Path;

    fn find_node<'a>(node: tree_sitter::Node<'a>, kind: &str) -> Option<tree_sitter::Node<'a>> {
        if node.kind() == kind {
            return Some(node);
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if let Some(found) = find_node(child, kind) {
                return Some(found);
            }
        }
        None
    }

    fn find_function_named<'a>(
        node: tree_sitter::Node<'a>,
        source: &'a str,
        name: &str,
    ) -> Option<tree_sitter::Node<'a>> {
        if node.kind() == "function_definition" {
            let text = node_text(&node, source);
            if text.contains(&format!("function {name}")) {
                return Some(node);
            }
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if let Some(found) = find_function_named(child, source, name) {
                return Some(found);
            }
        }
        None
    }
    #[test]
    fn test_unchecked_low_level_call_positive() {
        let source = r#"pragma solidity 0.8.20; contract C { function f(address target) internal { target.call{value: 0}(""); } }"#;
        let ast = parse_source(source, Path::new("Test.sol")).expect("parse source");
        let function = find_node(ast.tree.root_node(), "function_definition").expect("function");
        let body = function.child_by_field_name("body").unwrap();
        assert!(has_unchecked_low_level_call(body, source));
    }

    #[test]
    fn test_nested_reentrancy_positive() {
        let source = r#"pragma solidity 0.8.20; contract C { mapping(address => uint256) balances; function f() internal { if (true) { payable(msg.sender).transfer(1); } balances[msg.sender] = 0; } }"#;
        let ast = parse_source(source, Path::new("Test.sol")).expect("parse source");
        let function = find_node(ast.tree.root_node(), "function_definition").expect("function");
        let body = function.child_by_field_name("body").unwrap();
        assert!(has_external_call_before_state_update(body, source));
    }

    #[test]
    fn test_nested_reentrancy_negative() {
        let source = r#"pragma solidity 0.8.20; contract C { mapping(address => uint256) balances; function f() internal { balances[msg.sender] = 0; if (true) { payable(msg.sender).transfer(1); } } }"#;
        let ast = parse_source(source, Path::new("Test.sol")).expect("parse source");
        let function = find_node(ast.tree.root_node(), "function_definition").expect("function");
        let body = function.child_by_field_name("body").unwrap();
        assert!(!has_external_call_before_state_update(body, source));
    }

    #[test]
    fn test_checked_low_level_call_with_require_success() {
        let source = r#"pragma solidity 0.8.20; contract C { function f(address target) internal { (bool success, ) = target.call{value: 0}(""); require(success); } }"#;
        let ast = parse_source(source, Path::new("Test.sol")).expect("parse source");
        let function = find_node(ast.tree.root_node(), "function_definition").expect("function");
        let body = function.child_by_field_name("body").unwrap();
        assert!(!has_unchecked_low_level_call(body, source));
    }

    #[test]
    fn test_checked_low_level_call_with_if_revert() {
        let source = r#"pragma solidity 0.8.20; contract C { function f(address target) internal { (bool success, ) = target.call(""); if (!success) revert(); } }"#;
        let ast = parse_source(source, Path::new("Test.sol")).expect("parse source");
        let function = find_node(ast.tree.root_node(), "function_definition").expect("function");
        let body = function.child_by_field_name("body").unwrap();
        assert!(!has_unchecked_low_level_call(body, source));
    }

    #[test]
    fn test_internal_access_guard_negative() {
        let source = r#"pragma solidity 0.8.20; contract C { function _checkOwner() internal view {} function setValue(uint256 value) public { _checkOwner(); value; } }"#;
        let ast = parse_source(source, Path::new("Test.sol")).expect("parse source");
        let function =
            find_function_named(ast.tree.root_node(), source, "setValue").expect("setValue");
        assert!(!is_missing_access_control(function, source, Some("public")));
    }

    #[test]
    fn test_storage_array_unbounded_loop_positive() {
        let source = r#"pragma solidity 0.8.20; contract C { address[] users; function f() internal { for (uint256 i = 0; i < users.length; i++) {} } }"#;
        let ast = parse_source(source, Path::new("Test.sol")).expect("parse source");
        let function = find_node(ast.tree.root_node(), "function_definition").expect("function");
        let body = function.child_by_field_name("body").unwrap();
        assert!(has_unbounded_loop(function, body, source));
    }

    #[test]
    fn test_constant_bound_loop_negative() {
        let source = r#"pragma solidity 0.8.20; contract C { address[] users; function f() internal { for (uint256 i = 0; i < 10; i++) {} } }"#;
        let ast = parse_source(source, Path::new("Test.sol")).expect("parse source");
        let function = find_node(ast.tree.root_node(), "function_definition").expect("function");
        let body = function.child_by_field_name("body").unwrap();
        assert!(!has_unbounded_loop(function, body, source));
    }

    #[test]
    fn test_fixed_size_array_loop_negative() {
        let source = r#"pragma solidity 0.8.20; contract C { uint256[10] items; function f() internal { for (uint256 i = 0; i < items.length; i++) {} } }"#;
        let ast = parse_source(source, Path::new("Test.sol")).expect("parse source");
        let function = find_node(ast.tree.root_node(), "function_definition").expect("function");
        let body = function.child_by_field_name("body").unwrap();
        assert!(!has_unbounded_loop(function, body, source));
    }

    #[test]
    fn test_sentinel_address_not_flagged() {
        let source = r#"pragma solidity 0.8.20; contract C { function f() internal pure returns (address) { return address(0); } }"#;
        let ast = parse_source(source, Path::new("Test.sol")).expect("parse source");
        let function = find_node(ast.tree.root_node(), "function_definition").expect("function");
        let body = function.child_by_field_name("body").unwrap();
        assert!(!has_hardcoded_address(node_text(&body, source)));
    }
}
