//! Integration tests for anti-pattern detection in behavioral splits
//!
//! Tests the detection of anti-patterns (utilities modules, technical grouping)
//! in real module split recommendations and validates quality scoring.

use debtmap::analyzers::type_registry::MethodSignature;
use debtmap::organization::anti_pattern_detector::{
    AntiPatternDetector, AntiPatternSeverity, AntiPatternType,
};
use debtmap::organization::god_object_analysis::ModuleSplit;

/// Test detecting utilities and technical grouping anti-patterns in behavioral splits
#[test]
fn test_detect_behavioral_split_anti_patterns() {
    // Create a detector with default configuration
    let detector = AntiPatternDetector::new();

    // Simulate a behavioral split that recommends a "utilities" module
    let utilities_split = ModuleSplit {
        suggested_name: "god_object/utilities.rs".to_string(),
        methods_to_move: vec![
            "format_header".to_string(),
            "format_footer".to_string(),
            "calculate_total".to_string(),
            "validate_input".to_string(),
        ],
        responsibility: "utilities".to_string(),
        estimated_lines: 150,
        method_count: 4,
        ..Default::default()
    };

    // Simulate a behavioral split that uses technical/verb-based grouping
    let technical_split = ModuleSplit {
        suggested_name: "god_object/calculate.rs".to_string(),
        methods_to_move: vec![
            "calculate_score".to_string(),
            "calculate_total".to_string(),
            "calculate_average".to_string(),
        ],
        responsibility: "calculation".to_string(),
        estimated_lines: 100,
        method_count: 3,
        ..Default::default()
    };

    // Simulate a good domain-based split for comparison
    let good_split = ModuleSplit {
        suggested_name: "god_object/priority_metrics.rs".to_string(),
        methods_to_move: vec![
            "calculate_priority".to_string(),
            "update_priority".to_string(),
        ],
        responsibility: "priority metrics".to_string(),
        estimated_lines: 80,
        method_count: 2,
        ..Default::default()
    };

    let all_splits = vec![utilities_split, technical_split, good_split];

    // Create method signatures for testing parameter passing detection
    let signatures = vec![
        MethodSignature {
            name: "format_header".to_string(),
            self_param: None,
            param_types: vec![
                "String".to_string(),
                "usize".to_string(),
                "bool".to_string(),
            ],
            return_type: Some("String".to_string()),
        },
        MethodSignature {
            name: "calculate_total".to_string(),
            self_param: None,
            param_types: vec![
                "GodObject".to_string(),
                "Config".to_string(),
                "Context".to_string(),
                "Options".to_string(),
                "Metrics".to_string(),
            ],
            return_type: Some("usize".to_string()),
        },
    ];

    // Calculate split quality
    let report = detector.calculate_split_quality(&all_splits, &signatures);

    // Verify we detected anti-patterns
    assert!(
        !report.anti_patterns.is_empty(),
        "Should detect anti-patterns in behavioral splits"
    );

    // Should detect utilities module (critical severity)
    let has_utilities = report.anti_patterns.iter().any(|p| {
        p.pattern_type == AntiPatternType::UtilitiesModule
            && p.severity == AntiPatternSeverity::Critical
    });
    assert!(
        has_utilities,
        "Should detect utilities module anti-pattern"
    );

    // Should detect technical grouping (high severity)
    let has_technical_grouping = report.anti_patterns.iter().any(|p| {
        p.pattern_type == AntiPatternType::TechnicalGrouping
            && p.severity == AntiPatternSeverity::High
    });
    assert!(
        has_technical_grouping,
        "Should detect technical grouping anti-pattern"
    );

    // Should detect parameter passing (medium severity)
    let has_parameter_passing = report.anti_patterns.iter().any(|p| {
        p.pattern_type == AntiPatternType::ParameterPassing
            && p.severity == AntiPatternSeverity::Medium
    });
    assert!(
        has_parameter_passing,
        "Should detect parameter passing anti-pattern in calculate_total"
    );

    // Quality score should be significantly reduced due to anti-patterns
    assert!(
        report.quality_score < 60.0,
        "Quality score should be < 60.0 for anti-pattern-heavy splits, got {}",
        report.quality_score
    );

    // Should identify only 1 idiomatic split (the good one)
    assert!(
        report.idiomatic_splits <= 1,
        "Should have at most 1 idiomatic split, got {}",
        report.idiomatic_splits
    );
}

/// Test that quality score calculation follows the documented formula
#[test]
fn test_quality_score_formula() {
    let detector = AntiPatternDetector::new();

    // Create a split with only a critical anti-pattern (utilities)
    let critical_split = ModuleSplit {
        suggested_name: "utilities.rs".to_string(),
        methods_to_move: vec!["foo".to_string()],
        responsibility: "utilities".to_string(),
        estimated_lines: 50,
        method_count: 1,
        ..Default::default()
    };

    let report = detector.calculate_split_quality(&vec![critical_split], &[]);

    // Should have exactly 1 anti-pattern
    assert_eq!(report.anti_patterns.len(), 1);

    // Quality score should be 100 - 20 = 80 (one critical penalty)
    assert_eq!(
        report.quality_score, 80.0,
        "Expected 100 - 20 = 80, got {}",
        report.quality_score
    );
}

/// Test detection of mixed data types anti-pattern
#[test]
fn test_mixed_data_types_detection() {
    let detector = AntiPatternDetector::new();

    let split = ModuleSplit {
        suggested_name: "mixed_module.rs".to_string(),
        methods_to_move: vec![
            "process_item".to_string(),
            "handle_config".to_string(),
            "format_metrics".to_string(),
        ],
        responsibility: "mixed processing".to_string(),
        estimated_lines: 120,
        method_count: 3,
        ..Default::default()
    };

    // Create signatures with 3+ distinct non-primitive types
    let signatures = vec![
        MethodSignature {
            name: "process_item".to_string(),
            self_param: None,
            param_types: vec!["PriorityItem".to_string()],
            return_type: Some("Result".to_string()),
        },
        MethodSignature {
            name: "handle_config".to_string(),
            self_param: None,
            param_types: vec!["GodObjectConfig".to_string()],
            return_type: Some("bool".to_string()),
        },
        MethodSignature {
            name: "format_metrics".to_string(),
            self_param: None,
            param_types: vec!["ComplexityMetrics".to_string()],
            return_type: Some("String".to_string()),
        },
    ];

    let anti_patterns = detector.analyze_split(&split, &signatures);

    // Should detect mixed data types
    let has_mixed_types = anti_patterns.iter().any(|p| {
        p.pattern_type == AntiPatternType::MixedDataTypes
            && p.severity == AntiPatternSeverity::High
    });
    assert!(has_mixed_types, "Should detect mixed data types anti-pattern");
}

/// Test that good domain-based splits get high quality scores
#[test]
fn test_idiomatic_split_high_quality() {
    let detector = AntiPatternDetector::new();

    // Create well-designed domain splits
    let good_splits = vec![
        ModuleSplit {
            suggested_name: "priority_item.rs".to_string(),
            methods_to_move: vec![
                "calculate_priority".to_string(),
                "update_priority".to_string(),
            ],
            responsibility: "priority item management".to_string(),
            estimated_lines: 60,
            method_count: 2,
            ..Default::default()
        },
        ModuleSplit {
            suggested_name: "complexity_metrics.rs".to_string(),
            methods_to_move: vec!["score".to_string(), "aggregate".to_string()],
            responsibility: "complexity metrics".to_string(),
            estimated_lines: 70,
            method_count: 2,
            ..Default::default()
        },
    ];

    // Simple signatures without excessive parameters
    let signatures = vec![
        MethodSignature {
            name: "calculate_priority".to_string(),
            self_param: None,
            param_types: vec!["PriorityItem".to_string()],
            return_type: Some("f64".to_string()),
        },
        MethodSignature {
            name: "score".to_string(),
            self_param: None,
            param_types: vec!["ComplexityMetrics".to_string()],
            return_type: Some("f64".to_string()),
        },
    ];

    let report = detector.calculate_split_quality(&good_splits, &signatures);

    // Should have no or very few anti-patterns
    assert!(
        report.anti_patterns.len() <= 1,
        "Good splits should have minimal anti-patterns, got {}",
        report.anti_patterns.len()
    );

    // Quality score should be high (>= 90)
    assert!(
        report.quality_score >= 90.0,
        "Idiomatic splits should score >= 90, got {}",
        report.quality_score
    );

    // All splits should be considered idiomatic
    assert_eq!(
        report.idiomatic_splits,
        good_splits.len(),
        "All good splits should be idiomatic"
    );
}

/// Test that anti-patterns are sorted by severity
#[test]
fn test_anti_patterns_sorted_by_severity() {
    let detector = AntiPatternDetector::new();

    let splits = vec![
        ModuleSplit {
            suggested_name: "calculate.rs".to_string(),
            methods_to_move: vec!["calc".to_string()],
            responsibility: "calculation".to_string(),
            estimated_lines: 50,
            method_count: 1,
            ..Default::default()
        },
        ModuleSplit {
            suggested_name: "utilities.rs".to_string(),
            methods_to_move: vec!["util".to_string()],
            responsibility: "utilities".to_string(),
            estimated_lines: 50,
            method_count: 1,
            ..Default::default()
        },
    ];

    let report = detector.calculate_split_quality(&splits, &[]);

    // Should have at least 2 anti-patterns (technical + utilities)
    assert!(report.anti_patterns.len() >= 2);

    // Anti-patterns should be sorted by severity (descending)
    // First should be Critical or High, and severity should never increase
    for i in 1..report.anti_patterns.len() {
        assert!(
            report.anti_patterns[i - 1].severity >= report.anti_patterns[i].severity,
            "Anti-patterns should be sorted by severity (descending)"
        );
    }

    // Should have at least one Critical severity (utilities)
    let has_critical = report
        .anti_patterns
        .iter()
        .any(|p| p.severity == AntiPatternSeverity::Critical);
    assert!(has_critical, "Should have at least one Critical severity");
}

/// Test Display formatting produces readable output
#[test]
fn test_quality_report_display_formatting() {
    let detector = AntiPatternDetector::new();

    let split = ModuleSplit {
        suggested_name: "utilities.rs".to_string(),
        methods_to_move: vec!["foo".to_string(), "bar".to_string()],
        responsibility: "utilities".to_string(),
        estimated_lines: 100,
        method_count: 2,
        ..Default::default()
    };

    let report = detector.calculate_split_quality(&vec![split], &[]);

    // Format the report
    let output = format!("{}", report);

    // Should contain key sections
    assert!(output.contains("Split Quality Analysis"));
    assert!(output.contains("Quality Score:"));
    assert!(output.contains("Total Splits:"));
    assert!(output.contains("Anti-Patterns Found"));
    assert!(output.contains("CRITICAL"));
    assert!(output.contains("Utilities Module"));
    assert!(output.contains("Correction:"));
}
