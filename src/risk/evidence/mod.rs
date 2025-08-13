pub mod change_analyzer;
pub mod complexity_analyzer;
pub mod coupling_analyzer;
pub mod coverage_analyzer;

use crate::priority::semantic_classifier::FunctionRole;
use crate::priority::FunctionVisibility;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RiskType {
    Complexity {
        cyclomatic: u32,
        cognitive: u32,
        lines: u32,
        threshold_type: ComplexityThreshold,
    },
    Coverage {
        coverage_percentage: f64,
        critical_paths_uncovered: u32,
        test_quality: TestQuality,
    },
    Coupling {
        afferent_coupling: u32,
        efferent_coupling: u32,
        instability: f64,
        circular_dependencies: u32,
    },
    ChangeFrequency {
        commits_last_month: u32,
        bug_fix_ratio: f64,
        hotspot_intensity: f64,
    },
    Architecture {
        layer_violations: u32,
        god_class_indicators: Vec<String>,
        single_responsibility_score: f64,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum RiskSeverity {
    None,     // No significant risk
    Low,      // Monitor but no immediate action needed
    Moderate, // Should be addressed in next sprint
    High,     // Should be addressed this sprint
    Critical, // Immediate attention required
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskAssessment {
    pub score: f64,
    pub classification: RiskClassification,
    pub factors: Vec<RiskFactor>,
    pub role_context: FunctionRole,
    pub recommendations: Vec<RemediationAction>,
    pub confidence: f64,
    pub explanation: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum RiskClassification {
    WellDesigned,     // Score 0.0-2.0 - Good example
    Acceptable,       // Score 2.0-4.0 - Minor improvements possible
    NeedsImprovement, // Score 4.0-7.0 - Should be refactored
    Risky,            // Score 7.0-9.0 - High priority for improvement
    Critical,         // Score 9.0-10.0 - Immediate attention required
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskFactor {
    pub risk_type: RiskType,
    pub score: f64,
    pub severity: RiskSeverity,
    pub evidence: RiskEvidence,
    pub remediation_actions: Vec<RemediationAction>,
    pub weight: f64,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RiskEvidence {
    Complexity(ComplexityEvidence),
    Coverage(CoverageEvidence),
    Coupling(CouplingEvidence),
    ChangeFrequency(ChangeEvidence),
    Architecture(ArchitectureEvidence),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplexityEvidence {
    pub cyclomatic_complexity: u32,
    pub cognitive_complexity: u32,
    pub lines_of_code: u32,
    pub nesting_depth: u32,
    pub threshold_exceeded: bool,
    pub role_adjusted: bool,
    pub comparison_to_baseline: ComparisonResult,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageEvidence {
    pub coverage_percentage: f64,
    pub critical_paths_uncovered: u32,
    pub test_count: u32,
    pub test_quality: TestQuality,
    pub comparison_to_baseline: ComparisonResult,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CouplingEvidence {
    pub afferent_coupling: u32,
    pub efferent_coupling: u32,
    pub instability: f64,
    pub circular_dependencies: u32,
    pub comparison_to_baseline: ComparisonResult,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeEvidence {
    pub commits_last_month: u32,
    pub bug_fix_ratio: f64,
    pub hotspot_intensity: f64,
    pub comparison_to_baseline: ComparisonResult,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchitectureEvidence {
    pub layer_violations: u32,
    pub god_class_indicators: Vec<String>,
    pub single_responsibility_score: f64,
    pub comparison_to_baseline: ComparisonResult,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum ComparisonResult {
    BelowMedian, // Better than 50% of similar functions
    AboveMedian, // Worse than 50% of similar functions
    AboveP75,    // Worse than 75% of similar functions
    AboveP90,    // Worse than 90% of similar functions
    AboveP95,    // Worse than 95% of similar functions
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum ComplexityThreshold {
    Low,
    Moderate,
    High,
    Critical,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum TestQuality {
    Excellent,
    Good,
    Adequate,
    Poor,
    Missing,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RemediationAction {
    RefactorComplexity {
        current_complexity: u32,
        target_complexity: u32,
        suggested_techniques: Vec<RefactoringTechnique>,
        estimated_effort_hours: u32,
        expected_risk_reduction: f64,
    },
    AddTestCoverage {
        current_coverage: f64,
        target_coverage: f64,
        critical_paths: Vec<String>,
        test_types_needed: Vec<TestType>,
        estimated_effort_hours: u32,
    },
    ReduceCoupling {
        current_coupling: CouplingMetrics,
        coupling_issues: Vec<CouplingIssue>,
        suggested_patterns: Vec<DesignPattern>,
        estimated_effort_hours: u32,
    },
    ExtractLogic {
        extraction_candidates: Vec<ExtractionCandidate>,
        pure_function_opportunities: u32,
        testability_improvement: f64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RefactoringTechnique {
    ExtractMethod,
    ReduceNesting,
    EliminateElseAfterReturn,
    ReplaceConditionalWithPolymorphism,
    IntroduceParameterObject,
    ExtractClass,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TestType {
    Unit,
    Integration,
    Property,
    Parameterized,
    EdgeCase,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CouplingMetrics {
    pub afferent: u32,
    pub efferent: u32,
    pub instability: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CouplingIssue {
    CircularDependency(String),
    HighInstability,
    TooManyDependencies,
    GodClass,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DesignPattern {
    DependencyInjection,
    StrategyPattern,
    ObserverPattern,
    FacadePattern,
    AdapterPattern,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionCandidate {
    pub start_line: usize,
    pub end_line: usize,
    pub description: String,
    pub complexity_reduction: u32,
}

pub struct RiskContext {
    pub role: FunctionRole,
    pub visibility: FunctionVisibility,
    pub module_type: ModuleType,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ModuleType {
    Core,
    Api,
    Util,
    Test,
    Infrastructure,
}
