/// Systematic tests to identify DebtItem deserialization bug
/// Tests each component in isolation, then composed structures
use debtmap::output::json::UnifiedJsonOutput;
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
            "methods_count": 5,
            "fields_count": 0,
            "responsibilities": 1,
            "is_god_object": false,
            "god_object_score": 0.0
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
                "methods_count": 5,
                "fields_count": 0,
                "responsibilities": 1,
                "is_god_object": false,
                "god_object_score": 0.0
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
                "methods_count": 5,
                "fields_count": 0,
                "responsibilities": 1,
                "is_god_object": false,
                "god_object_score": 0.0
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

#[test]
fn test_debt_item_file_variant_externally_tagged() {
    let json = r#"{
        "File": {
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
                    "methods_count": 5,
                    "fields_count": 0,
                    "responsibilities": 1,
                    "is_god_object": false,
                    "god_object_score": 0.0
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
        }
    }"#;

    eprintln!("Testing DebtItem::File deserialization...");
    let result: Result<DebtItem, _> = serde_json::from_str(json);

    if let Err(ref e) = result {
        eprintln!("ERROR: {}", e);
        eprintln!("This is the bug we're trying to fix!");
    }

    assert!(result.is_ok(), "DebtItem::File failed: {:?}", result.err());

    match result.unwrap() {
        DebtItem::File(item) => {
            assert_eq!(item.score, 50.0);
        }
        DebtItem::Function(_) => panic!("Deserialized as Function instead of File!"),
    }
}

#[test]
fn test_debt_item_file_variant_with_real_json_from_analyze() {
    // This is copied from actual debtmap analyze output
    let json = r#"{
        "File": {
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
                "god_object_indicators": {
                    "methods_count": 105,
                    "fields_count": 6,
                    "responsibilities": 7,
                    "is_god_object": true,
                    "god_object_score": 1.0,
                    "responsibility_names": ["Core Operations"],
                    "recommended_splits": []
                },
                "function_scores": []
            },
            "score": 165.4954756566064,
            "priority_rank": 0,
            "recommendation": "Split cache",
            "impact": {
                "complexity_reduction": 69.4,
                "maintainability_improvement": 16.54954756566064,
                "test_effort": 252.9
            }
        }
    }"#;

    eprintln!("Testing with REAL JSON from debtmap analyze...");
    let result: Result<DebtItem, _> = serde_json::from_str(json);

    if let Err(ref e) = result {
        eprintln!("REAL JSON ERROR: {}", e);
    }

    assert!(result.is_ok(), "Real JSON failed: {:?}", result.err());
}

#[test]
fn test_unified_json_output_with_file_items() {
    let json = r#"{
        "items": [
            {
                "File": {
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
                            "methods_count": 5,
                            "fields_count": 0,
                            "responsibilities": 1,
                            "is_god_object": false,
                            "god_object_score": 0.0
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

    eprintln!("Testing UnifiedJsonOutput with File items...");
    let result: Result<UnifiedJsonOutput, _> = serde_json::from_str(json);

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
        DebtItem::File(item) => assert_eq!(item.score, 50.0),
        DebtItem::Function(_) => panic!("Item deserialized as Function!"),
    }
}
