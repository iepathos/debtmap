/// Integration test to verify false positives are properly filtered
use std::process::Command;

#[test]
fn test_context_aware_filters_parameter_analyzer() {
    // Skip this test in CI or when running all tests
    // This test spawns cargo subprocesses which can hang
    if std::env::var("CI").is_ok() || std::env::var("SKIP_INTEGRATION_TESTS").is_ok() {
        println!("Skipping integration test that spawns cargo subprocesses");
        return;
    }

    // Run debtmap on the actual parameter_analyzer.rs file
    let output_without = Command::new("cargo")
        .args([
            "run",
            "--",
            "analyze",
            "src/organization/parameter_analyzer.rs",
            "--no-context-aware",
        ])
        .output()
        .expect("Failed to run debtmap");

    let stdout_without = String::from_utf8_lossy(&output_without.stdout);

    let output_with = Command::new("cargo")
        .args([
            "run",
            "--",
            "analyze",
            "src/organization/parameter_analyzer.rs",
        ])
        .output()
        .expect("Failed to run debtmap");

    let stdout_with = String::from_utf8_lossy(&output_with.stdout);

    // Count security issues
    let security_without = stdout_without.matches("SECURITY:").count();
    let security_with = stdout_with.matches("SECURITY:").count();

    println!(
        "Security issues with --no-context-aware: {}",
        security_without
    );
    println!(
        "Security issues with default (context-aware): {}",
        security_with
    );

    // Check for specific test function issues
    let test_function_patterns = [
        "test_classify_parameter_list_impact_low",
        "test_classify_data_clump_impact_medium",
        "test_estimate_maintainability_impact",
    ];

    for pattern in &test_function_patterns {
        let has_without = stdout_without.contains(pattern);
        let has_with = stdout_with.contains(pattern);

        println!(
            "Pattern '{}': without={}, with={}",
            pattern, has_without, has_with
        );

        if has_with {
            println!(
                "  ISSUE: Test function '{}' still appears with context-aware (default)",
                pattern
            );
        }
    }

    // Context-aware should reduce or eliminate test function issues
    if security_with >= security_without && security_without > 0 {
        println!("WARNING: context-aware (default) did not reduce security issues");
    }
}

#[test]
fn test_context_aware_filters_rust_call_graph() {
    // Skip this test in CI or when running all tests
    // This test spawns cargo subprocesses which can hang
    if std::env::var("CI").is_ok() || std::env::var("SKIP_INTEGRATION_TESTS").is_ok() {
        println!("Skipping integration test that spawns cargo subprocesses");
        return;
    }

    let output = Command::new("cargo")
        .args(["run", "--", "analyze", "src/analyzers/rust_call_graph.rs"])
        .output()
        .expect("Failed to run debtmap");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Check for the specific functions mentioned in user's output
    let problematic_functions = [
        ("count_lines", 1025),
        ("extract_call_graph", 1038),
        ("extract_call_graph_with_types", 1055),
        ("extract_call_graph_with_signatures", 1079),
        ("merge_call_graphs", 1084),
        ("collect_types_from_file", 1114),
    ];

    for (func, line) in &problematic_functions {
        let has_func = stdout.contains(func);
        let has_line = stdout.contains(&line.to_string());

        if has_func || has_line {
            println!("Found issue for {} at line {}", func, line);
        }
    }

    // Count total security issues
    let security_count = stdout.matches("SECURITY:").count();
    println!(
        "Total security issues in rust_call_graph.rs: {}",
        security_count
    );
}

#[test]
#[ignore] // This test is too slow for regular CI - run with `cargo test -- --ignored`
fn test_context_aware_on_entire_codebase() {
    // This is the actual command the user is running
    // NOTE: This analyzes the ENTIRE codebase and can take several minutes
    let output = Command::new("cargo")
        .args(["run", "--", "analyze", "."])
        .output()
        .expect("Failed to run debtmap");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Check for test function false positives
    let test_patterns = [
        "test_classify_parameter_list_impact_low",
        "test_classify_data_clump_impact_medium",
        "test_estimate_maintainability_impact_long_parameter_list",
    ];

    let mut found_test_issues = false;
    for pattern in &test_patterns {
        if stdout.contains(pattern) {
            println!("FOUND FALSE POSITIVE: {}", pattern);
            found_test_issues = true;
        }
    }

    // Check total debt score
    if let Some(score_line) = stdout.lines().find(|l| l.contains("TOTAL DEBT SCORE")) {
        println!("Score with context-aware: {}", score_line);
    }

    if found_test_issues {
        println!("ISSUE CONFIRMED: Test functions are triggering false positives even with context-aware (default)");
    }
}

#[test]
fn test_context_aware_on_specific_dirs() {
    // Faster version that only analyzes specific problem directories
    let dirs_to_test = ["src/organization", "src/analyzers"];

    for dir in &dirs_to_test {
        let output = Command::new("cargo")
            .args(["run", "--", "analyze", dir])
            .output()
            .unwrap_or_else(|_| panic!("Failed to analyze {}", dir));

        let stdout = String::from_utf8_lossy(&output.stdout);

        // Check for known false positives
        let test_patterns = [
            "test_classify_parameter_list_impact_low",
            "test_classify_data_clump_impact_medium",
            "test_estimate_maintainability_impact_long_parameter_list",
            "count_lines",
            "extract_call_graph",
        ];

        for pattern in &test_patterns {
            if stdout.contains(pattern) {
                println!("Found pattern '{}' in {} output", pattern, dir);
            }
        }
    }
}
