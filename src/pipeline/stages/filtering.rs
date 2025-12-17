//! Pure filtering functions for analysis pipeline.
//!
//! These functions filter data based on various criteria without
//! performing I/O or side effects.

use crate::core::FunctionMetrics;
use crate::priority::call_graph::FunctionId;
use std::collections::HashSet;

/// Filter out test functions from metrics (pure).
///
/// Removes functions that are tests or only reachable from tests,
/// focusing analysis on production code.
///
/// # Arguments
///
/// * `metrics` - Vector of function metrics
/// * `test_only` - Set of function IDs that are test-only
///
/// # Returns
///
/// Filtered vector containing only non-test functions
pub fn filter_test_functions(
    metrics: Vec<FunctionMetrics>,
    test_only: &HashSet<FunctionId>,
) -> Vec<FunctionMetrics> {
    metrics
        .into_iter()
        .filter(|m| {
            let func_id = FunctionId::new(m.file.clone(), m.name.clone(), m.line);
            !m.is_test && !test_only.contains(&func_id)
        })
        .collect()
}

/// Filter functions by minimum complexity threshold (pure).
///
/// Keeps only functions that exceed a complexity threshold,
/// focusing on high-complexity code.
///
/// # Arguments
///
/// * `metrics` - Vector of function metrics
/// * `min_complexity` - Minimum cyclomatic complexity threshold
///
/// # Returns
///
/// Filtered vector containing only high-complexity functions
pub fn filter_by_complexity(
    metrics: Vec<FunctionMetrics>,
    min_complexity: u32,
) -> Vec<FunctionMetrics> {
    metrics
        .into_iter()
        .filter(|m| m.cyclomatic >= min_complexity)
        .collect()
}

/// Filter functions by minimum length threshold (pure).
///
/// Keeps only functions that exceed a length threshold,
/// focusing on long functions.
///
/// # Arguments
///
/// * `metrics` - Vector of function metrics
/// * `min_length` - Minimum function length in lines
///
/// # Returns
///
/// Filtered vector containing only long functions
pub fn filter_by_length(metrics: Vec<FunctionMetrics>, min_length: usize) -> Vec<FunctionMetrics> {
    metrics
        .into_iter()
        .filter(|m| m.length >= min_length)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn test_metric(name: &str, is_test: bool, complexity: u32, length: usize) -> FunctionMetrics {
        FunctionMetrics {
            name: name.to_string(),
            file: PathBuf::from("test.rs"),
            line: 1,
            cyclomatic: complexity,
            cognitive: 1,
            nesting: 1,
            length,
            is_test,
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
    fn test_filter_test_functions_removes_tests() {
        let metrics = vec![
            test_metric("foo", false, 5, 10),
            test_metric("test_bar", true, 3, 8),
            test_metric("baz", false, 7, 15),
        ];

        let test_only = HashSet::new();
        let filtered = filter_test_functions(metrics, &test_only);

        assert_eq!(filtered.len(), 2);
        assert_eq!(filtered[0].name, "foo");
        assert_eq!(filtered[1].name, "baz");
    }

    #[test]
    fn test_filter_test_functions_empty() {
        let metrics = vec![];
        let test_only = HashSet::new();
        let filtered = filter_test_functions(metrics, &test_only);
        assert!(filtered.is_empty());
    }

    #[test]
    fn test_filter_by_complexity() {
        let metrics = vec![
            test_metric("low", false, 2, 10),
            test_metric("medium", false, 5, 10),
            test_metric("high", false, 15, 10),
        ];

        let filtered = filter_by_complexity(metrics, 5);

        assert_eq!(filtered.len(), 2);
        assert_eq!(filtered[0].name, "medium");
        assert_eq!(filtered[1].name, "high");
    }

    #[test]
    fn test_filter_by_length() {
        let metrics = vec![
            test_metric("short", false, 5, 5),
            test_metric("medium", false, 5, 20),
            test_metric("long", false, 5, 50),
        ];

        let filtered = filter_by_length(metrics, 20);

        assert_eq!(filtered.len(), 2);
        assert_eq!(filtered[0].name, "medium");
        assert_eq!(filtered[1].name, "long");
    }
}
