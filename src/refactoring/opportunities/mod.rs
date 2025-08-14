use crate::core::{FileMetrics, FunctionMetrics};
use crate::refactoring::{
    ComplexityLevel, DetectedPattern, EffortEstimate, ExtractionStrategy, FunctionRole,
    FunctionalPattern, FunctionalTransformExample, IoShellSpec, MonadicPattern, Priority,
    PureFunctionSpec, RefactoringDetector, RefactoringOpportunity, TestabilityLevel,
};
use std::sync::Arc;

mod complexity_refactoring;
mod functional_transformation;
mod side_effect_extraction;

pub use complexity_refactoring::*;
pub use functional_transformation::*;
pub use side_effect_extraction::*;

pub fn create_refactoring_detectors() -> Vec<Arc<dyn RefactoringDetector>> {
    vec![
        Arc::new(ComplexityRefactoringDetector),
        Arc::new(FunctionalTransformationDetector),
        Arc::new(SideEffectExtractionDetector),
    ]
}

// Main complexity-based refactoring detector
pub struct ComplexityRefactoringDetector;

impl RefactoringDetector for ComplexityRefactoringDetector {
    fn detect_opportunities(
        &self,
        function: &FunctionMetrics,
        _file: &FileMetrics,
        _role: &FunctionRole,
        _patterns: &[DetectedPattern],
    ) -> Vec<RefactoringOpportunity> {
        let complexity_level = categorize_complexity(function.cyclomatic);

        match complexity_level {
            ComplexityLevel::Low => vec![], // No refactoring needed
            ComplexityLevel::Moderate => {
                vec![create_moderate_complexity_refactoring(function)]
            }
            ComplexityLevel::High => {
                vec![create_high_complexity_refactoring(function)]
            }
            ComplexityLevel::Severe => {
                vec![create_severe_complexity_refactoring(function)]
            }
        }
    }

    fn priority(&self) -> Priority {
        Priority::High
    }
}

fn categorize_complexity(complexity: u32) -> ComplexityLevel {
    match complexity {
        0..=5 => ComplexityLevel::Low,
        6..=10 => ComplexityLevel::Moderate,
        11..=15 => ComplexityLevel::High,
        _ => ComplexityLevel::Severe,
    }
}

fn create_moderate_complexity_refactoring(function: &FunctionMetrics) -> RefactoringOpportunity {
    RefactoringOpportunity::ExtractPureFunctions {
        source_function: function.name.clone(),
        complexity_level: ComplexityLevel::Moderate,
        extraction_strategy: ExtractionStrategy::DirectFunctionalTransformation {
            patterns_to_apply: vec![
                FunctionalPattern::MapOverLoop,
                FunctionalPattern::FilterPredicate,
                FunctionalPattern::FoldAccumulation,
            ],
            functions_to_extract: 2,
        },
        suggested_functions: vec![
            PureFunctionSpec {
                name: format!("{}_transform", function.name),
                inputs: vec!["data: &[T]".to_string()],
                output: "Vec<U>".to_string(),
                purpose: "Pure transformation logic".to_string(),
                no_side_effects: true,
                testability: TestabilityLevel::Easy,
            },
            PureFunctionSpec {
                name: format!("{}_validate", function.name),
                inputs: vec!["item: &T".to_string()],
                output: "bool".to_string(),
                purpose: "Pure validation predicate".to_string(),
                no_side_effects: true,
                testability: TestabilityLevel::Trivial,
            },
        ],
        functional_patterns: vec![
            FunctionalPattern::MapOverLoop,
            FunctionalPattern::FilterPredicate,
        ],
        benefits: vec![
            "Pure functions are easily testable".to_string(),
            "Functional patterns are more declarative".to_string(),
            "Immutable transformations prevent bugs".to_string(),
        ],
        effort_estimate: EffortEstimate::Low,
        example: Some(FunctionalTransformExample {
            before_imperative: r#"
// Before: Imperative loop with mutable state
let mut results = Vec::new();
for item in items {
    if item.is_valid() {
        results.push(item.transform());
    }
}"#
            .to_string(),
            after_functional: r#"
// After: Functional transformation
items.iter()
    .filter(|item| validate_item(item))  // Pure function
    .map(|item| transform_item(item))     // Pure function
    .collect()"#
                .to_string(),
            patterns_applied: vec![
                FunctionalPattern::FilterPredicate,
                FunctionalPattern::MapOverLoop,
            ],
            benefits_demonstrated: vec![
                "No mutable state".to_string(),
                "Clear data flow".to_string(),
                "Testable pure functions".to_string(),
            ],
        }),
    }
}

fn create_high_complexity_refactoring(function: &FunctionMetrics) -> RefactoringOpportunity {
    RefactoringOpportunity::ExtractPureFunctions {
        source_function: function.name.clone(),
        complexity_level: ComplexityLevel::High,
        extraction_strategy: ExtractionStrategy::DecomposeAndTransform {
            decomposition_steps: vec![
                "Identify logical sections of the function".to_string(),
                "Extract each section as a named function".to_string(),
                "Convert extracted functions to pure functions".to_string(),
                "Apply functional patterns to each".to_string(),
            ],
            functions_to_extract: 4,
            then_apply_patterns: vec![
                FunctionalPattern::ComposeFunctions,
                FunctionalPattern::Pipeline,
            ],
        },
        suggested_functions: vec![
            PureFunctionSpec {
                name: format!("{}_parse", function.name),
                inputs: vec!["input: &str".to_string()],
                output: "Result<ParsedData, Error>".to_string(),
                purpose: "Parse and validate input".to_string(),
                no_side_effects: true,
                testability: TestabilityLevel::Easy,
            },
            PureFunctionSpec {
                name: format!("{}_process", function.name),
                inputs: vec!["data: ParsedData".to_string()],
                output: "ProcessedData".to_string(),
                purpose: "Core business logic transformation".to_string(),
                no_side_effects: true,
                testability: TestabilityLevel::Easy,
            },
            PureFunctionSpec {
                name: format!("{}_validate", function.name),
                inputs: vec!["data: &ProcessedData".to_string()],
                output: "Result<(), ValidationError>".to_string(),
                purpose: "Validate processed results".to_string(),
                no_side_effects: true,
                testability: TestabilityLevel::Easy,
            },
            PureFunctionSpec {
                name: format!("{}_format", function.name),
                inputs: vec!["data: ProcessedData".to_string()],
                output: "String".to_string(),
                purpose: "Format output for display".to_string(),
                no_side_effects: true,
                testability: TestabilityLevel::Trivial,
            },
        ],
        functional_patterns: vec![
            FunctionalPattern::Pipeline,
            FunctionalPattern::Monadic(MonadicPattern::Result),
        ],
        benefits: vec![
            "Reduces complexity while maintaining functional purity".to_string(),
            "Each function has single responsibility".to_string(),
            "Pipeline composition makes flow clear".to_string(),
            "Error handling with Result monad".to_string(),
        ],
        effort_estimate: EffortEstimate::Medium,
        example: Some(FunctionalTransformExample {
            before_imperative:
                "// Complex function with multiple responsibilities\n// 15+ lines of mixed logic"
                    .to_string(),
            after_functional: r#"
// After: Function pipeline
input
    .parse_data()           // Pure function
    .and_then(process_data) // Pure function  
    .and_then(validate_data) // Pure function
    .map(format_output)      // Pure function"#
                .to_string(),
            patterns_applied: vec![
                FunctionalPattern::Pipeline,
                FunctionalPattern::Monadic(MonadicPattern::Result),
            ],
            benefits_demonstrated: vec![
                "Clear separation of concerns".to_string(),
                "Monadic error handling".to_string(),
                "Each step is testable".to_string(),
            ],
        }),
    }
}

fn create_severe_complexity_refactoring(function: &FunctionMetrics) -> RefactoringOpportunity {
    RefactoringOpportunity::ExtractPureFunctions {
        source_function: function.name.clone(),
        complexity_level: ComplexityLevel::Severe,
        extraction_strategy: ExtractionStrategy::ArchitecturalRefactoring {
            extract_modules: vec![
                format!("{}_core", function.name),
                format!("{}_validation", function.name),
                format!("{}_transformation", function.name),
            ],
            pure_core_functions: vec![
                PureFunctionSpec {
                    name: "validate_input".to_string(),
                    inputs: vec!["input: &Input".to_string()],
                    output: "Result<ValidatedInput, ValidationError>".to_string(),
                    purpose: "Input validation logic".to_string(),
                    no_side_effects: true,
                    testability: TestabilityLevel::Easy,
                },
                PureFunctionSpec {
                    name: "transform_data".to_string(),
                    inputs: vec!["data: ValidatedInput".to_string()],
                    output: "TransformedData".to_string(),
                    purpose: "Core transformation logic".to_string(),
                    no_side_effects: true,
                    testability: TestabilityLevel::Moderate,
                },
                PureFunctionSpec {
                    name: "apply_business_rules".to_string(),
                    inputs: vec!["data: TransformedData".to_string()],
                    output: "Result<FinalData, BusinessError>".to_string(),
                    purpose: "Business rule application".to_string(),
                    no_side_effects: true,
                    testability: TestabilityLevel::Moderate,
                },
            ],
            design_imperative_shell: IoShellSpec {
                name: format!("{}_orchestrator", function.name),
                io_operations: vec![
                    "Read input from source".to_string(),
                    "Write output to destination".to_string(),
                ],
                delegates_to: vec![
                    "validate_input".to_string(),
                    "transform_data".to_string(),
                    "apply_business_rules".to_string(),
                ],
            },
        },
        suggested_functions: vec![],
        functional_patterns: vec![
            FunctionalPattern::Monadic(MonadicPattern::Result),
            FunctionalPattern::ComposeFunctions,
            FunctionalPattern::Pipeline,
        ],
        benefits: vec![
            "Transforms monolithic function into modular architecture".to_string(),
            "Establishes functional core / imperative shell".to_string(),
            "Each module is independently testable".to_string(),
            "Enables parallel development and testing".to_string(),
            "Simplifies future maintenance and extensions".to_string(),
        ],
        effort_estimate: EffortEstimate::High,
        example: Some(FunctionalTransformExample {
            before_imperative: "// Monolithic function with 50+ lines\n// Multiple nested conditions and loops\n// Mixed I/O and business logic".to_string(),
            after_functional: r#"
// Functional core modules
mod validation { /* pure functions */ }
mod transformation { /* pure functions */ }
mod business_rules { /* pure functions */ }

// Imperative shell orchestrator
fn orchestrate(input_path: &Path) -> Result<()> {
    let input = read_input(input_path)?;  // I/O at boundary
    
    let result = validation::validate(input)
        .and_then(transformation::transform)
        .and_then(business_rules::apply)?;
    
    write_output(result)?;  // I/O at boundary
    Ok(())
}"#.to_string(),
            patterns_applied: vec![
                FunctionalPattern::Monadic(MonadicPattern::Result),
                FunctionalPattern::Pipeline,
            ],
            benefits_demonstrated: vec![
                "Clear architectural separation".to_string(),
                "Functional core is pure and testable".to_string(),
                "I/O isolated at boundaries".to_string(),
            ],
        }),
    }
}
