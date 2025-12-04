//! Integration test for spec 201: Prevent generation of "no action needed" items
//!
//! This test verifies that clean dispatchers (with inline_logic_branches == 0)
//! do NOT appear in the final output in any format (terminal, JSON, markdown).

use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

/// Test that clean dispatchers don't appear in JSON output
#[test]
fn test_clean_dispatcher_not_in_json_output() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("output.json");

    // Use the clean dispatcher fixture
    let test_codebase =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/data/fixtures/clean_dispatcher");

    // Run debtmap analyze with JSON output
    let output = Command::new("cargo")
        .args([
            "run",
            "--bin",
            "debtmap",
            "--quiet",
            "--",
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

    // Get items array
    let items = json
        .get("items")
        .expect("Missing items section")
        .as_array()
        .expect("items should be an array");

    // Check that NO items reference the clean dispatcher function
    for item in items {
        // Check description/recommendation for "no action needed" patterns
        if let Some(recommendation) = item.get("recommendation") {
            let rec_str = recommendation.to_string().to_lowercase();

            // These patterns should NOT appear in recommendations
            assert!(
                !rec_str.contains("no action needed"),
                "Found 'no action needed' in recommendation: {}",
                recommendation
            );
            assert!(
                !rec_str.contains("maintain current"),
                "Found 'maintain current' for dispatcher in: {}",
                recommendation
            );
        }

        // Check location to see if it's the clean dispatcher
        if let Some(location) = item.get("location") {
            if let Some(function_name) = location.get("function") {
                let func_str = function_name.as_str().unwrap();

                // If this is the handle_command function (our clean dispatcher)
                if func_str == "handle_command" {
                    // It should NOT be present at all for being a clean dispatcher
                    // Only other debt types (like testing gaps) might be valid
                    if let Some(category) = item.get("category") {
                        let cat_str = category.as_str().unwrap();

                        // If it's categorized as complexity, that's wrong for a clean dispatcher
                        assert!(
                            cat_str != "complexity" && cat_str != "maintainability",
                            "Clean dispatcher handle_command should not appear as complexity debt: {:?}",
                            item
                        );
                    }
                }
            }
        }
    }
}

/// Test that clean dispatchers don't appear in terminal output
#[test]
fn test_clean_dispatcher_not_in_terminal_output() {
    let test_codebase =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/data/fixtures/clean_dispatcher");

    // Run debtmap analyze with default terminal output
    let output = Command::new("cargo")
        .args([
            "run",
            "--bin",
            "debtmap",
            "--quiet",
            "--",
            "analyze",
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

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Verify no "no action needed" patterns appear in output
    assert!(
        !stdout.to_lowercase().contains("no action needed"),
        "Terminal output contains 'no action needed': {}",
        stdout
    );

    // If handle_command appears in output, it should not be for being a clean dispatcher
    if stdout.contains("handle_command") {
        // Check that it's not labeled as a complexity issue with low/no action
        let lines: Vec<&str> = stdout.lines().collect();
        for (i, line) in lines.iter().enumerate() {
            if line.contains("handle_command") {
                // Check surrounding lines for "no action needed" or "maintain current"
                let context_start = i.saturating_sub(2);
                let context_end = (i + 3).min(lines.len());
                let context = lines[context_start..context_end].join("\n");

                assert!(
                    !context.to_lowercase().contains("no action needed"),
                    "Found 'no action needed' near handle_command: {}",
                    context
                );
            }
        }
    }
}

/// Test that clean dispatchers don't appear in markdown output
#[test]
fn test_clean_dispatcher_not_in_markdown_output() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("output.md");

    let test_codebase =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/data/fixtures/clean_dispatcher");

    // Run debtmap analyze with markdown output
    let output = Command::new("cargo")
        .args([
            "run",
            "--bin",
            "debtmap",
            "--quiet",
            "--",
            "analyze",
            "--format",
            "markdown",
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

    // Read the output file
    let output_content =
        fs::read_to_string(&output_path).expect("Failed to read markdown output file");

    // Verify no "no action needed" patterns appear
    assert!(
        !output_content.to_lowercase().contains("no action needed"),
        "Markdown output contains 'no action needed': {}",
        output_content
    );

    // If handle_command appears, ensure it's not for being a clean dispatcher
    if output_content.contains("handle_command") {
        let lines: Vec<&str> = output_content.lines().collect();
        for (i, line) in lines.iter().enumerate() {
            if line.contains("handle_command") {
                let context_start = i.saturating_sub(2);
                let context_end = (i + 3).min(lines.len());
                let context = lines[context_start..context_end].join("\n");

                assert!(
                    !context.to_lowercase().contains("no action needed"),
                    "Found 'no action needed' near handle_command in markdown: {}",
                    context
                );
            }
        }
    }
}
