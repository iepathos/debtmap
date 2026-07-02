use crate::analyzers::solidity::parser::node_text;
use crate::config::SolidityLanguageConfig;
use tree_sitter::Node;

pub fn detect_advanced_patterns(
    node: Node,
    source: &str,
    config: &SolidityLanguageConfig,
) -> Vec<String> {
    let text = node_text(&node, source);
    let mut patterns = Vec::new();

    push_if(
        &mut patterns,
        config.security.unchecked_arithmetic && has_unchecked_block(text),
        "unchecked-arithmetic",
    );
    push_if(
        &mut patterns,
        config.security.unsafe_erc20_transfer && has_unsafe_erc20_transfer(text),
        "unsafe-erc20-transfer",
    );
    push_if(
        &mut patterns,
        config.security.push_without_length_cap && has_push_without_length_cap(text),
        "push-without-length-cap",
    );
    push_if(
        &mut patterns,
        config.security.block_timestamp_dependency && text.contains("block.timestamp"),
        "block-timestamp-dependency",
    );
    push_if(
        &mut patterns,
        config.security.tx_gas_price_dependency && text.contains("tx.gasprice"),
        "tx-gas-price-dependency",
    );
    push_if(
        &mut patterns,
        config.security.encode_packed_collision && has_encode_packed_collision_risk(node, source),
        "encode-packed-collision",
    );
    push_if(
        &mut patterns,
        config.security.delegatecall_in_constructor
            && matches!(node.kind(), "constructor" | "constructor_definition")
            && text.contains("delegatecall"),
        "delegatecall-in-constructor",
    );

    patterns
}

fn push_if(patterns: &mut Vec<String>, condition: bool, pattern: &str) {
    if condition {
        patterns.push(pattern.to_string());
    }
}

fn has_unchecked_block(text: &str) -> bool {
    text.contains("unchecked")
}

fn has_unsafe_erc20_transfer(text: &str) -> bool {
    let has_transfer = text.contains(".transfer(") || text.contains(".transferFrom(");
    let looks_wrapped = text.contains("safeTransfer")
        || text.contains("safeTransferFrom")
        || text.contains("require(")
        || text.contains("if (");

    has_transfer && !looks_wrapped
}

fn has_push_without_length_cap(text: &str) -> bool {
    text.contains(".push(") && !has_length_cap(text)
}

fn has_length_cap(text: &str) -> bool {
    text.contains("require(") && (text.contains(".length <") || text.contains(".length <="))
}

fn has_encode_packed_collision_risk(node: Node, source: &str) -> bool {
    let param_types = function_parameter_types(node, source);
    let mut found = false;
    walk_nodes(node, &mut |current| {
        if found || current.kind() != "call_expression" {
            return;
        }
        if !is_abi_encode_packed_call(current, source) {
            return;
        }
        found = encode_packed_has_multiple_dynamic_args(current, source, &param_types);
    });
    found
}

fn function_parameter_types(function_node: Node, source: &str) -> Vec<(String, String)> {
    let mut params = Vec::new();
    walk_nodes(function_node, &mut |node| {
        if node.kind() != "parameter" {
            return;
        }
        let name = node
            .child_by_field_name("name")
            .map(|name| node_text(&name, source).to_string())
            .unwrap_or_default();
        let type_text = node
            .child_by_field_name("type")
            .map(|type_node| node_text(&type_node, source).to_string())
            .unwrap_or_else(|| node_text(&node, source).to_string());
        if !name.is_empty() {
            params.push((name, type_text));
        }
    });

    if params.is_empty() {
        return fallback_parameter_types(crate::analyzers::solidity::parser::node_text(
            &function_node,
            source,
        ));
    }

    params
}

fn fallback_parameter_types(function_text: &str) -> Vec<(String, String)> {
    let Some(start) = function_text.find('(') else {
        return Vec::new();
    };
    let Some(end) = function_text.find(')') else {
        return Vec::new();
    };
    function_text[start + 1..end]
        .split(',')
        .filter_map(|param| {
            let param = param.trim();
            let name = param.split_whitespace().last()?;
            let type_text = param.trim_end_matches(name).trim();
            (!type_text.is_empty()).then(|| (name.to_string(), type_text.to_string()))
        })
        .collect()
}

fn is_abi_encode_packed_call(node: Node, source: &str) -> bool {
    if node_text(&node, source).contains("abi.encodePacked") {
        return true;
    }

    let Some(function) = node.child_by_field_name("function") else {
        return false;
    };

    if function.kind() != "member_expression" {
        return false;
    }

    let Some(object) = function.child_by_field_name("object") else {
        return false;
    };
    let Some(property) = function.child_by_field_name("property") else {
        return false;
    };
    node_text(&object, source) == "abi" && node_text(&property, source) == "encodePacked"
}

fn encode_packed_has_multiple_dynamic_args(
    node: Node,
    source: &str,
    param_types: &[(String, String)],
) -> bool {
    let args = call_argument_nodes(node);
    if args.len() >= 2 {
        let dynamic_count = args
            .iter()
            .filter(|arg| is_dynamic_encode_arg(**arg, source, param_types))
            .count();
        if dynamic_count >= 2 {
            return true;
        }
    }

    encode_packed_dynamic_arg_names(node_text(&node, source), param_types).len() >= 2
}

fn encode_packed_dynamic_arg_names(
    call_text: &str,
    param_types: &[(String, String)],
) -> Vec<String> {
    let Some(start) = call_text.find('(') else {
        return Vec::new();
    };
    let Some(end) = call_text.rfind(')') else {
        return Vec::new();
    };

    call_text[start + 1..end]
        .split(',')
        .filter_map(|arg| {
            let name = arg.trim();
            (!name.is_empty()).then(|| name.to_string())
        })
        .filter(|name| {
            param_types
                .iter()
                .find(|(param, _)| param == name)
                .is_some_and(|(_, type_text)| is_dynamic_type_text(type_text))
        })
        .collect()
}

fn call_argument_nodes(node: Node) -> Vec<Node> {
    let mut cursor = node.walk();
    let Some(call_args) = node
        .children(&mut cursor)
        .find(|child| child.kind() == "call_arguments" || child.kind() == "argument_list")
    else {
        return Vec::new();
    };

    call_args
        .children(&mut call_args.walk())
        .filter_map(|child| {
            if child.kind() == "call_argument" {
                return child
                    .children(&mut child.walk())
                    .find(|nested| nested.is_named());
            }
            child.is_named().then_some(child)
        })
        .collect()
}

fn is_dynamic_encode_arg(node: Node, source: &str, param_types: &[(String, String)]) -> bool {
    match node.kind() {
        "identifier" => param_types
            .iter()
            .find(|(name, _)| name == node_text(&node, source))
            .is_some_and(|(_, type_text)| is_dynamic_type_text(type_text)),
        "string_literal" | "string" => true,
        "number_literal" | "hex_string_literal" => false,
        _ => is_dynamic_type_text(node_text(&node, source)),
    }
}

fn is_dynamic_type_text(text: &str) -> bool {
    if text.contains("bytes32") || text.contains("uint") || text == "address" {
        return false;
    }

    text.contains("string")
        || (text.contains("bytes") && !text.contains("bytes32"))
        || text.contains("[]")
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

    fn first_function<'a>(source: &str, tree: &'a tree_sitter::Tree) -> Node<'a> {
        find_node(tree.root_node(), "function_definition")
            .or_else(|| find_node(tree.root_node(), "constructor"))
            .unwrap_or_else(|| panic!("missing function in {source}"))
    }

    fn find_node<'a>(node: Node<'a>, kind: &str) -> Option<Node<'a>> {
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

    #[test]
    fn test_detects_unchecked_arithmetic() {
        let source = "pragma solidity 0.8.20; contract C { function f(uint x) public pure { unchecked { x + 1; } } }";
        let ast = parse_source(source, Path::new("C.sol")).unwrap();
        let patterns = detect_advanced_patterns(
            first_function(source, &ast.tree),
            source,
            &Default::default(),
        );
        assert!(patterns.contains(&"unchecked-arithmetic".to_string()));
    }

    #[test]
    fn test_detects_encode_packed_collision_for_dynamic_args() {
        let source = "pragma solidity 0.8.20; contract C { function f(string memory a, string memory b) internal pure returns (bytes memory) { return abi.encodePacked(a, b); } }";
        let ast = parse_source(source, Path::new("C.sol")).unwrap();
        let patterns = detect_advanced_patterns(
            first_function(source, &ast.tree),
            source,
            &Default::default(),
        );
        assert!(patterns.contains(&"encode-packed-collision".to_string()));
    }

    #[test]
    fn test_skips_encode_packed_for_fixed_width_args() {
        let source = "pragma solidity 0.8.20; contract C { function f(uint256 a, address b) internal pure returns (bytes memory) { return abi.encodePacked(a, b); } }";
        let ast = parse_source(source, Path::new("C.sol")).unwrap();
        let patterns = detect_advanced_patterns(
            first_function(source, &ast.tree),
            source,
            &Default::default(),
        );
        assert!(!patterns.contains(&"encode-packed-collision".to_string()));
    }
}
