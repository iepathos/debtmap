//! Function analysis for TypeScript/JavaScript
//!
//! Extracts functions from tree-sitter AST and calculates their metrics.

use crate::analyzers::typescript::entropy::calculate_entropy;
use crate::analyzers::typescript::parser::{node_line, node_text};
use crate::analyzers::typescript::patterns::functional::detect_functional_chains;
use crate::analyzers::typescript::purity::TypeScriptPurityAnalyzer;
use crate::analyzers::typescript::types::{FunctionKind, JsFunctionMetrics};
use crate::complexity::entropy_core::EntropyConfig;
use crate::core::ast::TypeScriptAst;
use tree_sitter::Node;

use super::helpers::{
    calculate_cognitive_complexity, calculate_cyclomatic_complexity, calculate_nesting_depth,
    count_function_lines, is_test_function,
};

/// Extract all functions from a TypeScript/JavaScript AST
pub fn extract_functions(
    ast: &TypeScriptAst,
    enable_functional_analysis: bool,
) -> Vec<JsFunctionMetrics> {
    let mut functions = Vec::new();
    let root = ast.tree.root_node();

    extract_functions_recursive(
        &root,
        ast,
        &mut functions,
        false,
        None,
        enable_functional_analysis,
    );

    if enable_functional_analysis {
        annotate_functional_chains(&root, ast, &mut functions);
    }

    // Sort functions for deterministic analysis order (Spec 214 fix)
    functions.sort_by(|a, b| a.line.cmp(&b.line).then_with(|| a.name.cmp(&b.name)));

    functions
}

/// Recursively extract functions from AST nodes
fn extract_functions_recursive(
    node: &Node,
    ast: &TypeScriptAst,
    functions: &mut Vec<JsFunctionMetrics>,
    in_class: bool,
    class_name: Option<&str>,
    enable_functional_analysis: bool,
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
                extract_class_methods(
                    &child,
                    ast,
                    functions,
                    name.as_deref(),
                    enable_functional_analysis,
                );
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
                extract_functions_recursive(
                    &child,
                    ast,
                    functions,
                    in_class,
                    class_name,
                    enable_functional_analysis,
                );
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
    metrics.entropy_score = Some(calculate_entropy(
        &body,
        &ast.source,
        &EntropyConfig::default(),
    ));

    // Purity analysis
    let purity = TypeScriptPurityAnalyzer::analyze_function(node, &ast.source);
    metrics.purity_level = Some(purity.level);
    metrics.purity_confidence = Some(purity.confidence);
    metrics.impurity_reasons = purity.reasons.iter().map(|r| r.to_string()).collect();

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
    metrics.entropy_score = Some(calculate_entropy(
        &body,
        &ast.source,
        &EntropyConfig::default(),
    ));

    // Purity analysis
    let purity = TypeScriptPurityAnalyzer::analyze_function(node, &ast.source);
    metrics.purity_level = Some(purity.level);
    metrics.purity_confidence = Some(purity.confidence);
    metrics.impurity_reasons = purity.reasons.iter().map(|r| r.to_string()).collect();

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
    metrics.entropy_score = Some(calculate_entropy(
        &body_node,
        &ast.source,
        &EntropyConfig::default(),
    ));

    // Purity analysis
    let purity = TypeScriptPurityAnalyzer::analyze_function(node, &ast.source);
    metrics.purity_level = Some(purity.level);
    metrics.purity_confidence = Some(purity.confidence);
    metrics.impurity_reasons = purity.reasons.iter().map(|r| r.to_string()).collect();

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

    let kind = determine_method_kind(node, ast, in_class);

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
        metrics.entropy_score = Some(calculate_entropy(
            &body,
            &ast.source,
            &EntropyConfig::default(),
        ));
    }

    metrics.is_async = has_async_modifier(node);
    metrics.is_test = is_test_function(&name);
    metrics.parameter_count = count_method_parameters(node);

    // Purity analysis
    let purity = TypeScriptPurityAnalyzer::analyze_function(node, &ast.source);
    metrics.purity_level = Some(purity.level);
    metrics.purity_confidence = Some(purity.confidence);
    metrics.impurity_reasons = purity.reasons.iter().map(|r| r.to_string()).collect();

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
    metrics.entropy_score = Some(calculate_entropy(
        &body,
        &ast.source,
        &EntropyConfig::default(),
    ));

    // Purity analysis
    let purity = TypeScriptPurityAnalyzer::analyze_function(node, &ast.source);
    metrics.purity_level = Some(purity.level);
    metrics.purity_confidence = Some(purity.confidence);
    metrics.impurity_reasons = purity.reasons.iter().map(|r| r.to_string()).collect();

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
                extract_class_methods(&child, ast, functions, name.as_deref(), true);
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
    _enable_functional_analysis: bool,
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

fn annotate_functional_chains(
    node: &Node,
    ast: &TypeScriptAst,
    functions: &mut [JsFunctionMetrics],
) {
    let chains = detect_functional_chains(node, ast);
    if chains.is_empty() {
        return;
    }

    for chain in chains {
        if let Some(func) = functions
            .iter_mut()
            .filter(|func| func.line <= chain.line)
            .max_by_key(|func| func.line)
        {
            func.functional_chains.push(chain);
        }
    }
}

#[cfg(test)]
mod functional_analysis_tests {
    use super::*;
    use crate::analyzers::typescript::parser::parse_source;
    use crate::core::ast::JsLanguageVariant;
    use std::path::PathBuf;

    #[test]
    fn test_functional_analysis_toggle_controls_chain_annotation() {
        let source = r#"
const transform = (items) => items.filter(x => x > 0).map(x => x * 2);
"#;
        let path = PathBuf::from("test.ts");
        let ast = parse_source(source, &path, JsLanguageVariant::TypeScript).unwrap();

        let disabled = extract_functions(&ast, false);
        let enabled = extract_functions(&ast, true);

        assert_eq!(disabled.len(), 1);
        assert_eq!(enabled.len(), 1);
        assert!(disabled[0].functional_chains.is_empty());
        assert_eq!(enabled[0].functional_chains.len(), 1);
        assert_eq!(
            enabled[0].functional_chains[0].methods,
            vec!["map".to_string(), "filter".to_string()]
        );
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

            let full_name = field_full_name(class_name, name.as_deref());

            if let Some(mut metrics) = analyze_arrow_function(&value, ast, Some(full_name), false) {
                metrics.kind = FunctionKind::Method;
                functions.push(metrics);
            }
        }
    }
}

fn field_full_name(class_name: Option<&str>, name: Option<&str>) -> String {
    match (class_name, name) {
        (Some(cn), Some(n)) => format!("{}::{}", cn, n),
        (_, Some(n)) => n.to_string(),
        _ => "<field>".to_string(),
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

fn determine_method_kind(node: &Node, ast: &TypeScriptAst, in_class: bool) -> FunctionKind {
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
        if (kind == "property_identifier" || kind == "identifier")
            && node_text(&name, &ast.source) == "constructor"
        {
            return FunctionKind::Constructor;
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
        let path = PathBuf::from("test.ts");
        let ast = parse_source(source, &path, JsLanguageVariant::TypeScript).unwrap();

        let functions = extract_functions(&ast, false);

        assert_eq!(functions.len(), 1);
        assert_eq!(functions[0].name, "hello");
        assert_eq!(functions[0].kind, FunctionKind::Declaration);
    }

    #[test]
    fn test_extract_arrow_function() {
        let source = "const greet = (name) => `Hello ${name}`;";
        let path = PathBuf::from("test.ts");
        let ast = parse_source(source, &path, JsLanguageVariant::TypeScript).unwrap();

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
        assert!(
            functions
                .iter()
                .any(|function| function.kind == FunctionKind::Constructor),
            "class constructor should be classified as Constructor"
        );
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

    #[test]
    fn test_entropy_score_populated() {
        let source = r#"
function validateInput(a, b, c, d) {
    if (!a) throw new Error("a is required");
    if (!b) throw new Error("b is required");
    if (!c) throw new Error("c is required");
    if (!d) throw new Error("d is required");
    return { a, b, c, d };
}
"#;
        let path = PathBuf::from("test.js");
        let ast = parse_source(source, &path, JsLanguageVariant::JavaScript).unwrap();

        let functions = extract_functions(&ast, false);

        assert_eq!(functions.len(), 1);
        assert!(
            functions[0].entropy_score.is_some(),
            "entropy_score should be populated for JavaScript functions"
        );

        let entropy = functions[0].entropy_score.as_ref().unwrap();
        assert!(
            entropy.token_entropy >= 0.0 && entropy.token_entropy <= 1.0,
            "token_entropy should be in range [0, 1]: {}",
            entropy.token_entropy
        );
        assert!(
            entropy.pattern_repetition >= 0.0,
            "pattern_repetition should be non-negative"
        );
    }

    #[test]
    fn test_entropy_dampening_for_validation_code() {
        // Validation code with repetitive patterns should have high repetition score
        let source = r#"
function validate(a, b, c, d, e) {
    if (!a) throw new Error("a");
    if (!b) throw new Error("b");
    if (!c) throw new Error("c");
    if (!d) throw new Error("d");
    if (!e) throw new Error("e");
    return true;
}
"#;
        let path = PathBuf::from("test.js");
        let ast = parse_source(source, &path, JsLanguageVariant::JavaScript).unwrap();

        let functions = extract_functions(&ast, false);
        let entropy = functions[0].entropy_score.as_ref().unwrap();

        // Repetitive validation code should have pattern repetition detected
        assert!(
            entropy.pattern_repetition > 0.0 || entropy.token_entropy < 0.5,
            "Validation code should have either high repetition or low entropy: repetition={}, entropy={}",
            entropy.pattern_repetition,
            entropy.token_entropy
        );
    }

    #[test]
    fn test_purity_analysis_pure_function() {
        use crate::core::PurityLevel;

        let source = "function add(a, b) { return a + b; }";
        let path = PathBuf::from("test.js");
        let ast = parse_source(source, &path, JsLanguageVariant::JavaScript).unwrap();

        let functions = extract_functions(&ast, false);

        assert_eq!(functions.len(), 1);
        assert!(
            functions[0].purity_level.is_some(),
            "purity_level should be populated"
        );
        assert_eq!(
            functions[0].purity_level,
            Some(PurityLevel::StrictlyPure),
            "pure arithmetic function should be StrictlyPure"
        );
        assert!(
            functions[0].purity_confidence.is_some(),
            "purity_confidence should be populated"
        );
        assert!(
            functions[0].impurity_reasons.is_empty(),
            "pure function should have no impurity reasons"
        );
    }

    #[test]
    fn test_purity_analysis_impure_function() {
        use crate::core::PurityLevel;

        let source = "function log(msg) { console.log(msg); }";
        let path = PathBuf::from("test.js");
        let ast = parse_source(source, &path, JsLanguageVariant::JavaScript).unwrap();

        let functions = extract_functions(&ast, false);

        assert_eq!(functions.len(), 1);
        assert_eq!(
            functions[0].purity_level,
            Some(PurityLevel::Impure),
            "console.log should make function Impure"
        );
        assert!(
            !functions[0].impurity_reasons.is_empty(),
            "impure function should have impurity reasons"
        );
        assert!(
            functions[0]
                .impurity_reasons
                .iter()
                .any(|r| r.contains("console")),
            "impurity reasons should mention console"
        );
    }

    #[test]
    fn test_purity_flows_to_function_metrics() {
        use crate::analyzers::typescript::visitor::helpers::convert_to_function_metrics;
        use crate::complexity::threshold_manager::ComplexityThresholds;
        use crate::core::PurityLevel;

        // Test that purity data flows from JsFunctionMetrics to FunctionMetrics
        let source = "const rand = () => Math.random();";
        let path = PathBuf::from("test.js");
        let ast = parse_source(source, &path, JsLanguageVariant::JavaScript).unwrap();

        let js_functions = extract_functions(&ast, false);
        assert_eq!(js_functions.len(), 1);

        let function_metrics =
            convert_to_function_metrics(&js_functions[0], &ComplexityThresholds::default());

        // Verify purity data was transferred
        assert_eq!(
            function_metrics.purity_level,
            Some(PurityLevel::Impure),
            "Math.random should make function Impure"
        );
        assert_eq!(
            function_metrics.is_pure,
            Some(false),
            "is_pure should be derived from purity_level"
        );
        assert!(
            function_metrics.purity_confidence.is_some(),
            "purity_confidence should be populated"
        );
        assert!(
            function_metrics.purity_reason.is_some(),
            "purity_reason should be populated for impure functions"
        );
    }

    #[test]
    fn test_field_full_name_helper() {
        // With class and field name
        assert_eq!(
            super::field_full_name(Some("Greeter"), Some("greet")),
            "Greeter::greet"
        );
        // Without class name, keep field name
        assert_eq!(super::field_full_name(None, Some("greet")), "greet");
        // Missing field name falls back to placeholder
        assert_eq!(super::field_full_name(Some("Greeter"), None), "<field>");
    }
}
