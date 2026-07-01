use tree_sitter::Node;

use crate::analyzers::solidity::parser::node_text;

pub fn cyclomatic_complexity(node: Node, source: &str) -> u32 {
    1 + branch_complexity(node, source)
}

pub fn cognitive_complexity(node: Node, source: &str, depth: u32) -> u32 {
    if is_nested_callable(node) {
        return 0;
    }

    if node.kind() == "inline_assembly" {
        return 0;
    }

    let branch_cost = if is_branch_node(node, source) {
        1 + depth
    } else {
        0
    };
    let next_depth = if is_nesting_node(node) {
        depth + 1
    } else {
        depth
    };

    branch_cost
        + boolean_operator_count(node, source)
        + child_sum_with_depth(node, source, next_depth)
}

pub fn max_nesting(node: Node, depth: u32) -> u32 {
    if is_nested_callable(node) || node.kind() == "inline_assembly" {
        return depth;
    }

    let next_depth = if is_nesting_node(node) {
        depth + 1
    } else {
        depth
    };
    let child_max = children(node)
        .into_iter()
        .map(|child| max_nesting(child, next_depth))
        .max()
        .unwrap_or(next_depth);

    next_depth.max(child_max)
}

pub fn function_length(node: Node) -> usize {
    node.end_position().row + 1 - node.start_position().row
}

fn branch_complexity(node: Node, source: &str) -> u32 {
    if is_nested_callable(node) || node.kind() == "inline_assembly" {
        return 0;
    }

    let current = u32::from(is_branch_node(node, source)) + boolean_operator_count(node, source);
    current + child_sum(node, source)
}

fn is_branch_node(node: Node, source: &str) -> bool {
    matches!(
        node.kind(),
        "if_statement"
            | "for_statement"
            | "while_statement"
            | "do_while_statement"
            | "ternary_expression"
            | "try_statement"
            | "catch"
    ) || is_guard_call(node, source)
}

fn is_guard_call(node: Node, source: &str) -> bool {
    if node.kind() != "call_expression" {
        return false;
    }

    node.child_by_field_name("function")
        .map(|function| {
            matches!(
                function.kind(),
                "identifier" | "member_expression" | "builtin"
            )
        })
        .unwrap_or(false)
        && guard_call_name(node, source)
            .is_some_and(|name| matches!(name.as_str(), "require" | "assert" | "revert"))
}

fn guard_call_name(node: Node, source: &str) -> Option<String> {
    let function = node.child_by_field_name("function")?;
    match function.kind() {
        "identifier" => Some(node_text(&function, source).to_string()),
        "member_expression" => function
            .child_by_field_name("property")
            .map(|property| node_text(&property, source).to_string()),
        _ => None,
    }
}

fn is_nesting_node(node: Node) -> bool {
    matches!(
        node.kind(),
        "if_statement"
            | "for_statement"
            | "while_statement"
            | "do_while_statement"
            | "try_statement"
    )
}

fn is_nested_callable(node: Node) -> bool {
    matches!(
        node.kind(),
        "function_definition" | "modifier_definition" | "constructor" | "fallback" | "receive"
    )
}

fn boolean_operator_count(node: Node, source: &str) -> u32 {
    if node.kind() != "binary_expression" {
        return 0;
    }

    children(node)
        .into_iter()
        .filter(|child| {
            let text = node_text(child, source);
            text == "&&" || text == "||"
        })
        .count() as u32
}

fn child_sum(node: Node, source: &str) -> u32 {
    children(node)
        .into_iter()
        .map(|child| branch_complexity(child, source))
        .sum()
}

fn child_sum_with_depth(node: Node, source: &str, depth: u32) -> u32 {
    children(node)
        .into_iter()
        .map(|child| cognitive_complexity(child, source, depth))
        .sum()
}

fn children(node: Node) -> Vec<Node> {
    let mut cursor = node.walk();
    node.children(&mut cursor).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzers::solidity::parser::parse_source;
    use std::path::PathBuf;

    fn function_body(root: Node) -> Option<Node> {
        if root.kind() == "function_definition" {
            return root.child_by_field_name("body");
        }

        let mut cursor = root.walk();
        for child in root.children(&mut cursor) {
            if let Some(body) = function_body(child) {
                return Some(body);
            }
        }
        None
    }

    #[test]
    fn test_cyclomatic_counts_if_and_for() {
        let source = r#"contract C {
            function f(bool a) public {
                if (a) {}
                for (uint i = 0; i < 10; i++) {}
            }
        }"#;
        let ast = parse_source(source, &PathBuf::from("Test.sol")).unwrap();
        let body = function_body(ast.tree.root_node()).expect("function body");
        assert!(cyclomatic_complexity(body, source) >= 3);
    }

    #[test]
    fn test_cognitive_increases_with_nesting() {
        let source = r#"contract C {
            function f(bool a, bool b) public {
                if (a) {
                    if (b) {}
                }
            }
        }"#;
        let ast = parse_source(source, &PathBuf::from("Test.sol")).unwrap();
        let body = function_body(ast.tree.root_node()).expect("function body");
        assert!(cognitive_complexity(body, source, 0) >= 3);
    }
}
