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

fn create_validation_spec(base_name: &str) -> PureFunctionSpec {
    PureFunctionSpec {
        name: format!("{}_validate", base_name),
        inputs: vec!["input: &Input".to_string()],
        output: "Result<ValidInput, Error>".to_string(),
        purpose: "Validate input data".to_string(),
        no_side_effects: true,
        testability: TestabilityLevel::Easy,
    }
}

fn create_step_spec(base_name: &str, step_num: u32) -> PureFunctionSpec {
    PureFunctionSpec {
        name: format!("{}_step_{}", base_name, step_num),
        inputs: vec!["data: &Data".to_string()],
        output: "StepResult".to_string(),
        purpose: format!("Processing step {}", step_num),
        no_side_effects: true,
        testability: TestabilityLevel::Easy,
    }
}

fn generate_suggested_functions(base_name: &str, count: u32) -> Vec<PureFunctionSpec> {
    let mut functions = Vec::new();

    if count > 0 {
        functions.push(create_validation_spec(base_name));
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
            functions.push(create_step_spec(base_name, i));
        }
    }

    functions
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_validation_spec() {
        let spec = create_validation_spec("my_function");
        assert_eq!(spec.name, "my_function_validate");
        assert_eq!(spec.inputs, vec!["input: &Input".to_string()]);
        assert_eq!(spec.output, "Result<ValidInput, Error>");
        assert_eq!(spec.purpose, "Validate input data");
        assert!(spec.no_side_effects);
        assert!(matches!(spec.testability, TestabilityLevel::Easy));
    }

    #[test]
    fn test_create_step_spec() {
        let spec = create_step_spec("process", 5);
        assert_eq!(spec.name, "process_step_5");
        assert_eq!(spec.inputs, vec!["data: &Data".to_string()]);
        assert_eq!(spec.output, "StepResult");
        assert_eq!(spec.purpose, "Processing step 5");
        assert!(spec.no_side_effects);
        assert!(matches!(spec.testability, TestabilityLevel::Easy));
    }

    #[test]
    fn test_generate_suggested_functions_zero_count() {
        let functions = generate_suggested_functions("base", 0);
        assert!(functions.is_empty());
    }

    #[test]
    fn test_generate_suggested_functions_one_count() {
        let functions = generate_suggested_functions("handler", 1);
        assert_eq!(functions.len(), 1);
        assert_eq!(functions[0].name, "handler_validate");
    }

    #[test]
    fn test_generate_suggested_functions_three_count() {
        let functions = generate_suggested_functions("processor", 3);
        assert_eq!(functions.len(), 3);
        assert_eq!(functions[0].name, "processor_validate");
        assert_eq!(functions[1].name, "processor_transform");
        assert_eq!(functions[2].name, "processor_process");
    }

    #[test]
    fn test_generate_suggested_functions_many_count() {
        let functions = generate_suggested_functions("complex", 6);
        assert_eq!(functions.len(), 6);
        assert_eq!(functions[0].name, "complex_validate");
        assert_eq!(functions[1].name, "complex_transform");
        assert_eq!(functions[2].name, "complex_process");
        assert_eq!(functions[3].name, "complex_step_4");
        assert_eq!(functions[4].name, "complex_step_5");
        assert_eq!(functions[5].name, "complex_step_6");
    }
}
