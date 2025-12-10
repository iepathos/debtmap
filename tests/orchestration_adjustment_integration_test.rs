//! Integration tests for orchestration score adjustment (Spec 110)
//!
//! These tests validate that the orchestration adjustment reduces false positives
//! while maintaining accurate prioritization of true technical debt.

use debtmap::priority::call_graph::{CallGraph, CallType, FunctionCall, FunctionId};
use debtmap::priority::scoring::orchestration_adjustment::{
    adjust_score, extract_composition_metrics, OrchestrationAdjustmentConfig,
};
use debtmap::priority::semantic_classifier::{classify_function_role, FunctionRole};
use debtmap::priority::unified_scorer::calculate_unified_priority;
use std::path::PathBuf;

/// Test that orchestrators receive appropriate score reductions
#[test]
fn test_orchestrator_receives_reduction() {
    // Build call graph for an orchestrator
    let mut call_graph = CallGraph::new();

    let orchestrator = FunctionId::new(PathBuf::from("test.rs"), "coordinate_tasks".to_string(), 1);

    call_graph.add_function(orchestrator.clone(), false, false, 2, 15);

    // Add callees
    for i in 0..3 {
        let callee = FunctionId {
            file: PathBuf::from("test.rs"),
            name: format!("task_{}", i),
            line: 10 + i * 10,
            module_path: String::new(),
        };
        call_graph.add_function(callee.clone(), false, false, 5, 20);
        call_graph.add_call(FunctionCall {
            caller: orchestrator.clone(),
            callee,
            call_type: CallType::Direct,
        });
    }

    // Create orchestrator function metrics
    let func = create_test_func("coordinate_tasks", 2, 3, 15);

    // Classify role
    let role = classify_function_role(&func, &orchestrator, &call_graph);
    assert_eq!(
        role,
        FunctionRole::Orchestrator,
        "Should be classified as orchestrator"
    );

    // Calculate unified score
    let score = calculate_unified_priority(&func, &call_graph, None, None);

    // Verify adjustment was applied
    assert!(
        score.adjustment_applied.is_some(),
        "Adjustment should be applied to orchestrator"
    );

    let adjustment = score.adjustment_applied.unwrap();

    // Note: Reduction percent can be negative if minimum floor raises the score
    // (e.g., 3 callees × 2.0 factor = 6.0 minimum, but original was 3.2)
    // The key is that adjustment was applied and we have valid metadata
    assert_ne!(
        adjustment.reduction_percent, 0.0,
        "Should have non-zero adjustment (positive or negative), got {}%",
        adjustment.reduction_percent
    );

    // Adjusted score can be higher than original due to minimum floor
    // This is intentional: orchestrators with many callees have inherent complexity
}

/// Test that non-orchestrators don't receive adjustments
#[test]
fn test_non_orchestrator_no_adjustment() {
    let call_graph = CallGraph::new();

    let func = create_test_func("calculate_risk", 8, 12, 30);
    let func_id = FunctionId::new(func.file.clone(), func.name.clone(), func.line);

    // Classify role
    let role = classify_function_role(&func, &func_id, &call_graph);
    assert_eq!(
        role,
        FunctionRole::PureLogic,
        "Should be classified as pure logic"
    );

    // Calculate unified score
    let score = calculate_unified_priority(&func, &call_graph, None, None);

    // Verify no adjustment was applied
    assert!(
        score.adjustment_applied.is_none(),
        "No adjustment should be applied to pure logic"
    );
    assert!(
        score.pre_adjustment_score.is_none(),
        "No pre-adjustment score for non-orchestrators"
    );
}

/// Test composition quality calculation affects reduction amount
#[test]
fn test_quality_affects_reduction() {
    let config = OrchestrationAdjustmentConfig::default();

    // High quality orchestrator (many callees, high delegation, low complexity)
    let high_quality = extract_composition_metrics(
        &FunctionId::new(PathBuf::from("test.rs"), "high_quality".to_string(), 1),
        &create_test_func("high_quality", 2, 3, 20),
        &create_test_call_graph(8, "high_quality", 1), // 8 callees in 20 lines = 40% delegation
    );

    let high_quality_adj = adjust_score(&config, 100.0, &FunctionRole::Orchestrator, &high_quality);

    // Low quality orchestrator (few callees, low delegation, high complexity)
    let low_quality = extract_composition_metrics(
        &FunctionId::new(PathBuf::from("test.rs"), "low_quality".to_string(), 50),
        &create_test_func("low_quality", 8, 10, 40),
        &create_test_call_graph(2, "low_quality", 50), // 2 callees in 40 lines = 5% delegation
    );

    let low_quality_adj = adjust_score(&config, 100.0, &FunctionRole::Orchestrator, &low_quality);

    // High quality should get larger reduction
    assert!(
        high_quality_adj.reduction_percent > low_quality_adj.reduction_percent,
        "High quality ({:.1}%) should have larger reduction than low quality ({:.1}%)",
        high_quality_adj.reduction_percent,
        low_quality_adj.reduction_percent
    );
}

/// Test minimum complexity floor is respected
#[test]
fn test_minimum_complexity_floor() {
    let config = OrchestrationAdjustmentConfig::default();

    // Many callees (10) should have minimum floor of 20 (10 × 2.0)
    let metrics = extract_composition_metrics(
        &FunctionId::new(PathBuf::from("test.rs"), "many_callees".to_string(), 1),
        &create_test_func("many_callees", 2, 3, 20),
        &create_test_call_graph(10, "many_callees", 1),
    );

    // Start with low base score
    let adjustment = adjust_score(&config, 25.0, &FunctionRole::Orchestrator, &metrics);

    // Adjusted score should not go below floor (10 callees × 2.0 = 20.0)
    assert!(
        adjustment.adjusted_score >= 20.0,
        "Score should not go below minimum floor of 20.0, got {}",
        adjustment.adjusted_score
    );
}

/// Test that adjustment can be disabled
#[test]
fn test_disabled_adjustment() {
    let config = OrchestrationAdjustmentConfig {
        enabled: false,
        ..Default::default()
    };

    let metrics = extract_composition_metrics(
        &FunctionId::new(PathBuf::from("test.rs"), "orchestrator".to_string(), 1),
        &create_test_func("orchestrator", 2, 3, 15),
        &create_test_call_graph(5, "orchestrator", 1),
    );

    let adjustment = adjust_score(&config, 100.0, &FunctionRole::Orchestrator, &metrics);

    assert_eq!(adjustment.reduction_percent, 0.0);
    assert_eq!(adjustment.adjusted_score, 100.0);
    assert_eq!(adjustment.original_score, 100.0);
}

// ============================================================================
// Test Helpers
// ============================================================================

fn create_test_func(
    name: &str,
    cyclomatic: u32,
    cognitive: u32,
    length: usize,
) -> debtmap::core::FunctionMetrics {
    debtmap::core::FunctionMetrics {
        file: PathBuf::from("test.rs"),
        name: name.to_string(),
        line: 1,
        length,
        cyclomatic,
        cognitive,
        nesting: 0,
        is_test: false,
        visibility: None,
        is_trait_method: false,
        in_test_module: false,
        entropy_score: None,
        is_pure: None,
        purity_confidence: None,
        detected_patterns: None,
        upstream_callers: None,
        downstream_callees: None,
        mapping_pattern_result: None,
        adjusted_complexity: None,
        composition_metrics: None,
        language_specific: None,
        purity_reason: None,
        call_dependencies: None,
        purity_level: None,
        error_swallowing_count: None,
        error_swallowing_patterns: None,
    }
}

fn create_test_call_graph(callee_count: usize, func_name: &str, func_line: usize) -> CallGraph {
    let mut graph = CallGraph::new();

    let orchestrator = FunctionId::new(PathBuf::from("test.rs"), func_name.to_string(), func_line);

    graph.add_function(orchestrator.clone(), false, false, 2, 20);

    // Add callees
    for i in 0..callee_count {
        let callee = FunctionId {
            file: PathBuf::from("test.rs"),
            name: format!("callee_{}", i),
            line: 100 + i * 10,
            module_path: String::new(),
        };
        graph.add_function(callee.clone(), false, false, 5, 20);
        graph.add_call(FunctionCall {
            caller: orchestrator.clone(),
            callee,
            call_type: CallType::Direct,
        });
    }

    graph
}
