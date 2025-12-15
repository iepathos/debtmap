/// Integration test for spec 108: Coverage warning message
///
/// This test verifies that the coverage warning message infrastructure is in place.
/// The warning is displayed to stderr when coverage data is not provided.
use std::path::PathBuf;

#[test]
fn test_warning_message_content_in_source() {
    // This test documents the expected warning message content
    // by reading it from the source code to ensure it matches spec

    let source_path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/builders/unified_analysis.rs");
    let source = std::fs::read_to_string(&source_path).expect("Failed to read source file");

    // Verify the warning message exists in the source
    assert!(
        source.contains("Coverage data not provided"),
        "Source should contain coverage warning message"
    );
    assert!(
        source.contains("Analysis will focus on complexity and code smells"),
        "Warning should explain what analysis will focus on"
    );
    assert!(
        source.contains("--lcov-file coverage.info"),
        "Warning should show how to provide coverage data"
    );

    // Verify it's displayed via emit_coverage_tip helper function
    assert!(
        source.contains("fn emit_coverage_tip"),
        "Should have emit_coverage_tip helper function"
    );
    assert!(
        source.contains("emit_coverage_tip(coverage_data.is_none()"),
        "Warning should be emitted when no coverage data"
    );
}

#[test]
fn test_unified_analysis_has_coverage_data_flag() {
    // Test that UnifiedAnalysis correctly tracks whether coverage data is available
    use debtmap::priority::{CallGraph, UnifiedAnalysis};

    let call_graph = CallGraph::new();
    let mut analysis = UnifiedAnalysis::new(call_graph);

    // By default, should be false
    assert!(
        !analysis.has_coverage_data,
        "New analysis should default to has_coverage_data=false"
    );

    // Should be able to set it
    analysis.has_coverage_data = true;
    assert!(
        analysis.has_coverage_data,
        "Should be able to set has_coverage_data to true"
    );

    analysis.has_coverage_data = false;
    assert!(
        !analysis.has_coverage_data,
        "Should be able to set has_coverage_data to false"
    );
}

#[test]
fn test_warning_displayed_once_not_multiple_times() {
    // This test documents that the warning is in a conditional block
    // that executes at most once during analysis creation

    let source_path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/builders/unified_analysis.rs");
    let source = std::fs::read_to_string(&source_path).expect("Failed to read source file");

    // Find the emit_coverage_tip function
    let tip_section = source
        .split("fn emit_coverage_tip")
        .nth(1)
        .expect("Should find emit_coverage_tip function");

    // Verify the function uses structured logging (warn! macro) - spec 208
    let fn_body = tip_section
        .split("fn ") // Stop at next function
        .next()
        .unwrap_or(tip_section);

    // The warning should use the warn! macro for structured logging
    assert!(
        fn_body.contains("warn!"),
        "Warning should use warn! macro for structured logging"
    );

    // Verify emit_coverage_tip is called only once in perform_unified_analysis_with_options
    let main_fn = source
        .split("pub fn perform_unified_analysis_with_options")
        .nth(1)
        .and_then(|s| s.split("pub fn").next())
        .expect("Should find main function");

    let emit_calls = main_fn.matches("emit_coverage_tip").count();
    assert_eq!(
        emit_calls, 1,
        "emit_coverage_tip should be called exactly once in main function"
    );
}
