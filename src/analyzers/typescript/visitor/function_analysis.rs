//! Function analysis for TypeScript/JavaScript
//!
//! Extracts functions from tree-sitter AST and calculates their metrics.

use crate::analyzers::typescript::parser::{node_line, node_text};
use crate::analyzers::typescript::types::{FunctionKind, JsFunctionMetrics};
use crate::core::ast::TypeScriptAst;
use tree_sitter::Node;

use super::helpers::{
    calculate_cognitive_complexity, calculate_cyclomatic_complexity, calculate_nesting_depth,
    count_function_lines, is_test_function,
};

/// Extract all functions from a TypeScript/JavaScript AST
pub fn extract_functions(
    ast: &TypeScriptAst,
    _enable_functional_analysis: bool,
) -> Vec<JsFunctionMetrics> {
    let mut functions = Vec::new();
    let root = ast.tree.root_node();

    extract_functions_recursive(&root, ast, &mut functions, false, None);

    functions
}

/// Recursively extract functions from AST nodes
fn extract_functions_recursive(
    node: &Node,
    ast: &TypeScriptAst,
    functions: &mut Vec<JsFunctionMetrics>,
    in_class: bool,
    class_name: Option<&str>,
) {
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        match child.kind() {
            // Function declarations: function foo() {}
            "function_declaration" => {
                if let Some(metrics) =
                    analyze_function_declaration(&child, ast, in_class, class_name)
                {
                    functions.push(metrics);
                }
            }

            // Generator function declarations: function* foo() {}
            "generator_function_declaration" => {
                if let Some(metrics) = analyze_generator_function(&child, ast) {
                    functions.push(metrics);
                }
            }

            // Variable declarations may contain arrow functions or function expressions
            "lexical_declaration" | "variable_declaration" => {
                extract_variable_functions(&child, ast, functions);
            }

            // Export statements may contain functions
            "export_statement" => {
                extract_export_functions(&child, ast, functions);
            }

            // Class declarations
            "class_declaration" | "class" => {
                let name = get_class_name(&child, ast);
                extract_class_methods(&child, ast, functions, name.as_deref());
            }

            // Arrow functions at module level (rare but possible)
            "arrow_function" => {
                if let Some(metrics) = analyze_arrow_function(&child, ast, None, false) {
                    functions.push(metrics);
                }
            }

            // Method definitions (object literals)
            "method_definition" => {
                if let Some(metrics) = analyze_method_definition(&child, ast, false, None) {
                    functions.push(metrics);
                }
            }

            // Continue recursion for other nodes
            _ => {
                extract_functions_recursive(&child, ast, functions, in_class, class_name);
            }
        }
    }
}

/// Analyze a function declaration node
fn analyze_function_declaration(
    node: &Node,
    ast: &TypeScriptAst,
    _in_class: bool,
    _class_name: Option<&str>,
) -> Option<JsFunctionMetrics> {
    let name = get_function_name(node, ast)?;
    let line = node_line(node);
    let body = get_function_body(node)?;

    let is_async = has_async_modifier(node);
    let kind = if is_async {
        FunctionKind::Async
    } else {
        FunctionKind::Declaration
    };

    let mut metrics = JsFunctionMetrics::new(name.clone(), ast.path.clone(), line, kind);

    metrics.cyclomatic = calculate_cyclomatic_complexity(&body, &ast.source);
    metrics.cognitive = calculate_cognitive_complexity(&body, &ast.source);
    metrics.nesting = calculate_nesting_depth(&body);
    metrics.length = count_function_lines(&body, &ast.source);
    metrics.is_async = is_async;
    metrics.is_test = is_test_function(&name);
    metrics.parameter_count = count_parameters(node);

    Some(metrics)
}

/// Analyze a generator function declaration
fn analyze_generator_function(node: &Node, ast: &TypeScriptAst) -> Option<JsFunctionMetrics> {
    let name = get_function_name(node, ast)?;
    let line = node_line(node);
    let body = get_function_body(node)?;

    let is_async = has_async_modifier(node);
    let kind = if is_async {
        FunctionKind::AsyncGenerator
    } else {
        FunctionKind::Generator
    };

    let mut metrics = JsFunctionMetrics::new(name.clone(), ast.path.clone(), line, kind);

    metrics.cyclomatic = calculate_cyclomatic_complexity(&body, &ast.source);
    metrics.cognitive = calculate_cognitive_complexity(&body, &ast.source);
    metrics.nesting = calculate_nesting_depth(&body);
    metrics.length = count_function_lines(&body, &ast.source);
    metrics.is_async = is_async;
    metrics.is_test = is_test_function(&name);
    metrics.parameter_count = count_parameters(node);

    Some(metrics)
}

/// Analyze an arrow function
fn analyze_arrow_function(
    node: &Node,
    ast: &TypeScriptAst,
    name: Option<String>,
    is_exported: bool,
) -> Option<JsFunctionMetrics> {
    let line = node_line(node);
    let func_name = name.unwrap_or_else(|| "<anonymous>".to_string());

    let mut metrics = JsFunctionMetrics::new(
        func_name.clone(),
        ast.path.clone(),
        line,
        FunctionKind::Arrow,
    );

    // Arrow functions may have expression body or block body
    let body_node = node.child_by_field_name("body")?;

    metrics.cyclomatic = calculate_cyclomatic_complexity(&body_node, &ast.source);
    metrics.cognitive = calculate_cognitive_complexity(&body_node, &ast.source);
    metrics.nesting = calculate_nesting_depth(&body_node);
    metrics.length = count_function_lines(&body_node, &ast.source);
    metrics.is_async = has_async_modifier(node);
    metrics.is_test = is_test_function(&func_name);
    metrics.is_exported = is_exported;
    metrics.parameter_count = count_arrow_parameters(node);

    Some(metrics)
}

/// Analyze a method definition (in class or object)
fn analyze_method_definition(
    node: &Node,
    ast: &TypeScriptAst,
    in_class: bool,
    class_name: Option<&str>,
) -> Option<JsFunctionMetrics> {
    let name = get_method_name(node, ast)?;
    let line = node_line(node);

    let kind = determine_method_kind(node, in_class);

    let mut metrics = JsFunctionMetrics::new(
        if let Some(cn) = class_name {
            format!("{}::{}", cn, name)
        } else {
            name.clone()
        },
        ast.path.clone(),
        line,
        kind,
    );

    if let Some(body) = get_method_body(node) {
        metrics.cyclomatic = calculate_cyclomatic_complexity(&body, &ast.source);
        metrics.cognitive = calculate_cognitive_complexity(&body, &ast.source);
        metrics.nesting = calculate_nesting_depth(&body);
        metrics.length = count_function_lines(&body, &ast.source);
    }

    metrics.is_async = has_async_modifier(node);
    metrics.is_test = is_test_function(&name);
    metrics.parameter_count = count_method_parameters(node);

    Some(metrics)
}

/// Extract functions from variable declarations
fn extract_variable_functions(
    node: &Node,
    ast: &TypeScriptAst,
    functions: &mut Vec<JsFunctionMetrics>,
) {
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if child.kind() == "variable_declarator" {
            if let Some(name_node) = child.child_by_field_name("name") {
                let name = node_text(&name_node, &ast.source).to_string();

                if let Some(value_node) = child.child_by_field_name("value") {
                    match value_node.kind() {
                        "arrow_function" => {
                            if let Some(metrics) =
                                analyze_arrow_function(&value_node, ast, Some(name), false)
                            {
                                functions.push(metrics);
                            }
                        }
                        "function_expression" | "function" => {
                            if let Some(metrics) =
                                analyze_function_expression(&value_node, ast, Some(name))
                            {
                                functions.push(metrics);
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}

/// Analyze a function expression
fn analyze_function_expression(
    node: &Node,
    ast: &TypeScriptAst,
    name: Option<String>,
) -> Option<JsFunctionMetrics> {
    let line = node_line(node);
    let func_name = name.unwrap_or_else(|| {
        // Check for inline name: const foo = function bar() {}
        get_function_name(node, ast).unwrap_or_else(|| "<anonymous>".to_string())
    });

    let body = get_function_body(node)?;
    let is_async = has_async_modifier(node);

    let kind = if is_async {
        FunctionKind::Async
    } else {
        FunctionKind::Expression
    };

    let mut metrics = JsFunctionMetrics::new(func_name.clone(), ast.path.clone(), line, kind);

    metrics.cyclomatic = calculate_cyclomatic_complexity(&body, &ast.source);
    metrics.cognitive = calculate_cognitive_complexity(&body, &ast.source);
    metrics.nesting = calculate_nesting_depth(&body);
    metrics.length = count_function_lines(&body, &ast.source);
    metrics.is_async = is_async;
    metrics.is_test = is_test_function(&func_name);
    metrics.parameter_count = count_parameters(node);

    Some(metrics)
}

/// Extract functions from export statements
fn extract_export_functions(
    node: &Node,
    ast: &TypeScriptAst,
    functions: &mut Vec<JsFunctionMetrics>,
) {
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        match child.kind() {
            "function_declaration" => {
                if let Some(mut metrics) = analyze_function_declaration(&child, ast, false, None) {
                    metrics.is_exported = true;
                    functions.push(metrics);
                }
            }
            "lexical_declaration" | "variable_declaration" => {
                let len_before = functions.len();
                extract_variable_functions(&child, ast, functions);
                // Mark newly added functions as exported
                for func in functions.iter_mut().skip(len_before) {
                    func.is_exported = true;
                }
            }
            "class_declaration" | "class" => {
                let name = get_class_name(&child, ast);
                extract_class_methods(&child, ast, functions, name.as_deref());
            }
            _ => {}
        }
    }
}

/// Extract methods from a class
fn extract_class_methods(
    node: &Node,
    ast: &TypeScriptAst,
    functions: &mut Vec<JsFunctionMetrics>,
    class_name: Option<&str>,
) {
    // Find class body
    let body = node
        .children(&mut node.walk())
        .find(|c| c.kind() == "class_body");

    if let Some(body) = body {
        let mut cursor = body.walk();

        for child in body.children(&mut cursor) {
            match child.kind() {
                "method_definition" => {
                    if let Some(metrics) = analyze_method_definition(&child, ast, true, class_name)
                    {
                        functions.push(metrics);
                    }
                }
                "public_field_definition" | "field_definition" => {
                    // Check if field has function value
                    extract_field_functions(&child, ast, functions, class_name);
                }
                _ => {}
            }
        }
    }
}

/// Extract functions from class field definitions
fn extract_field_functions(
    node: &Node,
    ast: &TypeScriptAst,
    functions: &mut Vec<JsFunctionMetrics>,
    class_name: Option<&str>,
) {
    if let Some(value) = node.child_by_field_name("value") {
        if value.kind() == "arrow_function" {
            let name = node
                .child_by_field_name("name")
                .map(|n| node_text(&n, &ast.source).to_string());

            let full_name = if let (Some(cn), Some(n)) = (class_name, &name) {
                format!("{}::{}", cn, n)
            } else {
                name.unwrap_or_else(|| "<field>".to_string())
            };

            if let Some(mut metrics) = analyze_arrow_function(&value, ast, Some(full_name), false) {
                metrics.kind = FunctionKind::Method;
                functions.push(metrics);
            }
        }
    }
}

// Helper functions

fn get_function_name(node: &Node, ast: &TypeScriptAst) -> Option<String> {
    node.child_by_field_name("name")
        .map(|n| node_text(&n, &ast.source).to_string())
}

fn get_class_name(node: &Node, ast: &TypeScriptAst) -> Option<String> {
    node.child_by_field_name("name")
        .map(|n| node_text(&n, &ast.source).to_string())
}

fn get_method_name(node: &Node, ast: &TypeScriptAst) -> Option<String> {
    node.child_by_field_name("name")
        .map(|n| node_text(&n, &ast.source).to_string())
}

fn get_function_body<'a>(node: &'a Node<'a>) -> Option<Node<'a>> {
    node.child_by_field_name("body")
}

fn get_method_body<'a>(node: &'a Node<'a>) -> Option<Node<'a>> {
    node.child_by_field_name("body")
}

fn has_async_modifier(node: &Node) -> bool {
    let mut cursor = node.walk();
    let result = node.children(&mut cursor).any(|c| c.kind() == "async");
    result
}

fn determine_method_kind(node: &Node, in_class: bool) -> FunctionKind {
    let mut cursor = node.walk();

    // Check for getter/setter
    for child in node.children(&mut cursor) {
        match child.kind() {
            "get" => return FunctionKind::Getter,
            "set" => return FunctionKind::Setter,
            _ => {}
        }
    }

    // Check for constructor
    if let Some(name) = node.child_by_field_name("name") {
        let kind = name.kind();
        if kind == "property_identifier" || kind == "identifier" {
            // We'd need to check the text but this is just for kind determination
        }
    }

    if in_class {
        FunctionKind::ClassMethod
    } else {
        FunctionKind::Method
    }
}

fn count_parameters(node: &Node) -> u32 {
    node.child_by_field_name("parameters")
        .map(|params| params.named_child_count() as u32)
        .unwrap_or(0)
}

fn count_arrow_parameters(node: &Node) -> u32 {
    // Arrow functions may have a single parameter (no parens) or formal_parameters
    if let Some(params) = node.child_by_field_name("parameters") {
        params.named_child_count() as u32
    } else if let Some(_param) = node.child_by_field_name("parameter") {
        1
    } else {
        0
    }
}

fn count_method_parameters(node: &Node) -> u32 {
    node.child_by_field_name("parameters")
        .map(|params| params.named_child_count() as u32)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzers::typescript::parser::parse_source;
    use crate::core::ast::JsLanguageVariant;
    use std::path::PathBuf;

    #[test]
    fn test_extract_function_declaration() {
        let source = "function hello() { return 'world'; }";
        let path = PathBuf::from("test.js");
        let ast = parse_source(source, &path, JsLanguageVariant::JavaScript).unwrap();

        let functions = extract_functions(&ast, false);

        assert_eq!(functions.len(), 1);
        assert_eq!(functions[0].name, "hello");
        assert_eq!(functions[0].kind, FunctionKind::Declaration);
    }

    #[test]
    fn test_extract_arrow_function() {
        let source = "const greet = (name) => `Hello ${name}`;";
        let path = PathBuf::from("test.js");
        let ast = parse_source(source, &path, JsLanguageVariant::JavaScript).unwrap();

        let functions = extract_functions(&ast, false);

        assert_eq!(functions.len(), 1);
        assert_eq!(functions[0].name, "greet");
        assert_eq!(functions[0].kind, FunctionKind::Arrow);
    }

    #[test]
    fn test_extract_async_function() {
        let source = "async function fetchData() { await fetch('/api'); }";
        let path = PathBuf::from("test.js");
        let ast = parse_source(source, &path, JsLanguageVariant::JavaScript).unwrap();

        let functions = extract_functions(&ast, false);

        assert_eq!(functions.len(), 1);
        assert_eq!(functions[0].name, "fetchData");
        assert!(functions[0].is_async);
        assert_eq!(functions[0].kind, FunctionKind::Async);
    }

    #[test]
    fn test_extract_class_methods() {
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

        let functions = extract_functions(&ast, false);

        // Should find constructor and greet method
        assert!(functions.len() >= 2);
    }

    #[test]
    fn test_extract_exported_function() {
        let source = "export function publicFunc() { return 42; }";
        let path = PathBuf::from("test.js");
        let ast = parse_source(source, &path, JsLanguageVariant::JavaScript).unwrap();

        let functions = extract_functions(&ast, false);

        assert_eq!(functions.len(), 1);
        assert!(functions[0].is_exported);
    }

    #[test]
    fn test_extract_typescript_function() {
        let source = "function greet(name: string): string { return `Hello ${name}`; }";
        let path = PathBuf::from("test.ts");
        let ast = parse_source(source, &path, JsLanguageVariant::TypeScript).unwrap();

        let functions = extract_functions(&ast, false);

        assert_eq!(functions.len(), 1);
        assert_eq!(functions[0].name, "greet");
    }
}
