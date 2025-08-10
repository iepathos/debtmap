use super::{Difficulty, FunctionRisk, RiskAnalyzer, TestEffort, TestingRecommendation};
use crate::core::ComplexityMetrics;
use im::{HashMap, Vector};
use std::path::{Path, PathBuf};

// =============== Prioritization Pipeline ===============

pub trait PrioritizationStage {
    fn process(&self, targets: Vec<TestTarget>) -> Vec<TestTarget>;
    fn name(&self) -> &str;
}

pub struct PrioritizationPipeline {
    stages: Vec<Box<dyn PrioritizationStage>>,
}

impl Default for PrioritizationPipeline {
    fn default() -> Self {
        Self::new()
    }
}

impl PrioritizationPipeline {
    pub fn new() -> Self {
        Self {
            stages: vec![
                Box::new(ZeroCoverageStage::new()),
                Box::new(CriticalPathStage::new()),
                Box::new(ComplexityRiskStage::new()),
                Box::new(DependencyImpactStage::new()),
                Box::new(EffortOptimizationStage::new()),
            ],
        }
    }

    pub fn process(&self, targets: Vec<TestTarget>) -> Vec<TestTarget> {
        self.stages
            .iter()
            .fold(targets, |acc, stage| stage.process(acc))
    }
}

// =============== Test Target ===============

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

#[derive(Clone, Debug, PartialEq)]
pub enum ModuleType {
    EntryPoint, // main.rs, lib.rs
    Core,       // Core business logic
    Api,        // API/service layer
    Model,      // Data models
    IO,         // Input/output modules
    Utility,    // Helper utilities
    Test,       // Test modules
    Unknown,    // Default/unknown
}

// =============== Prioritization Stages ===============

pub struct ZeroCoverageStage {
    boost_factor: f64,
}

impl Default for ZeroCoverageStage {
    fn default() -> Self {
        Self::new()
    }
}

impl ZeroCoverageStage {
    pub fn new() -> Self {
        Self {
            boost_factor: 100.0,
        }
    }
}

impl PrioritizationStage for ZeroCoverageStage {
    fn process(&self, mut targets: Vec<TestTarget>) -> Vec<TestTarget> {
        for target in &mut targets {
            if target.current_coverage == 0.0 {
                // Heavily boost zero-coverage items, scaled by criticality
                let criticality_factor = match target.module_type {
                    ModuleType::EntryPoint => 10.0,
                    ModuleType::Core => 8.0,
                    ModuleType::Api => 6.0,
                    ModuleType::Model => 4.0,
                    ModuleType::IO => 3.0,
                    ModuleType::Utility => 2.0,
                    _ => 1.0,
                };

                // Factor in size of untested code
                let size_factor = (target.lines as f64).ln().max(1.0);

                target.priority_score += self.boost_factor * criticality_factor * size_factor;
            }
        }
        targets
    }

    fn name(&self) -> &str {
        "ZeroCoverageStage"
    }
}

pub struct CriticalPathStage {
    scorer: CriticalityScorer,
}

impl Default for CriticalPathStage {
    fn default() -> Self {
        Self::new()
    }
}

impl CriticalPathStage {
    pub fn new() -> Self {
        Self {
            scorer: CriticalityScorer::new(),
        }
    }
}

impl PrioritizationStage for CriticalPathStage {
    fn process(&self, mut targets: Vec<TestTarget>) -> Vec<TestTarget> {
        for target in &mut targets {
            let criticality = self.scorer.score(target);
            target.priority_score += criticality * 10.0;
        }
        targets
    }

    fn name(&self) -> &str {
        "CriticalPathStage"
    }
}

pub struct ComplexityRiskStage;

impl Default for ComplexityRiskStage {
    fn default() -> Self {
        Self::new()
    }
}

impl ComplexityRiskStage {
    pub fn new() -> Self {
        Self
    }
}

impl PrioritizationStage for ComplexityRiskStage {
    fn process(&self, mut targets: Vec<TestTarget>) -> Vec<TestTarget> {
        for target in &mut targets {
            // Complexity contribution to priority
            let complexity_score = (target.complexity.cyclomatic_complexity as f64
                + target.complexity.cognitive_complexity as f64)
                / 2.0;

            // Scale by current risk
            target.priority_score += complexity_score * target.current_risk / 10.0;
        }
        targets
    }

    fn name(&self) -> &str {
        "ComplexityRiskStage"
    }
}

pub struct DependencyImpactStage;

impl Default for DependencyImpactStage {
    fn default() -> Self {
        Self::new()
    }
}

impl DependencyImpactStage {
    pub fn new() -> Self {
        Self
    }
}

impl PrioritizationStage for DependencyImpactStage {
    fn process(&self, mut targets: Vec<TestTarget>) -> Vec<TestTarget> {
        for target in &mut targets {
            // More dependents = higher impact when fixed
            let impact_factor = (target.dependents.len() as f64).sqrt();
            target.priority_score += impact_factor * 5.0;
        }
        targets
    }

    fn name(&self) -> &str {
        "DependencyImpactStage"
    }
}

pub struct EffortOptimizationStage;

impl Default for EffortOptimizationStage {
    fn default() -> Self {
        Self::new()
    }
}

impl EffortOptimizationStage {
    pub fn new() -> Self {
        Self
    }
}

impl PrioritizationStage for EffortOptimizationStage {
    fn process(&self, mut targets: Vec<TestTarget>) -> Vec<TestTarget> {
        for target in &mut targets {
            // Adjust priority by estimated effort (favor quick wins)
            let effort = EffortEstimator::new().estimate(target);
            if effort > 0.0 {
                // Divide by effort to get ROI-like score
                target.priority_score /= effort.sqrt();
            }
        }
        targets
    }

    fn name(&self) -> &str {
        "EffortOptimizationStage"
    }
}

// =============== Criticality Scorer ===============

pub struct CriticalityScorer {
    patterns: HashMap<String, f64>,
}

impl Default for CriticalityScorer {
    fn default() -> Self {
        Self::new()
    }
}

impl CriticalityScorer {
    pub fn new() -> Self {
        let mut patterns = HashMap::new();
        patterns.insert("main".to_string(), 10.0);
        patterns.insert("lib".to_string(), 10.0);
        patterns.insert("core".to_string(), 8.0);
        patterns.insert("api".to_string(), 7.0);
        patterns.insert("service".to_string(), 6.0);
        patterns.insert("model".to_string(), 5.0);
        patterns.insert("handler".to_string(), 6.0);
        patterns.insert("controller".to_string(), 6.0);
        patterns.insert("repository".to_string(), 5.0);
        patterns.insert("util".to_string(), 3.0);
        patterns.insert("helper".to_string(), 3.0);
        patterns.insert("test".to_string(), 1.0);

        Self { patterns }
    }

    pub fn score(&self, target: &TestTarget) -> f64 {
        let base_score = self.pattern_match_score(&target.path);
        let dependency_factor = self.dependency_score(target);
        let size_factor = (target.lines as f64).ln() / 10.0;
        let debt_factor = 1.0 + (target.debt_items as f64 * 0.1);

        (base_score * dependency_factor * size_factor * debt_factor).min(10.0)
    }

    fn pattern_match_score(&self, path: &Path) -> f64 {
        let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        // Check exact matches first
        match file_name {
            "main.rs" | "lib.rs" => return 10.0,
            _ => {}
        }

        // Check pattern matches
        let file_lower = file_name.to_lowercase();
        for (pattern, score) in self.patterns.iter() {
            if file_lower.contains(pattern) {
                return *score;
            }
        }

        // Check path components for module type hints
        let path_str = path.to_string_lossy().to_lowercase();
        for (pattern, score) in self.patterns.iter() {
            if path_str.contains(pattern) {
                return *score * 0.8; // Slightly lower score for path matches
            }
        }

        4.0 // Default score
    }

    fn dependency_score(&self, target: &TestTarget) -> f64 {
        let dependent_count = target.dependents.len() as f64;
        let dependency_count = target.dependencies.len() as f64;

        // More dependents = more critical
        // More dependencies = potentially more complex
        let dependent_factor = (1.0 + dependent_count / 10.0).min(2.0);
        let dependency_factor = (1.0 + dependency_count / 20.0).min(1.5);

        dependent_factor * dependency_factor
    }
}

// =============== ROI Calculator (Legacy) ===============
// This is kept for backward compatibility but delegates to the new advanced ROI system

pub use super::roi::{
    ROIBreakdown, ROICalculator as AdvancedROICalculator, ROIComponent, ROIConfig,
    ROI as AdvancedROI,
};

pub struct ROICalculator {
    advanced_calculator: AdvancedROICalculator,
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
        // Create context for advanced calculator
        let context = self.create_context(target);

        // Use advanced calculator
        let advanced_roi = self.advanced_calculator.calculate(target, &context);

        // Convert to legacy ROI format for compatibility
        ROI {
            value: advanced_roi.value,
            risk_reduction: advanced_roi.direct_impact.percentage,
            effort: advanced_roi.effort.hours,
            cascade_impact: advanced_roi.cascade_impact.total_risk_reduction,
            explanation: advanced_roi.breakdown.explanation,
        }
    }

    fn create_context(&self, target: &TestTarget) -> super::roi::Context {
        // Build dependency graph from target information
        let mut nodes = im::HashMap::new();
        let mut edges = im::Vector::new();

        // Add current target as a node
        nodes.insert(
            target.id.clone(),
            super::roi::DependencyNode {
                id: target.id.clone(),
                path: target.path.clone(),
                risk: target.current_risk,
                complexity: target.complexity.clone(),
            },
        );

        // Add dependents as edges
        for dependent in &target.dependents {
            edges.push_back(super::roi::DependencyEdge {
                from: target.id.clone(),
                to: dependent.clone(),
                weight: 0.8, // Default weight
            });
        }

        super::roi::Context {
            dependency_graph: super::roi::DependencyGraph { nodes, edges },
            critical_paths: vec![],
            historical_data: None,
        }
    }
}

// Legacy ROI struct for backward compatibility
#[derive(Clone, Debug)]
pub struct ROI {
    pub value: f64,
    pub risk_reduction: f64,
    pub effort: f64,
    pub cascade_impact: f64,
    pub explanation: String,
}

// =============== Effort Estimator ===============

pub struct EffortEstimator;

impl Default for EffortEstimator {
    fn default() -> Self {
        Self::new()
    }
}

impl EffortEstimator {
    pub fn new() -> Self {
        Self
    }

    pub fn estimate(&self, target: &TestTarget) -> f64 {
        let base_effort = self.complexity_to_test_cases(&target.complexity);
        let setup_effort = self.estimate_setup_complexity(target);
        let mock_effort = self.estimate_mocking_needs(target);

        base_effort + setup_effort + mock_effort
    }

    fn complexity_to_test_cases(&self, complexity: &ComplexityMetrics) -> f64 {
        // McCabe's formula: minimum test cases = cyclomatic complexity + 1
        let min_cases = complexity.cyclomatic_complexity as f64 + 1.0;

        // Adjust for cognitive complexity
        let cognitive_factor = (complexity.cognitive_complexity as f64 / 10.0).max(1.0);

        min_cases * cognitive_factor
    }

    fn estimate_setup_complexity(&self, target: &TestTarget) -> f64 {
        // Entry points and IO modules need more setup
        match target.module_type {
            ModuleType::EntryPoint => 5.0,
            ModuleType::IO => 3.0,
            ModuleType::Api => 2.0,
            ModuleType::Core => 1.0,
            _ => 0.5,
        }
    }

    fn estimate_mocking_needs(&self, target: &TestTarget) -> f64 {
        // More dependencies = more mocking needed
        let dep_count = target.dependencies.len() as f64;
        dep_count * 0.5
    }

    pub fn explain(&self, target: &TestTarget) -> String {
        let base = self.complexity_to_test_cases(&target.complexity);
        let setup = self.estimate_setup_complexity(target);
        let mocking = self.estimate_mocking_needs(target);

        format!(
            "Estimated effort: {:.0} (base: {:.0}, setup: {:.0}, mocking: {:.0})",
            base + setup + mocking,
            base,
            setup,
            mocking
        )
    }
}

// =============== Testing Recommendations ===============

#[derive(Clone, Debug)]
pub struct TestRecommendation {
    pub target: TestTarget,
    pub priority: f64,
    pub roi: ROI,
    pub effort: TestEffortDetails,
    pub impact: ImpactAnalysis,
    pub rationale: String,
    pub suggested_approach: TestApproach,
}

#[derive(Clone, Debug)]
pub struct TestEffortDetails {
    pub estimated_cases: usize,
    pub estimated_hours: f64,
    pub complexity_level: ComplexityLevel,
    pub setup_requirements: Vec<String>,
}

#[derive(Clone, Debug)]
pub enum ComplexityLevel {
    Trivial,
    Simple,
    Moderate,
    Complex,
    VeryComplex,
}

#[derive(Clone, Debug)]
pub enum TestApproach {
    UnitTest,
    IntegrationTest,
    ModuleTest,
    EndToEndTest,
}

#[derive(Clone, Debug)]
pub struct ImpactAnalysis {
    pub direct_risk_reduction: f64,
    pub cascade_effect: f64,
    pub affected_modules: Vec<String>,
    pub coverage_improvement: f64,
}

// =============== Main Entry Functions ===============

pub fn prioritize_by_roi(
    functions: &Vector<FunctionRisk>,
    analyzer: &RiskAnalyzer,
) -> Vector<TestingRecommendation> {
    // Convert FunctionRisk to TestTarget
    let targets: Vec<TestTarget> = functions.iter().map(function_risk_to_target).collect();

    // Run through prioritization pipeline
    let pipeline = PrioritizationPipeline::new();
    let mut prioritized = pipeline.process(targets);

    // Sort by priority score
    prioritized.sort_by(|a, b| b.priority_score.partial_cmp(&a.priority_score).unwrap());

    // Generate recommendations for top targets
    let mut recommendations = Vector::new();
    for target in prioritized.into_iter().take(10) {
        let roi_calc = ROICalculator::new(analyzer.clone());
        let roi = roi_calc.calculate(&target, 90.0);

        let recommendation = TestingRecommendation {
            function: target
                .function
                .clone()
                .unwrap_or_else(|| target.path.to_string_lossy().to_string()),
            file: target.path.clone(),
            current_risk: target.current_risk,
            potential_risk_reduction: roi.risk_reduction,
            test_effort_estimate: complexity_to_test_effort(&target.complexity),
            rationale: generate_enhanced_rationale(&target, &roi),
        };

        recommendations.push_back(recommendation);
    }

    recommendations
}

fn function_risk_to_target(risk: &FunctionRisk) -> TestTarget {
    let module_type = determine_module_type(&risk.file);

    TestTarget {
        id: format!("{}:{}", risk.file.display(), risk.function_name),
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
        dependencies: vec![],
        dependents: vec![],
        lines: (risk.line_range.1 - risk.line_range.0) + 1,
        priority_score: 0.0,
        debt_items: 0, // Would need to be populated from debt analysis
    }
}

fn determine_module_type(path: &Path) -> ModuleType {
    let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

    match file_name {
        "main.rs" => ModuleType::EntryPoint,
        "lib.rs" => ModuleType::EntryPoint,
        _ => {
            let path_str = path.to_string_lossy().to_lowercase();
            if path_str.contains("test") {
                ModuleType::Test
            } else if path_str.contains("core") {
                ModuleType::Core
            } else if path_str.contains("api") || path_str.contains("handler") {
                ModuleType::Api
            } else if path_str.contains("model") {
                ModuleType::Model
            } else if path_str.contains("io") || path_str.contains("output") {
                ModuleType::IO
            } else if path_str.contains("util") || path_str.contains("helper") {
                ModuleType::Utility
            } else {
                ModuleType::Unknown
            }
        }
    }
}

fn complexity_to_test_effort(complexity: &ComplexityMetrics) -> TestEffort {
    let cognitive = complexity.cognitive_complexity;
    let cyclomatic = complexity.cyclomatic_complexity;

    let difficulty = match cognitive {
        0..=4 => Difficulty::Trivial,
        5..=9 => Difficulty::Simple,
        10..=19 => Difficulty::Moderate,
        20..=39 => Difficulty::Complex,
        _ => Difficulty::VeryComplex,
    };

    TestEffort {
        estimated_difficulty: difficulty,
        cognitive_load: cognitive,
        branch_count: cyclomatic,
        recommended_test_cases: cyclomatic + 1,
    }
}

fn generate_enhanced_rationale(target: &TestTarget, roi: &ROI) -> String {
    let coverage_status = if target.current_coverage == 0.0 {
        match target.module_type {
            ModuleType::EntryPoint => "Critical entry point with NO test coverage",
            ModuleType::Core => "Core module completely untested",
            ModuleType::Api => "API handler with zero coverage",
            ModuleType::IO => "I/O module without any tests",
            _ => "Module has no test coverage",
        }
    } else if target.current_coverage < 30.0 {
        "Poorly tested"
    } else if target.current_coverage < 60.0 {
        "Moderately tested"
    } else {
        "Well tested"
    };

    let complexity_desc = match (
        target.complexity.cyclomatic_complexity,
        target.complexity.cognitive_complexity,
    ) {
        (c, g) if c > 20 || g > 40 => "extremely complex",
        (c, g) if c > 10 || g > 20 => "highly complex",
        (c, g) if c > 5 || g > 10 => "moderately complex",
        _ => "simple",
    };

    let impact_desc = if target.dependents.len() > 5 {
        format!(
            " - critical dependency for {} modules",
            target.dependents.len()
        )
    } else if !target.dependents.is_empty() {
        format!(" - affects {} other modules", target.dependents.len())
    } else {
        String::new()
    };

    format!(
        "{} - {} code (cyclo={}, cognitive={}){}. ROI: {:.1}x with {:.1}% risk reduction",
        coverage_status,
        complexity_desc,
        target.complexity.cyclomatic_complexity,
        target.complexity.cognitive_complexity,
        impact_desc,
        roi.value,
        roi.risk_reduction
    )
}

// =============== Utility Functions ===============

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
    // Dynamic threshold: more complex code needs higher coverage
    // Base: 50% + 2% per complexity point, max 100%
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

        assert!(targets[0].priority_score > 1000.0); // Should be heavily boosted
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

        assert!(main_score > util_score); // Entry points should score higher
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
        assert!(complex_effort > 20.0); // Should be significant for complex code
        assert!(simple_effort < 10.0); // Should be low for simple code
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
