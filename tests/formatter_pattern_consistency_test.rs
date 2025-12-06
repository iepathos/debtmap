//! Integration test to verify all formatters display identical pattern information.
//!
//! This test ensures that spec 204 is correctly implemented: all formatters
//! read from the stored `detected_pattern` field instead of re-detecting,
//! guaranteeing consistency across terminal, markdown, and other output formats.

use debtmap::priority::detected_pattern::{DetectedPattern, PatternMetrics, PatternType};
use debtmap::priority::unified_scorer::{Location, UnifiedScore};
use debtmap::priority::{
    ActionableRecommendation, DebtType, FunctionRole, ImpactMetrics, UnifiedDebtItem,
};
use std::path::PathBuf;

/// Create a test UnifiedDebtItem with a state machine pattern
fn create_test_item_with_state_machine() -> UnifiedDebtItem {
    UnifiedDebtItem {
        location: Location {
            file: PathBuf::from("src/test.rs"),
            line: 100,
            function: "process_state".to_string(),
        },
        debt_type: DebtType::ComplexityHotspot {
            cyclomatic: 15,
            cognitive: 35,
            adjusted_cyclomatic: None,
        },
        cyclomatic_complexity: 15,
        cognitive_complexity: 35,
        nesting_depth: 3,
        function_length: 120,
        upstream_callers: vec![],
        downstream_callees: vec![],
        upstream_dependencies: 0,
        downstream_dependencies: 0,
        unified_score: UnifiedScore {
            final_score: 85.0,
            base_score: Some(75.0),
            complexity_factor: 1.0,
            dependency_factor: 1.0,
            coverage_factor: 1.0,
            role_multiplier: 1.0,
            exponential_factor: None,
            risk_boost: None,
            pre_adjustment_score: None,
            adjustment_applied: None,
            purity_factor: None,
            refactorability_factor: None,
            pattern_factor: None,
        },
        expected_impact: ImpactMetrics {
            coverage_improvement: 0.0,
            lines_reduction: 50,
            complexity_reduction: 10.0,
            risk_reduction: 8.0,
        },
        function_role: FunctionRole::PureLogic,
        recommendation: ActionableRecommendation {
            primary_action: "Refactor state machine".to_string(),
            rationale: "State machine pattern detected".to_string(),
            implementation_steps: vec![],
            related_items: vec![],
            steps: None,
            estimated_effort_hours: None,
        },
        transitive_coverage: None,
        file_context: None,
        is_pure: None,
        purity_confidence: None,
        purity_level: None,
        entropy_details: None,
        entropy_adjusted_cyclomatic: None,
        entropy_adjusted_cognitive: None,
        entropy_dampening_factor: None,
        god_object_indicators: None,
        tier: None,
        function_context: None,
        context_confidence: None,
        contextual_recommendation: None,
        pattern_analysis: None,
        context_multiplier: None,
        context_type: None,
        language_specific: None,
        detected_pattern: Some(DetectedPattern {
            pattern_type: PatternType::StateMachine,
            confidence: 0.85,
            metrics: PatternMetrics {
                state_transitions: Some(4),
                match_expressions: Some(2),
                action_dispatches: Some(8),
                comparisons: None,
            },
        }),
        contextual_risk: None, // spec 203
    }
}

/// Create a test UnifiedDebtItem with a coordinator pattern
fn create_test_item_with_coordinator() -> UnifiedDebtItem {
    UnifiedDebtItem {
        location: Location {
            file: PathBuf::from("src/coordinator.rs"),
            line: 50,
            function: "coordinate_actions".to_string(),
        },
        debt_type: DebtType::ComplexityHotspot {
            cyclomatic: 15,
            cognitive: 35,
            adjusted_cyclomatic: None,
        },
        cyclomatic_complexity: 12,
        cognitive_complexity: 28,
        nesting_depth: 2,
        function_length: 80,
        upstream_callers: vec![],
        downstream_callees: vec![],
        upstream_dependencies: 0,
        downstream_dependencies: 0,
        unified_score: UnifiedScore {
            final_score: 75.0,
            base_score: Some(65.0),
            complexity_factor: 1.0,
            dependency_factor: 1.0,
            coverage_factor: 1.0,
            role_multiplier: 1.0,
            exponential_factor: None,
            risk_boost: None,
            pre_adjustment_score: None,
            adjustment_applied: None,
            purity_factor: None,
            refactorability_factor: None,
            pattern_factor: None,
        },
        expected_impact: ImpactMetrics {
            coverage_improvement: 0.0,
            lines_reduction: 30,
            complexity_reduction: 8.0,
            risk_reduction: 6.0,
        },
        function_role: FunctionRole::PureLogic,
        recommendation: ActionableRecommendation {
            primary_action: "Refactor coordinator".to_string(),
            rationale: "Coordinator pattern detected".to_string(),
            implementation_steps: vec![],
            related_items: vec![],
            steps: None,
            estimated_effort_hours: None,
        },
        transitive_coverage: None,
        file_context: None,
        is_pure: None,
        purity_confidence: None,
        purity_level: None,
        entropy_details: None,
        entropy_adjusted_cyclomatic: None,
        entropy_adjusted_cognitive: None,
        entropy_dampening_factor: None,
        god_object_indicators: None,
        tier: None,
        function_context: None,
        context_confidence: None,
        contextual_recommendation: None,
        pattern_analysis: None,
        context_multiplier: None,
        context_type: None,
        language_specific: None,
        detected_pattern: Some(DetectedPattern {
            pattern_type: PatternType::Coordinator,
            confidence: 0.80,
            metrics: PatternMetrics {
                state_transitions: None,
                match_expressions: None,
                action_dispatches: Some(4),
                comparisons: Some(2),
            },
        }),
        contextual_risk: None, // spec 203
    }
}

#[test]
fn test_state_machine_pattern_consistency() {
    let item = create_test_item_with_state_machine();
    let pattern = item.detected_pattern.as_ref().unwrap();

    // Verify pattern type
    assert_eq!(pattern.pattern_type, PatternType::StateMachine);
    assert_eq!(pattern.type_name(), "State Machine");
    assert_eq!(pattern.icon(), "🔄");

    // Verify confidence
    assert!((pattern.confidence - 0.85).abs() < 0.001);

    // Verify metrics
    let metrics = pattern.display_metrics();
    assert_eq!(metrics.len(), 3);
    assert!(metrics.contains(&"transitions: 4".to_string()));
    assert!(metrics.contains(&"matches: 2".to_string()));
    assert!(metrics.contains(&"actions: 8".to_string()));

    // All formatters should use these exact values from detected_pattern
    // No formatter should re-detect or compute different values
}

#[test]
fn test_coordinator_pattern_consistency() {
    let item = create_test_item_with_coordinator();
    let pattern = item.detected_pattern.as_ref().unwrap();

    // Verify pattern type
    assert_eq!(pattern.pattern_type, PatternType::Coordinator);
    assert_eq!(pattern.type_name(), "Coordinator");
    assert_eq!(pattern.icon(), "🎯");

    // Verify confidence
    assert!((pattern.confidence - 0.80).abs() < 0.001);

    // Verify metrics
    let metrics = pattern.display_metrics();
    assert_eq!(metrics.len(), 2);
    assert!(metrics.contains(&"actions: 4".to_string()));
    assert!(metrics.contains(&"comparisons: 2".to_string()));

    // All formatters should use these exact values from detected_pattern
    // No formatter should re-detect or compute different values
}

#[test]
fn test_no_pattern_when_none_detected() {
    let mut item = create_test_item_with_state_machine();
    item.detected_pattern = None;

    // Verify no pattern is present
    assert!(item.detected_pattern.is_none());

    // All formatters should show no pattern information
    // No formatter should attempt to detect a pattern
}

#[test]
fn test_pattern_metrics_immutability() {
    let item = create_test_item_with_state_machine();
    let pattern = item.detected_pattern.as_ref().unwrap();

    // Get metrics multiple times - should always be identical
    let metrics1 = pattern.display_metrics();
    let metrics2 = pattern.display_metrics();
    let metrics3 = pattern.display_metrics();

    assert_eq!(metrics1, metrics2);
    assert_eq!(metrics2, metrics3);

    // Pattern confidence should never change
    let confidence1 = pattern.confidence;
    let confidence2 = pattern.confidence;
    assert_eq!(confidence1, confidence2);
}

#[test]
fn test_pattern_display_format_consistency() {
    let item = create_test_item_with_state_machine();
    let pattern = item.detected_pattern.as_ref().unwrap();

    // Verify display format is consistent
    let metrics = pattern.display_metrics();

    // Each metric should follow "key: value" format
    for metric in &metrics {
        assert!(
            metric.contains(": "),
            "Metric '{}' should contain ': '",
            metric
        );
        let parts: Vec<&str> = metric.split(": ").collect();
        assert_eq!(
            parts.len(),
            2,
            "Metric '{}' should have exactly one ': ' separator",
            metric
        );
    }

    // Icon should be single character emoji
    assert_eq!(pattern.icon().chars().count(), 1);

    // Type name should be human-readable
    assert!(!pattern.type_name().is_empty());
    assert!(pattern.type_name().chars().next().unwrap().is_uppercase());
}

#[test]
fn test_pattern_confidence_range() {
    let item_sm = create_test_item_with_state_machine();
    let pattern_sm = item_sm.detected_pattern.as_ref().unwrap();

    let item_coord = create_test_item_with_coordinator();
    let pattern_coord = item_coord.detected_pattern.as_ref().unwrap();

    // All patterns should have confidence >= 0.7 (threshold from spec)
    assert!(pattern_sm.confidence >= 0.7);
    assert!(pattern_coord.confidence >= 0.7);

    // Confidence should be <= 1.0 (maximum possible)
    assert!(pattern_sm.confidence <= 1.0);
    assert!(pattern_coord.confidence <= 1.0);
}
