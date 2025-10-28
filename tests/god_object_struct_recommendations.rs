use debtmap::organization::{
    calculate_struct_ratio, count_distinct_domains, determine_cross_domain_severity,
    GodObjectDetector, RecommendationSeverity, StructMetrics,
};
use std::path::Path;

/// Test helper functions work correctly with realistic struct data
#[test]
fn test_helper_functions_integration() {
    // Test struct_ratio calculation with realistic values
    assert_eq!(calculate_struct_ratio(10, 20), 0.5); // struct-heavy if > 0.3
    assert_eq!(calculate_struct_ratio(3, 20), 0.15); // method-heavy
    assert_eq!(calculate_struct_ratio(8, 15), 8.0 / 15.0); // borderline

    // Test severity determination with various scenarios
    let severity = determine_cross_domain_severity(10, 3, 600, true);
    assert!(matches!(severity, RecommendationSeverity::Critical)); // god object + cross-domain

    let severity = determine_cross_domain_severity(16, 5, 400, false);
    assert!(matches!(severity, RecommendationSeverity::Critical)); // massive mixing

    let severity = determine_cross_domain_severity(10, 4, 500, false);
    assert!(matches!(severity, RecommendationSeverity::High)); // significant issues

    let severity = determine_cross_domain_severity(8, 2, 300, false);
    assert!(matches!(severity, RecommendationSeverity::Medium)); // proactive improvement
}

/// Test domain counting with realistic struct metrics
#[test]
fn test_domain_counting_integration() {
    let structs = vec![
        StructMetrics {
            name: "ThresholdConfig".to_string(),
            line_span: (10, 30),
            method_count: 5,
            field_count: 10,
            responsibilities: vec!["configuration".to_string()],
        },
        StructMetrics {
            name: "ThresholdValidator".to_string(),
            line_span: (31, 50),
            method_count: 3,
            field_count: 4,
            responsibilities: vec!["validation".to_string()],
        },
        StructMetrics {
            name: "ScoringWeight".to_string(),
            line_span: (51, 70),
            method_count: 2,
            field_count: 6,
            responsibilities: vec!["calculation".to_string()],
        },
        StructMetrics {
            name: "ScoringMultiplier".to_string(),
            line_span: (71, 90),
            method_count: 4,
            field_count: 3,
            responsibilities: vec!["calculation".to_string()],
        },
        StructMetrics {
            name: "DetectionConfig".to_string(),
            line_span: (91, 110),
            method_count: 2,
            field_count: 5,
            responsibilities: vec!["configuration".to_string()],
        },
    ];

    // Should detect 3 distinct domains: thresholds, scoring, detection
    let domain_count = count_distinct_domains(&structs);
    assert_eq!(domain_count, 3, "Should detect 3 distinct semantic domains");
}

/// Test that god object analysis includes new spec 140 fields
#[test]
fn test_god_object_analysis_includes_new_fields() {
    // Simple smoke test to verify new fields are populated
    let code = r#"
        pub struct Config1 { field: u32 }
        pub fn func1() {}
    "#;

    let file = syn::parse_file(code).expect("Failed to parse");
    let detector = GodObjectDetector::with_source_content(code);
    let analysis = detector.analyze_comprehensive(Path::new("test.rs"), &file);

    // Verify new fields exist and have sensible defaults
    // domain_count should be >= 0
    let _ = analysis.domain_count;

    // domain_diversity should be 0.0 to 1.0
    assert!(analysis.domain_diversity >= 0.0 && analysis.domain_diversity <= 1.0);

    // struct_ratio should be >= 0.0
    assert!(analysis.struct_ratio >= 0.0);

    // analysis_method should have a value
    let _ = analysis.analysis_method;

    // cross_domain_severity is optional
    let _ = analysis.cross_domain_severity;
}
