use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn test_validation_command_with_successful_improvement() {
    let temp_dir = TempDir::new().unwrap();
    let comparison_path = temp_dir.path().join("comparison.json");
    let output_path = temp_dir.path().join("validation.json");

    // Create a comparison JSON with successful improvement
    let comparison_json = r#"{
        "metadata": {
            "comparison_date": "2025-10-02T10:30:00Z",
            "before_file": "before.json",
            "after_file": "after.json",
            "target_location": "test.rs:foo:10"
        },
        "target_item": {
            "location": "test.rs:foo:10",
            "before": {
                "score": 80.0,
                "cyclomatic_complexity": 15,
                "cognitive_complexity": 20,
                "coverage": 0.0,
                "function_length": 150,
                "nesting_depth": 4
            },
            "after": {
                "score": 10.0,
                "cyclomatic_complexity": 3,
                "cognitive_complexity": 2,
                "coverage": 0.8,
                "function_length": 30,
                "nesting_depth": 1
            },
            "improvements": {
                "score_reduction_pct": 87.5,
                "complexity_reduction_pct": 90.0,
                "coverage_improvement_pct": 80.0
            },
            "status": "Improved"
        },
        "project_health": {
            "before": {
                "total_debt_score": 1000.0,
                "total_items": 100,
                "critical_items": 10,
                "high_priority_items": 20,
                "average_score": 10.0
            },
            "after": {
                "total_debt_score": 900.0,
                "total_items": 95,
                "critical_items": 8,
                "high_priority_items": 15,
                "average_score": 9.5
            },
            "changes": {
                "debt_score_change": -100.0,
                "debt_score_change_pct": -10.0,
                "items_change": -5,
                "critical_items_change": -2
            }
        },
        "regressions": [],
        "improvements": [],
        "summary": {
            "target_improved": true,
            "new_critical_count": 0,
            "resolved_count": 5,
            "overall_debt_trend": "Improving"
        }
    }"#;

    fs::write(&comparison_path, comparison_json).unwrap();

    // Run the validation command
    let output = Command::new("cargo")
        .args([
            "run",
            "--bin",
            "prodigy-validate-debtmap-improvement",
            "--quiet",
        ])
        .env(
            "ARGUMENTS",
            format!(
                "--comparison {} --output {}",
                comparison_path.display(),
                output_path.display()
            ),
        )
        .env("PRODIGY_AUTOMATION", "true")
        .output()
        .unwrap();

    assert!(output.status.success(), "Command failed: {:?}", output);

    // Read and parse validation result
    let validation_json = fs::read_to_string(&output_path).unwrap();
    let validation: serde_json::Value = serde_json::from_str(&validation_json).unwrap();

    // Verify results
    assert_eq!(validation["status"], "complete");
    assert!(validation["completion_percentage"].as_f64().unwrap() >= 75.0);
    assert!(!validation["improvements"].as_array().unwrap().is_empty());
    assert_eq!(validation["remaining_issues"].as_array().unwrap().len(), 0);
}

#[test]
fn test_validation_command_with_regressions() {
    let temp_dir = TempDir::new().unwrap();
    let comparison_path = temp_dir.path().join("comparison.json");
    let output_path = temp_dir.path().join("validation.json");

    // Create a comparison JSON with regressions
    let comparison_json = r#"{
        "metadata": {
            "comparison_date": "2025-10-02T10:30:00Z",
            "before_file": "before.json",
            "after_file": "after.json",
            "target_location": "test.rs:foo:10"
        },
        "target_item": {
            "location": "test.rs:foo:10",
            "before": {
                "score": 80.0,
                "cyclomatic_complexity": 15,
                "cognitive_complexity": 20,
                "coverage": 0.0,
                "function_length": 150,
                "nesting_depth": 4
            },
            "after": {
                "score": 20.0,
                "cyclomatic_complexity": 5,
                "cognitive_complexity": 6,
                "coverage": 0.5,
                "function_length": 50,
                "nesting_depth": 2
            },
            "improvements": {
                "score_reduction_pct": 75.0,
                "complexity_reduction_pct": 70.0,
                "coverage_improvement_pct": 50.0
            },
            "status": "Improved"
        },
        "project_health": {
            "before": {
                "total_debt_score": 1000.0,
                "total_items": 100,
                "critical_items": 10,
                "high_priority_items": 20,
                "average_score": 10.0
            },
            "after": {
                "total_debt_score": 980.0,
                "total_items": 101,
                "critical_items": 11,
                "high_priority_items": 20,
                "average_score": 9.7
            },
            "changes": {
                "debt_score_change": -20.0,
                "debt_score_change_pct": -2.0,
                "items_change": 1,
                "critical_items_change": 1
            }
        },
        "regressions": [
            {
                "location": "test.rs:bar:50",
                "score": 70.0,
                "debt_type": "high_complexity",
                "description": "New complex function"
            }
        ],
        "improvements": [],
        "summary": {
            "target_improved": true,
            "new_critical_count": 1,
            "resolved_count": 1,
            "overall_debt_trend": "Improving"
        }
    }"#;

    fs::write(&comparison_path, comparison_json).unwrap();

    // Run the validation command
    let output = Command::new("cargo")
        .args([
            "run",
            "--bin",
            "prodigy-validate-debtmap-improvement",
            "--quiet",
        ])
        .env(
            "ARGUMENTS",
            format!(
                "--comparison {} --output {}",
                comparison_path.display(),
                output_path.display()
            ),
        )
        .env("PRODIGY_AUTOMATION", "true")
        .output()
        .unwrap();

    assert!(output.status.success(), "Command failed: {:?}", output);

    // Read and parse validation result
    let validation_json = fs::read_to_string(&output_path).unwrap();
    let validation: serde_json::Value = serde_json::from_str(&validation_json).unwrap();

    // Verify results
    assert_eq!(validation["status"], "incomplete");
    assert!(!validation["remaining_issues"]
        .as_array()
        .unwrap()
        .is_empty());
    assert!(validation["gaps"]
        .as_object()
        .unwrap()
        .contains_key("regression_0"));
}

#[test]
fn test_validation_command_missing_comparison_file() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("validation.json");

    // Run the validation command with non-existent file
    let output = Command::new("cargo")
        .args([
            "run",
            "--bin",
            "prodigy-validate-debtmap-improvement",
            "--quiet",
        ])
        .env(
            "ARGUMENTS",
            format!(
                "--comparison /nonexistent/file.json --output {}",
                output_path.display()
            ),
        )
        .env("PRODIGY_AUTOMATION", "true")
        .output()
        .unwrap();

    assert!(!output.status.success(), "Command should have failed");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("does not exist"));
}
