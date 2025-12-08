// Integration tests for blog sample code recommendations (spec 178)
//
// These tests validate that real-world functions from blog samples
// receive appropriate recommendations based on their complexity profile.

use debtmap::core::FunctionMetrics;
use debtmap::priority::scoring::concise_recommendation::generate_concise_recommendation;
use debtmap::priority::semantic_classifier::FunctionRole;
use debtmap::priority::DebtType;
use std::path::PathBuf;

/// Create metrics matching blog sample reconcile_state function
/// Profile: cyclomatic=9, cognitive=16, moderate complexity
fn create_reconcile_state_metrics() -> FunctionMetrics {
    FunctionMetrics {
        name: "reconcile_state".to_string(),
        file: PathBuf::from("state_manager.rs"),
        line: 42,
        cyclomatic: 9,
        cognitive: 16,
        nesting: 3,
        length: 85,
        is_test: false,
        visibility: None,
        is_trait_method: false,
        in_test_module: false,
        entropy_score: None,
        is_pure: None,
        purity_confidence: None,
        purity_reason: None,
        call_dependencies: None,
        detected_patterns: None,
        upstream_callers: None,
        downstream_callees: None,
        mapping_pattern_result: None,
        adjusted_complexity: None,
        composition_metrics: None,
        language_specific: None,
        purity_level: None,
        error_swallowing_count: None,
        error_swallowing_patterns: None,
    }
}

/// Create metrics matching blog sample validate_config function
/// Profile: cyclomatic=6, cognitive=6, low complexity
fn create_validate_config_metrics() -> FunctionMetrics {
    FunctionMetrics {
        name: "validate_config".to_string(),
        file: PathBuf::from("config.rs"),
        line: 15,
        cyclomatic: 6,
        cognitive: 6,
        nesting: 2,
        length: 30,
        is_test: false,
        visibility: None,
        is_trait_method: false,
        in_test_module: false,
        entropy_score: None,
        is_pure: None,
        purity_confidence: None,
        purity_reason: None,
        call_dependencies: None,
        detected_patterns: None,
        upstream_callers: None,
        downstream_callees: None,
        mapping_pattern_result: None,
        adjusted_complexity: None,
        composition_metrics: None,
        language_specific: None,
        purity_level: None,
        error_swallowing_count: None,
        error_swallowing_patterns: None,
    }
}

#[test]
fn test_blog_sample_reconcile_state_recommendation() {
    // Blog sample: reconcile_state with cyclo=9, cognitive=16
    // Should get MODERATE tier recommendation (optional preventive refactoring)
    let metrics = create_reconcile_state_metrics();

    let rec = generate_concise_recommendation(
        &DebtType::ComplexityHotspot {
            cyclomatic: 9,
            cognitive: 16,
            adjusted_cyclomatic: None,
        },
        &metrics,
        FunctionRole::PureLogic,
        &None,
    )
    .expect("Test should generate recommendation");

    // Verify it's treated as moderate complexity
    assert!(
        rec.primary_action.contains("Optional") || rec.primary_action.contains("Reduce"),
        "reconcile_state (9/16) should get preventive refactoring recommendation, got: {}",
        rec.primary_action
    );

    // Should NOT suggest increasing complexity
    assert!(
        !rec.primary_action.contains("from 9 to ~10"),
        "Should not suggest increasing complexity from 9 to 10"
    );

    // Should suggest reducing to single digit (5-6) since current is 9
    assert!(
        rec.primary_action.contains("to ~5") || rec.primary_action.contains("to ~6"),
        "Should suggest reducing to 5-6, got: {}",
        rec.primary_action
    );

    // Should have actionable steps
    assert!(rec.steps.is_some());
    let steps = rec.steps.unwrap();
    assert!(!steps.is_empty(), "Should have actionable steps");
    assert!(steps.len() <= 5, "Should have max 5 steps");

    // Rationale should mention moderate complexity
    assert!(
        rec.rationale.contains("Moderate") || rec.rationale.contains("moderate"),
        "Rationale should mention moderate complexity, got: {}",
        rec.rationale
    );
}

#[test]
fn test_blog_sample_validate_config_recommendation() {
    // Blog sample: validate_config with cyclo=6, cognitive=6
    // Should get LOW tier recommendation (maintain current patterns)
    let metrics = create_validate_config_metrics();

    let rec = generate_concise_recommendation(
        &DebtType::ComplexityHotspot {
            cyclomatic: 6,
            cognitive: 6,
            adjusted_cyclomatic: None,
        },
        &metrics,
        FunctionRole::PureLogic,
        &None,
    )
    .expect("Test should generate recommendation");

    // Verify it's treated as low complexity
    assert!(
        rec.primary_action.contains("Maintain"),
        "validate_config (6/6) should get maintenance recommendation, got: {}",
        rec.primary_action
    );

    // Should NOT suggest refactoring
    assert!(
        !rec.primary_action.contains("Reduce"),
        "Low complexity should not suggest reduction, got: {}",
        rec.primary_action
    );

    // Rationale should mention low complexity
    assert!(
        rec.rationale.contains("low complexity"),
        "Rationale should mention low complexity, got: {}",
        rec.rationale
    );

    // Should have simple steps (like add tests for safety)
    assert!(rec.steps.is_some());
    let steps = rec.steps.unwrap();
    assert!(!steps.is_empty(), "Should have at least one step");
}

#[test]
fn test_blog_samples_complexity_tier_boundaries() {
    // Test the boundary between low and moderate tiers
    // Low tier: cyclo < 8, cognitive < 15
    // Moderate tier: cyclo 8-14, cognitive 15-24

    // Just below moderate threshold (7/14) - should be LOW
    let low_metrics = FunctionMetrics {
        name: "just_below_threshold".to_string(),
        file: PathBuf::from("test.rs"),
        line: 1,
        cyclomatic: 7,
        cognitive: 14,
        nesting: 2,
        length: 50,
        is_test: false,
        visibility: None,
        is_trait_method: false,
        in_test_module: false,
        entropy_score: None,
        is_pure: None,
        purity_confidence: None,
        purity_reason: None,
        call_dependencies: None,
        detected_patterns: None,
        upstream_callers: None,
        downstream_callees: None,
        mapping_pattern_result: None,
        adjusted_complexity: None,
        composition_metrics: None,
        language_specific: None,
        purity_level: None,
        error_swallowing_count: None,
        error_swallowing_patterns: None,
    };

    let rec_low = generate_concise_recommendation(
        &DebtType::ComplexityHotspot {
            cyclomatic: 7,
            cognitive: 14,
            adjusted_cyclomatic: None,
        },
        &low_metrics,
        FunctionRole::PureLogic,
        &None,
    )
    .expect("Test should generate recommendation");

    assert!(
        rec_low.primary_action.contains("Maintain"),
        "7/14 should be LOW tier (maintain), got: {}",
        rec_low.primary_action
    );

    // Just at moderate threshold (8/15) - should be MODERATE
    let moderate_metrics = FunctionMetrics {
        name: "at_threshold".to_string(),
        file: PathBuf::from("test.rs"),
        line: 1,
        cyclomatic: 8,
        cognitive: 15,
        nesting: 2,
        length: 50,
        is_test: false,
        visibility: None,
        is_trait_method: false,
        in_test_module: false,
        entropy_score: None,
        is_pure: None,
        purity_confidence: None,
        purity_reason: None,
        call_dependencies: None,
        detected_patterns: None,
        upstream_callers: None,
        downstream_callees: None,
        mapping_pattern_result: None,
        adjusted_complexity: None,
        composition_metrics: None,
        language_specific: None,
        purity_level: None,
        error_swallowing_count: None,
        error_swallowing_patterns: None,
    };

    let rec_moderate = generate_concise_recommendation(
        &DebtType::ComplexityHotspot {
            cyclomatic: 8,
            cognitive: 15,
            adjusted_cyclomatic: None,
        },
        &moderate_metrics,
        FunctionRole::PureLogic,
        &None,
    )
    .expect("Test should generate recommendation");

    assert!(
        rec_moderate.primary_action.contains("Optional")
            || rec_moderate.primary_action.contains("Reduce"),
        "8/15 should be MODERATE tier (preventive), got: {}",
        rec_moderate.primary_action
    );
}

#[test]
fn test_blog_sample_recommendations_have_effort_estimates() {
    // Both blog samples should have effort estimates
    let reconcile_metrics = create_reconcile_state_metrics();
    let validate_metrics = create_validate_config_metrics();

    let rec1 = generate_concise_recommendation(
        &DebtType::ComplexityHotspot {
            cyclomatic: 9,
            cognitive: 16,
            adjusted_cyclomatic: None,
        },
        &reconcile_metrics,
        FunctionRole::PureLogic,
        &None,
    )
    .expect("Test should generate recommendation");

    let rec2 = generate_concise_recommendation(
        &DebtType::ComplexityHotspot {
            cyclomatic: 6,
            cognitive: 6,
            adjusted_cyclomatic: None,
        },
        &validate_metrics,
        FunctionRole::PureLogic,
        &None,
    )
    .expect("Test should generate recommendation");

    assert!(
        rec1.estimated_effort_hours.is_some(),
        "reconcile_state should have effort estimate"
    );
    assert!(
        rec2.estimated_effort_hours.is_some(),
        "validate_config should have effort estimate"
    );

    // Low complexity should have lower effort than moderate
    let effort1 = rec1.estimated_effort_hours.unwrap();
    let effort2 = rec2.estimated_effort_hours.unwrap();

    assert!(
        effort2 <= effort1,
        "Low complexity (6/6) should have <= effort than moderate (9/16), got {} vs {}",
        effort2,
        effort1
    );
}
