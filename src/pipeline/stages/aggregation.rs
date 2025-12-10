//! Pure aggregation functions for combining and summarizing data.
//!
//! These functions aggregate metrics and debt items without side effects.

use crate::core::FunctionMetrics;
use std::collections::HashMap;
use std::path::PathBuf;

/// Group functions by file path (pure).
///
/// Organizes function metrics by their containing file for
/// file-level analysis.
///
/// # Arguments
///
/// * `metrics` - Slice of function metrics to group
///
/// # Returns
///
/// HashMap mapping file paths to vectors of functions in that file
pub fn group_by_file(metrics: &[FunctionMetrics]) -> HashMap<PathBuf, Vec<FunctionMetrics>> {
    let mut files_map = HashMap::new();

    for metric in metrics {
        files_map
            .entry(metric.file.clone())
            .or_insert_with(Vec::new)
            .push(metric.clone());
    }

    files_map
}

/// Calculate average complexity across functions (pure).
///
/// Computes mean cyclomatic complexity for a collection of functions.
///
/// # Arguments
///
/// * `metrics` - Slice of function metrics
///
/// # Returns
///
/// Average complexity, or 0.0 if no metrics provided
pub fn average_complexity(metrics: &[FunctionMetrics]) -> f64 {
    if metrics.is_empty() {
        return 0.0;
    }

    let total: u32 = metrics.iter().map(|m| m.cyclomatic).sum();
    total as f64 / metrics.len() as f64
}

/// Calculate average function length (pure).
///
/// Computes mean function length in lines of code.
///
/// # Arguments
///
/// * `metrics` - Slice of function metrics
///
/// # Returns
///
/// Average length, or 0.0 if no metrics provided
pub fn average_length(metrics: &[FunctionMetrics]) -> f64 {
    if metrics.is_empty() {
        return 0.0;
    }

    let total: usize = metrics.iter().map(|m| m.length).sum();
    total as f64 / metrics.len() as f64
}

/// Count functions exceeding complexity threshold (pure).
///
/// Returns count of functions with cyclomatic complexity above threshold.
///
/// # Arguments
///
/// * `metrics` - Slice of function metrics
/// * `threshold` - Complexity threshold
///
/// # Returns
///
/// Number of functions exceeding threshold
pub fn count_high_complexity(metrics: &[FunctionMetrics], threshold: u32) -> usize {
    metrics.iter().filter(|m| m.cyclomatic > threshold).count()
}

/// Count functions exceeding length threshold (pure).
///
/// Returns count of functions longer than threshold.
///
/// # Arguments
///
/// * `metrics` - Slice of function metrics
/// * `threshold` - Length threshold in lines
///
/// # Returns
///
/// Number of functions exceeding threshold
pub fn count_long_functions(metrics: &[FunctionMetrics], threshold: usize) -> usize {
    metrics.iter().filter(|m| m.length > threshold).count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn test_metric(name: &str, file: &str, complexity: u32, length: usize) -> FunctionMetrics {
        FunctionMetrics {
            name: name.to_string(),
            file: PathBuf::from(file),
            line: 1,
            cyclomatic: complexity,
            cognitive: 1,
            nesting: 1,
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
        }
    }

    #[test]
    fn test_group_by_file() {
        let metrics = vec![
            test_metric("foo", "main.rs", 5, 10),
            test_metric("bar", "main.rs", 3, 8),
            test_metric("baz", "lib.rs", 7, 15),
        ];

        let grouped = group_by_file(&metrics);

        assert_eq!(grouped.len(), 2);
        assert_eq!(grouped[&PathBuf::from("main.rs")].len(), 2);
        assert_eq!(grouped[&PathBuf::from("lib.rs")].len(), 1);
    }

    #[test]
    fn test_group_by_file_empty() {
        let metrics = vec![];
        let grouped = group_by_file(&metrics);
        assert!(grouped.is_empty());
    }

    #[test]
    fn test_average_complexity() {
        let metrics = vec![
            test_metric("foo", "main.rs", 2, 10),
            test_metric("bar", "main.rs", 4, 10),
            test_metric("baz", "main.rs", 6, 10),
        ];

        let avg = average_complexity(&metrics);
        assert_eq!(avg, 4.0);
    }

    #[test]
    fn test_average_complexity_empty() {
        let avg = average_complexity(&[]);
        assert_eq!(avg, 0.0);
    }

    #[test]
    fn test_average_length() {
        let metrics = vec![
            test_metric("foo", "main.rs", 5, 10),
            test_metric("bar", "main.rs", 5, 20),
            test_metric("baz", "main.rs", 5, 30),
        ];

        let avg = average_length(&metrics);
        assert_eq!(avg, 20.0);
    }

    #[test]
    fn test_count_high_complexity() {
        let metrics = vec![
            test_metric("foo", "main.rs", 5, 10),
            test_metric("bar", "main.rs", 15, 10),
            test_metric("baz", "main.rs", 25, 10),
        ];

        let count = count_high_complexity(&metrics, 10);
        assert_eq!(count, 2); // bar and baz exceed 10
    }

    #[test]
    fn test_count_long_functions() {
        let metrics = vec![
            test_metric("foo", "main.rs", 5, 10),
            test_metric("bar", "main.rs", 5, 50),
            test_metric("baz", "main.rs", 5, 100),
        ];

        let count = count_long_functions(&metrics, 20);
        assert_eq!(count, 2); // bar and baz exceed 20
    }
}
