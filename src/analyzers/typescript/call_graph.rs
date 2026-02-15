//! Call graph extraction for TypeScript/JavaScript
//!
//! Extracts function call relationships from JS/TS files to build a call graph.

use crate::analyzers::typescript::parser::node_text;
use crate::core::ast::TypeScriptAst;
use crate::priority::call_graph::{CallGraph, CallType, FunctionCall, FunctionId};
use std::path::PathBuf;
use tree_sitter::Node;

/// Function call information extracted from AST
#[derive(Debug, Clone)]
pub struct ExtractedCall {
    /// Name of the function being called
    pub callee_name: String,
    /// Line number where the call occurs
    pub line: usize,
}

/// Function definition with its calls
#[derive(Debug, Clone)]
pub struct FunctionWithCalls {
    /// Function name
    pub name: String,
    /// File path
    pub file: PathBuf,
    /// Line number
    pub line: usize,
    /// Functions called by this function
    pub calls: Vec<ExtractedCall>,
    /// Whether this function is exported
    pub is_exported: bool,
    /// Whether this is a test function
    pub is_test: bool,
}

/// Extract function call graph from a TypeScript/JavaScript AST
pub fn extract_call_graph(ast: &TypeScriptAst) -> CallGraph {
    let mut call_graph = CallGraph::new();
    let functions = extract_functions_with_calls(ast);

    // First pass: add all functions to the call graph
    for func in &functions {
        let func_id = FunctionId::new(func.file.clone(), func.name.clone(), func.line);
        call_graph.add_function(func_id, func.is_exported, func.is_test, 1, 10);
    }

    // Second pass: add call relationships
    for func in &functions {
        let caller_id = FunctionId::new(func.file.clone(), func.name.clone(), func.line);

        // Extract class name if caller is a class method (e.g., "Calculator::add" -> "Calculator")
        let caller_class = func
            .name
            .split("::")
            .next()
            .filter(|_| func.name.contains("::"));

        for call in &func.calls {
            // Try multiple matching strategies
            let callee_func = find_callee_function(&functions, &call.callee_name, caller_class);

            if let Some(callee_func) = callee_func {
                let callee_id = FunctionId::new(
                    callee_func.file.clone(),
                    callee_func.name.clone(),
                    callee_func.line,
                );

                call_graph.add_call(FunctionCall {
                    caller: caller_id.clone(),
                    callee: callee_id,
                    call_type: CallType::Direct,
                });
            }
        }
    }

    call_graph
}

/// Find a callee function using multiple matching strategies
fn find_callee_function<'a>(
    functions: &'a [FunctionWithCalls],
    callee_name: &str,
    caller_class: Option<&str>,
) -> Option<&'a FunctionWithCalls> {
    // Strategy 1: Exact match
    if let Some(func) = functions.iter().find(|f| f.name == callee_name) {
        return Some(func);
    }

    // Strategy 2: If caller is in a class, try matching as a method of that class
    // e.g., if caller is "Calculator::add" and callee is "validate", try "Calculator::validate"
    if let Some(class_name) = caller_class {
        let qualified_name = format!("{}::{}", class_name, callee_name);
        if let Some(func) = functions.iter().find(|f| f.name == qualified_name) {
            return Some(func);
        }
    }

    // Strategy 3: Match by simple name (for cross-class or module calls)
    // If callee_name is a simple name like "helper", find any function ending with that name
    functions.iter().find(|f| {
        // Check if function name ends with ::callee_name or is exactly callee_name
        f.name == callee_name || f.name.ends_with(&format!("::{}", callee_name))
    })
}

/// Extract all functions with their call information
fn extract_functions_with_calls(ast: &TypeScriptAst) -> Vec<FunctionWithCalls> {
    let mut functions = Vec::new();
    let root = ast.tree.root_node();

    extract_functions_recursive(&root, ast, &mut functions, false);

    functions
}

/// Recursively extract functions and their calls
fn extract_functions_recursive(
    node: &Node,
    ast: &TypeScriptAst,
    functions: &mut Vec<FunctionWithCalls>,
    is_exported: bool,
) {
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        match child.kind() {
            "function_declaration" | "generator_function_declaration" => {
                if let Some(func) = extract_function_with_calls(&child, ast, is_exported) {
                    functions.push(func);
                }
            }
            "lexical_declaration" | "variable_declaration" => {
                extract_variable_functions_with_calls(&child, ast, functions, is_exported);
            }
            "export_statement" => {
                extract_functions_recursive(&child, ast, functions, true);
            }
            "class_declaration" | "class" => {
                extract_class_methods_with_calls(&child, ast, functions, is_exported);
            }
            "method_definition" => {
                if let Some(func) = extract_method_with_calls(&child, ast, None, is_exported) {
                    functions.push(func);
                }
            }
            _ => {
                extract_functions_recursive(&child, ast, functions, is_exported);
            }
        }
    }
}

/// Extract a function declaration with its calls
fn extract_function_with_calls(
    node: &Node,
    ast: &TypeScriptAst,
    is_exported: bool,
) -> Option<FunctionWithCalls> {
    let name = node
        .child_by_field_name("name")
        .map(|n| node_text(&n, &ast.source).to_string())?;

    let line = node.start_position().row + 1;
    let body = node.child_by_field_name("body")?;

    let calls = extract_calls_from_body(&body, ast);
    let is_test = is_test_function(&name);

    Some(FunctionWithCalls {
        name,
        file: ast.path.clone(),
        line,
        calls,
        is_exported,
        is_test,
    })
}

/// Extract variable-declared functions with their calls
fn extract_variable_functions_with_calls(
    node: &Node,
    ast: &TypeScriptAst,
    functions: &mut Vec<FunctionWithCalls>,
    is_exported: bool,
) {
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if child.kind() == "variable_declarator" {
            if let Some(name_node) = child.child_by_field_name("name") {
                let name = node_text(&name_node, &ast.source).to_string();

                if let Some(value_node) = child.child_by_field_name("value") {
                    match value_node.kind() {
                        "arrow_function" => {
                            let line = value_node.start_position().row + 1;
                            let body_node = value_node.child_by_field_name("body");

                            let calls = body_node
                                .map(|b| extract_calls_from_body(&b, ast))
                                .unwrap_or_default();

                            functions.push(FunctionWithCalls {
                                name: name.clone(),
                                file: ast.path.clone(),
                                line,
                                calls,
                                is_exported,
                                is_test: is_test_function(&name),
                            });
                        }
                        "function_expression" | "function" => {
                            let line = value_node.start_position().row + 1;
                            let body = value_node.child_by_field_name("body");

                            let calls = body
                                .map(|b| extract_calls_from_body(&b, ast))
                                .unwrap_or_default();

                            functions.push(FunctionWithCalls {
                                name: name.clone(),
                                file: ast.path.clone(),
                                line,
                                calls,
                                is_exported,
                                is_test: is_test_function(&name),
                            });
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}

/// Extract class methods with their calls
fn extract_class_methods_with_calls(
    node: &Node,
    ast: &TypeScriptAst,
    functions: &mut Vec<FunctionWithCalls>,
    is_exported: bool,
) {
    let class_name = node
        .child_by_field_name("name")
        .map(|n| node_text(&n, &ast.source).to_string());

    // Find class body
    let body = node
        .children(&mut node.walk())
        .find(|c| c.kind() == "class_body");

    if let Some(body) = body {
        let mut cursor = body.walk();

        for child in body.children(&mut cursor) {
            if child.kind() == "method_definition" {
                if let Some(func) =
                    extract_method_with_calls(&child, ast, class_name.as_deref(), is_exported)
                {
                    functions.push(func);
                }
            }
        }
    }
}

/// Extract a method definition with its calls
fn extract_method_with_calls(
    node: &Node,
    ast: &TypeScriptAst,
    class_name: Option<&str>,
    is_exported: bool,
) -> Option<FunctionWithCalls> {
    let method_name = node
        .child_by_field_name("name")
        .map(|n| node_text(&n, &ast.source).to_string())?;

    let full_name = if let Some(cn) = class_name {
        format!("{}::{}", cn, method_name)
    } else {
        method_name.clone()
    };

    let line = node.start_position().row + 1;
    let body = node.child_by_field_name("body");

    let calls = body
        .map(|b| extract_calls_from_body(&b, ast))
        .unwrap_or_default();

    Some(FunctionWithCalls {
        name: full_name,
        file: ast.path.clone(),
        line,
        calls,
        is_exported,
        is_test: is_test_function(&method_name),
    })
}

/// Extract all function calls from a function body
fn extract_calls_from_body(body: &Node, ast: &TypeScriptAst) -> Vec<ExtractedCall> {
    let mut calls = Vec::new();
    extract_calls_recursive(body, ast, &mut calls);
    calls
}

/// Recursively extract function calls from AST nodes
fn extract_calls_recursive(node: &Node, ast: &TypeScriptAst, calls: &mut Vec<ExtractedCall>) {
    if node.kind() == "call_expression" {
        if let Some(callee_name) = extract_callee_name(node, ast) {
            calls.push(ExtractedCall {
                callee_name,
                line: node.start_position().row + 1,
            });
        }
    }

    // Recurse into children
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        extract_calls_recursive(&child, ast, calls);
    }
}

/// Extract the function name from a call expression
fn extract_callee_name(call_expr: &Node, ast: &TypeScriptAst) -> Option<String> {
    let function_node = call_expr.child_by_field_name("function")?;

    match function_node.kind() {
        // Simple function call: foo()
        "identifier" => Some(node_text(&function_node, &ast.source).to_string()),

        // Method call: obj.method() - extract just the method name
        "member_expression" => {
            let property = function_node.child_by_field_name("property")?;
            Some(node_text(&property, &ast.source).to_string())
        }

        // Optional chain: obj?.method()
        "optional_chain_expression" => {
            // Try to find the member access via property field first
            if let Some(property) = function_node.child_by_field_name("property") {
                return Some(node_text(&property, &ast.source).to_string());
            }
            // Fall back to finding identifier in children
            let mut cursor = function_node.walk();
            for child in function_node.children(&mut cursor) {
                if child.kind() == "identifier" {
                    return Some(node_text(&child, &ast.source).to_string());
                }
            }
            None
        }

        _ => None,
    }
}

/// Check if a function name indicates it's a test
fn is_test_function(name: &str) -> bool {
    let lower = name.to_lowercase();
    lower.starts_with("test")
        || lower.starts_with("it_")
        || lower.starts_with("should_")
        || lower == "it"
        || lower == "test"
        || lower == "describe"
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzers::typescript::parser::parse_source;
    use crate::core::ast::JsLanguageVariant;

    #[test]
    fn test_extract_simple_call() {
        let source = r#"
function helper() {
    return 42;
}

function main() {
    return helper();
}
"#;
        let path = PathBuf::from("test.js");
        let ast = parse_source(source, &path, JsLanguageVariant::JavaScript).unwrap();

        let call_graph = extract_call_graph(&ast);

        // Check that main calls helper
        let main_id = call_graph
            .get_all_functions()
            .into_iter()
            .find(|f| f.name == "main")
            .expect("main should exist");

        let callees = call_graph.get_callees(&main_id);
        assert!(
            callees.iter().any(|c| c.name == "helper"),
            "main should call helper. Found: {:?}",
            callees
        );

        // Check that helper has main as upstream caller
        let helper_id = call_graph
            .get_all_functions()
            .into_iter()
            .find(|f| f.name == "helper")
            .expect("helper should exist");

        let callers = call_graph.get_callers(&helper_id);
        assert!(
            callers.iter().any(|c| c.name == "main"),
            "helper should have main as caller. Found: {:?}",
            callers
        );
    }

    #[test]
    fn test_extract_arrow_function_calls() {
        let source = r#"
const greet = (name) => {
    return formatName(name);
};

const formatName = (name) => name.toUpperCase();
"#;
        let path = PathBuf::from("test.js");
        let ast = parse_source(source, &path, JsLanguageVariant::JavaScript).unwrap();

        let call_graph = extract_call_graph(&ast);

        let greet_id = call_graph
            .get_all_functions()
            .into_iter()
            .find(|f| f.name == "greet")
            .expect("greet should exist");

        let callees = call_graph.get_callees(&greet_id);
        assert!(
            callees.iter().any(|c| c.name == "formatName"),
            "greet should call formatName. Found: {:?}",
            callees
        );
    }

    #[test]
    fn test_extract_class_method_calls() {
        let source = r#"
class Calculator {
    add(a, b) {
        return this.validate(a, b) ? a + b : 0;
    }

    validate(a, b) {
        return typeof a === 'number' && typeof b === 'number';
    }
}
"#;
        let path = PathBuf::from("test.js");
        let ast = parse_source(source, &path, JsLanguageVariant::JavaScript).unwrap();

        let call_graph = extract_call_graph(&ast);

        // Check that add calls validate
        let add_id = call_graph
            .get_all_functions()
            .into_iter()
            .find(|f| f.name == "Calculator::add")
            .expect("Calculator::add should exist");

        let callees = call_graph.get_callees(&add_id);
        // Method calls like this.validate() extract just "validate"
        assert!(
            callees.iter().any(|c| c.name.contains("validate")),
            "add should call validate. Found: {:?}",
            callees
        );
    }

    #[test]
    fn test_multiple_calls_in_function() {
        let source = r#"
function processData(data) {
    const validated = validate(data);
    const transformed = transform(validated);
    const result = format(transformed);
    return result;
}

function validate(d) { return d; }
function transform(d) { return d; }
function format(d) { return d; }
"#;
        let path = PathBuf::from("test.js");
        let ast = parse_source(source, &path, JsLanguageVariant::JavaScript).unwrap();

        let call_graph = extract_call_graph(&ast);

        let process_id = call_graph
            .get_all_functions()
            .into_iter()
            .find(|f| f.name == "processData")
            .expect("processData should exist");

        let callees = call_graph.get_callees(&process_id);

        assert!(callees.iter().any(|c| c.name == "validate"));
        assert!(callees.iter().any(|c| c.name == "transform"));
        assert!(callees.iter().any(|c| c.name == "format"));
    }
}
