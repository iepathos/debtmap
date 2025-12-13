// Complexity-based recommendation generation
// This module handles complexity classification, pattern-based extraction recommendations,
// coverage-focused recommendations, and heuristic recommendations for refactoring.

use crate::core::FunctionMetrics;
use crate::extraction_patterns::{ExtractionAnalyzer, UnifiedExtractionAnalyzer};
use crate::priority::call_graph::{CallGraph, FunctionId};
use crate::priority::{DebtType, FunctionVisibility, TransitiveCoverage};

/// Enum for complexity classification
#[derive(Debug, Clone)]
pub enum ComplexityLevel {
    Low,         // 1-4
    LowModerate, // 5-6
    Moderate,    // 7-10
    High,        // 11+
}

/// Classify complexity level based on cyclomatic complexity
pub fn classify_complexity_level(cyclo: u32) -> ComplexityLevel {
    match cyclo {
        1..=4 => ComplexityLevel::Low,
        5..=6 => ComplexityLevel::LowModerate,
        7..=10 => ComplexityLevel::Moderate,
        _ => ComplexityLevel::High,
    }
}

/// Helper to determine visibility from function
fn determine_visibility(func: &FunctionMetrics) -> FunctionVisibility {
    let vis_str = func.visibility.as_deref();
    match vis_str {
        Some("pub") => FunctionVisibility::Public,
        Some("pub(crate)") => FunctionVisibility::Crate,
        Some(vis) if vis.starts_with("pub(") => FunctionVisibility::Crate,
        _ => FunctionVisibility::Private,
    }
}

/// Generate enhanced dead code hints
fn generate_enhanced_dead_code_hints(
    func: &FunctionMetrics,
    visibility: &FunctionVisibility,
) -> Vec<String> {
    let mut hints = Vec::new();

    match visibility {
        FunctionVisibility::Public => {
            hints.push("Public function - verify not used by external crates".to_string());
        }
        FunctionVisibility::Private => {
            hints.push("Private function - safe to remove if no local callers".to_string());
        }
        FunctionVisibility::Crate => {
            hints.push("Crate-visible function - check for usage within crate".to_string());
        }
    }

    let file_str = func.file.to_string_lossy();
    if file_str.contains("test") {
        hints.push("Test-related function - may be test helper".to_string());
    }

    if func.name.starts_with("test_") {
        hints.push("Test function - verify no test dependencies".to_string());
    }

    hints
}

/// Generate usage hints for dead code analysis
pub fn generate_usage_hints(
    func: &FunctionMetrics,
    call_graph: &CallGraph,
    func_id: &FunctionId,
) -> Vec<String> {
    let visibility = determine_visibility(func);
    let mut hints = generate_enhanced_dead_code_hints(func, &visibility);

    let callees = call_graph.get_callees(func_id);
    if callees.is_empty() {
        hints.push("Function has no dependencies - safe to remove".to_string());
    } else {
        hints.push(format!("Function calls {} other functions", callees.len()));
    }

    hints
}

/// Generate complexity-based recommendation for risk debt
pub fn generate_complexity_risk_recommendation(
    cyclo: u32,
    coverage: &Option<TransitiveCoverage>,
    factors: &[String],
) -> (String, String, Vec<String>) {
    let complexity_level = classify_complexity_level(cyclo);
    let has_good_coverage = coverage.as_ref().map(|c| c.direct >= 0.8).unwrap_or(false);
    let has_coverage_issue = factors
        .iter()
        .any(|f| f.contains("coverage") || f.contains("Coverage") || f.contains("uncovered"));

    match complexity_level {
        ComplexityLevel::Low => generate_low_complexity_recommendation(cyclo, has_coverage_issue),
        ComplexityLevel::LowModerate => {
            generate_low_moderate_complexity_recommendation(cyclo, has_good_coverage)
        }
        ComplexityLevel::Moderate => {
            generate_moderate_complexity_recommendation(cyclo, has_good_coverage)
        }
        ComplexityLevel::High => {
            generate_high_complexity_recommendation(cyclo, has_good_coverage, has_coverage_issue)
        }
    }
}

/// Generate recommendation for low complexity functions
fn generate_low_complexity_recommendation(
    cyclo: u32,
    has_coverage_issue: bool,
) -> (String, String, Vec<String>) {
    let action = if has_coverage_issue || cyclo > 3 {
        format!(
            "Extract helper functions for clarity, then add {} unit tests",
            cyclo.max(3)
        )
    } else {
        "Simplify function structure and improve testability".to_string()
    };

    (
        action,
        "Low complexity but flagged for improvement".to_string(),
        vec![
            "Extract helper functions for clarity".to_string(),
            "Remove unnecessary branching".to_string(),
            "Consolidate similar code paths".to_string(),
            format!(
                "Add {} unit tests for edge cases and main paths",
                cyclo.max(3)
            ),
        ],
    )
}

/// Generate recommendation for low-moderate complexity functions (5-6)
fn generate_low_moderate_complexity_recommendation(
    cyclo: u32,
    has_good_coverage: bool,
) -> (String, String, Vec<String>) {
    let functions_to_extract = 2;
    let target_complexity = 3;

    let action = if has_good_coverage {
        format!(
            "Extract {} pure functions (complexity {} → {})",
            functions_to_extract, cyclo, target_complexity
        )
    } else {
        format!(
            "Extract {} pure functions (complexity {} → {}) and add comprehensive tests",
            functions_to_extract, cyclo, target_complexity
        )
    };

    let mut steps = vec![
        format!(
            "Identify {} logical sections from {} branches:",
            functions_to_extract, cyclo
        ),
        format!(
            "  • Look for groups of ~{} related conditions",
            cyclo / functions_to_extract.max(1)
        ),
        format!(
            "  • Each extracted function should have complexity ≤{}",
            target_complexity
        ),
        "Extraction candidates:".to_string(),
        "  • Validation logic → validate_preconditions()".to_string(),
        "  • Main logic → process_core()".to_string(),
        "Move all I/O operations to a single orchestrator function".to_string(),
    ];

    if !has_good_coverage {
        steps.push(format!(
            "Write {} unit tests for the extracted pure functions",
            functions_to_extract * 3
        ));
        steps.push("Achieve 80%+ test coverage for all functions".to_string());
    } else {
        steps.push(format!(
            "Goal: Reduce cyclomatic complexity from {} to ≤{}",
            cyclo, target_complexity
        ));
    }

    (
        action,
        "Low-moderate complexity requiring refactoring".to_string(),
        steps,
    )
}

/// Generate recommendation for moderate complexity functions (7-10)
fn generate_moderate_complexity_recommendation(
    cyclo: u32,
    has_good_coverage: bool,
) -> (String, String, Vec<String>) {
    let functions_to_extract = (cyclo / 3).max(2);
    let target_complexity = 3;

    let action = if has_good_coverage {
        format!(
            "Extract {} pure functions (complexity {} → {})",
            functions_to_extract, cyclo, target_complexity
        )
    } else {
        format!(
            "Extract {} pure functions (complexity {} → {}) and add comprehensive tests",
            functions_to_extract, cyclo, target_complexity
        )
    };

    let mut steps = vec![
        format!(
            "Identify {} logical sections from {} branches:",
            functions_to_extract, cyclo
        ),
        format!(
            "  • Look for groups of ~{} related conditions",
            cyclo / functions_to_extract.max(1)
        ),
        format!(
            "  • Each extracted function should have complexity ≤{}",
            target_complexity
        ),
        "Extraction candidates:".to_string(),
        "  • Validation logic → validate_preconditions()".to_string(),
        "  • Complex calculations → calculate_specific()".to_string(),
        "  • Loop processing → process_items()".to_string(),
        "Move all I/O operations to a single orchestrator function".to_string(),
    ];

    if !has_good_coverage {
        steps.push(format!(
            "Write {} unit tests for the extracted pure functions",
            functions_to_extract * 3
        ));
        steps.push("Achieve 80%+ test coverage for all functions".to_string());
    } else {
        steps.push(format!(
            "Goal: Reduce cyclomatic complexity from {} to ≤{}",
            cyclo, target_complexity
        ));
    }

    (
        action,
        "Moderate complexity requiring refactoring".to_string(),
        steps,
    )
}

/// Generate recommendation for high complexity functions (11+)
fn generate_high_complexity_recommendation(
    cyclo: u32,
    has_good_coverage: bool,
    has_coverage_issue: bool,
) -> (String, String, Vec<String>) {
    let functions_to_extract = (cyclo / 4).max(3);
    let target_complexity = 5;

    let action = if has_good_coverage {
        format!(
            "Decompose into {} pure functions (complexity {} → {})",
            functions_to_extract, cyclo, target_complexity
        )
    } else if has_coverage_issue {
        format!(
            "Add {} tests, then decompose into {} pure functions (complexity {} → {})",
            cyclo, functions_to_extract, cyclo, target_complexity
        )
    } else {
        format!(
            "Decompose into {} pure functions (complexity {} → {}) with comprehensive tests",
            functions_to_extract, cyclo, target_complexity
        )
    };

    let mut steps = vec![
        format!(
            "This high-complexity function needs to be broken down into {} logical units:",
            functions_to_extract
        ),
        format!("1. Map {} execution paths into logical groupings:", cyclo),
        format!("  • ~{} paths for input validation", cyclo / 4),
        format!("  • ~{} paths for core logic", cyclo / 2),
        format!("  • ~{} paths for output/error handling", cyclo / 4),
    ];

    if has_coverage_issue && !has_good_coverage {
        steps.extend(vec![
            format!(
                "2. Add {} unit tests before refactoring to prevent regressions:",
                cyclo
            ),
            "  • Focus on happy path and major error conditions first".to_string(),
            "  • Cover all branches for confidence in refactoring".to_string(),
        ]);
    }

    steps.extend(vec![
        format!(
            "{}. Extract functions with single responsibilities:",
            if has_coverage_issue && !has_good_coverage {
                3
            } else {
                2
            }
        ),
        "  • validate_inputs() for precondition checks".to_string(),
        "  • process_core_logic() for main algorithm".to_string(),
        "  • handle_results() for output formatting".to_string(),
        "  • handle_errors() for error cases".to_string(),
        format!(
            "{}. Each function should have cyclomatic complexity ≤{}",
            if has_coverage_issue && !has_good_coverage {
                4
            } else {
                3
            },
            target_complexity
        ),
        format!(
            "{}. Add unit tests for each extracted function (~3-5 tests per function)",
            if has_coverage_issue && !has_good_coverage {
                5
            } else {
                4
            }
        ),
    ]);

    (
        action,
        "High complexity requiring decomposition".to_string(),
        steps,
    )
}

/// Generate recommendation for infrastructure debt types (duplication, risk)
pub fn generate_infrastructure_recommendation_with_coverage(
    debt_type: &DebtType,
    coverage: &Option<TransitiveCoverage>,
) -> (String, String, Vec<String>) {
    match debt_type {
        DebtType::Duplication {
            instances,
            total_lines,
        } => (
            "Extract common logic into shared module".to_string(),
            format!("Duplicated across {instances} locations ({total_lines} lines total)"),
            vec![
                "Create shared utility module".to_string(),
                "Replace duplicated code with calls to shared module".to_string(),
                "Add comprehensive tests to shared module".to_string(),
            ],
        ),
        DebtType::Risk {
            risk_score,
            factors,
        } => {
            let has_complexity_issue = factors.iter().any(|f| {
                f.contains("complexity") || f.contains("cyclomatic") || f.contains("cognitive")
            });

            if has_complexity_issue {
                let cyclo = extract_cyclomatic_from_factors(factors).unwrap_or(0);
                let (action, _, steps) =
                    generate_complexity_risk_recommendation(cyclo, coverage, factors);
                (
                    action,
                    format!("Risk score {:.1}: {}", risk_score, factors.join(", ")),
                    steps,
                )
            } else {
                (
                    "Address identified risk factors".to_string(),
                    format!("Risk score {:.1}: {}", risk_score, factors.join(", ")),
                    vec![
                        "Review and refactor problematic areas".to_string(),
                        "Add missing tests if coverage is low".to_string(),
                        "Update documentation".to_string(),
                    ],
                )
            }
        }
        DebtType::ComplexityHotspot {
            cyclomatic,
            cognitive,
        } => generate_complexity_hotspot_recommendation(*cyclomatic, *cognitive),
        _ => unreachable!("Not an infrastructure debt type"),
    }
}

/// Extract cyclomatic complexity value from factors strings
fn extract_cyclomatic_from_factors(factors: &[String]) -> Option<u32> {
    factors
        .iter()
        .find(|f| f.contains("cyclomatic"))
        .and_then(|f| {
            f.split(':')
                .nth(1)?
                .trim()
                .strip_suffix(')')?
                .parse::<u32>()
                .ok()
        })
}

/// Generate recommendation for complexity hotspots
pub fn generate_complexity_hotspot_recommendation(
    cyclomatic: u32,
    cognitive: u32,
) -> (String, String, Vec<String>) {
    use crate::priority::scoring::recommendation::calculate_functions_to_extract;

    let functions_to_extract = calculate_functions_to_extract(cyclomatic, cognitive);
    let target_per_function = (cyclomatic / functions_to_extract).max(3);
    (
        format!(
            "Extract {} pure functions, each handling ~{} branches (complexity {} → ~{})",
            functions_to_extract,
            cyclomatic / functions_to_extract.max(1),
            cyclomatic,
            target_per_function
        ),
        format!(
            "High complexity function (cyclo={}, cog={}) likely with low coverage - needs testing and refactoring",
            cyclomatic, cognitive
        ),
        vec![
            format!("Identify {} branch clusters from {} total branches:", functions_to_extract, cyclomatic),
            format!("  • Each cluster should handle ~{} related conditions", cyclomatic / functions_to_extract.max(1)),
            "Common extraction patterns:".to_string(),
            "  • Early validation checks → validate_preconditions()".to_string(),
            "  • Complex calculations in branches → calculate_[specific]()".to_string(),
            "  • Data processing in loops → process_[item_type]()".to_string(),
            "  • Error handling branches → handle_[error_case]()".to_string(),
            format!("Each extracted function should have cyclomatic complexity ≤{}", target_per_function),
            format!("Write ~{} tests per extracted function for full branch coverage", target_per_function),
            "Use property-based testing for complex logic validation".to_string(),
        ],
    )
}

/// Detect programming language from file path
fn detect_file_language(file_path: &std::path::Path) -> crate::core::Language {
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
fn pattern_type_name(
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

/// Generate coverage-focused recommendation when coverage is the primary issue
fn generate_coverage_focused_recommendation(
    func: &FunctionMetrics,
    cyclomatic: u32,
    cognitive: u32,
    cov: &TransitiveCoverage,
) -> (String, String, Vec<String>) {
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
        coverage_pct,
        uncovered_count,
        cyclomatic,
        cognitive,
        cyclomatic
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

/// Generate complexity recommendation using pattern analysis when available
pub fn generate_complexity_recommendation_with_patterns_and_coverage(
    func: &FunctionMetrics,
    cyclomatic: u32,
    cognitive: u32,
    coverage: &Option<TransitiveCoverage>,
    data_flow: Option<&crate::data_flow::DataFlowGraph>,
) -> (String, String, Vec<String>) {
    if should_prioritize_coverage(coverage) {
        return generate_coverage_focused_recommendation(
            func,
            cyclomatic,
            cognitive,
            coverage.as_ref().unwrap(),
        );
    }

    let suggestions = analyze_extraction_patterns(func, data_flow);

    if !suggestions.is_empty() {
        generate_pattern_based_recommendation(func, cyclomatic, &suggestions, coverage)
    } else {
        generate_heuristic_recommendations_with_line_estimates(
            func, cyclomatic, cognitive, coverage, data_flow,
        )
    }
}

fn should_prioritize_coverage(coverage: &Option<TransitiveCoverage>) -> bool {
    coverage
        .as_ref()
        .map(|cov| cov.direct < 0.8 && !cov.uncovered_lines.is_empty())
        .unwrap_or(false)
}

fn analyze_extraction_patterns(
    func: &FunctionMetrics,
    data_flow: Option<&crate::data_flow::DataFlowGraph>,
) -> Vec<crate::extraction_patterns::ExtractionSuggestion> {
    let analyzer = UnifiedExtractionAnalyzer::new();
    let file_metrics = create_minimal_file_metrics(func);
    analyzer.analyze_function(func, &file_metrics, data_flow)
}

fn create_minimal_file_metrics(func: &FunctionMetrics) -> crate::core::FileMetrics {
    crate::core::FileMetrics {
        path: func.file.clone(),
        language: detect_file_language(&func.file),
        complexity: crate::core::ComplexityMetrics::default(),
        debt_items: vec![],
        dependencies: vec![],
        duplications: vec![],
        module_scope: None,
        classes: None,
    }
}

fn generate_pattern_based_recommendation(
    func: &FunctionMetrics,
    cyclomatic: u32,
    suggestions: &[crate::extraction_patterns::ExtractionSuggestion],
    coverage: &Option<TransitiveCoverage>,
) -> (String, String, Vec<String>) {
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

fn process_suggestions(
    suggestions: &[&crate::extraction_patterns::ExtractionSuggestion],
) -> (Vec<String>, Vec<String>, u32) {
    suggestions.iter().enumerate().fold(
        (Vec::new(), Vec::new(), 0u32),
        |(mut actions, mut steps, mut total), (i, suggestion)| {
            actions.push(format!(
                "{} (confidence: {:.0}%)",
                suggestion.suggested_name,
                suggestion.confidence * 100.0
            ));

            steps.push(format!(
                "{}. Extract {} pattern at lines {}-{} as '{}' (complexity {} → {})",
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

fn explain_complexity(cyclomatic: u32) -> String {
    match cyclomatic {
        16.. => format!("Cyclomatic complexity of {} indicates {} independent execution paths, requiring at least {} test cases for full path coverage",
                cyclomatic, cyclomatic, cyclomatic),
        11..=15 => format!("Cyclomatic complexity of {} indicates {} independent paths through the code, making thorough testing difficult",
                cyclomatic, cyclomatic),
        6..=10 => format!("Cyclomatic complexity of {} indicates {} independent paths requiring {} test cases minimum - extraction will reduce this to 3-5 tests per function",
                cyclomatic, cyclomatic, cyclomatic),
        _ => format!("Cyclomatic complexity of {} indicates moderate complexity that can be improved through extraction", cyclomatic),
    }
}

fn explain_pattern_benefits(num_patterns: usize) -> String {
    match num_patterns {
        1 => "This extraction will create a focused, testable unit".to_string(),
        2 => "These extractions will separate distinct concerns into testable units".to_string(),
        _ => format!("These {} extractions will decompose the function into smaller, focused units that are easier to test and understand", num_patterns),
    }
}

fn has_good_coverage(coverage: &Option<TransitiveCoverage>) -> bool {
    coverage.as_ref().map(|c| c.direct >= 0.8).unwrap_or(false)
}

fn generate_coverage_steps(
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

fn calculate_reduction_percentage(reduction: u32, total: u32) -> u32 {
    if total > 0 {
        (reduction as f32 / total as f32 * 100.0) as u32
    } else {
        0
    }
}

/// Generate recommendations based on data flow analysis when AST is unavailable
pub fn generate_heuristic_recommendations_with_line_estimates(
    func: &FunctionMetrics,
    cyclomatic: u32,
    cognitive: u32,
    coverage: &Option<TransitiveCoverage>,
    data_flow: Option<&crate::data_flow::DataFlowGraph>,
) -> (String, String, Vec<String>) {
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

#[derive(Debug, Clone)]
struct FunctionCharacteristics {
    has_high_branching: bool,
    has_deep_nesting: bool,
    has_complex_cognition: bool,
    num_dependencies: usize,
    is_pure: bool,
    purity_confidence: f32,
}

fn analyze_function_characteristics(
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

fn extract_dependencies_count(
    func: &FunctionMetrics,
    data_flow: Option<&crate::data_flow::DataFlowGraph>,
) -> usize {
    data_flow
        .and_then(|df| {
            let func_id = crate::priority::call_graph::FunctionId::new(
                func.file.clone(),
                func.name.clone(),
                func.line,
            );
            df.get_variable_dependencies(&func_id).map(|d| d.len())
        })
        .unwrap_or(0)
}

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
            "Identify validation checks from {} branches → extract as validate_*()",
            branches_to_extract
        ));
        reduction += branches_to_extract;
    }

    if characteristics.has_deep_nesting {
        extractions.push("nested processing");
        steps.push(format!(
            "Extract nested logic (depth {}) → process_*() functions",
            nesting
        ));
        reduction += 2;
    }

    if characteristics.has_complex_cognition {
        extractions.push("complex calculations");
        let calc_complexity = cognitive / 5;
        steps.push(format!(
            "Extract calculations from {} cognitive complexity → calculate_*()",
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

fn generate_data_flow_recommendations(
    func: &FunctionMetrics,
    data_flow: Option<&crate::data_flow::DataFlowGraph>,
) -> Vec<String> {
    use crate::priority::call_graph::FunctionId;

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

fn generate_heuristic_coverage_steps(
    func: &FunctionMetrics,
    coverage: &Option<TransitiveCoverage>,
    extractions: &[&str],
    cyclomatic: u32,
) -> Vec<String> {
    let mut steps = Vec::new();
    let has_good_cov = coverage.as_ref().map(|c| c.direct >= 0.8).unwrap_or(false);

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

fn calculate_test_count(extractions: &[&str], cyclomatic: u32) -> u32 {
    if extractions.is_empty() {
        (cyclomatic / 2).max(3)
    } else {
        (extractions.len() as u32) * 4
    }
}

fn build_heuristic_action(
    extractions: &[&str],
    cyclomatic: u32,
    complexity_reduction: u32,
) -> String {
    let target_complexity = cyclomatic.saturating_sub(complexity_reduction);

    if extractions.is_empty() {
        format!(
            "Refactor to reduce complexity from {} → ~{}",
            cyclomatic, target_complexity
        )
    } else {
        format!(
            "Extract {} to reduce complexity {} → ~{}",
            extractions.join(", "),
            cyclomatic,
            target_complexity
        )
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn create_test_function(name: &str, visibility: Option<&str>) -> FunctionMetrics {
        FunctionMetrics {
            name: name.to_string(),
            file: PathBuf::from("test.rs"),
            line: 10,
            cyclomatic: 5,
            cognitive: 8,
            nesting: 2,
            length: 50,
            is_test: false,
            visibility: visibility.map(|v| v.to_string()),
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

    fn create_test_function_with_file(
        name: &str,
        file: &str,
        visibility: Option<&str>,
    ) -> FunctionMetrics {
        let mut func = create_test_function(name, visibility);
        func.file = PathBuf::from(file);
        func
    }

    #[test]
    fn test_generate_enhanced_dead_code_hints_public() {
        let func = create_test_function("my_function", Some("pub"));
        let visibility = FunctionVisibility::Public;

        let hints = generate_enhanced_dead_code_hints(&func, &visibility);

        assert!(!hints.is_empty());
        assert!(hints.contains(&"Public function - verify not used by external crates".to_string()));
    }

    #[test]
    fn test_generate_enhanced_dead_code_hints_private() {
        let func = create_test_function("my_function", None);
        let visibility = FunctionVisibility::Private;

        let hints = generate_enhanced_dead_code_hints(&func, &visibility);

        assert!(!hints.is_empty());
        assert!(
            hints.contains(&"Private function - safe to remove if no local callers".to_string())
        );
    }

    #[test]
    fn test_generate_enhanced_dead_code_hints_crate() {
        let func = create_test_function("my_function", Some("pub(crate)"));
        let visibility = FunctionVisibility::Crate;

        let hints = generate_enhanced_dead_code_hints(&func, &visibility);

        assert!(!hints.is_empty());
        assert!(
            hints.contains(&"Crate-visible function - check for usage within crate".to_string())
        );
    }

    #[test]
    fn test_generate_enhanced_dead_code_hints_test_file() {
        let func =
            create_test_function_with_file("helper_function", "tests/integration_test.rs", None);
        let visibility = FunctionVisibility::Private;

        let hints = generate_enhanced_dead_code_hints(&func, &visibility);

        assert!(hints.len() >= 2);
        assert!(
            hints.contains(&"Private function - safe to remove if no local callers".to_string())
        );
        assert!(hints.contains(&"Test-related function - may be test helper".to_string()));
    }

    #[test]
    fn test_generate_enhanced_dead_code_hints_test_function() {
        let func = create_test_function("test_something", None);
        let visibility = FunctionVisibility::Private;

        let hints = generate_enhanced_dead_code_hints(&func, &visibility);

        assert!(hints.len() >= 2);
        assert!(
            hints.contains(&"Private function - safe to remove if no local callers".to_string())
        );
        assert!(hints.contains(&"Test function - verify no test dependencies".to_string()));
    }

    #[test]
    fn test_generate_enhanced_dead_code_hints_test_file_and_test_function() {
        let func =
            create_test_function_with_file("test_helper", "src/tests/helpers.rs", Some("pub"));
        let visibility = FunctionVisibility::Public;

        let hints = generate_enhanced_dead_code_hints(&func, &visibility);

        assert!(hints.len() >= 3);
        assert!(hints.contains(&"Public function - verify not used by external crates".to_string()));
        assert!(hints.contains(&"Test-related function - may be test helper".to_string()));
        assert!(hints.contains(&"Test function - verify no test dependencies".to_string()));
    }

    #[test]
    fn test_classify_complexity_level() {
        assert!(matches!(classify_complexity_level(1), ComplexityLevel::Low));
        assert!(matches!(classify_complexity_level(4), ComplexityLevel::Low));
        assert!(matches!(
            classify_complexity_level(5),
            ComplexityLevel::LowModerate
        ));
        assert!(matches!(
            classify_complexity_level(6),
            ComplexityLevel::LowModerate
        ));
        assert!(matches!(
            classify_complexity_level(7),
            ComplexityLevel::Moderate
        ));
        assert!(matches!(
            classify_complexity_level(10),
            ComplexityLevel::Moderate
        ));
        assert!(matches!(
            classify_complexity_level(11),
            ComplexityLevel::High
        ));
        assert!(matches!(
            classify_complexity_level(20),
            ComplexityLevel::High
        ));
    }

    #[test]
    fn test_determine_visibility() {
        let public_func = create_test_function("test", Some("pub"));
        assert!(matches!(
            determine_visibility(&public_func),
            FunctionVisibility::Public
        ));

        let crate_func = create_test_function("test", Some("pub(crate)"));
        assert!(matches!(
            determine_visibility(&crate_func),
            FunctionVisibility::Crate
        ));

        let super_func = create_test_function("test", Some("pub(super)"));
        assert!(matches!(
            determine_visibility(&super_func),
            FunctionVisibility::Crate
        ));

        let private_func = create_test_function("test", None);
        assert!(matches!(
            determine_visibility(&private_func),
            FunctionVisibility::Private
        ));
    }

    #[test]
    fn test_generate_usage_hints_basic() {
        let func = create_test_function("unused_func", None);
        let call_graph = CallGraph::new();
        let func_id = FunctionId::new(PathBuf::from("test.rs"), "unused_func".to_string(), 10);

        let hints = generate_usage_hints(&func, &call_graph, &func_id);

        assert!(!hints.is_empty());
        assert!(hints.iter().any(|h| h.contains("Private function")));
    }

    #[test]
    fn test_generate_low_complexity_recommendation() {
        let (action, rationale, steps) = generate_low_complexity_recommendation(3, true);

        assert!(action.contains("unit tests"));
        assert!(rationale.contains("Low complexity"));
        assert!(!steps.is_empty());
        assert!(steps.iter().any(|s| s.contains("unit tests")));

        let (action2, rationale2, steps2) = generate_low_complexity_recommendation(3, false);

        assert!(action2.contains("Simplify function structure"));
        assert!(rationale2.contains("Low complexity"));
        assert!(!steps2.is_empty());
    }

    #[test]
    fn test_generate_low_moderate_complexity_recommendation() {
        let (action, rationale, steps) = generate_low_moderate_complexity_recommendation(5, true);

        assert!(action.contains("Extract"));
        assert!(action.contains("pure functions"));
        assert!(rationale.contains("Low-moderate complexity"));
        assert!(!steps.is_empty());

        let (action2, rationale2, steps2) =
            generate_low_moderate_complexity_recommendation(5, false);

        assert!(action2.contains("Extract"));
        assert!(action2.contains("comprehensive tests"));
        assert!(rationale2.contains("Low-moderate complexity"));
        assert!(steps2.iter().any(|s| s.contains("test")));
    }

    #[test]
    fn test_generate_moderate_complexity_recommendation() {
        let (action, rationale, steps) = generate_moderate_complexity_recommendation(9, true);

        assert!(action.contains("Extract"));
        assert!(action.contains("pure functions"));
        assert!(rationale.contains("Moderate complexity"));
        assert!(!steps.is_empty());
        assert!(steps.iter().any(|s| s.contains("logical sections")));

        let (action2, rationale2, steps2) = generate_moderate_complexity_recommendation(9, false);

        assert!(action2.contains("Extract"));
        assert!(action2.contains("comprehensive tests"));
        assert!(rationale2.contains("Moderate complexity"));
        assert!(steps2.iter().any(|s| s.contains("unit tests")));
    }

    #[test]
    fn test_generate_high_complexity_recommendation() {
        let (action, rationale, steps) = generate_high_complexity_recommendation(15, true, false);

        assert!(action.contains("Decompose"));
        assert!(action.contains("pure functions"));
        assert!(rationale.contains("High complexity"));
        assert!(!steps.is_empty());

        let (action2, rationale2, steps2) =
            generate_high_complexity_recommendation(15, false, true);

        assert!(action2.contains("Add"));
        assert!(action2.contains("tests"));
        assert!(action2.contains("decompose"));
        assert!(rationale2.contains("High complexity"));
        assert!(!steps2.is_empty());

        let (action3, rationale3, steps3) =
            generate_high_complexity_recommendation(15, false, false);

        assert!(action3.contains("Decompose"));
        assert!(action3.contains("comprehensive tests"));
        assert!(rationale3.contains("High complexity"));
        assert!(!steps3.is_empty());
    }

    #[test]
    fn test_generate_complexity_hotspot_recommendation() {
        let (action, rationale, steps) = generate_complexity_hotspot_recommendation(20, 30);

        assert!(action.contains("Extract"));
        assert!(action.contains("pure functions"));
        assert!(rationale.contains("High complexity"));
        assert!(rationale.contains("cyclo=20"));
        assert!(rationale.contains("cog=30"));
        assert!(!steps.is_empty());
        assert!(steps.iter().any(|s| s.contains("branch clusters")));
        assert!(steps.iter().any(|s| s.contains("property-based testing")));
    }

    #[test]
    fn test_detect_file_language() {
        use std::path::Path;

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
}
