//! Pattern-based recommendation generators
//!
//! Pure functions that generate refactoring recommendations based on
//! detected extractable patterns in code. Uses AST analysis to provide
//! precise, actionable extraction suggestions.
//! Following Stillwater philosophy: pure transformations, clear data flow.

use crate::core::FunctionMetrics;
use crate::extraction_patterns::{
    ExtractionAnalyzer, ExtractionSuggestion, UnifiedExtractionAnalyzer,
};
use crate::priority::TransitiveCoverage;

use super::complexity_generators::RecommendationOutput;

/// Detect programming language from file path
pub fn detect_file_language(file_path: &std::path::Path) -> crate::core::Language {
    let extension = file_path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("rs");

    match extension {
        "rs" => crate::core::Language::Rust,
        "py" => crate::core::Language::Python,
        _ => crate::core::Language::Rust,
    }
}

/// Get pattern type name for display
pub fn pattern_type_name(
    pattern_type: &crate::extraction_patterns::ExtractablePattern,
) -> &'static str {
    use crate::extraction_patterns::ExtractablePattern;

    match pattern_type {
        ExtractablePattern::AccumulationLoop { .. } => "accumulation loop",
        ExtractablePattern::GuardChainSequence { .. } => "guard chain",
        ExtractablePattern::TransformationPipeline { .. } => "transformation pipeline",
        ExtractablePattern::SimilarBranches { .. } => "similar branches",
        ExtractablePattern::NestedExtraction { .. } => "nested extraction",
    }
}

/// Check if coverage should be prioritized
pub fn should_prioritize_coverage(coverage: &Option<TransitiveCoverage>) -> bool {
    coverage
        .as_ref()
        .map(|cov| cov.direct < 0.8 && !cov.uncovered_lines.is_empty())
        .unwrap_or(false)
}

/// Check if function has good test coverage
pub fn has_good_coverage(coverage: &Option<TransitiveCoverage>) -> bool {
    coverage.as_ref().map(|c| c.direct >= 0.8).unwrap_or(false)
}

/// Analyze function for extractable patterns
pub fn analyze_extraction_patterns(
    func: &FunctionMetrics,
    data_flow: Option<&crate::data_flow::DataFlowGraph>,
) -> Vec<ExtractionSuggestion> {
    let analyzer = UnifiedExtractionAnalyzer::new();
    let file_metrics = create_minimal_file_metrics(func);
    analyzer.analyze_function(func, &file_metrics, data_flow)
}

/// Create minimal file metrics for pattern analysis
fn create_minimal_file_metrics(func: &FunctionMetrics) -> crate::core::FileMetrics {
    crate::core::FileMetrics {
        path: func.file.clone(),
        language: detect_file_language(&func.file),
        complexity: crate::core::ComplexityMetrics::default(),
        debt_items: vec![],
        dependencies: vec![],
        duplications: vec![],
        total_lines: 0,
        module_scope: None,
        classes: None,
    }
}

/// Generate pattern-based recommendation from extraction suggestions
pub fn generate_pattern_based_recommendation(
    func: &FunctionMetrics,
    cyclomatic: u32,
    suggestions: &[ExtractionSuggestion],
    coverage: &Option<TransitiveCoverage>,
) -> RecommendationOutput {
    let top_suggestions: Vec<_> = suggestions.iter().take(3).collect();
    let (action_parts, extraction_steps, total_reduction) = process_suggestions(&top_suggestions);
    let predicted_complexity = cyclomatic.saturating_sub(total_reduction);

    let action = build_action_string(
        &action_parts,
        cyclomatic,
        predicted_complexity,
        suggestions.len(),
    );
    let rationale = build_rationale(cyclomatic, suggestions.len());

    let mut steps = extraction_steps;

    if !has_good_coverage(coverage) {
        steps.extend(generate_coverage_steps(func, coverage, suggestions.len()));
    }

    steps.push(format!(
        "Expected complexity reduction: {}%",
        calculate_reduction_percentage(total_reduction, cyclomatic)
    ));

    (action, rationale, steps)
}

/// Process extraction suggestions into action parts, steps, and reduction estimate
fn process_suggestions(suggestions: &[&ExtractionSuggestion]) -> (Vec<String>, Vec<String>, u32) {
    suggestions.iter().enumerate().fold(
        (Vec::new(), Vec::new(), 0u32),
        |(mut actions, mut steps, mut total), (i, suggestion)| {
            actions.push(format!(
                "{} (confidence: {:.0}%)",
                suggestion.suggested_name,
                suggestion.confidence * 100.0
            ));

            steps.push(format!(
                "{}. Extract {} pattern at lines {}-{} as '{}' (complexity {} -> {})",
                i + 1,
                pattern_type_name(&suggestion.pattern_type),
                suggestion.start_line,
                suggestion.end_line,
                suggestion.suggested_name,
                suggestion.complexity_reduction.current_cyclomatic,
                suggestion.complexity_reduction.predicted_cyclomatic
            ));

            total += suggestion
                .complexity_reduction
                .current_cyclomatic
                .saturating_sub(suggestion.complexity_reduction.predicted_cyclomatic);

            (actions, steps, total)
        },
    )
}

/// Build action string from extraction parts
fn build_action_string(
    action_parts: &[String],
    cyclomatic: u32,
    predicted_complexity: u32,
    total_suggestions: usize,
) -> String {
    if !action_parts.is_empty() {
        format!(
            "Extract {} to reduce complexity from {} to ~{}",
            action_parts.join(", "),
            cyclomatic,
            predicted_complexity
        )
    } else {
        format!(
            "Extract {} identified patterns to reduce complexity from {} to {}",
            total_suggestions, cyclomatic, predicted_complexity
        )
    }
}

/// Build rationale string explaining complexity and pattern benefits
fn build_rationale(cyclomatic: u32, num_patterns: usize) -> String {
    let complexity_explanation = explain_complexity(cyclomatic);
    let pattern_benefits = explain_pattern_benefits(num_patterns);

    format!(
        "{}. Function has {} extractable patterns that can be isolated. {}. Target complexity per function is 5 or less for optimal maintainability.",
        complexity_explanation,
        num_patterns,
        pattern_benefits
    )
}

/// Explain complexity impact based on cyclomatic value
fn explain_complexity(cyclomatic: u32) -> String {
    match cyclomatic {
        16.. => format!(
            "Cyclomatic complexity of {} indicates {} independent execution paths, requiring at least {} test cases for full path coverage",
            cyclomatic, cyclomatic, cyclomatic
        ),
        11..=15 => format!(
            "Cyclomatic complexity of {} indicates {} independent paths through the code, making thorough testing difficult",
            cyclomatic, cyclomatic
        ),
        6..=10 => format!(
            "Cyclomatic complexity of {} indicates {} independent paths requiring {} test cases minimum - extraction will reduce this to 3-5 tests per function",
            cyclomatic, cyclomatic, cyclomatic
        ),
        _ => format!(
            "Cyclomatic complexity of {} indicates moderate complexity that can be improved through extraction",
            cyclomatic
        ),
    }
}

/// Explain benefits of pattern extraction
fn explain_pattern_benefits(num_patterns: usize) -> String {
    match num_patterns {
        1 => "This extraction will create a focused, testable unit".to_string(),
        2 => "These extractions will separate distinct concerns into testable units".to_string(),
        _ => format!(
            "These {} extractions will decompose the function into smaller, focused units that are easier to test and understand",
            num_patterns
        ),
    }
}

/// Generate coverage improvement steps
pub fn generate_coverage_steps(
    func: &FunctionMetrics,
    coverage: &Option<TransitiveCoverage>,
    num_suggestions: usize,
) -> Vec<String> {
    let mut steps = Vec::new();

    if let Some(cov) = coverage {
        if !cov.uncovered_lines.is_empty() {
            use crate::priority::scoring::recommendation::analyze_uncovered_lines;
            steps.extend(analyze_uncovered_lines(func, &cov.uncovered_lines));
        }
    }

    steps.push(format!(
        "{}. Write unit tests for each extracted pure function",
        num_suggestions + 2
    ));
    steps.push(format!(
        "{}. Add property-based tests for complex transformations",
        num_suggestions + 3
    ));

    steps
}

/// Calculate reduction percentage
pub fn calculate_reduction_percentage(reduction: u32, total: u32) -> u32 {
    if total > 0 {
        (reduction as f32 / total as f32 * 100.0) as u32
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_detect_file_language() {
        assert_eq!(
            detect_file_language(Path::new("test.rs")),
            crate::core::Language::Rust
        );
        assert_eq!(
            detect_file_language(Path::new("test.py")),
            crate::core::Language::Python
        );
        assert_eq!(
            detect_file_language(Path::new("test.unknown")),
            crate::core::Language::Rust
        );
    }

    #[test]
    fn test_should_prioritize_coverage() {
        // No coverage data
        assert!(!should_prioritize_coverage(&None));

        // Good coverage
        let good_cov = TransitiveCoverage {
            direct: 0.85,
            transitive: 0.80,
            propagated_from: vec![],
            uncovered_lines: vec![],
        };
        assert!(!should_prioritize_coverage(&Some(good_cov)));

        // Low coverage with uncovered lines
        let low_cov = TransitiveCoverage {
            direct: 0.5,
            transitive: 0.4,
            propagated_from: vec![],
            uncovered_lines: vec![10, 20, 30],
        };
        assert!(should_prioritize_coverage(&Some(low_cov)));
    }

    #[test]
    fn test_has_good_coverage() {
        assert!(!has_good_coverage(&None));

        let good = TransitiveCoverage {
            direct: 0.85,
            transitive: 0.80,
            propagated_from: vec![],
            uncovered_lines: vec![],
        };
        assert!(has_good_coverage(&Some(good)));

        let bad = TransitiveCoverage {
            direct: 0.5,
            transitive: 0.4,
            propagated_from: vec![],
            uncovered_lines: vec![10],
        };
        assert!(!has_good_coverage(&Some(bad)));
    }

    #[test]
    fn test_calculate_reduction_percentage() {
        assert_eq!(calculate_reduction_percentage(5, 10), 50);
        assert_eq!(calculate_reduction_percentage(3, 12), 25);
        assert_eq!(calculate_reduction_percentage(0, 10), 0);
        assert_eq!(calculate_reduction_percentage(10, 0), 0);
    }

    #[test]
    fn test_explain_complexity() {
        let low = explain_complexity(4);
        assert!(low.contains("moderate complexity"));

        let medium = explain_complexity(8);
        assert!(medium.contains("independent paths"));

        let high = explain_complexity(12);
        assert!(high.contains("thorough testing difficult"));

        let very_high = explain_complexity(20);
        assert!(very_high.contains("full path coverage"));
    }

    #[test]
    fn test_explain_pattern_benefits() {
        let one = explain_pattern_benefits(1);
        assert!(one.contains("focused, testable unit"));

        let two = explain_pattern_benefits(2);
        assert!(two.contains("separate distinct concerns"));

        let many = explain_pattern_benefits(5);
        assert!(many.contains("5 extractions"));
    }

    #[test]
    fn test_build_action_string() {
        let with_parts = build_action_string(
            &["validate_input".to_string(), "process_data".to_string()],
            15,
            8,
            2,
        );
        assert!(with_parts.contains("validate_input, process_data"));
        assert!(with_parts.contains("15 to ~8"));

        let without_parts = build_action_string(&[], 15, 8, 3);
        assert!(without_parts.contains("3 identified patterns"));
    }
}
