#[cfg(test)]
use super::*;
use crate::core::ComplexityMetrics;
use crate::risk::priority::{ModuleType, TestTarget};
use crate::risk::roi::effort::{AdvancedEffortModel, ComplexityLevel};
use crate::risk::roi::learning::{ROIActual, ROIPrediction};
use crate::risk::roi::reduction::AdvancedRiskReductionModel;
use crate::risk::RiskAnalyzer;
use im::HashMap;
use std::path::PathBuf;

fn create_test_target(
    id: &str,
    cyclomatic: u32,
    cognitive: u32,
    coverage: f64,
    dependents: Vec<String>,
) -> TestTarget {
    TestTarget {
        id: id.to_string(),
        path: PathBuf::from(format!("src/{id}.rs")),
        function: Some(format!("{id}_fn")),
        line: 1,
        module_type: ModuleType::Core,
        current_coverage: coverage,
        current_risk: 8.0, // Increased to ensure non-zero risk
        complexity: ComplexityMetrics {
            cyclomatic_complexity: cyclomatic,
            cognitive_complexity: cognitive,
            functions: vec![],
        },
        dependencies: vec![],
        dependents,
        lines: 100,
        priority_score: 0.0,
        debt_items: 0,
    }
}

fn create_test_context() -> Context {
    let mut nodes = HashMap::new();
    nodes.insert(
        "module_a".to_string(),
        DependencyNode {
            id: "module_a".to_string(),
            path: PathBuf::from("src/module_a.rs"),
            risk: 5.0,
            complexity: ComplexityMetrics {
                cyclomatic_complexity: 10,
                cognitive_complexity: 15,
                functions: vec![],
            },
        },
    );

    let edges = im::Vector::new();

    Context {
        dependency_graph: DependencyGraph { nodes, edges },
        critical_paths: vec![],
        historical_data: None,
    }
}

#[test]
fn test_effort_model_simple_function() {
    let model = AdvancedEffortModel::new();
    let target = create_test_target("simple", 3, 5, 0.0, vec![]);

    let estimate = model.estimate(&target);

    assert!(estimate.hours > 0.0);
    assert!(estimate.hours < 2.0); // Simple function should be quick
    assert_eq!(estimate.test_cases, 6); // cyclomatic(3) + 1 + edge cases(2 for Core)
    assert_eq!(estimate.complexity, ComplexityLevel::Simple);
}

#[test]
fn test_effort_model_complex_function() {
    let model = AdvancedEffortModel::new();
    let mut target = create_test_target("complex", 20, 35, 0.0, vec![]);
    target.dependencies = vec!["io".to_string(), "net".to_string(), "db".to_string()];

    let estimate = model.estimate(&target);

    assert!(estimate.hours > 5.0); // Complex function should take longer
    assert!(estimate.test_cases > 20); // Many test cases needed
    assert_eq!(estimate.complexity, ComplexityLevel::VeryComplex);
}

#[test]
fn test_risk_reduction_zero_coverage() {
    let analyzer = RiskAnalyzer::default();
    let model = AdvancedRiskReductionModel::new(analyzer);
    let target = create_test_target("untested", 10, 15, 0.0, vec![]);

    let reduction = model.calculate(&target);

    assert!(reduction.percentage > 50.0); // Should show significant reduction potential
    assert!(reduction.coverage_increase > 60.0); // Should project high coverage increase
    assert!(reduction.confidence > 0.7); // Good confidence for zero coverage
}

#[test]
fn test_risk_reduction_partial_coverage() {
    let analyzer = RiskAnalyzer::default();
    let model = AdvancedRiskReductionModel::new(analyzer);
    let target = create_test_target("partial", 10, 15, 40.0, vec![]);

    let reduction = model.calculate(&target);

    assert!(reduction.percentage < 50.0); // Less reduction potential
    assert!(reduction.coverage_increase < 60.0); // Moderate coverage increase
    assert!(reduction.confidence >= 0.5); // Moderate confidence (min threshold)
}

#[test]
fn test_cascade_impact_with_dependents() {
    let calculator = CascadeCalculator::new();
    let target = create_test_target(
        "module_with_deps",
        10,
        15,
        0.0,
        vec!["dep1".to_string(), "dep2".to_string(), "dep3".to_string()],
    );
    let context = create_test_context();

    let impact = calculator.calculate(&target, &context);

    // With no actual dependency graph edges from this module, impact should be zero
    assert_eq!(impact.total_risk_reduction, 0.0);
    assert_eq!(impact.affected_modules.len(), 0);
}

#[test]
fn test_roi_calculator_integration() {
    let analyzer = RiskAnalyzer::default();
    let calculator = ROICalculator::new(analyzer);
    let target = create_test_target("test_module", 15, 25, 20.0, vec!["dep1".to_string()]);
    let context = create_test_context();

    let roi = calculator.calculate(&target, &context);

    assert!(roi.value > 0.0); // Should have positive ROI
    assert!(roi.value < 100.0); // Should be reasonable
    assert!(roi.confidence >= 0.5); // Should have at least minimum confidence
    assert!(!roi.breakdown.components.is_empty()); // Should have breakdown
    assert!(!roi.breakdown.formula.is_empty()); // Should have formula
    assert!(!roi.breakdown.explanation.is_empty()); // Should have explanation
}

#[test]
fn test_roi_values_vary() {
    let analyzer = RiskAnalyzer::default();
    let calculator = ROICalculator::new(analyzer);
    let context = create_test_context();

    // Create targets with different characteristics
    let simple_target = create_test_target("simple", 3, 5, 80.0, vec![]);
    let complex_untested = create_test_target("complex", 25, 40, 0.0, vec!["dep1".to_string()]);
    let moderate_partial = create_test_target("moderate", 10, 15, 40.0, vec![]);

    let simple_roi = calculator.calculate(&simple_target, &context);
    let complex_roi = calculator.calculate(&complex_untested, &context);
    let moderate_roi = calculator.calculate(&moderate_partial, &context);

    // ROI values should vary based on characteristics
    assert_ne!(simple_roi.value, complex_roi.value);
    assert_ne!(simple_roi.value, moderate_roi.value);
    assert_ne!(complex_roi.value, moderate_roi.value);

    // Complex untested should have positive ROI
    assert!(complex_roi.value > 0.0);

    // Each target should have a valid ROI value
    assert!(simple_roi.value >= 0.0);
    assert!(moderate_roi.value >= 0.0);
}

#[test]
fn test_learning_system() {
    let mut learning = ROILearningSystem::new();
    let target = create_test_target("test", 10, 15, 0.0, vec![]);

    // Record an outcome
    let prediction = ROIPrediction {
        effort: 5.0,
        risk_reduction: 30.0,
        roi: 6.0,
        target_id: "test".to_string(),
    };

    let actual = ROIActual {
        effort: 4.0,
        risk_reduction: 35.0,
        test_cases_written: 8,
        coverage_achieved: 75.0,
    };

    learning.record_outcome(prediction, actual, &target);

    // Adjustment should reflect the actual being less effort than predicted
    let adjusted = learning.adjust_estimate(5.0, &target);
    assert!(adjusted < 5.0); // Should be adjusted down

    // Confidence should be low with only one sample
    let confidence = learning.get_confidence(&target);
    assert!(confidence < 0.7);
}

#[test]
fn test_roi_breakdown_components() {
    let analyzer = RiskAnalyzer::default();
    let calculator = ROICalculator::new(analyzer);
    let target = create_test_target(
        "test",
        10,
        15,
        0.0,
        vec!["dep1".to_string(), "dep2".to_string()],
    );
    let context = create_test_context();

    let roi = calculator.calculate(&target, &context);

    // Should have multiple components
    assert!(roi.breakdown.components.len() >= 3);

    // Check component names
    let component_names: Vec<String> = roi
        .breakdown
        .components
        .iter()
        .map(|c| c.name.clone())
        .collect();

    assert!(component_names.contains(&"Direct Risk Reduction".to_string()));
    assert!(component_names.contains(&"Cascade Impact".to_string()));
    assert!(component_names.contains(&"Effort Required".to_string()));

    // Formula should mention key metrics
    assert!(roi.breakdown.formula.contains("Direct"));
    assert!(roi.breakdown.formula.contains("Cascade"));
    assert!(roi.breakdown.formula.contains("Effort"));
}

#[test]
fn test_diminishing_returns() {
    let analyzer = RiskAnalyzer::default();
    let model = AdvancedRiskReductionModel::new(analyzer);

    let low_coverage = create_test_target("low", 10, 15, 10.0, vec![]);
    let high_coverage = create_test_target("high", 10, 15, 70.0, vec![]);

    let low_reduction = model.calculate(&low_coverage);
    let high_reduction = model.calculate(&high_coverage);

    // Same complexity but different coverage should show diminishing returns
    assert!(low_reduction.percentage > high_reduction.percentage);
    assert!(low_reduction.coverage_increase > high_reduction.coverage_increase);
}
