use crate::core::{FileMetrics, FunctionMetrics};
use std::sync::Arc;

pub mod guidance;
pub mod opportunities;
pub mod patterns;

pub use guidance::*;
pub use opportunities::*;
pub use patterns::*;

#[derive(Debug, Clone)]
pub struct RefactoringAnalysis {
    pub function_name: String,
    pub function_role: FunctionRole,
    pub detected_patterns: Vec<DetectedPattern>,
    pub refactoring_opportunities: Vec<RefactoringOpportunity>,
    pub quality_assessment: QualityAssessment,
    pub recommendations: Vec<Recommendation>,
}

#[derive(Debug, Clone)]
pub enum FunctionRole {
    PureLogic {
        complexity_tolerance: u32,
        testing_expectation: TestingExpectation,
    },
    IOOrchestrator {
        expected_patterns: Vec<OrchestrationPattern>,
        complexity_tolerance: u32,
    },
    FormattingFunction {
        input_types: Vec<String>,
        output_type: String,
        testability_importance: TestabilityImportance,
    },
    TraitImplementation {
        trait_name: String,
        testing_strategy: TraitTestingStrategy,
    },
    FrameworkCallback {
        framework: String,
        callback_type: CallbackType,
    },
}

#[derive(Debug, Clone)]
pub enum TestingExpectation {
    HighCoverage,
    ModerateCoverage,
    LowCoverage,
}

#[derive(Debug, Clone)]
pub enum TestabilityImportance {
    Critical,
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone)]
pub enum TraitTestingStrategy {
    TestThroughCallers,
    DirectUnitTests,
    IntegrationTests,
}

#[derive(Debug, Clone)]
pub enum CallbackType {
    EventHandler,
    Lifecycle,
    DataTransform,
}

#[derive(Debug, Clone)]
pub struct OrchestrationPattern {
    pub pattern_type: String,
    pub description: String,
}

#[derive(Debug, Clone)]
pub struct DetectedPattern {
    pub pattern_type: PatternType,
    pub confidence: f64,
    pub evidence: PatternEvidence,
    pub assessment: PatternAssessment,
}

#[derive(Debug, Clone)]
pub enum PatternType {
    IOOrchestration(OrchestrationPattern),
    PureFormatting(FormattingPattern),
    MixedConcerns(ConcernMixingPattern),
    TraitImplementation(TraitPattern),
    TestFunction(TestPattern),
    FunctionalComposition,
    ImperativeLoop,
    MutableState,
    SideEffects,
}

#[derive(Debug, Clone)]
pub struct FormattingPattern {
    pub format_type: String,
    pub complexity: u32,
}

#[derive(Debug, Clone)]
pub struct ConcernMixingPattern {
    pub concerns: Vec<String>,
    pub separation_difficulty: SeparationDifficulty,
}

#[derive(Debug, Clone)]
pub enum SeparationDifficulty {
    Trivial,
    Low,
    Medium,
    High,
    VeryHigh,
}

#[derive(Debug, Clone)]
pub struct TraitPattern {
    pub trait_name: String,
    pub method_name: String,
}

#[derive(Debug, Clone)]
pub struct TestPattern {
    pub test_type: String,
    pub framework: String,
}

#[derive(Debug, Clone)]
pub struct PatternEvidence {
    pub code_snippets: Vec<String>,
    pub line_numbers: Vec<u32>,
    pub confidence_factors: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum PatternAssessment {
    GoodExample {
        strengths: Vec<String>,
        why_good: String,
    },
    ImprovementOpportunity {
        current_issues: Vec<String>,
        potential_benefits: Vec<String>,
        refactoring_suggestions: Vec<RefactoringOpportunity>,
    },
    AntiPattern {
        problems: Vec<String>,
        recommended_patterns: Vec<PatternType>,
        urgency: Urgency,
    },
}

#[derive(Debug, Clone)]
pub enum Urgency {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone)]
pub struct QualityAssessment {
    pub overall_score: f64,
    pub strengths: Vec<String>,
    pub improvement_areas: Vec<String>,
    pub pattern_compliance: f64,
    pub role_appropriateness: f64,
}

#[derive(Debug, Clone)]
pub struct Recommendation {
    pub title: String,
    pub description: String,
    pub priority: Priority,
    pub effort_estimate: EffortEstimate,
    pub benefits: Vec<String>,
    pub example: Option<RefactoringExample>,
}

#[derive(Debug, Clone)]
pub enum Priority {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone)]
pub struct RefactoringExample {
    pub before: String,
    pub after: String,
    pub explanation: String,
}

#[derive(Debug, Clone)]
pub enum EffortEstimate {
    Trivial,     // < 15 minutes
    Low,         // 15-60 minutes
    Medium,      // 1-4 hours
    High,        // 4-8 hours
    Significant, // > 8 hours
}

pub struct PatternRecognitionEngine {
    pattern_matchers: Vec<Arc<dyn PatternMatcher>>,
    function_classifier: FunctionRoleClassifier,
    refactoring_advisor: RefactoringAdvisor,
}

impl Default for PatternRecognitionEngine {
    fn default() -> Self {
        Self {
            pattern_matchers: patterns::create_pattern_matchers(),
            function_classifier: FunctionRoleClassifier::default(),
            refactoring_advisor: RefactoringAdvisor::default(),
        }
    }
}

impl PatternRecognitionEngine {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn analyze_function(
        &self,
        function: &FunctionMetrics,
        file: &FileMetrics,
    ) -> RefactoringAnalysis {
        let role = self.function_classifier.classify(function, file);
        let patterns = self.identify_patterns(function, file);
        let opportunities = self
            .refactoring_advisor
            .find_opportunities(function, file, &role, &patterns);
        let quality = self.assess_quality(function, &patterns, &role);
        let recommendations = self.generate_recommendations(&opportunities, &quality);

        RefactoringAnalysis {
            function_name: function.name.clone(),
            function_role: role,
            detected_patterns: patterns,
            refactoring_opportunities: opportunities,
            quality_assessment: quality,
            recommendations,
        }
    }

    fn identify_patterns(
        &self,
        function: &FunctionMetrics,
        file: &FileMetrics,
    ) -> Vec<DetectedPattern> {
        self.pattern_matchers
            .iter()
            .filter_map(|matcher| matcher.match_pattern(function, file))
            .collect()
    }

    fn assess_quality(
        &self,
        function: &FunctionMetrics,
        patterns: &[DetectedPattern],
        role: &FunctionRole,
    ) -> QualityAssessment {
        let mut strengths = Vec::new();
        let mut improvement_areas = Vec::new();
        let mut pattern_score = 0.0;
        let mut pattern_count = 0.0;

        for pattern in patterns {
            pattern_count += 1.0;
            match &pattern.assessment {
                PatternAssessment::GoodExample { strengths: s, .. } => {
                    strengths.extend(s.clone());
                    pattern_score += 1.0;
                }
                PatternAssessment::ImprovementOpportunity { current_issues, .. } => {
                    improvement_areas.extend(current_issues.clone());
                    pattern_score += 0.5;
                }
                PatternAssessment::AntiPattern { problems, .. } => {
                    improvement_areas.extend(problems.clone());
                }
            }
        }

        let pattern_compliance = if pattern_count > 0.0 {
            pattern_score / pattern_count
        } else {
            1.0
        };

        let role_appropriateness = self.calculate_role_appropriateness(function, role);
        let overall_score = (pattern_compliance + role_appropriateness) / 2.0;

        QualityAssessment {
            overall_score,
            strengths,
            improvement_areas,
            pattern_compliance,
            role_appropriateness,
        }
    }

    fn calculate_role_appropriateness(
        &self,
        function: &FunctionMetrics,
        role: &FunctionRole,
    ) -> f64 {
        match role {
            FunctionRole::PureLogic {
                complexity_tolerance,
                ..
            } => {
                if function.cyclomatic <= *complexity_tolerance {
                    1.0
                } else {
                    0.5
                }
            }
            FunctionRole::IOOrchestrator {
                complexity_tolerance,
                ..
            } => {
                if function.cyclomatic <= *complexity_tolerance {
                    1.0
                } else {
                    0.7
                }
            }
            _ => 0.8,
        }
    }

    fn generate_recommendations(
        &self,
        opportunities: &[RefactoringOpportunity],
        _quality: &QualityAssessment,
    ) -> Vec<Recommendation> {
        let mut recommendations = Vec::new();

        for opportunity in opportunities {
            let recommendation = match opportunity {
                RefactoringOpportunity::ExtractPureFunctions {
                    source_function,
                    complexity_level,
                    extraction_strategy: _,
                    suggested_functions,
                    functional_patterns,
                    benefits,
                    effort_estimate,
                    example,
                } => {
                    let description = match complexity_level {
                        ComplexityLevel::Moderate => {
                            format!(
                                "Extract {} pure functions using direct functional transformation. \
                                Apply patterns: {}",
                                suggested_functions.len(),
                                functional_patterns.iter()
                                    .map(|p| format!("{:?}", p))
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            )
                        }
                        ComplexityLevel::High => {
                            format!(
                                "Extract {} pure functions using decompose-then-transform strategy. \
                                First decompose into logical units, then apply functional patterns.",
                                suggested_functions.len()
                            )
                        }
                        ComplexityLevel::Severe => {
                            format!(
                                "Architectural refactoring needed. Extract {} pure functions into modules \
                                and design functional core with imperative shell.",
                                suggested_functions.len()
                            )
                        }
                        _ => continue,
                    };

                    Recommendation {
                        title: format!("Extract pure functions from {}", source_function),
                        description,
                        priority: match complexity_level {
                            ComplexityLevel::Severe => Priority::Critical,
                            ComplexityLevel::High => Priority::High,
                            ComplexityLevel::Moderate => Priority::Medium,
                            _ => Priority::Low,
                        },
                        effort_estimate: effort_estimate.clone(),
                        benefits: benefits.clone(),
                        example: example.as_ref().map(|e| RefactoringExample {
                            before: e.before_imperative.clone(),
                            after: e.after_functional.clone(),
                            explanation: e
                                .patterns_applied
                                .iter()
                                .map(|p| format!("{:?}", p))
                                .collect::<Vec<_>>()
                                .join(", "),
                        }),
                    }
                }
                RefactoringOpportunity::ConvertToFunctionalStyle {
                    imperative_function,
                    target_patterns,
                    benefits,
                    effort_estimate,
                    ..
                } => Recommendation {
                    title: format!("Convert {} to functional style", imperative_function),
                    description: format!(
                        "Apply functional patterns: {}",
                        target_patterns
                            .iter()
                            .map(|p| format!("{:?}", p))
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                    priority: Priority::Medium,
                    effort_estimate: effort_estimate.clone(),
                    benefits: benefits.clone(),
                    example: None,
                },
                RefactoringOpportunity::ExtractSideEffects {
                    mixed_function,
                    pure_core,
                    benefits,
                    effort_estimate,
                    ..
                } => Recommendation {
                    title: format!("Extract side effects from {}", mixed_function),
                    description: format!(
                        "Create pure function '{}' and move I/O to boundaries",
                        pure_core.name
                    ),
                    priority: Priority::High,
                    effort_estimate: effort_estimate.clone(),
                    benefits: benefits.clone(),
                    example: None,
                },
            };
            recommendations.push(recommendation);
        }

        recommendations
    }
}

pub struct FunctionRoleClassifier {
    io_detectors: Vec<Arc<dyn IoDetector>>,
    formatting_detectors: Vec<Arc<dyn FormattingDetector>>,
    trait_analyzers: Vec<Arc<dyn TraitAnalyzer>>,
}

impl Default for FunctionRoleClassifier {
    fn default() -> Self {
        Self {
            io_detectors: patterns::create_io_detectors(),
            formatting_detectors: patterns::create_formatting_detectors(),
            trait_analyzers: patterns::create_trait_analyzers(),
        }
    }
}

impl FunctionRoleClassifier {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn classify(&self, function: &FunctionMetrics, file: &FileMetrics) -> FunctionRole {
        // Check for trait implementation first
        for analyzer in &self.trait_analyzers {
            if let Some(trait_info) = analyzer.detect_trait_implementation(function, file) {
                return FunctionRole::TraitImplementation {
                    trait_name: trait_info.trait_name,
                    testing_strategy: TraitTestingStrategy::TestThroughCallers,
                };
            }
        }

        // Check for IO orchestration
        for detector in &self.io_detectors {
            if let Some(io_info) = detector.detect_io_orchestration(function, file) {
                return FunctionRole::IOOrchestrator {
                    expected_patterns: io_info.patterns,
                    complexity_tolerance: 5,
                };
            }
        }

        // Check for formatting function
        for detector in &self.formatting_detectors {
            if let Some(formatting_info) = detector.detect_formatting_function(function, file) {
                return FunctionRole::FormattingFunction {
                    input_types: formatting_info.inputs,
                    output_type: formatting_info.output,
                    testability_importance: TestabilityImportance::High,
                };
            }
        }

        // Default to pure logic with strict expectations
        FunctionRole::PureLogic {
            complexity_tolerance: 3,
            testing_expectation: TestingExpectation::HighCoverage,
        }
    }
}

pub struct RefactoringAdvisor {
    opportunity_detectors: Vec<Arc<dyn RefactoringDetector>>,
}

impl Default for RefactoringAdvisor {
    fn default() -> Self {
        Self {
            opportunity_detectors: opportunities::create_refactoring_detectors(),
        }
    }
}

impl RefactoringAdvisor {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn find_opportunities(
        &self,
        function: &FunctionMetrics,
        file: &FileMetrics,
        role: &FunctionRole,
        patterns: &[DetectedPattern],
    ) -> Vec<RefactoringOpportunity> {
        let mut opportunities = Vec::new();

        for detector in &self.opportunity_detectors {
            opportunities.extend(detector.detect_opportunities(function, file, role, patterns));
        }

        // Sort by priority
        opportunities.sort_by_key(|o| match o {
            RefactoringOpportunity::ExtractPureFunctions {
                complexity_level, ..
            } => match complexity_level {
                ComplexityLevel::Severe => 0,
                ComplexityLevel::High => 1,
                ComplexityLevel::Moderate => 2,
                ComplexityLevel::Low => 3,
            },
            RefactoringOpportunity::ExtractSideEffects { .. } => 1,
            RefactoringOpportunity::ConvertToFunctionalStyle { .. } => 2,
        });

        opportunities
    }
}

// Trait definitions for extensibility
pub trait PatternMatcher: Send + Sync {
    fn match_pattern(
        &self,
        function: &FunctionMetrics,
        file: &FileMetrics,
    ) -> Option<DetectedPattern>;
}

pub trait IoDetector: Send + Sync {
    fn detect_io_orchestration(
        &self,
        function: &FunctionMetrics,
        file: &FileMetrics,
    ) -> Option<IoInfo>;
}

pub trait FormattingDetector: Send + Sync {
    fn detect_formatting_function(
        &self,
        function: &FunctionMetrics,
        file: &FileMetrics,
    ) -> Option<FormattingInfo>;
}

pub trait TraitAnalyzer: Send + Sync {
    fn detect_trait_implementation(
        &self,
        function: &FunctionMetrics,
        file: &FileMetrics,
    ) -> Option<TraitInfo>;
}

pub trait RefactoringDetector: Send + Sync {
    fn detect_opportunities(
        &self,
        function: &FunctionMetrics,
        file: &FileMetrics,
        role: &FunctionRole,
        patterns: &[DetectedPattern],
    ) -> Vec<RefactoringOpportunity>;
    fn priority(&self) -> Priority;
}

#[derive(Debug, Clone)]
pub struct IoInfo {
    pub patterns: Vec<OrchestrationPattern>,
    pub io_operations: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct FormattingInfo {
    pub inputs: Vec<String>,
    pub output: String,
    pub format_type: String,
}

#[derive(Debug, Clone)]
pub struct TraitInfo {
    pub trait_name: String,
    pub method_name: String,
}

#[derive(Debug, Clone)]
pub enum RefactoringOpportunity {
    ExtractPureFunctions {
        source_function: String,
        complexity_level: ComplexityLevel,
        extraction_strategy: ExtractionStrategy,
        suggested_functions: Vec<PureFunctionSpec>,
        functional_patterns: Vec<FunctionalPattern>,
        benefits: Vec<String>,
        effort_estimate: EffortEstimate,
        example: Option<FunctionalTransformExample>,
    },
    ConvertToFunctionalStyle {
        imperative_function: String,
        current_patterns: Vec<ImperativePattern>,
        target_patterns: Vec<FunctionalPattern>,
        transformation_steps: Vec<TransformationStep>,
        benefits: Vec<String>,
        effort_estimate: EffortEstimate,
    },
    ExtractSideEffects {
        mixed_function: String,
        pure_core: PureFunctionSpec,
        io_shell: IoShellSpec,
        benefits: Vec<String>,
        effort_estimate: EffortEstimate,
    },
}

#[derive(Debug, Clone)]
pub enum ComplexityLevel {
    Low,      // ≤5 - No action needed
    Moderate, // 6-10 - Direct functional transformation
    High,     // 11-15 - Decompose then transform
    Severe,   // >15 - Architectural refactoring
}

#[derive(Debug, Clone)]
pub enum ExtractionStrategy {
    DirectFunctionalTransformation {
        patterns_to_apply: Vec<FunctionalPattern>,
        functions_to_extract: u32,
    },
    DecomposeAndTransform {
        decomposition_steps: Vec<String>,
        functions_to_extract: u32,
        then_apply_patterns: Vec<FunctionalPattern>,
    },
    ArchitecturalRefactoring {
        extract_modules: Vec<String>,
        pure_core_functions: Vec<PureFunctionSpec>,
        design_imperative_shell: IoShellSpec,
    },
}

#[derive(Debug, Clone)]
pub struct PureFunctionSpec {
    pub name: String,
    pub inputs: Vec<String>,
    pub output: String,
    pub purpose: String,
    pub no_side_effects: bool,
    pub testability: TestabilityLevel,
}

#[derive(Debug, Clone)]
pub enum TestabilityLevel {
    Trivial,
    Easy,
    Moderate,
    Hard,
}

#[derive(Debug, Clone)]
pub struct IoShellSpec {
    pub name: String,
    pub io_operations: Vec<String>,
    pub delegates_to: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum FunctionalPattern {
    MapOverLoop,
    FilterPredicate,
    FoldAccumulation,
    PatternMatchOverIfElse,
    ComposeFunctions,
    PartialApplication,
    Monadic(MonadicPattern),
    Pipeline,
    Recursion,
}

#[derive(Debug, Clone)]
pub enum MonadicPattern {
    Option,
    Result,
    Future,
    State,
}

#[derive(Debug, Clone)]
pub enum ImperativePattern {
    MutableLoop,
    StateModification,
    NestedConditions,
    SideEffectMixing,
}

#[derive(Debug, Clone)]
pub struct TransformationStep {
    pub description: String,
    pub pattern_applied: FunctionalPattern,
}

#[derive(Debug, Clone)]
pub struct FunctionalTransformExample {
    pub before_imperative: String,
    pub after_functional: String,
    pub patterns_applied: Vec<FunctionalPattern>,
    pub benefits_demonstrated: Vec<String>,
}
