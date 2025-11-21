//! Integration tests for confidence-based responsibility classification (Spec 174).
//!
//! These tests validate:
//! - Utilities classification rate is below 10%
//! - Module splits require minimum confidence threshold
//! - Low-confidence classifications are properly rejected

use debtmap::organization::{
    emit_classification_metrics, infer_responsibility_with_confidence,
    recommend_module_splits_with_evidence, ClassificationMetrics,
};
use std::collections::HashMap;

/// Test that utilities classification rate is below 10% threshold (Spec 174).
///
/// This validates that the confidence-based approach successfully reduces
/// the utilities classification rate from the previous ~30% to below 10%.
#[test]
fn test_utilities_classification_reduced() {
    // Sample of diverse method names that should NOT all be classified as utilities
    let method_names = vec![
        "save_to_database",
        "load_from_file",
        "calculate_total",
        "validate_input",
        "format_output",
        "parse_json",
        "send_email",
        "log_error",
        "handle_request",
        "process_data",
        "get_user",
        "set_config",
        "check_permissions",
        "render_template",
        "execute_query",
        "transform_data",
        "build_response",
        "encode_value",
        "decode_value",
        "serialize_object",
        "deserialize_object",
        "validate_schema",
        "normalize_data",
        "sanitize_input",
        "authenticate_user",
        "authorize_action",
        "hash_password",
        "verify_token",
        "encrypt_data",
        "decrypt_data",
        "compress_file",
        "decompress_file",
        "monitor_health",
        "track_metrics",
        "audit_action",
        "schedule_task",
        "cancel_task",
        "retry_operation",
        "rollback_transaction",
        "commit_transaction",
        // Some actual utility-like methods
        "to_string",
        "from_str",
        "clone_data",
        "copy_values",
    ];

    let mut metrics = ClassificationMetrics::new();

    for method in &method_names {
        let result = infer_responsibility_with_confidence(method, None);
        metrics.record_classification(result.category.as_deref());
    }

    // Emit metrics for observability
    emit_classification_metrics(&metrics);

    // Validate utilities rate is below 10%
    let utilities_rate = metrics.utilities_rate();
    assert!(
        utilities_rate < 0.10,
        "Utilities classification rate {:.1}% exceeds 10% threshold",
        utilities_rate * 100.0
    );

    // Validate we still classify a reasonable percentage
    let classification_rate = metrics.classification_rate();
    assert!(
        classification_rate >= 0.40,
        "Classification rate {:.1}% is too low (may be rejecting too many)",
        classification_rate * 100.0
    );
}

/// Test that module splits require high confidence (>= 0.65).
///
/// This validates that `recommend_module_splits_with_evidence` properly
/// filters out low-confidence responsibility groups.
#[test]
fn test_module_splits_require_high_confidence() {
    use debtmap::analysis::multi_signal_aggregation::{
        AggregatedClassification, ResponsibilityCategory,
    };

    let type_name = "GodObjectExample";
    let methods = vec![
        "save_data".to_string(),
        "load_data".to_string(),
        "validate_data".to_string(),
        "process_data".to_string(),
        "format_data".to_string(),
        "send_data".to_string(),
    ];

    // Create responsibility groups
    let mut responsibility_groups = HashMap::new();
    responsibility_groups.insert("data_persistence".to_string(), methods.clone());

    // Test 1: Low confidence (0.55) - should NOT recommend split
    let mut evidence_map_low = HashMap::new();
    evidence_map_low.insert(
        "data_persistence".to_string(),
        AggregatedClassification {
            primary: ResponsibilityCategory::DatabaseIO,
            confidence: 0.55, // Below MODULE_SPLIT_CONFIDENCE (0.65)
            evidence: vec![],
            alternatives: vec![],
        },
    );

    let splits_low = recommend_module_splits_with_evidence(
        type_name,
        &[],
        &responsibility_groups,
        &evidence_map_low,
    );

    assert_eq!(
        splits_low.len(),
        0,
        "Should NOT recommend split for low confidence (0.55 < 0.65)"
    );

    // Test 2: High confidence (0.75) - should recommend split
    let mut evidence_map_high = HashMap::new();
    evidence_map_high.insert(
        "data_persistence".to_string(),
        AggregatedClassification {
            primary: ResponsibilityCategory::DatabaseIO,
            confidence: 0.75, // Above MODULE_SPLIT_CONFIDENCE (0.65)
            evidence: vec![],
            alternatives: vec![],
        },
    );

    let splits_high = recommend_module_splits_with_evidence(
        type_name,
        &[],
        &responsibility_groups,
        &evidence_map_high,
    );

    assert_eq!(
        splits_high.len(),
        1,
        "Should recommend split for high confidence (0.75 >= 0.65)"
    );

    // Validate the recommended split has the expected properties
    let split = &splits_high[0];
    assert_eq!(split.methods_to_move.len(), 6);
    assert_eq!(split.responsibility, "data_persistence");
}

/// Test that minimum method count is enforced for module splits.
///
/// Even with high confidence, splits should only be recommended for
/// groups with at least MIN_METHODS_FOR_SPLIT (5) methods.
#[test]
fn test_module_splits_enforce_minimum_methods() {
    use debtmap::analysis::multi_signal_aggregation::{
        AggregatedClassification, ResponsibilityCategory,
    };

    let type_name = "SmallClass";

    // Group with only 4 methods (below MIN_METHODS_FOR_SPLIT = 5)
    let small_methods = vec![
        "read_config".to_string(),
        "write_config".to_string(),
        "validate_config".to_string(),
        "parse_config".to_string(),
    ];

    let mut responsibility_groups_small = HashMap::new();
    responsibility_groups_small.insert("configuration".to_string(), small_methods);

    let mut evidence_map = HashMap::new();
    evidence_map.insert(
        "configuration".to_string(),
        AggregatedClassification {
            primary: ResponsibilityCategory::ConfigurationIO,
            confidence: 0.85, // High confidence
            evidence: vec![],
            alternatives: vec![],
        },
    );

    let splits = recommend_module_splits_with_evidence(
        type_name,
        &[],
        &responsibility_groups_small,
        &evidence_map,
    );

    assert_eq!(
        splits.len(),
        0,
        "Should NOT recommend split for only 4 methods (min is 5)"
    );

    // Now test with 6 methods (above threshold)
    let sufficient_methods = vec![
        "read_config".to_string(),
        "write_config".to_string(),
        "validate_config".to_string(),
        "parse_config".to_string(),
        "merge_config".to_string(),
        "update_config".to_string(),
    ];

    let mut responsibility_groups_large = HashMap::new();
    responsibility_groups_large.insert("configuration".to_string(), sufficient_methods);

    let splits_large = recommend_module_splits_with_evidence(
        type_name,
        &[],
        &responsibility_groups_large,
        &evidence_map,
    );

    assert_eq!(
        splits_large.len(),
        1,
        "Should recommend split for 6 methods (above min of 5)"
    );
}

/// Test that ClassificationMetrics correctly tracks and calculates rates.
#[test]
fn test_classification_metrics_tracking() {
    let mut metrics = ClassificationMetrics::new();

    // Record 10 classifications: 7 classified, 3 unclassified, 1 utilities
    metrics.record_classification(Some("data_persistence"));
    metrics.record_classification(Some("validation"));
    metrics.record_classification(Some("formatting"));
    metrics.record_classification(Some("utilities"));
    metrics.record_classification(Some("authentication"));
    metrics.record_classification(Some("authorization"));
    metrics.record_classification(Some("logging"));
    metrics.record_classification(None); // unclassified
    metrics.record_classification(None); // unclassified
    metrics.record_classification(None); // unclassified

    assert_eq!(metrics.total_methods, 10);
    assert_eq!(metrics.classified_methods, 7);
    assert_eq!(metrics.unclassified_methods, 3);
    assert_eq!(metrics.utilities_count, 1);

    // Check calculated rates
    assert_eq!(metrics.utilities_rate(), 0.1); // 1/10 = 10%
    assert_eq!(metrics.classification_rate(), 0.7); // 7/10 = 70%
}

/// Test that emit_classification_metrics doesn't panic and produces valid output.
#[test]
fn test_emit_classification_metrics_valid() {
    let mut metrics = ClassificationMetrics::new();

    // Normal case
    for _ in 0..20 {
        metrics.record_classification(Some("data_persistence"));
    }
    for _ in 0..5 {
        metrics.record_classification(None);
    }

    // Should not panic
    emit_classification_metrics(&metrics);

    // Edge case: empty metrics
    let empty_metrics = ClassificationMetrics::new();
    emit_classification_metrics(&empty_metrics);
}
