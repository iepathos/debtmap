//! Integration tests for JSON output format.
//!
//! Note: The legacy JSON format was removed in spec 202.
//! JSON output now always uses the unified format with consistent structure.

#![allow(deprecated)] // cargo_bin deprecation - tests are ignored anyway

use assert_cmd::cargo::CommandCargoExt;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

/// Test that --format json generates valid unified format output
#[test]
#[ignore = "requires pre-built binary, run with --ignored"]
fn test_cli_output_format_unified_produces_valid_structure() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("unified_output.json");

    // Use the sample codebase fixture for analysis
    let test_codebase =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/data/fixtures/sample_codebase");

    // Run debtmap analyze with --format json (always uses unified format now)
    let output = Command::cargo_bin("debtmap")
        .unwrap()
        .args([
            "analyze",
            "--format",
            "json",
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
    let output_content = fs::read_to_string(&output_path).expect("Failed to read output file");

    let json: Value = serde_json::from_str(&output_content).expect("Output is not valid JSON");

    // Validate top-level structure
    assert!(json.get("metadata").is_some(), "Missing metadata section");
    assert!(json.get("items").is_some(), "Missing items section");
    assert!(json.get("summary").is_some(), "Missing summary section");

    // Validate metadata structure
    let metadata = json.get("metadata").unwrap();
    assert!(
        metadata.get("debtmap_version").is_some(),
        "Missing metadata.debtmap_version"
    );
    assert!(
        metadata.get("generated_at").is_some(),
        "Missing metadata.generated_at"
    );
    assert!(
        metadata.get("analysis_type").is_some(),
        "Missing metadata.analysis_type"
    );

    // Validate items is an array
    let items = json.get("items").unwrap();
    assert!(items.is_array(), "items should be an array");

    // If there are debt items, validate their structure
    if let Some(item_array) = items.as_array()
        && !item_array.is_empty()
    {
        let first_item = &item_array[0];

        // Check required fields per spec 108
        assert!(
            first_item.get("type").is_some(),
            "Debt item missing 'type' field"
        );
        assert!(
            first_item.get("location").is_some(),
            "Debt item missing 'location' field"
        );
        assert!(
            first_item.get("category").is_some(),
            "Debt item missing 'category' field"
        );
        assert!(
            first_item.get("priority").is_some(),
            "Debt item missing 'priority' field"
        );
        assert!(
            first_item.get("score").is_some(),
            "Debt item missing 'score' field"
        );

        // Validate type is either "File" or "Function"
        let item_type = first_item.get("type").unwrap().as_str().unwrap();
        assert!(
            item_type == "File" || item_type == "Function",
            "Type must be 'File' or 'Function', got: {}",
            item_type
        );

        // Validate location structure
        let location = first_item.get("location").unwrap();
        assert!(location.get("file").is_some(), "Location missing 'file'");

        // Validate priority is one of the expected values
        let priority = first_item.get("priority").unwrap().as_str().unwrap();
        assert!(
            priority == "high" || priority == "medium" || priority == "low",
            "Priority must be 'high', 'medium', or 'low', got: {}",
            priority
        );

        // Validate score is a number
        assert!(
            first_item.get("score").unwrap().is_number(),
            "Score should be a number"
        );
    }

    // Validate summary structure
    let summary = json.get("summary").unwrap();
    assert!(
        summary.get("total_items").is_some(),
        "Summary missing 'total_items'"
    );

    if summary.get("by_category").is_some() {
        assert!(
            summary.get("by_category").unwrap().is_object(),
            "by_category should be an object"
        );
    }

    if summary.get("by_severity").is_some() {
        assert!(
            summary.get("by_severity").unwrap().is_object(),
            "by_severity should be an object"
        );
    }
}

/// Test that unified format can be parsed and filtered by scope
#[test]
#[ignore = "requires pre-built binary, run with --ignored"]
fn test_cli_unified_format_scope_filtering() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("unified_output.json");

    let test_codebase =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/data/fixtures/sample_codebase");

    // Run analysis (JSON output always uses unified format)
    let output = Command::cargo_bin("debtmap")
        .unwrap()
        .args([
            "analyze",
            "--format",
            "json",
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

    // Test that items can be filtered by type
    let items = json.get("items").unwrap().as_array().unwrap();

    let file_items: Vec<_> = items
        .iter()
        .filter(|item| item.get("type").unwrap().as_str().unwrap() == "File")
        .collect();

    let function_items: Vec<_> = items
        .iter()
        .filter(|item| item.get("type").unwrap().as_str().unwrap() == "Function")
        .collect();

    // All items should be categorized as either File or Function
    assert_eq!(
        file_items.len() + function_items.len(),
        items.len(),
        "All items should have type of 'File' or 'Function'"
    );
}

/// Test that unified format includes proper metric data
#[test]
#[ignore = "requires pre-built binary, run with --ignored"]
fn test_cli_unified_format_metrics_presence() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("unified_output.json");

    let test_codebase =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/data/fixtures/sample_codebase");

    // Run analysis (JSON output always uses unified format)
    let output = Command::cargo_bin("debtmap")
        .unwrap()
        .args([
            "analyze",
            "--format",
            "json",
            "--output",
            output_path.to_str().unwrap(),
            test_codebase.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to execute debtmap command");

    assert!(output.status.success(), "Command should succeed");

    let output_content = fs::read_to_string(&output_path).unwrap();
    let json: Value = serde_json::from_str(&output_content).unwrap();

    let items = json.get("items").unwrap().as_array().unwrap();

    // Check that each item has a score field (unified metric)
    for item in items {
        let score = item.get("score").expect("Each item should have score");
        assert!(score.is_number(), "Score should be a number");

        // Items may have additional metrics in details
        if let Some(details) = item.get("details") {
            assert!(details.is_object(), "Details should be an object");
        }
    }
}

// Note: test_cli_default_output_format_is_legacy removed in spec 202
// Legacy JSON format has been removed - unified format is now the only format

#[test]
#[ignore = "requires pre-built binary, run with --ignored"]
fn test_cli_go_json_output_contains_go_function_location() {
    let temp_dir = TempDir::new().unwrap();
    write_go_fixture(temp_dir.path());
    let output_path = temp_dir.path().join("go_output.json");

    let output = Command::cargo_bin("debtmap")
        .unwrap()
        .args([
            "analyze",
            "--format",
            "json",
            "--languages",
            "go",
            "--threshold-complexity",
            "1",
            "--output",
            output_path.to_str().unwrap(),
            temp_dir.path().to_str().unwrap(),
        ])
        .output()
        .expect("Failed to execute debtmap command");

    assert_command_success(output);

    let output_content = fs::read_to_string(&output_path).expect("Failed to read output file");
    let json: Value = serde_json::from_str(&output_content).expect("Output is not valid JSON");
    let items = json["items"].as_array().expect("items should be an array");

    assert!(
        items.iter().any(is_go_function_item),
        "expected Go function debt item in JSON output: {}",
        output_content
    );
}

#[test]
#[ignore = "requires pre-built binary, run with --ignored"]
fn test_cli_go_markdown_output_contains_function_name() {
    let temp_dir = TempDir::new().unwrap();
    write_go_fixture(temp_dir.path());
    let output_path = temp_dir.path().join("go_output.md");

    let output = Command::cargo_bin("debtmap")
        .unwrap()
        .args([
            "analyze",
            "--format",
            "markdown",
            "--languages",
            "golang",
            "--threshold-complexity",
            "1",
            "--output",
            output_path.to_str().unwrap(),
            temp_dir.path().to_str().unwrap(),
        ])
        .output()
        .expect("Failed to execute debtmap command");

    assert_command_success(output);

    let markdown = fs::read_to_string(&output_path).expect("Failed to read markdown output file");
    assert!(markdown.contains("Decide"), "markdown output: {markdown}");
    assert!(
        markdown.contains("service.go"),
        "markdown output: {markdown}"
    );
}

fn write_go_fixture(root: &Path) {
    fs::write(
        root.join("service.go"),
        r#"package service

func Decide(value int) int {
    if value == 1 {
        return 1
    }
    if value == 2 {
        return 2
    }
    if value == 3 {
        return 3
    }
    if value == 4 {
        return 4
    }
    if value == 5 {
        return 5
    }
    if value == 6 {
        return 6
    }
    if value == 7 {
        return 7
    }
    if value == 8 {
        return 8
    }
    if value == 9 {
        return 9
    }
    if value == 10 {
        return 10
    }
    if value == 11 {
        return 11
    }
    if value == 12 {
        return 12
    }
    return 0
}
"#,
    )
    .expect("Failed to write Go fixture");
}

fn assert_command_success(output: std::process::Output) {
    if !output.status.success() {
        eprintln!("stdout: {}", String::from_utf8_lossy(&output.stdout));
        eprintln!("stderr: {}", String::from_utf8_lossy(&output.stderr));
        panic!("debtmap analyze command failed");
    }
}

fn is_go_function_item(item: &Value) -> bool {
    item["type"] == "Function"
        && item["location"]["file"]
            .as_str()
            .map(|file| file.ends_with("service.go"))
            .unwrap_or(false)
        && item["location"]["function"] == "Decide"
}
