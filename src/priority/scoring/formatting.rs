// Formatting and helper utility functions for debt item processing

use crate::core::FunctionMetrics;
use crate::priority::{FunctionRole, FunctionVisibility};

/// Helper to format complexity metrics for display
pub(super) fn format_complexity_display(cyclomatic: &u32, cognitive: &u32) -> String {
    format!("cyclo={cyclomatic}, cog={cognitive}")
}

/// Helper to format role description
pub(super) fn format_role_description(role: FunctionRole) -> &'static str {
    match role {
        FunctionRole::PureLogic => "business logic",
        FunctionRole::Orchestrator => "orchestration",
        FunctionRole::IOWrapper => "I/O wrapper",
        FunctionRole::EntryPoint => "entry point",
        FunctionRole::PatternMatch => "pattern matching",
        FunctionRole::Debug => "debug/diagnostic",
        FunctionRole::Unknown => "function",
    }
}

/// Get display name for a function role
pub(super) fn get_role_display_name(role: FunctionRole) -> &'static str {
    match role {
        FunctionRole::PureLogic => "Business logic",
        FunctionRole::Orchestrator => "Orchestration",
        FunctionRole::IOWrapper => "I/O wrapper",
        FunctionRole::EntryPoint => "Entry point",
        FunctionRole::PatternMatch => "Pattern matching",
        FunctionRole::Debug => "Debug/diagnostic",
        FunctionRole::Unknown => "Function",
    }
}

/// Pure function to determine if file is Rust
pub(super) fn is_rust_file(file_path: &std::path::Path) -> bool {
    file_path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e == "rs")
        .unwrap_or(false)
}

/// Determine function visibility from metrics
pub fn determine_visibility(func: &FunctionMetrics) -> FunctionVisibility {
    // Use the visibility field from FunctionMetrics if available
    match &func.visibility {
        Some(vis) if vis == "pub" => FunctionVisibility::Public,
        Some(vis) if vis == "pub(crate)" => FunctionVisibility::Crate,
        Some(vis) if vis.starts_with("pub(") => FunctionVisibility::Crate, // pub(super), pub(in ...), etc.
        _ => FunctionVisibility::Private,
    }
}

/// Generate steps for dead code based on visibility
pub(super) fn generate_dead_code_steps(visibility: &FunctionVisibility) -> Vec<String> {
    match visibility {
        FunctionVisibility::Private => vec![
            "Verify no dynamic calls or reflection usage".to_string(),
            "Remove function definition".to_string(),
            "Remove associated tests if any".to_string(),
            "Check if removal enables further cleanup".to_string(),
        ],
        FunctionVisibility::Crate => vec![
            "Check if function is intended as internal API".to_string(),
            "Add documentation if keeping for future use".to_string(),
            "Remove if truly unused".to_string(),
            "Consider making private if only locally needed".to_string(),
        ],
        FunctionVisibility::Public => vec![
            "Verify no external callers exist".to_string(),
            "Add comprehensive documentation if keeping".to_string(),
            "Mark as deprecated if phasing out".to_string(),
            "Consider adding usage examples or tests".to_string(),
        ],
    }
}

/// Generate steps for testing gap based on complexity
pub(super) fn generate_testing_gap_steps(is_complex: bool) -> Vec<String> {
    if is_complex {
        vec![
            "Identify and extract pure functions (no side effects)".to_string(),
            "Add property-based tests for pure logic".to_string(),
            "Replace conditionals with pattern matching where possible".to_string(),
            "Convert loops to map/filter/fold operations".to_string(),
            "Push I/O to the boundaries".to_string(),
        ]
    } else {
        vec![
            "Test happy path scenarios".to_string(),
            "Add edge case tests".to_string(),
            "Cover error conditions".to_string(),
        ]
    }
}

/// Generate combined testing and refactoring steps for complex functions with low coverage
pub(super) fn generate_combined_testing_refactoring_steps(
    cyclomatic: u32,
    cognitive: u32,
    coverage_pct: i32,
) -> Vec<String> {
    use crate::priority::scoring::computation::calculate_functions_to_extract;

    let functions_to_extract = calculate_functions_to_extract(cyclomatic, cognitive);
    let target_complexity = (cyclomatic / functions_to_extract).max(3);
    let uncovered_branches = ((100 - coverage_pct) as f32 / 100.0 * cyclomatic as f32) as u32;

    vec![
        format!(
            "Currently ~{} of {} branches are uncovered ({}% coverage)",
            uncovered_branches, cyclomatic, coverage_pct
        ),
        format!(
            "Write {} tests to cover critical uncovered branches first",
            uncovered_branches.min(cyclomatic / 2)
        ),
        format!(
            "Extract {} pure functions from {} branches:",
            functions_to_extract, cyclomatic
        ),
        format!(
            "  • Group ~{} related branches per function",
            cyclomatic / functions_to_extract.max(1)
        ),
        format!(
            "  • Target complexity ≤{} per extracted function",
            target_complexity
        ),
        "Extraction patterns to look for:".to_string(),
        "  • Validation logic → validate_input()".to_string(),
        "  • Complex calculations → calculate_result()".to_string(),
        "  • Error handling → handle_errors()".to_string(),
        format!("Write ~{} tests per extracted function", target_complexity),
        "Add property-based tests for complex logic".to_string(),
        format!(
            "Final goal: {}+ functions with ≤{} complexity each, 80%+ coverage",
            functions_to_extract, target_complexity
        ),
    ]
}

/// Analyze uncovered lines to provide specific testing recommendations
pub(super) fn analyze_uncovered_lines(
    func: &FunctionMetrics,
    uncovered_lines: &[usize],
) -> Vec<String> {
    let mut recommendations = Vec::new();

    if uncovered_lines.is_empty() {
        return recommendations;
    }

    // Group consecutive lines into ranges for better readability
    let mut ranges = Vec::new();
    let mut current_start = uncovered_lines[0];
    let mut current_end = uncovered_lines[0];

    for &line in &uncovered_lines[1..] {
        if line == current_end + 1 {
            current_end = line;
        } else {
            ranges.push((current_start, current_end));
            current_start = line;
            current_end = line;
        }
    }
    ranges.push((current_start, current_end));

    // Format line ranges
    let range_strings: Vec<String> = ranges
        .iter()
        .take(5)
        .map(|(start, end)| {
            if start == end {
                format!("{}", start)
            } else {
                format!("{}-{}", start, end)
            }
        })
        .collect();

    let more = if ranges.len() > 5 {
        format!(" and {} more ranges", ranges.len() - 5)
    } else {
        String::new()
    };

    recommendations.push(format!(
        "Add tests for uncovered lines: {}{}",
        range_strings.join(", "),
        more
    ));

    // Provide specific guidance based on the function characteristics
    if func.cyclomatic > 5 {
        // Use helper function from recommendation_helpers for consistent path count formatting
        let path_text = if func.cyclomatic == 1 {
            "single execution path".to_string()
        } else {
            format!("{} execution paths", func.cyclomatic)
        };
        recommendations.push(format!(
            "Focus on testing {} decision points to cover {}",
            func.cyclomatic - 1,
            path_text
        ));
    }

    recommendations
}

/// Add uncovered lines recommendations to steps
pub(super) fn add_uncovered_lines_to_steps(
    steps: &mut Vec<String>,
    func: &FunctionMetrics,
    transitive_coverage: &Option<crate::priority::TransitiveCoverage>,
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_format_complexity_display() {
        assert_eq!(format_complexity_display(&5, &8), "cyclo=5, cog=8");
        assert_eq!(format_complexity_display(&10, &15), "cyclo=10, cog=15");
        assert_eq!(format_complexity_display(&0, &0), "cyclo=0, cog=0");
    }

    #[test]
    fn test_format_role_description() {
        assert_eq!(
            format_role_description(FunctionRole::PureLogic),
            "business logic"
        );
        assert_eq!(
            format_role_description(FunctionRole::Orchestrator),
            "orchestration"
        );
        assert_eq!(
            format_role_description(FunctionRole::IOWrapper),
            "I/O wrapper"
        );
        assert_eq!(
            format_role_description(FunctionRole::EntryPoint),
            "entry point"
        );
        assert_eq!(
            format_role_description(FunctionRole::PatternMatch),
            "pattern matching"
        );
        assert_eq!(format_role_description(FunctionRole::Unknown), "function");
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
    fn test_is_rust_file() {
        assert!(is_rust_file(Path::new("test.rs")));
        assert!(is_rust_file(Path::new("/path/to/file.rs")));
        assert!(!is_rust_file(Path::new("test.py")));
        assert!(!is_rust_file(Path::new("test.js")));
        assert!(!is_rust_file(Path::new("test")));
    }

    #[test]
    fn test_determine_visibility() {
        let pub_func = FunctionMetrics {
            name: "test".to_string(),
            file: "test.rs".into(),
            line: 1,
            cyclomatic: 1,
            cognitive: 1,
            nesting: 1,
            length: 10,
            is_test: false,
            visibility: Some("pub".to_string()),
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
        };

        assert_eq!(determine_visibility(&pub_func), FunctionVisibility::Public);

        let priv_func = FunctionMetrics {
            visibility: None,
            ..pub_func.clone()
        };

        assert_eq!(
            determine_visibility(&priv_func),
            FunctionVisibility::Private
        );
    }

    #[test]
    fn test_generate_dead_code_steps() {
        let private_steps = generate_dead_code_steps(&FunctionVisibility::Private);
        assert_eq!(private_steps.len(), 4);
        assert!(private_steps[0].contains("Verify no dynamic calls"));

        let public_steps = generate_dead_code_steps(&FunctionVisibility::Public);
        assert_eq!(public_steps.len(), 4);
        assert!(public_steps[0].contains("Verify no external callers"));
    }

    #[test]
    fn test_generate_testing_gap_steps() {
        let simple_steps = generate_testing_gap_steps(false);
        assert_eq!(simple_steps.len(), 3);
        assert!(simple_steps[0].contains("happy path"));

        let complex_steps = generate_testing_gap_steps(true);
        assert_eq!(complex_steps.len(), 5);
        assert!(complex_steps[0].contains("pure functions"));
    }

    #[test]
    fn test_analyze_uncovered_lines_empty() {
        let func = FunctionMetrics {
            name: "test".to_string(),
            file: "test.rs".into(),
            line: 1,
            cyclomatic: 5,
            cognitive: 5,
            nesting: 1,
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
        };

        let recommendations = analyze_uncovered_lines(&func, &[]);
        assert!(recommendations.is_empty());
    }

    #[test]
    fn test_analyze_uncovered_lines_with_ranges() {
        let func = FunctionMetrics {
            name: "test".to_string(),
            file: "test.rs".into(),
            line: 1,
            cyclomatic: 8,
            cognitive: 10,
            nesting: 2,
            length: 30,
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
        };

        let recommendations = analyze_uncovered_lines(&func, &[10, 11, 12, 15, 20, 21]);
        assert_eq!(recommendations.len(), 2); // One for uncovered lines, one for decision points
        assert!(recommendations[0].contains("10-12, 15, 20-21"));
        assert!(recommendations[1].contains("7 decision points"));
    }
}
