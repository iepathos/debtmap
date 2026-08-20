//! Call graph extraction for TypeScript/JavaScript
//!
//! Extracts function call relationships from JS/TS files to build a call graph.

use crate::analyzers::typescript::parser::node_text;
use crate::core::ast::TypeScriptAst;
use crate::priority::call_graph::{
    CallEdgeProvenance, CallGraph, CallSite, CallType, FunctionId, ResolutionOutcome,
};
use std::path::PathBuf;
use tree_sitter::Node;

/// Function call information extracted from AST
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum CallShape {
    Identifier(String),
    Member { receiver: String, property: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractedCall {
    pub shape: CallShape,
    /// Line number where the call occurs
    pub line: usize,
    /// One-based column where the call occurs.
    pub column: usize,
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
    let mut functions = extract_functions_with_calls(ast);

    // Sort functions by line and name for deterministic graph construction (Spec 214 fix)
    functions.sort_by(|a, b| a.line.cmp(&b.line).then_with(|| a.name.cmp(&b.name)));

    // First pass: add all functions to the call graph
    for func in &functions {
        let func_id = FunctionId::new(func.file.clone(), func.name.clone(), func.line);
        call_graph.add_function(func_id, func.is_exported, func.is_test, 1, 10);
    }

    // Second pass: add call relationships
    for func in &functions {
        let caller_id = FunctionId::new(func.file.clone(), func.name.clone(), func.line);

        let caller_owner = func.name.split_once("::").map(|(owner, _)| owner);

        // Sort calls by line and name for deterministic edge addition (Spec 214 fix)
        let mut sorted_calls = func.calls.clone();
        sorted_calls
            .sort_by(|a, b| (a.line, a.column, &a.shape).cmp(&(b.line, b.column, &b.shape)));

        for call in sorted_calls {
            let outcome = resolve_call(&functions, &call, caller_owner, &ast.path);
            call_graph.add_resolution(caller_id.clone(), CallType::Direct, outcome);
        }
    }

    call_graph
}

fn resolve_call(
    functions: &[FunctionWithCalls],
    call: &ExtractedCall,
    caller_owner: Option<&str>,
    file: &std::path::Path,
) -> ResolutionOutcome {
    let query = match resolution_query(&call.shape, caller_owner) {
        QueryDecision::Resolve(query) => query,
        QueryDecision::Reject(outcome) => return outcome,
    };
    let candidates = functions
        .iter()
        .filter(|function| function.name == query.name)
        .map(function_id)
        .collect();
    resolution_from_candidates(candidates, query, call, file)
}

struct ResolutionQuery {
    name: String,
    provenance: CallEdgeProvenance,
    confidence: u8,
}

enum QueryDecision {
    Resolve(ResolutionQuery),
    Reject(ResolutionOutcome),
}

fn resolution_query(shape: &CallShape, caller_owner: Option<&str>) -> QueryDecision {
    match shape {
        CallShape::Identifier(name) => QueryDecision::Resolve(ResolutionQuery {
            name: name.clone(),
            provenance: CallEdgeProvenance::AstDirect,
            confidence: 100,
        }),
        CallShape::Member { receiver, property } if receiver == "this" => {
            let Some(owner) = caller_owner else {
                return QueryDecision::Reject(ResolutionOutcome::Unresolved {
                    query: format!("this.{property}"),
                });
            };
            QueryDecision::Resolve(ResolutionQuery {
                name: format!("{owner}::{property}"),
                provenance: CallEdgeProvenance::TypeResolution,
                confidence: 95,
            })
        }
        CallShape::Member { receiver, property } => {
            QueryDecision::Reject(ResolutionOutcome::Ignored {
                reason: format!("dynamic receiver {receiver}.{property}"),
            })
        }
    }
}

fn function_id(function: &FunctionWithCalls) -> FunctionId {
    FunctionId::new(function.file.clone(), function.name.clone(), function.line)
}

fn resolution_from_candidates(
    mut candidates: Vec<FunctionId>,
    query: ResolutionQuery,
    call: &ExtractedCall,
    file: &std::path::Path,
) -> ResolutionOutcome {
    candidates.sort();
    match candidates.as_slice() {
        [target] => ResolutionOutcome::Resolved {
            target: target.clone(),
            provenance: query.provenance,
            confidence: query.confidence,
            call_site: Some(CallSite {
                file: file.to_path_buf(),
                line: call.line,
                column: Some(call.column),
            }),
        },
        [] => ResolutionOutcome::Unresolved { query: query.name },
        _ => ResolutionOutcome::Ambiguous { candidates },
    }
}

/// Extract all functions with their call information
pub(crate) fn extract_functions_with_calls(ast: &TypeScriptAst) -> Vec<FunctionWithCalls> {
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
        if child.kind() == "variable_declarator"
            && let Some(name_node) = child.child_by_field_name("name")
        {
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
            if child.kind() == "method_definition"
                && let Some(func) =
                    extract_method_with_calls(&child, ast, class_name.as_deref(), is_exported)
            {
                functions.push(func);
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
    if node.kind() == "call_expression"
        && let Some(shape) = extract_call_shape(node, ast)
    {
        calls.push(ExtractedCall {
            shape,
            line: node.start_position().row + 1,
            column: node.start_position().column + 1,
        });
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if is_nested_callable(&child) {
            continue;
        }
        extract_calls_recursive(&child, ast, calls);
    }
}

fn is_nested_callable(node: &Node) -> bool {
    matches!(
        node.kind(),
        "function_declaration"
            | "generator_function_declaration"
            | "function_expression"
            | "function"
            | "arrow_function"
            | "method_definition"
    )
}

fn extract_call_shape(call_expr: &Node, ast: &TypeScriptAst) -> Option<CallShape> {
    let function_node = call_expr.child_by_field_name("function")?;

    match function_node.kind() {
        "identifier" => Some(CallShape::Identifier(
            node_text(&function_node, &ast.source).to_string(),
        )),
        "member_expression" => extract_member_shape(&function_node, ast),
        "optional_chain_expression" => function_node
            .named_children(&mut function_node.walk())
            .find(|child| child.kind() == "member_expression")
            .and_then(|member| extract_member_shape(&member, ast)),
        _ => None,
    }
}

fn extract_member_shape(node: &Node, ast: &TypeScriptAst) -> Option<CallShape> {
    let receiver = node.child_by_field_name("object")?;
    let property = node.child_by_field_name("property")?;
    Some(CallShape::Member {
        receiver: node_text(&receiver, &ast.source).to_string(),
        property: node_text(&property, &ast.source).to_string(),
    })
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

        let callees = call_graph.get_callees(main_id);
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

        let callers = call_graph.get_callers(helper_id);
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

        let callees = call_graph.get_callees(greet_id);
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

        let callees = call_graph.get_callees(add_id);
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

        let callees = call_graph.get_callees(process_id);

        assert!(callees.iter().any(|c| c.name == "validate"));
        assert!(callees.iter().any(|c| c.name == "transform"));
        assert!(callees.iter().any(|c| c.name == "format"));
    }

    #[test]
    fn this_call_resolves_only_within_caller_class_and_keeps_evidence() {
        let source = "class A {\n  run() { return this.validate(); }\n  validate() { return true; }\n}\nclass B {\n  validate() { return false; }\n}";
        let path = PathBuf::from("classes.ts");
        let ast = parse_source(source, &path, JsLanguageVariant::TypeScript).unwrap();

        let graph = extract_call_graph(&ast);
        let run = graph
            .get_all_functions()
            .find(|function| function.name == "A::run")
            .unwrap();
        let callees = graph.get_callees(run);

        assert_eq!(callees.len(), 1);
        assert_eq!(callees[0].name, "A::validate");
        let evidence = graph.edge_evidence().next().unwrap();
        assert_eq!(evidence.provenance, CallEdgeProvenance::TypeResolution);
        assert_eq!(evidence.confidence, 95);
        assert_eq!(
            evidence.call_site,
            Some(CallSite {
                file: path,
                line: 2,
                column: Some(18),
            })
        );
    }

    #[test]
    fn arbitrary_receiver_and_bare_method_do_not_guess_local_edges() {
        let source = r#"
class Service {
    run(other) {
        other.validate();
        validate();
    }
    validate() { return true; }
}
"#;
        let path = PathBuf::from("service.js");
        let ast = parse_source(source, &path, JsLanguageVariant::JavaScript).unwrap();

        let graph = extract_call_graph(&ast);
        let run = graph
            .get_all_functions()
            .find(|function| function.name == "Service::run")
            .unwrap();

        assert!(graph.get_callees(run).is_empty());
        assert_eq!(graph.edge_evidence().count(), 0);
    }

    #[test]
    fn duplicate_top_level_candidates_are_ambiguous() {
        let source = "function run() { helper(); }\nfunction helper() {}\nfunction helper() {}";
        let path = PathBuf::from("duplicate.js");
        let ast = parse_source(source, &path, JsLanguageVariant::JavaScript).unwrap();

        let graph = extract_call_graph(&ast);
        let run = graph
            .get_all_functions()
            .find(|function| function.name == "run")
            .unwrap();

        assert!(graph.get_callees(run).is_empty());
    }

    #[test]
    fn nested_callable_calls_are_not_attributed_to_outer_function() {
        let source = r#"
function outer() {
    const nested = () => helper();
    return 1;
}
function helper() { return 2; }
"#;
        let path = PathBuf::from("nested.js");
        let ast = parse_source(source, &path, JsLanguageVariant::JavaScript).unwrap();

        let graph = extract_call_graph(&ast);
        let outer = graph
            .get_all_functions()
            .find(|function| function.name == "outer")
            .unwrap();

        assert!(graph.get_callees(outer).is_empty());
    }
}
