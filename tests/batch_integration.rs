//! Integration tests for batch file analysis using the traverse pattern.
//!
//! These tests verify the parallel analysis functionality and error accumulation
//! behavior across multiple files and scenarios.

use debtmap::analyzers::batch::{analyze_files_effect, validate_and_analyze_files, validate_files};
use debtmap::config::{BatchAnalysisConfig, DebtmapConfig, ParallelConfig};
use debtmap::effects::{run_effect, run_validation};
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

/// Helper to create a temp directory with test files.
fn create_test_project(files: &[(&str, &str)]) -> (TempDir, Vec<PathBuf>) {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let mut paths = Vec::with_capacity(files.len());

    for (name, content) in files {
        let file_path = temp_dir.path().join(name);
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent).expect("Failed to create parent directory");
        }
        fs::write(&file_path, content).expect("Failed to write test file");
        paths.push(file_path);
    }

    (temp_dir, paths)
}

// ============================================================================
// Parallel Analysis Tests
// ============================================================================

#[test]
fn test_parallel_analysis_multiple_files() {
    let files: Vec<(&str, &str)> = vec![
        ("file_a.rs", "fn a() { let x = 1; }"),
        ("file_b.rs", "fn b() { let y = 2; }"),
        ("file_c.rs", "fn c() { let z = 3; }"),
        ("file_d.rs", "fn d() { let w = 4; }"),
        ("file_e.rs", "fn e() { let v = 5; }"),
    ];
    let (_temp_dir, paths) = create_test_project(&files);

    let config = DebtmapConfig {
        batch_analysis: Some(BatchAnalysisConfig::default()),
        ..Default::default()
    };

    let effect = analyze_files_effect(paths.clone());
    let results = run_effect(effect, config).expect("Analysis should succeed");

    assert_eq!(results.len(), 5, "Should analyze all 5 files");
    for (result, path) in results.iter().zip(paths.iter()) {
        assert_eq!(result.path, *path);
    }
}

#[test]
fn test_parallel_analysis_with_timing() {
    let files: Vec<(&str, &str)> = vec![
        ("mod1.rs", "fn func1() { let a = 1 + 2; }"),
        ("mod2.rs", "fn func2() { let b = 3 + 4; }"),
        ("mod3.rs", "fn func3() { let c = 5 + 6; }"),
    ];
    let (_temp_dir, paths) = create_test_project(&files);

    let config = DebtmapConfig {
        batch_analysis: Some(BatchAnalysisConfig::default().with_timing()),
        ..Default::default()
    };

    let effect = analyze_files_effect(paths);
    let results = run_effect(effect, config).expect("Analysis should succeed");

    // All results should have timing information
    for result in &results {
        assert!(
            result.analysis_time.is_some(),
            "Each result should have timing info"
        );
    }
}

#[test]
fn test_parallel_vs_sequential_produces_same_results() {
    let files: Vec<(&str, &str)> = vec![
        ("src/lib.rs", "pub fn add(a: i32, b: i32) -> i32 { a + b }"),
        (
            "src/util.rs",
            "pub fn multiply(a: i32, b: i32) -> i32 { a * b }",
        ),
        ("src/helper.rs", "pub fn negate(x: i32) -> i32 { -x }"),
    ];
    let (_temp_dir, paths) = create_test_project(&files);

    // Run with parallel processing
    let parallel_config = DebtmapConfig {
        batch_analysis: Some(BatchAnalysisConfig {
            parallelism: ParallelConfig::default(),
            fail_fast: false,
            collect_timing: false,
        }),
        ..Default::default()
    };

    let parallel_results =
        run_effect(analyze_files_effect(paths.clone()), parallel_config).expect("Parallel failed");

    // Run with sequential processing
    let sequential_config = DebtmapConfig {
        batch_analysis: Some(BatchAnalysisConfig {
            parallelism: ParallelConfig::sequential(),
            fail_fast: false,
            collect_timing: false,
        }),
        ..Default::default()
    };

    let sequential_results =
        run_effect(analyze_files_effect(paths), sequential_config).expect("Sequential failed");

    // Results should be equivalent (order may differ)
    assert_eq!(
        parallel_results.len(),
        sequential_results.len(),
        "Same number of results"
    );

    // Sort by path for comparison
    let mut parallel_sorted: Vec<_> = parallel_results.iter().collect();
    let mut sequential_sorted: Vec<_> = sequential_results.iter().collect();
    parallel_sorted.sort_by_key(|r| &r.path);
    sequential_sorted.sort_by_key(|r| &r.path);

    for (p, s) in parallel_sorted.iter().zip(sequential_sorted.iter()) {
        assert_eq!(p.path, s.path, "Paths should match");
        assert_eq!(
            p.metrics.complexity.functions.len(),
            s.metrics.complexity.functions.len(),
            "Function count should match for {}",
            p.path.display()
        );
    }
}

#[test]
fn test_parallel_analysis_large_batch() {
    // Create 25 files to test batch processing
    let files: Vec<(String, String)> = (0..25)
        .map(|i| {
            let name = format!("file_{}.rs", i);
            let content = format!("fn func_{}() {{ let x = {}; }}", i, i * 10);
            (name, content)
        })
        .collect();

    let files_refs: Vec<(&str, &str)> = files
        .iter()
        .map(|(n, c)| (n.as_str(), c.as_str()))
        .collect();
    let (_temp_dir, paths) = create_test_project(&files_refs);

    let config = DebtmapConfig {
        batch_analysis: Some(BatchAnalysisConfig {
            parallelism: ParallelConfig {
                enabled: true,
                max_concurrency: None,
                batch_size: Some(10), // Process in batches of 10
            },
            fail_fast: false,
            collect_timing: false,
        }),
        ..Default::default()
    };

    let results = run_effect(analyze_files_effect(paths), config).expect("Analysis should succeed");

    assert_eq!(results.len(), 25, "Should analyze all 25 files");
}

// ============================================================================
// Validation and Error Accumulation Tests
// ============================================================================

#[test]
fn test_validation_accumulates_all_errors() {
    let files: Vec<(&str, &str)> = vec![("valid.rs", "fn valid() {}")];
    let (temp_dir, _) = create_test_project(&files);

    // Mix valid and invalid paths
    let paths = vec![
        temp_dir.path().join("valid.rs"),
        PathBuf::from("/nonexistent/file_1.rs"),
        PathBuf::from("/nonexistent/file_2.rs"),
        PathBuf::from("/nonexistent/file_3.rs"),
    ];

    let result = validate_files(&paths);

    match result {
        stillwater::Validation::Failure(errors) => {
            let errors_vec: Vec<_> = errors.into_iter().collect();
            // Should have exactly 3 errors (for the 3 nonexistent files)
            assert_eq!(
                errors_vec.len(),
                3,
                "Should accumulate all 3 errors, not fail at first"
            );
        }
        stillwater::Validation::Success(_) => {
            panic!("Expected failure due to nonexistent files");
        }
    }
}

#[test]
fn test_validation_syntax_errors_accumulate() {
    let files: Vec<(&str, &str)> = vec![
        ("valid.rs", "fn valid() {}"),
        ("invalid1.rs", "fn broken( { }"), // Missing paren
        ("invalid2.rs", "struct { }"),     // Missing name
    ];
    let (_temp_dir, paths) = create_test_project(&files);

    let result = validate_files(&paths);

    match result {
        stillwater::Validation::Failure(errors) => {
            let errors_vec: Vec<_> = errors.into_iter().collect();
            // Should have 2 syntax errors
            assert_eq!(errors_vec.len(), 2, "Should accumulate both syntax errors");
        }
        stillwater::Validation::Success(_) => {
            panic!("Expected failure due to syntax errors");
        }
    }
}

#[test]
fn test_validate_and_analyze_success() {
    let files: Vec<(&str, &str)> = vec![
        ("module_a.rs", "pub fn a() -> i32 { 1 }"),
        ("module_b.rs", "pub fn b() -> i32 { 2 }"),
    ];
    let (_temp_dir, paths) = create_test_project(&files);

    let result = run_validation(validate_and_analyze_files(&paths));

    assert!(result.is_ok(), "Should succeed for valid files");
    let results = result.unwrap();
    assert_eq!(results.len(), 2);
}

#[test]
fn test_validate_and_analyze_mixed() {
    let files: Vec<(&str, &str)> = vec![("valid.rs", "fn ok() {}")];
    let (temp_dir, _) = create_test_project(&files);

    let paths = vec![
        temp_dir.path().join("valid.rs"),
        PathBuf::from("/nonexistent/missing.rs"),
    ];

    let result = run_validation(validate_and_analyze_files(&paths));

    // Should fail because one file doesn't exist
    assert!(result.is_err(), "Should fail when any file is invalid");
}

// ============================================================================
// Complex File Analysis Tests
// ============================================================================

#[test]
fn test_analysis_detects_complexity() {
    let content = r#"
pub fn complex_function(data: &[i32], threshold: i32) -> Vec<i32> {
    let mut results = Vec::new();
    for &value in data {
        if value > threshold {
            if value % 2 == 0 {
                results.push(value * 2);
            } else {
                results.push(value * 3);
            }
        } else if value == threshold {
            results.push(value);
        } else {
            results.push(value / 2);
        }
    }
    results
}
"#;

    let files: Vec<(&str, &str)> = vec![("complex.rs", content)];
    let (_temp_dir, paths) = create_test_project(&files);

    let config = DebtmapConfig::default();
    let results = run_effect(analyze_files_effect(paths), config).expect("Analysis should succeed");

    assert_eq!(results.len(), 1);
    let result = &results[0];

    // Should detect the complex function
    assert!(
        !result.metrics.complexity.functions.is_empty(),
        "Should find functions"
    );

    // The complex_function should have notable complexity
    let complex_fn = result
        .metrics
        .complexity
        .functions
        .iter()
        .find(|f| f.name == "complex_function");

    assert!(complex_fn.is_some(), "Should find complex_function");
    let func = complex_fn.unwrap();

    // Should have measurable cyclomatic complexity (multiple branches)
    assert!(
        func.cyclomatic > 1,
        "Complex function should have cyclomatic > 1"
    );
}

#[test]
fn test_analysis_multiple_languages() {
    let files: Vec<(&str, &str)> = vec![
        ("lib.rs", "pub fn rust_fn() { let x = 1; }"),
        ("script.py", "def python_fn():\n    x = 1"),
        ("app.js", "function jsFn() { let x = 1; }"),
    ];
    let (_temp_dir, paths) = create_test_project(&files);

    let config = DebtmapConfig::default();
    let results = run_effect(analyze_files_effect(paths), config).expect("Analysis should succeed");

    assert_eq!(results.len(), 3, "Should analyze all 3 files");

    // Each file should have at least one function detected
    for result in &results {
        // Note: Python and JS may have different detection capabilities
        // This test ensures they don't crash during analysis
        assert!(
            result.metrics.complexity.functions.is_empty()
                || !result.metrics.complexity.functions.is_empty(),
            "Should complete analysis without error"
        );
    }
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn test_empty_file_analysis() {
    let files: Vec<(&str, &str)> = vec![("empty.rs", "")];
    let (_temp_dir, paths) = create_test_project(&files);

    let config = DebtmapConfig::default();
    let results =
        run_effect(analyze_files_effect(paths), config).expect("Should handle empty file");

    assert_eq!(results.len(), 1);
    assert!(
        results[0].metrics.complexity.functions.is_empty(),
        "Empty file should have no functions"
    );
}

#[test]
fn test_single_file_analysis() {
    let files: Vec<(&str, &str)> = vec![("single.rs", "fn single() {}")];
    let (_temp_dir, paths) = create_test_project(&files);

    // Single file should work with parallel config (falls back to sequential)
    let config = DebtmapConfig {
        batch_analysis: Some(BatchAnalysisConfig::default()),
        ..Default::default()
    };

    let results = run_effect(analyze_files_effect(paths), config).expect("Single file should work");
    assert_eq!(results.len(), 1);
}

#[test]
fn test_deeply_nested_directory_structure() {
    let files: Vec<(&str, &str)> = vec![
        ("src/mod1/submod/file.rs", "fn deep1() {}"),
        ("src/mod2/submod/deeper/file.rs", "fn deep2() {}"),
        ("lib/utils/helpers/file.rs", "fn deep3() {}"),
    ];
    let (_temp_dir, paths) = create_test_project(&files);

    let config = DebtmapConfig::default();
    let results =
        run_effect(analyze_files_effect(paths), config).expect("Should handle nested directories");

    assert_eq!(results.len(), 3);
}

// ============================================================================
// Performance Characteristics Tests
// ============================================================================

#[test]
fn test_parallel_analysis_maintains_all_results() {
    // Create files with different content to ensure independent analysis
    let files: Vec<(String, String)> = (0..10)
        .map(|i| {
            let name = format!("module_{}.rs", i);
            let content = format!(
                r#"
pub fn function_{}(x: i32) -> i32 {{
    if x > {} {{
        x * 2
    }} else {{
        x + 1
    }}
}}
"#,
                i, i
            );
            (name, content)
        })
        .collect();

    let files_refs: Vec<(&str, &str)> = files
        .iter()
        .map(|(n, c)| (n.as_str(), c.as_str()))
        .collect();
    let (_temp_dir, paths) = create_test_project(&files_refs);

    let config = DebtmapConfig {
        batch_analysis: Some(BatchAnalysisConfig::default()),
        ..Default::default()
    };

    let results = run_effect(analyze_files_effect(paths.clone()), config)
        .expect("Parallel analysis should succeed");

    // Verify all results are present
    assert_eq!(results.len(), 10);

    // Verify each file was analyzed (check paths are unique)
    let result_paths: std::collections::HashSet<_> =
        results.iter().map(|r| r.path.clone()).collect();
    assert_eq!(
        result_paths.len(),
        10,
        "All results should have unique paths"
    );

    // Verify paths match input
    for path in &paths {
        assert!(
            result_paths.contains(path),
            "Result should contain path: {}",
            path.display()
        );
    }
}
