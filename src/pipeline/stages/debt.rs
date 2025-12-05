//! Pure functions for technical debt detection.
//!
//! These functions detect various types of technical debt based on
//! function metrics without performing I/O.

use crate::core::FunctionMetrics;

/// Thresholds for debt detection.
#[derive(Debug, Clone)]
pub struct Thresholds {
    pub complexity: u32,
    pub nesting: u32,
    pub length: usize,
    pub parameters: usize,
}

impl Default for Thresholds {
    fn default() -> Self {
        Self {
            complexity: 10,
            nesting: 4,
            length: 50,
            parameters: 5,
        }
    }
}

/// Simplified debt item for pure functions.
#[derive(Debug, Clone, PartialEq)]
pub struct DebtItem {
    pub function_name: String,
    pub debt_type: DebtItemType,
    pub severity: f64,
}

/// Types of debt items.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DebtItemType {
    HighComplexity,
    DeepNesting,
    LongFunction,
    HighParameterCount,
}

/// Detect high complexity (pure).
pub fn detect_complexity_debt(
    metric: &FunctionMetrics,
    thresholds: &Thresholds,
) -> Option<DebtItem> {
    if metric.cyclomatic > thresholds.complexity {
        let excess = metric.cyclomatic - thresholds.complexity;
        Some(DebtItem {
            function_name: metric.name.clone(),
            debt_type: DebtItemType::HighComplexity,
            severity: 50.0 + (excess as f64 * 5.0),
        })
    } else {
        None
    }
}

/// Detect deep nesting (pure).
pub fn detect_nesting_debt(metric: &FunctionMetrics, thresholds: &Thresholds) -> Option<DebtItem> {
    if metric.nesting > thresholds.nesting {
        let excess = metric.nesting - thresholds.nesting;
        Some(DebtItem {
            function_name: metric.name.clone(),
            debt_type: DebtItemType::DeepNesting,
            severity: 40.0 + (excess as f64 * 10.0),
        })
    } else {
        None
    }
}

/// Detect long functions (pure).
pub fn detect_length_debt(metric: &FunctionMetrics, thresholds: &Thresholds) -> Option<DebtItem> {
    if metric.length > thresholds.length {
        let excess = metric.length - thresholds.length;
        Some(DebtItem {
            function_name: metric.name.clone(),
            debt_type: DebtItemType::LongFunction,
            severity: 30.0 + (excess as f64 * 0.5),
        })
    } else {
        None
    }
}

/// Detect all debt in a function (pure).
pub fn detect_all_debt(metric: &FunctionMetrics, thresholds: &Thresholds) -> Vec<DebtItem> {
    [
        detect_complexity_debt(metric, thresholds),
        detect_nesting_debt(metric, thresholds),
        detect_length_debt(metric, thresholds),
    ]
    .into_iter()
    .flatten()
    .collect()
}

/// Detect debt from pipeline data (adapter for pipeline integration).
///
/// TODO: Full integration with UnifiedDebtItem structure.
/// For now returns empty vector to allow compilation.
pub fn detect_debt_from_pipeline(
    _metrics: &[FunctionMetrics],
    _call_graph: Option<&crate::priority::call_graph::CallGraph>,
) -> Vec<crate::priority::UnifiedDebtItem> {
    // TODO: Implement full debt detection using existing unified analysis code
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn test_metric(name: &str, complexity: u32, nesting: u32, length: usize) -> FunctionMetrics {
        FunctionMetrics {
            name: name.to_string(),
            file: PathBuf::from("test.rs"),
            line: 1,
            cyclomatic: complexity,
            cognitive: 1,
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
        }
    }

    #[test]
    fn test_detect_complexity_debt_none() {
        let metric = test_metric("foo", 5, 2, 20);
        let thresholds = Thresholds::default();
        let debt = detect_complexity_debt(&metric, &thresholds);
        assert!(debt.is_none());
    }

    #[test]
    fn test_detect_complexity_debt_found() {
        let metric = test_metric("foo", 15, 2, 20);
        let thresholds = Thresholds::default();
        let debt = detect_complexity_debt(&metric, &thresholds);
        assert!(debt.is_some());
        let item = debt.unwrap();
        assert_eq!(item.debt_type, DebtItemType::HighComplexity);
        assert!(item.severity > 50.0);
    }

    #[test]
    fn test_detect_all_debt_multiple() {
        let metric = test_metric("complex_long", 15, 5, 100);
        let thresholds = Thresholds::default();
        let debt = detect_all_debt(&metric, &thresholds);
        assert_eq!(debt.len(), 3); // All three types
    }

    #[test]
    fn test_detect_all_debt_none() {
        let metric = test_metric("simple", 3, 2, 10);
        let thresholds = Thresholds::default();
        let debt = detect_all_debt(&metric, &thresholds);
        assert!(debt.is_empty());
    }
}
