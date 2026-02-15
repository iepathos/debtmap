//! Complexity calculation utilities
//!
//! Aggregates complexity metrics across functions.

use crate::core::FunctionMetrics;

/// Calculate total cyclomatic and cognitive complexity from function metrics
pub fn calculate_total_complexity(functions: &[FunctionMetrics]) -> (u32, u32) {
    functions
        .iter()
        .fold((0u32, 0u32), |(cyc_sum, cog_sum), func| {
            (cyc_sum + func.cyclomatic, cog_sum + func.cognitive)
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn make_function(name: &str, cyclomatic: u32, cognitive: u32) -> FunctionMetrics {
        FunctionMetrics {
            name: name.to_string(),
            file: PathBuf::from("test.js"),
            line: 1,
            cyclomatic,
            cognitive,
            nesting: 0,
            length: 10,
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
    fn test_calculate_total_complexity_empty() {
        let functions: Vec<FunctionMetrics> = vec![];
        let (cyc, cog) = calculate_total_complexity(&functions);
        assert_eq!(cyc, 0);
        assert_eq!(cog, 0);
    }

    #[test]
    fn test_calculate_total_complexity() {
        let functions = vec![
            make_function("foo", 5, 10),
            make_function("bar", 3, 5),
            make_function("baz", 2, 3),
        ];

        let (cyc, cog) = calculate_total_complexity(&functions);
        assert_eq!(cyc, 10);
        assert_eq!(cog, 18);
    }
}
