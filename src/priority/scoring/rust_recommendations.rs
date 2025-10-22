use crate::priority::UnifiedDebtItem;
use crate::risk::coverage_gap::{calculate_coverage_gap, CoverageGap};
use crate::risk::lcov::LcovData;

use super::test_calculation::calculate_tests_needed;

/// Generate Rust-idiomatic refactoring recommendations based on complexity patterns
pub fn generate_rust_refactoring_recommendation(
    item: &UnifiedDebtItem,
    cyclo: u32,
    coverage_percent: f64,
    has_coverage_data: bool,
) -> (String, String, Vec<String>) {
    generate_rust_refactoring_recommendation_with_lcov(
        item,
        cyclo,
        coverage_percent,
        has_coverage_data,
        None,
    )
}

/// Generate Rust-idiomatic refactoring recommendations with optional LCOV data for precise coverage gaps
pub fn generate_rust_refactoring_recommendation_with_lcov(
    item: &UnifiedDebtItem,
    cyclo: u32,
    coverage_percent: f64,
    has_coverage_data: bool,
    lcov_data: Option<&LcovData>,
) -> (String, String, Vec<String>) {
    let function_name = &item.location.function;
    let is_async = function_name.contains("async") || function_name.contains("poll");
    let is_builder = function_name.contains("build") || function_name.contains("new");
    let is_parser = function_name.contains("parse") || function_name.contains("from_");
    let nesting_depth = item.nesting_depth;

    // Calculate coverage gap with line-level precision if available
    let coverage_gap = calculate_coverage_gap(
        coverage_percent,
        item.function_length as u32,
        &item.location.file,
        function_name,
        item.location.line,
        lcov_data,
    );

    // Determine refactoring strategy based on function characteristics
    if cyclo > 30 {
        generate_extreme_complexity_rust_recommendation(
            cyclo,
            &coverage_gap,
            is_async,
            has_coverage_data,
        )
    } else if is_parser || function_name.contains("extract") || function_name.contains("analyze") {
        generate_parser_pattern_recommendation(cyclo, &coverage_gap, has_coverage_data)
    } else if is_builder {
        generate_builder_pattern_recommendation(cyclo, coverage_percent)
    } else if is_async {
        generate_async_refactoring_recommendation(cyclo, &coverage_gap, has_coverage_data)
    } else if cyclo > 15 {
        generate_functional_decomposition_recommendation(cyclo, &coverage_gap, has_coverage_data)
    } else {
        generate_simple_extraction_recommendation(
            cyclo,
            &coverage_gap,
            nesting_depth,
            has_coverage_data,
        )
    }
}

fn generate_extreme_complexity_rust_recommendation(
    cyclo: u32,
    coverage_gap: &CoverageGap,
    is_async: bool,
    has_coverage_data: bool,
) -> (String, String, Vec<String>) {
    let estimated_functions = (cyclo as f64 / 3.0).ceil() as usize;

    let action = format!(
        "Decompose into {} smaller functions using Result chaining",
        estimated_functions
    );

    let mut steps = vec![
        "Identify distinct phases in the function:".to_string(),
        "  • Input validation → validate_inputs() -> Result<ValidatedInput, Error>".to_string(),
        "  • Data transformation → transform_data(input) -> Result<ProcessedData, Error>"
            .to_string(),
        "  • Business logic → apply_business_rules(data) -> Result<Output, Error>".to_string(),
        "  • Side effects → perform_side_effects(output) -> Result<(), Error>".to_string(),
    ];

    if has_coverage_data && coverage_gap.percentage() > 20.0 {
        steps.push(format!(
            "Add property-based tests using proptest or quickcheck - {}",
            coverage_gap.format()
        ));
    }

    steps.extend(vec![
        "Use Result combinators to chain operations:".to_string(),
        "  • map() for transformations".to_string(),
        "  • and_then() for operations that can fail".to_string(),
        "  • map_err() for error context".to_string(),
    ]);

    if is_async {
        steps.push("Consider using async stream processing with futures::stream".to_string());
    }

    (
        action,
        format!(
            "Extreme complexity ({}), needs functional decomposition",
            cyclo
        ),
        steps,
    )
}

fn generate_parser_pattern_recommendation(
    cyclo: u32,
    coverage_gap: &CoverageGap,
    has_coverage_data: bool,
) -> (String, String, Vec<String>) {
    let action = "Refactor using nom or pest parser combinators".to_string();

    let mut steps = vec![
        "Break parsing into composable units:".to_string(),
        "  • Define atomic parsers for basic elements".to_string(),
        "  • Combine parsers using combinators".to_string(),
        "  • Separate tokenization from parsing".to_string(),
    ];

    if cyclo > 20 {
        steps.extend(vec![
            "Consider using a parser generator:".to_string(),
            "  • pest for PEG grammars".to_string(),
            "  • lalrpop for LR(1) grammars".to_string(),
        ]);
    }

    if has_coverage_data && coverage_gap.percentage() > 20.0 {
        steps.push(format!(
            "Add fuzz testing with cargo-fuzz for parser robustness - {}",
            coverage_gap.format()
        ));
    }

    (
        action,
        format!("Parser complexity ({}), use combinators", cyclo),
        steps,
    )
}

fn generate_builder_pattern_recommendation(
    cyclo: u32,
    _coverage_percent: f64,
) -> (String, String, Vec<String>) {
    let action = "Implement typed builder pattern with compile-time validation".to_string();

    let steps = vec![
        "Use typestate pattern for builder:".to_string(),
        "  • Create marker types for builder states".to_string(),
        "  • Use phantom data to track required fields".to_string(),
        "  • Ensure compile-time validation of required fields".to_string(),
        "Leverage derive_builder crate for boilerplate reduction".to_string(),
        "Add #[must_use] attributes to builder methods".to_string(),
    ];

    (
        action,
        format!("Builder complexity ({}), use typestate pattern", cyclo),
        steps,
    )
}

fn generate_async_refactoring_recommendation(
    cyclo: u32,
    coverage_gap: &CoverageGap,
    has_coverage_data: bool,
) -> (String, String, Vec<String>) {
    let action = "Refactor using async/await patterns and futures combinators".to_string();

    let mut steps = vec![
        "Decompose into async sub-tasks:".to_string(),
        "  • Use tokio::spawn for independent tasks".to_string(),
        "  • Apply futures::join! for parallel execution".to_string(),
        "  • Use select! for racing operations".to_string(),
    ];

    if cyclo > 15 {
        steps.extend(vec![
            "Consider async stream processing:".to_string(),
            "  • Use Stream trait for async iteration".to_string(),
            "  • Apply stream combinators (map, filter, fold)".to_string(),
        ]);
    }

    if has_coverage_data && coverage_gap.percentage() > 20.0 {
        steps.push(format!(
            "Add async tests with #[tokio::test] or #[async_std::test] - {}",
            coverage_gap.format()
        ));
    }

    (
        action,
        format!("Async complexity ({}), use futures combinators", cyclo),
        steps,
    )
}

fn generate_functional_decomposition_recommendation(
    cyclo: u32,
    coverage_gap: &CoverageGap,
    has_coverage_data: bool,
) -> (String, String, Vec<String>) {
    let functions_needed = (cyclo as f64 / 5.0).ceil() as usize;

    let action = format!(
        "Apply functional patterns: {} pure functions with Iterator chains",
        functions_needed
    );

    let mut steps = vec![
        "Extract pure functions for each logical operation:".to_string(),
        "  • Predicates: is_valid(), should_process()".to_string(),
        "  • Transformations: map_to_domain(), normalize()".to_string(),
        "  • Aggregations: fold(), collect_results()".to_string(),
        "Use Iterator methods instead of loops:".to_string(),
        "  • filter() instead of if statements in loops".to_string(),
        "  • map() for transformations".to_string(),
        "  • fold() or reduce() for aggregations".to_string(),
        "  • partition() for splitting collections".to_string(),
    ];

    if has_coverage_data && coverage_gap.percentage() > 20.0 {
        steps.push(format!(
            "Use property-based testing to verify function invariants - {}",
            coverage_gap.format()
        ));
    }

    (
        action,
        format!(
            "Moderate complexity ({}), needs functional decomposition",
            cyclo
        ),
        steps,
    )
}

fn generate_simple_extraction_recommendation(
    cyclo: u32,
    coverage_gap: &CoverageGap,
    nesting_depth: u32,
    has_coverage_data: bool,
) -> (String, String, Vec<String>) {
    let coverage_percent = 1.0 - (coverage_gap.percentage() / 100.0);
    let coverage_gap_pct = coverage_gap.percentage() as u32;

    // Use unified test calculation module for consistency (spec 109)
    let test_rec = calculate_tests_needed(cyclo, coverage_percent, None);
    let tests_needed = test_rec.count;

    // Generate specific action based on actual metrics with precise gap info
    let action = if has_coverage_data && coverage_gap_pct > 40 {
        format!(
            "Add {} tests for {}. NO refactoring needed (complexity {} is acceptable)",
            tests_needed,
            coverage_gap.format(),
            cyclo
        )
    } else if cyclo > 10 && nesting_depth > 3 {
        if has_coverage_data {
            format!("Reduce nesting from {} levels using early returns. Add {} tests for uncovered branches",
                    nesting_depth, tests_needed)
        } else {
            format!(
                "Reduce nesting from {} levels using early returns",
                nesting_depth
            )
        }
    } else if cyclo > 10 {
        if has_coverage_data {
            format!(
                "Apply early returns to simplify control flow. Add {} tests for {}",
                tests_needed,
                coverage_gap.format()
            )
        } else {
            "Apply early returns to simplify control flow".to_string()
        }
    } else if has_coverage_data && coverage_gap_pct > 0 {
        format!(
            "Maintain current structure. Add {} tests for {}",
            tests_needed.max(1),
            coverage_gap.format()
        )
    } else if has_coverage_data {
        "Function is well-tested and simple. No action required".to_string()
    } else {
        // No coverage data - focus on complexity only
        format!(
            "Complexity {} is manageable. Consider refactoring if complexity increases",
            cyclo
        )
    };

    let why = if has_coverage_data && coverage_gap_pct > 0 {
        format!(
            "Complexity {} is manageable. {}. Focus on test coverage, not refactoring",
            cyclo,
            coverage_gap.format()
        )
    } else if has_coverage_data {
        format!(
            "Complexity {} with full coverage. Well-maintained function",
            cyclo
        )
    } else {
        // No coverage data provided - don't mention coverage
        format!(
            "Complexity {} is manageable. Focus on maintaining simplicity",
            cyclo
        )
    };

    // Generate specific steps based on actual values, no conditional "IF" statements
    let mut steps = vec![];

    if has_coverage_data && coverage_gap_pct > 20 {
        steps.push(format!(
            "Write {} focused tests for uncovered branches",
            tests_needed
        ));
        steps.push("Each test should be <15 lines and test ONE path".to_string());

        // Add specific guidance based on complexity
        if cyclo <= 5 {
            steps.push("Test edge cases and boundary conditions".to_string());
        } else if cyclo <= 10 {
            steps.push("Focus on testing each decision branch independently".to_string());
        }
    }

    // Provide specific refactoring guidance based on actual nesting depth
    if nesting_depth > 3 && cyclo > 10 {
        steps.push(format!(
            "Current nesting depth is {} levels - use early returns with ? operator",
            nesting_depth
        ));
        steps.push("Extract deeply nested blocks into separate functions".to_string());
        steps.push(
            "Replace nested if-else chains with match expressions where appropriate".to_string(),
        );
    } else if nesting_depth > 2 && cyclo > 5 {
        steps.push(format!(
            "Nesting depth of {} can be reduced with guard clauses",
            nesting_depth
        ));
        steps.push("Move validation checks to the beginning with early returns".to_string());
    } else if cyclo > 10 {
        steps.push("Extract helper functions for complex boolean expressions".to_string());
        steps.push("Consider using match expressions instead of if-else chains".to_string());
    } else if cyclo > 5 {
        steps.push("Current structure is acceptable - prioritize test coverage".to_string());
        if nesting_depth > 1 {
            steps.push("Consider extracting guard clauses for precondition checks".to_string());
        }
    } else {
        steps.push("Current structure is clean and simple".to_string());
        if has_coverage_data && coverage_gap_pct > 0 {
            steps.push(format!(
                "Focus on adding the {} missing tests - {}",
                tests_needed,
                coverage_gap.format()
            ));
        }
    }

    (action, why, steps)
}

/// Generate file-level recommendations for Rust modules
pub fn generate_rust_file_recommendation(
    function_count: usize,
    avg_complexity: f64,
    is_god_object: bool,
) -> String {
    if is_god_object {
        format!(
            "Split into {} modules using Rust's module system. \
             Group by trait implementations or functional domains. \
             Consider using workspace members for large refactors.",
            (function_count / 8).max(2)
        )
    } else if function_count > 30 {
        format!(
            "Module has {} functions (avg complexity: {:.1}). \
             Consider splitting into submodules by functionality. \
             Use pub(crate) for internal APIs.",
            function_count, avg_complexity
        )
    } else if avg_complexity > 10.0 {
        format!(
            "High average complexity ({:.1}). \
             Extract complex logic into separate modules. \
             Use newtype pattern for domain modeling.",
            avg_complexity
        )
    } else {
        "Consider extracting related functions into trait implementations".to_string()
    }
}
