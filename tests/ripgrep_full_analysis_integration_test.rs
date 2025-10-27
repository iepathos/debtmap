/// Integration test that runs full debtmap analysis on actual ripgrep codebase
/// and verifies that flags/defs.rs is correctly detected as boilerplate.
///
/// This test verifies the end-to-end workflow from spec 140:
/// 1. Run full analysis on ripgrep codebase (if available)
/// 2. Find flags/defs.rs in the results
/// 3. Verify it's flagged with BOILERPLATE DETECTED recommendation
/// 4. Verify confidence is in the expected range (~87-88% for 888 impls)
use std::path::Path;
use std::process::Command;

#[test]
fn test_ripgrep_full_analysis_integration() {
    // Check if ripgrep source is available
    let ripgrep_path = Path::new("../ripgrep");
    let flags_file = ripgrep_path.join("crates/core/flags/defs.rs");

    if !flags_file.exists() {
        println!("\n⚠️  Skipping integration test - ripgrep source not found");
        println!(
            "   Expected location: {:?}",
            ripgrep_path.canonicalize().ok()
        );
        println!("   To run this test:");
        println!("   1. Clone ripgrep: git clone https://github.com/BurntSushi/ripgrep ../ripgrep");
        println!("   2. Re-run: cargo test test_ripgrep_full_analysis_integration");
        return;
    }

    println!("\n=== RUNNING FULL DEBTMAP ANALYSIS ON RIPGREP ===");
    println!("Target: {:?}", ripgrep_path.canonicalize().unwrap());

    // Run debtmap analyze command on ripgrep
    let output = Command::new("cargo")
        .args(["run", "--", "analyze", "--path"])
        .arg(ripgrep_path.to_str().unwrap())
        .args(["--format", "json"])
        .output()
        .expect("Failed to run debtmap analyze command");

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!("debtmap analyze failed:\n{}", stderr);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    println!("Analysis completed. Output size: {} bytes", stdout.len());

    // Parse JSON output
    let results: serde_json::Value =
        serde_json::from_str(&stdout).expect("Failed to parse debtmap JSON output");

    // Find flags/defs.rs in the results
    let files = results["files"]
        .as_array()
        .expect("Expected 'files' array in output");

    println!("Total files analyzed: {}", files.len());

    let flags_result = files.iter().find(|f| {
        let path = f["path"].as_str().unwrap_or("");
        path.contains("flags") && path.contains("defs.rs")
    });

    let flags_data = flags_result.expect(
        "flags/defs.rs not found in analysis results. \
         This may indicate the analysis didn't complete or the file wasn't analyzed.",
    );

    println!("\n=== FLAGS/DEFS.RS ANALYSIS RESULT ===");
    println!("Path: {}", flags_data["path"].as_str().unwrap());
    println!("Lines: {}", flags_data["metrics"]["total_lines"]);
    println!("Functions: {}", flags_data["metrics"]["function_count"]);

    // Extract recommendation
    let recommendation = flags_data["recommendation"]
        .as_str()
        .expect("No recommendation field in flags/defs.rs result");

    println!("\n=== RECOMMENDATION ===");
    println!("{}", recommendation);
    println!("=====================\n");

    // Verify BOILERPLATE DETECTED is in the recommendation
    assert!(
        recommendation.contains("BOILERPLATE DETECTED"),
        "Expected 'BOILERPLATE DETECTED' in recommendation, got: {}",
        recommendation
    );

    // Verify it mentions the trait pattern
    assert!(
        recommendation.contains("Flag") || recommendation.contains("trait"),
        "Expected recommendation to mention Flag trait, got: {}",
        recommendation
    );

    // Verify it recommends macros
    assert!(
        recommendation.contains("macro") || recommendation.contains("code-generat"),
        "Expected recommendation to mention macros or code generation, got: {}",
        recommendation
    );

    // Verify it explicitly says NOT to split into modules
    assert!(
        recommendation.contains("NOT")
            && (recommendation.contains("god object") || recommendation.contains("module split")),
        "Expected recommendation to clarify this is NOT a module splitting case, got: {}",
        recommendation
    );

    // Check for confidence percentage if available
    // Note: The exact field structure may vary, so we check multiple possible locations
    let confidence_pct = if let Some(conf) = flags_data["confidence"].as_f64() {
        Some(conf * 100.0)
    } else if let Some(god_type) = flags_data["metrics"]["god_object_type"].as_object() {
        god_type
            .get("confidence")
            .and_then(|c| c.as_f64())
            .map(|c| c * 100.0)
    } else {
        None
    };

    if let Some(conf) = confidence_pct {
        println!("Confidence: {:.1}%", conf);

        // For 888 implementations, we expect ~87-88% confidence
        // Allow some variance (85-95% range is acceptable)
        assert!(
            (85.0..=95.0).contains(&conf),
            "Expected confidence in 85-95% range for 888 implementations, got {:.1}%",
            conf
        );
    } else {
        println!("⚠️  Warning: Could not extract confidence score from output");
        println!("   This is non-fatal but the spec expects confidence reporting");
    }

    println!("\n✅ Integration test passed!");
    println!("   - flags/defs.rs detected as boilerplate");
    println!("   - Recommendation includes macro/codegen approach");
    println!("   - Correctly identified as NOT a god object needing module splitting");
}
