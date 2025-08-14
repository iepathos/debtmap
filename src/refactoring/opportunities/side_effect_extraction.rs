use crate::core::{FileMetrics, FunctionMetrics};
use crate::refactoring::{
    DetectedPattern, EffortEstimate, FunctionRole, IoShellSpec, PatternType, Priority,
    PureFunctionSpec, RefactoringDetector, RefactoringOpportunity, TestabilityLevel,
};

pub struct SideEffectExtractionDetector;

impl RefactoringDetector for SideEffectExtractionDetector {
    fn detect_opportunities(
        &self,
        function: &FunctionMetrics,
        _file: &FileMetrics,
        role: &FunctionRole,
        patterns: &[DetectedPattern],
    ) -> Vec<RefactoringOpportunity> {
        // Check for mixed concerns (I/O with business logic)
        let has_mixed_concerns = patterns.iter().any(|p| {
            matches!(p.pattern_type, PatternType::MixedConcerns(_))
                || matches!(p.pattern_type, PatternType::SideEffects)
        });

        // Only suggest extraction for non-IO functions with side effects
        let should_extract = has_mixed_concerns
            && !matches!(role, FunctionRole::IOOrchestrator { .. })
            && function.cyclomatic > 3;

        if should_extract {
            vec![RefactoringOpportunity::ExtractSideEffects {
                mixed_function: function.name.clone(),
                pure_core: PureFunctionSpec {
                    name: format!("{}_pure", function.name),
                    inputs: vec!["data: &Input".to_string()],
                    output: "Result<Output, Error>".to_string(),
                    purpose: "Pure business logic without side effects".to_string(),
                    no_side_effects: true,
                    testability: TestabilityLevel::Easy,
                },
                io_shell: IoShellSpec {
                    name: format!("{}_io", function.name),
                    io_operations: detect_io_operations(patterns),
                    delegates_to: vec![format!("{}_pure", function.name)],
                },
                benefits: vec![
                    "Business logic becomes unit testable without mocks".to_string(),
                    "Clear separation between pure logic and I/O".to_string(),
                    "Follows functional core / imperative shell pattern".to_string(),
                    "Easier to reason about and maintain".to_string(),
                ],
                effort_estimate: if function.cyclomatic > 10 {
                    EffortEstimate::Medium
                } else {
                    EffortEstimate::Low
                },
            }]
        } else {
            vec![]
        }
    }

    fn priority(&self) -> Priority {
        Priority::High
    }
}

fn detect_io_operations(patterns: &[DetectedPattern]) -> Vec<String> {
    let mut operations = Vec::new();

    for pattern in patterns {
        if let PatternType::MixedConcerns(ref mixing) = pattern.pattern_type {
            for concern in &mixing.concerns {
                if concern.contains("I/O") {
                    operations.push("File or network I/O".to_string());
                }
                if concern.contains("Database") {
                    operations.push("Database operations".to_string());
                }
            }
        } else if matches!(pattern.pattern_type, PatternType::SideEffects) {
            operations.push("Side effects (I/O or state mutation)".to_string());
        }
    }

    if operations.is_empty() {
        operations.push("Unspecified side effects".to_string());
    }

    operations
}

pub struct MixedLogicDetector;

impl RefactoringDetector for MixedLogicDetector {
    fn detect_opportunities(
        &self,
        function: &FunctionMetrics,
        _file: &FileMetrics,
        _role: &FunctionRole,
        patterns: &[DetectedPattern],
    ) -> Vec<RefactoringOpportunity> {
        // Look for functions that have both formatting and business logic
        let has_formatting = function.name.contains("format") || function.name.contains("display");
        let has_logic = function.cyclomatic > 3;
        let has_io = patterns
            .iter()
            .any(|p| matches!(p.pattern_type, PatternType::SideEffects));

        if has_formatting && has_logic && has_io {
            vec![RefactoringOpportunity::ExtractSideEffects {
                mixed_function: function.name.clone(),
                pure_core: PureFunctionSpec {
                    name: format!("{}_format_logic", function.name),
                    inputs: vec!["data: &Data".to_string()],
                    output: "FormattedOutput".to_string(),
                    purpose: "Pure formatting logic".to_string(),
                    no_side_effects: true,
                    testability: TestabilityLevel::Trivial,
                },
                io_shell: IoShellSpec {
                    name: format!("{}_write", function.name),
                    io_operations: vec!["Write formatted output".to_string()],
                    delegates_to: vec![format!("{}_format_logic", function.name)],
                },
                benefits: vec![
                    "Formatting logic becomes testable without I/O".to_string(),
                    "Can reuse formatting in different contexts".to_string(),
                    "Simplifies testing with example-based tests".to_string(),
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
