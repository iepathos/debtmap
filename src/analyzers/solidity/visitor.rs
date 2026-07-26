use crate::analyzers::solidity::advanced::detect_advanced_patterns;
use crate::analyzers::solidity::calls::{call_display, extract_calls};
use crate::analyzers::solidity::complexity::{
    cognitive_complexity, cyclomatic_complexity, function_length, max_nesting,
};
use crate::analyzers::solidity::debt::security_patterns::detect_function_patterns;
use crate::analyzers::solidity::effects::{analyze_callable_effects, mutability_mismatch_patterns};
use crate::analyzers::solidity::entropy::calculate_entropy;
use crate::analyzers::solidity::extraction::{SolidityExtraction, extract_solidity};
use crate::analyzers::solidity::parser::{node_line, node_text};
use crate::analyzers::solidity::test_detection::function_is_test;
use crate::analyzers::solidity::types::{SolidityAnalysis, SolidityFunction, SolidityFunctionKind};
use crate::complexity::entropy_core::{EntropyAnalysis, EntropyConfig};
use crate::config::SolidityLanguageConfig;
use crate::core::ast::SolidityAst;
use tree_sitter::Node;

pub fn analyze_ast(ast: &SolidityAst, config: &SolidityLanguageConfig) -> SolidityAnalysis {
    let extraction = extract_solidity(ast);
    analyze_extracted(ast, config, &extraction)
}

pub fn analyze_extracted(
    ast: &SolidityAst,
    config: &SolidityLanguageConfig,
    extraction: &SolidityExtraction<'_>,
) -> SolidityAnalysis {
    let is_test_file =
        crate::analyzers::solidity::test_detection::is_test_context(&ast.path, &ast.source, None);
    let has_floating_pragma =
        crate::analyzers::solidity::test_detection::has_floating_pragma(&ast.source);
    let contracts = extraction
        .contracts
        .iter()
        .map(|contract| contract.info.clone())
        .collect::<Vec<_>>();
    let mut functions = extraction
        .callables
        .iter()
        .filter_map(|callable| {
            callable_from_node(
                callable.node,
                ast,
                config,
                callable.contract_name.clone(),
                &contracts,
                &extraction.modifier_bodies,
            )
        })
        .collect::<Vec<_>>();
    functions.sort_by(|a, b| a.line.cmp(&b.line).then_with(|| a.name.cmp(&b.name)));

    SolidityAnalysis {
        contracts,
        functions,
        is_test_file,
        has_floating_pragma,
    }
}

fn callable_from_node(
    node: Node,
    ast: &SolidityAst,
    config: &SolidityLanguageConfig,
    contract_name: Option<String>,
    contracts: &[crate::analyzers::solidity::types::ContractInfo],
    modifiers: &std::collections::HashMap<String, Node<'_>>,
) -> Option<SolidityFunction> {
    let (kind, name) = callable_kind_and_name(node, ast, contract_name.as_deref())?;
    let body = node.child_by_field_name("body")?;
    let visibility = visibility_from_node(node, ast);
    let state_mutability = state_mutability_from_node(node, ast);
    let qualified_name = qualify_name(contract_name.as_deref(), &name);
    let is_test = function_is_test(
        &ast.path,
        &ast.source,
        contract_name.as_deref(),
        &qualified_name,
    );
    let state_variables = contract_name
        .as_ref()
        .and_then(|contract| {
            contracts
                .iter()
                .find(|info| info.name == *contract)
                .map(|info| info.state_variables.clone())
        })
        .unwrap_or_default();
    let effects = analyze_callable_effects(node, &ast.source, &state_variables, modifiers);

    let mut advisory_patterns =
        detect_function_patterns(node, &ast.source, visibility.as_deref(), is_test, config);
    advisory_patterns.extend(detect_advanced_patterns(node, &ast.source, config));
    advisory_patterns.extend(mutability_mismatch_patterns(
        state_mutability.as_deref(),
        &effects,
    ));
    advisory_patterns.sort();
    advisory_patterns.dedup();
    let cognitive = cognitive_complexity(body, &ast.source, 0);
    let entropy_analysis = entropy_analysis_for_body(body, &ast.source, cognitive);

    Some(SolidityFunction {
        name: qualified_name,
        file: ast.path.clone(),
        line: node_line(&node),
        length: function_length(node),
        cyclomatic: cyclomatic_complexity(body, &ast.source),
        cognitive,
        nesting: max_nesting(body, 0),
        kind,
        is_test,
        visibility,
        calls: extract_calls(body, ast).iter().map(call_display).collect(),
        advisory_patterns,
        contract_name,
        entropy_analysis,
        state_mutability,
        effects,
    })
}

fn entropy_analysis_for_body(
    body: Node,
    source: &str,
    cognitive_complexity: u32,
) -> Option<EntropyAnalysis> {
    let config = EntropyConfig::default();
    config.enabled.then(|| {
        let raw = calculate_entropy(body, source, &config);
        EntropyAnalysis::from_raw(&raw, cognitive_complexity, &config)
    })
}

fn callable_kind_and_name(
    node: Node,
    ast: &SolidityAst,
    contract_name: Option<&str>,
) -> Option<(SolidityFunctionKind, String)> {
    match node.kind() {
        "function_definition" => {
            let name = node
                .child_by_field_name("name")
                .map(|n| node_text(&n, &ast.source).to_string())
                .unwrap_or_else(|| "function".to_string());
            Some((SolidityFunctionKind::Function, name))
        }
        "modifier_definition" => {
            let name = node_text(&node.child_by_field_name("name")?, &ast.source).to_string();
            Some((SolidityFunctionKind::Modifier, name))
        }
        "constructor" | "constructor_definition" => Some((
            SolidityFunctionKind::Constructor,
            format!("{}.constructor", contract_name.unwrap_or("Contract")),
        )),
        "fallback" => Some((
            SolidityFunctionKind::Fallback,
            format!("{}.fallback", contract_name.unwrap_or("Contract")),
        )),
        "receive" => Some((
            SolidityFunctionKind::Receive,
            format!("{}.receive", contract_name.unwrap_or("Contract")),
        )),
        _ => None,
    }
}

fn qualify_name(contract_name: Option<&str>, name: &str) -> String {
    contract_name
        .map(|contract| format!("{contract}.{name}"))
        .unwrap_or_else(|| name.to_string())
}

fn visibility_from_node(node: Node, ast: &SolidityAst) -> Option<String> {
    visibility_in_subtree(node, ast).or_else(|| {
        let text = node_text(&node, &ast.source);
        for visibility in ["public", "external", "internal", "private"] {
            if text.contains(visibility) {
                return Some(visibility.to_string());
            }
        }
        None
    })
}

fn state_mutability_from_node(node: Node, ast: &SolidityAst) -> Option<String> {
    let text = node_text(&node, &ast.source);
    ["pure", "view", "payable"]
        .into_iter()
        .find(|mutability| text.split_whitespace().any(|word| word == *mutability))
        .map(str::to_string)
}

fn visibility_in_subtree(node: Node, ast: &SolidityAst) -> Option<String> {
    if node.kind() == "visibility" {
        return Some(node_text(&node, &ast.source).to_string());
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if let Some(visibility) = visibility_in_subtree(child, ast) {
            return Some(visibility);
        }
    }
    None
}

#[cfg(test)]
#[path = "visitor_equivalence_tests.rs"]
mod equivalence_tests;
#[cfg(test)]
#[path = "visitor_legacy_oracle.rs"]
mod legacy_oracle;
