//! Integration tests for unified progress display (spec 195, updated by spec 227).
//!
//! Verifies that all 2 analysis phases appear in output during real analysis workflow.

use std::path::PathBuf;
use std::process::Command;

/// Test that progress display shows all 2 phases during analysis
#[test]
fn test_progress_display_shows_all_phases() {
    // Use the sample codebase fixture for analysis
    let test_codebase =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/data/fixtures/sample_codebase");

    // Run debtmap analyze on the sample codebase
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

    // Get stderr output (where progress is displayed)
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Check command succeeded
    if !output.status.success() {
        eprintln!("stdout: {}", String::from_utf8_lossy(&output.stdout));
        eprintln!("stderr: {}", stderr);
        panic!("debtmap analyze command failed");
    }

    // Verify all 2 phases appear in output (files and parse are now combined)
    assert!(
        stderr.contains("1/2 files parse"),
        "Phase 1 'files parse' should appear in output"
    );
    assert!(
        stderr.contains("2/2 Building call graph"),
        "Phase 2 'Building call graph' should appear in output"
    );

    // Verify completion message appears
    assert!(
        stderr.contains("Analysis complete in"),
        "Completion message should appear in output"
    );
}

/// Test that progress display shows completion indicators
#[test]
fn test_progress_display_shows_completion_indicators() {
    let test_codebase =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/data/fixtures/sample_codebase");

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

    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(output.status.success(), "Command should succeed");

    // Verify completion checkmarks (✓) appear for completed phases
    assert!(
        stderr.contains("✓ 1/2") || stderr.contains("✓ 2/2"),
        "Completion indicators (✓) should appear for completed phases"
    );
}

/// Test that progress display completes all phases successfully
#[test]
fn test_progress_display_completes_all_phases() {
    let test_codebase =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/data/fixtures/sample_codebase");

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

    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(output.status.success(), "Command should succeed");

    // All phases should complete successfully (in CI/CD mode, only completion lines appear)
    // Count the number of completed phases
    let completed_phases = stderr.matches("✓").count();

    assert!(
        completed_phases >= 2,
        "Should have at least 2 completed phases, found: {}",
        completed_phases
    );
}

/// Test that progress display shows timing information
#[test]
fn test_progress_display_shows_timing() {
    let test_codebase =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/data/fixtures/sample_codebase");

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

    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(output.status.success(), "Command should succeed");

    // Verify timing appears in output (format: "Xs" for seconds)
    assert!(
        stderr.contains("s") && stderr.contains("Analysis complete in"),
        "Timing information should appear in seconds"
    );

    // Phase durations should be shown (format: "- Xs")
    assert!(
        stderr.contains(" - ") && stderr.contains("s"),
        "Individual phase durations should be shown"
    );
}

/// Test that progress display works with empty codebase
#[test]
fn test_progress_display_with_empty_codebase() {
    use tempfile::TempDir;

    // Create empty temporary directory
    let temp_dir = TempDir::new().unwrap();

    let output = Command::new("cargo")
        .args([
            "run",
            "--bin",
            "debtmap",
            "--quiet",
            "--",
            "analyze",
            temp_dir.path().to_str().unwrap(),
        ])
        .output()
        .expect("Failed to execute debtmap command");

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Even with empty codebase, phases should still appear
    assert!(
        stderr.contains("1/2 files parse"),
        "Phase 1 should appear even for empty codebase"
    );

    // Should complete all phases even with no files
    let completed_phases = stderr.matches("✓").count();
    assert!(
        completed_phases >= 2,
        "All 2 phases should complete even for empty codebase, found: {}",
        completed_phases
    );
}
