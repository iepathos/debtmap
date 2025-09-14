use crate::priority::UnifiedDebtItem;

/// Generate Rust-idiomatic refactoring recommendations based on complexity patterns
pub fn generate_rust_refactoring_recommendation(
    item: &UnifiedDebtItem,
    cyclo: u32,
    coverage_percent: f64,
) -> (String, String, Vec<String>) {
    let function_name = &item.location.function;
    let is_async = function_name.contains("async") || function_name.contains("poll");
    let is_builder = function_name.contains("build") || function_name.contains("new");
    let is_parser = function_name.contains("parse") || function_name.contains("from_");
    let nesting_depth = item.nesting_depth;

    // Determine refactoring strategy based on function characteristics
    if cyclo > 30 {
        generate_extreme_complexity_rust_recommendation(cyclo, coverage_percent, is_async)
    } else if is_parser || function_name.contains("extract") || function_name.contains("analyze") {
        generate_parser_pattern_recommendation(cyclo, coverage_percent)
    } else if is_builder {
        generate_builder_pattern_recommendation(cyclo, coverage_percent)
    } else if is_async {
        generate_async_refactoring_recommendation(cyclo, coverage_percent)
    } else if cyclo > 15 {
        generate_functional_decomposition_recommendation(cyclo, coverage_percent)
    } else {
        generate_simple_extraction_recommendation(cyclo, coverage_percent, nesting_depth)
    }
}

fn generate_extreme_complexity_rust_recommendation(
    cyclo: u32,
    coverage_percent: f64,
    is_async: bool,
) -> (String, String, Vec<String>) {
    let estimated_functions = (cyclo as f64 / 3.0).ceil() as usize;
    let test_gap = ((1.0 - coverage_percent) * 100.0) as usize;

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

    if coverage_percent < 0.8 {
        steps.push(format!(
            "Add property-based tests using proptest or quickcheck for {}% coverage gap",
            test_gap
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
    coverage_percent: f64,
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

    if coverage_percent < 0.8 {
        steps.push("Add fuzz testing with cargo-fuzz for parser robustness".to_string());
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
    coverage_percent: f64,
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

    if coverage_percent < 0.8 {
        steps.push("Add async tests with #[tokio::test] or #[async_std::test]".to_string());
    }

    (
        action,
        format!("Async complexity ({}), use futures combinators", cyclo),
        steps,
    )
}

fn generate_functional_decomposition_recommendation(
    cyclo: u32,
    coverage_percent: f64,
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

    if coverage_percent < 0.8 {
        steps.push("Use property-based testing to verify function invariants".to_string());
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
    coverage_percent: f64,
    nesting_depth: u32,
) -> (String, String, Vec<String>) {
    let coverage_gap = ((1.0 - coverage_percent) * 100.0) as u32;
    let tests_needed = ((cyclo as f64) * (1.0 - coverage_percent)).ceil() as u32;

    // Generate specific action based on actual metrics
    let action = if coverage_gap > 40 {
        format!("Add {} tests for {}% coverage gap. NO refactoring needed (complexity {} is acceptable)",
                tests_needed, coverage_gap, cyclo)
    } else if cyclo > 10 && nesting_depth > 3 {
        format!("Reduce nesting from {} levels using early returns. Add {} tests for uncovered branches",
                nesting_depth, tests_needed)
    } else if cyclo > 10 {
        format!(
            "Apply early returns to simplify control flow. Add {} tests for uncovered branches",
            tests_needed
        )
    } else if coverage_gap > 0 {
        format!(
            "Maintain current structure. Add {} tests for {}% coverage gap",
            tests_needed.max(1),
            coverage_gap
        )
    } else {
        "Function is well-tested and simple. No action required".to_string()
    };

    let why = if coverage_gap > 0 {
        format!(
            "Complexity {} is manageable. Coverage at {:.0}%. Focus on test coverage, not refactoring",
            cyclo,
            coverage_percent * 100.0
        )
    } else {
        format!(
            "Complexity {} with full coverage. Well-maintained function",
            cyclo
        )
    };

    // Generate specific steps based on actual values, no conditional "IF" statements
    let mut steps = vec![];

    if coverage_gap > 20 {
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
        if coverage_gap > 0 {
            steps.push(format!(
                "Focus on adding the {} missing tests",
                tests_needed
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
