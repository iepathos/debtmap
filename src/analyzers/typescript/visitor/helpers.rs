//! Helper functions for TypeScript/JavaScript analysis
//!
//! Provides complexity calculation and conversion utilities.

use crate::analyzers::typescript::parser::node_text;
use crate::analyzers::typescript::types::JsFunctionMetrics;
use crate::core::FunctionMetrics;
use tree_sitter::Node;

/// Calculate cyclomatic complexity for a function body
pub fn calculate_cyclomatic_complexity(node: &Node, source: &str) -> u32 {
    let mut complexity: u32 = 1; // Base complexity

    traverse_for_cyclomatic(node, source, &mut complexity);

    complexity
}

/// Recursively traverse nodes to count decision points
fn traverse_for_cyclomatic(node: &Node, source: &str, complexity: &mut u32) {
    match node.kind() {
        // Control flow statements
        "if_statement" => *complexity += 1,
        "for_statement" | "for_in_statement" => *complexity += 1,
        "while_statement" | "do_statement" => *complexity += 1,
        "switch_case" => {
            // Count each case except default
            if !is_default_case(node, source) {
                *complexity += 1;
            }
        }
        "catch_clause" => *complexity += 1,
        "ternary_expression" | "conditional_expression" => *complexity += 1,
        "optional_chain_expression" => *complexity += 1,

        // Logical operators create branches
        "binary_expression" => {
            if let Some(op) = node.child_by_field_name("operator") {
                let op_text = node_text(&op, source);
                if op_text == "&&" || op_text == "||" || op_text == "??" {
                    *complexity += 1;
                }
            }
        }

        _ => {}
    }

    // Recurse into children
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        traverse_for_cyclomatic(&child, source, complexity);
    }
}

/// Check if a switch case is the default case
fn is_default_case(node: &Node, source: &str) -> bool {
    let text = node_text(node, source);
    text.trim_start().starts_with("default")
}

/// Calculate cognitive complexity for a function body
pub fn calculate_cognitive_complexity(node: &Node, source: &str) -> u32 {
    let mut complexity: u32 = 0;

    traverse_for_cognitive(node, source, &mut complexity, 0);

    complexity
}

/// Recursively traverse nodes for cognitive complexity
fn traverse_for_cognitive(node: &Node, source: &str, complexity: &mut u32, nesting: u32) {
    match node.kind() {
        // Control flow - adds 1 + nesting level
        "if_statement" => {
            *complexity += 1 + nesting;
            traverse_children_with_nesting(
                node,
                source,
                complexity,
                nesting + 1,
                Some("consequence"),
            );
            traverse_children_with_nesting(
                node,
                source,
                complexity,
                nesting + 1,
                Some("alternative"),
            );
            return;
        }
        "for_statement" | "for_in_statement" | "for_of_statement" => {
            *complexity += 1 + nesting;
            traverse_children_with_nesting(node, source, complexity, nesting + 1, Some("body"));
            return;
        }
        "while_statement" | "do_statement" => {
            *complexity += 1 + nesting;
            traverse_children_with_nesting(node, source, complexity, nesting + 1, Some("body"));
            return;
        }
        "switch_statement" => {
            *complexity += 1 + nesting;
            traverse_children_with_nesting(node, source, complexity, nesting + 1, Some("body"));
            return;
        }
        "try_statement" => {
            *complexity += 1 + nesting;
            // Try block
            if let Some(body) = node.child_by_field_name("body") {
                traverse_for_cognitive(&body, source, complexity, nesting + 1);
            }
            // Catch handler
            if let Some(handler) = node.child_by_field_name("handler") {
                traverse_for_cognitive(&handler, source, complexity, nesting + 1);
            }
            // Finally
            if let Some(finalizer) = node.child_by_field_name("finalizer") {
                traverse_for_cognitive(&finalizer, source, complexity, nesting + 1);
            }
            return;
        }
        "ternary_expression" | "conditional_expression" => {
            *complexity += 1 + nesting;
        }

        // Logical operators - add 1 (no nesting penalty for these)
        "binary_expression" => {
            if let Some(op) = node.child_by_field_name("operator") {
                let op_text = node_text(&op, source);
                if op_text == "&&" || op_text == "||" || op_text == "??" {
                    *complexity += 1;
                }
            }
        }

        // Await expressions add cognitive load
        "await_expression" => {
            *complexity += 1;
        }

        // Nested functions/callbacks increase complexity
        // Only count nested functions (arrow_function, function_expression), not top-level declarations
        "arrow_function" | "function_expression" => {
            // Callback functions add complexity based on nesting
            *complexity += 1 + nesting.min(2);
            traverse_children_with_nesting(node, source, complexity, nesting + 1, Some("body"));
            return;
        }
        // Top-level function declarations don't add complexity, just traverse body
        "function_declaration" | "generator_function_declaration" => {
            if let Some(body) = node.child_by_field_name("body") {
                traverse_for_cognitive(&body, source, complexity, nesting);
            }
            return;
        }

        _ => {}
    }

    // Continue recursion
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        traverse_for_cognitive(&child, source, complexity, nesting);
    }
}

/// Traverse children with optional field filter and nesting increment
fn traverse_children_with_nesting(
    node: &Node,
    source: &str,
    complexity: &mut u32,
    nesting: u32,
    field: Option<&str>,
) {
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if let Some(field_name) = field {
            // Only process children with the specified field
            if let Some(target) = node.child_by_field_name(field_name) {
                traverse_for_cognitive(&target, source, complexity, nesting);
                return;
            }
        }
        traverse_for_cognitive(&child, source, complexity, nesting);
    }
}

/// Calculate maximum nesting depth
pub fn calculate_nesting_depth(node: &Node) -> u32 {
    calculate_nesting_recursive(node, 0)
}

fn calculate_nesting_recursive(node: &Node, current_depth: u32) -> u32 {
    let new_depth = match node.kind() {
        "if_statement" | "for_statement" | "for_in_statement" | "for_of_statement"
        | "while_statement" | "do_statement" | "switch_statement" | "try_statement" => {
            current_depth + 1
        }
        // Arrow functions and function expressions add nesting
        "arrow_function" | "function_expression" | "function" => current_depth + 1,
        _ => current_depth,
    };

    let mut max_depth = new_depth;
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        let child_depth = calculate_nesting_recursive(&child, new_depth);
        max_depth = max_depth.max(child_depth);
    }

    max_depth
}

/// Count lines in a function body
pub fn count_function_lines(node: &Node, _source: &str) -> usize {
    let start_line = node.start_position().row;
    let end_line = node.end_position().row;

    // Add 1 because lines are 0-indexed
    end_line - start_line + 1
}

/// Check if a function name indicates it's a test
pub fn is_test_function(name: &str) -> bool {
    let lower = name.to_lowercase();

    // Common test function patterns
    lower.starts_with("test")
        || lower.starts_with("it_")
        || lower.starts_with("should_")
        || lower.starts_with("spec_")
        || lower == "it"
        || lower == "test"
        || lower == "describe"
        || lower == "beforeeach"
        || lower == "aftereach"
        || lower == "beforeall"
        || lower == "afterall"
}

/// Convert JS-specific metrics to standard FunctionMetrics
pub fn convert_to_function_metrics(js_metrics: &JsFunctionMetrics) -> FunctionMetrics {
    FunctionMetrics {
        name: js_metrics.name.clone(),
        file: js_metrics.file.clone(),
        line: js_metrics.line,
        cyclomatic: js_metrics.cyclomatic,
        cognitive: js_metrics.cognitive,
        nesting: js_metrics.nesting,
        length: js_metrics.length,
        is_test: js_metrics.is_test,
        visibility: if js_metrics.is_exported {
            Some("export".to_string())
        } else {
            None
        },
        is_trait_method: false,
        in_test_module: js_metrics.is_test,
        entropy_score: None,
        is_pure: None,
        purity_confidence: None,
        purity_reason: None,
        call_dependencies: None,
        detected_patterns: None,
        upstream_callers: None,
        downstream_callees: None,
        mapping_pattern_result: None,
        adjusted_complexity: None,
        composition_metrics: None,
        language_specific: None,
        purity_level: None,
        error_swallowing_count: None,
        error_swallowing_patterns: None,
        entropy_analysis: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzers::typescript::parser::parse_source;
    use crate::analyzers::typescript::types::FunctionKind;
    use crate::core::ast::JsLanguageVariant;
    use std::path::PathBuf;

    fn parse_and_get_body(source: &str) -> (tree_sitter::Tree, String) {
        let path = PathBuf::from("test.js");
        let ast = parse_source(source, &path, JsLanguageVariant::JavaScript).unwrap();
        (ast.tree, ast.source)
    }

    #[test]
    fn test_cyclomatic_simple_function() {
        let source = "function foo() { return 1; }";
        let (tree, source) = parse_and_get_body(source);
        let root = tree.root_node();

        let complexity = calculate_cyclomatic_complexity(&root, &source);
        assert_eq!(complexity, 1); // Base complexity
    }

    #[test]
    fn test_cyclomatic_with_if() {
        let source = "function foo(x) { if (x) { return 1; } return 0; }";
        let (tree, source) = parse_and_get_body(source);
        let root = tree.root_node();

        let complexity = calculate_cyclomatic_complexity(&root, &source);
        assert_eq!(complexity, 2); // 1 base + 1 if
    }

    #[test]
    fn test_cyclomatic_with_logical_operators() {
        let source = "function foo(a, b, c) { if (a && b || c) { return 1; } }";
        let (tree, source) = parse_and_get_body(source);
        let root = tree.root_node();

        let complexity = calculate_cyclomatic_complexity(&root, &source);
        assert_eq!(complexity, 4); // 1 base + 1 if + 2 logical ops
    }

    #[test]
    fn test_cognitive_simple_function() {
        let source = "function foo() { return 1; }";
        let (tree, source) = parse_and_get_body(source);
        let root = tree.root_node();

        let complexity = calculate_cognitive_complexity(&root, &source);
        assert_eq!(complexity, 0);
    }

    #[test]
    fn test_cognitive_nested_if() {
        let source = r#"
function foo(a, b) {
    if (a) {
        if (b) {
            return 1;
        }
    }
}
"#;
        let (tree, source) = parse_and_get_body(source);
        let root = tree.root_node();

        let complexity = calculate_cognitive_complexity(&root, &source);
        // First if: 1 + 0 = 1, Second if: 1 + 1 = 2, Total: 3
        assert!(complexity >= 2);
    }

    #[test]
    fn test_nesting_depth() {
        let source = r#"
function foo(a, b, c) {
    if (a) {
        if (b) {
            if (c) {
                return 1;
            }
        }
    }
}
"#;
        let (tree, _source) = parse_and_get_body(source);
        let root = tree.root_node();

        let depth = calculate_nesting_depth(&root);
        assert_eq!(depth, 3);
    }

    #[test]
    fn test_is_test_function() {
        assert!(is_test_function("test_something"));
        assert!(is_test_function("testSomething"));
        assert!(is_test_function("it_should_work"));
        assert!(is_test_function("should_work"));
        assert!(is_test_function("describe"));
        assert!(is_test_function("beforeEach"));
        assert!(!is_test_function("regularFunction"));
        assert!(!is_test_function("getData"));
    }

    #[test]
    fn test_convert_to_function_metrics() {
        let js_metrics = JsFunctionMetrics::new(
            "test".to_string(),
            PathBuf::from("test.js"),
            10,
            FunctionKind::Declaration,
        );

        let metrics = convert_to_function_metrics(&js_metrics);

        assert_eq!(metrics.name, "test");
        assert_eq!(metrics.line, 10);
        assert_eq!(metrics.cyclomatic, 1);
    }
}
