use crate::analyzers::solidity::advanced::detect_advanced_patterns;
use crate::analyzers::solidity::calls::{call_display, extract_calls};
use crate::analyzers::solidity::complexity::{
    cognitive_complexity, cyclomatic_complexity, function_length, max_nesting,
};
use crate::analyzers::solidity::debt::security_patterns::detect_function_patterns;
use crate::analyzers::solidity::effects::{
    analyze_callable_effects, modifier_bodies_by_name, mutability_mismatch_patterns,
    state_variable_names,
};
use crate::analyzers::solidity::entropy::calculate_entropy;
use crate::analyzers::solidity::parser::{node_line, node_text};
use crate::analyzers::solidity::test_detection::function_is_test;
use crate::analyzers::solidity::types::{
    ContractInfo, ContractKind, SolidityAnalysis, SolidityFunction, SolidityFunctionKind,
};
use crate::complexity::entropy_core::{EntropyAnalysis, EntropyConfig};
use crate::config::SolidityLanguageConfig;
use crate::core::ast::SolidityAst;
use tree_sitter::Node;

pub fn analyze_ast(ast: &SolidityAst, config: &SolidityLanguageConfig) -> SolidityAnalysis {
    let root = ast.tree.root_node();
    let is_test_file =
        crate::analyzers::solidity::test_detection::is_test_context(&ast.path, &ast.source, None);
    let has_floating_pragma =
        crate::analyzers::solidity::test_detection::has_floating_pragma(&ast.source);

    let mut analysis = SolidityAnalysis {
        is_test_file,
        has_floating_pragma,
        ..Default::default()
    };

    collect_contracts(root, ast, &mut analysis);
    let modifiers = modifier_bodies_by_name(root, &ast.source);
    collect_callables(
        root,
        ast,
        config,
        None,
        &analysis.contracts,
        &modifiers,
        &mut analysis.functions,
    );

    analysis
        .functions
        .sort_by(|a, b| a.line.cmp(&b.line).then_with(|| a.name.cmp(&b.name)));

    analysis
}

fn collect_contracts(node: Node, ast: &SolidityAst, analysis: &mut SolidityAnalysis) {
    if let Some(info) = contract_from_node(node, ast) {
        analysis.contracts.push(info);
    }

    walk_children(node, |child| collect_contracts(child, ast, analysis));
}

fn contract_from_node(node: Node, ast: &SolidityAst) -> Option<ContractInfo> {
    let (kind, name_node) = match node.kind() {
        "contract_declaration" => (ContractKind::Contract, node.child_by_field_name("name")?),
        "interface_declaration" => (ContractKind::Interface, node.child_by_field_name("name")?),
        "library_declaration" => (ContractKind::Library, node.child_by_field_name("name")?),
        _ => return None,
    };

    let name = node_text(&name_node, &ast.source).to_string();
    let base_classes = inheritance_names(node, ast);
    let state_variable_count = count_nodes(node, "state_variable_declaration");
    let function_count = count_callables(node);

    Some(ContractInfo {
        name,
        kind,
        base_classes,
        state_variable_count,
        function_count,
        state_variables: state_variable_names(node, &ast.source),
    })
}

fn collect_callables(
    node: Node,
    ast: &SolidityAst,
    config: &SolidityLanguageConfig,
    contract_name: Option<String>,
    contracts: &[ContractInfo],
    modifiers: &std::collections::HashMap<String, Node<'_>>,
    functions: &mut Vec<SolidityFunction>,
) {
    let current_contract = contract_name.or_else(|| contract_name_from_ancestor(node, ast));

    if let Some(function) = callable_from_node(
        node,
        ast,
        config,
        current_contract.clone(),
        contracts,
        modifiers,
    ) {
        functions.push(function);
    }

    let next_contract = match node.kind() {
        "contract_declaration" | "interface_declaration" | "library_declaration" => node
            .child_by_field_name("name")
            .map(|name| node_text(&name, &ast.source).to_string())
            .or(current_contract),
        _ => current_contract,
    };

    walk_children(node, |child| {
        collect_callables(
            child,
            ast,
            config,
            next_contract.clone(),
            contracts,
            modifiers,
            functions,
        )
    });
}

fn callable_from_node(
    node: Node,
    ast: &SolidityAst,
    config: &SolidityLanguageConfig,
    contract_name: Option<String>,
    contracts: &[ContractInfo],
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

fn inheritance_names(node: Node, ast: &SolidityAst) -> Vec<String> {
    let mut names = Vec::new();
    walk_children(node, |child| {
        if child.kind() == "inheritance_specifier" {
            if let Some(name) = child.child_by_field_name("name") {
                names.push(node_text(&name, &ast.source).to_string());
            }
        }
    });
    names
}

fn contract_name_from_ancestor(node: Node, ast: &SolidityAst) -> Option<String> {
    let mut current = node.parent();
    while let Some(parent) = current {
        if let Some(name) = parent.child_by_field_name("name") {
            if matches!(
                parent.kind(),
                "contract_declaration" | "interface_declaration" | "library_declaration"
            ) {
                return Some(node_text(&name, &ast.source).to_string());
            }
        }
        current = parent.parent();
    }
    None
}

fn count_nodes(node: Node, kind: &str) -> usize {
    let mut count = usize::from(node.kind() == kind);
    walk_children(node, |child| count += count_nodes(child, kind));
    count
}

fn count_callables(node: Node) -> usize {
    let mut count = usize::from(is_callable_node(node));
    walk_children(node, |child| count += count_callables(child));
    count
}

fn is_callable_node(node: Node) -> bool {
    matches!(
        node.kind(),
        "function_definition"
            | "modifier_definition"
            | "constructor"
            | "constructor_definition"
            | "fallback"
            | "receive"
    )
}

fn walk_children(node: Node, mut f: impl FnMut(Node)) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        f(child);
    }
}
