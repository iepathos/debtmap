//! Integration tests for output validation (spec 230)
//!
//! These tests validate the structure and invariants of debtmap's JSON output,
//! ensuring data quality and consistency.

use debtmap::output::unified::{
    CohesionClassification, CohesionOutput, Dependencies, FileDebtItemOutput, FileImpactOutput,
    FileMetricsOutput, FunctionDebtItemOutput, FunctionImpactOutput, FunctionMetricsOutput,
    Priority, PurityAnalysis, RecommendationOutput, UnifiedDebtItemOutput, UnifiedLocation,
    UnifiedOutput,
};
use debtmap::priority::{DebtType, FunctionRole};
use serde_json::Value;

/// Create a simple test DebtType for use in tests
fn test_debt_type() -> DebtType {
    DebtType::TestingGap {
        coverage: 0.5,
        cyclomatic: 5,
        cognitive: 3,
    }
}

/// Create a simple test FunctionRole for use in tests
fn test_function_role() -> FunctionRole {
    FunctionRole::Unknown
}

/// Test that the unified output structure matches expected schema
#[test]
fn test_unified_output_has_required_fields() {
    // Create a minimal valid output
    let output = UnifiedOutput {
        format_version: "2.0".to_string(),
        metadata: debtmap::output::unified::OutputMetadata {
            debtmap_version: "0.9.0".to_string(),
            generated_at: "2025-01-01T00:00:00Z".to_string(),
            project_root: None,
            analysis_type: "unified".to_string(),
        },
        summary: debtmap::output::unified::DebtSummary {
            total_items: 0,
            total_debt_score: 0.0,
            debt_density: 0.0,
            total_loc: 0,
            by_type: debtmap::output::unified::TypeBreakdown {
                file: 0,
                function: 0,
            },
            by_category: std::collections::HashMap::new(),
            score_distribution: debtmap::output::unified::ScoreDistribution {
                critical: 0,
                high: 0,
                medium: 0,
                low: 0,
            },
            cohesion: None,
        },
        items: vec![],
    };

    // Serialize to JSON
    let json_str = serde_json::to_string(&output).expect("Serialization failed");
    let json: Value = serde_json::from_str(&json_str).expect("Parse failed");

    // Validate required top-level fields
    assert!(json["format_version"].is_string());
    assert!(json["metadata"].is_object());
    assert!(json["summary"].is_object());
    assert!(json["items"].is_array());

    // Validate metadata fields
    assert!(json["metadata"]["debtmap_version"].is_string());
    assert!(json["metadata"]["generated_at"].is_string());
    assert!(json["metadata"]["analysis_type"].is_string());

    // Validate summary fields
    assert!(json["summary"]["total_items"].is_number());
    assert!(json["summary"]["total_debt_score"].is_number());
    assert!(json["summary"]["debt_density"].is_number());
    assert!(json["summary"]["total_loc"].is_number());
}

/// Test that function items have required fields and valid ranges
#[test]
fn test_function_item_required_fields_and_ranges() {
    let item = FunctionDebtItemOutput {
        score: 42.5,
        category: "Testing".to_string(),
        priority: Priority::Medium,
        location: UnifiedLocation {
            file: "test.rs".to_string(),
            line: Some(10),
            function: Some("test_fn".to_string()),
            file_context_label: None,
        },
        metrics: FunctionMetricsOutput {
            cyclomatic_complexity: 5,
            cognitive_complexity: 3,
            length: 20,
            nesting_depth: 2,
            coverage: Some(0.75),
            uncovered_lines: None,
            entropy_score: Some(0.5),
        },
        debt_type: test_debt_type(),
        function_role: test_function_role(),
        purity_analysis: Some(PurityAnalysis {
            is_pure: true,
            confidence: 0.9,
            side_effects: None,
        }),
        dependencies: Dependencies {
            upstream_count: 2,
            downstream_count: 3,
            upstream_callers: vec![],
            downstream_callees: vec![],
        },
        recommendation: RecommendationOutput {
            action: "Add tests".to_string(),
            priority: None,
            implementation_steps: vec![],
        },
        impact: FunctionImpactOutput {
            coverage_improvement: 0.2,
            complexity_reduction: 0.1,
            risk_reduction: 0.15,
        },
        scoring_details: None,
        adjusted_complexity: None,
        complexity_pattern: None,
        pattern_type: None,
        pattern_confidence: None,
        pattern_details: None,
        context: None,
    };

    // Serialize to JSON
    let json_str = serde_json::to_string(&item).expect("Serialization failed");
    let json: Value = serde_json::from_str(&json_str).expect("Parse failed");

    // Validate required fields exist
    assert!(json["score"].is_number());
    assert!(json["category"].is_string());
    assert!(json["priority"].is_string());
    assert!(json["location"].is_object());
    assert!(json["metrics"].is_object());

    // Validate score is non-negative
    let score = json["score"].as_f64().unwrap();
    assert!(score >= 0.0, "Score must be non-negative: {}", score);

    // Validate coverage is in valid range
    if let Some(coverage) = json["metrics"]["coverage"].as_f64() {
        assert!(
            (0.0..=1.0).contains(&coverage),
            "Coverage must be in [0, 1]: {}",
            coverage
        );
    }

    // Validate entropy is in valid range
    if let Some(entropy) = json["metrics"]["entropy_score"].as_f64() {
        assert!(
            (0.0..=1.0).contains(&entropy),
            "Entropy must be in [0, 1]: {}",
            entropy
        );
    }
}

/// Test that file items have required fields and valid ranges
#[test]
fn test_file_item_required_fields_and_ranges() {
    let item = FileDebtItemOutput {
        score: 75.25,
        category: "Architecture".to_string(),
        priority: Priority::High,
        location: UnifiedLocation {
            file: "big_file.rs".to_string(),
            line: None,
            function: None,
            file_context_label: None,
        },
        metrics: FileMetricsOutput {
            lines: 500,
            functions: 25,
            classes: 0,
            avg_complexity: 8.5,
            max_complexity: 15,
            total_complexity: 212,
            coverage: 0.65,
            uncovered_lines: 175,
        },
        god_object_indicators: None,
        dependencies: None,
        anti_patterns: None,
        cohesion: Some(CohesionOutput {
            score: 0.45,
            internal_calls: 10,
            external_calls: 15,
            classification: CohesionClassification::Medium,
            functions_analyzed: 25,
        }),
        recommendation: RecommendationOutput {
            action: "Split file".to_string(),
            priority: None,
            implementation_steps: vec![],
        },
        impact: FileImpactOutput {
            complexity_reduction: 0.3,
            maintainability_improvement: 0.4,
            test_effort: 0.5,
        },
        scoring_details: None,
    };

    // Serialize to JSON
    let json_str = serde_json::to_string(&item).expect("Serialization failed");
    let json: Value = serde_json::from_str(&json_str).expect("Parse failed");

    // Validate required fields exist
    assert!(json["score"].is_number());
    assert!(json["category"].is_string());
    assert!(json["priority"].is_string());
    assert!(json["location"].is_object());
    assert!(json["metrics"].is_object());

    // Validate score is non-negative
    let score = json["score"].as_f64().unwrap();
    assert!(score >= 0.0, "Score must be non-negative: {}", score);

    // Validate coverage is in valid range
    let coverage = json["metrics"]["coverage"].as_f64().unwrap();
    assert!(
        (0.0..=1.0).contains(&coverage),
        "Coverage must be in [0, 1]: {}",
        coverage
    );

    // Validate cohesion score is in valid range
    if let Some(cohesion_score) = json["cohesion"]["score"].as_f64() {
        assert!(
            (0.0..=1.0).contains(&cohesion_score),
            "Cohesion must be in [0, 1]: {}",
            cohesion_score
        );
    }
}

/// Test priority consistency with score thresholds
#[test]
fn test_priority_matches_score_thresholds() {
    let test_cases = vec![
        (150.0, "critical"),
        (100.0, "critical"),
        (99.99, "high"),
        (50.0, "high"),
        (49.99, "medium"),
        (20.0, "medium"),
        (19.99, "low"),
        (0.0, "low"),
    ];

    for (score, expected_priority) in test_cases {
        let item = UnifiedDebtItemOutput::Function(Box::new(FunctionDebtItemOutput {
            score,
            category: "Test".to_string(),
            priority: priority_from_score(score),
            location: UnifiedLocation {
                file: "test.rs".to_string(),
                line: Some(1),
                function: Some("test".to_string()),
                file_context_label: None,
            },
            metrics: FunctionMetricsOutput {
                cyclomatic_complexity: 1,
                cognitive_complexity: 1,
                length: 1,
                nesting_depth: 0,
                coverage: None,
                uncovered_lines: None,
                entropy_score: None,
            },
            debt_type: test_debt_type(),
            function_role: test_function_role(),
            purity_analysis: None,
            dependencies: Dependencies {
                upstream_count: 0,
                downstream_count: 0,
                upstream_callers: vec![],
                downstream_callees: vec![],
            },
            recommendation: RecommendationOutput {
                action: "".to_string(),
                priority: None,
                implementation_steps: vec![],
            },
            impact: FunctionImpactOutput {
                coverage_improvement: 0.0,
                complexity_reduction: 0.0,
                risk_reduction: 0.0,
            },
            scoring_details: None,
            adjusted_complexity: None,
            complexity_pattern: None,
            pattern_type: None,
            pattern_confidence: None,
            pattern_details: None,
            context: None,
        }));

        let json_str = serde_json::to_string(&item).expect("Serialization failed");
        let json: Value = serde_json::from_str(&json_str).expect("Parse failed");

        let priority = json["priority"].as_str().unwrap();
        assert_eq!(
            priority, expected_priority,
            "Score {} should have priority {}, got {}",
            score, expected_priority, priority
        );
    }
}

/// Test that no floating-point noise patterns appear in output
#[test]
fn test_no_floating_point_noise_in_serialized_output() {
    let item = FunctionDebtItemOutput {
        score: 42.57,
        category: "Testing".to_string(),
        priority: Priority::Medium,
        location: UnifiedLocation {
            file: "test.rs".to_string(),
            line: Some(10),
            function: Some("test_fn".to_string()),
            file_context_label: None,
        },
        metrics: FunctionMetricsOutput {
            cyclomatic_complexity: 5,
            cognitive_complexity: 3,
            length: 20,
            nesting_depth: 2,
            coverage: Some(0.8),
            uncovered_lines: None,
            entropy_score: Some(0.5),
        },
        debt_type: test_debt_type(),
        function_role: test_function_role(),
        purity_analysis: None,
        dependencies: Dependencies {
            upstream_count: 0,
            downstream_count: 0,
            upstream_callers: vec![],
            downstream_callees: vec![],
        },
        recommendation: RecommendationOutput {
            action: "Add tests".to_string(),
            priority: None,
            implementation_steps: vec![],
        },
        impact: FunctionImpactOutput {
            coverage_improvement: 0.2,
            complexity_reduction: 0.1,
            risk_reduction: 0.15,
        },
        scoring_details: None,
        adjusted_complexity: None,
        complexity_pattern: None,
        pattern_type: None,
        pattern_confidence: None,
        pattern_details: None,
        context: None,
    };

    let json_str = serde_json::to_string(&item).expect("Serialization failed");

    // Check for common floating-point noise patterns
    let noise_patterns = [
        "9999999999", // e.g., 1.9999999999
        "0000000001", // e.g., 1.0000000001
        "9999999998", // e.g., 42.9999999998
    ];

    for pattern in noise_patterns {
        assert!(
            !json_str.contains(pattern),
            "Found floating-point noise pattern '{}' in output: {}",
            pattern,
            json_str
        );
    }
}

/// Test cohesion classification consistency
#[test]
fn test_cohesion_classification_consistency() {
    let test_cases = vec![
        (0.0, "low"),
        (0.39, "low"),
        (0.4, "medium"),
        (0.69, "medium"),
        (0.7, "high"),
        (1.0, "high"),
    ];

    for (score, expected_classification) in test_cases {
        let cohesion = CohesionOutput {
            score,
            internal_calls: 5,
            external_calls: 5,
            classification: CohesionClassification::from_score(score),
            functions_analyzed: 10,
        };

        let json_str = serde_json::to_string(&cohesion).expect("Serialization failed");
        let json: Value = serde_json::from_str(&json_str).expect("Parse failed");

        let classification = json["classification"].as_str().unwrap();
        assert_eq!(
            classification, expected_classification,
            "Score {} should have classification {}, got {}",
            score, expected_classification, classification
        );
    }
}

/// Test that tagged enum serialization works correctly
#[test]
fn test_unified_debt_item_tagged_serialization() {
    let function_item = UnifiedDebtItemOutput::Function(Box::new(FunctionDebtItemOutput {
        score: 25.0,
        category: "Testing".to_string(),
        priority: Priority::Medium,
        location: UnifiedLocation {
            file: "test.rs".to_string(),
            line: Some(1),
            function: Some("test".to_string()),
            file_context_label: None,
        },
        metrics: FunctionMetricsOutput {
            cyclomatic_complexity: 1,
            cognitive_complexity: 1,
            length: 1,
            nesting_depth: 0,
            coverage: None,
            uncovered_lines: None,
            entropy_score: None,
        },
        debt_type: test_debt_type(),
        function_role: test_function_role(),
        purity_analysis: None,
        dependencies: Dependencies {
            upstream_count: 0,
            downstream_count: 0,
            upstream_callers: vec![],
            downstream_callees: vec![],
        },
        recommendation: RecommendationOutput {
            action: "".to_string(),
            priority: None,
            implementation_steps: vec![],
        },
        impact: FunctionImpactOutput {
            coverage_improvement: 0.0,
            complexity_reduction: 0.0,
            risk_reduction: 0.0,
        },
        scoring_details: None,
        adjusted_complexity: None,
        complexity_pattern: None,
        pattern_type: None,
        pattern_confidence: None,
        pattern_details: None,
        context: None,
    }));

    let json_str = serde_json::to_string(&function_item).expect("Serialization failed");
    let json: Value = serde_json::from_str(&json_str).expect("Parse failed");

    // Verify the "type" tag is present
    assert_eq!(json["type"].as_str().unwrap(), "Function");
}

/// Helper function to get Priority from score (mirrors the internal logic)
fn priority_from_score(score: f64) -> Priority {
    if score >= 100.0 {
        Priority::Critical
    } else if score >= 50.0 {
        Priority::High
    } else if score >= 20.0 {
        Priority::Medium
    } else {
        Priority::Low
    }
}
