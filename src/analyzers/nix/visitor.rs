use crate::analyzers::nix::parser::{node_line, node_text};
use crate::analyzers::nix::types::{NixAnalysis, NixFunction};
use crate::analyzers::syntax_tree::{self, Scope, Visitor, children, function_length};
use crate::core::ast::NixAst;
use std::path::Path;
use tree_sitter::Node;

pub fn analyze_ast(ast: &NixAst) -> NixAnalysis {
    let root = ast.tree.root_node();
    let mut functions: Vec<_> = top_level_function(root, ast)
        .into_iter()
        .chain(collect_functions(root, ast, ""))
        .collect();

    functions.sort_by(|a, b| a.line.cmp(&b.line).then_with(|| a.name.cmp(&b.name)));
    NixAnalysis { functions }
}

fn top_level_function(root: Node, ast: &NixAst) -> Option<NixFunction> {
    let expression = children(root)
        .into_iter()
        .find(|child| child.is_named() && child.kind() != "comment")?;
    let lambda = unwrap_parens(expression);
    (lambda.kind() == "function_expression")
        .then(|| build_function(lambda, lambda, file_stem(&ast.path), ast))
}

fn collect_functions(node: Node, ast: &NixAst, prefix: &str) -> Vec<NixFunction> {
    if node.kind() == "binding" {
        return binding_functions(node, ast, prefix);
    }

    children(node)
        .into_iter()
        .flat_map(|child| collect_functions(child, ast, prefix))
        .collect()
}

fn binding_functions(binding: Node, ast: &NixAst, prefix: &str) -> Vec<NixFunction> {
    let (Some(name), Some(value)) = (binding_name(binding, ast, prefix), binding_value(binding))
    else {
        return Vec::new();
    };
    let lambda = unwrap_parens(value);
    let own = (lambda.kind() == "function_expression")
        .then(|| build_function(binding, lambda, name.clone(), ast));

    own.into_iter()
        .chain(collect_functions(value, ast, &name))
        .collect()
}

fn build_function(decl: Node, lambda: Node, name: String, ast: &NixAst) -> NixFunction {
    let mut complexity = Complexity::default();
    syntax_tree::walk(
        lambda,
        Some(innermost_body(lambda)),
        is_separate_function,
        &mut [&mut complexity],
    );

    NixFunction {
        visibility: Some("public".to_string()),
        is_test: is_test_file(&ast.path) || is_test_function(&name),
        name,
        file: ast.path.clone(),
        line: node_line(&decl),
        length: function_length(decl),
        cyclomatic: 1 + complexity.branches,
        cognitive: complexity.cognitive,
        nesting: complexity.nesting,
    }
}

pub(crate) fn binding_name(binding: Node, ast: &NixAst, prefix: &str) -> Option<String> {
    let path = binding.child_by_field_name("attrpath")?;
    let text = node_text(&path, &ast.source);
    Some(if prefix.is_empty() {
        text.to_string()
    } else {
        format!("{prefix}.{text}")
    })
}

pub(crate) fn binding_value(binding: Node) -> Option<Node> {
    binding.child_by_field_name("expression")
}

pub(crate) fn unwrap_parens(node: Node) -> Node {
    match node.child_by_field_name("expression") {
        Some(inner) if node.kind() == "parenthesized_expression" => unwrap_parens(inner),
        _ => node,
    }
}

fn innermost_body(lambda: Node) -> Node {
    match lambda.child_by_field_name("body") {
        Some(body) if body.kind() == "function_expression" => innermost_body(body),
        Some(body) => body,
        None => lambda,
    }
}

fn file_stem(path: &Path) -> String {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("module")
        .to_string()
}

fn is_test_file(path: &Path) -> bool {
    let in_tests_dir = path
        .parent()
        .is_some_and(|dir| dir.components().any(|part| part.as_os_str() == "tests"));
    let named_test = path
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.ends_with("-test.nix") || name.starts_with("test"));
    in_tests_dir || named_test
}

fn is_test_function(name: &str) -> bool {
    name.split('.')
        .any(|segment| matches!(segment, "tests" | "checks"))
}

fn is_separate_function(node: Node) -> bool {
    node.kind() == "binding"
        && binding_value(node)
            .is_some_and(|value| unwrap_parens(value).kind() == "function_expression")
}

#[derive(Default)]
struct Complexity {
    branches: u32,
    cognitive: u32,
    nesting: u32,
    depth: u32,
}

impl<'t> Visitor<'t> for Complexity {
    fn enter(&mut self, node: Node<'t>, scope: Scope) {
        if !scope.own_body() {
            return;
        }
        self.branches += u32::from(is_branch_node(node));
        self.cognitive += cognitive_cost(node, self.depth);
        self.depth += u32::from(is_nesting_node(node));
        self.nesting = self.nesting.max(self.depth);
    }

    fn exit(&mut self, node: Node<'t>, scope: Scope) {
        if scope.own_body() {
            self.depth -= u32::from(is_nesting_node(node));
        }
    }
}

fn cognitive_cost(node: Node, depth: u32) -> u32 {
    let branch = u32::from(is_cognitive_branch(node));
    branch * (1 + depth) + u32::from(is_logical_operator(node))
}

fn is_branch_node(node: Node) -> bool {
    match node.kind() {
        "if_expression" | "assert_expression" | "has_attr_expression" => true,
        "select_expression" => node.child_by_field_name("default").is_some(),
        _ => is_logical_operator(node),
    }
}

fn is_cognitive_branch(node: Node) -> bool {
    matches!(node.kind(), "if_expression" | "assert_expression")
}

fn is_nesting_node(node: Node) -> bool {
    match node.kind() {
        "if_expression" | "let_expression" | "with_expression" => true,
        "function_expression" => node
            .parent()
            .is_some_and(|parent| parent.kind() != "function_expression"),
        _ => false,
    }
}

fn is_logical_operator(node: Node) -> bool {
    node.kind() == "binary_expression"
        && node
            .child_by_field_name("operator")
            .is_some_and(|operator| matches!(operator.kind(), "&&" | "||" | "->"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzers::nix::parser::parse_source;
    use std::path::PathBuf;

    fn analyze(source: &str, file: &str) -> NixAnalysis {
        analyze_ast(&parse_source(source, &PathBuf::from(file)).unwrap())
    }

    fn find<'a>(analysis: &'a NixAnalysis, name: &str) -> &'a NixFunction {
        analysis
            .functions
            .iter()
            .find(|function| function.name == name)
            .unwrap_or_else(|| panic!("missing function {name}"))
    }

    #[test]
    fn test_flake_outputs_lambda() {
        let source = r#"{
  description = "example";
  inputs.nixpkgs.url = "github:NixOS/nixpkgs";
  outputs = { self, nixpkgs }: {
    packages.x86_64-linux.default = nixpkgs.legacyPackages.x86_64-linux.hello;
  };
}
"#;
        let analysis = analyze(source, "flake.nix");

        assert_eq!(analysis.functions.len(), 1);
        let outputs = find(&analysis, "outputs");
        assert_eq!(outputs.line, 4);
        assert_eq!(outputs.length, 3);
        assert_eq!(outputs.cyclomatic, 1);
        assert_eq!(outputs.visibility.as_deref(), Some("public"));
    }

    #[test]
    fn test_top_level_function_named_after_file_stem() {
        let source = r#"{ lib, stdenv, fetchurl }:
stdenv.mkDerivation {
  pname = "hello";
  src = fetchurl { url = "x"; };
}
"#;
        let analysis = analyze(source, "pkgs/hello/default.nix");

        assert_eq!(analysis.functions.len(), 1);
        assert_eq!(analysis.functions[0].name, "default");
        assert_eq!(analysis.functions[0].line, 1);
        assert_eq!(analysis.functions[0].length, 5);
    }

    #[test]
    fn test_parenthesized_top_level_function() {
        let analysis = analyze("({ pkgs }: pkgs.hello)", "flake.nix");
        assert_eq!(analysis.functions[0].name, "flake");
    }

    #[test]
    fn test_let_bound_helper() {
        let source = "let mkThing = { a, b ? true }: if a then b else !b; in mkThing";
        let analysis = analyze(source, "x.nix");

        assert_eq!(analysis.functions.len(), 1);
        assert_eq!(find(&analysis, "mkThing").cyclomatic, 2);
    }

    #[test]
    fn test_curried_lambda_counted_once() {
        let source = "{ f = x: y: if x then y else null; }";
        let analysis = analyze(source, "x.nix");

        assert_eq!(analysis.functions.len(), 1);
        let f = find(&analysis, "f");
        assert_eq!(f.cyclomatic, 2);
        assert_eq!(f.nesting, 1);
    }

    #[test]
    fn test_anonymous_lambda_folds_into_parent() {
        let source = "{ run = xs: map (x: if x then 1 else 2) xs; }";
        let analysis = analyze(source, "x.nix");

        assert_eq!(analysis.functions.len(), 1);
        let run = find(&analysis, "run");
        assert_eq!(run.cyclomatic, 2);
        assert_eq!(run.nesting, 2);
        assert_eq!(run.cognitive, 2);
    }

    #[test]
    fn test_nested_named_lambda_not_double_counted() {
        let source = r#"{
  lib = {
    outer = x: let inner = y: if y then 1 else 2; in if x then inner x else 0;
  };
}"#;
        let analysis = analyze(source, "x.nix");

        assert_eq!(find(&analysis, "lib.outer").cyclomatic, 2);
        assert_eq!(find(&analysis, "lib.outer.inner").cyclomatic, 2);
    }

    #[test]
    fn test_or_default_and_has_attr_increase_cyclomatic() {
        let source = "{ f = cfg: (cfg.a or 1) + (if cfg ? b then 1 else 0); }";
        let analysis = analyze(source, "x.nix");

        assert_eq!(find(&analysis, "f").cyclomatic, 4);
    }

    #[test]
    fn test_logical_operators() {
        let source = "{ f = a: b: c: if a && b || c then 1 else 0; g = a: b: a -> b; h = a: !a; }";
        let analysis = analyze(source, "x.nix");

        let f = find(&analysis, "f");
        assert_eq!(f.cyclomatic, 4);
        assert_eq!(f.cognitive, 3);
        assert_eq!(find(&analysis, "g").cyclomatic, 2);
        assert_eq!(find(&analysis, "h").cyclomatic, 1);
    }

    #[test]
    fn test_assert_counts() {
        let analysis = analyze("{ f = a: assert a; a; }", "x.nix");

        let f = find(&analysis, "f");
        assert_eq!(f.cyclomatic, 2);
        assert_eq!(f.cognitive, 1);
    }

    #[test]
    fn test_nesting_depth_let_with_if() {
        let source = r#"{
  f = pkgs:
    let a = 1; in
    with pkgs;
    if a then (if b then 1 else 2) else 3;
}"#;
        let analysis = analyze(source, "x.nix");
        let f = find(&analysis, "f");

        assert_eq!(f.nesting, 4);
        assert_eq!(f.cyclomatic, 3);
        assert_eq!(f.cognitive, 3 + 4);
    }

    #[test]
    fn test_nested_binding_names_are_qualified() {
        let source = r#"{
  outputs = { self }: {
    lib.mkFoo = x: x;
    packages.default = pkgs: pkgs.hello;
  };
}"#;
        let analysis = analyze(source, "flake.nix");
        let names: Vec<_> = analysis.functions.iter().map(|f| f.name.as_str()).collect();

        assert_eq!(
            names,
            vec!["outputs", "outputs.lib.mkFoo", "outputs.packages.default"]
        );
    }

    #[test]
    fn test_rec_attrset_and_dotted_attrpath() {
        let analysis = analyze("rec { a.b = x: x; c = a.b 1; }", "x.nix");

        assert_eq!(analysis.functions.len(), 1);
        assert_eq!(analysis.functions[0].name, "a.b");
    }

    #[test]
    fn test_test_detection() {
        let source = "{ checks.x86_64-linux.smoke = pkgs: pkgs.hello; build = x: x; }";
        let analysis = analyze(source, "flake.nix");

        assert!(find(&analysis, "checks.x86_64-linux.smoke").is_test);
        assert!(!find(&analysis, "build").is_test);
        assert!(analyze("{ f = x: x; }", "nixos/tests/foo.nix").functions[0].is_test);
        assert!(analyze("{ f = x: x; }", "foo-test.nix").functions[0].is_test);
        assert!(!analyze("{ f = x: x; }", "foo.nix").functions[0].is_test);
    }

    #[test]
    fn test_non_function_file_has_no_functions() {
        let analysis = analyze("{ a = 1; b = \"x\"; }", "x.nix");
        assert!(analysis.functions.is_empty());
    }
}
