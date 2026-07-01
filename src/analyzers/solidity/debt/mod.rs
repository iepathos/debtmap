pub mod security_patterns;

use std::path::Path;

use crate::analyzers::solidity::debt::security_patterns::detect_contract_patterns;
use crate::core::ast::SolidityAst;
use crate::core::{DebtItem, DebtType, FunctionMetrics, Priority};
use crate::debt::patterns::find_todos_and_fixmes;
use crate::debt::smells::analyze_function_smells;

pub fn detect_debt(
    path: &Path,
    threshold: u32,
    functions: &[FunctionMetrics],
    ast: &SolidityAst,
    skip_debt: bool,
) -> Vec<DebtItem> {
    if skip_debt {
        return Vec::new();
    }

    let mut items = detect_complexity_debt(path, threshold, functions);
    items.extend(find_todos_and_fixmes(&ast.source, path));
    items.extend(function_smell_debt(functions));
    items.extend(contract_level_debt(path, ast));
    items
}

pub fn detect_complexity_debt(
    path: &Path,
    threshold: u32,
    functions: &[FunctionMetrics],
) -> Vec<DebtItem> {
    functions
        .iter()
        .filter(|function| !function.is_test)
        .flat_map(|function| debt_for_function(path, threshold, function))
        .collect()
}

fn debt_for_function(path: &Path, threshold: u32, function: &FunctionMetrics) -> Vec<DebtItem> {
    let mut items = Vec::new();

    if function.cyclomatic > threshold || function.cognitive > threshold {
        items.push(complexity_debt(path, function, threshold));
    }

    if function.nesting > 4 {
        items.push(nesting_debt(path, function));
    }

    if function.length > 50 {
        items.push(length_debt(path, function));
    }

    items.extend(advisory_debt_items(path, function));
    items
}

fn function_smell_debt(functions: &[FunctionMetrics]) -> Vec<DebtItem> {
    functions
        .iter()
        .filter(|function| !function.is_test)
        .flat_map(|function| analyze_function_smells(function, 0))
        .map(|smell| smell.to_debt_item())
        .collect()
}

fn contract_level_debt(path: &Path, ast: &SolidityAst) -> Vec<DebtItem> {
    let root = ast.tree.root_node();
    let mut items = Vec::new();
    collect_contract_debt(root, path, ast, &mut items);
    items
}

fn collect_contract_debt(
    node: tree_sitter::Node,
    path: &Path,
    ast: &SolidityAst,
    items: &mut Vec<DebtItem>,
) {
    if matches!(
        node.kind(),
        "contract_declaration" | "interface_declaration" | "library_declaration"
    ) {
        let name = node
            .child_by_field_name("name")
            .map(|n| crate::analyzers::solidity::parser::node_text(&n, &ast.source))
            .unwrap_or("Contract");
        let function_count = count_callables(node);
        for pattern in detect_contract_patterns(node, &ast.source, function_count) {
            if let Some(item) = contract_advisory(path, name, &pattern, node) {
                items.push(item);
            }
        }
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_contract_debt(child, path, ast, items);
    }
}

fn contract_advisory(
    path: &Path,
    contract_name: &str,
    pattern: &str,
    node: tree_sitter::Node,
) -> Option<DebtItem> {
    let definition = contract_advisory_definition(pattern)?;
    let line = crate::analyzers::solidity::parser::node_line(&node);

    Some(DebtItem {
        id: format!("solidity-{pattern}-{}-{line}", path.display()),
        debt_type: DebtType::CodeSmell {
            smell_type: Some(pattern.to_string()),
        },
        priority: definition.priority,
        file: path.to_path_buf(),
        line,
        column: None,
        message: format!("Contract '{contract_name}' {}", definition.message),
        context: Some(definition.context.to_string()),
    })
}

fn complexity_debt(path: &Path, function: &FunctionMetrics, threshold: u32) -> DebtItem {
    let max_complexity = function.cyclomatic.max(function.cognitive);

    DebtItem {
        id: format!(
            "solidity-high-complexity-{}-{}",
            path.display(),
            function.line
        ),
        debt_type: DebtType::Complexity {
            cyclomatic: function.cyclomatic,
            cognitive: function.cognitive,
        },
        priority: complexity_priority(max_complexity, threshold),
        file: path.to_path_buf(),
        line: function.line,
        column: None,
        message: format!(
            "Function '{}' has high complexity (cyclomatic: {}, cognitive: {})",
            function.name, function.cyclomatic, function.cognitive
        ),
        context: Some("Consider breaking this function into smaller functions.".to_string()),
    }
}

fn nesting_debt(path: &Path, function: &FunctionMetrics) -> DebtItem {
    DebtItem {
        id: format!("solidity-deep-nesting-{}-{}", path.display(), function.line),
        debt_type: DebtType::NestedLoops {
            depth: function.nesting,
            complexity_estimate: format!("O(n^{})", function.nesting),
        },
        priority: if function.nesting > 6 {
            Priority::High
        } else {
            Priority::Medium
        },
        file: path.to_path_buf(),
        line: function.line,
        column: None,
        message: format!(
            "Function '{}' has deep nesting ({} levels)",
            function.name, function.nesting
        ),
        context: Some("Consider guard clauses or extracting nested logic.".to_string()),
    }
}

fn length_debt(path: &Path, function: &FunctionMetrics) -> DebtItem {
    DebtItem {
        id: format!(
            "solidity-long-function-{}-{}",
            path.display(),
            function.line
        ),
        debt_type: DebtType::CodeSmell {
            smell_type: Some("long_function".to_string()),
        },
        priority: if function.length > 100 {
            Priority::High
        } else {
            Priority::Medium
        },
        file: path.to_path_buf(),
        line: function.line,
        column: None,
        message: format!(
            "Function '{}' is too long ({} lines)",
            function.name, function.length
        ),
        context: Some("Consider splitting this function into smaller units.".to_string()),
    }
}

fn advisory_debt_items(path: &Path, function: &FunctionMetrics) -> Vec<DebtItem> {
    function
        .detected_patterns
        .clone()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|pattern| advisory_debt(path, function, &pattern))
        .collect()
}

fn advisory_debt(path: &Path, function: &FunctionMetrics, pattern: &str) -> Option<DebtItem> {
    let definition = advisory_definition(pattern)?;

    Some(DebtItem {
        id: format!("solidity-{pattern}-{}-{}", path.display(), function.line),
        debt_type: DebtType::CodeSmell {
            smell_type: Some(pattern.to_string()),
        },
        priority: definition.priority,
        file: path.to_path_buf(),
        line: function.line,
        column: None,
        message: format!(
            "Function '{}' {} — review recommended",
            function.name, definition.message
        ),
        context: Some(definition.context.to_string()),
    })
}

struct AdvisoryDefinition {
    priority: Priority,
    message: &'static str,
    context: &'static str,
}

fn advisory_definition(pattern: &str) -> Option<AdvisoryDefinition> {
    let definition = match pattern {
        "tx-origin-usage" => AdvisoryDefinition {
            priority: Priority::High,
            message: "uses tx.origin for authorization",
            context: "Prefer msg.sender; tx.origin is vulnerable to phishing-style attacks.",
        },
        "unchecked-low-level-call" => AdvisoryDefinition {
            priority: Priority::High,
            message: "may perform an unchecked low-level call",
            context: "Check the return value or use a safe wrapper that reverts on failure.",
        },
        "delegatecall-usage" => AdvisoryDefinition {
            priority: Priority::Medium,
            message: "uses delegatecall",
            context: "Delegatecall executes foreign code in this contract's context; review carefully.",
        },
        "selfdestruct-usage" => AdvisoryDefinition {
            priority: Priority::Medium,
            message: "uses selfdestruct",
            context: "Contract destruction is irreversible; confirm this is intentional.",
        },
        "assembly-block" => AdvisoryDefinition {
            priority: Priority::Medium,
            message: "contains inline assembly",
            context: "Assembly bypasses Solidity safety checks and increases audit complexity.",
        },
        "unbounded-loop" => AdvisoryDefinition {
            priority: Priority::Medium,
            message: "contains a potentially unbounded loop",
            context: "Unbounded loops can cause gas DoS; consider pagination or caps.",
        },
        "external-call-before-state-update" => AdvisoryDefinition {
            priority: Priority::High,
            message: "may perform an external call before updating state",
            context: "Follow checks-effects-interactions; external calls before state updates increase reentrancy risk.",
        },
        "hardcoded-address" => AdvisoryDefinition {
            priority: Priority::Low,
            message: "contains a hardcoded address literal",
            context: "Hardcoded addresses reduce maintainability across deployments.",
        },
        "missing-access-control" => AdvisoryDefinition {
            priority: Priority::Medium,
            message: "is public/external without visible access control",
            context: "Confirm authorization is enforced via modifiers or explicit checks.",
        },
        _ => return None,
    };

    Some(definition)
}

fn contract_advisory_definition(pattern: &str) -> Option<AdvisoryDefinition> {
    let definition = match pattern {
        "floating-pragma" => AdvisoryDefinition {
            priority: Priority::Low,
            message: "uses a floating pragma",
            context: "Pin compiler versions to reduce unexpected behavior across builds.",
        },
        "large-contract" => AdvisoryDefinition {
            priority: Priority::Medium,
            message: "is large (many functions or state variables)",
            context: "Large contracts are harder to audit and may approach deployment size limits.",
        },
        _ => return None,
    };

    Some(definition)
}

fn complexity_priority(complexity: u32, threshold: u32) -> Priority {
    if complexity > threshold * 2 {
        Priority::Critical
    } else if complexity > threshold + 5 {
        Priority::High
    } else {
        Priority::Medium
    }
}

fn count_callables(node: tree_sitter::Node) -> usize {
    let mut count = usize::from(matches!(
        node.kind(),
        "function_definition" | "modifier_definition" | "constructor" | "fallback" | "receive"
    ));
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        count += count_callables(child);
    }
    count
}
