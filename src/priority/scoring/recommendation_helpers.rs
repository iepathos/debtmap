// Pure helper functions for generating recommendations

use crate::core::FunctionMetrics;
use crate::priority::{
    ActionableRecommendation, DebtType, FunctionRole, FunctionVisibility, TransitiveCoverage,
    UnifiedScore,
};

// Re-use functions from other modules
use super::computation::{
    calculate_functions_to_extract, calculate_needed_test_cases, calculate_simple_test_cases,
};
use super::formatting::{
    add_uncovered_lines_to_steps, format_complexity_display, format_role_description,
    generate_combined_testing_refactoring_steps, generate_dead_code_steps,
    generate_testing_gap_steps, get_role_display_name,
};
use super::recommendation_extended::generate_usage_hints;
use crate::priority::external_api_detector::is_likely_external_api;

/// Generate action and rationale for dead code
pub(super) fn generate_dead_code_action(
    func: &FunctionMetrics,
    visibility: &FunctionVisibility,
    func_name: &str,
    cyclomatic: &u32,
    cognitive: &u32,
) -> (String, String) {
    let complexity_str = format_complexity_display(cyclomatic, cognitive);

    match visibility {
        FunctionVisibility::Private => (
            "Remove unused private function".to_string(),
            format!("Private function '{func_name}' has no callers and can be safely removed (complexity: {complexity_str})"),
        ),
        FunctionVisibility::Crate => (
            "Remove or document unused crate function".to_string(),
            format!("Crate-public function '{func_name}' has no internal callers (complexity: {complexity_str})"),
        ),
        FunctionVisibility::Public => {
            let (is_likely_api, _) = is_likely_external_api(func, visibility);
            if is_likely_api {
                (
                    "Verify external usage before removal or deprecation".to_string(),
                    format!("Public function '{func_name}' appears to be external API - verify usage before action (complexity: {complexity_str})"),
                )
            } else {
                (
                    "Remove unused public function (no API indicators)".to_string(),
                    format!("Public function '{func_name}' has no callers and no external API indicators (complexity: {complexity_str})"),
                )
            }
        }
    }
}

/// Generate recommendation when function is fully covered
pub(super) fn generate_full_coverage_recommendation(role: FunctionRole) -> (String, String, Vec<String>) {
    let role_display = get_role_display_name(role);
    (
        "Maintain test coverage".to_string(),
        format!("{} function is currently 100% covered", role_display),
        vec![
            "Keep tests up to date with code changes".to_string(),
            "Consider property-based testing for edge cases".to_string(),
            "Monitor coverage in CI/CD pipeline".to_string(),
        ],
    )
}

/// Generate recommendation for complex functions with testing gaps
pub(super) fn generate_complex_function_recommendation(
    cyclomatic: u32,
    cognitive: u32,
    coverage_pct: f64,
    coverage_gap: i32,
    role_str: &str,
    func: &FunctionMetrics,
    transitive_coverage: &Option<TransitiveCoverage>,
) -> (String, String, Vec<String>) {
    let functions_to_extract = calculate_functions_to_extract(cyclomatic, cognitive);
    let needed_test_cases = calculate_needed_test_cases(cyclomatic, coverage_pct);
    let coverage_pct_int = (coverage_pct * 100.0) as i32;

    let complexity_explanation = format!(
        "Cyclomatic complexity of {} requires at least {} test cases for full path coverage. After extracting {} functions, each will need only 3-5 tests",
        cyclomatic, cyclomatic, functions_to_extract
    );

    let mut steps =
        generate_combined_testing_refactoring_steps(cyclomatic, cognitive, coverage_pct_int);
    add_uncovered_lines_to_steps(&mut steps, func, transitive_coverage);

    (
        format!("Add {} tests for {}% coverage gap, then refactor complexity {} into {} functions",
               needed_test_cases, coverage_gap, cyclomatic, functions_to_extract),
        format!("Complex {role_str} with {coverage_gap}% gap. {}. Testing before refactoring ensures no regressions",
               complexity_explanation),
        steps,
    )
}

/// Generate recommendation for simple functions with testing gaps
pub(super) fn generate_simple_function_recommendation(
    cyclomatic: u32,
    coverage_pct: f64,
    coverage_gap: i32,
    role: FunctionRole,
    func: &FunctionMetrics,
    transitive_coverage: &Option<TransitiveCoverage>,
) -> (String, String, Vec<String>) {
    let role_display = get_role_display_name(role);
    let test_cases_needed = calculate_simple_test_cases(cyclomatic, coverage_pct);
    let coverage_pct_int = (coverage_pct * 100.0) as i32;

    let coverage_explanation = if coverage_pct_int == 0 {
        format!("{role_display} with {coverage_gap}% coverage gap, currently {coverage_pct_int}% covered. Needs {} test cases to cover all {} execution paths",
               test_cases_needed, cyclomatic.max(2))
    } else {
        format!("{role_display} with {coverage_gap}% coverage gap, currently {coverage_pct_int}% covered. Needs {} more test cases",
               test_cases_needed)
    };

    let mut steps = generate_testing_gap_steps(false);
    add_uncovered_lines_to_steps(&mut steps, func, transitive_coverage);

    (
        format!(
            "Add {} tests for {}% coverage gap",
            test_cases_needed, coverage_gap
        ),
        coverage_explanation,
        steps,
    )
}

/// Generate recommendation for testing gap debt type
pub(super) fn generate_testing_gap_recommendation(
    coverage_pct: f64,
    cyclomatic: u32,
    cognitive: u32,
    role: FunctionRole,
    func: &FunctionMetrics,
    transitive_coverage: &Option<TransitiveCoverage>,
) -> (String, String, Vec<String>) {
    let coverage_gap = 100 - (coverage_pct * 100.0) as i32;

    // If function is fully covered, no testing gap exists
    if coverage_gap == 0 {
        return generate_full_coverage_recommendation(role);
    }

    let is_complex = cyclomatic > 10 || cognitive > 15;

    if is_complex {
        let role_str = format_role_description(role);
        generate_complex_function_recommendation(
            cyclomatic,
            cognitive,
            coverage_pct,
            coverage_gap,
            role_str,
            func,
            transitive_coverage,
        )
    } else {
        generate_simple_function_recommendation(
            cyclomatic,
            coverage_pct,
            coverage_gap,
            role,
            func,
            transitive_coverage,
        )
    }
}

/// Generate recommendation for dead code debt type
pub(super) fn generate_dead_code_recommendation(
    func: &FunctionMetrics,
    visibility: &FunctionVisibility,
    usage_hints: &[String],
    cyclomatic: u32,
    cognitive: u32,
) -> (String, String, Vec<String>) {
    let (action, rationale) =
        generate_dead_code_action(func, visibility, &func.name, &cyclomatic, &cognitive);
    let mut steps = generate_dead_code_steps(visibility);

    // Add usage hints to the steps
    for hint in usage_hints {
        steps.push(format!("Note: {hint}"));
    }

    (action, rationale, steps)
}

/// Generate recommendation for error swallowing debt
pub(super) fn generate_error_swallowing_recommendation(
    pattern: &str,
    context: &Option<String>,
) -> (String, String, Vec<String>) {
    let primary_action = format!("Fix error swallowing: {}", pattern);

    let rationale = match context {
        Some(ctx) => format!("Error being silently ignored using '{}' pattern. Context: {}", pattern, ctx),
        None => format!("Error being silently ignored using '{}' pattern. This can hide critical failures in production", pattern),
    };

    let steps = vec![
        "Replace error swallowing with proper error handling".to_string(),
        "Log errors at minimum, even if they can't be handled".to_string(),
        "Consider propagating errors to caller with ?".to_string(),
        "Add context to errors using .context() or .with_context()".to_string(),
        "Test error paths explicitly".to_string(),
    ];

    (primary_action, rationale, steps)
}

/// Generate recommendation for test-specific debt types
pub(super) fn generate_test_debt_recommendation(debt_type: &DebtType) -> (String, String, Vec<String>) {
    match debt_type {
        DebtType::TestComplexityHotspot {
            cyclomatic,
            cognitive,
            threshold
        } => (
            format!("Simplify test - complexity {} exceeds test threshold {}", cyclomatic.max(cognitive), threshold),
            format!("Test has high complexity (cyclo={cyclomatic}, cognitive={cognitive}) - consider splitting into smaller tests"),
            vec![
                "Break complex test into multiple smaller tests".to_string(),
                "Extract test setup into helper functions".to_string(),
                "Use parameterized tests for similar test cases".to_string(),
            ],
        ),
        DebtType::TestTodo { priority: _, reason } => (
            "Complete test TODO".to_string(),
            format!("Test contains TODO: {}", reason.as_ref().unwrap_or(&"No reason specified".to_string())),
            vec![
                "Address the TODO comment".to_string(),
                "Implement missing test logic".to_string(),
                "Remove TODO once completed".to_string(),
            ],
        ),
        DebtType::TestDuplication { instances, total_lines, similarity: _ } => (
            format!("Remove test duplication - {instances} similar test blocks"),
            format!("{instances} duplicated test blocks found across {total_lines} lines"),
            vec![
                "Extract common test logic into helper functions".to_string(),
                "Create parameterized tests for similar test cases".to_string(),
                "Use test fixtures for shared setup".to_string(),
            ],
        ),
        _ => unreachable!("Not a test debt type"),
    }
}

/// Pure function to build final actionable recommendation
pub(super) fn build_actionable_recommendation(
    primary_action: String,
    rationale: String,
    steps: Vec<String>,
) -> ActionableRecommendation {
    ActionableRecommendation {
        primary_action,
        rationale,
        implementation_steps: steps,
        related_items: vec![],
    }
}

/// Pure function to extract coverage percentage
pub(super) fn extract_coverage_percent(coverage: &Option<TransitiveCoverage>) -> f64 {
    coverage.as_ref().map(|c| c.direct).unwrap_or(0.0)
}

/// Pure function to extract cyclomatic complexity from debt type
pub(super) fn extract_cyclomatic_complexity(debt_type: &DebtType) -> u32 {
    match debt_type {
        DebtType::ComplexityHotspot { cyclomatic, .. } => *cyclomatic,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn create_test_function(name: &str, cyclomatic: u32, cognitive: u32) -> FunctionMetrics {
        FunctionMetrics {
            name: name.to_string(),
            file: PathBuf::from("test.rs"),
            line: 10,
            cyclomatic,
            cognitive,
            nesting: 2,
            length: 20,
            is_test: false,
            visibility: None,
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
    fn test_generate_full_coverage_recommendation() {
        let (action, rationale, steps) =
            generate_full_coverage_recommendation(FunctionRole::PureLogic);
        assert_eq!(action, "Maintain test coverage");
        assert!(rationale.contains("Business logic"));
        assert!(rationale.contains("100% covered"));
        assert_eq!(steps.len(), 3);
    }

    #[test]
    fn test_extract_coverage_percent() {
        let coverage = TransitiveCoverage {
            direct: 0.75,
            transitive: 0.85,
            propagated_from: vec![],
            uncovered_lines: vec![],
        };
        assert_eq!(extract_coverage_percent(&Some(coverage)), 0.75);
        assert_eq!(extract_coverage_percent(&None), 0.0);
    }

    #[test]
    fn test_extract_cyclomatic_complexity() {
        assert_eq!(
            extract_cyclomatic_complexity(&DebtType::ComplexityHotspot {
                cyclomatic: 15,
                cognitive: 20,
            }),
            15
        );

        assert_eq!(
            extract_cyclomatic_complexity(&DebtType::TestingGap {
                coverage: 0.5,
                cyclomatic: 10,
                cognitive: 12,
            }),
            0
        );
    }

    #[test]
    fn test_build_actionable_recommendation() {
        let recommendation = build_actionable_recommendation(
            "Fix the issue".to_string(),
            "This is why it matters".to_string(),
            vec!["Step 1".to_string(), "Step 2".to_string()],
        );

        assert_eq!(recommendation.primary_action, "Fix the issue");
        assert_eq!(recommendation.rationale, "This is why it matters");
        assert_eq!(recommendation.implementation_steps.len(), 2);
        assert!(recommendation.related_items.is_empty());
    }

    #[test]
    fn test_generate_testing_gap_recommendation_full_coverage() {
        let func = create_test_function("test_func", 5, 8);

        let (action, rationale, steps) = generate_testing_gap_recommendation(
            1.0, // 100% coverage
            5,   // cyclomatic
            8,   // cognitive
            FunctionRole::PureLogic,
            &func,
            &None,
        );

        assert_eq!(action, "Maintain test coverage");
        assert!(rationale.contains("Business logic function is currently 100% covered"));
        assert_eq!(steps.len(), 3);
    }

    #[test]
    fn test_generate_error_swallowing_recommendation() {
        let (action, rationale, steps) =
            generate_error_swallowing_recommendation("unwrap_or_default", &None);

        assert!(action.contains("Fix error swallowing"));
        assert!(rationale.contains("unwrap_or_default"));
        assert_eq!(steps.len(), 5);
    }
}
