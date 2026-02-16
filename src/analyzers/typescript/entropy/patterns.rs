//! Pattern detection for JavaScript/TypeScript entropy analysis
//!
//! Detects repetitive JS/TS patterns that indicate low genuine complexity:
//! - Validation guards: `if (!x) throw/return`
//! - Similar object access patterns
//! - Method chains: `.map()`, `.filter()`, `.forEach()` sequences
//! - Try-catch blocks with similar error handling

use crate::complexity::entropy_core::PatternMetrics;
use std::collections::HashMap;
use tree_sitter::Node;

/// Detect JavaScript/TypeScript specific patterns
pub fn detect_js_patterns(node: &Node, source: &str) -> PatternMetrics {
    let mut patterns = PatternMetrics::new();
    let mut pattern_counts: HashMap<String, usize> = HashMap::new();

    detect_patterns_recursive(node, source, &mut pattern_counts);

    patterns.total_patterns = pattern_counts.values().sum();
    patterns.unique_patterns = pattern_counts.len();
    patterns.calculate_repetition();

    patterns
}

fn detect_patterns_recursive(node: &Node, source: &str, patterns: &mut HashMap<String, usize>) {
    let kind = node.kind();

    match kind {
        // Validation guard pattern: if (!x) throw/return
        "if_statement" => {
            if is_validation_guard(node, source) {
                *patterns.entry("validation_guard".to_string()).or_insert(0) += 1;
            }
            if is_null_check(node, source) {
                *patterns.entry("null_check".to_string()).or_insert(0) += 1;
            }
        }

        // Switch case patterns
        "switch_case" => {
            let pattern_key = get_case_pattern(node, source);
            *patterns.entry(pattern_key).or_insert(0) += 1;
        }

        // Try-catch patterns
        "try_statement" => {
            let pattern_key = get_try_pattern(node, source);
            *patterns.entry(pattern_key).or_insert(0) += 1;
        }

        // Method chain patterns
        "call_expression" => {
            if let Some(pattern) = get_method_chain_pattern(node, source) {
                *patterns.entry(pattern).or_insert(0) += 1;
            }
        }

        // Object property access patterns
        "member_expression" => {
            let pattern_key = get_member_pattern(node, source);
            *patterns.entry(pattern_key).or_insert(0) += 1;
        }

        // Assignment patterns (common in validation/initialization)
        "assignment_expression" => {
            let pattern_key = get_assignment_pattern(node, source);
            *patterns.entry(pattern_key).or_insert(0) += 1;
        }

        // Return statement patterns
        "return_statement" => {
            let pattern_key = get_return_pattern(node, source);
            *patterns.entry(pattern_key).or_insert(0) += 1;
        }

        _ => {}
    }

    // Recurse into children
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        detect_patterns_recursive(&child, source, patterns);
    }
}

/// Check if an if statement is a validation guard pattern
fn is_validation_guard(node: &Node, source: &str) -> bool {
    // Look for pattern: if (!x) { throw/return }
    if let Some(condition) = node.child_by_field_name("condition") {
        let cond_text = node_text(&condition, source);
        let is_negation = cond_text.contains('!') || cond_text.contains("===");

        if let Some(consequence) = node.child_by_field_name("consequence") {
            let body_text = node_text(&consequence, source);
            let is_early_exit = body_text.contains("throw")
                || body_text.contains("return") && body_text.len() < 100;

            return is_negation && is_early_exit;
        }
    }
    false
}

/// Check if an if statement is a null/undefined check
fn is_null_check(node: &Node, source: &str) -> bool {
    if let Some(condition) = node.child_by_field_name("condition") {
        let cond_text = node_text(&condition, source);
        cond_text.contains("null")
            || cond_text.contains("undefined")
            || cond_text.contains("=== null")
            || cond_text.contains("!== null")
            || cond_text.contains("== null")
            || cond_text.contains("!= null")
    } else {
        false
    }
}

/// Get a normalized pattern key for switch cases
fn get_case_pattern(node: &Node, source: &str) -> String {
    // Normalize case body to detect similar case handling
    if let Some(body) = get_case_body(node) {
        let structure = categorize_statement_structure(&body, source);
        format!("switch_case:{}", structure)
    } else {
        "switch_case:empty".to_string()
    }
}

fn get_case_body<'a>(node: &'a Node<'a>) -> Option<Node<'a>> {
    let mut cursor = node.walk();
    // Case body is everything after the colon
    for child in node.children(&mut cursor) {
        match child.kind() {
            "statement_block" | "expression_statement" | "return_statement" => {
                return Some(child);
            }
            _ => continue,
        }
    }
    None
}

/// Get a normalized pattern key for try statements
fn get_try_pattern(node: &Node, source: &str) -> String {
    let has_catch = node.child_by_field_name("handler").is_some();
    let has_finally = node.child_by_field_name("finalizer").is_some();

    if let Some(handler) = node.child_by_field_name("handler") {
        if let Some(body) = handler.child_by_field_name("body") {
            let structure = categorize_statement_structure(&body, source);
            return format!("try:catch:{}", structure);
        }
    }

    match (has_catch, has_finally) {
        (true, true) => "try:catch_finally".to_string(),
        (true, false) => "try:catch".to_string(),
        (false, true) => "try:finally".to_string(),
        (false, false) => "try:empty".to_string(),
    }
}

/// Get method chain pattern (map, filter, reduce sequences)
fn get_method_chain_pattern(node: &Node, source: &str) -> Option<String> {
    let mut chain = Vec::new();
    collect_method_chain(node, source, &mut chain);

    if chain.len() >= 2 {
        Some(format!("method_chain:{}", chain.join("_")))
    } else {
        None
    }
}

fn collect_method_chain(node: &Node, source: &str, chain: &mut Vec<String>) {
    if node.kind() == "call_expression" {
        if let Some(func) = node.child_by_field_name("function") {
            if func.kind() == "member_expression" {
                if let Some(prop) = func.child_by_field_name("property") {
                    let method_name = node_text(&prop, source);
                    match method_name {
                        "map" | "filter" | "reduce" | "forEach" | "find" | "some" | "every"
                        | "flatMap" | "then" | "catch" | "finally" => {
                            chain.push(method_name.to_string());
                        }
                        _ => {}
                    }

                    // Check if the object is also a call expression (chained)
                    if let Some(obj) = func.child_by_field_name("object") {
                        collect_method_chain(&obj, source, chain);
                    }
                }
            }
        }
    }
}

/// Get member expression pattern
fn get_member_pattern(node: &Node, source: &str) -> String {
    // Count depth and get property access pattern
    let mut depth = 0;
    let mut current = *node;

    while current.kind() == "member_expression" {
        depth += 1;
        if let Some(obj) = current.child_by_field_name("object") {
            current = obj;
        } else {
            break;
        }
    }

    if depth > 2 {
        format!("deep_member_access:{}", depth)
    } else {
        let text = node_text(node, source);
        if text.contains("this.") {
            "member:this".to_string()
        } else {
            "member:simple".to_string()
        }
    }
}

/// Get assignment pattern
fn get_assignment_pattern(node: &Node, source: &str) -> String {
    if let Some(left) = node.child_by_field_name("left") {
        let left_kind = left.kind();
        match left_kind {
            "member_expression" => {
                let text = node_text(&left, source);
                if text.starts_with("this.") {
                    return "assign:this_property".to_string();
                }
                "assign:member".to_string()
            }
            "identifier" => "assign:variable".to_string(),
            "array_pattern" | "object_pattern" => "assign:destructuring".to_string(),
            _ => "assign:other".to_string(),
        }
    } else {
        "assign:unknown".to_string()
    }
}

/// Get return statement pattern
fn get_return_pattern(node: &Node, _source: &str) -> String {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "object" => return "return:object".to_string(),
            "array" => return "return:array".to_string(),
            "call_expression" => return "return:call".to_string(),
            "identifier" => return "return:identifier".to_string(),
            "member_expression" => return "return:member".to_string(),
            "binary_expression" | "ternary_expression" | "conditional_expression" => {
                return "return:expression".to_string();
            }
            "null" | "undefined" => return "return:nullish".to_string(),
            "true" | "false" => return "return:boolean".to_string(),
            "number" | "string" => return "return:literal".to_string(),
            _ => continue,
        }
    }
    "return:void".to_string()
}

/// Categorize statement structure for pattern matching
fn categorize_statement_structure(node: &Node, source: &str) -> String {
    let text = node_text(node, source);
    let len = text.len();

    // Look at what the statement does
    if text.contains("throw") {
        return "throw".to_string();
    }
    if text.contains("return") {
        if len < 20 {
            return "return_simple".to_string();
        }
        return "return_complex".to_string();
    }
    if text.contains("console.") {
        return "console".to_string();
    }
    if text.contains("await") {
        return "await".to_string();
    }

    if len < 30 {
        "simple".to_string()
    } else if len < 100 {
        "medium".to_string()
    } else {
        "complex".to_string()
    }
}

/// Extract text from a tree-sitter node
fn node_text<'a>(node: &Node, source: &'a str) -> &'a str {
    let start = node.start_byte();
    let end = node.end_byte();
    &source[start..end]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzers::typescript::parser::parse_source;
    use crate::core::ast::JsLanguageVariant;
    use std::path::PathBuf;

    fn parse_js(source: &str) -> tree_sitter::Tree {
        let path = PathBuf::from("test.js");
        let ast = parse_source(source, &path, JsLanguageVariant::JavaScript).unwrap();
        ast.tree
    }

    #[test]
    fn test_detect_validation_guards() {
        let source = r#"
function validate(x, y, z) {
    if (!x) throw new Error("x is required");
    if (!y) throw new Error("y is required");
    if (!z) throw new Error("z is required");
    return x + y + z;
}
"#;
        let tree = parse_js(source);
        let patterns = detect_js_patterns(&tree.root_node(), source);

        // Should detect repetitive validation guards
        assert!(
            patterns.repetition_ratio > 0.0,
            "Should detect repetitive validation guards: {:?}",
            patterns
        );
    }

    #[test]
    fn test_detect_method_chains() {
        let source = r#"
const result = items
    .filter(x => x.active)
    .map(x => x.value)
    .reduce((a, b) => a + b, 0);
"#;
        let tree = parse_js(source);
        let patterns = detect_js_patterns(&tree.root_node(), source);

        assert!(
            patterns.total_patterns > 0,
            "Should detect method chain patterns"
        );
    }

    #[test]
    fn test_detect_similar_switch_cases() {
        let source = r#"
function handleAction(action) {
    switch (action.type) {
        case 'INCREMENT': return state + 1;
        case 'DECREMENT': return state - 1;
        case 'RESET': return 0;
        default: return state;
    }
}
"#;
        let tree = parse_js(source);
        let patterns = detect_js_patterns(&tree.root_node(), source);

        // Should detect similar case patterns
        assert!(
            patterns.unique_patterns < patterns.total_patterns || patterns.total_patterns > 0,
            "Should detect switch case patterns: {:?}",
            patterns
        );
    }

    #[test]
    fn test_detect_try_catch_patterns() {
        let source = r#"
async function fetchData() {
    try {
        const a = await fetch('/api/a');
    } catch (e) {
        console.error(e);
    }
    try {
        const b = await fetch('/api/b');
    } catch (e) {
        console.error(e);
    }
}
"#;
        let tree = parse_js(source);
        let patterns = detect_js_patterns(&tree.root_node(), source);

        assert!(
            patterns.repetition_ratio > 0.0,
            "Should detect repetitive try-catch patterns"
        );
    }

    #[test]
    fn test_null_check_detection() {
        let source = r#"
function process(a, b, c) {
    if (a === null) return;
    if (b == null) return;
    if (c !== null) {
        doSomething(c);
    }
}
"#;
        let tree = parse_js(source);
        let patterns = detect_js_patterns(&tree.root_node(), source);

        assert!(
            patterns.total_patterns >= 3,
            "Should detect null check patterns"
        );
    }
}
