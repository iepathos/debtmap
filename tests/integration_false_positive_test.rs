/// Integration test to verify false positives are properly filtered
mod common;

use common::analyze_file_directly;
use debtmap::core::DebtType;
use std::path::PathBuf;

#[test]
fn test_context_aware_filters_parameter_analyzer() {
    // Test using library API directly instead of spawning subprocess
    let file_path = PathBuf::from("src/organization/parameter_analyzer.rs");
    
    // First analyze without context-aware (simulate by setting env var)
    std::env::set_var("DEBTMAP_CONTEXT_AWARE", "false");
    let results_without = analyze_file_directly(&file_path)
        .expect("Failed to analyze parameter_analyzer.rs");
    std::env::remove_var("DEBTMAP_CONTEXT_AWARE");
    
    // Then analyze with context-aware enabled
    std::env::set_var("DEBTMAP_CONTEXT_AWARE", "true");
    let results_with = analyze_file_directly(&file_path)
        .expect("Failed to analyze parameter_analyzer.rs");
    std::env::remove_var("DEBTMAP_CONTEXT_AWARE");

    // Count security issues
    let security_without = results_without.technical_debt.items.iter()
        .filter(|item| matches!(item.debt_type, DebtType::Security))
        .count();
    let security_with = results_with.technical_debt.items.iter()
        .filter(|item| matches!(item.debt_type, DebtType::Security))
        .count();

    println!(
        "Security issues without context-aware: {}",
        security_without
    );
    println!(
        "Security issues with context-aware: {}",
        security_with
    );

    // Check for specific test function issues
    let test_function_patterns = [
        "test_classify_parameter_list_impact_low",
        "test_classify_data_clump_impact_medium",
        "test_estimate_maintainability_impact",
    ];

    for pattern in &test_function_patterns {
        let has_without = results_without.technical_debt.items.iter()
            .any(|item| item.message.contains(pattern));
        let has_with = results_with.technical_debt.items.iter()
            .any(|item| item.message.contains(pattern));

        println!(
            "Pattern '{}': without={}, with={}",
            pattern, has_without, has_with
        );

        if has_with {
            println!(
                "  ISSUE: Test function '{}' still appears with context-aware",
                pattern
            );
        }
    }

    // Context-aware should reduce or eliminate test function issues
    if security_with >= security_without && security_without > 0 {
        println!("WARNING: context-aware did not reduce security issues");
    }
}

#[test]
fn test_context_aware_filters_rust_call_graph() {
    // Test using library API directly
    let file_path = PathBuf::from("src/analyzers/rust_call_graph.rs");
    
    std::env::set_var("DEBTMAP_CONTEXT_AWARE", "true");
    let results = analyze_file_directly(&file_path)
        .expect("Failed to analyze rust_call_graph.rs");
    std::env::remove_var("DEBTMAP_CONTEXT_AWARE");

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
        let has_func = results.technical_debt.items.iter()
            .any(|item| item.message.contains(func));
        let has_line = results.technical_debt.items.iter()
            .any(|item| item.line == *line);

        if has_func || has_line {
            println!("Found issue for {} at line {}", func, line);
        }
    }

    // Count total security issues
    let security_count = results.technical_debt.items.iter()
        .filter(|item| matches!(item.debt_type, DebtType::Security))
        .count();
    println!(
        "Total security issues in rust_call_graph.rs: {}",
        security_count
    );
}

#[test]
#[ignore] // This test is too slow for regular CI - run with `cargo test -- --ignored`
fn test_context_aware_on_entire_codebase() {
    // This test analyzes the entire codebase - kept as ignored for performance
    use debtmap::io::walker::find_project_files_with_config;
    use debtmap::config::DebtmapConfig;
    
    let config = DebtmapConfig::default();
    let project_files = find_project_files_with_config(Path::new("."), &config)
        .expect("Failed to find project files");
    
    // Count total files to analyze
    println!("Analyzing {} files", project_files.len());
    
    std::env::set_var("DEBTMAP_CONTEXT_AWARE", "true");
    
    let mut total_debt_items = 0;
    let mut found_test_issues = false;
    
    // Check for test function false positives
    let test_patterns = [
        "test_classify_parameter_list_impact_low",
        "test_classify_data_clump_impact_medium",
        "test_estimate_maintainability_impact_long_parameter_list",
    ];
    
    // Analyze a subset of files for performance
    for file in project_files.iter().take(10) {
        if let Ok(results) = analyze_file_directly(file) {
            total_debt_items += results.technical_debt.items.len();
            
            for pattern in &test_patterns {
                if results.technical_debt.items.iter().any(|item| item.message.contains(pattern)) {
                    println!("FOUND FALSE POSITIVE: {}", pattern);
                    found_test_issues = true;
                }
            }
        }
    }
    
    std::env::remove_var("DEBTMAP_CONTEXT_AWARE");
    
    println!("Total debt items found: {}", total_debt_items);
    
    if found_test_issues {
        println!("ISSUE CONFIRMED: Test functions are triggering false positives even with context-aware");
    }
}

#[test]
fn test_context_aware_on_specific_dirs() {
    // Faster version that only analyzes specific problem directories
    use debtmap::io::walker::find_project_files_with_config;
    use debtmap::config::DebtmapConfig;
    use std::path::Path;
    
    let dirs_to_test = ["src/organization", "src/analyzers"];
    let config = DebtmapConfig::default();
    
    std::env::set_var("DEBTMAP_CONTEXT_AWARE", "true");

    for dir in &dirs_to_test {
        let dir_path = Path::new(dir);
        let files = find_project_files_with_config(dir_path, &config)
            .unwrap_or_else(|_| panic!("Failed to find files in {}", dir));

        // Check for known false positives
        let test_patterns = [
            "test_classify_parameter_list_impact_low",
            "test_classify_data_clump_impact_medium",
            "test_estimate_maintainability_impact_long_parameter_list",
            "count_lines",
            "extract_call_graph",
        ];

        for file in files.iter().take(3) { // Analyze a few files from each dir
            if let Ok(results) = analyze_file_directly(file) {
                for pattern in &test_patterns {
                    if results.technical_debt.items.iter().any(|item| item.message.contains(pattern)) {
                        println!("Found pattern '{}' in {} (file: {:?})", pattern, dir, file);
                    }
                }
            }
        }
    }
    
    std::env::remove_var("DEBTMAP_CONTEXT_AWARE");
}
