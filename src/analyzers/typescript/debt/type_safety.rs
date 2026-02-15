//! TypeScript type safety debt detection
//!
//! Detects patterns that weaken TypeScript's type safety.

use crate::analyzers::typescript::parser::{node_line, node_text};
use crate::core::ast::TypeScriptAst;
use crate::core::{DebtItem, Priority};
use crate::priority::DebtType;
use tree_sitter::Node;

/// Detect TypeScript type safety issues
pub fn detect_type_safety_debt(ast: &TypeScriptAst) -> Vec<DebtItem> {
    let mut items = Vec::new();
    let root = ast.tree.root_node();

    // Count 'any' usage
    let any_locations = detect_any_usage(&root, ast);
    if any_locations.len() > 5 {
        items.push(DebtItem {
            id: format!("ts-excessive-any-{}", ast.path.display()),
            debt_type: DebtType::CodeSmell {
                smell_type: Some("excessive_any_type".to_string()),
            },
            priority: Priority::Medium,
            file: ast.path.clone(),
            line: any_locations.first().copied().unwrap_or(1),
            column: None,
            message: format!(
                "Excessive use of 'any' type ({} occurrences)",
                any_locations.len()
            ),
            context: Some(
                "Using 'any' defeats TypeScript's type checking. Consider using specific types, \
                 generics, or 'unknown' for truly dynamic values."
                    .to_string(),
            ),
        });
    }

    // Detect type assertions
    let assertion_locations = detect_type_assertions(&root, ast);
    if assertion_locations.len() > 3 {
        items.push(DebtItem {
            id: format!("ts-many-assertions-{}", ast.path.display()),
            debt_type: DebtType::CodeSmell {
                smell_type: Some("type_assertion_abuse".to_string()),
            },
            priority: Priority::Low,
            file: ast.path.clone(),
            line: assertion_locations.first().copied().unwrap_or(1),
            column: None,
            message: format!(
                "Many type assertions ({} occurrences)",
                assertion_locations.len()
            ),
            context: Some(
                "Type assertions can hide type errors. Consider improving type definitions \
                 or using type guards instead."
                    .to_string(),
            ),
        });
    }

    // Detect non-null assertions
    let non_null_locations = detect_non_null_assertions(&root, ast);
    if non_null_locations.len() > 5 {
        items.push(DebtItem {
            id: format!("ts-non-null-assertions-{}", ast.path.display()),
            debt_type: DebtType::CodeSmell {
                smell_type: Some("non_null_assertion_abuse".to_string()),
            },
            priority: Priority::Medium,
            file: ast.path.clone(),
            line: non_null_locations.first().copied().unwrap_or(1),
            column: None,
            message: format!(
                "Excessive non-null assertions ({} occurrences)",
                non_null_locations.len()
            ),
            context: Some(
                "Non-null assertions (!) can lead to runtime errors. Consider using optional \
                 chaining (?.) or proper null checks."
                    .to_string(),
            ),
        });
    }

    items
}

/// Detect uses of the 'any' type
fn detect_any_usage(node: &Node, ast: &TypeScriptAst) -> Vec<usize> {
    let mut locations = Vec::new();
    detect_any_recursive(node, ast, &mut locations);
    locations
}

fn detect_any_recursive(node: &Node, ast: &TypeScriptAst, locations: &mut Vec<usize>) {
    // Check for 'any' type annotation
    if node.kind() == "predefined_type" {
        let text = node_text(node, &ast.source);
        if text == "any" {
            locations.push(node_line(node));
        }
    }

    // Also check for type_identifier that might be 'any'
    if node.kind() == "type_identifier" {
        let text = node_text(node, &ast.source);
        if text == "any" {
            locations.push(node_line(node));
        }
    }

    // Recurse
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        detect_any_recursive(&child, ast, locations);
    }
}

/// Detect type assertions (as Type or <Type>)
fn detect_type_assertions(node: &Node, ast: &TypeScriptAst) -> Vec<usize> {
    let mut locations = Vec::new();
    detect_assertions_recursive(node, ast, &mut locations);
    locations
}

fn detect_assertions_recursive(node: &Node, ast: &TypeScriptAst, locations: &mut Vec<usize>) {
    match node.kind() {
        "as_expression" | "type_assertion" => {
            locations.push(node_line(node));
        }
        _ => {}
    }

    // Recurse
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        detect_assertions_recursive(&child, ast, locations);
    }
}

/// Detect non-null assertions (expr!)
fn detect_non_null_assertions(node: &Node, ast: &TypeScriptAst) -> Vec<usize> {
    let mut locations = Vec::new();
    detect_non_null_recursive(node, ast, &mut locations);
    locations
}

fn detect_non_null_recursive(node: &Node, ast: &TypeScriptAst, locations: &mut Vec<usize>) {
    if node.kind() == "non_null_expression" {
        locations.push(node_line(node));
    }

    // Recurse
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        detect_non_null_recursive(&child, ast, locations);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzers::typescript::parser::parse_source;
    use crate::core::ast::JsLanguageVariant;
    use std::path::PathBuf;

    #[test]
    fn test_detect_any_usage() {
        let source = r#"
function foo(x: any): any {
    const y: any = x;
    const z: any = y;
    const a: any = z;
    const b: any = a;
    return b;
}
"#;
        let path = PathBuf::from("test.ts");
        let ast = parse_source(source, &path, JsLanguageVariant::TypeScript).unwrap();

        let items = detect_type_safety_debt(&ast);

        // Should detect excessive 'any' usage
        assert!(items.iter().any(|i| i.message.contains("any")));
    }

    #[test]
    fn test_detect_type_assertions() {
        let source = r#"
const a = x as string;
const b = y as number;
const c = z as boolean;
const d = w as any;
"#;
        let path = PathBuf::from("test.ts");
        let ast = parse_source(source, &path, JsLanguageVariant::TypeScript).unwrap();

        let items = detect_type_safety_debt(&ast);

        // Should detect type assertions
        assert!(items.iter().any(|i| i.message.contains("assertions")));
    }

    #[test]
    fn test_no_debt_for_clean_code() {
        let source = r#"
function greet(name: string): string {
    return `Hello ${name}`;
}

interface User {
    name: string;
    age: number;
}
"#;
        let path = PathBuf::from("test.ts");
        let ast = parse_source(source, &path, JsLanguageVariant::TypeScript).unwrap();

        let items = detect_type_safety_debt(&ast);

        // Clean code should have no debt
        assert!(items.is_empty());
    }
}
