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
        config.security.encode_packed_collision && text.contains("abi.encodePacked"),
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
    fn test_safe_transfer_wrapper_not_flagged() {
        let source = "pragma solidity 0.8.20; contract C { function f(IERC20 token, address to) public { token.safeTransfer(to, 1); } }";
        let ast = parse_source(source, Path::new("C.sol")).unwrap();
        let patterns = detect_advanced_patterns(
            first_function(source, &ast.tree),
            source,
            &Default::default(),
        );
        assert!(!patterns.contains(&"unsafe-erc20-transfer".to_string()));
    }
}
