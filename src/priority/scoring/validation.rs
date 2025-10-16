// Validation functions for debt type detection
//
// This module contains pure functions for classifying different types of technical debt
// based on function metrics, coverage data, and call graph analysis.

use crate::core::FunctionMetrics;
use crate::priority::{
    call_graph::{CallGraph, FunctionId},
    scoring::recommendation_extended::generate_usage_hints,
    DebtType, TransitiveCoverage,
};
use std::collections::HashSet;

// Re-export for backward compatibility
pub use super::debt_item::{determine_visibility, is_dead_code, is_dead_code_with_exclusions};

/// Pure function to check for testing gaps
pub(super) fn check_testing_gap(
    func: &FunctionMetrics,
    coverage: &Option<TransitiveCoverage>,
) -> Option<DebtType> {
    coverage
        .as_ref()
        .filter(|cov| cov.direct < 0.2 && !func.is_test)
        .map(|cov| DebtType::TestingGap {
            coverage: cov.direct,
            cyclomatic: func.cyclomatic,
            cognitive: func.cognitive,
        })
}

/// Pure function to check for complexity hotspots
pub(super) fn check_complexity_hotspot(func: &FunctionMetrics) -> Option<DebtType> {
    if func.cyclomatic > 10 || func.cognitive > 15 {
        Some(DebtType::ComplexityHotspot {
            cyclomatic: func.cyclomatic,
            cognitive: func.cognitive,
        })
    } else {
        None
    }
}

/// Pure function to check for dead code
pub(super) fn check_dead_code(
    func: &FunctionMetrics,
    call_graph: &CallGraph,
    func_id: &FunctionId,
) -> Option<DebtType> {
    if is_dead_code(func, call_graph, func_id, None) {
        Some(DebtType::DeadCode {
            visibility: determine_visibility(func),
            cyclomatic: func.cyclomatic,
            cognitive: func.cognitive,
            usage_hints: generate_usage_hints(func, call_graph, func_id),
        })
    } else {
        None
    }
}

// Pure function to check if function has testing gap
pub(super) fn has_testing_gap(coverage: f64, is_test: bool) -> bool {
    coverage < 0.2 && !is_test
}

// Pure function to check if function is complexity hotspot based on metrics only
pub(super) fn is_complexity_hotspot_by_metrics(cyclomatic: u32, cognitive: u32) -> bool {
    cyclomatic > 5 || cognitive > 8
}

/// Context structure for enhanced debt classification
pub(super) struct ClassificationContext<'a> {
    pub func: &'a FunctionMetrics,
    pub call_graph: &'a CallGraph,
    pub func_id: &'a FunctionId,
    pub framework_exclusions: &'a HashSet<FunctionId>,
    pub function_pointer_used_functions: Option<&'a HashSet<FunctionId>>,
    pub coverage: Option<&'a TransitiveCoverage>,
}

/// Pure function to check for enhanced testing gaps
pub(super) fn check_enhanced_testing_gap(context: &ClassificationContext) -> Option<DebtType> {
    context.coverage.and_then(|cov| {
        let has_gap = has_testing_gap(cov.direct, context.func.is_test)
            || (cov.direct < 0.8 && context.func.cyclomatic > 5 && !cov.uncovered_lines.is_empty());

        if has_gap {
            Some(DebtType::TestingGap {
                coverage: cov.direct,
                cyclomatic: context.func.cyclomatic,
                cognitive: context.func.cognitive,
            })
        } else {
            None
        }
    })
}

/// Pure function to check for enhanced complexity hotspots
pub(super) fn check_enhanced_complexity_hotspot(func: &FunctionMetrics) -> Option<DebtType> {
    if is_complexity_hotspot_by_metrics(func.cyclomatic, func.cognitive) {
        Some(DebtType::ComplexityHotspot {
            cyclomatic: func.cyclomatic,
            cognitive: func.cognitive,
        })
    } else {
        None
    }
}

/// Pure function to check for enhanced dead code
pub(super) fn check_enhanced_dead_code(context: &ClassificationContext) -> Option<DebtType> {
    if is_dead_code_with_exclusions(
        context.func,
        context.call_graph,
        context.func_id,
        context.framework_exclusions,
        context.function_pointer_used_functions,
    ) {
        Some(DebtType::DeadCode {
            visibility: determine_visibility(context.func),
            cyclomatic: context.func.cyclomatic,
            cognitive: context.func.cognitive,
            usage_hints: generate_usage_hints(context.func, context.call_graph, context.func_id),
        })
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn create_test_function(
        cyclomatic: u32,
        cognitive: u32,
        is_test: bool,
    ) -> FunctionMetrics {
        FunctionMetrics {
            name: "test_func".to_string(),
            file: PathBuf::from("test.rs"),
            line: 1,
            length: 20,
            cyclomatic,
            cognitive,
            nesting: 1,
            visibility: Some("pub".to_string()),
            is_test,
            is_trait_method: false,
            in_test_module: false,
            entropy_score: None,
            is_pure: Some(false),
            purity_confidence: Some(0.5),
            detected_patterns: None,
            upstream_callers: None,
            downstream_callees: None,
        }
    }

    #[test]
    fn test_has_testing_gap() {
        // Coverage below 20% and not a test = testing gap
        assert!(has_testing_gap(0.1, false));
        assert!(has_testing_gap(0.19, false));

        // Coverage at or above 20% = no testing gap
        assert!(!has_testing_gap(0.2, false));
        assert!(!has_testing_gap(0.5, false));

        // Test functions never have testing gaps
        assert!(!has_testing_gap(0.0, true));
        assert!(!has_testing_gap(0.1, true));
    }

    #[test]
    fn test_is_complexity_hotspot_by_metrics() {
        // High cyclomatic complexity
        assert!(is_complexity_hotspot_by_metrics(6, 3));
        assert!(is_complexity_hotspot_by_metrics(10, 5));

        // High cognitive complexity
        assert!(is_complexity_hotspot_by_metrics(3, 9));
        assert!(is_complexity_hotspot_by_metrics(4, 15));

        // Both low = not a hotspot
        assert!(!is_complexity_hotspot_by_metrics(5, 8));
        assert!(!is_complexity_hotspot_by_metrics(3, 5));
        assert!(!is_complexity_hotspot_by_metrics(0, 0));
    }

    #[test]
    fn test_check_testing_gap() {
        let func = create_test_function(5, 8, false);

        // Low coverage should trigger testing gap
        let coverage_low = Some(TransitiveCoverage {
            direct: 0.1,
            transitive: 0.1,
            propagated_from: vec![],
            uncovered_lines: vec![],
        });
        assert!(check_testing_gap(&func, &coverage_low).is_some());

        // High coverage should not trigger testing gap
        let coverage_high = Some(TransitiveCoverage {
            direct: 0.9,
            transitive: 0.9,
            propagated_from: vec![],
            uncovered_lines: vec![],
        });
        assert!(check_testing_gap(&func, &coverage_high).is_none());
    }

    #[test]
    fn test_check_complexity_hotspot() {
        // High cyclomatic complexity
        let complex_func = create_test_function(15, 8, false);
        assert!(check_complexity_hotspot(&complex_func).is_some());

        // High cognitive complexity
        let complex_func2 = create_test_function(8, 20, false);
        assert!(check_complexity_hotspot(&complex_func2).is_some());

        // Low complexity
        let simple_func = create_test_function(5, 8, false);
        assert!(check_complexity_hotspot(&simple_func).is_none());
    }

    #[test]
    fn test_check_enhanced_complexity_hotspot() {
        let complex_func = create_test_function(10, 12, false);
        let debt = check_enhanced_complexity_hotspot(&complex_func);
        assert!(debt.is_some());

        match debt.unwrap() {
            DebtType::ComplexityHotspot {
                cyclomatic,
                cognitive,
            } => {
                assert_eq!(cyclomatic, 10);
                assert_eq!(cognitive, 12);
            }
            _ => panic!("Expected ComplexityHotspot debt type"),
        }
    }

    #[test]
    fn test_check_enhanced_complexity_hotspot_simple() {
        let simple_func = create_test_function(2, 3, false);
        let debt = check_enhanced_complexity_hotspot(&simple_func);
        assert!(debt.is_none());
    }
}
