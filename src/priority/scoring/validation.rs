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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn create_test_function(cyclomatic: u32, cognitive: u32, is_test: bool) -> FunctionMetrics {
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
            mapping_pattern_result: None,
            adjusted_complexity: None,
            composition_metrics: None,
            language_specific: None,
        }
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
}
