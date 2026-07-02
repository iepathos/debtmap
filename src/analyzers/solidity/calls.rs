//! Structured Solidity call extraction from tree-sitter AST nodes.

use crate::analyzers::solidity::parser::node_text;
use crate::core::ast::SolidityAst;
use tree_sitter::Node;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SolidityCallKind {
    Bare,
    Selector,
    TypeCast,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SolidityCallShape {
    pub kind: SolidityCallKind,
    pub method_name: String,
    pub receiver: Option<String>,
    pub cast_type: Option<String>,
    pub argument_count: usize,
    pub display: String,
}

pub fn extract_calls(body: Node, ast: &SolidityAst) -> Vec<SolidityCallShape> {
    let mut calls = Vec::new();
    collect_calls(body, ast, &mut calls);
    calls.sort_by(|a, b| a.display.cmp(&b.display));
    calls.dedup_by(|a, b| a.display == b.display);
    calls
}

pub fn call_display(call: &SolidityCallShape) -> String {
    call.display.clone()
}

fn collect_calls(node: Node, ast: &SolidityAst, calls: &mut Vec<SolidityCallShape>) {
    if is_callable_node(node) {
        return;
    }

    if node.kind() == "call_expression"
        && let Some(call) = call_shape(node, ast)
    {
        calls.push(call);
    }

    walk_children(node, |child| collect_calls(child, ast, calls));
}

fn call_shape(node: Node, ast: &SolidityAst) -> Option<SolidityCallShape> {
    let function = node.child_by_field_name("function")?;
    let display = node_text(&function, &ast.source).to_string();
    let argument_count = call_argument_count(node);

    match function.kind() {
        "identifier" => Some(parse_flat_call(&display, argument_count)),
        "member_expression" => selector_call(function, ast, argument_count),
        _ => Some(parse_flat_call(&display, argument_count)),
    }
}

fn parse_flat_call(display: &str, argument_count: usize) -> SolidityCallShape {
    if let Some((cast_type, method)) = parse_cast_selector(display) {
        return SolidityCallShape {
            kind: SolidityCallKind::TypeCast,
            method_name: method,
            receiver: None,
            cast_type: Some(cast_type),
            argument_count,
            display: display.to_string(),
        };
    }

    if let Some((receiver, method)) = display.rsplit_once('.') {
        return SolidityCallShape {
            kind: SolidityCallKind::Selector,
            method_name: method.to_string(),
            receiver: Some(receiver.to_string()),
            cast_type: None,
            argument_count,
            display: display.to_string(),
        };
    }

    SolidityCallShape {
        kind: SolidityCallKind::Bare,
        method_name: display.to_string(),
        receiver: None,
        cast_type: None,
        argument_count,
        display: display.to_string(),
    }
}

fn parse_cast_selector(display: &str) -> Option<(String, String)> {
    let (left, method) = display.rsplit_once('.')?;
    let open = left.find('(')?;
    let cast_type = left[..open].trim();
    (!cast_type.is_empty()).then(|| (cast_type.to_string(), method.to_string()))
}

fn selector_call(
    function: Node,
    ast: &SolidityAst,
    argument_count: usize,
) -> Option<SolidityCallShape> {
    let object = function.child_by_field_name("object")?;
    let property = function.child_by_field_name("property")?;
    let method_name = node_text(&property, &ast.source).to_string();
    let receiver = node_text(&object, &ast.source).to_string();
    let cast_type = cast_type_from_expression(object, ast);
    let display = format!("{receiver}.{method_name}");

    Some(SolidityCallShape {
        kind: if cast_type.is_some() {
            SolidityCallKind::TypeCast
        } else {
            SolidityCallKind::Selector
        },
        method_name,
        receiver: Some(receiver),
        cast_type,
        argument_count,
        display,
    })
}

fn cast_type_from_expression(expression: Node, ast: &SolidityAst) -> Option<String> {
    match expression.kind() {
        "type_cast_expression" => type_name_from_cast(expression, ast),
        "identifier" if node_text(&expression, &ast.source) == "this" => None,
        "identifier" => None,
        _ => None,
    }
}

fn type_name_from_cast(expression: Node, ast: &SolidityAst) -> Option<String> {
    let mut cursor = expression.walk();
    for child in expression.children(&mut cursor) {
        if child.kind() == "type_name" || child.kind() == "user_defined_type" {
            return Some(node_text(&child, &ast.source).trim().to_string());
        }
    }
    None
}

fn call_argument_count(node: Node) -> usize {
    let mut cursor = node.walk();
    node.children(&mut cursor)
        .filter(|child| child.kind() == "call_argument")
        .count()
}

fn is_callable_node(node: Node) -> bool {
    matches!(
        node.kind(),
        "function_definition"
            | "modifier_definition"
            | "constructor"
            | "constructor_definition"
            | "fallback"
            | "receive"
    )
}

fn walk_children(node: Node, mut visit: impl FnMut(Node)) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        visit(child);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzers::solidity::parser::parse_source;
    use std::path::PathBuf;

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
    fn test_extracts_bare_and_selector_calls() {
        let source = r#"pragma solidity 0.8.20;
contract Vault {
    Vault vault;
    function withdraw() public {
        settle();
        vault.settle();
    }
}"#;
        let ast = parse_source(source, &PathBuf::from("Sample.sol")).unwrap();
        let function = find_node(ast.tree.root_node(), "function_definition").unwrap();
        let body = function.child_by_field_name("body").unwrap();
        let calls = extract_calls(body, &ast);

        assert!(
            calls.iter().any(|call| {
                call.kind == SolidityCallKind::Bare && call.method_name == "settle"
            })
        );
        assert!(calls.iter().any(|call| {
            call.kind == SolidityCallKind::Selector
                && call.method_name == "settle"
                && call.receiver.as_deref() == Some("vault")
        }));
    }

    #[test]
    fn test_extracts_type_cast_selector_call() {
        let source = r#"pragma solidity 0.8.20;
contract Vault {
    function withdraw(address token, address to, uint256 amount) public {
        IERC20(token).transfer(to, amount);
    }
}"#;
        let ast = parse_source(source, &PathBuf::from("Sample.sol")).unwrap();
        let function = find_node(ast.tree.root_node(), "function_definition").unwrap();
        let body = function.child_by_field_name("body").unwrap();
        let calls = extract_calls(body, &ast);
        let transfer = calls
            .iter()
            .find(|call| call.method_name == "transfer")
            .expect("transfer call");

        assert_eq!(transfer.kind, SolidityCallKind::TypeCast);
        assert_eq!(transfer.cast_type.as_deref(), Some("IERC20"));
        assert_eq!(transfer.argument_count, 2);
    }
}
