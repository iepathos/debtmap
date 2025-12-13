//! Heuristic-based recommendation generators
//!
//! Pure functions that generate refactoring recommendations using
//! heuristics when AST-based pattern analysis is unavailable.
//! Following Stillwater philosophy: pure core with clear data transformations.

use crate::core::FunctionMetrics;
use crate::priority::call_graph::FunctionId;
use crate::priority::TransitiveCoverage;

use super::complexity_generators::RecommendationOutput;
use super::pattern_generators::has_good_coverage;

/// Function characteristics derived from metrics and data flow
#[derive(Debug, Clone)]
pub struct FunctionCharacteristics {
    /// Has high branching (cyclomatic > 7)
    pub has_high_branching: bool,
    /// Has deep nesting (> 3 levels)
    pub has_deep_nesting: bool,
    /// Has complex cognition (cognitive > 2x cyclomatic)
    pub has_complex_cognition: bool,
    /// Number of data dependencies
    pub num_dependencies: usize,
    /// Whether function is pure
    pub is_pure: bool,
    /// Confidence in purity assessment
    pub purity_confidence: f32,
}

/// Analyze function to determine characteristics
pub fn analyze_function_characteristics(
    func: &FunctionMetrics,
    cyclomatic: u32,
    cognitive: u32,
    data_flow: Option<&crate::data_flow::DataFlowGraph>,
) -> FunctionCharacteristics {
    FunctionCharacteristics {
        has_high_branching: cyclomatic > 7,
        has_deep_nesting: func.nesting > 3,
        has_complex_cognition: cognitive > cyclomatic * 2,
        num_dependencies: extract_dependencies_count(func, data_flow),
        is_pure: func.is_pure.unwrap_or(false),
        purity_confidence: func.purity_confidence.unwrap_or(0.0),
    }
}

/// Extract dependency count from data flow graph
fn extract_dependencies_count(
    func: &FunctionMetrics,
    data_flow: Option<&crate::data_flow::DataFlowGraph>,
) -> usize {
    data_flow
        .and_then(|df| {
            let func_id = FunctionId::new(func.file.clone(), func.name.clone(), func.line);
            df.get_variable_dependencies(&func_id).map(|d| d.len())
        })
        .unwrap_or(0)
}

/// Generate heuristic-based recommendations with line estimates
pub fn generate_heuristic_recommendations_with_line_estimates(
    func: &FunctionMetrics,
    cyclomatic: u32,
    cognitive: u32,
    coverage: &Option<TransitiveCoverage>,
    data_flow: Option<&crate::data_flow::DataFlowGraph>,
) -> RecommendationOutput {
    let characteristics = analyze_function_characteristics(func, cyclomatic, cognitive, data_flow);
    let (extractions, steps, complexity_reduction) =
        generate_extraction_recommendations(&characteristics, cyclomatic, cognitive, func.nesting);

    let mut all_steps = steps;
    all_steps.extend(generate_purity_recommendations(&characteristics));
    all_steps.extend(generate_data_flow_recommendations(func, data_flow));
    all_steps.extend(generate_heuristic_coverage_steps(
        func,
        coverage,
        &extractions,
        cyclomatic,
    ));

    let action = build_heuristic_action(&extractions, cyclomatic, complexity_reduction);
    let rationale = build_heuristic_rationale(
        cyclomatic,
        cognitive,
        func.nesting,
        &extractions,
        complexity_reduction,
    );

    (action, rationale, all_steps)
}

/// Generate extraction recommendations based on function characteristics
fn generate_extraction_recommendations(
    characteristics: &FunctionCharacteristics,
    cyclomatic: u32,
    cognitive: u32,
    nesting: u32,
) -> (Vec<&'static str>, Vec<String>, u32) {
    let mut extractions = Vec::new();
    let mut steps = Vec::new();
    let mut reduction = 0u32;

    if characteristics.has_high_branching {
        extractions.push("validation logic");
        let branches_to_extract = cyclomatic / 4;
        steps.push(format!(
            "Identify validation checks from {} branches -> extract as validate_*()",
            branches_to_extract
        ));
        reduction += branches_to_extract;
    }

    if characteristics.has_deep_nesting {
        extractions.push("nested processing");
        steps.push(format!(
            "Extract nested logic (depth {}) -> process_*() functions",
            nesting
        ));
        reduction += 2;
    }

    if characteristics.has_complex_cognition {
        extractions.push("complex calculations");
        let calc_complexity = cognitive / 5;
        steps.push(format!(
            "Extract calculations from {} cognitive complexity -> calculate_*()",
            calc_complexity
        ));
        reduction += calc_complexity;
    }

    if characteristics.num_dependencies > 5 {
        extractions.push("data transformation pipeline");
        steps.push(format!(
            "Create data transformation pipeline to manage {} dependencies",
            characteristics.num_dependencies
        ));
        reduction += 1;
    }

    (extractions, steps, reduction)
}

/// Generate purity-based recommendations
fn generate_purity_recommendations(characteristics: &FunctionCharacteristics) -> Vec<String> {
    match (characteristics.is_pure, characteristics.purity_confidence) {
        (true, conf) if conf > 0.8 => {
            vec![
                "Function is likely pure - focus on breaking down into smaller pure functions"
                    .to_string(),
            ]
        }
        (_, conf) if conf < 0.3 => {
            vec!["Isolate side effects at function boundaries before extraction".to_string()]
        }
        _ => vec![],
    }
}

/// Generate data flow based recommendations
fn generate_data_flow_recommendations(
    func: &FunctionMetrics,
    data_flow: Option<&crate::data_flow::DataFlowGraph>,
) -> Vec<String> {
    let mut recommendations = Vec::new();

    if let Some(df) = data_flow {
        let func_id = FunctionId::new(func.file.clone(), func.name.clone(), func.line);

        if let Some(mutation_info) = df.get_mutation_info(&func_id) {
            if mutation_info.is_pure() {
                recommendations
                    .push("Function is pure - consider extracting as utility".to_string());
            }
        }

        if let Some(io_ops) = df.get_io_operations(&func_id) {
            if !io_ops.is_empty() {
                recommendations.push(format!(
                    "Isolate {} I/O operation(s) to separate function",
                    io_ops.len()
                ));
            }
        }
    }

    recommendations
}

/// Generate coverage steps for heuristic recommendations
fn generate_heuristic_coverage_steps(
    func: &FunctionMetrics,
    coverage: &Option<TransitiveCoverage>,
    extractions: &[&str],
    cyclomatic: u32,
) -> Vec<String> {
    let mut steps = Vec::new();
    let has_good_cov = has_good_coverage(coverage);

    if let Some(cov) = coverage {
        if !cov.uncovered_lines.is_empty() && !has_good_cov {
            use crate::priority::scoring::recommendation::analyze_uncovered_lines;
            let uncovered_recs = analyze_uncovered_lines(func, &cov.uncovered_lines);
            steps.extend(uncovered_recs);
        }
    }

    if !has_good_cov {
        let test_count = calculate_test_count(extractions, cyclomatic);
        steps.push(format!(
            "Add {} unit tests (3-5 per extracted function)",
            test_count
        ));
    }

    steps
}

/// Calculate recommended test count
fn calculate_test_count(extractions: &[&str], cyclomatic: u32) -> u32 {
    if extractions.is_empty() {
        (cyclomatic / 2).max(3)
    } else {
        (extractions.len() as u32) * 4
    }
}

/// Build action string for heuristic recommendations
fn build_heuristic_action(
    extractions: &[&str],
    cyclomatic: u32,
    complexity_reduction: u32,
) -> String {
    let target_complexity = cyclomatic.saturating_sub(complexity_reduction);

    if extractions.is_empty() {
        format!(
            "Refactor to reduce complexity from {} -> ~{}",
            cyclomatic, target_complexity
        )
    } else {
        format!(
            "Extract {} to reduce complexity {} -> ~{}",
            extractions.join(", "),
            cyclomatic,
            target_complexity
        )
    }
}

/// Build rationale string for heuristic recommendations
fn build_heuristic_rationale(
    cyclomatic: u32,
    cognitive: u32,
    nesting: u32,
    extractions: &[&str],
    complexity_reduction: u32,
) -> String {
    let reduction_percentage = if cyclomatic > 0 {
        (complexity_reduction as f32 / cyclomatic as f32 * 100.0) as u32
    } else {
        0
    };

    format!(
        "Complex function (cyclo={}, cog={}, nesting={}) with {} suggested extraction patterns. Predicted complexity reduction: {}%",
        cyclomatic,
        cognitive,
        nesting,
        extractions.len(),
        reduction_percentage
    )
}

/// Generate coverage-focused recommendation when coverage is the primary issue
pub fn generate_coverage_focused_recommendation(
    func: &FunctionMetrics,
    cyclomatic: u32,
    cognitive: u32,
    cov: &TransitiveCoverage,
) -> RecommendationOutput {
    use crate::priority::scoring::recommendation::analyze_uncovered_lines;

    let coverage_pct = cov.direct * 100.0;
    let uncovered_count = cov.uncovered_lines.len();

    let action = format!(
        "Add tests to improve coverage from {:.1}% to >80% ({} uncovered lines)",
        coverage_pct, uncovered_count
    );

    let rationale = format!(
        "Function has poor test coverage ({:.1}%) with {} uncovered lines. \
         With complexity of {} (cyclomatic) and {} (cognitive), this function needs {} test cases minimum. \
         Improving coverage will reduce risk and enable safe refactoring.",
        coverage_pct, uncovered_count, cyclomatic, cognitive, cyclomatic
    );

    let mut steps = vec![];

    let uncovered_recommendations = analyze_uncovered_lines(func, &cov.uncovered_lines);
    for rec in uncovered_recommendations {
        steps.push(rec);
    }

    if cyclomatic > 10 {
        steps.push(format!(
            "Focus on high-risk paths first - this function has {} independent execution paths",
            cyclomatic
        ));
    }

    if func.nesting > 3 {
        steps.push("Test deeply nested conditions with edge cases".to_string());
    }

    steps.push(format!(
        "Target: Add {} test cases to achieve >80% coverage",
        (cyclomatic as f32 * 0.8).ceil() as u32
    ));

    if cyclomatic > 7 {
        steps.push(
            "After achieving coverage, consider refactoring to reduce complexity".to_string(),
        );
    }

    (action, rationale, steps)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn create_test_function() -> FunctionMetrics {
        FunctionMetrics {
            name: "test_func".to_string(),
            file: PathBuf::from("test.rs"),
            line: 10,
            cyclomatic: 5,
            cognitive: 8,
            nesting: 2,
            length: 50,
            is_test: false,
            visibility: None,
            is_trait_method: false,
            in_test_module: false,
            entropy_score: None,
            is_pure: None,
            purity_confidence: None,
            purity_reason: None,
            call_dependencies: None,
            detected_patterns: None,
            upstream_callers: None,
            downstream_callees: None,
            mapping_pattern_result: None,
            adjusted_complexity: None,
            composition_metrics: None,
            language_specific: None,
            purity_level: None,
            error_swallowing_count: None,
            error_swallowing_patterns: None,
        }
    }

    #[test]
    fn test_analyze_function_characteristics() {
        let func = create_test_function();
        let chars = analyze_function_characteristics(&func, 10, 25, None);

        assert!(chars.has_high_branching); // 10 > 7
        assert!(!chars.has_deep_nesting); // 2 <= 3
        assert!(chars.has_complex_cognition); // 25 > 10*2
        assert_eq!(chars.num_dependencies, 0);
    }

    #[test]
    fn test_generate_extraction_recommendations_high_branching() {
        let chars = FunctionCharacteristics {
            has_high_branching: true,
            has_deep_nesting: false,
            has_complex_cognition: false,
            num_dependencies: 2,
            is_pure: false,
            purity_confidence: 0.5,
        };

        let (extractions, steps, reduction) =
            generate_extraction_recommendations(&chars, 12, 15, 2);

        assert!(extractions.contains(&"validation logic"));
        assert!(!steps.is_empty());
        assert!(reduction > 0);
    }

    #[test]
    fn test_generate_extraction_recommendations_deep_nesting() {
        let chars = FunctionCharacteristics {
            has_high_branching: false,
            has_deep_nesting: true,
            has_complex_cognition: false,
            num_dependencies: 2,
            is_pure: false,
            purity_confidence: 0.5,
        };

        let (extractions, steps, _) = generate_extraction_recommendations(&chars, 5, 8, 5);

        assert!(extractions.contains(&"nested processing"));
        assert!(steps.iter().any(|s| s.contains("depth 5")));
    }

    #[test]
    fn test_generate_purity_recommendations() {
        let pure_confident = FunctionCharacteristics {
            has_high_branching: false,
            has_deep_nesting: false,
            has_complex_cognition: false,
            num_dependencies: 0,
            is_pure: true,
            purity_confidence: 0.9,
        };
        let recs = generate_purity_recommendations(&pure_confident);
        assert!(recs.iter().any(|r| r.contains("likely pure")));

        let impure_uncertain = FunctionCharacteristics {
            has_high_branching: false,
            has_deep_nesting: false,
            has_complex_cognition: false,
            num_dependencies: 0,
            is_pure: false,
            purity_confidence: 0.2,
        };
        let recs2 = generate_purity_recommendations(&impure_uncertain);
        assert!(recs2.iter().any(|r| r.contains("Isolate side effects")));
    }

    #[test]
    fn test_calculate_test_count() {
        // No extractions - based on cyclomatic
        assert_eq!(calculate_test_count(&[], 10), 5);
        assert_eq!(calculate_test_count(&[], 4), 3); // min 3

        // With extractions - 4 per extraction
        assert_eq!(calculate_test_count(&["a", "b"], 10), 8);
        assert_eq!(calculate_test_count(&["a", "b", "c"], 10), 12);
    }

    #[test]
    fn test_build_heuristic_action() {
        let with_extractions =
            build_heuristic_action(&["validation logic", "nested processing"], 15, 5);
        assert!(with_extractions.contains("validation logic, nested processing"));
        assert!(with_extractions.contains("15 -> ~10"));

        let without_extractions = build_heuristic_action(&[], 15, 5);
        assert!(without_extractions.contains("Refactor"));
        assert!(without_extractions.contains("15 -> ~10"));
    }

    #[test]
    fn test_build_heuristic_rationale() {
        let rationale = build_heuristic_rationale(15, 25, 4, &["a", "b"], 5);

        assert!(rationale.contains("cyclo=15"));
        assert!(rationale.contains("cog=25"));
        assert!(rationale.contains("nesting=4"));
        assert!(rationale.contains("2 suggested extraction"));
        assert!(rationale.contains("33%")); // 5/15 = 33%
    }
}
