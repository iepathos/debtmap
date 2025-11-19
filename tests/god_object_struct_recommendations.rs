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

/// Test behavioral decomposition fields (Spec 178)
#[test]
fn test_behavioral_decomposition_fields_populated() {
    use debtmap::organization::recommend_module_splits;
    use std::collections::HashMap;

    // Create test responsibility groups
    let mut responsibility_groups = HashMap::new();
    responsibility_groups.insert(
        "rendering".to_string(),
        vec![
            "render".to_string(),
            "draw".to_string(),
            "paint".to_string(),
            "display".to_string(),
            "show".to_string(),
            "update_view".to_string(),
        ],
    );
    responsibility_groups.insert(
        "event_handling".to_string(),
        vec![
            "handle_click".to_string(),
            "on_mouse_down".to_string(),
            "on_key_press".to_string(),
            "handle_input".to_string(),
            "dispatch_event".to_string(),
            "process_event".to_string(),
        ],
    );

    let splits = recommend_module_splits("Editor", &[], &responsibility_groups);

    // Verify we got splits
    assert!(
        !splits.is_empty(),
        "Should generate module splits from responsibility groups"
    );

    for split in &splits {
        // Verify new Spec 178 fields are populated
        assert!(
            !split.representative_methods.is_empty(),
            "representative_methods should be populated"
        );
        assert!(
            split.representative_methods.len() <= 8,
            "Should have at most 8 representative methods"
        );

        assert!(
            split.behavior_category.is_some(),
            "behavior_category should be populated"
        );

        // Verify responsibility name is not "misc"
        assert!(
            !split.responsibility.to_lowercase().contains("misc"),
            "Should not use 'misc' category - got: {}",
            split.responsibility
        );

        // Verify suggested name is not "misc"
        assert!(
            !split.suggested_name.to_lowercase().contains("misc"),
            "Module name should not contain 'misc' - got: {}",
            split.suggested_name
        );

        // Verify method_count matches methods_to_move length
        assert_eq!(
            split.method_count,
            split.methods_to_move.len(),
            "method_count should match methods_to_move length"
        );
    }
}

/// Test that "misc" category is eliminated from recommendations (Spec 178)
#[test]
fn test_misc_category_eliminated() {
    use debtmap::organization::recommend_module_splits;
    use std::collections::HashMap;

    // Create various responsibility groups
    let mut responsibility_groups = HashMap::new();
    responsibility_groups.insert(
        "validation".to_string(),
        vec![
            "validate".to_string(),
            "check".to_string(),
            "verify".to_string(),
            "ensure".to_string(),
            "assert".to_string(),
            "confirm".to_string(),
        ],
    );
    responsibility_groups.insert(
        "persistence".to_string(),
        vec![
            "save".to_string(),
            "load".to_string(),
            "store".to_string(),
            "retrieve".to_string(),
            "persist".to_string(),
            "restore".to_string(),
        ],
    );

    let splits = recommend_module_splits("Service", &[], &responsibility_groups);

    // Verify no "misc" category in any recommendation
    for split in &splits {
        assert!(
            !split.responsibility.eq_ignore_ascii_case("misc"),
            "Should not have 'misc' responsibility"
        );
        assert!(
            !split.suggested_name.to_lowercase().contains("misc"),
            "Should not have 'misc' in module name"
        );
        if let Some(category) = &split.behavior_category {
            assert!(
                !category.eq_ignore_ascii_case("misc"),
                "Should not have 'misc' behavior category"
            );
        }
    }
}
