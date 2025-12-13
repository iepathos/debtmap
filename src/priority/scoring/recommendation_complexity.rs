//! Complexity-based recommendation generation facade
//!
//! This module provides the unified API for generating refactoring recommendations
//! based on function complexity analysis. It composes smaller, focused modules
//! following the Stillwater philosophy of pure core with clear boundaries.
//!
//! ## Module Structure
//!
//! - `complexity_classification`: Complexity level classification
//! - `dead_code_hints`: Dead code analysis and hints
//! - `complexity_generators`: Level-based recommendation generators
//! - `pattern_generators`: Pattern-based recommendation generators
//! - `heuristic_generators`: Heuristic-based recommendation generators
//!
//! ## Usage
//!
//! ```rust,ignore
//! use debtmap::priority::scoring::recommendation_complexity::{
//!     classify_complexity_level,
//!     generate_complexity_recommendation_with_patterns_and_coverage,
//! };
//!
//! let level = classify_complexity_level(15);
//! let (action, rationale, steps) = generate_complexity_recommendation_with_patterns_and_coverage(
//!     &func, 15, 25, &coverage, data_flow.as_ref()
//! );
//! ```

// Re-export from complexity_classification
pub use super::complexity_classification::{classify_complexity_level, ComplexityLevel};

// Re-export from dead_code_hints
pub use super::dead_code_hints::{
    determine_visibility, generate_enhanced_dead_code_hints, generate_usage_hints,
};

// Re-export from complexity_generators
pub use super::complexity_generators::{
    generate_complexity_hotspot_recommendation, generate_complexity_risk_recommendation,
    generate_high_complexity_recommendation, generate_infrastructure_recommendation_with_coverage,
    generate_low_complexity_recommendation, generate_low_moderate_complexity_recommendation,
    generate_moderate_complexity_recommendation, RecommendationOutput,
};

// Re-export from pattern_generators
pub use super::pattern_generators::{
    analyze_extraction_patterns, calculate_reduction_percentage, detect_file_language,
    generate_coverage_steps, generate_pattern_based_recommendation, has_good_coverage,
    pattern_type_name, should_prioritize_coverage,
};

// Re-export from heuristic_generators
pub use super::heuristic_generators::{
    analyze_function_characteristics, generate_coverage_focused_recommendation,
    generate_heuristic_recommendations_with_line_estimates, FunctionCharacteristics,
};

use crate::core::FunctionMetrics;
use crate::priority::TransitiveCoverage;

/// Generate comprehensive complexity recommendation using patterns and coverage
///
/// This is the main entry point for generating refactoring recommendations.
/// It intelligently chooses between:
/// 1. Coverage-focused recommendations (when coverage is the primary issue)
/// 2. Pattern-based recommendations (when extractable patterns are detected)
/// 3. Heuristic recommendations (fallback when patterns aren't available)
///
/// # Arguments
///
/// * `func` - Function metrics for the function being analyzed
/// * `cyclomatic` - Cyclomatic complexity score
/// * `cognitive` - Cognitive complexity score
/// * `coverage` - Optional transitive coverage information
/// * `data_flow` - Optional data flow graph for additional analysis
///
/// # Returns
///
/// Tuple of (action, rationale, steps) describing the recommended refactoring
pub fn generate_complexity_recommendation_with_patterns_and_coverage(
    func: &FunctionMetrics,
    cyclomatic: u32,
    cognitive: u32,
    coverage: &Option<TransitiveCoverage>,
    data_flow: Option<&crate::data_flow::DataFlowGraph>,
) -> RecommendationOutput {
    // Priority 1: Coverage-focused when coverage is the main issue
    if should_prioritize_coverage(coverage) {
        return generate_coverage_focused_recommendation(
            func,
            cyclomatic,
            cognitive,
            coverage.as_ref().unwrap(),
        );
    }

    // Priority 2: Pattern-based when patterns are detected
    let suggestions = analyze_extraction_patterns(func, data_flow);
    if !suggestions.is_empty() {
        return generate_pattern_based_recommendation(func, cyclomatic, &suggestions, coverage);
    }

    // Priority 3: Heuristic-based as fallback
    generate_heuristic_recommendations_with_line_estimates(
        func, cyclomatic, cognitive, coverage, data_flow,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn create_test_function(name: &str, visibility: Option<&str>) -> FunctionMetrics {
        FunctionMetrics {
            name: name.to_string(),
            file: PathBuf::from("test.rs"),
            line: 10,
            cyclomatic: 5,
            cognitive: 8,
            nesting: 2,
            length: 50,
            is_test: false,
            visibility: visibility.map(|v| v.to_string()),
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
    fn test_classify_complexity_level() {
        assert!(matches!(classify_complexity_level(1), ComplexityLevel::Low));
        assert!(matches!(classify_complexity_level(4), ComplexityLevel::Low));
        assert!(matches!(
            classify_complexity_level(5),
            ComplexityLevel::LowModerate
        ));
        assert!(matches!(
            classify_complexity_level(6),
            ComplexityLevel::LowModerate
        ));
        assert!(matches!(
            classify_complexity_level(7),
            ComplexityLevel::Moderate
        ));
        assert!(matches!(
            classify_complexity_level(10),
            ComplexityLevel::Moderate
        ));
        assert!(matches!(
            classify_complexity_level(11),
            ComplexityLevel::High
        ));
        assert!(matches!(
            classify_complexity_level(20),
            ComplexityLevel::High
        ));
    }

    #[test]
    fn test_generate_complexity_recommendation_returns_tuple() {
        let func = create_test_function("test_func", None);
        let (action, rationale, steps) =
            generate_complexity_recommendation_with_patterns_and_coverage(
                &func, 10, 15, &None, None,
            );

        assert!(!action.is_empty());
        assert!(!rationale.is_empty());
        assert!(!steps.is_empty());
    }

    #[test]
    fn test_coverage_prioritization() {
        let func = create_test_function("test_func", None);
        let low_coverage = Some(TransitiveCoverage {
            direct: 0.3,
            transitive: 0.2,
            propagated_from: vec![],
            uncovered_lines: vec![10, 20, 30, 40, 50],
        });

        let (action, rationale, _) = generate_complexity_recommendation_with_patterns_and_coverage(
            &func,
            10,
            15,
            &low_coverage,
            None,
        );

        // Should focus on coverage when coverage is low
        assert!(action.contains("coverage") || action.contains("tests"));
        assert!(rationale.contains("coverage") || rationale.contains("uncovered"));
    }

    #[test]
    fn test_module_reexports_work() {
        // Test that re-exports from sub-modules work correctly
        use crate::priority::FunctionVisibility;

        let func = create_test_function("my_func", Some("pub"));
        let visibility = determine_visibility(&func);
        assert!(matches!(visibility, FunctionVisibility::Public));

        let hints = generate_enhanced_dead_code_hints(&func, &visibility);
        assert!(!hints.is_empty());
    }

    #[test]
    fn test_complexity_generators_reexport() {
        let (action, rationale, steps) = generate_low_complexity_recommendation(3, false);
        assert!(!action.is_empty());
        assert!(rationale.contains("Low complexity"));
        assert!(!steps.is_empty());
    }

    #[test]
    fn test_pattern_generators_reexport() {
        use std::path::Path;

        assert_eq!(
            detect_file_language(Path::new("test.rs")),
            crate::core::Language::Rust
        );
        assert_eq!(
            detect_file_language(Path::new("script.py")),
            crate::core::Language::Python
        );
    }

    #[test]
    fn test_heuristic_generators_reexport() {
        let func = create_test_function("test_func", None);
        let chars = analyze_function_characteristics(&func, 10, 25, None);

        assert!(chars.has_high_branching);
        assert!(chars.has_complex_cognition);
    }
}
