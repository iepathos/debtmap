pub mod security_patterns;

use std::path::Path;

use crate::analyzers::solidity::debt::security_patterns::detect_contract_patterns;
use crate::config::SolidityLanguageConfig;
use crate::core::ast::SolidityAst;
use crate::core::{DebtItem, DebtType, FunctionMetrics, Priority};
use crate::debt::patterns::find_todos_and_fixmes_with_suppression;
use crate::debt::smells::analyze_function_smells;
use crate::debt::suppression::{SuppressionContext, parse_suppression_comments};

pub fn detect_debt(
    path: &Path,
    threshold: u32,
    functions: &[FunctionMetrics],
    ast: &SolidityAst,
    skip_debt: bool,
    config: &SolidityLanguageConfig,
) -> Vec<DebtItem> {
    if skip_debt {
        return Vec::new();
    }

    let suppression =
        parse_suppression_comments(&ast.source, crate::core::Language::Solidity, path);
    let mut items = if config.features.detect_complexity {
        detect_complexity_debt(path, threshold, functions)
    } else {
        advisory_debt_items_for_functions(path, functions)
    };
    items.extend(find_todos_and_fixmes_with_suppression(
        &ast.source,
        path,
        Some(&suppression),
    ));
    items.extend(function_smell_debt(functions));
    items.extend(natspec_debt(path, ast, functions));
    items.extend(contract_level_debt(path, ast, config));
    filter_suppressed_items(items, &suppression)
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

fn natspec_debt(path: &Path, ast: &SolidityAst, functions: &[FunctionMetrics]) -> Vec<DebtItem> {
    let lines = ast.source.lines().collect::<Vec<_>>();
    functions
        .iter()
        .filter(|function| !function.is_test)
        .filter(|function| matches!(function.visibility.as_deref(), Some("public" | "external")))
        .filter(|function| !has_natspec_before(&lines, function.line))
        .map(|function| missing_natspec_debt(path, function))
        .collect()
}

fn has_natspec_before(lines: &[&str], function_line: usize) -> bool {
    let start = function_line.saturating_sub(6);
    let end = function_line.saturating_sub(1);
    lines.get(start..end).unwrap_or(&[]).iter().any(|line| {
        let trimmed = line.trim();
        trimmed.starts_with("///") && (trimmed.contains("@notice") || trimmed.contains("@dev"))
    })
}

fn missing_natspec_debt(path: &Path, function: &FunctionMetrics) -> DebtItem {
    DebtItem {
        id: format!(
            "solidity-missing-natspec-{}-{}",
            path.display(),
            function.line
        ),
        debt_type: DebtType::CodeSmell {
            smell_type: Some("missing-natspec".to_string()),
        },
        priority: Priority::Low,
        file: path.to_path_buf(),
        line: function.line,
        column: None,
        message: format!(
            "Function '{}' is public/external without NatSpec",
            function.name
        ),
        context: Some(
            "Add /// @notice or /// @dev documentation for external callers.".to_string(),
        ),
    }
}

fn contract_level_debt(
    path: &Path,
    ast: &SolidityAst,
    config: &SolidityLanguageConfig,
) -> Vec<DebtItem> {
    let root = ast.tree.root_node();
    let mut items = Vec::new();
    collect_contract_debt(root, path, ast, config, &mut items);
    items
}

fn collect_contract_debt(
    node: tree_sitter::Node,
    path: &Path,
    ast: &SolidityAst,
    config: &SolidityLanguageConfig,
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
        for pattern in detect_contract_patterns(node, &ast.source, function_count, config) {
            if let Some(item) = contract_advisory(path, name, &pattern, node) {
                items.push(item);
            }
        }
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_contract_debt(child, path, ast, config, items);
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

fn advisory_debt_items_for_functions(path: &Path, functions: &[FunctionMetrics]) -> Vec<DebtItem> {
    functions
        .iter()
        .filter(|function| !function.is_test)
        .flat_map(|function| advisory_debt_items(path, function))
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

fn filter_suppressed_items(
    items: Vec<DebtItem>,
    suppression: &SuppressionContext,
) -> Vec<DebtItem> {
    items
        .into_iter()
        .filter(|item| !is_suppressed_item(item, suppression))
        .collect()
}

fn is_suppressed_item(item: &DebtItem, suppression: &SuppressionContext) -> bool {
    suppression.is_suppressed(item.line, &item.debt_type)
        || suppression.is_function_allowed(item.line, &item.debt_type)
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
        "unchecked-arithmetic" => AdvisoryDefinition {
            priority: Priority::Medium,
            message: "uses an unchecked arithmetic block",
            context: "Unchecked arithmetic skips overflow checks; confirm bounds are proven elsewhere.",
        },
        "unsafe-erc20-transfer" => AdvisoryDefinition {
            priority: Priority::Medium,
            message: "may call ERC20 transfer functions without checking the result",
            context: "Use SafeERC20 wrappers or explicitly validate token transfer return values.",
        },
        "push-without-length-cap" => AdvisoryDefinition {
            priority: Priority::Medium,
            message: "pushes to a collection without an obvious length cap",
            context: "Unbounded storage growth can increase gas costs or create denial-of-service risk.",
        },
        "block-timestamp-dependency" => AdvisoryDefinition {
            priority: Priority::Medium,
            message: "depends on block.timestamp",
            context: "Timestamp-dependent logic can be miner/validator-influenced within protocol limits.",
        },
        "tx-gas-price-dependency" => AdvisoryDefinition {
            priority: Priority::Low,
            message: "depends on tx.gasprice",
            context: "Gas price assumptions are brittle across fee markets and transaction relayers.",
        },
        "encode-packed-collision" => AdvisoryDefinition {
            priority: Priority::Medium,
            message: "uses abi.encodePacked",
            context: "Packed encoding with dynamic values can create hash collision ambiguity; review typed inputs.",
        },
        "delegatecall-in-constructor" => AdvisoryDefinition {
            priority: Priority::High,
            message: "uses delegatecall during construction",
            context: "Constructor delegatecalls can create proxy initialization and storage-layout risks.",
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
