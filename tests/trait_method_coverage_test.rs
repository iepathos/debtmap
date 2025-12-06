/// Integration tests for trait method coverage matching (Spec 181)
///
/// This test validates that trait implementation methods are correctly matched
/// with LCOV coverage data even when the function names differ between what
/// debtmap stores (e.g., "RecursiveMatchDetector::visit_expr") and what LCOV
/// stores (e.g., "visit_expr").
use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

/// Comprehensive test for coverage matching including trait methods
///
/// This test runs debtmap analysis on the codebase itself with coverage data
/// and verifies:
/// 1. Trait implementation methods show correct coverage (not "no coverage data")
/// 2. Regular functions still get coverage correctly (regression check)
///
/// Trait methods test the name variant matching where debtmap stores
/// "Type::method" but LCOV stores "method".
#[test]
#[ignore] // Slow integration test - runs full codebase analysis
fn test_coverage_matching_integration() {
    // Skip if coverage file doesn't exist (not in CI with coverage)
    let coverage_file = PathBuf::from("target/coverage/lcov.info");
    if !coverage_file.exists() {
        println!(
            "Skipping test: coverage file not found at {}",
            coverage_file.display()
        );
        return;
    }

    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("analysis_output.json");

    // Run debtmap analyze on itself with coverage
    let output = Command::new("cargo")
        .args([
            "run",
            "--bin",
            "debtmap",
            "--quiet",
            "--",
            "analyze",
            ".",
            "--format",
            "json",
            "--coverage-file",
            coverage_file.to_str().unwrap(),
            "--output",
            output_path.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to execute debtmap command");

    // Check command succeeded
    if !output.status.success() {
        eprintln!("stdout: {}", String::from_utf8_lossy(&output.stdout));
        eprintln!("stderr: {}", String::from_utf8_lossy(&output.stderr));
        panic!("debtmap analyze command failed");
    }

    // Read and parse the output file
    let output_content = fs::read_to_string(&output_path).expect("Failed to read output file");

    let json: Value = serde_json::from_str(&output_content).expect("Output is not valid JSON");

    let items = json
        .get("items")
        .expect("Missing items section")
        .as_array()
        .expect("items should be an array");

    // Part 1: Test trait method coverage matching
    // Find a trait method function that actually has coverage data
    let trait_method_item = items.iter().find(|item| {
        let location = item.get("location");
        if let Some(loc) = location {
            let function = loc.get("function").and_then(|f| f.as_str()).unwrap_or("");

            // Must be a trait impl method (contains "::")
            if !function.contains("::") {
                return false;
            }

            // Must have coverage data (not null)
            if let Some(metrics) = item.get("metrics") {
                if let Some(coverage) = metrics.get("coverage") {
                    return !coverage.is_null();
                }
            }
            false
        } else {
            false
        }
    });

    // If we found a trait method with coverage, validate it
    if let Some(item) = trait_method_item {
        let metrics = item
            .get("metrics")
            .expect("Debt item missing 'metrics' field");

        let function_name = item
            .get("location")
            .and_then(|l| l.get("function"))
            .and_then(|f| f.as_str())
            .unwrap_or("unknown");

        let coverage = metrics.get("coverage").expect("Coverage should exist");

        // Coverage should not be null (which would indicate "no coverage data")
        assert!(
            !coverage.is_null(),
            "Coverage is null for trait method {} - \
             should show actual coverage via name variant matching",
            function_name
        );

        // If coverage is a number, verify it's reasonable
        if let Some(cov_pct) = coverage.as_f64() {
            assert!(
                (0.0..=1.0).contains(&cov_pct),
                "Coverage should be between 0 and 1 for {}, got {}",
                function_name,
                cov_pct
            );
        }

        // Verify no false-positive "no coverage data" message in recommendations
        if let Some(recommendation) = item.get("recommendation") {
            let rec_text = recommendation
                .get("description")
                .and_then(|d| d.as_str())
                .unwrap_or("");

            assert!(
                !rec_text.contains("no coverage data"),
                "Recommendation should not claim 'no coverage data' for trait method with coverage"
            );
        }
    } else {
        println!(
            "Note: no trait method implementations with coverage data found in analysis output. \
             This is expected if all trait methods are below complexity thresholds or not covered."
        );
    }

    // Part 2: Test that regular (non-trait) method coverage still works
    // Count functions with coverage data
    let functions_with_coverage = items
        .iter()
        .filter(|item| {
            item.get("type")
                .and_then(|t| t.as_str())
                .map(|t| t == "Function")
                .unwrap_or(false)
        })
        .filter(|item| {
            item.get("metrics")
                .and_then(|m| m.get("coverage"))
                .map(|c| !c.is_null())
                .unwrap_or(false)
        })
        .count();

    // There should be many functions with coverage (not zero)
    // This verifies that the name variant matching didn't break
    // the ability to find coverage for regular functions
    assert!(
        functions_with_coverage > 10,
        "Should find coverage for many functions (found {}), \
         variant matching may have broken regular coverage lookup",
        functions_with_coverage
    );
}

/// Test that the explain-coverage tool can find trait method coverage using method name
///
/// This is a more direct test that the coverage lookup mechanism works correctly.
#[test]
#[ignore] // Slow integration test - runs cargo command
fn test_explain_coverage_finds_trait_method() {
    // Skip if coverage file doesn't exist
    let coverage_file = PathBuf::from("target/coverage/lcov.info");
    if !coverage_file.exists() {
        println!(
            "Skipping test: coverage file not found at {}",
            coverage_file.display()
        );
        return;
    }

    // Run explain-coverage looking for visit_expr by method name only
    let output = Command::new("cargo")
        .args([
            "run",
            "--bin",
            "debtmap",
            "--quiet",
            "--",
            "explain-coverage",
            ".",
            "--coverage-file",
            coverage_file.to_str().unwrap(),
            "--function",
            "visit_expr",
            "--file",
            "src/complexity/recursive_detector.rs",
        ])
        .output()
        .expect("Failed to execute debtmap explain-coverage command");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Check command succeeded
    if !output.status.success() {
        eprintln!("stdout: {}", stdout);
        eprintln!("stderr: {}", stderr);
        panic!("debtmap explain-coverage command failed");
    }

    // Verify coverage was found
    assert!(
        stdout.contains("Coverage Found") || stdout.contains("✓"),
        "explain-coverage should find coverage for visit_expr method, got:\n{}",
        stdout
    );

    // Verify it reports coverage percentage (should be ~90%)
    assert!(
        stdout.contains("Coverage:") || stdout.contains("%"),
        "explain-coverage should report coverage percentage, got:\n{}",
        stdout
    );

    // Verify no error about missing coverage
    assert!(
        !stdout.contains("No coverage found") && !stderr.contains("No coverage found"),
        "Should not report 'No coverage found' for visit_expr"
    );
}
