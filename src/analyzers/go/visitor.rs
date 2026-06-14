use crate::analyzers::go::advanced::detect_advanced_signals;
use crate::analyzers::go::parser::{node_line, node_text};
use crate::analyzers::go::purity::analyze_purity;
use crate::analyzers::go::types::{GoAnalysis, GoFunction, GoFunctionKind};
use crate::core::PurityLevel;
use crate::core::ast::GoAst;
use std::collections::HashMap;
use std::path::Path;
use tree_sitter::Node;

pub fn analyze_ast(ast: &GoAst) -> GoAnalysis {
    let root = ast.tree.root_node();
    let return_types = function_return_types(root, ast);
    let package_variables = package_variables(root, ast);
    let mut analysis = GoAnalysis {
        package_name: package_name_from_ast(ast),
        functions: Vec::new(),
    };

    collect_functions(
        root,
        ast,
        &return_types,
        &package_variables,
        &mut analysis.functions,
    );
    analysis
        .functions
        .sort_by(|a, b| a.line.cmp(&b.line).then_with(|| a.name.cmp(&b.name)));
    analysis
}

pub fn package_name_from_ast(ast: &GoAst) -> Option<String> {
    package_name(ast.tree.root_node(), ast)
}

fn collect_functions(
    node: Node,
    ast: &GoAst,
    return_types: &HashMap<String, String>,
    package_variables: &[String],
    functions: &mut Vec<GoFunction>,
) {
    if let Some(function) = function_from_node(node, ast, return_types, package_variables) {
        functions.push(function);
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_functions(child, ast, return_types, package_variables, functions);
    }
}

fn function_from_node(
    node: Node,
    ast: &GoAst,
    return_types: &HashMap<String, String>,
    package_variables: &[String],
) -> Option<GoFunction> {
    match node.kind() {
        "function_declaration" => named_function(node, ast, return_types, package_variables),
        "method_declaration" => method_function(node, ast, return_types, package_variables),
        _ => None,
    }
}

fn named_function(
    node: Node,
    ast: &GoAst,
    return_types: &HashMap<String, String>,
    package_variables: &[String],
) -> Option<GoFunction> {
    let name = child_text(node, "name", ast)?;
    Some(build_function(
        node,
        ast,
        name,
        GoFunctionKind::Function,
        FunctionContext::new(return_types, None),
        package_variables,
    ))
}

fn method_function(
    node: Node,
    ast: &GoAst,
    return_types: &HashMap<String, String>,
    package_variables: &[String],
) -> Option<GoFunction> {
    let name = child_text(node, "name", ast)?;
    let receiver = receiver_info(node, ast);
    let receiver_type = receiver
        .as_ref()
        .map(|receiver| receiver.type_name.clone())
        .unwrap_or_else(|| "?".to_string());
    Some(build_function(
        node,
        ast,
        format!("{receiver_type}.{name}"),
        GoFunctionKind::Method,
        FunctionContext::new(return_types, receiver),
        package_variables,
    ))
}

fn build_function(
    node: Node,
    ast: &GoAst,
    name: String,
    kind: GoFunctionKind,
    context: FunctionContext<'_>,
    package_variables: &[String],
) -> GoFunction {
    let body = node.child_by_field_name("body");
    let cyclomatic = body.map(cyclomatic_complexity).unwrap_or(1);
    let cognitive = body.map(|node| cognitive_complexity(node, 0)).unwrap_or(0);
    let nesting = body.map(|node| max_nesting(node, 0)).unwrap_or(0);
    let purity = body.map(|node| analyze_purity(node, &ast.source));
    let is_test = is_test_file(&ast.path) || is_test_function(&name);
    let advisory = body
        .map(|node| detect_advanced_signals(node, &ast.source, &name, is_test, package_variables))
        .unwrap_or_default();
    let local_types = body
        .map(|node| local_type_environment(node, ast, &context))
        .unwrap_or_default();

    GoFunction {
        visibility: visibility_for(&name),
        is_test,
        name,
        file: ast.path.clone(),
        line: node_line(&node),
        length: function_length(node),
        cyclomatic,
        cognitive,
        nesting,
        kind,
        calls: body
            .map(|node| collect_calls(node, ast, &local_types))
            .unwrap_or_default(),
        purity_level: purity
            .as_ref()
            .map(|purity| purity.level)
            .unwrap_or(PurityLevel::Impure),
        purity_confidence: purity
            .as_ref()
            .map(|purity| purity.confidence)
            .unwrap_or(0.0),
        purity_patterns: purity.map(|purity| purity.patterns).unwrap_or_default(),
        advisory_patterns: advisory.patterns,
        error_swallowing_count: advisory.error_swallowing_count,
        error_swallowing_patterns: advisory.error_swallowing_patterns,
    }
}

fn package_name(root: Node, ast: &GoAst) -> Option<String> {
    root.child_by_field_name("package")
        .map(|node| node_text(&node, &ast.source).to_string())
        .or_else(|| first_child_text(root, "package_identifier", ast))
}

#[derive(Debug, Clone)]
struct ReceiverInfo {
    name: Option<String>,
    type_name: String,
}

struct FunctionContext<'a> {
    return_types: &'a HashMap<String, String>,
    receiver: Option<ReceiverInfo>,
}

impl<'a> FunctionContext<'a> {
    fn new(return_types: &'a HashMap<String, String>, receiver: Option<ReceiverInfo>) -> Self {
        Self {
            return_types,
            receiver,
        }
    }
}

fn receiver_info(node: Node, ast: &GoAst) -> Option<ReceiverInfo> {
    let receiver = node.child_by_field_name("receiver")?;
    let text = node_text(&receiver, &ast.source);
    let parts = receiver_parts(text);
    receiver_type_from_parts(&parts).map(|type_name| ReceiverInfo {
        name: receiver_name_from_parts(&parts),
        type_name,
    })
}

fn receiver_parts(text: &str) -> Vec<&str> {
    text.trim_matches(|ch| ch == '(' || ch == ')')
        .split_whitespace()
        .collect()
}

fn receiver_name_from_parts(parts: &[&str]) -> Option<String> {
    (parts.len() > 1).then(|| parts[0].to_string())
}

fn receiver_type_from_parts(parts: &[&str]) -> Option<String> {
    parts
        .last()
        .map(|part| normalize_go_type(part))
        .filter(|part| !part.is_empty())
}

fn normalize_go_type(text: &str) -> String {
    text.trim()
        .trim_start_matches('*')
        .trim_start_matches('&')
        .trim_end_matches("{}")
        .split('[')
        .next()
        .unwrap_or("")
        .to_string()
}

fn function_return_types(root: Node, ast: &GoAst) -> HashMap<String, String> {
    children(root)
        .into_iter()
        .filter_map(|child| function_return_type(child, ast))
        .collect()
}

fn package_variables(root: Node, ast: &GoAst) -> Vec<String> {
    children(root)
        .into_iter()
        .filter(|child| child.kind() == "var_declaration")
        .flat_map(|child| variable_names(child, ast))
        .collect()
}

fn variable_names(node: Node, ast: &GoAst) -> Vec<String> {
    if node.kind() == "var_spec" {
        return first_child_text(node, "identifier", ast)
            .into_iter()
            .collect();
    }

    children(node)
        .into_iter()
        .flat_map(|child| variable_names(child, ast))
        .collect()
}

fn function_return_type(node: Node, ast: &GoAst) -> Option<(String, String)> {
    if node.kind() != "function_declaration" {
        return None;
    }

    let name = child_text(node, "name", ast)?;
    declaration_return_type(node, ast).map(|type_name| (name, type_name))
}

fn declaration_return_type(node: Node, ast: &GoAst) -> Option<String> {
    node.child_by_field_name("result")
        .map(|result| normalize_go_type(node_text(&result, &ast.source)))
        .or_else(|| signature_return_type(node_text(&node, &ast.source)))
        .filter(|type_name| !type_name.is_empty())
}

fn signature_return_type(text: &str) -> Option<String> {
    let signature = text.split('{').next()?.trim();
    let result = signature.rsplit_once(')')?.1.trim();
    (!result.is_empty()).then(|| normalize_go_type(result))
}

fn child_text(node: Node, field: &str, ast: &GoAst) -> Option<String> {
    node.child_by_field_name(field)
        .map(|child| node_text(&child, &ast.source).to_string())
}

fn first_child_text(node: Node, kind: &str, ast: &GoAst) -> Option<String> {
    if node.kind() == kind {
        return Some(node_text(&node, &ast.source).to_string());
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if let Some(text) = first_child_text(child, kind, ast) {
            return Some(text);
        }
    }

    None
}

fn function_length(node: Node) -> usize {
    node.end_position()
        .row
        .saturating_sub(node.start_position().row)
        + 1
}

fn visibility_for(name: &str) -> Option<String> {
    name.rsplit('.')
        .next()
        .filter(|short_name| starts_exported(short_name))
        .map(|_| "public".to_string())
}

fn starts_exported(name: &str) -> bool {
    name.chars()
        .next()
        .map(|ch| ch.is_uppercase())
        .unwrap_or(false)
}

fn is_test_file(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.ends_with("_test.go"))
        .unwrap_or(false)
}

fn is_test_function(name: &str) -> bool {
    let short_name = name.rsplit('.').next().unwrap_or(name);
    ["Test", "Benchmark", "Fuzz", "Example"]
        .iter()
        .any(|prefix| short_name.starts_with(prefix))
}

fn cyclomatic_complexity(node: Node) -> u32 {
    1 + branch_complexity(node)
}

fn branch_complexity(node: Node) -> u32 {
    if is_nested_callable(node) {
        return 0;
    }

    let current = u32::from(is_branch_node(node)) + boolean_operator_count(node);
    current + child_sum(node, branch_complexity)
}

fn cognitive_complexity(node: Node, depth: u32) -> u32 {
    if is_nested_callable(node) {
        return 0;
    }

    let branch_cost = if is_branch_node(node) { 1 + depth } else { 0 };
    let next_depth = if is_nesting_node(node) {
        depth + 1
    } else {
        depth
    };

    branch_cost + boolean_operator_count(node) + child_sum_with_depth(node, next_depth)
}

fn max_nesting(node: Node, depth: u32) -> u32 {
    if is_nested_callable(node) {
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

fn is_branch_node(node: Node) -> bool {
    matches!(
        node.kind(),
        "if_statement"
            | "for_statement"
            | "expression_switch_statement"
            | "type_switch_statement"
            | "select_statement"
            | "case_clause"
            | "communication_case"
    )
}

fn is_nesting_node(node: Node) -> bool {
    matches!(
        node.kind(),
        "if_statement"
            | "for_statement"
            | "expression_switch_statement"
            | "type_switch_statement"
            | "select_statement"
    )
}

fn is_nested_callable(node: Node) -> bool {
    matches!(
        node.kind(),
        "func_literal" | "function_declaration" | "method_declaration"
    )
}

fn boolean_operator_count(node: Node) -> u32 {
    if node.kind() != "binary_expression" {
        return 0;
    }

    children(node)
        .into_iter()
        .filter(|child| matches!(child.kind(), "&&" | "||"))
        .count() as u32
}

fn child_sum(node: Node, f: fn(Node) -> u32) -> u32 {
    children(node).into_iter().map(f).sum()
}

fn child_sum_with_depth(node: Node, depth: u32) -> u32 {
    children(node)
        .into_iter()
        .map(|child| cognitive_complexity(child, depth))
        .sum()
}

fn children(node: Node) -> Vec<Node> {
    let mut cursor = node.walk();
    node.children(&mut cursor).collect()
}

fn collect_calls(node: Node, ast: &GoAst, local_types: &HashMap<String, String>) -> Vec<String> {
    if is_nested_callable(node) {
        return Vec::new();
    }

    let current = if node.kind() == "call_expression" {
        call_name(node, ast, local_types).into_iter().collect()
    } else {
        Vec::new()
    };

    current
        .into_iter()
        .chain(
            children(node)
                .into_iter()
                .flat_map(|child| collect_calls(child, ast, local_types)),
        )
        .collect()
}

fn call_name(node: Node, ast: &GoAst, local_types: &HashMap<String, String>) -> Option<String> {
    node.child_by_field_name("function")
        .map(|function| normalize_call_name(function, ast, local_types))
}

fn normalize_call_name(
    function: Node,
    ast: &GoAst,
    local_types: &HashMap<String, String>,
) -> String {
    let text = node_text(&function, &ast.source);
    selector_call_name(text, local_types).unwrap_or_else(|| text.to_string())
}

fn selector_call_name(text: &str, local_types: &HashMap<String, String>) -> Option<String> {
    let (receiver, method) = text.rsplit_once('.')?;
    local_types
        .get(receiver)
        .map(|type_name| format!("{type_name}.{method}"))
        .or_else(|| Some(format!("{receiver}.{method}")))
}

fn local_type_environment(
    body: Node,
    ast: &GoAst,
    context: &FunctionContext<'_>,
) -> HashMap<String, String> {
    let initial = context
        .receiver
        .as_ref()
        .and_then(receiver_binding)
        .into_iter()
        .collect();

    collect_local_types(body, ast, context.return_types, initial)
}

fn receiver_binding(receiver: &ReceiverInfo) -> Option<(String, String)> {
    receiver
        .name
        .as_ref()
        .map(|name| (name.clone(), receiver.type_name.clone()))
}

fn collect_local_types(
    node: Node,
    ast: &GoAst,
    return_types: &HashMap<String, String>,
    mut types: HashMap<String, String>,
) -> HashMap<String, String> {
    if let Some((name, type_name)) = local_type_binding(node, ast, return_types) {
        types.insert(name, type_name);
    }

    children(node).into_iter().fold(types, |types, child| {
        collect_local_types(child, ast, return_types, types)
    })
}

fn local_type_binding(
    node: Node,
    ast: &GoAst,
    return_types: &HashMap<String, String>,
) -> Option<(String, String)> {
    match node.kind() {
        "short_var_declaration" => short_var_binding(node, ast, return_types),
        "var_spec" => var_spec_binding(node, ast),
        _ => None,
    }
}

fn short_var_binding(
    node: Node,
    ast: &GoAst,
    return_types: &HashMap<String, String>,
) -> Option<(String, String)> {
    let text = node_text(&node, &ast.source);
    let (left, right) = text.split_once(":=")?;
    let name = single_identifier(left)?;
    inferred_type(right.trim(), return_types).map(|type_name| (name.to_string(), type_name))
}

fn var_spec_binding(node: Node, ast: &GoAst) -> Option<(String, String)> {
    let name = first_child_text(node, "identifier", ast)?;
    explicit_var_type(node, ast).map(|type_name| (name, type_name))
}

fn explicit_var_type(node: Node, ast: &GoAst) -> Option<String> {
    first_child_text(node, "type_identifier", ast)
        .or_else(|| first_child_text(node, "pointer_type", ast))
        .map(|text| normalize_go_type(&text))
}

fn single_identifier(text: &str) -> Option<&str> {
    let name = text.trim();
    name.chars()
        .all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
        .then_some(name)
        .filter(|name| !name.is_empty() && !name.contains(','))
}

fn inferred_type(text: &str, return_types: &HashMap<String, String>) -> Option<String> {
    composite_literal_type(text)
        .or_else(|| constructor_return_type(text, return_types))
        .map(|type_name| normalize_go_type(&type_name))
}

fn composite_literal_type(text: &str) -> Option<String> {
    let text = text.trim_start_matches('&');
    text.contains('{')
        .then(|| text.split('{').next())
        .flatten()
        .map(str::trim)
        .filter(|name| starts_exported(name))
        .map(str::to_string)
}

fn constructor_return_type(text: &str, return_types: &HashMap<String, String>) -> Option<String> {
    let name = normalize_go_type(text.split('(').next()?.trim());
    return_types.get(&name).cloned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzers::go::parser::parse_source;
    use std::path::PathBuf;

    #[test]
    fn test_extract_package_functions_and_methods() {
        let source = r#"package service

func exported() {}
func Exported() {}
func (h *Handler) Serve() {}
"#;
        let ast = parse_source(source, &PathBuf::from("handler.go")).unwrap();
        let analysis = analyze_ast(&ast);

        assert_eq!(analysis.package_name, Some("service".to_string()));
        assert_eq!(analysis.functions.len(), 3);
        assert_eq!(analysis.functions[2].name, "Handler.Serve");
        assert_eq!(analysis.functions[1].visibility, Some("public".to_string()));
        assert_eq!(analysis.functions[2].kind, GoFunctionKind::Method);
    }

    #[test]
    fn test_detect_go_tests() {
        let source = "package service\n\nfunc TestServe(t *testing.T) {}\n";
        let ast = parse_source(source, &PathBuf::from("handler_test.go")).unwrap();
        let analysis = analyze_ast(&ast);

        assert!(analysis.functions[0].is_test);
    }

    #[test]
    fn test_calculates_go_complexity() {
        let source = r#"package service

func decide(a, b, c bool, xs []int) int {
    if a && b {
        for range xs {
            if c {
                return 1
            }
        }
    }
    switch {
    case a:
        return 2
    default:
        return 3
    }
}
"#;
        let ast = parse_source(source, &PathBuf::from("service.go")).unwrap();
        let analysis = analyze_ast(&ast);
        let function = &analysis.functions[0];

        assert!(function.cyclomatic >= 6);
        assert!(function.cognitive >= 6);
        assert!(function.nesting >= 3);
    }

    #[test]
    fn test_extracts_same_file_calls() {
        let source = r#"package service

func Serve() {
    helper()
    fmt.Println("ignored")
}

func helper() {}
"#;
        let ast = parse_source(source, &PathBuf::from("service.go")).unwrap();
        let analysis = analyze_ast(&ast);
        let serve = analysis
            .functions
            .iter()
            .find(|function| function.name == "Serve")
            .unwrap();

        assert_eq!(
            serve.calls,
            vec!["helper".to_string(), "fmt.Println".to_string()]
        );
    }

    #[test]
    fn test_extracts_generic_functions_with_stable_names() {
        let source = r#"package collections

func Map[T any, U any](items []T, f func(T) U) []U {
    if len(items) == 0 {
        return nil
    }
    return []U{}
}
"#;
        let ast = parse_source(source, &PathBuf::from("collections.go")).unwrap();
        let analysis = analyze_ast(&ast);
        let map = analysis
            .functions
            .iter()
            .find(|function| function.name == "Map")
            .unwrap();

        assert_eq!(map.visibility, Some("public".to_string()));
        assert!(map.cyclomatic > 1);
    }

    #[test]
    fn test_extracts_generic_receiver_methods_with_stable_names() {
        let source = r#"package collections

type Set[T comparable] map[T]struct{}

func (s Set[T]) Has(item T) bool {
    _, ok := s[item]
    return ok
}
"#;
        let ast = parse_source(source, &PathBuf::from("set.go")).unwrap();
        let analysis = analyze_ast(&ast);

        assert_eq!(analysis.functions[0].name, "Set.Has");
    }

    #[test]
    fn test_extracts_generic_instantiated_calls_without_type_arguments() {
        let source = r#"package collections

func Run(xs []int) []string {
    return Map[int, string](xs, format)
}

func Map[T any, U any](items []T, f func(T) U) []U {
    return nil
}

func format(item int) string {
    return ""
}
"#;
        let ast = parse_source(source, &PathBuf::from("collections.go")).unwrap();
        let analysis = analyze_ast(&ast);
        let run = analysis
            .functions
            .iter()
            .find(|function| function.name == "Run")
            .unwrap();

        assert_eq!(run.calls, vec!["Map".to_string()]);
    }

    #[test]
    fn test_extracts_generic_constructor_receiver_call() {
        let source = r#"package collections

type Set[T comparable] map[T]struct{}

func Run() bool {
    set := NewSet[int]()
    return set.Has(1)
}

func NewSet[T comparable]() Set[T] {
    return Set[T]{}
}

func (s Set[T]) Has(item T) bool {
    return true
}
"#;
        let ast = parse_source(source, &PathBuf::from("set.go")).unwrap();
        let analysis = analyze_ast(&ast);
        let run = analysis
            .functions
            .iter()
            .find(|function| function.name == "Run")
            .unwrap();

        assert_eq!(run.calls, vec!["NewSet".to_string(), "Set.Has".to_string()]);
    }
}
