//! Pure scoring and debt prioritization functions.
//!
//! This module provides pure functions for calculating complexity scores
//! and prioritizing debt items without any I/O or progress reporting.
//!
//! # Parallelism (spec 196)
//!
//! The `process_metrics_to_debt_items` function uses rayon's `par_iter()` for
//! parallel processing of function metrics. Shared detectors are created once
//! and passed to all threads via immutable references.

use crate::analysis::ContextDetector;
use crate::core::{DebtItem, FunctionMetrics};
use crate::data_flow::DataFlowGraph;
use crate::priority::call_graph::{CallGraph, FunctionId};
use crate::priority::debt_aggregator::{DebtAggregator, FunctionId as AggregatorFunctionId};
use crate::priority::scoring::{debt_item, ContextRecommendationEngine};
use crate::priority::UnifiedDebtItem;
use crate::risk::lcov::LcovData;
use crate::risk::RiskAnalyzer;
use rayon::prelude::*;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

/// Configuration for scoring weights.
#[derive(Debug, Clone)]
pub struct ScoringWeights {
    pub cyclomatic: f64,
    pub cognitive: f64,
    pub coupling: f64,
    pub coverage: f64,
}

impl Default for ScoringWeights {
    fn default() -> Self {
        Self {
            cyclomatic: 1.0,
            cognitive: 1.5,
            coupling: 0.5,
            coverage: 2.0,
        }
    }
}

/// Configuration for priority calculation.
#[derive(Debug, Clone)]
pub struct PriorityConfig {
    pub threshold: f64,
}

impl Default for PriorityConfig {
    fn default() -> Self {
        Self { threshold: 10.0 }
    }
}

/// Cache of file line counts for efficient lookup (spec 195).
/// Key: file path, Value: physical line count
pub type FileLineCountCache = HashMap<PathBuf, usize>;

/// Build cache of file line counts for all unique files in metrics (spec 195).
///
/// This is an I/O operation that reads each unique file once.
/// Should be called at the I/O boundary before pure debt item construction.
///
/// # Performance
///
/// - Before: O(functions) file reads (e.g., 5000 reads for 5000 functions)
/// - After: O(files) file reads (e.g., 800 reads for 800 unique files)
///
/// # Pure Core Pattern
///
/// This is an I/O operation at the boundary. The returned HashMap
/// enables pure lookups during debt item construction.
pub fn build_file_line_count_cache(metrics: &[FunctionMetrics]) -> FileLineCountCache {
    use crate::metrics::LocCounter;

    let loc_counter = LocCounter::default();

    // Collect unique file paths
    let unique_files: HashSet<&PathBuf> = metrics.iter().map(|m| &m.file).collect();

    // Read each file once
    unique_files
        .into_iter()
        .filter_map(|path| {
            loc_counter
                .count_file(path)
                .ok()
                .map(|count| (path.clone(), count.physical_lines))
        })
        .collect()
}

/// Pure function to create function mappings from metrics.
pub fn create_function_mappings(
    metrics: &[FunctionMetrics],
) -> Vec<(AggregatorFunctionId, usize, usize)> {
    metrics
        .iter()
        .map(|m| {
            let func_id = AggregatorFunctionId::new(m.file.clone(), m.name.clone(), m.line);
            (func_id, m.line, m.line + m.length)
        })
        .collect()
}

/// Pure function to setup debt aggregator.
pub fn setup_debt_aggregator(
    metrics: &[FunctionMetrics],
    debt_items: Option<&[DebtItem]>,
) -> DebtAggregator {
    let mut debt_aggregator = DebtAggregator::new();
    if let Some(items) = debt_items {
        let function_mappings = create_function_mappings(metrics);
        debt_aggregator.aggregate_debt(items.to_vec(), &function_mappings);
    }
    debt_aggregator
}

/// Pure function to transform metrics into purity map.
pub fn metrics_to_purity_map(
    metrics: &[FunctionMetrics],
) -> std::collections::HashMap<String, bool> {
    metrics
        .iter()
        .map(|m| (m.name.clone(), m.is_pure.unwrap_or(false)))
        .collect()
}

/// Create debt items from a metric (pure transformation).
///
/// Returns a Vec of UnifiedDebtItem - one per debt type found for the function.
///
/// # Parallelism (spec 196)
///
/// Accepts shared `context_detector` and `recommendation_engine` references
/// to enable efficient parallel processing without redundant initialization.
#[allow(clippy::too_many_arguments)]
pub fn create_debt_items_from_metric(
    metric: &FunctionMetrics,
    call_graph: &CallGraph,
    coverage_data: Option<&LcovData>,
    framework_exclusions: &HashSet<FunctionId>,
    function_pointer_used_functions: Option<&HashSet<FunctionId>>,
    debt_aggregator: &DebtAggregator,
    data_flow: Option<&DataFlowGraph>,
    risk_analyzer: Option<&RiskAnalyzer>,
    project_path: &Path,
    file_line_counts: &FileLineCountCache,
    context_detector: &ContextDetector,
    recommendation_engine: &ContextRecommendationEngine,
) -> Vec<UnifiedDebtItem> {
    debt_item::create_unified_debt_item_with_aggregator_and_data_flow(
        metric,
        call_graph,
        coverage_data,
        framework_exclusions,
        function_pointer_used_functions,
        debt_aggregator,
        data_flow,
        risk_analyzer,
        project_path,
        file_line_counts,
        context_detector,
        recommendation_engine,
    )
}

/// Process multiple metrics to create debt items in parallel (spec 196).
///
/// # Parallelism
///
/// Uses rayon's `par_iter()` for automatic work-stealing parallelism.
/// Each function is processed independently with no shared mutable state.
///
/// # Shared Resources
///
/// The `context_detector` and `recommendation_engine` are created once
/// and shared across all threads via immutable references. This eliminates
/// the overhead of creating 17 compiled regexes per function.
///
/// # Thread Safety
///
/// All shared references are to `Sync` types:
/// - `ContextDetector`: Compiled regexes (read-only)
/// - `ContextRecommendationEngine`: Static recommendations (read-only)
/// - `HashMap<PathBuf, usize>`: File line counts (read-only)
///
/// # Performance
///
/// - Time complexity: O(n/p) where n = functions, p = cores
/// - Space complexity: O(n) for output, O(1) shared state
/// - Expected speedup: 2-8x on multi-core systems
#[allow(clippy::too_many_arguments)]
pub fn process_metrics_to_debt_items(
    metrics: &[FunctionMetrics],
    call_graph: &CallGraph,
    test_only_functions: &HashSet<FunctionId>,
    coverage_data: Option<&LcovData>,
    framework_exclusions: &HashSet<FunctionId>,
    function_pointer_used_functions: Option<&HashSet<FunctionId>>,
    debt_aggregator: &DebtAggregator,
    data_flow: Option<&DataFlowGraph>,
    risk_analyzer: Option<&RiskAnalyzer>,
    project_path: &Path,
    file_line_counts: &FileLineCountCache,
) -> Vec<UnifiedDebtItem> {
    use super::call_graph::should_process_metric;

    // Pre-create shared detectors once (I/O boundary) - spec 196
    // These are Sync types that can be safely shared across threads
    let context_detector = ContextDetector::new();
    let recommendation_engine = ContextRecommendationEngine::new();

    // Parallel processing with shared references - spec 196
    metrics
        .par_iter() // Parallel iteration with rayon
        .filter(|metric| should_process_metric(metric, call_graph, test_only_functions))
        .flat_map(|metric| {
            create_debt_items_from_metric(
                metric,
                call_graph,
                coverage_data,
                framework_exclusions,
                function_pointer_used_functions,
                debt_aggregator,
                data_flow,
                risk_analyzer,
                project_path,
                file_line_counts,
                &context_detector,
                &recommendation_engine,
            )
        })
        .collect()
}

/// Calculate total complexity from metrics (pure).
pub fn calculate_total_complexity(metrics: &[FunctionMetrics]) -> u32 {
    metrics.iter().map(|m| m.cyclomatic + m.cognitive).sum()
}

/// Calculate average complexity (pure).
pub fn calculate_average_complexity(metrics: &[FunctionMetrics]) -> f64 {
    if metrics.is_empty() {
        return 0.0;
    }
    calculate_total_complexity(metrics) as f64 / metrics.len() as f64
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn create_test_metric(name: &str, cyclomatic: u32, cognitive: u32) -> FunctionMetrics {
        FunctionMetrics {
            name: name.to_string(),
            file: PathBuf::from("test.rs"),
            line: 1,
            length: 10,
            cyclomatic,
            cognitive,
            nesting: 0,
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

    fn create_metric_for_mappings(name: &str, line: usize, length: usize) -> FunctionMetrics {
        let mut metric = create_test_metric(name, 1, 0);
        metric.line = line;
        metric.length = length;
        metric
    }

    fn create_metric_with_purity(name: &str, is_pure: Option<bool>) -> FunctionMetrics {
        let mut metric = create_test_metric(name, 1, 0);
        metric.is_pure = is_pure;
        metric
    }

    #[test]
    fn test_calculate_total_complexity() {
        let metrics = vec![
            create_test_metric("a", 5, 3),
            create_test_metric("b", 10, 7),
        ];

        let total = calculate_total_complexity(&metrics);
        assert_eq!(total, 25); // (5+3) + (10+7)
    }

    #[test]
    fn test_calculate_average_complexity() {
        let metrics = vec![
            create_test_metric("a", 5, 3),
            create_test_metric("b", 10, 7),
        ];

        let avg = calculate_average_complexity(&metrics);
        assert!((avg - 12.5).abs() < 0.001); // 25 / 2
    }

    #[test]
    fn test_calculate_average_complexity_empty() {
        let metrics: Vec<FunctionMetrics> = vec![];
        let avg = calculate_average_complexity(&metrics);
        assert_eq!(avg, 0.0);
    }

    #[test]
    fn test_create_function_mappings() {
        let metrics = vec![
            create_metric_for_mappings("foo", 10, 20),
            create_metric_for_mappings("bar", 50, 30),
        ];

        let mappings = create_function_mappings(&metrics);

        assert_eq!(mappings.len(), 2);
        assert_eq!(mappings[0].1, 10); // start line
        assert_eq!(mappings[0].2, 30); // end line (10 + 20)
        assert_eq!(mappings[1].1, 50);
        assert_eq!(mappings[1].2, 80); // 50 + 30
    }

    #[test]
    fn test_metrics_to_purity_map() {
        let metrics = vec![
            create_metric_with_purity("pure_fn", Some(true)),
            create_metric_with_purity("impure_fn", Some(false)),
            create_metric_with_purity("unknown_fn", None),
        ];

        let map = metrics_to_purity_map(&metrics);

        assert_eq!(map.get("pure_fn"), Some(&true));
        assert_eq!(map.get("impure_fn"), Some(&false));
        assert_eq!(map.get("unknown_fn"), Some(&false)); // None defaults to false
    }
}
