/// Systematic tests to identify DebtItem deserialization bug
/// Tests each component in isolation, then composed structures
use debtmap::commands::compare_debtmap::DebtmapJsonInput;
use debtmap::output::unified::UnifiedDebtItemOutput;
use debtmap::priority::{DebtItem, FileDebtItem, FileDebtMetrics, FileImpact};

#[test]
fn test_file_debt_metrics_minimal() {
    let json = r#"{
        "path": "./test.rs",
        "total_lines": 100,
        "function_count": 5,
        "class_count": 1,
        "avg_complexity": 3.0,
        "max_complexity": 10,
        "total_complexity": 50,
        "coverage_percent": 0.5,
        "uncovered_lines": 50,
        "god_object_indicators": {
            "method_count": 5,
            "field_count": 0,
            "responsibility_count": 1,
            "lines_of_code": 100,
            "complexity_sum": 50,
            "is_god_object": false,
            "god_object_score": 0.0,
            "responsibilities": ["General"],
            "recommended_splits": [],
            "confidence": "NotGodObject",
            "detection_type": "GodFile"
        },
        "function_scores": []
    }"#;

    let result: Result<FileDebtMetrics, _> = serde_json::from_str(json);
    assert!(result.is_ok(), "FileDebtMetrics failed: {:?}", result.err());
}

#[test]
fn test_file_impact_deserialization() {
    let json = r#"{
        "complexity_reduction": 10.0,
        "maintainability_improvement": 5.0,
        "test_effort": 2.0
    }"#;

    let result: Result<FileImpact, _> = serde_json::from_str(json);
    assert!(result.is_ok(), "FileImpact failed: {:?}", result.err());
}

#[test]
fn test_file_debt_item_full() {
    let json = r#"{
        "metrics": {
            "path": "./test.rs",
            "total_lines": 100,
            "function_count": 5,
            "class_count": 1,
            "avg_complexity": 3.0,
            "max_complexity": 10,
            "total_complexity": 50,
            "coverage_percent": 0.5,
            "uncovered_lines": 50,
            "god_object_indicators": {
                "method_count": 5,
                "field_count": 0,
                "responsibility_count": 1,
                "lines_of_code": 100,
                "complexity_sum": 50,
                "is_god_object": false,
                "god_object_score": 0.0,
                "responsibilities": ["General"],
                "recommended_splits": [],
                "confidence": "NotGodObject",
                "detection_type": "GodFile"
            },
            "function_scores": []
        },
        "score": 50.0,
        "priority_rank": 1,
        "recommendation": "Test",
        "impact": {
            "complexity_reduction": 10.0,
            "maintainability_improvement": 5.0,
            "test_effort": 2.0
        }
    }"#;

    let result: Result<FileDebtItem, _> = serde_json::from_str(json);
    assert!(
        result.is_ok(),
        "FileDebtItem full failed: {:?}",
        result.err()
    );

    let item = result.unwrap();
    assert_eq!(item.score, 50.0);
    assert_eq!(item.priority_rank, 1);
}

#[test]
fn test_file_debt_item_minimal_with_defaults() {
    // Test with only metrics field, others should default
    let json = r#"{
        "metrics": {
            "path": "./test.rs",
            "total_lines": 100,
            "function_count": 5,
            "class_count": 1,
            "avg_complexity": 3.0,
            "max_complexity": 10,
            "total_complexity": 50,
            "coverage_percent": 0.5,
            "uncovered_lines": 50,
            "god_object_indicators": {
                "method_count": 5,
                "field_count": 0,
                "responsibility_count": 1,
                "lines_of_code": 100,
                "complexity_sum": 50,
                "is_god_object": false,
                "god_object_score": 0.0,
                "responsibilities": ["General"],
                "recommended_splits": [],
                "confidence": "NotGodObject",
                "detection_type": "GodFile"
            },
            "function_scores": []
        }
    }"#;

    let result: Result<FileDebtItem, _> = serde_json::from_str(json);
    assert!(
        result.is_ok(),
        "FileDebtItem minimal failed: {:?}",
        result.err()
    );

    let item = result.unwrap();
    assert_eq!(item.score, 0.0); // Should use default
    assert_eq!(item.priority_rank, 0); // Should use default
}

/// Test DebtItem::File with internally tagged format (using "type": "File")
/// This is the new format after adding #[serde(tag = "type")] to DebtItem
#[test]
fn test_debt_item_file_variant_internally_tagged() {
    let json = r#"{
        "type": "File",
        "metrics": {
            "path": "./test.rs",
            "total_lines": 100,
            "function_count": 5,
            "class_count": 1,
            "avg_complexity": 3.0,
            "max_complexity": 10,
            "total_complexity": 50,
            "coverage_percent": 0.5,
            "uncovered_lines": 50,
            "god_object_indicators": {
                "method_count": 5,
                "field_count": 0,
                "responsibility_count": 1,
                "lines_of_code": 100,
                "complexity_sum": 50,
                "is_god_object": false,
                "god_object_score": 0.0,
                "responsibilities": ["General"],
                "recommended_splits": [],
                "confidence": "NotGodObject",
                "detection_type": "GodFile"
            },
            "function_scores": []
        },
        "score": 50.0,
        "priority_rank": 1,
        "recommendation": "Test",
        "impact": {
            "complexity_reduction": 10.0,
            "maintainability_improvement": 5.0,
            "test_effort": 2.0
        }
    }"#;

    eprintln!("Testing DebtItem::File deserialization with internal tagging...");
    let result: Result<DebtItem, _> = serde_json::from_str(json);

    if let Err(ref e) = result {
        eprintln!("ERROR: {}", e);
    }

    assert!(result.is_ok(), "DebtItem::File failed: {:?}", result.err());

    match result.unwrap() {
        DebtItem::File(item) => {
            assert_eq!(item.score, 50.0);
        }
        DebtItem::Function(_) => panic!("Deserialized as Function instead of File!"),
    }
}

/// Test that DebtItem deserialization with internal tagging format works for real-world data
#[test]
fn test_debt_item_file_variant_with_real_data() {
    // Minimal test that verifies DebtItem can be deserialized with "type" tag
    let json = r#"{
        "type": "File",
        "metrics": {
            "path": "./src/cache/shared_cache.rs",
            "total_lines": 2529,
            "function_count": 129,
            "class_count": 4,
            "avg_complexity": 2.689922480620155,
            "max_complexity": 13,
            "total_complexity": 347,
            "coverage_percent": 0.0,
            "uncovered_lines": 2529,
            "function_scores": []
        },
        "score": 165.5,
        "priority_rank": 0,
        "recommendation": "Split cache",
        "impact": {
            "complexity_reduction": 69.4,
            "maintainability_improvement": 16.5,
            "test_effort": 252.9
        }
    }"#;

    eprintln!("Testing with internally tagged DebtItem...");
    let result: Result<DebtItem, _> = serde_json::from_str(json);

    if let Err(ref e) = result {
        eprintln!("ERROR: {}", e);
    }

    assert!(
        result.is_ok(),
        "DebtItem deserialization failed: {:?}",
        result.err()
    );
}

/// Test deserialization of DebtmapJsonInput with File items in the output format.
/// This uses the output format with "type": "File" tagging.
#[test]
fn test_unified_json_output_with_file_items() {
    // Use the actual output format with type-tagged items
    let json = r#"{
        "items": [
            {
                "type": "File",
                "score": 50.0,
                "category": "Architecture",
                "priority": "high",
                "location": {
                    "file": "./test.rs"
                },
                "metrics": {
                    "lines": 100,
                    "functions": 5,
                    "classes": 1,
                    "avg_complexity": 3.0,
                    "max_complexity": 10,
                    "total_complexity": 50,
                    "coverage": 0.5,
                    "uncovered_lines": 50
                },
                "recommendation": {
                    "action": "Refactor file",
                    "implementation_steps": []
                },
                "impact": {
                    "complexity_reduction": 10.0,
                    "maintainability_improvement": 5.0,
                    "test_effort": 2.0
                }
            }
        ],
        "total_impact": {
            "coverage_improvement": 0.0,
            "lines_reduction": 0,
            "complexity_reduction": 0.0,
            "risk_reduction": 0.0
        },
        "total_debt_score": 0.0,
        "debt_density": 0.0,
        "total_lines_of_code": 0,
        "overall_coverage": null
    }"#;

    eprintln!("Testing DebtmapJsonInput with File items in output format...");
    let result: Result<DebtmapJsonInput, _> = serde_json::from_str(json);

    if let Err(ref e) = result {
        eprintln!("UnifiedJsonOutput ERROR: {}", e);
    }

    assert!(
        result.is_ok(),
        "UnifiedJsonOutput failed: {:?}",
        result.err()
    );

    let output = result.unwrap();
    assert_eq!(output.items.len(), 1);
    match &output.items[0] {
        UnifiedDebtItemOutput::File(item) => assert_eq!(item.score, 50.0),
        UnifiedDebtItemOutput::Function(_) => panic!("Item deserialized as Function!"),
    }
}
