use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

/// Test that --output-format unified flag generates valid unified format output
#[test]
fn test_cli_output_format_unified_produces_valid_structure() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("unified_output.json");

    // Use the sample codebase fixture for analysis
    let test_codebase = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/data/fixtures/sample_codebase");

    // Run debtmap analyze with --output-format unified
    let output = Command::new("cargo")
        .args([
            "run",
            "--bin",
            "debtmap",
            "--quiet",
            "--",
            "analyze",
            "--output-format",
            "unified",
            "--output",
            output_path.to_str().unwrap(),
            test_codebase.to_str().unwrap(),
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
    let output_content = fs::read_to_string(&output_path)
        .expect("Failed to read output file");

    let json: Value = serde_json::from_str(&output_content)
        .expect("Output is not valid JSON");

    // Validate top-level structure
    assert!(json.get("metadata").is_some(), "Missing metadata section");
    assert!(json.get("debt_items").is_some(), "Missing debt_items section");
    assert!(json.get("summary").is_some(), "Missing summary section");

    // Validate metadata structure
    let metadata = json.get("metadata").unwrap();
    assert!(metadata.get("version").is_some(), "Missing metadata.version");
    assert!(metadata.get("timestamp").is_some(), "Missing metadata.timestamp");
    assert!(metadata.get("analysis_config").is_some(), "Missing metadata.analysis_config");

    // Validate debt_items is an array
    let debt_items = json.get("debt_items").unwrap();
    assert!(debt_items.is_array(), "debt_items should be an array");

    // If there are debt items, validate their structure
    if let Some(items) = debt_items.as_array() {
        if !items.is_empty() {
            let first_item = &items[0];

            // Check required fields
            assert!(first_item.get("scope").is_some(), "Debt item missing 'scope' field");
            assert!(first_item.get("location").is_some(), "Debt item missing 'location' field");
            assert!(first_item.get("category").is_some(), "Debt item missing 'category' field");
            assert!(first_item.get("severity").is_some(), "Debt item missing 'severity' field");
            assert!(first_item.get("metrics").is_some(), "Debt item missing 'metrics' field");

            // Validate scope is either "file" or "function"
            let scope = first_item.get("scope").unwrap().as_str().unwrap();
            assert!(
                scope == "file" || scope == "function",
                "Scope must be 'file' or 'function', got: {}",
                scope
            );

            // Validate location structure
            let location = first_item.get("location").unwrap();
            assert!(location.get("file_path").is_some(), "Location missing 'file_path'");

            // Validate severity is one of the expected values
            let severity = first_item.get("severity").unwrap().as_str().unwrap();
            assert!(
                severity == "high" || severity == "medium" || severity == "low",
                "Severity must be 'high', 'medium', or 'low', got: {}",
                severity
            );

            // Validate metrics is an object
            assert!(first_item.get("metrics").unwrap().is_object(), "Metrics should be an object");
        }
    }

    // Validate summary structure
    let summary = json.get("summary").unwrap();
    assert!(summary.get("total_items").is_some(), "Summary missing 'total_items'");

    if summary.get("by_category").is_some() {
        assert!(summary.get("by_category").unwrap().is_object(), "by_category should be an object");
    }

    if summary.get("by_severity").is_some() {
        assert!(summary.get("by_severity").unwrap().is_object(), "by_severity should be an object");
    }
}

/// Test that unified format can be parsed and filtered by scope
#[test]
fn test_cli_unified_format_scope_filtering() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("unified_output.json");

    let test_codebase = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/data/fixtures/sample_codebase");

    // Run analysis
    let output = Command::new("cargo")
        .args([
            "run",
            "--bin",
            "debtmap",
            "--quiet",
            "--",
            "analyze",
            "--output-format",
            "unified",
            "--output",
            output_path.to_str().unwrap(),
            test_codebase.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to execute debtmap command");

    assert!(output.status.success(), "Command should succeed");

    // Read and parse
    let output_content = fs::read_to_string(&output_path).unwrap();
    let json: Value = serde_json::from_str(&output_content).unwrap();

    // Test that items can be filtered by scope
    let debt_items = json.get("debt_items").unwrap().as_array().unwrap();

    let file_items: Vec<_> = debt_items
        .iter()
        .filter(|item| item.get("scope").unwrap().as_str().unwrap() == "file")
        .collect();

    let function_items: Vec<_> = debt_items
        .iter()
        .filter(|item| item.get("scope").unwrap().as_str().unwrap() == "function")
        .collect();

    // All items should be categorized as either file or function
    assert_eq!(
        file_items.len() + function_items.len(),
        debt_items.len(),
        "All items should have scope of 'file' or 'function'"
    );
}

/// Test that unified format includes proper metric data
#[test]
fn test_cli_unified_format_metrics_presence() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("unified_output.json");

    let test_codebase = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/data/fixtures/sample_codebase");

    // Run analysis
    let output = Command::new("cargo")
        .args([
            "run",
            "--bin",
            "debtmap",
            "--quiet",
            "--",
            "analyze",
            "--output-format",
            "unified",
            "--output",
            output_path.to_str().unwrap(),
            test_codebase.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to execute debtmap command");

    assert!(output.status.success(), "Command should succeed");

    let output_content = fs::read_to_string(&output_path).unwrap();
    let json: Value = serde_json::from_str(&output_content).unwrap();

    let debt_items = json.get("debt_items").unwrap().as_array().unwrap();

    // Check that metrics are present and contain expected fields
    for item in debt_items {
        let metrics = item.get("metrics").expect("Each item should have metrics");
        assert!(metrics.is_object(), "Metrics should be an object");

        // Metrics should have at least one of the common metric fields
        let has_complexity = metrics.get("cyclomatic_complexity").is_some();
        let has_cognitive = metrics.get("cognitive_complexity").is_some();
        let has_loc = metrics.get("lines_of_code").is_some();
        let has_risk = metrics.get("risk_score").is_some();

        assert!(
            has_complexity || has_cognitive || has_loc || has_risk,
            "Metrics should contain at least one standard metric field"
        );
    }
}

/// Test that default output format is unified
#[test]
fn test_cli_default_output_format_is_unified() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("default_output.json");

    let test_codebase = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/data/fixtures/sample_codebase");

    // Run without explicit --output-format flag
    let output = Command::new("cargo")
        .args([
            "run",
            "--bin",
            "debtmap",
            "--quiet",
            "--",
            "analyze",
            "--output",
            output_path.to_str().unwrap(),
            test_codebase.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to execute debtmap command");

    assert!(output.status.success(), "Command should succeed");

    let output_content = fs::read_to_string(&output_path).unwrap();
    let json: Value = serde_json::from_str(&output_content).unwrap();

    // Should have unified format structure
    assert!(
        json.get("metadata").is_some() &&
        json.get("debt_items").is_some() &&
        json.get("summary").is_some(),
        "Default output should use unified format"
    );
}
