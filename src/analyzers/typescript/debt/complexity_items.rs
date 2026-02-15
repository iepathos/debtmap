//! Complexity-based debt detection
//!
//! Detects debt items based on function complexity metrics.

use crate::core::{DebtItem, FunctionMetrics, Priority};
use crate::priority::DebtType;
use std::path::Path;

/// Detect complexity-based debt items
pub fn detect_complexity_debt(
    path: &Path,
    threshold: u32,
    functions: &[FunctionMetrics],
) -> Vec<DebtItem> {
    let mut items = Vec::new();

    for func in functions {
        // Skip test functions
        if func.is_test {
            continue;
        }

        // High complexity (cyclomatic or cognitive)
        if func.cyclomatic > threshold || func.cognitive > threshold {
            items.push(create_complexity_debt(path, func, threshold));
        }

        // Deep nesting
        if func.nesting > 4 {
            items.push(create_nesting_debt(path, func));
        }

        // Long function
        if func.length > 50 {
            items.push(create_length_debt(path, func));
        }
    }

    items
}

fn create_complexity_debt(path: &Path, func: &FunctionMetrics, threshold: u32) -> DebtItem {
    let max_complexity = func.cyclomatic.max(func.cognitive);
    let priority = if max_complexity > threshold * 2 {
        Priority::Critical
    } else if max_complexity > threshold + 5 {
        Priority::High
    } else {
        Priority::Medium
    };

    DebtItem {
        id: format!("js-high-complexity-{}-{}", path.display(), func.line),
        debt_type: DebtType::Complexity {
            cyclomatic: func.cyclomatic,
            cognitive: func.cognitive,
        },
        priority,
        file: path.to_path_buf(),
        line: func.line,
        column: None,
        message: format!(
            "Function '{}' has high complexity (cyclomatic: {}, cognitive: {})",
            func.name, func.cyclomatic, func.cognitive
        ),
        context: Some(
            "Consider breaking this function into smaller, more focused functions. \
             High complexity makes code harder to test and maintain."
                .to_string(),
        ),
    }
}

fn create_nesting_debt(path: &Path, func: &FunctionMetrics) -> DebtItem {
    let priority = if func.nesting > 6 {
        Priority::High
    } else {
        Priority::Medium
    };

    DebtItem {
        id: format!("js-deep-nesting-{}-{}", path.display(), func.line),
        debt_type: DebtType::NestedLoops {
            depth: func.nesting,
            complexity_estimate: format!("O(n^{})", func.nesting),
        },
        priority,
        file: path.to_path_buf(),
        line: func.line,
        column: None,
        message: format!(
            "Function '{}' has deep nesting ({} levels)",
            func.name, func.nesting
        ),
        context: Some(
            "Deep nesting makes code harder to follow. Consider using early returns, \
             guard clauses, or extracting nested logic into separate functions."
                .to_string(),
        ),
    }
}

fn create_length_debt(path: &Path, func: &FunctionMetrics) -> DebtItem {
    let priority = if func.length > 100 {
        Priority::High
    } else {
        Priority::Medium
    };

    DebtItem {
        id: format!("js-long-function-{}-{}", path.display(), func.line),
        debt_type: DebtType::CodeSmell {
            smell_type: Some("long_function".to_string()),
        },
        priority,
        file: path.to_path_buf(),
        line: func.line,
        column: None,
        message: format!(
            "Function '{}' is too long ({} lines)",
            func.name, func.length
        ),
        context: Some(
            "Long functions are harder to understand and test. Consider breaking \
             this function into smaller, single-purpose functions."
                .to_string(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn make_function(
        name: &str,
        cyclomatic: u32,
        cognitive: u32,
        nesting: u32,
        length: usize,
    ) -> FunctionMetrics {
        FunctionMetrics {
            name: name.to_string(),
            file: PathBuf::from("test.js"),
            line: 10,
            cyclomatic,
            cognitive,
            nesting,
            length,
            is_test: false,
            visibility: None,
            is_trait_method: false,
            in_test_module: false,
            entropy_score: None,
            is_pure: None,
            purity_confidence: None,
            purity_reason: None,
            call_dependencies: None,
            detected_patterns: None,
            upstream_callers: None,
            downstream_callees: None,
            mapping_pattern_result: None,
            adjusted_complexity: None,
            composition_metrics: None,
            language_specific: None,
            purity_level: None,
            error_swallowing_count: None,
            error_swallowing_patterns: None,
            entropy_analysis: None,
        }
    }

    #[test]
    fn test_detect_high_cyclomatic() {
        let path = PathBuf::from("test.js");
        let functions = vec![make_function("complex", 15, 5, 2, 20)];

        let items = detect_complexity_debt(&path, 10, &functions);

        assert_eq!(items.len(), 1);
        assert!(items[0].message.contains("cyclomatic"));
    }

    #[test]
    fn test_detect_high_cognitive() {
        let path = PathBuf::from("test.js");
        let functions = vec![make_function("cognitive", 5, 15, 2, 20)];

        let items = detect_complexity_debt(&path, 10, &functions);

        assert_eq!(items.len(), 1);
        assert!(items[0].message.contains("cognitive"));
    }

    #[test]
    fn test_detect_deep_nesting() {
        let path = PathBuf::from("test.js");
        let functions = vec![make_function("nested", 5, 5, 6, 20)];

        let items = detect_complexity_debt(&path, 10, &functions);

        assert_eq!(items.len(), 1);
        assert!(items[0].message.contains("nesting"));
    }

    #[test]
    fn test_detect_long_function() {
        let path = PathBuf::from("test.js");
        let functions = vec![make_function("long", 5, 5, 2, 75)];

        let items = detect_complexity_debt(&path, 10, &functions);

        assert_eq!(items.len(), 1);
        assert!(items[0].message.contains("too long"));
    }

    #[test]
    fn test_skip_test_functions() {
        let path = PathBuf::from("test.js");
        let mut func = make_function("test_complex", 15, 15, 6, 75);
        func.is_test = true;

        let items = detect_complexity_debt(&path, 10, &[func]);

        assert!(items.is_empty());
    }
}
