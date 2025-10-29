use std::fs;
use std::process::Command;
use tempfile::TempDir;

/// Integration test for the validate-improvement subcommand end-to-end workflow.
/// Tests the complete flow: create comparison file -> run validation -> verify output.
#[test]
fn test_validate_improvement_subcommand_end_to_end() {
    let temp_dir = TempDir::new().unwrap();
    let comparison_path = temp_dir.path().join("comparison.json");
    let output_path = temp_dir.path().join("validation.json");

    // Create a comparison JSON with successful improvement (matching actual structure)
    let comparison_json = r#"{
        "metadata": {
            "comparison_date": "2025-10-28T10:30:00Z",
            "before_file": "before.json",
            "after_file": "after.json",
            "target_location": "src/test.rs:complex_function:10"
        },
        "target_item": {
            "location": "src/test.rs:complex_function:10",
            "before": {
                "score": 85.0,
                "cyclomatic_complexity": 15,
                "cognitive_complexity": 22,
                "coverage": 0.0,
                "function_length": 150,
                "nesting_depth": 4
            },
            "after": {
                "score": 12.0,
                "cyclomatic_complexity": 3,
                "cognitive_complexity": 2,
                "coverage": 0.8,
                "function_length": 30,
                "nesting_depth": 1
            },
            "improvements": {
                "score_reduction_pct": 85.9,
                "complexity_reduction_pct": 90.0,
                "coverage_improvement_pct": 80.0
            },
            "status": "Improved"
        },
        "project_health": {
            "before": {
                "total_debt_score": 1500.0,
                "total_items": 100,
                "critical_items": 10,
                "high_priority_items": 20,
                "average_score": 15.0
            },
            "after": {
                "total_debt_score": 1425.0,
                "total_items": 98,
                "critical_items": 8,
                "high_priority_items": 18,
                "average_score": 14.5
            },
            "changes": {
                "debt_score_change": -75.0,
                "debt_score_change_pct": -5.0,
                "items_change": -2,
                "critical_items_change": -2
            }
        },
        "regressions": [],
        "improvements": [
            {
                "location": "src/test.rs:complex_function:10",
                "before_score": 85.0,
                "after_score": 12.0,
                "improvement_type": "ScoreReduced"
            }
        ],
        "summary": {
            "target_improved": true,
            "new_critical_count": 0,
            "resolved_count": 5,
            "overall_debt_trend": "Improving"
        }
    }"#;

    fs::write(&comparison_path, comparison_json).unwrap();

    // Run the validate-improvement subcommand
    let output = Command::new(env!("CARGO_BIN_EXE_debtmap"))
        .args([
            "validate-improvement",
            "--comparison",
            comparison_path.to_str().unwrap(),
            "--output",
            output_path.to_str().unwrap(),
            "--threshold",
            "75.0",
            "--format",
            "json",
        ])
        .output()
        .expect("Failed to execute validate-improvement command");

    // Verify command succeeded
    assert!(
        output.status.success(),
        "Command failed with status: {:?}\nstdout: {}\nstderr: {}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    // Verify output file was created
    assert!(
        output_path.exists(),
        "Validation output file was not created"
    );

    // Read and verify output content
    let validation_result = fs::read_to_string(&output_path).unwrap();
    let validation: serde_json::Value = serde_json::from_str(&validation_result).unwrap();

    // Verify required fields exist
    assert!(
        validation.get("completion_percentage").is_some(),
        "Missing completion_percentage field"
    );
    assert!(validation.get("status").is_some(), "Missing status field");

    // Verify completion percentage is high (target improved significantly with no regressions)
    let completion = validation["completion_percentage"].as_f64().unwrap();
    assert!(
        completion >= 75.0,
        "Expected completion >= 75%, got {}",
        completion
    );

    // Verify status is "complete"
    let status = validation["status"].as_str().unwrap();
    assert_eq!(
        status, "complete",
        "Expected status 'complete', got '{}'",
        status
    );
}

/// Integration test for validate-improvement with regressions
#[test]
fn test_validate_improvement_with_regressions() {
    let temp_dir = TempDir::new().unwrap();
    let comparison_path = temp_dir.path().join("comparison.json");
    let output_path = temp_dir.path().join("validation.json");

    // Create a comparison JSON with improvement but also regressions
    let comparison_json = r#"{
        "metadata": {
            "comparison_date": "2025-10-28T10:30:00Z",
            "before_file": "before.json",
            "after_file": "after.json",
            "target_location": "src/test.rs:complex_function:10"
        },
        "target_item": {
            "location": "src/test.rs:complex_function:10",
            "before": {
                "score": 80.0,
                "cyclomatic_complexity": 15,
                "cognitive_complexity": 20,
                "coverage": 0.0,
                "function_length": 150,
                "nesting_depth": 4
            },
            "after": {
                "score": 35.0,
                "cyclomatic_complexity": 8,
                "cognitive_complexity": 10,
                "coverage": 0.5,
                "function_length": 80,
                "nesting_depth": 2
            },
            "improvements": {
                "score_reduction_pct": 56.25,
                "complexity_reduction_pct": 50.0,
                "coverage_improvement_pct": 50.0
            },
            "status": "Improved"
        },
        "project_health": {
            "before": {
                "total_debt_score": 1500.0,
                "total_items": 100,
                "critical_items": 10,
                "high_priority_items": 20,
                "average_score": 15.0
            },
            "after": {
                "total_debt_score": 1480.0,
                "total_items": 102,
                "critical_items": 12,
                "high_priority_items": 21,
                "average_score": 14.5
            },
            "changes": {
                "debt_score_change": -20.0,
                "debt_score_change_pct": -1.33,
                "items_change": 2,
                "critical_items_change": 2
            }
        },
        "regressions": [
            {
                "location": "src/test.rs:new_helper:50",
                "score": 65.0,
                "debt_type": "Complexity",
                "description": "New complex helper function"
            },
            {
                "location": "src/test.rs:another_helper:75",
                "score": 62.0,
                "debt_type": "Complexity",
                "description": "Another complex helper function"
            }
        ],
        "improvements": [
            {
                "location": "src/test.rs:complex_function:10",
                "before_score": 80.0,
                "after_score": 35.0,
                "improvement_type": "ScoreReduced"
            }
        ],
        "summary": {
            "target_improved": true,
            "new_critical_count": 2,
            "resolved_count": 3,
            "overall_debt_trend": "Improving"
        }
    }"#;

    fs::write(&comparison_path, comparison_json).unwrap();

    // Run the validate-improvement subcommand
    let output = Command::new(env!("CARGO_BIN_EXE_debtmap"))
        .args([
            "validate-improvement",
            "--comparison",
            comparison_path.to_str().unwrap(),
            "--output",
            output_path.to_str().unwrap(),
            "--threshold",
            "75.0",
            "--format",
            "json",
        ])
        .output()
        .expect("Failed to execute validate-improvement command");

    // Verify command succeeded (even though validation may not meet threshold)
    assert!(
        output.status.success(),
        "Command failed with status: {:?}",
        output.status
    );

    // Verify output file was created
    assert!(
        output_path.exists(),
        "Validation output file was not created"
    );

    // Read and verify output content
    let validation_result = fs::read_to_string(&output_path).unwrap();
    let validation: serde_json::Value = serde_json::from_str(&validation_result).unwrap();

    // Verify completion percentage is lower due to regressions
    let completion = validation["completion_percentage"].as_f64().unwrap();
    assert!(
        completion < 75.0,
        "Expected completion < 75% due to regressions, got {}",
        completion
    );

    // Verify status is "incomplete"
    let status = validation["status"].as_str().unwrap();
    assert_eq!(
        status, "incomplete",
        "Expected status 'incomplete', got '{}'",
        status
    );

    // Verify gaps field exists and contains regression information
    assert!(
        validation.get("gaps").is_some(),
        "Missing gaps field for incomplete validation"
    );
}

/// Integration test for validate-improvement with previous validation (trend tracking)
#[test]
fn test_validate_improvement_with_previous_validation() {
    let temp_dir = TempDir::new().unwrap();
    let comparison_path = temp_dir.path().join("comparison.json");
    let previous_validation_path = temp_dir.path().join("previous_validation.json");
    let output_path = temp_dir.path().join("validation.json");

    // Create previous validation result (must match ValidationResult structure)
    let previous_validation = r#"{
        "completion_percentage": 60.0,
        "status": "incomplete",
        "improvements": [],
        "remaining_issues": ["Target improvement incomplete"],
        "gaps": {},
        "project_summary": {
            "total_debt_before": 1500.0,
            "total_debt_after": 1450.0,
            "improvement_percent": 3.33,
            "items_resolved": 3,
            "items_new": 2
        },
        "attempt_number": 1
    }"#;
    fs::write(&previous_validation_path, previous_validation).unwrap();

    // Create a comparison JSON showing improvement from previous attempt
    let comparison_json = r#"{
        "metadata": {
            "comparison_date": "2025-10-28T10:30:00Z",
            "before_file": "before.json",
            "after_file": "after.json",
            "target_location": "src/test.rs:complex_function:10"
        },
        "target_item": {
            "location": "src/test.rs:complex_function:10",
            "before": {
                "score": 80.0,
                "cyclomatic_complexity": 15,
                "cognitive_complexity": 20,
                "coverage": 0.0,
                "function_length": 150,
                "nesting_depth": 4
            },
            "after": {
                "score": 15.0,
                "cyclomatic_complexity": 3,
                "cognitive_complexity": 3,
                "coverage": 0.9,
                "function_length": 40,
                "nesting_depth": 1
            },
            "improvements": {
                "score_reduction_pct": 81.25,
                "complexity_reduction_pct": 85.0,
                "coverage_improvement_pct": 90.0
            },
            "status": "Improved"
        },
        "project_health": {
            "before": {
                "total_debt_score": 1500.0,
                "total_items": 100,
                "critical_items": 10,
                "high_priority_items": 20,
                "average_score": 15.0
            },
            "after": {
                "total_debt_score": 1410.0,
                "total_items": 97,
                "critical_items": 7,
                "high_priority_items": 17,
                "average_score": 14.5
            },
            "changes": {
                "debt_score_change": -90.0,
                "debt_score_change_pct": -6.0,
                "items_change": -3,
                "critical_items_change": -3
            }
        },
        "regressions": [],
        "improvements": [
            {
                "location": "src/test.rs:complex_function:10",
                "before_score": 80.0,
                "after_score": 15.0,
                "improvement_type": "ScoreReduced"
            }
        ],
        "summary": {
            "target_improved": true,
            "new_critical_count": 0,
            "resolved_count": 6,
            "overall_debt_trend": "Improving"
        }
    }"#;

    fs::write(&comparison_path, comparison_json).unwrap();

    // Run the validate-improvement subcommand with previous validation
    let output = Command::new(env!("CARGO_BIN_EXE_debtmap"))
        .args([
            "validate-improvement",
            "--comparison",
            comparison_path.to_str().unwrap(),
            "--previous-validation",
            previous_validation_path.to_str().unwrap(),
            "--output",
            output_path.to_str().unwrap(),
            "--threshold",
            "75.0",
            "--format",
            "json",
        ])
        .output()
        .expect("Failed to execute validate-improvement command");

    // Verify command succeeded
    assert!(
        output.status.success(),
        "Command failed with status: {:?}\nstdout: {}\nstderr: {}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    // Read and verify output content
    let validation_result = fs::read_to_string(&output_path).unwrap();
    let validation: serde_json::Value = serde_json::from_str(&validation_result).unwrap();

    // Verify trend analysis exists when previous validation is provided
    assert!(
        validation.get("trend_analysis").is_some(),
        "Missing trend_analysis field when previous validation provided"
    );

    let trend = &validation["trend_analysis"];
    assert!(
        trend.get("previous_completion").is_some(),
        "Missing previous_completion in trend_analysis"
    );
    assert!(
        trend.get("change").is_some(),
        "Missing change in trend_analysis"
    );
    assert!(
        trend.get("direction").is_some(),
        "Missing direction in trend_analysis"
    );

    // Verify attempt number is incremented
    let attempt_number = validation["attempt_number"].as_u64().unwrap();
    assert_eq!(
        attempt_number, 2,
        "Expected attempt_number to be incremented to 2"
    );
}
