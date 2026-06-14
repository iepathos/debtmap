use crate::core::{DebtItem, DebtType, FunctionMetrics, Priority};
use std::path::Path;

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

fn complexity_debt(path: &Path, function: &FunctionMetrics, threshold: u32) -> DebtItem {
    let max_complexity = function.cyclomatic.max(function.cognitive);

    DebtItem {
        id: format!("go-high-complexity-{}-{}", path.display(), function.line),
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
        id: format!("go-deep-nesting-{}-{}", path.display(), function.line),
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
        id: format!("go-long-function-{}-{}", path.display(), function.line),
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
        id: format!("go-{}-{}-{}", pattern, path.display(), function.line),
        debt_type: DebtType::CodeSmell {
            smell_type: Some(pattern.to_string()),
        },
        priority: definition.priority,
        file: path.to_path_buf(),
        line: function.line,
        column: None,
        message: format!("Function '{}' {}", function.name, definition.message),
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
        "repetitive-error-handling" => AdvisoryDefinition {
            priority: Priority::Low,
            message: "has repetitive error handling",
            context: "Consider extracting a helper if the repeated handling obscures core logic.",
        },
        "swallowed-error" => AdvisoryDefinition {
            priority: Priority::Medium,
            message: "appears to discard an error value",
            context: "Handle, return, or explicitly document why the error can be ignored.",
        },
        "panic-in-production" => AdvisoryDefinition {
            priority: Priority::High,
            message: "calls panic outside test or main code",
            context: "Prefer returning an error unless process termination is intentional.",
        },
        "recover-without-handling" => AdvisoryDefinition {
            priority: Priority::Medium,
            message: "uses recover without visible handling",
            context: "Recover paths should log, convert, or otherwise handle the panic value.",
        },
        "goroutine-without-synchronization" => AdvisoryDefinition {
            priority: Priority::Medium,
            message: "starts a goroutine with no local lifecycle signal",
            context: "Check for cancellation, synchronization, or ownership of the goroutine lifetime.",
        },
        "defer-in-loop" => AdvisoryDefinition {
            priority: Priority::Medium,
            message: "defers work inside a loop",
            context: "Deferred calls in loops can delay cleanup and retain resources longer than expected.",
        },
        "channel-operation" => AdvisoryDefinition {
            priority: Priority::Low,
            message: "uses channel operations",
            context: "Review blocking behavior, buffering, and cancellation for this concurrency path.",
        },
        "pointer-receiver-mutation" => AdvisoryDefinition {
            priority: Priority::Low,
            message: "mutates receiver state",
            context: "State mutation is expected in some methods; keep ownership and invariants explicit.",
        },
        "collection-mutation" => AdvisoryDefinition {
            priority: Priority::Low,
            message: "mutates a map or slice element",
            context: "Review shared ownership and aliasing before passing mutable collections across boundaries.",
        },
        "package-global-mutation" => AdvisoryDefinition {
            priority: Priority::Medium,
            message: "mutates package-level state",
            context: "Package-level mutation can hide dependencies and complicate concurrent use.",
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
