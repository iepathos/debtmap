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

pub fn prioritize_by_roi(
    functions: &Vector<FunctionRisk>,
    analyzer: &RiskAnalyzer,
) -> Vector<TestingRecommendation> {
    // Filter out test functions before generating recommendations
    // We check against the function metrics to see if it's marked as a test
    let targets: Vec<TestTarget> = functions
        .iter()
        .filter(|f| !f.is_test_function)
        .map(function_risk_to_target)
        .collect();
    let pipeline = PrioritizationPipeline::new();
    let mut prioritized = pipeline.process(targets);
    prioritized.sort_by(|a, b| b.priority_score.partial_cmp(&a.priority_score).unwrap());

    let dependency_graph = build_dependency_graph(&prioritized);
    let critical_paths = identify_critical_paths(&prioritized);
    let roi_calc = crate::risk::roi::ROICalculator::new(analyzer.clone());

    let context = crate::risk::roi::Context {
        dependency_graph,
        critical_paths,
        historical_data: None,
    };

    let mut recommendations = Vector::new();
    for target in prioritized.into_iter().take(10) {
        let roi = roi_calc.calculate(&target, &context);

        let recommendation = TestingRecommendation {
            function: target
                .function
                .clone()
                .unwrap_or_else(|| target.path.to_string_lossy().to_string()),
            file: target.path.clone(),
            current_risk: target.current_risk,
            potential_risk_reduction: roi.direct_impact.percentage,
            test_effort_estimate: complexity_to_test_effort(&target.complexity),
            rationale: generate_enhanced_rationale_v2(&target, &roi),
            roi: Some(roi.value),
            dependencies: target.dependencies.clone(),
            dependents: target.dependents.clone(),
        };
        recommendations.push_back(recommendation);
    }

    recommendations
}

fn function_risk_to_target(risk: &FunctionRisk) -> TestTarget {
    let module_type = determine_module_type(&risk.file);
    let (dependencies, dependents) = infer_module_relationships(&risk.file, &module_type);

    TestTarget {
        id: format!("{}::{}", risk.file.display(), risk.function_name),
        path: risk.file.clone(),
        function: Some(risk.function_name.clone()),
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

pub fn identify_untested_complex_functions(
    functions: &Vector<FunctionRisk>,
    complexity_threshold: u32,
) -> Vector<FunctionRisk> {
    functions
        .iter()
        .filter(|f| {
            let avg_complexity = (f.cyclomatic_complexity + f.cognitive_complexity) / 2;
            match f.coverage_percentage {
                Some(cov) => avg_complexity > complexity_threshold && cov < 30.0,
                None => avg_complexity > complexity_threshold,
            }
        })
        .cloned()
        .collect()
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
}
