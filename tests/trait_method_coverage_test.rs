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

/// Test that trait method coverage is correctly detected when LCOV uses method name only
///
/// This test runs debtmap analysis on the codebase itself with coverage data
/// and verifies that trait implementation methods show correct coverage instead
/// of "no coverage data".
///
/// Tests that trait implementation methods can match with LCOV coverage data
/// even when the function names differ (e.g., "Type::method" in debtmap vs
/// "method" in LCOV). Finds any trait method in the output to validate this.
#[test]
fn test_trait_method_coverage_matching_integration() {
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

    // Find the RecursiveMatchDetector::determine_function_role function
    let items = json
        .get("items")
        .expect("Missing items section")
        .as_array()
        .expect("items should be an array");

    // Find any trait method function in the output to test coverage matching
    // We're specifically looking for impl methods that should have coverage data
    let function_item = items.iter().find(|item| {
        let location = item.get("location");
        if let Some(loc) = location {
            let function = loc.get("function").and_then(|f| f.as_str()).unwrap_or("");

            // Look for any function with "::" which indicates a trait impl method
            // This tests the core functionality: matching trait method names in coverage
            function.contains("::")
        } else {
            false
        }
    });

    // If no trait methods are in the output, skip the test
    // This can happen if all functions are below complexity thresholds (spec 201)
    if function_item.is_none() {
        println!(
            "Skipping test: no trait method implementations found in analysis output. \
             This is expected if all functions are below complexity thresholds."
        );
        return;
    }

    let item = function_item.unwrap();

    // Extract coverage information
    let metrics = item
        .get("metrics")
        .expect("Debt item missing 'metrics' field");

    let function_name = item
        .get("location")
        .and_then(|l| l.get("function"))
        .and_then(|f| f.as_str())
        .unwrap_or("unknown");

    // Check if coverage data exists
    // The key assertion: coverage should be detected (not null/missing)
    let coverage = metrics.get("coverage");

    assert!(
        coverage.is_some(),
        "Coverage data missing for trait method {} - \
         trait method coverage matching may have failed",
        function_name
    );

    let coverage_value = coverage.unwrap();

    // Coverage should not be null (which would indicate "no coverage data")
    assert!(
        !coverage_value.is_null(),
        "Coverage is null for trait method {} - \
         should show actual coverage via name variant matching",
        function_name
    );

    // If coverage is a number, verify it's reasonable
    if let Some(cov_pct) = coverage_value.as_f64() {
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
}

/// Test that the explain-coverage tool can find trait method coverage using method name
///
/// This is a more direct test that the coverage lookup mechanism works correctly.
#[test]
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

/// Test that regular (non-trait) methods still work correctly
///
/// This ensures the name variant matching doesn't break existing functionality.
#[test]
fn test_non_trait_method_coverage_still_works() {
    // Skip if coverage file doesn't exist
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

    // Count functions with coverage data
    let items = json
        .get("items")
        .expect("Missing items section")
        .as_array()
        .expect("items should be an array");

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
