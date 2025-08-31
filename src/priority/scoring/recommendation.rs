// Recommendation generation functions for debt items

use crate::priority::semantic_classifier::FunctionRole;
use crate::core::FunctionMetrics;
use crate::priority::{FunctionVisibility, DebtType, TransitiveCoverage};

/// Generate recommendation for testing gap debt type
pub fn generate_testing_gap_recommendation(
    coverage_pct: f64,
    cyclomatic: u32,
    cognitive: u32,
    role: FunctionRole,
    func: &FunctionMetrics,
    transitive_coverage: &Option<TransitiveCoverage>,
) -> (String, String, Vec<String>) {
    let is_complex = cyclomatic > 10 || cognitive > 15;
    let coverage_pct_int = (coverage_pct * 100.0) as i32;
    let role_str = format_role_description(role);
    let coverage_gap = 100 - coverage_pct_int;

    // If function is fully covered, no testing gap exists
    if coverage_gap == 0 {
        let role_display = match role {
            FunctionRole::PureLogic => "Business logic",
            FunctionRole::Orchestrator => "Orchestration",
            FunctionRole::IOWrapper => "I/O wrapper",
            FunctionRole::EntryPoint => "Entry point",
            FunctionRole::PatternMatch => "Pattern matching",
            FunctionRole::Unknown => "Function",
        };

        return (
            "Maintain test coverage".to_string(),
            format!("{} function is currently 100% covered", role_display),
            vec![
                "Keep tests up to date with code changes".to_string(),
                "Consider property-based testing for edge cases".to_string(),
                "Monitor coverage in CI/CD pipeline".to_string(),
            ],
        );
    }

    if is_complex {
        let functions_to_extract = calculate_functions_to_extract(cyclomatic, cognitive);

        // Calculate test cases needed
        let current_test_cases = if coverage_pct_int > 0 {
            (cyclomatic as f64 * coverage_pct).ceil() as u32
        } else {
            0
        };
        let needed_test_cases = cyclomatic.saturating_sub(current_test_cases);

        // Explain why both testing and refactoring are needed
        let complexity_explanation = format!(
            "Cyclomatic complexity of {} requires at least {} test cases for full path coverage. After extracting {} functions, each will need only 3-5 tests",
            cyclomatic, cyclomatic, functions_to_extract
        );

        // Add uncovered lines info if available
        let mut steps =
            generate_combined_testing_refactoring_steps(cyclomatic, cognitive, coverage_pct_int);
        if let Some(cov) = transitive_coverage {
            if !cov.uncovered_lines.is_empty() {
                let uncovered_recommendations = analyze_uncovered_lines(func, &cov.uncovered_lines);
                // Insert uncovered lines info at the beginning of steps
                for (i, rec) in uncovered_recommendations.into_iter().enumerate() {
                    steps.insert(i, rec);
                }
            }
        }

        (
            format!("Add {} tests for {}% coverage gap, then refactor complexity {} into {} functions", 
                   needed_test_cases, coverage_gap, cyclomatic, functions_to_extract),
            format!("Complex {role_str} with {coverage_gap}% gap. {}. Testing before refactoring ensures no regressions",
                   complexity_explanation),
            steps,
        )
    } else {
        let role_display = match role {
            FunctionRole::PureLogic => "Business logic",
            FunctionRole::Orchestrator => "Orchestration",
            FunctionRole::IOWrapper => "I/O wrapper",
            FunctionRole::EntryPoint => "Entry point",
            FunctionRole::PatternMatch => "Pattern matching",
            FunctionRole::Unknown => "Function",
        };

        // Calculate approximate test cases needed (minimum 2 for basic happy/error paths)
        let test_cases_needed =
            ((cyclomatic.max(2) as f64 * (1.0 - coverage_pct)).ceil() as u32).max(2);

        let coverage_explanation = if coverage_pct_int == 0 {
            format!("{role_display} with {coverage_gap}% coverage gap, currently {coverage_pct_int}% covered. Needs {} test cases to cover all {} execution paths",
                   test_cases_needed, cyclomatic.max(2))
        } else {
            format!("{role_display} with {coverage_gap}% coverage gap, currently {coverage_pct_int}% covered. Needs {} more test cases",
                   test_cases_needed)
        };

        // Add uncovered lines info if available
        let mut steps = generate_testing_gap_steps(false);
        if let Some(cov) = transitive_coverage {
            if !cov.uncovered_lines.is_empty() {
                let uncovered_recommendations = analyze_uncovered_lines(func, &cov.uncovered_lines);
                // Insert uncovered lines info at the beginning of steps
                for (i, rec) in uncovered_recommendations.into_iter().enumerate() {
                    steps.insert(i, rec);
                }
            }
        }

        (
            format!(
                "Add {} tests for {}% coverage gap",
                test_cases_needed, coverage_gap
            ),
            coverage_explanation,
            steps,
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
        format!("Add tests for the {} uncovered branches (current coverage: {}%)", 
                cyclomatic - (cyclomatic as f64 * coverage_pct as f64 / 100.0) as u32, coverage_pct),
        "Identify logical sections within the function".to_string(),
        format!("Extract {} helper functions to reduce complexity", 
                calculate_functions_to_extract(cyclomatic, cognitive)),
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
    for group in line_groups.iter().take(3) {  // Limit to first 3 groups
        if group.len() > 1 {
            recommendations.push(format!(
                "Focus on uncovered block at lines {}-{}", 
                group.first().unwrap(), 
                group.last().unwrap()
            ));
        } else {
            recommendations.push(format!(
                "Cover uncovered line {}", 
                group[0]
            ));
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
                    format!("Unused test helper function with complexity {}/{}", cyclomatic, cognitive),
                )
            } else {
                (
                    "Remove unused public function (no API indicators)".to_string(),
                    format!("Public function with no callers and complexity {}/{}", cyclomatic, cognitive),
                )
            }
        }
        FunctionVisibility::Private | FunctionVisibility::Crate => (
            "Remove unused private function".to_string(),
            format!("Private function with no callers and complexity {}/{}", cyclomatic, cognitive),
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