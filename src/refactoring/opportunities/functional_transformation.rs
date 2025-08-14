use crate::core::{FileMetrics, FunctionMetrics};
use crate::refactoring::{
    DetectedPattern, EffortEstimate, FunctionRole, FunctionalPattern, ImperativePattern,
    PatternType, Priority, RefactoringDetector, RefactoringOpportunity, TransformationStep,
};

pub struct FunctionalTransformationDetector;

impl RefactoringDetector for FunctionalTransformationDetector {
    fn detect_opportunities(
        &self,
        function: &FunctionMetrics,
        _file: &FileMetrics,
        _role: &FunctionRole,
        patterns: &[DetectedPattern],
    ) -> Vec<RefactoringOpportunity> {
        let mut opportunities = Vec::new();

        // Check for imperative patterns that can be converted
        let imperative_patterns: Vec<ImperativePattern> = patterns
            .iter()
            .filter_map(|p| match &p.pattern_type {
                PatternType::ImperativeLoop => Some(ImperativePattern::MutableLoop),
                PatternType::MutableState => Some(ImperativePattern::StateModification),
                _ => None,
            })
            .collect();

        if !imperative_patterns.is_empty() {
            opportunities.push(RefactoringOpportunity::ConvertToFunctionalStyle {
                imperative_function: function.name.clone(),
                current_patterns: imperative_patterns.clone(),
                target_patterns: suggest_functional_alternatives(&imperative_patterns),
                transformation_steps: generate_transformation_steps(&imperative_patterns),
                benefits: vec![
                    "Eliminate mutable state".to_string(),
                    "Make code more declarative".to_string(),
                    "Improve testability".to_string(),
                    "Enable safe parallelization".to_string(),
                ],
                effort_estimate: EffortEstimate::Low,
            });
        }

        opportunities
    }

    fn priority(&self) -> Priority {
        Priority::Medium
    }
}

fn suggest_functional_alternatives(imperative: &[ImperativePattern]) -> Vec<FunctionalPattern> {
    let mut patterns = Vec::new();

    for pattern in imperative {
        match pattern {
            ImperativePattern::MutableLoop => {
                patterns.push(FunctionalPattern::MapOverLoop);
                patterns.push(FunctionalPattern::FilterPredicate);
                patterns.push(FunctionalPattern::FoldAccumulation);
            }
            ImperativePattern::StateModification => {
                patterns.push(FunctionalPattern::FoldAccumulation);
                patterns.push(FunctionalPattern::Monadic(
                    crate::refactoring::MonadicPattern::State,
                ));
            }
            ImperativePattern::NestedConditions => {
                patterns.push(FunctionalPattern::PatternMatchOverIfElse);
                patterns.push(FunctionalPattern::Monadic(
                    crate::refactoring::MonadicPattern::Option,
                ));
            }
            ImperativePattern::SideEffectMixing => {
                patterns.push(FunctionalPattern::ComposeFunctions);
                patterns.push(FunctionalPattern::Pipeline);
            }
        }
    }

    patterns
}

fn generate_transformation_steps(imperative: &[ImperativePattern]) -> Vec<TransformationStep> {
    let mut steps = Vec::new();

    for pattern in imperative {
        match pattern {
            ImperativePattern::MutableLoop => {
                steps.push(TransformationStep {
                    description: "Replace for loop with iterator methods".to_string(),
                    pattern_applied: FunctionalPattern::MapOverLoop,
                });
                steps.push(TransformationStep {
                    description: "Extract loop body as pure function".to_string(),
                    pattern_applied: FunctionalPattern::ComposeFunctions,
                });
            }
            ImperativePattern::StateModification => {
                steps.push(TransformationStep {
                    description: "Replace mutable variables with fold/reduce".to_string(),
                    pattern_applied: FunctionalPattern::FoldAccumulation,
                });
            }
            ImperativePattern::NestedConditions => {
                steps.push(TransformationStep {
                    description: "Convert if-else chains to pattern matching".to_string(),
                    pattern_applied: FunctionalPattern::PatternMatchOverIfElse,
                });
            }
            ImperativePattern::SideEffectMixing => {
                steps.push(TransformationStep {
                    description: "Extract side effects to boundaries".to_string(),
                    pattern_applied: FunctionalPattern::Pipeline,
                });
            }
        }
    }

    steps
}

pub struct LoopToFunctionalDetector;

impl RefactoringDetector for LoopToFunctionalDetector {
    fn detect_opportunities(
        &self,
        function: &FunctionMetrics,
        _file: &FileMetrics,
        _role: &FunctionRole,
        patterns: &[DetectedPattern],
    ) -> Vec<RefactoringOpportunity> {
        let has_loops = patterns
            .iter()
            .any(|p| matches!(p.pattern_type, PatternType::ImperativeLoop));

        if has_loops && function.cyclomatic > 3 {
            vec![RefactoringOpportunity::ConvertToFunctionalStyle {
                imperative_function: function.name.clone(),
                current_patterns: vec![ImperativePattern::MutableLoop],
                target_patterns: vec![
                    FunctionalPattern::MapOverLoop,
                    FunctionalPattern::FilterPredicate,
                    FunctionalPattern::FoldAccumulation,
                ],
                transformation_steps: vec![
                    TransformationStep {
                        description: "Identify loop purpose (transform/filter/aggregate)"
                            .to_string(),
                        pattern_applied: FunctionalPattern::MapOverLoop,
                    },
                    TransformationStep {
                        description: "Replace loop with appropriate iterator method".to_string(),
                        pattern_applied: FunctionalPattern::FilterPredicate,
                    },
                    TransformationStep {
                        description: "Extract loop body as pure function".to_string(),
                        pattern_applied: FunctionalPattern::ComposeFunctions,
                    },
                ],
                benefits: vec![
                    "More declarative and readable".to_string(),
                    "Eliminates off-by-one errors".to_string(),
                    "Enables lazy evaluation".to_string(),
                    "Parallelizable with rayon".to_string(),
                ],
                effort_estimate: EffortEstimate::Low,
            }]
        } else {
            vec![]
        }
    }

    fn priority(&self) -> Priority {
        Priority::Medium
    }
}
