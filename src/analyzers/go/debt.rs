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

fn complexity_priority(complexity: u32, threshold: u32) -> Priority {
    if complexity > threshold * 2 {
        Priority::Critical
    } else if complexity > threshold + 5 {
        Priority::High
    } else {
        Priority::Medium
    }
}
