// Recommendation generation functions for debt items

use crate::core::FunctionMetrics;
use crate::priority::semantic_classifier::FunctionRole;
use crate::priority::{DebtType, FunctionVisibility, TransitiveCoverage};

/// Get display name for a function role
fn get_role_display_name(role: FunctionRole) -> &'static str {
    match role {
        FunctionRole::PureLogic => "Business logic",
        FunctionRole::Orchestrator => "Orchestration",
        FunctionRole::IOWrapper => "I/O wrapper",
        FunctionRole::EntryPoint => "Entry point",
        FunctionRole::PatternMatch => "Pattern matching",
        FunctionRole::Unknown => "Function",
    }
}

/// Calculate test cases needed based on complexity and current coverage
fn calculate_needed_test_cases(cyclomatic: u32, coverage_pct: f64) -> u32 {
    if coverage_pct >= 1.0 {
        return 0;
    }

    let current_test_cases = if coverage_pct > 0.0 {
        (cyclomatic as f64 * coverage_pct).ceil() as u32
    } else {
        0
    };

    cyclomatic.saturating_sub(current_test_cases)
}

/// Calculate approximate test cases for simple functions
fn calculate_simple_test_cases(cyclomatic: u32, coverage_pct: f64) -> u32 {
    ((cyclomatic.max(2) as f64 * (1.0 - coverage_pct)).ceil() as u32).max(2)
}

/// Add uncovered lines recommendations to steps
fn add_uncovered_lines_to_steps(
    steps: &mut Vec<String>,
    func: &FunctionMetrics,
    transitive_coverage: &Option<TransitiveCoverage>,
) {
    if let Some(cov) = transitive_coverage {
        if !cov.uncovered_lines.is_empty() {
            let uncovered_recommendations = analyze_uncovered_lines(func, &cov.uncovered_lines);
            for (i, rec) in uncovered_recommendations.into_iter().enumerate() {
                steps.insert(i, rec);
            }
        }
    }
}

/// Generate recommendation when function is fully covered
fn generate_full_coverage_recommendation(role: FunctionRole) -> (String, String, Vec<String>) {
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
fn generate_complex_function_recommendation(
    cyclomatic: u32,
    cognitive: u32,
    coverage_pct: f64,
    coverage_gap: i32,
    role_str: String,
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
fn generate_simple_function_recommendation(
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
pub fn generate_testing_gap_recommendation(
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
pub fn generate_dead_code_recommendation(
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
pub fn generate_error_swallowing_recommendation(
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
pub fn generate_test_debt_recommendation(debt_type: &DebtType) -> (String, String, Vec<String>) {
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

// Helper functions (these need to be imported or defined)

fn format_role_description(role: FunctionRole) -> String {
    match role {
        FunctionRole::PureLogic => "pure logic".to_string(),
        FunctionRole::Orchestrator => "orchestrator".to_string(),
        FunctionRole::IOWrapper => "I/O wrapper".to_string(),
        FunctionRole::EntryPoint => "entry point".to_string(),
        FunctionRole::PatternMatch => "pattern matching".to_string(),
        FunctionRole::Unknown => "function".to_string(),
    }
}

pub fn calculate_functions_to_extract(cyclomatic: u32, cognitive: u32) -> u32 {
    // Estimate number of functions to extract based on complexity
    if cyclomatic > 20 || cognitive > 30 {
        4
    } else if cyclomatic > 15 || cognitive > 20 {
        3
    } else if cyclomatic > 10 || cognitive > 15 {
        2
    } else {
        1
    }
}

fn generate_combined_testing_refactoring_steps(
    cyclomatic: u32,
    cognitive: u32,
    coverage_pct: i32,
) -> Vec<String> {
    vec![
        format!(
            "Add tests for the {} uncovered branches (current coverage: {}%)",
            cyclomatic - (cyclomatic as f64 * coverage_pct as f64 / 100.0) as u32,
            coverage_pct
        ),
        "Identify logical sections within the function".to_string(),
        format!(
            "Extract {} helper functions to reduce complexity",
            calculate_functions_to_extract(cyclomatic, cognitive)
        ),
        "Ensure each extracted function has single responsibility".to_string(),
        "Add unit tests for each extracted function".to_string(),
    ]
}

fn generate_testing_gap_steps(is_complex: bool) -> Vec<String> {
    if is_complex {
        vec![
            "Write tests for critical paths first".to_string(),
            "Cover error handling scenarios".to_string(),
            "Test edge cases and boundary conditions".to_string(),
            "Consider refactoring after achieving coverage".to_string(),
        ]
    } else {
        vec![
            "Write tests for main execution paths".to_string(),
            "Cover error handling scenarios".to_string(),
            "Test edge cases and boundary conditions".to_string(),
        ]
    }
}

pub fn analyze_uncovered_lines(_func: &FunctionMetrics, uncovered_lines: &[usize]) -> Vec<String> {
    let mut recommendations = Vec::new();

    // Group consecutive lines
    let mut line_groups = Vec::new();
    let mut current_group = Vec::new();

    for &line in uncovered_lines {
        if current_group.is_empty() || line == current_group.last().unwrap() + 1 {
            current_group.push(line);
        } else {
            if !current_group.is_empty() {
                line_groups.push(current_group.clone());
            }
            current_group = vec![line];
        }
    }
    if !current_group.is_empty() {
        line_groups.push(current_group);
    }

    // Generate recommendations based on line groups
    for group in line_groups.iter().take(3) {
        // Limit to first 3 groups
        if group.len() > 1 {
            recommendations.push(format!(
                "Focus on uncovered block at lines {}-{}",
                group.first().unwrap(),
                group.last().unwrap()
            ));
        } else {
            recommendations.push(format!("Cover uncovered line {}", group[0]));
        }
    }

    if line_groups.len() > 3 {
        recommendations.push(format!(
            "...and {} more uncovered sections",
            line_groups.len() - 3
        ));
    }

    recommendations
}

fn generate_dead_code_action(
    func: &FunctionMetrics,
    visibility: &FunctionVisibility,
    name: &str,
    cyclomatic: &u32,
    cognitive: &u32,
) -> (String, String) {
    match visibility {
        FunctionVisibility::Public => {
            if name.starts_with("test_") || func.file.to_string_lossy().contains("test") {
                (
                    "Remove unused test helper".to_string(),
                    format!(
                        "Unused test helper function with complexity {}/{}",
                        cyclomatic, cognitive
                    ),
                )
            } else {
                (
                    "Remove unused public function (no API indicators)".to_string(),
                    format!(
                        "Public function with no callers and complexity {}/{}",
                        cyclomatic, cognitive
                    ),
                )
            }
        }
        FunctionVisibility::Private | FunctionVisibility::Crate => (
            "Remove unused private function".to_string(),
            format!(
                "Private function with no callers and complexity {}/{}",
                cyclomatic, cognitive
            ),
        ),
    }
}

fn generate_dead_code_steps(visibility: &FunctionVisibility) -> Vec<String> {
    match visibility {
        FunctionVisibility::Public => vec![
            "Verify function is not used by external crates".to_string(),
            "Check if function is part of public API contract".to_string(),
            "If truly unused, remove the function".to_string(),
            "Update any documentation referencing this function".to_string(),
        ],
        FunctionVisibility::Private | FunctionVisibility::Crate => vec![
            "Confirm function has no callers in codebase".to_string(),
            "Remove the function definition".to_string(),
            "Clean up any related test code".to_string(),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::FunctionMetrics;
    use crate::priority::semantic_classifier::FunctionRole;
    use crate::priority::TransitiveCoverage;

    fn create_test_function(name: &str) -> FunctionMetrics {
        FunctionMetrics {
            name: name.to_string(),
            file: "test.rs".into(),
            line: 10,
            cyclomatic: 10,
            cognitive: 15,
            nesting: 2,
            length: 50,
            is_test: false,
            visibility: None,
            is_trait_method: false,
            in_test_module: false,
            entropy_score: None,
            is_pure: None,
            purity_confidence: None,
        }
    }

    #[test]
    fn test_get_role_display_name() {
        assert_eq!(
            get_role_display_name(FunctionRole::PureLogic),
            "Business logic"
        );
        assert_eq!(
            get_role_display_name(FunctionRole::Orchestrator),
            "Orchestration"
        );
        assert_eq!(
            get_role_display_name(FunctionRole::IOWrapper),
            "I/O wrapper"
        );
        assert_eq!(
            get_role_display_name(FunctionRole::EntryPoint),
            "Entry point"
        );
        assert_eq!(
            get_role_display_name(FunctionRole::PatternMatch),
            "Pattern matching"
        );
        assert_eq!(get_role_display_name(FunctionRole::Unknown), "Function");
    }

    #[test]
    fn test_calculate_needed_test_cases() {
        // Full coverage
        assert_eq!(calculate_needed_test_cases(10, 1.0), 0);

        // No coverage
        assert_eq!(calculate_needed_test_cases(10, 0.0), 10);

        // Partial coverage (50%)
        assert_eq!(calculate_needed_test_cases(10, 0.5), 5);

        // Partial coverage (75%)
        assert_eq!(calculate_needed_test_cases(10, 0.75), 2);

        // Almost full coverage
        assert_eq!(calculate_needed_test_cases(10, 0.9), 1);
    }

    #[test]
    fn test_calculate_simple_test_cases() {
        // No coverage
        assert_eq!(calculate_simple_test_cases(5, 0.0), 5);

        // Partial coverage
        assert_eq!(calculate_simple_test_cases(5, 0.5), 3);

        // Low complexity, no coverage - minimum 2 tests
        assert_eq!(calculate_simple_test_cases(1, 0.0), 2);

        // Full coverage
        assert_eq!(calculate_simple_test_cases(5, 1.0), 2);
    }

    #[test]
    fn test_generate_full_coverage_recommendation() {
        let (action, rationale, steps) =
            generate_full_coverage_recommendation(FunctionRole::PureLogic);

        assert_eq!(action, "Maintain test coverage");
        assert!(rationale.contains("Business logic"));
        assert!(rationale.contains("100% covered"));
        assert_eq!(steps.len(), 3);
        assert!(steps[0].contains("Keep tests up to date"));
    }

    #[test]
    fn test_generate_testing_gap_recommendation_full_coverage() {
        let func = create_test_function("test_func");
        let (action, rationale, steps) = generate_testing_gap_recommendation(
            1.0, // 100% coverage
            10,
            15,
            FunctionRole::PureLogic,
            &func,
            &None,
        );

        assert_eq!(action, "Maintain test coverage");
        assert!(rationale.contains("100% covered"));
        assert_eq!(steps.len(), 3);
    }

    #[test]
    fn test_generate_testing_gap_recommendation_complex_no_coverage() {
        let func = create_test_function("complex_func");
        let (action, rationale, steps) = generate_testing_gap_recommendation(
            0.0, // 0% coverage
            25,  // High complexity
            30,  // High cognitive complexity
            FunctionRole::PureLogic,
            &func,
            &None,
        );

        assert!(action.contains("Add"));
        assert!(action.contains("tests"));
        assert!(action.contains("refactor"));
        assert!(rationale.contains("Complex"));
        assert!(rationale.contains("100% gap"));
        assert!(!steps.is_empty());
    }

    #[test]
    fn test_generate_testing_gap_recommendation_simple_partial_coverage() {
        let func = create_test_function("simple_func");
        let (action, rationale, steps) = generate_testing_gap_recommendation(
            0.5, // 50% coverage
            5,   // Low complexity
            8,   // Low cognitive complexity
            FunctionRole::Orchestrator,
            &func,
            &None,
        );

        assert!(action.contains("Add"));
        assert!(action.contains("tests"));
        assert!(!action.contains("refactor")); // Simple function, no refactoring
        assert!(rationale.contains("Orchestration"));
        // Check for both "gap" or "covered" since message format varies
        assert!(rationale.contains("50%"));
        assert!(!steps.is_empty());
    }

    #[test]
    fn test_generate_testing_gap_recommendation_with_uncovered_lines() {
        let func = create_test_function("func_with_uncovered");
        let transitive_cov = TransitiveCoverage {
            direct: 0.5,
            transitive: 0.6,
            uncovered_lines: vec![15, 16, 20, 25],
            propagated_from: vec![],
        };

        let (action, _rationale, steps) = generate_testing_gap_recommendation(
            0.5,
            8,
            12,
            FunctionRole::IOWrapper,
            &func,
            &Some(transitive_cov),
        );

        assert!(action.contains("Add"));
        assert!(action.contains("tests"));
        // Should have additional steps from uncovered lines analysis
        assert!(steps.len() > 3);
    }

    #[test]
    fn test_boundary_conditions() {
        let func = create_test_function("boundary_func");

        // Exactly at complexity threshold (10)
        let (action, _, _) = generate_testing_gap_recommendation(
            0.5,
            10, // Exactly at threshold
            14, // Below cognitive threshold
            FunctionRole::Unknown,
            &func,
            &None,
        );
        assert!(action.contains("tests"));
        assert!(!action.contains("refactor")); // Not complex enough

        // Just above complexity threshold
        let (action, _, _) = generate_testing_gap_recommendation(
            0.5,
            11, // Just above threshold
            14,
            FunctionRole::Unknown,
            &func,
            &None,
        );
        assert!(action.contains("refactor")); // Now complex enough
    }

    #[test]
    fn test_all_function_roles() {
        let func = create_test_function("role_test");
        let roles = vec![
            FunctionRole::PureLogic,
            FunctionRole::Orchestrator,
            FunctionRole::IOWrapper,
            FunctionRole::EntryPoint,
            FunctionRole::PatternMatch,
            FunctionRole::Unknown,
        ];

        for role in roles {
            let (action, rationale, steps) =
                generate_testing_gap_recommendation(0.3, 7, 10, role, &func, &None);

            assert!(!action.is_empty());
            assert!(!rationale.is_empty());
            assert!(!steps.is_empty());
        }
    }
}
