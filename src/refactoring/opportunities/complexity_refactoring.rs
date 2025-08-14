use crate::core::{FileMetrics, FunctionMetrics};
use crate::refactoring::{
    ComplexityLevel, DetectedPattern, EffortEstimate, FunctionRole, FunctionalPattern, Priority,
    PureFunctionSpec, RefactoringDetector, RefactoringOpportunity, TestabilityLevel,
};

pub struct HighComplexityDetector;

impl RefactoringDetector for HighComplexityDetector {
    fn detect_opportunities(
        &self,
        function: &FunctionMetrics,
        _file: &FileMetrics,
        role: &FunctionRole,
        _patterns: &[DetectedPattern],
    ) -> Vec<RefactoringOpportunity> {
        let tolerance = match role {
            FunctionRole::PureLogic {
                complexity_tolerance,
                ..
            } => *complexity_tolerance,
            FunctionRole::IOOrchestrator {
                complexity_tolerance,
                ..
            } => *complexity_tolerance,
            _ => 5,
        };

        if function.cyclomatic > tolerance {
            let excess_complexity = function.cyclomatic - tolerance;
            let functions_to_extract = ((excess_complexity as f32) / 3.0).ceil() as u32 + 1;

            vec![RefactoringOpportunity::ExtractPureFunctions {
                source_function: function.name.clone(),
                complexity_level: if function.cyclomatic > 15 {
                    ComplexityLevel::Severe
                } else if function.cyclomatic > 10 {
                    ComplexityLevel::High
                } else {
                    ComplexityLevel::Moderate
                },
                extraction_strategy:
                    crate::refactoring::ExtractionStrategy::DirectFunctionalTransformation {
                        patterns_to_apply: vec![
                            FunctionalPattern::MapOverLoop,
                            FunctionalPattern::FilterPredicate,
                        ],
                        functions_to_extract,
                    },
                suggested_functions: generate_suggested_functions(
                    &function.name,
                    functions_to_extract,
                ),
                functional_patterns: vec![
                    FunctionalPattern::MapOverLoop,
                    FunctionalPattern::FilterPredicate,
                    FunctionalPattern::ComposeFunctions,
                ],
                benefits: vec![
                    format!(
                        "Reduces complexity from {} to ~{}",
                        function.cyclomatic, tolerance
                    ),
                    "Pure functions are easily unit tested".to_string(),
                    "Improves code readability and maintainability".to_string(),
                ],
                effort_estimate: if functions_to_extract > 3 {
                    EffortEstimate::Medium
                } else {
                    EffortEstimate::Low
                },
                example: None,
            }]
        } else {
            vec![]
        }
    }

    fn priority(&self) -> Priority {
        Priority::High
    }
}

fn generate_suggested_functions(base_name: &str, count: u32) -> Vec<PureFunctionSpec> {
    let mut functions = Vec::new();

    if count > 0 {
        functions.push(PureFunctionSpec {
            name: format!("{}_validate", base_name),
            inputs: vec!["input: &Input".to_string()],
            output: "Result<ValidInput, Error>".to_string(),
            purpose: "Validate input data".to_string(),
            no_side_effects: true,
            testability: TestabilityLevel::Easy,
        });
    }

    if count > 1 {
        functions.push(PureFunctionSpec {
            name: format!("{}_transform", base_name),
            inputs: vec!["data: ValidInput".to_string()],
            output: "TransformedData".to_string(),
            purpose: "Transform validated data".to_string(),
            no_side_effects: true,
            testability: TestabilityLevel::Easy,
        });
    }

    if count > 2 {
        functions.push(PureFunctionSpec {
            name: format!("{}_process", base_name),
            inputs: vec!["data: TransformedData".to_string()],
            output: "ProcessedResult".to_string(),
            purpose: "Apply business logic".to_string(),
            no_side_effects: true,
            testability: TestabilityLevel::Moderate,
        });
    }

    if count > 3 {
        for i in 4..=count {
            functions.push(PureFunctionSpec {
                name: format!("{}_step_{}", base_name, i),
                inputs: vec!["data: &Data".to_string()],
                output: "StepResult".to_string(),
                purpose: format!("Processing step {}", i),
                no_side_effects: true,
                testability: TestabilityLevel::Easy,
            });
        }
    }

    functions
}
