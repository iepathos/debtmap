pub mod module_detection;
pub mod pipeline;
pub mod recommendations;
pub mod scoring;
pub mod stages;

pub use module_detection::{determine_module_type, infer_module_relationships, ModuleType};
pub use pipeline::{PrioritizationPipeline, PrioritizationStage};
pub use recommendations::{
    generate_enhanced_rationale_v2, ComplexityLevel, ImpactAnalysis, TestApproach,
    TestEffortDetails, TestRecommendation,
};
pub use scoring::{CriticalityScorer, EffortEstimator};
pub use stages::{
    ComplexityRiskStage, CriticalPathStage, DependencyImpactStage, EffortOptimizationStage,
    ZeroCoverageStage,
};

use super::{Difficulty, FunctionRisk, RiskAnalyzer, TestEffort, TestingRecommendation};
use crate::core::ComplexityMetrics;
use im::Vector;
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct TestTarget {
    pub id: String,
    pub path: PathBuf,
    pub function: Option<String>,
    pub line: usize,
    pub module_type: ModuleType,
    pub current_coverage: f64,
    pub current_risk: f64,
    pub complexity: ComplexityMetrics,
    pub dependencies: Vec<String>,
    pub dependents: Vec<String>,
    pub lines: usize,
    pub priority_score: f64,
    pub debt_items: usize,
}

pub use super::roi::{
    ROIBreakdown, ROICalculator as AdvancedROICalculator, ROIComponent, ROIConfig,
    ROI as AdvancedROI,
};

pub struct ROICalculator {
    advanced_calculator: AdvancedROICalculator,
    #[allow(dead_code)]
    risk_analyzer: RiskAnalyzer,
}

impl ROICalculator {
    pub fn new(risk_analyzer: RiskAnalyzer) -> Self {
        let advanced_calculator = AdvancedROICalculator::new(risk_analyzer.clone());
        Self {
            advanced_calculator,
            risk_analyzer,
        }
    }

    pub fn calculate(&self, target: &TestTarget, _target_coverage: f64) -> ROI {
        let context = self.create_context(target);
        let advanced_roi = self.advanced_calculator.calculate(target, &context);

        ROI {
            value: advanced_roi.value,
            risk_reduction: advanced_roi.direct_impact.percentage,
            effort: advanced_roi.effort.hours,
            cascade_impact: advanced_roi.cascade_impact.total_risk_reduction,
            explanation: advanced_roi.breakdown.explanation,
        }
    }

    fn create_context(&self, target: &TestTarget) -> super::roi::Context {
        let mut nodes = im::HashMap::new();
        let mut edges = im::Vector::new();

        nodes.insert(
            target.id.clone(),
            super::roi::DependencyNode {
                id: target.id.clone(),
                path: target.path.clone(),
                risk: target.current_risk,
                complexity: target.complexity.clone(),
            },
        );

        for dependent in &target.dependents {
            edges.push_back(super::roi::DependencyEdge {
                from: target.id.clone(),
                to: dependent.clone(),
                weight: 0.8,
            });
        }

        super::roi::Context {
            dependency_graph: super::roi::DependencyGraph { nodes, edges },
            critical_paths: vec![],
            historical_data: None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ROI {
    pub value: f64,
    pub risk_reduction: f64,
    pub effort: f64,
    pub cascade_impact: f64,
    pub explanation: String,
}

fn is_untested_function(f: &FunctionRisk) -> bool {
    !f.is_test_function && f.coverage_percentage.unwrap_or(100.0) == 0.0
}

fn is_closure(function_name: &str) -> bool {
    function_name.starts_with("<closure@")
}

fn has_untested_parent(
    closure_file: &std::path::Path,
    closure_line: usize,
    untested_functions: &[&FunctionRisk],
) -> bool {
    untested_functions.iter().any(|parent| {
        parent.file == closure_file
            && !is_closure(&parent.function_name)
            && is_likely_parent(parent.line_range.0, closure_line)
    })
}

fn is_likely_parent(parent_start: usize, closure_line: usize) -> bool {
    // More lenient: closure within 20 lines of function start
    parent_start <= closure_line && closure_line <= parent_start + 20
}

fn should_include_function(f: &FunctionRisk, untested_functions: &[&FunctionRisk]) -> bool {
    if f.is_test_function {
        return false;
    }

    if is_closure(&f.function_name) {
        return !has_untested_parent(&f.file, f.line_range.0, untested_functions);
    }

    true
}

pub fn prioritize_by_roi(
    functions: &Vector<FunctionRisk>,
    analyzer: &RiskAnalyzer,
) -> Vector<TestingRecommendation> {
    let untested_functions: Vec<&FunctionRisk> = functions
        .iter()
        .filter(|f| is_untested_function(f))
        .collect();

    let targets: Vec<TestTarget> = functions
        .iter()
        .filter(|f| should_include_function(f, &untested_functions))
        .map(function_risk_to_target)
        .collect();
    let prioritized = process_and_sort_targets(targets);
    let context = build_roi_context(&prioritized);
    let roi_calc = crate::risk::roi::ROICalculator::new(analyzer.clone());

    let mut recommendations = create_recommendations(&prioritized, &roi_calc, &context);
    sort_recommendations_by_roi(&mut recommendations);

    recommendations
}

fn process_and_sort_targets(targets: Vec<TestTarget>) -> Vec<TestTarget> {
    let pipeline = PrioritizationPipeline::new();
    let mut prioritized = pipeline.process(targets);
    prioritized.sort_by(|a, b| b.priority_score.partial_cmp(&a.priority_score).unwrap());
    prioritized
}

fn build_roi_context(prioritized: &[TestTarget]) -> crate::risk::roi::Context {
    crate::risk::roi::Context {
        dependency_graph: build_dependency_graph(prioritized),
        critical_paths: identify_critical_paths(prioritized),
        historical_data: None,
    }
}

fn create_recommendations(
    prioritized: &[TestTarget],
    roi_calc: &crate::risk::roi::ROICalculator,
    context: &crate::risk::roi::Context,
) -> Vector<TestingRecommendation> {
    prioritized
        .iter()
        .take(10)
        .map(|target| create_single_recommendation(target, roi_calc, context))
        .collect()
}

fn create_single_recommendation(
    target: &TestTarget,
    roi_calc: &crate::risk::roi::ROICalculator,
    context: &crate::risk::roi::Context,
) -> TestingRecommendation {
    let roi = roi_calc.calculate(target, context);

    TestingRecommendation {
        function: target
            .function
            .clone()
            .unwrap_or_else(|| target.path.to_string_lossy().to_string()),
        file: target.path.clone(),
        line: target.line,
        current_risk: target.current_risk,
        potential_risk_reduction: roi.direct_impact.percentage,
        test_effort_estimate: complexity_to_test_effort(&target.complexity),
        rationale: generate_enhanced_rationale_v2(target, &roi),
        roi: Some(roi.value),
        dependencies: target.dependencies.clone(),
        dependents: target.dependents.clone(),
    }
}

fn sort_recommendations_by_roi(recommendations: &mut Vector<TestingRecommendation>) {
    recommendations.sort_by(|a, b| {
        let a_roi = a.roi.unwrap_or(0.0);
        let b_roi = b.roi.unwrap_or(0.0);
        b_roi
            .partial_cmp(&a_roi)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
}

fn function_risk_to_target(risk: &FunctionRisk) -> TestTarget {
    let module_type = determine_module_type(&risk.file);
    let (dependencies, dependents) = infer_module_relationships(&risk.file, &module_type);

    TestTarget {
        id: format!("{}::{}", risk.file.display(), risk.function_name),
        path: risk.file.clone(),
        function: Some(risk.function_name.clone()),
        line: risk.line_range.0,
        module_type,
        current_coverage: risk.coverage_percentage.unwrap_or(0.0),
        current_risk: risk.risk_score,
        complexity: ComplexityMetrics {
            functions: vec![],
            cyclomatic_complexity: risk.cyclomatic_complexity,
            cognitive_complexity: risk.cognitive_complexity,
        },
        dependencies,
        dependents,
        lines: 100,
        priority_score: 0.0,
        debt_items: 0,
    }
}

fn build_dependency_graph(targets: &[TestTarget]) -> crate::risk::roi::DependencyGraph {
    let mut nodes = im::HashMap::new();
    let mut edges = im::Vector::new();

    for target in targets {
        nodes.insert(
            target.id.clone(),
            crate::risk::roi::DependencyNode {
                id: target.id.clone(),
                path: target.path.clone(),
                risk: target.current_risk,
                complexity: target.complexity.clone(),
            },
        );

        let edge_weight = calculate_edge_weight(target);
        for dependent in &target.dependents {
            edges.push_back(crate::risk::roi::DependencyEdge {
                from: target.id.clone(),
                to: dependent.clone(),
                weight: edge_weight,
            });
        }
    }

    crate::risk::roi::DependencyGraph { nodes, edges }
}

fn calculate_edge_weight(target: &TestTarget) -> f64 {
    let base_weight = match target.module_type {
        ModuleType::EntryPoint | ModuleType::Core => 1.0,
        ModuleType::Api | ModuleType::Model => 0.8,
        ModuleType::IO => 0.6,
        _ => 0.4,
    };

    let risk_factor = (target.current_risk / 10.0).min(1.5);
    base_weight * risk_factor
}

fn identify_critical_paths(targets: &[TestTarget]) -> Vec<PathBuf> {
    targets
        .iter()
        .filter(|t| matches!(t.module_type, ModuleType::EntryPoint | ModuleType::Core))
        .map(|t| t.path.clone())
        .collect()
}

fn complexity_to_test_effort(complexity: &ComplexityMetrics) -> TestEffort {
    let cyclomatic = complexity.cyclomatic_complexity;
    let cognitive = complexity.cognitive_complexity;
    let combined = cyclomatic + cognitive / 2;

    let difficulty = match combined {
        0..=5 => Difficulty::Trivial,
        6..=10 => Difficulty::Simple,
        11..=20 => Difficulty::Moderate,
        _ => Difficulty::Complex,
    };

    TestEffort {
        estimated_difficulty: difficulty,
        cognitive_load: cognitive,
        branch_count: cyclomatic,
        recommended_test_cases: cyclomatic + 1,
    }
}

pub fn identify_well_tested_complex_functions(
    functions: &Vector<FunctionRisk>,
    complexity_threshold: u32,
    coverage_threshold: f64,
) -> Vector<FunctionRisk> {
    functions
        .iter()
        .filter(|f| {
            let avg_complexity = (f.cyclomatic_complexity + f.cognitive_complexity) / 2;
            match f.coverage_percentage {
                Some(cov) => avg_complexity > complexity_threshold && cov >= coverage_threshold,
                None => false,
            }
        })
        .cloned()
        .collect()
}

pub fn calculate_dynamic_coverage_threshold(complexity: u32) -> f64 {
    let threshold = 50.0 + (complexity as f64 * 2.0);
    threshold.min(100.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_zero_coverage_prioritization() {
        let target = TestTarget {
            id: "test1".to_string(),
            path: PathBuf::from("src/main.rs"),
            function: Some("main".to_string()),
            line: 1,
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
            line: 1,
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
            line: 1,
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
            line: 10,
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
            line: 5,
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

    #[test]
    fn test_calculate_edge_weight_entry_point_and_core() {
        let entry_target = TestTarget {
            id: "entry".to_string(),
            path: PathBuf::from("src/main.rs"),
            function: Some("main".to_string()),
            line: 1,
            module_type: ModuleType::EntryPoint,
            current_coverage: 0.0,
            current_risk: 5.0,
            complexity: ComplexityMetrics::default(),
            dependencies: vec![],
            dependents: vec![],
            lines: 50,
            priority_score: 0.0,
            debt_items: 0,
        };

        let core_target = TestTarget {
            id: "core".to_string(),
            path: PathBuf::from("src/core/engine.rs"),
            function: Some("process".to_string()),
            line: 10,
            module_type: ModuleType::Core,
            current_coverage: 0.0,
            current_risk: 8.0,
            complexity: ComplexityMetrics::default(),
            dependencies: vec![],
            dependents: vec![],
            lines: 100,
            priority_score: 0.0,
            debt_items: 0,
        };

        assert_eq!(calculate_edge_weight(&entry_target), 0.5);
        assert_eq!(calculate_edge_weight(&core_target), 0.8);
    }

    #[test]
    fn test_complexity_to_test_effort_trivial() {
        let complexity = ComplexityMetrics {
            functions: vec![],
            cyclomatic_complexity: 2,
            cognitive_complexity: 3,
        };

        let effort = complexity_to_test_effort(&complexity);

        assert_eq!(effort.estimated_difficulty, Difficulty::Trivial);
        assert_eq!(effort.cognitive_load, 3);
        assert_eq!(effort.branch_count, 2);
        assert_eq!(effort.recommended_test_cases, 3);
    }

    #[test]
    fn test_complexity_to_test_effort_simple() {
        let complexity = ComplexityMetrics {
            functions: vec![],
            cyclomatic_complexity: 5,
            cognitive_complexity: 8,
        };

        let effort = complexity_to_test_effort(&complexity);

        assert_eq!(effort.estimated_difficulty, Difficulty::Simple);
        assert_eq!(effort.cognitive_load, 8);
        assert_eq!(effort.branch_count, 5);
        assert_eq!(effort.recommended_test_cases, 6);
    }

    #[test]
    fn test_complexity_to_test_effort_moderate() {
        let complexity = ComplexityMetrics {
            functions: vec![],
            cyclomatic_complexity: 10,
            cognitive_complexity: 16,
        };

        let effort = complexity_to_test_effort(&complexity);

        assert_eq!(effort.estimated_difficulty, Difficulty::Moderate);
        assert_eq!(effort.cognitive_load, 16);
        assert_eq!(effort.branch_count, 10);
        assert_eq!(effort.recommended_test_cases, 11);
    }

    #[test]
    fn test_complexity_to_test_effort_complex() {
        let complexity = ComplexityMetrics {
            functions: vec![],
            cyclomatic_complexity: 15,
            cognitive_complexity: 30,
        };

        let effort = complexity_to_test_effort(&complexity);

        assert_eq!(effort.estimated_difficulty, Difficulty::Complex);
        assert_eq!(effort.cognitive_load, 30);
        assert_eq!(effort.branch_count, 15);
        assert_eq!(effort.recommended_test_cases, 16);
    }

    #[test]
    fn test_calculate_edge_weight_api_and_model() {
        let api_target = TestTarget {
            id: "api".to_string(),
            path: PathBuf::from("src/api/handler.rs"),
            function: Some("handle_request".to_string()),
            line: 5,
            module_type: ModuleType::Api,
            current_coverage: 0.0,
            current_risk: 3.0,
            complexity: ComplexityMetrics::default(),
            dependencies: vec![],
            dependents: vec![],
            lines: 30,
            priority_score: 0.0,
            debt_items: 0,
        };

        let model_target = TestTarget {
            id: "model".to_string(),
            path: PathBuf::from("src/models/user.rs"),
            function: Some("validate".to_string()),
            line: 15,
            module_type: ModuleType::Model,
            current_coverage: 0.0,
            current_risk: 10.0,
            complexity: ComplexityMetrics::default(),
            dependencies: vec![],
            dependents: vec![],
            lines: 40,
            priority_score: 0.0,
            debt_items: 0,
        };

        assert_eq!(calculate_edge_weight(&api_target), 0.24);
        assert_eq!(calculate_edge_weight(&model_target), 0.8);
    }

    #[test]
    fn test_calculate_edge_weight_io_and_other() {
        let io_target = TestTarget {
            id: "io".to_string(),
            path: PathBuf::from("src/io/file.rs"),
            function: Some("read_file".to_string()),
            line: 20,
            module_type: ModuleType::IO,
            current_coverage: 0.0,
            current_risk: 6.0,
            complexity: ComplexityMetrics::default(),
            dependencies: vec![],
            dependents: vec![],
            lines: 60,
            priority_score: 0.0,
            debt_items: 0,
        };

        let utility_target = TestTarget {
            id: "util".to_string(),
            path: PathBuf::from("src/utils/helper.rs"),
            function: Some("format_string".to_string()),
            line: 25,
            module_type: ModuleType::Utility,
            current_coverage: 0.0,
            current_risk: 2.0,
            complexity: ComplexityMetrics::default(),
            dependencies: vec![],
            dependents: vec![],
            lines: 20,
            priority_score: 0.0,
            debt_items: 0,
        };

        assert!((calculate_edge_weight(&io_target) - 0.36).abs() < 1e-10);
        assert!((calculate_edge_weight(&utility_target) - 0.08).abs() < 1e-10);
    }

    #[test]
    fn test_calculate_edge_weight_risk_factor_capping() {
        let high_risk_target = TestTarget {
            id: "high_risk".to_string(),
            path: PathBuf::from("src/main.rs"),
            function: Some("critical".to_string()),
            line: 1,
            module_type: ModuleType::EntryPoint,
            current_coverage: 0.0,
            current_risk: 20.0,
            complexity: ComplexityMetrics::default(),
            dependencies: vec![],
            dependents: vec![],
            lines: 100,
            priority_score: 0.0,
            debt_items: 0,
        };

        let extreme_risk_target = TestTarget {
            id: "extreme_risk".to_string(),
            path: PathBuf::from("src/core/critical.rs"),
            function: Some("process".to_string()),
            line: 1,
            module_type: ModuleType::Core,
            current_coverage: 0.0,
            current_risk: 50.0,
            complexity: ComplexityMetrics::default(),
            dependencies: vec![],
            dependents: vec![],
            lines: 200,
            priority_score: 0.0,
            debt_items: 10,
        };

        let zero_risk_target = TestTarget {
            id: "zero_risk".to_string(),
            path: PathBuf::from("src/api/safe.rs"),
            function: Some("safe_handler".to_string()),
            line: 1,
            module_type: ModuleType::Api,
            current_coverage: 100.0,
            current_risk: 0.0,
            complexity: ComplexityMetrics::default(),
            dependencies: vec![],
            dependents: vec![],
            lines: 10,
            priority_score: 0.0,
            debt_items: 0,
        };

        assert_eq!(calculate_edge_weight(&high_risk_target), 1.5);
        assert_eq!(calculate_edge_weight(&extreme_risk_target), 1.5);
        assert_eq!(calculate_edge_weight(&zero_risk_target), 0.0);
    }
}
