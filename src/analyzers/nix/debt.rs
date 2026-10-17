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
    let complexity = (function.cyclomatic > threshold || function.cognitive > threshold)
        .then(|| complexity_debt(path, function, threshold));
    let nesting = (function.nesting > 4).then(|| nesting_debt(path, function));
    let length = (function.length > 50).then(|| length_debt(path, function));

    complexity
        .into_iter()
        .chain(nesting)
        .chain(length)
        .collect()
}

fn complexity_debt(path: &Path, function: &FunctionMetrics, threshold: u32) -> DebtItem {
    let max_complexity = function.cyclomatic.max(function.cognitive);

    DebtItem {
        id: format!("nix-high-complexity-{}-{}", path.display(), function.line),
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
        context: Some("Consider splitting this expression into smaller bindings.".to_string()),
    }
}

fn nesting_debt(path: &Path, function: &FunctionMetrics) -> DebtItem {
    DebtItem {
        id: format!("nix-deep-nesting-{}-{}", path.display(), function.line),
        debt_type: DebtType::CodeSmell {
            smell_type: Some("deep_nesting".to_string()),
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
        context: Some("Consider extracting nested let/if/with expressions.".to_string()),
    }
}

fn length_debt(path: &Path, function: &FunctionMetrics) -> DebtItem {
    DebtItem {
        id: format!("nix-long-function-{}-{}", path.display(), function.line),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzers::nix::metrics::to_function_metrics;
    use crate::analyzers::nix::types::NixFunction;
    use std::path::PathBuf;

    fn metrics(cyclomatic: u32, nesting: u32, length: usize, is_test: bool) -> FunctionMetrics {
        to_function_metrics(&NixFunction {
            name: "f".to_string(),
            file: PathBuf::from("a.nix"),
            line: 4,
            length,
            cyclomatic,
            cognitive: 0,
            nesting,
            is_test,
            visibility: None,
        })
    }

    #[test]
    fn test_complexity_debt_above_threshold() {
        let items = detect_complexity_debt(Path::new("a.nix"), 5, &[metrics(6, 0, 3, false)]);

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].id, "nix-high-complexity-a.nix-4");
        assert_eq!(items[0].priority, Priority::Medium);
    }

    #[test]
    fn test_no_debt_at_threshold() {
        let items = detect_complexity_debt(Path::new("a.nix"), 5, &[metrics(5, 4, 50, false)]);
        assert!(items.is_empty());
    }

    #[test]
    fn test_nesting_and_length_debt() {
        let items = detect_complexity_debt(Path::new("a.nix"), 50, &[metrics(1, 5, 120, false)]);

        assert_eq!(items.len(), 2);
        assert!(items.iter().any(|i| i.id.starts_with("nix-deep-nesting-")));
        assert!(items.iter().any(|i| i.priority == Priority::High));
    }

    #[test]
    fn test_test_functions_skipped() {
        let items = detect_complexity_debt(Path::new("a.nix"), 1, &[metrics(9, 9, 99, true)]);
        assert!(items.is_empty());
    }

    #[test]
    fn test_critical_priority() {
        assert_eq!(complexity_priority(21, 10), Priority::Critical);
        assert_eq!(complexity_priority(16, 10), Priority::High);
    }
}
