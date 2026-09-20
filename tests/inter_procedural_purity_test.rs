/// Integration tests for inter-procedural purity propagation (spec 156)
///
/// These tests verify that purity information propagates correctly through
/// the call graph to reduce false negatives in purity detection.
use debtmap::analysis::call_graph::{
    CrossModuleTracker, FrameworkPatternDetector, FunctionPointerTracker, RustCallGraph,
    TraitRegistry,
};
use debtmap::analysis::effect_evidence::{
    EffectAssessment, EffectClassification, EffectDependency, EffectProvenance, ObservedEffect,
    ObservedEffectKind,
};
use debtmap::analysis::purity_analysis::PurityAnalyzer;
use debtmap::analysis::purity_propagation::{
    PurityCallGraphAdapter, PurityPropagator, PurityReason,
};
use debtmap::core::FunctionMetrics;
use debtmap::priority::call_graph::{CallGraph, CallType, FunctionId};
use std::path::PathBuf;

fn create_test_metric(
    name: &str,
    file: &str,
    line: usize,
    is_pure: Option<bool>,
    confidence: Option<f32>,
) -> FunctionMetrics {
    FunctionMetrics {
        column: None,
        name: name.to_string(),
        file: PathBuf::from(file),
        line,
        cyclomatic: 1,
        cognitive: 0,
        nesting: 0,
        length: 5,
        is_test: false,
        visibility: None,
        is_trait_method: false,
        in_test_module: false,
        entropy_score: None,
        is_pure,
        purity_confidence: confidence,
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
        entropy_analysis: None,
    }
}

fn record_complete(graph: &mut CallGraph, id: &FunctionId) {
    graph.record_effect_assessment(id.clone(), EffectAssessment::complete());
}

fn record_impure(graph: &mut CallGraph, id: &FunctionId) {
    graph.record_effect_assessment(
        id.clone(),
        EffectAssessment::complete().with_effect(ObservedEffect {
            kind: ObservedEffectKind::ExternalWrite,
            detail: "test external write".into(),
            provenance: EffectProvenance::source(id.clone(), id.line, id.column),
        }),
    );
}

fn record_invocation(graph: &mut CallGraph, caller: &FunctionId, callee: &FunctionId) {
    graph.add_call_parts(caller.clone(), callee.clone(), CallType::Direct);
    graph.record_effect_assessment(
        caller.clone(),
        EffectAssessment::complete().with_dependency(EffectDependency {
            target: callee.clone(),
            provenance: EffectProvenance::source(caller.clone(), caller.line, caller.column),
        }),
    );
}

#[test]
fn test_pure_function_calling_pure_function() {
    // Test spec requirement: pure function calling another pure function
    // should maintain high confidence purity

    let mut call_graph = CallGraph::new();

    // Function A: pure helper (no calls)
    let helper_id = FunctionId::new(PathBuf::from("test.rs"), "pure_helper".to_string(), 10);

    // Function B: calls helper
    let caller_id = FunctionId::new(PathBuf::from("test.rs"), "caller".to_string(), 20);
    record_complete(&mut call_graph, &helper_id);
    record_invocation(&mut call_graph, &caller_id, &helper_id);

    // Create metrics
    let metrics = vec![
        create_test_metric("pure_helper", "test.rs", 10, Some(true), Some(0.95)),
        create_test_metric("caller", "test.rs", 20, Some(true), Some(0.95)),
    ];

    // Run propagation
    let rust_graph = RustCallGraph {
        base_graph: call_graph,
        trait_registry: TraitRegistry::new(),
        function_pointer_tracker: FunctionPointerTracker::new(),
        framework_patterns: FrameworkPatternDetector::new(),
        cross_module_tracker: CrossModuleTracker::new(),
    };

    let adapter = PurityCallGraphAdapter::from_rust_graph(rust_graph);
    let purity_analyzer = PurityAnalyzer::new();
    let mut propagator = PurityPropagator::new(adapter, purity_analyzer);

    propagator.propagate(&metrics).unwrap();

    // Verify: caller should remain pure with high confidence
    let caller_result = propagator.get_result(&caller_id).unwrap();
    assert!(
        caller_result.level == debtmap::analysis::purity_analysis::PurityLevel::StrictlyPure,
        "Pure function calling pure function should be pure"
    );
    assert!(
        caller_result.confidence > 0.85,
        "Confidence should remain high, got {}",
        caller_result.confidence
    );
}

#[test]
fn test_pure_recursive_function() {
    // Test spec requirement: pure recursive functions (e.g., factorial)
    // should maintain purity but with reduced confidence

    let mut call_graph = CallGraph::new();

    let factorial_id = FunctionId::new(PathBuf::from("test.rs"), "factorial".to_string(), 10);
    record_complete(&mut call_graph, &factorial_id);
    record_invocation(&mut call_graph, &factorial_id, &factorial_id);

    let metrics = vec![create_test_metric(
        "factorial",
        "test.rs",
        10,
        Some(true),
        Some(0.95),
    )];

    let rust_graph = RustCallGraph {
        base_graph: call_graph,
        trait_registry: TraitRegistry::new(),
        function_pointer_tracker: FunctionPointerTracker::new(),
        framework_patterns: FrameworkPatternDetector::new(),
        cross_module_tracker: CrossModuleTracker::new(),
    };

    let adapter = PurityCallGraphAdapter::from_rust_graph(rust_graph);
    let purity_analyzer = PurityAnalyzer::new();
    let mut propagator = PurityPropagator::new(adapter, purity_analyzer);

    propagator.propagate(&metrics).unwrap();

    let result = propagator.get_result(&factorial_id).unwrap();
    assert!(
        result.level == debtmap::analysis::purity_analysis::PurityLevel::StrictlyPure,
        "Pure recursive function should remain pure"
    );
    assert_eq!(
        result.confidence, 1.0,
        "Recursion alone is not negative evidence"
    );
}

#[test]
fn test_impure_recursive_function() {
    // Test spec requirement: recursive functions with side effects
    // should be classified as impure

    let mut call_graph = CallGraph::new();

    let recursive_id = FunctionId::new(
        PathBuf::from("test.rs"),
        "recursive_with_io".to_string(),
        10,
    );
    record_impure(&mut call_graph, &recursive_id);
    record_invocation(&mut call_graph, &recursive_id, &recursive_id);

    let metrics = vec![create_test_metric(
        "recursive_with_io",
        "test.rs",
        10,
        Some(false), // Has side effects
        Some(0.95),
    )];

    let rust_graph = RustCallGraph {
        base_graph: call_graph,
        trait_registry: TraitRegistry::new(),
        function_pointer_tracker: FunctionPointerTracker::new(),
        framework_patterns: FrameworkPatternDetector::new(),
        cross_module_tracker: CrossModuleTracker::new(),
    };

    let adapter = PurityCallGraphAdapter::from_rust_graph(rust_graph);
    let purity_analyzer = PurityAnalyzer::new();
    let mut propagator = PurityPropagator::new(adapter, purity_analyzer);

    propagator.propagate(&metrics).unwrap();

    let result = propagator.get_result(&recursive_id).unwrap();
    assert!(
        result.level == debtmap::analysis::purity_analysis::PurityLevel::Impure,
        "Recursive function with side effects should be impure"
    );
}

#[test]
fn test_confidence_is_deterministic_across_depth() {
    // Classification is evidence-driven; depth alone does not relabel evidence.

    let mut call_graph = CallGraph::new();

    // Create chain: A -> B -> C (all pure)
    let a_id = FunctionId::new(PathBuf::from("test.rs"), "func_a".to_string(), 10);
    let b_id = FunctionId::new(PathBuf::from("test.rs"), "func_b".to_string(), 20);
    let c_id = FunctionId::new(PathBuf::from("test.rs"), "func_c".to_string(), 30);

    record_complete(&mut call_graph, &c_id);
    record_invocation(&mut call_graph, &b_id, &c_id);
    record_invocation(&mut call_graph, &a_id, &b_id);

    let metrics = vec![
        create_test_metric("func_a", "test.rs", 10, Some(true), Some(0.95)),
        create_test_metric("func_b", "test.rs", 20, Some(true), Some(0.95)),
        create_test_metric("func_c", "test.rs", 30, Some(true), Some(0.95)),
    ];

    let rust_graph = RustCallGraph {
        base_graph: call_graph,
        trait_registry: TraitRegistry::new(),
        function_pointer_tracker: FunctionPointerTracker::new(),
        framework_patterns: FrameworkPatternDetector::new(),
        cross_module_tracker: CrossModuleTracker::new(),
    };

    let adapter = PurityCallGraphAdapter::from_rust_graph(rust_graph);
    let purity_analyzer = PurityAnalyzer::new();
    let mut propagator = PurityPropagator::new(adapter, purity_analyzer);

    propagator.propagate(&metrics).unwrap();

    let a_result = propagator.get_result(&a_id).unwrap();
    let c_result = propagator.get_result(&c_id).unwrap();

    assert_eq!(a_result.confidence, c_result.confidence);
}

#[test]
fn test_cross_file_purity_propagation() {
    // Test spec requirement: purity should propagate across file boundaries

    let mut call_graph = CallGraph::new();

    // Helper in file1.rs
    let helper_id = FunctionId::new(PathBuf::from("file1.rs"), "helper".to_string(), 10);

    // Caller in file2.rs
    let caller_id = FunctionId::new(PathBuf::from("file2.rs"), "caller".to_string(), 20);
    record_complete(&mut call_graph, &helper_id);
    record_invocation(&mut call_graph, &caller_id, &helper_id);

    let metrics = vec![
        create_test_metric("helper", "file1.rs", 10, Some(true), Some(0.95)),
        create_test_metric("caller", "file2.rs", 20, Some(true), Some(0.95)),
    ];

    let rust_graph = RustCallGraph {
        base_graph: call_graph,
        trait_registry: TraitRegistry::new(),
        function_pointer_tracker: FunctionPointerTracker::new(),
        framework_patterns: FrameworkPatternDetector::new(),
        cross_module_tracker: CrossModuleTracker::new(),
    };

    let adapter = PurityCallGraphAdapter::from_rust_graph(rust_graph);
    let purity_analyzer = PurityAnalyzer::new();
    let mut propagator = PurityPropagator::new(adapter, purity_analyzer);

    propagator.propagate(&metrics).unwrap();

    let caller_result = propagator.get_result(&caller_id).unwrap();
    assert!(
        caller_result.level == debtmap::analysis::purity_analysis::PurityLevel::StrictlyPure,
        "Purity should propagate across file boundaries"
    );
}

#[test]
fn test_impure_caller_propagates_impurity() {
    // Test that calling an impure function makes the caller impure

    let mut call_graph = CallGraph::new();

    let impure_id = FunctionId::new(PathBuf::from("test.rs"), "impure_func".to_string(), 10);
    let caller_id = FunctionId::new(PathBuf::from("test.rs"), "caller".to_string(), 20);
    record_impure(&mut call_graph, &impure_id);
    record_invocation(&mut call_graph, &caller_id, &impure_id);

    let metrics = vec![
        create_test_metric("impure_func", "test.rs", 10, Some(false), Some(0.95)),
        create_test_metric("caller", "test.rs", 20, Some(true), Some(0.95)), // Initially thinks it's pure
    ];

    let rust_graph = RustCallGraph {
        base_graph: call_graph,
        trait_registry: TraitRegistry::new(),
        function_pointer_tracker: FunctionPointerTracker::new(),
        framework_patterns: FrameworkPatternDetector::new(),
        cross_module_tracker: CrossModuleTracker::new(),
    };

    let adapter = PurityCallGraphAdapter::from_rust_graph(rust_graph);
    let purity_analyzer = PurityAnalyzer::new();
    let mut propagator = PurityPropagator::new(adapter, purity_analyzer);

    propagator.propagate(&metrics).unwrap();

    let caller_result = propagator.get_result(&caller_id).unwrap();
    assert!(
        caller_result.level == debtmap::analysis::purity_analysis::PurityLevel::Impure,
        "Calling impure function should make caller impure"
    );
}

#[test]
fn external_callee_does_not_abort_project_propagation() {
    let mut call_graph = CallGraph::new();
    let caller_id = FunctionId::new(PathBuf::from("test.rs"), "caller".to_string(), 20);
    let external_id = FunctionId::new(
        PathBuf::from("external.rs"),
        "external_crate::send".to_string(),
        1,
    );
    record_invocation(&mut call_graph, &caller_id, &external_id);

    let rust_graph = RustCallGraph {
        base_graph: call_graph,
        trait_registry: TraitRegistry::new(),
        function_pointer_tracker: FunctionPointerTracker::new(),
        framework_patterns: FrameworkPatternDetector::new(),
        cross_module_tracker: CrossModuleTracker::new(),
    };
    let adapter = PurityCallGraphAdapter::from_rust_graph(rust_graph);
    let mut propagator = PurityPropagator::new(adapter, PurityAnalyzer::new());
    let metrics = vec![create_test_metric(
        "caller",
        "test.rs",
        20,
        Some(true),
        Some(0.95),
    )];

    propagator.propagate(&metrics).unwrap();

    let result = propagator.get_result(&caller_id).unwrap();
    assert_eq!(
        result.assessment.classification(),
        EffectClassification::Unknown
    );
    assert!(matches!(result.reason, PurityReason::Evidence { .. }));
}
