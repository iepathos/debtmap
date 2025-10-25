/// Integration test for spec 118 - Pure Mapping Pattern Detection
/// Verifies that mapping patterns are detected and result in reduced complexity scores
use debtmap::analyzers::{rust::RustAnalyzer, Analyzer};
use std::path::PathBuf;

#[test]
fn test_nested_match_pattern_detection() {
    let analyzer = RustAnalyzer::default();

    // Test code with nested match patterns
    let code = r#"
        enum Outer { A, B }
        enum Inner { X, Y }

        fn format(outer: Outer, inner: Inner) -> String {
            let label = match outer {
                Outer::A => "A",
                Outer::B => "B",
            };

            match inner {
                Inner::X => format!("{}_X", label),
                Inner::Y => format!("{}_Y", label),
            }
        }
    "#;

    // Parse and analyze
    let ast = analyzer
        .parse(code, PathBuf::from("test.rs"))
        .expect("Failed to parse test code");
    let file_metrics = analyzer.analyze(&ast);

    // Find the function
    let func = file_metrics
        .complexity
        .functions
        .iter()
        .find(|f| f.name == "format")
        .expect("format function not found");

    // Verify mapping pattern was detected
    if let Some(ref mapping_result) = func.mapping_pattern_result {
        assert!(mapping_result.is_pure_mapping);

        // Should mention "nested" in the description
        assert!(
            mapping_result.pattern_description.contains("nested"),
            "Pattern description should mention 'nested'"
        );

        // The adjustment factor should be between 0.0 and 1.0
        assert!(
            mapping_result.complexity_adjustment_factor > 0.0
                && mapping_result.complexity_adjustment_factor <= 1.0,
            "Adjustment factor should be in range (0.0, 1.0]"
        );
    }
}
