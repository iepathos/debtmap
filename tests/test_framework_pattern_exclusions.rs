use debtmap::analysis::call_graph::FrameworkPatternDetector;
use debtmap::core::FunctionMetrics;
use debtmap::priority::call_graph::{CallGraph, FunctionId};
use debtmap::priority::scoring::classification::classify_debt_type_with_exclusions;
use debtmap::priority::DebtType;
use std::collections::HashSet;
use std::path::PathBuf;

#[test]
fn test_framework_pattern_exclusions_in_dead_code_detection() {
    // Create a test function that looks like dead code but should be excluded
    let test_func = FunctionMetrics {
        name: "test_something".to_string(),
        file: PathBuf::from("src/lib.rs"),
        line: 10,
        cyclomatic: 5,
        cognitive: 10,
        is_test: true,
        visibility: Some("pub".to_string()),
        is_trait_method: false,
        length: 20,
        nesting: 2,
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
    };

    let func_id = FunctionId::new(
        test_func.file.clone(),
        test_func.name.clone(),
        test_func.line,
    );

    // Create a call graph
    let mut call_graph = CallGraph::new();
    // Add the test function to the graph
    call_graph.add_function(
        func_id.clone(),
        false, // not entry point initially
        test_func.is_test,
        test_func.cyclomatic,
        test_func.length,
    );

    // Even without framework exclusions, test functions should be excluded
    // by the hardcoded logic (is_test flag)
    let framework_patterns = HashSet::new(); // Empty framework patterns for this test
    let debt_type = classify_debt_type_with_exclusions(
        &test_func,
        &call_graph,
        &func_id,
        &framework_patterns,
        None,
        None,
    );

    // Verify it's not dead code
    match debt_type {
        DebtType::DeadCode { .. } => {
            panic!("Test function should not be classified as dead code");
        }
        _ => {
            // Success - test functions are excluded from dead code detection
        }
    }
}

#[test]
fn test_visit_trait_pattern_exclusion() {
    // Create a function that implements the Visit trait
    let visit_func = FunctionMetrics {
        name: "visit_expr".to_string(),
        file: PathBuf::from("src/visitor.rs"),
        line: 50,
        cyclomatic: 7,
        cognitive: 15,
        is_test: false,
        visibility: Some("pub".to_string()),
        is_trait_method: false,
        length: 30,
        nesting: 3,
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
    };

    let func_id = FunctionId::new(
        visit_func.file.clone(),
        visit_func.name.clone(),
        visit_func.line,
    );

    // Create call graph
    let mut call_graph = CallGraph::new();
    call_graph.add_function(
        func_id.clone(),
        false,
        false,
        visit_func.cyclomatic,
        visit_func.length,
    );

    // Create framework pattern detector and mark this as a Visit trait function
    let mut detector = FrameworkPatternDetector::new();
    detector.add_visit_trait_function(func_id.clone());

    // Get the exclusions
    let exclusions_im = detector.get_exclusions();
    let exclusions: HashSet<FunctionId> = exclusions_im.into_iter().collect();

    // The function should not be classified as dead code when using exclusions
    let debt_type = classify_debt_type_with_exclusions(
        &visit_func,
        &call_graph,
        &func_id,
        &exclusions,
        None,
        None,
    );

    match debt_type {
        DebtType::DeadCode { .. } => {
            panic!("Visit trait implementation should not be classified as dead code");
        }
        _ => {
            // Success - Visit trait methods are excluded
        }
    }
}

#[test]
fn test_get_exclusions_returns_framework_patterns() {
    let mut detector = FrameworkPatternDetector::new();

    // Create some test function IDs
    let test_func_id = FunctionId::new(PathBuf::from("tests/test.rs"), "test_foo".to_string(), 10);

    let handler_func_id = FunctionId::new(
        PathBuf::from("src/handlers.rs"),
        "handle_request".to_string(),
        20,
    );

    let visit_func_id = FunctionId::new(
        PathBuf::from("src/visitor.rs"),
        "visit_expr".to_string(),
        30,
    );

    // Mark functions with patterns using the add_visit_trait_function method
    // For testing purposes, we'll mark all as Visit trait implementations
    // since that's the only public method available
    detector.add_visit_trait_function(test_func_id.clone());
    detector.add_visit_trait_function(handler_func_id.clone());
    detector.add_visit_trait_function(visit_func_id.clone());

    // Get exclusions
    let exclusions = detector.get_exclusions();

    // Verify all marked functions are in exclusions
    assert!(
        exclusions.contains(&test_func_id),
        "Test function should be excluded"
    );
    assert!(
        exclusions.contains(&handler_func_id),
        "Web handler should be excluded"
    );
    assert!(
        exclusions.contains(&visit_func_id),
        "Visit trait should be excluded"
    );
    assert_eq!(exclusions.len(), 3, "Should have exactly 3 exclusions");
}
