use super::*;
use std::path::PathBuf;

#[test]
fn test_zero_coverage_prioritization() {
    let target = TestTarget {
        id: "test1".to_string(),
        path: PathBuf::from("src/main.rs"),
        function: Some("main".to_string()),
        module_type: ModuleType::EntryPoint,
        current_coverage: 0.0,
        current_risk: 8.0,
        complexity: ComplexityMetrics {
            functions: vec![],
            cyclomatic_complexity: 5,
            cognitive_complexity: 10,
        },
        dependencies: vec![],
        dependents: vec!["module1".to_string(), "module2".to_string()],
        lines: 100,
        priority_score: 0.0,
        debt_items: 2,
    };

    let stage = ZeroCoverageStage::new();
    let mut targets = vec![target.clone()];
    targets = stage.process(targets);

    assert!(targets[0].priority_score > 1000.0);
}

#[test]
fn test_criticality_scorer() {
    let scorer = CriticalityScorer::new();

    let main_target = TestTarget {
        id: "main".to_string(),
        path: PathBuf::from("src/main.rs"),
        function: None,
        module_type: ModuleType::EntryPoint,
        current_coverage: 0.0,
        current_risk: 5.0,
        complexity: ComplexityMetrics::default(),
        dependencies: vec![],
        dependents: vec!["a".to_string(), "b".to_string()],
        lines: 50,
        priority_score: 0.0,
        debt_items: 0,
    };

    let util_target = TestTarget {
        id: "util".to_string(),
        path: PathBuf::from("src/utils/helper.rs"),
        function: None,
        module_type: ModuleType::Utility,
        current_coverage: 50.0,
        current_risk: 2.0,
        complexity: ComplexityMetrics::default(),
        dependencies: vec![],
        dependents: vec![],
        lines: 50,
        priority_score: 0.0,
        debt_items: 0,
    };

    let main_score = scorer.score(&main_target);
    let util_score = scorer.score(&util_target);

    assert!(main_score > util_score);
}

#[test]
fn test_effort_estimation() {
    let estimator = EffortEstimator::new();

    let complex_target = TestTarget {
        id: "complex".to_string(),
        path: PathBuf::from("src/complex.rs"),
        function: Some("complex_fn".to_string()),
        module_type: ModuleType::Core,
        current_coverage: 0.0,
        current_risk: 8.0,
        complexity: ComplexityMetrics {
            functions: vec![],
            cyclomatic_complexity: 15,
            cognitive_complexity: 30,
        },
        dependencies: vec!["dep1".to_string(), "dep2".to_string()],
        dependents: vec![],
        lines: 200,
        priority_score: 0.0,
        debt_items: 5,
    };

    let simple_target = TestTarget {
        id: "simple".to_string(),
        path: PathBuf::from("src/simple.rs"),
        function: Some("simple_fn".to_string()),
        module_type: ModuleType::Utility,
        current_coverage: 0.0,
        current_risk: 2.0,
        complexity: ComplexityMetrics {
            functions: vec![],
            cyclomatic_complexity: 2,
            cognitive_complexity: 3,
        },
        dependencies: vec![],
        dependents: vec![],
        lines: 20,
        priority_score: 0.0,
        debt_items: 0,
    };

    let complex_effort = estimator.estimate(&complex_target);
    let simple_effort = estimator.estimate(&simple_target);

    assert!(complex_effort > simple_effort);
    assert!(complex_effort > 20.0);
    assert!(simple_effort < 10.0);
}

#[test]
fn test_module_type_detection() {
    assert_eq!(
        determine_module_type(&PathBuf::from("src/main.rs")),
        ModuleType::EntryPoint
    );
    assert_eq!(
        determine_module_type(&PathBuf::from("src/lib.rs")),
        ModuleType::EntryPoint
    );
    assert_eq!(
        determine_module_type(&PathBuf::from("src/core/engine.rs")),
        ModuleType::Core
    );
    assert_eq!(
        determine_module_type(&PathBuf::from("src/api/handler.rs")),
        ModuleType::Api
    );
    assert_eq!(
        determine_module_type(&PathBuf::from("src/utils/helper.rs")),
        ModuleType::Utility
    );
    assert_eq!(
        determine_module_type(&PathBuf::from("tests/integration.rs")),
        ModuleType::Test
    );
}