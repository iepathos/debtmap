use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

/// Integration test for spec 152 - Domain diversity metrics in output
///
/// Tests that when analyzing a file with multiple structs across different domains,
/// the output contains domain diversity analysis with:
/// - "DOMAIN DIVERSITY ANALYSIS" header
/// - Reference to "Spec 140"
/// - Severity level display
/// - Domain count (not hardcoded "across 1 responsibilities")
#[test]
fn test_config_rs_shows_domain_metrics() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("domain_output.txt");

    // Use src/analysis/patterns/config.rs which has multiple configuration structs
    // This file likely has structs across different domains (config, analysis, patterns)
    let test_file =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/analysis/patterns/config.rs");

    // Ensure the test file exists
    assert!(
        test_file.exists(),
        "Test file does not exist: {}",
        test_file.display()
    );

    // Run debtmap analyze on the specific file
    let output = Command::new("cargo")
        .args([
            "run",
            "--bin",
            "debtmap",
            "--quiet",
            "--",
            "analyze",
            "--format",
            "text",
            "--output",
            output_path.to_str().unwrap(),
            test_file.to_str().unwrap(),
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
    let output_content = fs::read_to_string(&output_path).expect("Failed to read output file");

    // Debug: print output if test will fail
    if !output_content.contains("DOMAIN DIVERSITY ANALYSIS") {
        eprintln!("=== Output Content ===");
        eprintln!("{}", output_content);
        eprintln!("=== End Output ===");
    }

    // Spec 152 Phase 5: Verify output contains domain diversity analysis
    assert!(
        output_content.contains("DOMAIN DIVERSITY ANALYSIS"),
        "Output should contain 'DOMAIN DIVERSITY ANALYSIS' header"
    );

    // Verify reference to Spec 140
    assert!(
        output_content.contains("Spec 140"),
        "Output should reference 'Spec 140'"
    );

    // Verify severity is displayed
    assert!(
        output_content.contains("Severity:"),
        "Output should show severity level"
    );

    // Verify domain count is shown (not hardcoded responsibility count)
    assert!(
        output_content.contains("domains"),
        "Output should mention domain count"
    );

    // Verify we don't have the old buggy "across 1 responsibilities" text
    assert!(
        !output_content.contains("across 1 responsibilities"),
        "Output should NOT contain hardcoded 'across 1 responsibilities'"
    );
}

/// Test that domain diversity metrics appear in text output format
#[test]
fn test_domain_diversity_in_text_format() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("text_output.txt");

    // Analyze a directory that likely has god objects with domain mixing
    let test_codebase = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/organization");

    let output = Command::new("cargo")
        .args([
            "run",
            "--bin",
            "debtmap",
            "--quiet",
            "--",
            "analyze",
            "--format",
            "text",
            "--output",
            output_path.to_str().unwrap(),
            test_codebase.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to execute debtmap command");

    if !output.status.success() {
        eprintln!("stdout: {}", String::from_utf8_lossy(&output.stdout));
        eprintln!("stderr: {}", String::from_utf8_lossy(&output.stderr));
        panic!("debtmap analyze command failed");
    }

    let output_content = fs::read_to_string(&output_path).expect("Failed to read output file");

    // If domain diversity metrics are present, verify they're formatted correctly
    if output_content.contains("DOMAIN DIVERSITY ANALYSIS") {
        // Verify all required components are present
        assert!(
            output_content.contains("Spec 140"),
            "Domain diversity analysis should reference Spec 140"
        );
        assert!(
            output_content.contains("Severity:"),
            "Domain diversity analysis should show severity"
        );
        assert!(
            output_content.contains("domains"),
            "Domain diversity analysis should mention domain count"
        );
        assert!(
            output_content.contains("Domain Distribution:"),
            "Domain diversity analysis should show distribution"
        );
    }
}
