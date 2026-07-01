//! Solidity-specific security and quality pattern detection.

use tree_sitter::Node;

use crate::analyzers::solidity::parser::node_text;
use crate::config::SolidityLanguageConfig;

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
    if config.security.unbounded_loops && has_unbounded_loop(body, body_text) {
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
        let token = token
            .trim_matches(|ch: char| matches!(ch, ';' | ',' | ')' | '(' | '[' | ']' | '{' | '}'));
        token.starts_with("0x")
            && token.len() >= 42
            && token[2..].chars().all(|ch| ch.is_ascii_hexdigit())
    })
}

fn has_unchecked_low_level_call(node: Node, source: &str) -> bool {
    let mut found = false;
    walk_nodes(node, &mut |current| {
        if current.kind() != "call_expression" {
            return;
        }

        let text = node_text(&current, source);
        if text.contains(".call{")
            || text.contains(".delegatecall(")
            || text.contains(".staticcall(")
        {
            let parent_text = current
                .parent()
                .map(|parent| node_text(&parent, source))
                .unwrap_or("");
            if !parent_text.contains("require(") && !parent_text.contains("if (") {
                found = true;
            }
        }
    });
    found
}

fn has_unbounded_loop(node: Node, text: &str) -> bool {
    if text.contains(".length") && (text.contains("for ") || text.contains("while ")) {
        return true;
    }

    let mut found = false;
    walk_nodes(node, &mut |current| {
        if !matches!(current.kind(), "for_statement" | "while_statement") {
            return;
        }

        let has_array_length = child_kind_exists(current, "member_expression")
            || child_kind_exists(current, "subscript_expression");
        let has_fixed_bound = child_kind_exists(current, "number_literal");
        if has_array_length && !has_fixed_bound {
            found = true;
        }
    });
    found
}

fn has_external_call_before_state_update(node: Node, source: &str) -> bool {
    let statements = top_level_statements(node);
    let mut saw_external_call = false;

    for statement in statements {
        if is_external_call(statement, source) {
            saw_external_call = true;
            continue;
        }

        if saw_external_call && is_state_update(statement) {
            return true;
        }
    }

    false
}

fn top_level_statements(node: Node) -> Vec<Node> {
    let mut cursor = node.walk();
    let children = node
        .children(&mut cursor)
        .filter(|child| child.is_named())
        .collect::<Vec<_>>();

    if !children.is_empty() {
        return children;
    }

    vec![node]
}

fn is_external_call(node: Node, source: &str) -> bool {
    let text = node_text(&node, source);
    text.contains(".call")
        || text.contains(".transfer(")
        || text.contains(".send(")
        || text.contains(".delegatecall(")
}

fn is_state_update(node: Node) -> bool {
    child_kind_exists(node, "assignment_expression")
        || child_kind_exists(node, "augmented_assignment_expression")
}

fn is_missing_access_control(node: Node, source: &str, visibility: Option<&str>) -> bool {
    if !matches!(visibility, Some("public") | Some("external")) {
        return false;
    }

    if node.kind() != "function_definition" {
        return false;
    }

    let text = node_text(&node, source);
    if text.contains("onlyOwner")
        || text.contains("onlyRole")
        || text.contains("msg.sender")
        || has_modifier(node)
    {
        return false;
    }

    true
}

fn has_modifier(node: Node) -> bool {
    let mut cursor = node.walk();
    node.children(&mut cursor)
        .any(|child| child.kind() == "modifier_invocation")
}

fn count_kind(node: Node, kind: &str) -> usize {
    let mut count = usize::from(node.kind() == kind);
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        count += count_kind(child, kind);
    }
    count
}

fn child_kind_exists(node: Node, kind: &str) -> bool {
    if node.kind() == kind {
        return true;
    }

    let mut cursor = node.walk();
    node.children(&mut cursor)
        .any(|child| child_kind_exists(child, kind))
}

fn walk_nodes(node: Node, f: &mut impl FnMut(Node)) {
    f(node);
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_nodes(child, f);
    }
}
