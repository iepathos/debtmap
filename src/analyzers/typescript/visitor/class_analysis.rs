//! Class analysis for TypeScript/JavaScript
//!
//! Analyzes ES6 classes for complexity and pattern detection.

use crate::analyzers::typescript::parser::{node_line, node_text};
use crate::core::ast::TypeScriptAst;
use tree_sitter::Node;

/// Information about a class definition
#[derive(Debug, Clone)]
pub struct ClassInfo {
    /// Class name
    pub name: String,
    /// Line number
    pub line: usize,
    /// Base class (extends)
    pub extends: Option<String>,
    /// Implemented interfaces (TypeScript only)
    pub implements: Vec<String>,
    /// Number of methods
    pub method_count: usize,
    /// Number of properties
    pub property_count: usize,
    /// Whether the class has a constructor
    pub has_constructor: bool,
    /// Is this class exported
    pub is_exported: bool,
}

/// Extract class information from an AST
pub fn extract_classes(ast: &TypeScriptAst) -> Vec<ClassInfo> {
    let mut classes = Vec::new();
    let root = ast.tree.root_node();

    extract_classes_recursive(&root, ast, &mut classes, false);

    classes
}

fn extract_classes_recursive(
    node: &Node,
    ast: &TypeScriptAst,
    classes: &mut Vec<ClassInfo>,
    is_exported: bool,
) {
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        match child.kind() {
            "class_declaration" | "class" => {
                if let Some(info) = analyze_class(&child, ast, is_exported) {
                    classes.push(info);
                }
            }
            "export_statement" => {
                extract_classes_recursive(&child, ast, classes, true);
            }
            _ => {
                extract_classes_recursive(&child, ast, classes, is_exported);
            }
        }
    }
}

fn analyze_class(node: &Node, ast: &TypeScriptAst, is_exported: bool) -> Option<ClassInfo> {
    let name = node
        .child_by_field_name("name")
        .map(|n| node_text(&n, &ast.source).to_string())?;

    let line = node_line(node);

    let extends = extract_extends(node, ast);
    let implements = extract_implements(node, ast);

    let body = node
        .children(&mut node.walk())
        .find(|c| c.kind() == "class_body");

    let (method_count, property_count, has_constructor) = if let Some(body) = body {
        count_members(&body, ast)
    } else {
        (0, 0, false)
    };

    Some(ClassInfo {
        name,
        line,
        extends,
        implements,
        method_count,
        property_count,
        has_constructor,
        is_exported,
    })
}

fn extract_extends(node: &Node, ast: &TypeScriptAst) -> Option<String> {
    // Look for class_heritage which contains the extends clause
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if child.kind() == "class_heritage" {
            // In tree-sitter-javascript, class_heritage directly contains:
            // - "extends" keyword
            // - identifier (the base class name)
            let mut heritage_cursor = child.walk();
            for heritage_child in child.children(&mut heritage_cursor) {
                // Look for identifier or type_identifier (for TS)
                if heritage_child.kind() == "identifier"
                    || heritage_child.kind() == "type_identifier"
                {
                    return Some(node_text(&heritage_child, &ast.source).to_string());
                }
                // Also handle extends_clause for TypeScript
                if heritage_child.kind() == "extends_clause" {
                    if let Some(value) = heritage_child.child_by_field_name("value") {
                        return Some(node_text(&value, &ast.source).to_string());
                    }
                    let mut extends_cursor = heritage_child.walk();
                    for extends_child in heritage_child.children(&mut extends_cursor) {
                        if extends_child.kind() == "identifier"
                            || extends_child.kind() == "type_identifier"
                        {
                            return Some(node_text(&extends_child, &ast.source).to_string());
                        }
                    }
                }
            }
        }
    }

    None
}

fn extract_implements(node: &Node, ast: &TypeScriptAst) -> Vec<String> {
    let mut implements = Vec::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if child.kind() == "class_heritage" {
            let mut heritage_cursor = child.walk();
            for heritage_child in child.children(&mut heritage_cursor) {
                if heritage_child.kind() == "implements_clause" {
                    let mut impl_cursor = heritage_child.walk();
                    for impl_child in heritage_child.children(&mut impl_cursor) {
                        if impl_child.kind() == "type_identifier"
                            || impl_child.kind() == "identifier"
                        {
                            implements.push(node_text(&impl_child, &ast.source).to_string());
                        }
                    }
                }
            }
        }
    }

    implements
}

fn count_members(body: &Node, ast: &TypeScriptAst) -> (usize, usize, bool) {
    let mut methods = 0;
    let mut properties = 0;
    let mut has_constructor = false;

    let mut cursor = body.walk();

    for child in body.children(&mut cursor) {
        match child.kind() {
            "method_definition" => {
                methods += 1;
                // Check if it's a constructor
                if let Some(name) = child.child_by_field_name("name") {
                    if node_text(&name, &ast.source) == "constructor" {
                        has_constructor = true;
                    }
                }
            }
            "public_field_definition" | "field_definition" | "property_definition" => {
                properties += 1;
            }
            _ => {}
        }
    }

    (methods, properties, has_constructor)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzers::typescript::parser::parse_source;
    use crate::core::ast::JsLanguageVariant;
    use std::path::PathBuf;

    #[test]
    fn test_extract_simple_class() {
        let source = r#"
class Greeter {
    constructor(name) {
        this.name = name;
    }

    greet() {
        return `Hello ${this.name}`;
    }
}
"#;
        let path = PathBuf::from("test.js");
        let ast = parse_source(source, &path, JsLanguageVariant::JavaScript).unwrap();

        let classes = extract_classes(&ast);

        assert_eq!(classes.len(), 1);
        assert_eq!(classes[0].name, "Greeter");
        assert!(classes[0].has_constructor);
        assert!(classes[0].method_count >= 2); // constructor + greet
    }

    #[test]
    fn test_extract_class_with_extends() {
        let source = r#"
class Animal {
    speak() {}
}

class Dog extends Animal {
    bark() {}
}
"#;
        let path = PathBuf::from("test.js");
        let ast = parse_source(source, &path, JsLanguageVariant::JavaScript).unwrap();

        let classes = extract_classes(&ast);

        assert_eq!(classes.len(), 2);
        assert_eq!(classes[0].name, "Animal");
        assert!(classes[0].extends.is_none());
        assert_eq!(classes[1].name, "Dog");
        assert_eq!(classes[1].extends, Some("Animal".to_string()));
    }

    #[test]
    fn test_extract_exported_class() {
        let source = "export class PublicClass { foo() {} }";
        let path = PathBuf::from("test.js");
        let ast = parse_source(source, &path, JsLanguageVariant::JavaScript).unwrap();

        let classes = extract_classes(&ast);

        assert_eq!(classes.len(), 1);
        assert!(classes[0].is_exported);
    }
}
