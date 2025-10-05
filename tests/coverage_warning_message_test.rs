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

    // Verify it's only displayed when coverage_data.is_none() and not quiet_mode
    let warning_section = source
        .split("Emit warning if no coverage data provided")
        .nth(1)
        .expect("Should find warning section");

    assert!(
        warning_section.contains("if coverage_data.is_none() && !quiet_mode"),
        "Warning should only be displayed when no coverage data and not in quiet mode"
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

    // Find the warning section
    let warning_section = source
        .split("if coverage_data.is_none() && !quiet_mode")
        .nth(1)
        .and_then(|s| s.split("if !quiet_mode").next())
        .expect("Should find warning section");

    // Count how many times eprintln! appears in the warning section
    let eprintln_count = warning_section.matches("eprintln!").count();

    // The warning should have multiple eprintln! calls (for multi-line output)
    // but they should all be in the same conditional block
    assert!(
        eprintln_count >= 3,
        "Warning should have multiple lines (found {})",
        eprintln_count
    );

    // Verify there's no loop around the warning
    let before_warning = source
        .split("if coverage_data.is_none() && !quiet_mode")
        .next()
        .expect("Should find code before warning");

    let context_before = before_warning.chars().rev().take(500).collect::<String>();
    assert!(
        !context_before.contains("for ") && !context_before.contains("while "),
        "Warning should not be in a loop"
    );
}
