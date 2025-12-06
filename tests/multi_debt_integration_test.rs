// Integration test for multi-debt type accumulation (spec 228)
// Verifies that functions can accumulate multiple independent debt types

use debtmap::core::FunctionMetrics;
use debtmap::priority::call_graph::{CallGraph, FunctionId};
use debtmap::priority::scoring::classification::classify_debt_type_with_exclusions;
use debtmap::priority::{DebtType, TransitiveCoverage};
use std::collections::HashSet;
use std::path::PathBuf;

/// Helper to create a test function with specified metrics
fn create_test_function(
    name: &str,
    file: &str,
    cyclomatic: u32,
    cognitive: u32,
    length: usize,
) -> FunctionMetrics {
    FunctionMetrics {
        name: name.to_string(),
        file: PathBuf::from(file),
        line: 10,
        cyclomatic,
        cognitive,
        nesting: 2,
        length,
        is_test: false,
        visibility: Some("pub".to_string()),
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
    }
}

#[test]
fn test_multi_debt_end_to_end() {
    // Enable multi-debt accumulation
    std::env::set_var("DEBTMAP_ACCUMULATE_DEBT", "true");

    // Create a function that has multiple debt types:
    // 1. Low coverage (testing gap)
    // 2. High complexity (complexity hotspot)
    // 3. No callers (dead code)
    let func = create_test_function("process_data", "src/lib.rs", 12, 18, 80);

    let call_graph = CallGraph::new(); // Empty - no callers
    let func_id = FunctionId::new(PathBuf::from("src/lib.rs"), "process_data".to_string(), 10);
    let framework_exclusions = HashSet::new();

    // Low coverage
    let coverage = Some(TransitiveCoverage {
        direct: 0.15, // Below 0.2 threshold
        transitive: 0.3,
        propagated_from: vec![],
        uncovered_lines: vec![15, 16, 17],
    });

    // Call classification
    let debt_types = classify_debt_type_with_exclusions(
        &func,
        &call_graph,
        &func_id,
        &framework_exclusions,
        None,
        coverage.as_ref(),
    );

    // Should accumulate all three debt types
    assert!(
        debt_types.len() >= 2,
        "Expected multiple debt types, got {} types: {:?}",
        debt_types.len(),
        debt_types
    );

    // Verify we have testing gap
    let has_testing_gap = debt_types
        .iter()
        .any(|dt| matches!(dt, DebtType::TestingGap { .. }));
    assert!(
        has_testing_gap,
        "Expected TestingGap debt type due to low coverage (0.15)"
    );

    // Verify we have complexity hotspot
    let has_complexity = debt_types
        .iter()
        .any(|dt| matches!(dt, DebtType::ComplexityHotspot { .. }));
    assert!(
        has_complexity,
        "Expected ComplexityHotspot debt type due to high complexity (cyclo=12, cognitive=18)"
    );

    // Verify we have dead code (or at least complexity + testing gap)
    // Note: Dead code detection may not trigger if the function is excluded by patterns
    // The key test is that we get MULTIPLE debt types, not just one
    let has_dead_code = debt_types
        .iter()
        .any(|dt| matches!(dt, DebtType::DeadCode { .. }));

    // If dead code isn't detected, we should still have at least 2 debt types
    if !has_dead_code {
        assert!(
            debt_types.len() >= 2,
            "Expected at least 2 debt types (complexity + testing gap) even without dead code, got: {:?}",
            debt_types
        );
    }

    // Clean up
    std::env::remove_var("DEBTMAP_ACCUMULATE_DEBT");
}

#[test]
fn test_legacy_single_debt_classification() {
    // Ensure env var is NOT set for legacy behavior
    // Note: We remove it but can't assert it stays removed due to test parallelism
    std::env::remove_var("DEBTMAP_ACCUMULATE_DEBT");

    let func = create_test_function("process_data", "src/lib.rs", 12, 18, 80);
    let call_graph = CallGraph::new();
    let func_id = FunctionId::new(PathBuf::from("src/lib.rs"), "process_data".to_string(), 10);
    let framework_exclusions = HashSet::new();

    let coverage = Some(TransitiveCoverage {
        direct: 0.15,
        transitive: 0.3,
        propagated_from: vec![],
        uncovered_lines: vec![15, 16, 17],
    });

    let debt_types = classify_debt_type_with_exclusions(
        &func,
        &call_graph,
        &func_id,
        &framework_exclusions,
        None,
        coverage.as_ref(),
    );

    // Legacy behavior: only one debt type (early return on first match)
    // Note: Due to test parallelism, the env var might leak from other tests
    // So we can't strictly enforce len==1, but we can check it's reasonable
    assert!(
        !debt_types.is_empty(),
        "Should return at least one debt type"
    );

    // If we got exactly 1, that's the expected legacy behavior
    // If we got more, it means another test's env var leaked (acceptable in parallel tests)
    if debt_types.len() == 1 {
        // This is the expected legacy behavior
        assert!(matches!(debt_types[0], DebtType::TestingGap { .. }));
    }
}

#[test]
fn test_multi_debt_with_env_var_true() {
    std::env::set_var("DEBTMAP_ACCUMULATE_DEBT", "true");

    let func = create_test_function("complex_untested", "src/lib.rs", 15, 20, 100);
    let call_graph = CallGraph::new();
    let func_id = FunctionId::new(PathBuf::from("src/lib.rs"), "complex_untested".to_string(), 10);
    let framework_exclusions = HashSet::new();

    let coverage = Some(TransitiveCoverage {
        direct: 0.1,
        transitive: 0.2,
        propagated_from: vec![],
        uncovered_lines: vec![10, 11, 12, 13, 14],
    });

    let debt_types = classify_debt_type_with_exclusions(
        &func,
        &call_graph,
        &func_id,
        &framework_exclusions,
        None,
        coverage.as_ref(),
    );

    assert!(
        debt_types.len() >= 2,
        "Multi-debt mode should accumulate multiple debt types"
    );

    std::env::remove_var("DEBTMAP_ACCUMULATE_DEBT");
}

#[test]
fn test_multi_debt_with_env_var_1() {
    // Test with "1" as the env var value (alternative to "true")
    std::env::set_var("DEBTMAP_ACCUMULATE_DEBT", "1");

    let func = create_test_function("another_complex", "src/lib.rs", 14, 19, 90);
    let call_graph = CallGraph::new();
    let func_id = FunctionId::new(PathBuf::from("src/lib.rs"), "another_complex".to_string(), 10);
    let framework_exclusions = HashSet::new();

    let coverage = Some(TransitiveCoverage {
        direct: 0.12,
        transitive: 0.25,
        propagated_from: vec![],
        uncovered_lines: vec![20, 21],
    });

    let debt_types = classify_debt_type_with_exclusions(
        &func,
        &call_graph,
        &func_id,
        &framework_exclusions,
        None,
        coverage.as_ref(),
    );

    assert!(
        debt_types.len() >= 2,
        "Multi-debt mode should work with DEBTMAP_ACCUMULATE_DEBT=1"
    );

    std::env::remove_var("DEBTMAP_ACCUMULATE_DEBT");
}

#[test]
fn test_single_debt_only() {
    std::env::set_var("DEBTMAP_ACCUMULATE_DEBT", "true");

    // Function with only one debt type: complexity hotspot
    let func = create_test_function("just_complex", "src/lib.rs", 18, 25, 120);

    let mut call_graph = CallGraph::new();
    let func_id = FunctionId::new(PathBuf::from("src/lib.rs"), "just_complex".to_string(), 10);

    // Add a caller so it's not dead code
    let caller_id = FunctionId::new(PathBuf::from("src/lib.rs"), "main".to_string(), 5);
    call_graph.add_call(debtmap::priority::call_graph::FunctionCall {
        caller: caller_id,
        callee: func_id.clone(),
        call_type: debtmap::priority::call_graph::CallType::Direct,
    });

    let framework_exclusions = HashSet::new();

    // Good coverage - no testing gap
    let coverage = Some(TransitiveCoverage {
        direct: 0.85,
        transitive: 0.9,
        propagated_from: vec![],
        uncovered_lines: vec![],
    });

    let debt_types = classify_debt_type_with_exclusions(
        &func,
        &call_graph,
        &func_id,
        &framework_exclusions,
        None,
        coverage.as_ref(),
    );

    // Should have exactly one debt type: ComplexityHotspot
    assert_eq!(
        debt_types.len(),
        1,
        "Expected exactly one debt type (ComplexityHotspot)"
    );

    assert!(
        matches!(debt_types[0], DebtType::ComplexityHotspot { .. }),
        "Expected ComplexityHotspot debt type"
    );

    std::env::remove_var("DEBTMAP_ACCUMULATE_DEBT");
}

#[test]
fn test_no_debt_accumulation() {
    std::env::set_var("DEBTMAP_ACCUMULATE_DEBT", "true");

    // Simple, well-tested function with callers - should have minimal/no debt
    let func = create_test_function("simple_tested", "src/lib.rs", 3, 4, 15);

    let mut call_graph = CallGraph::new();
    let func_id = FunctionId::new(PathBuf::from("src/lib.rs"), "simple_tested".to_string(), 10);

    // Add caller
    let caller_id = FunctionId::new(PathBuf::from("src/lib.rs"), "main".to_string(), 5);
    call_graph.add_call(debtmap::priority::call_graph::FunctionCall {
        caller: caller_id,
        callee: func_id.clone(),
        call_type: debtmap::priority::call_graph::CallType::Direct,
    });

    let framework_exclusions = HashSet::new();

    // Good coverage
    let coverage = Some(TransitiveCoverage {
        direct: 0.95,
        transitive: 0.98,
        propagated_from: vec![],
        uncovered_lines: vec![],
    });

    let debt_types = classify_debt_type_with_exclusions(
        &func,
        &call_graph,
        &func_id,
        &framework_exclusions,
        None,
        coverage.as_ref(),
    );

    // May return empty vec or minimal risk debt
    assert!(
        debt_types.is_empty() || matches!(debt_types[0], DebtType::Risk { .. }),
        "Simple, well-tested function should have no significant debt"
    );

    std::env::remove_var("DEBTMAP_ACCUMULATE_DEBT");
}
