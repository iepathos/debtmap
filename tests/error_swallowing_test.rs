use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn test_error_swallowing_detection() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let test_file = temp_dir.path().join("test.rs");

    // Create a test file with various error swallowing patterns
    std::fs::write(
        &test_file,
        r#"
fn example1() -> Result<(), std::io::Error> {
    // This should be detected: if let Ok without else
    if let Ok(value) = some_function() {
        println!("{}", value);
    }
    
    // This should be detected: let _ discarding Result
    let _ = function_returning_result();
    
    // This should be detected: .ok() discarding error
    some_result.ok();
    
    // This should be detected: match with ignored Err
    match another_result {
        Ok(v) => println!("{}", v),
        Err(_) => {},
    }
    
    // This should be detected: unwrap_or without logging
    let value = result.unwrap_or(0);
    
    Ok(())
}

#[test]
fn test_function() {
    // Lower priority in test functions
    if let Ok(value) = some_test_function() {
        assert_eq!(value, 42);
    }
}

fn some_function() -> Result<i32, std::io::Error> {
    Ok(42)
}

fn function_returning_result() -> Result<(), std::io::Error> {
    Ok(())
}
"#,
    )
    .expect("Failed to write test file");

    // Run debtmap on the test file
    let output = Command::new("cargo")
        .args(&[
            "run",
            "--",
            "analyze",
            "--format",
            "json",
            test_file.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to run debtmap");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Parse JSON output
    let result: serde_json::Value =
        serde_json::from_str(&stdout).expect("Failed to parse JSON output");

    // Check that error swallowing patterns were detected
    let debt_items = result["technical_debt"]["items"]
        .as_array()
        .expect("No debt items found");

    let error_swallowing_items: Vec<_> = debt_items
        .iter()
        .filter(|item| item["debt_type"] == "ErrorSwallowing")
        .collect();

    // We should detect at least 5 error swallowing patterns in the main function
    assert!(
        error_swallowing_items.len() >= 5,
        "Expected at least 5 error swallowing patterns, found {}",
        error_swallowing_items.len()
    );

    // Check that test function patterns have lower priority
    let test_items: Vec<_> = error_swallowing_items
        .iter()
        .filter(|item| {
            item["message"]
                .as_str()
                .map(|msg| msg.contains("test_function"))
                .unwrap_or(false)
        })
        .collect();

    for item in test_items {
        assert_eq!(
            item["priority"], "Low",
            "Test function error swallowing should have Low priority"
        );
    }
}

#[test]
fn test_error_swallowing_suppression() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let test_file = temp_dir.path().join("test_suppression.rs");

    // Create a test file with suppressed error swallowing
    std::fs::write(
        &test_file,
        r#"
fn example() {
    // debtmap:ignore-next-line [ErrorSwallowing] - Intentionally ignoring error for demo
    if let Ok(value) = some_function() {
        println!("{}", value);
    }
    
    // This should still be detected
    let _ = another_function();
}

fn some_function() -> Result<i32, std::io::Error> {
    Ok(42)
}

fn another_function() -> Result<(), std::io::Error> {
    Ok(())
}
"#,
    )
    .expect("Failed to write test file");

    // Run debtmap on the test file
    let output = Command::new("cargo")
        .args(&[
            "run",
            "--",
            "analyze",
            "--format",
            "json",
            test_file.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to run debtmap");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Parse JSON output
    let result: serde_json::Value =
        serde_json::from_str(&stdout).expect("Failed to parse JSON output");

    // Check that only one error swallowing pattern was detected (the unsuppressed one)
    let debt_items = result["technical_debt"]["items"]
        .as_array()
        .expect("No debt items found");

    let error_swallowing_items: Vec<_> = debt_items
        .iter()
        .filter(|item| item["debt_type"] == "ErrorSwallowing")
        .collect();

    assert_eq!(
        error_swallowing_items.len(),
        1,
        "Expected 1 unsuppressed error swallowing pattern, found {}",
        error_swallowing_items.len()
    );
}
