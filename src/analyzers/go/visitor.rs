use crate::analyzers::go::parser::{node_line, node_text};
use crate::analyzers::go::types::{GoAnalysis, GoFunction, GoFunctionKind};
use crate::core::ast::GoAst;
use std::path::Path;
use tree_sitter::Node;

pub fn analyze_ast(ast: &GoAst) -> GoAnalysis {
    let root = ast.tree.root_node();
    let mut analysis = GoAnalysis {
        package_name: package_name(root, ast),
        functions: Vec::new(),
    };

    collect_functions(root, ast, &mut analysis.functions);
    resolve_same_file_calls(&mut analysis.functions);
    analysis
        .functions
        .sort_by(|a, b| a.line.cmp(&b.line).then_with(|| a.name.cmp(&b.name)));
    analysis
}

fn collect_functions(node: Node, ast: &GoAst, functions: &mut Vec<GoFunction>) {
    if let Some(function) = function_from_node(node, ast) {
        functions.push(function);
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_functions(child, ast, functions);
    }
}

fn function_from_node(node: Node, ast: &GoAst) -> Option<GoFunction> {
    match node.kind() {
        "function_declaration" => named_function(node, ast),
        "method_declaration" => method_function(node, ast),
        _ => None,
    }
}

fn named_function(node: Node, ast: &GoAst) -> Option<GoFunction> {
    let name = child_text(node, "name", ast)?;
    Some(build_function(node, ast, name, GoFunctionKind::Function))
}

fn method_function(node: Node, ast: &GoAst) -> Option<GoFunction> {
    let name = child_text(node, "name", ast)?;
    let receiver = receiver_type(node, ast).unwrap_or_else(|| "?".to_string());
    Some(build_function(
        node,
        ast,
        format!("{receiver}.{name}"),
        GoFunctionKind::Method,
    ))
}

fn build_function(node: Node, ast: &GoAst, name: String, kind: GoFunctionKind) -> GoFunction {
    let body = node.child_by_field_name("body");
    let cyclomatic = body.map(cyclomatic_complexity).unwrap_or(1);
    let cognitive = body.map(|node| cognitive_complexity(node, 0)).unwrap_or(0);
    let nesting = body.map(|node| max_nesting(node, 0)).unwrap_or(0);

    GoFunction {
        visibility: visibility_for(&name),
        is_test: is_test_file(&ast.path) || is_test_function(&name),
        name,
        file: ast.path.clone(),
        line: node_line(&node),
        length: function_length(node),
        cyclomatic,
        cognitive,
        nesting,
        kind,
        calls: body
            .map(|node| collect_calls(node, ast))
            .unwrap_or_default(),
    }
}

fn package_name(root: Node, ast: &GoAst) -> Option<String> {
    root.child_by_field_name("package")
        .map(|node| node_text(&node, &ast.source).to_string())
        .or_else(|| first_child_text(root, "package_identifier", ast))
}

fn receiver_type(node: Node, ast: &GoAst) -> Option<String> {
    let receiver = node.child_by_field_name("receiver")?;
    first_child_text(receiver, "type_identifier", ast)
        .or_else(|| first_child_text(receiver, "pointer_type", ast))
        .map(|text| text.trim_start_matches('*').to_string())
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

fn collect_calls(node: Node, ast: &GoAst) -> Vec<String> {
    if is_nested_callable(node) {
        return Vec::new();
    }

    let current = if node.kind() == "call_expression" {
        call_name(node, ast).into_iter().collect()
    } else {
        Vec::new()
    };

    current
        .into_iter()
        .chain(
            children(node)
                .into_iter()
                .flat_map(|child| collect_calls(child, ast)),
        )
        .collect()
}

fn call_name(node: Node, ast: &GoAst) -> Option<String> {
    node.child_by_field_name("function")
        .map(|function| normalize_call_name(node_text(&function, &ast.source)))
}

fn normalize_call_name(text: &str) -> String {
    text.rsplit('.').next().unwrap_or(text).to_string()
}

fn resolve_same_file_calls(functions: &mut [GoFunction]) {
    let names: Vec<String> = functions
        .iter()
        .map(|function| short_name(&function.name))
        .collect();

    for function in functions {
        function.calls = function
            .calls
            .iter()
            .filter(|call| names.contains(call))
            .cloned()
            .collect();
    }
}

fn short_name(name: &str) -> String {
    name.rsplit('.').next().unwrap_or(name).to_string()
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

        assert_eq!(serve.calls, vec!["helper".to_string()]);
    }
}
