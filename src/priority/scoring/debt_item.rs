// Functions for creating UnifiedDebtItem instances

use crate::analysis::PythonDeadCodeDetector;
use crate::core::FunctionMetrics;
use crate::priority::{
    call_graph::{CallGraph, FunctionId},
    external_api_detector::is_likely_external_api,
    scoring::recommendation_extended::{
        generate_assertion_complexity_recommendation, generate_async_misuse_recommendation,
        generate_collection_inefficiency_recommendation,
        generate_complexity_recommendation_with_patterns_and_coverage,
        generate_data_structure_recommendation, generate_feature_envy_recommendation,
        generate_flaky_test_recommendation, generate_god_object_recommendation,
        generate_infrastructure_recommendation_with_coverage, generate_magic_values_recommendation,
        generate_nested_loops_recommendation, generate_primitive_obsession_recommendation,
        generate_resource_leak_recommendation, generate_resource_management_recommendation,
        generate_string_concat_recommendation, generate_usage_hints,
    },
    scoring::rust_recommendations::generate_rust_refactoring_recommendation,
    semantic_classifier::classify_function_role,
    ActionableRecommendation, DebtType, FunctionRole, FunctionVisibility, ImpactMetrics, Location,
    TransitiveCoverage, UnifiedDebtItem, UnifiedScore,
};
use std::collections::HashSet;

// Re-export construction functions for backward compatibility
pub use super::construction::{
    create_unified_debt_item, create_unified_debt_item_enhanced,
    create_unified_debt_item_with_aggregator,
    create_unified_debt_item_with_aggregator_and_data_flow,
    create_unified_debt_item_with_data_flow, create_unified_debt_item_with_exclusions,
    create_unified_debt_item_with_exclusions_and_data_flow,
};

// Re-export computation functions for backward compatibility
pub(super) use super::computation::{
    calculate_entropy_details, calculate_expected_impact, calculate_functions_to_extract,
    calculate_needed_test_cases, calculate_risk_score, calculate_simple_test_cases,
};

// Import computation functions for tests
#[cfg(test)]
use super::computation::{
    calculate_coverage_improvement, calculate_lines_reduction, calculate_risk_factor,
    is_function_complex,
};

// Re-export validation functions for backward compatibility
pub(super) use super::validation::{
    check_complexity_hotspot, check_dead_code, check_enhanced_complexity_hotspot,
    check_enhanced_dead_code, check_enhanced_testing_gap, check_testing_gap, ClassificationContext,
};

// Re-export formatting and helper functions for backward compatibility
pub use super::formatting::determine_visibility;
pub(super) use super::formatting::{
    add_uncovered_lines_to_steps, format_complexity_display, format_role_description,
    generate_combined_testing_refactoring_steps, generate_dead_code_steps,
    generate_testing_gap_steps, get_role_display_name, is_excluded_from_dead_code_analysis,
    is_rust_file,
};

// Helper functions

pub(super) fn determine_debt_type(
    func: &FunctionMetrics,
    coverage: &Option<TransitiveCoverage>,
    call_graph: &CallGraph,
    func_id: &FunctionId,
) -> DebtType {
    // Use functional composition to determine debt type
    if let Some(testing_gap) = check_testing_gap(func, coverage) {
        return testing_gap;
    }

    if let Some(complexity_debt) = check_complexity_hotspot(func) {
        return complexity_debt;
    }

    if let Some(dead_code_debt) = check_dead_code(func, call_graph, func_id) {
        return dead_code_debt;
    }

    // Classify remaining functions based on role and complexity
    let role = classify_function_role(func, func_id, call_graph);
    classify_remaining_debt(func, coverage, &role)
}

/// Pure function to classify remaining debt based on role and complexity
fn classify_remaining_debt(
    func: &FunctionMetrics,
    coverage: &Option<TransitiveCoverage>,
    role: &FunctionRole,
) -> DebtType {
    // Check for simple acceptable patterns first
    if let Some(simple_debt) = classify_simple_acceptable_patterns(func, role) {
        return simple_debt;
    }

    // Classify based on complexity indicators
    if func.cyclomatic > 5 || func.cognitive > 8 || func.length > 50 {
        DebtType::Risk {
            risk_score: calculate_risk_score(func),
            factors: identify_risk_factors(func, coverage),
        }
    } else {
        classify_simple_function_debt(role)
    }
}

/// Pure function to classify simple acceptable patterns
fn classify_simple_acceptable_patterns(
    func: &FunctionMetrics,
    role: &FunctionRole,
) -> Option<DebtType> {
    if func.cyclomatic <= 3 && func.cognitive <= 5 {
        match role {
            FunctionRole::IOWrapper | FunctionRole::EntryPoint | FunctionRole::PatternMatch => {
                Some(DebtType::Risk {
                    risk_score: 0.0,
                    factors: vec!["Simple I/O wrapper or entry point - minimal risk".to_string()],
                })
            }
            FunctionRole::PureLogic if func.length <= 10 => Some(DebtType::Risk {
                risk_score: 0.0,
                factors: vec!["Trivial pure function - not technical debt".to_string()],
            }),
            _ => None,
        }
    } else {
        None
    }
}

/// Pure function to classify simple function debt
fn classify_simple_function_debt(role: &FunctionRole) -> DebtType {
    match role {
        FunctionRole::PureLogic => DebtType::Risk {
            risk_score: 0.0,
            factors: vec!["Simple pure function - minimal risk".to_string()],
        },
        _ => DebtType::Risk {
            risk_score: 0.1,
            factors: vec!["Simple function with low complexity".to_string()],
        },
    }
}

fn identify_risk_factors(
    func: &FunctionMetrics,
    coverage: &Option<TransitiveCoverage>,
) -> Vec<String> {
    let mut factors = Vec::new();

    if func.cyclomatic > 5 {
        factors.push(format!(
            "Moderate complexity (cyclomatic: {})",
            func.cyclomatic
        ));
    }

    if func.cognitive > 8 {
        factors.push(format!("Cognitive complexity: {}", func.cognitive));
    }

    if func.length > 50 {
        factors.push(format!("Long function ({} lines)", func.length));
    }

    if let Some(cov) = coverage {
        if cov.direct < 0.5 {
            factors.push(format!("Low coverage: {:.0}%", cov.direct * 100.0));
        }
    }

    if factors.is_empty() {
        factors.push("Potential improvement opportunity".to_string());
    }

    factors
}

pub fn is_dead_code(
    func: &FunctionMetrics,
    call_graph: &CallGraph,
    func_id: &FunctionId,
    function_pointer_used_functions: Option<&HashSet<FunctionId>>,
) -> bool {
    // FIRST: Check if function has incoming calls in the call graph
    // This includes event handlers bound via Bind() and other framework patterns
    let callers = call_graph.get_callers(func_id);
    if !callers.is_empty() {
        return false;
    }

    // Check if function is definitely used through function pointers
    if let Some(fp_used) = function_pointer_used_functions {
        if fp_used.contains(func_id) {
            return false;
        }
    }

    // For Python files, use the PythonDeadCodeDetector for additional pattern-based detection
    // This handles magic methods, framework patterns, etc. that might not show up in call graph
    let language = crate::core::Language::from_path(&func.file);
    if language == crate::core::Language::Python {
        let detector = PythonDeadCodeDetector::new();
        // Use the enhanced detection that considers both call graph and implicit calls
        if let Some((is_dead, _confidence)) =
            detector.is_dead_code_with_confidence(func, call_graph, func_id)
        {
            return is_dead;
        }
    }

    // LAST: Check hardcoded exclusions (includes test functions, main, etc.)
    // This is now a fallback for functions with no callers but might be implicitly called
    if is_excluded_from_dead_code_analysis(func) {
        return false;
    }

    // No callers found and not excluded by patterns
    true
}

/// Enhanced dead code detection that uses framework pattern exclusions
pub fn is_dead_code_with_exclusions(
    func: &FunctionMetrics,
    call_graph: &CallGraph,
    func_id: &FunctionId,
    framework_exclusions: &std::collections::HashSet<FunctionId>,
    function_pointer_used_functions: Option<&HashSet<FunctionId>>,
) -> bool {
    // Check if dead code detection is enabled for this file's language
    let language = crate::core::Language::from_path(&func.file);
    let language_features = crate::config::get_language_features(&language);

    if !language_features.detect_dead_code {
        // Dead code detection disabled for this language
        return false;
    }

    // First check if this function is excluded by framework patterns
    if framework_exclusions.contains(func_id) {
        return false;
    }

    // Use the enhanced dead code detection with function pointer information
    is_dead_code(func, call_graph, func_id, function_pointer_used_functions)
}

/// Enhanced version of debt type classification with framework pattern exclusions
pub fn classify_debt_type_with_exclusions(
    func: &FunctionMetrics,
    call_graph: &CallGraph,
    func_id: &FunctionId,
    framework_exclusions: &HashSet<FunctionId>,
    function_pointer_used_functions: Option<&HashSet<FunctionId>>,
    coverage: Option<&TransitiveCoverage>,
) -> DebtType {
    // Create classification context
    let context = ClassificationContext {
        func,
        call_graph,
        func_id,
        framework_exclusions,
        function_pointer_used_functions,
        coverage,
    };

    // Use functional pipeline for classification
    classify_debt_with_context(&context)
}

/// Pure function to classify debt using context
fn classify_debt_with_context(context: &ClassificationContext) -> DebtType {
    if context.func.is_test {
        return classify_test_debt(context.func);
    }

    // Check each debt type in priority order
    if let Some(debt) = check_enhanced_testing_gap(context) {
        return debt;
    }

    if let Some(debt) = check_enhanced_complexity_hotspot(context.func) {
        return debt;
    }

    if let Some(debt) = check_enhanced_dead_code(context) {
        return debt;
    }

    // Classify remaining based on function characteristics
    classify_remaining_enhanced_debt(context)
}

/// Pure function to classify remaining enhanced debt
fn classify_remaining_enhanced_debt(context: &ClassificationContext) -> DebtType {
    let role = classify_function_role(context.func, context.func_id, context.call_graph);

    if context.func.cyclomatic <= 3 && context.func.cognitive <= 5 {
        if let Some(debt) = classify_simple_function_risk(context.func, &role) {
            return debt;
        }
    }

    DebtType::Risk {
        risk_score: 0.0,
        factors: vec!["Well-designed simple function - not technical debt".to_string()],
    }
}

/// Classify test function debt type based on complexity
fn classify_test_debt(func: &FunctionMetrics) -> DebtType {
    match () {
        _ if func.cyclomatic > 15 || func.cognitive > 20 => DebtType::TestComplexityHotspot {
            cyclomatic: func.cyclomatic,
            cognitive: func.cognitive,
            threshold: 15,
        },
        _ => DebtType::TestingGap {
            coverage: 0.0, // Test functions don't have coverage themselves
            cyclomatic: func.cyclomatic,
            cognitive: func.cognitive,
        },
    }
}

/// Check if function is a complexity hotspot based on role and metrics
fn is_complexity_hotspot(func: &FunctionMetrics, role: &FunctionRole) -> Option<DebtType> {
    // Direct complexity check
    if func.cyclomatic > 10 || func.cognitive > 15 {
        return Some(DebtType::ComplexityHotspot {
            cyclomatic: func.cyclomatic,
            cognitive: func.cognitive,
        });
    }

    // Orchestrator-specific complexity check
    if *role == FunctionRole::Orchestrator && func.cyclomatic > 5 {
        return Some(DebtType::ComplexityHotspot {
            cyclomatic: func.cyclomatic,
            cognitive: func.cognitive,
        });
    }

    None
}

/// Classify simple function risk based on role and metrics
fn classify_simple_function_risk(func: &FunctionMetrics, role: &FunctionRole) -> Option<DebtType> {
    // Check if it's a very simple function
    if func.cyclomatic <= 3 && func.cognitive <= 5 {
        match role {
            FunctionRole::IOWrapper | FunctionRole::EntryPoint | FunctionRole::PatternMatch => {
                return Some(DebtType::Risk {
                    risk_score: 0.0,
                    factors: vec!["Simple I/O wrapper or entry point - minimal risk".to_string()],
                });
            }
            FunctionRole::PureLogic if func.length <= 10 => {
                return Some(DebtType::Risk {
                    risk_score: 0.0,
                    factors: vec!["Trivial pure function - not technical debt".to_string()],
                });
            }
            _ => {}
        }
    }
    None
}

/// Classify risk-based debt for moderate complexity functions
fn classify_risk_based_debt(func: &FunctionMetrics, role: &FunctionRole) -> DebtType {
    if func.cyclomatic > 5 || func.cognitive > 8 || func.length > 50 {
        DebtType::Risk {
            risk_score: calculate_risk_score(func),
            factors: identify_risk_factors(func, &None),
        }
    } else {
        match role {
            FunctionRole::PureLogic => DebtType::Risk {
                risk_score: 0.0,
                factors: vec!["Simple pure function - minimal risk".to_string()],
            },
            _ => DebtType::Risk {
                risk_score: 0.1,
                factors: vec!["Simple function with low complexity".to_string()],
            },
        }
    }
}

/// Enhanced version of debt type classification (legacy - kept for compatibility)
pub fn classify_debt_type_enhanced(
    func: &FunctionMetrics,
    call_graph: &CallGraph,
    func_id: &FunctionId,
) -> DebtType {
    // Test functions are special debt cases
    if func.is_test {
        return classify_test_debt(func);
    }

    let role = classify_function_role(func, func_id, call_graph);

    // Check for complexity hotspots
    if let Some(debt) = is_complexity_hotspot(func, &role) {
        return debt;
    }

    // Check for dead code
    if is_dead_code(func, call_graph, func_id, None) {
        return DebtType::DeadCode {
            visibility: determine_visibility(func),
            cyclomatic: func.cyclomatic,
            cognitive: func.cognitive,
            usage_hints: generate_usage_hints(func, call_graph, func_id),
        };
    }

    // Check for simple functions that aren't debt
    if let Some(debt) = classify_simple_function_risk(func, &role) {
        return debt;
    }

    // Default to risk-based classification
    classify_risk_based_debt(func, &role)
}

/// Generate action and rationale for dead code
fn generate_dead_code_action(
    func: &FunctionMetrics,
    visibility: &FunctionVisibility,
    func_name: &str,
    cyclomatic: &u32,
    cognitive: &u32,
) -> (String, String) {
    let complexity_str = format_complexity_display(cyclomatic, cognitive);

    match visibility {
        FunctionVisibility::Private => (
            "Remove unused private function".to_string(),
            format!("Private function '{func_name}' has no callers and can be safely removed (complexity: {complexity_str})"),
        ),
        FunctionVisibility::Crate => (
            "Remove or document unused crate function".to_string(),
            format!("Crate-public function '{func_name}' has no internal callers (complexity: {complexity_str})"),
        ),
        FunctionVisibility::Public => {
            let (is_likely_api, _) = is_likely_external_api(func, visibility);
            if is_likely_api {
                (
                    "Verify external usage before removal or deprecation".to_string(),
                    format!("Public function '{func_name}' appears to be external API - verify usage before action (complexity: {complexity_str})"),
                )
            } else {
                (
                    "Remove unused public function (no API indicators)".to_string(),
                    format!("Public function '{func_name}' has no callers and no external API indicators (complexity: {complexity_str})"),
                )
            }
        }
    }
}

/// Generate recommendation when function is fully covered
fn generate_full_coverage_recommendation(role: FunctionRole) -> (String, String, Vec<String>) {
    let role_display = get_role_display_name(role);
    (
        "Maintain test coverage".to_string(),
        format!("{} function is currently 100% covered", role_display),
        vec![
            "Keep tests up to date with code changes".to_string(),
            "Consider property-based testing for edge cases".to_string(),
            "Monitor coverage in CI/CD pipeline".to_string(),
        ],
    )
}

/// Generate recommendation for complex functions with testing gaps
fn generate_complex_function_recommendation(
    cyclomatic: u32,
    cognitive: u32,
    coverage_pct: f64,
    coverage_gap: i32,
    role_str: &str,
    func: &FunctionMetrics,
    transitive_coverage: &Option<TransitiveCoverage>,
) -> (String, String, Vec<String>) {
    let functions_to_extract = calculate_functions_to_extract(cyclomatic, cognitive);
    let needed_test_cases = calculate_needed_test_cases(cyclomatic, coverage_pct);
    let coverage_pct_int = (coverage_pct * 100.0) as i32;

    let complexity_explanation = format!(
        "Cyclomatic complexity of {} requires at least {} test cases for full path coverage. After extracting {} functions, each will need only 3-5 tests",
        cyclomatic, cyclomatic, functions_to_extract
    );

    let mut steps =
        generate_combined_testing_refactoring_steps(cyclomatic, cognitive, coverage_pct_int);
    add_uncovered_lines_to_steps(&mut steps, func, transitive_coverage);

    (
        format!("Add {} tests for {}% coverage gap, then refactor complexity {} into {} functions",
               needed_test_cases, coverage_gap, cyclomatic, functions_to_extract),
        format!("Complex {role_str} with {coverage_gap}% gap. {}. Testing before refactoring ensures no regressions",
               complexity_explanation),
        steps,
    )
}

/// Generate recommendation for simple functions with testing gaps
fn generate_simple_function_recommendation(
    cyclomatic: u32,
    coverage_pct: f64,
    coverage_gap: i32,
    role: FunctionRole,
    func: &FunctionMetrics,
    transitive_coverage: &Option<TransitiveCoverage>,
) -> (String, String, Vec<String>) {
    let role_display = get_role_display_name(role);
    let test_cases_needed = calculate_simple_test_cases(cyclomatic, coverage_pct);
    let coverage_pct_int = (coverage_pct * 100.0) as i32;

    let coverage_explanation = if coverage_pct_int == 0 {
        format!("{role_display} with {coverage_gap}% coverage gap, currently {coverage_pct_int}% covered. Needs {} test cases to cover all {} execution paths",
               test_cases_needed, cyclomatic.max(2))
    } else {
        format!("{role_display} with {coverage_gap}% coverage gap, currently {coverage_pct_int}% covered. Needs {} more test cases",
               test_cases_needed)
    };

    let mut steps = generate_testing_gap_steps(false);
    add_uncovered_lines_to_steps(&mut steps, func, transitive_coverage);

    (
        format!(
            "Add {} tests for {}% coverage gap",
            test_cases_needed, coverage_gap
        ),
        coverage_explanation,
        steps,
    )
}

fn generate_testing_gap_recommendation(
    coverage_pct: f64,
    cyclomatic: u32,
    cognitive: u32,
    role: FunctionRole,
    func: &FunctionMetrics,
    transitive_coverage: &Option<TransitiveCoverage>,
) -> (String, String, Vec<String>) {
    let coverage_gap = 100 - (coverage_pct * 100.0) as i32;

    // If function is fully covered, no testing gap exists
    if coverage_gap == 0 {
        return generate_full_coverage_recommendation(role);
    }

    let is_complex = cyclomatic > 10 || cognitive > 15;

    if is_complex {
        let role_str = format_role_description(role);
        generate_complex_function_recommendation(
            cyclomatic,
            cognitive,
            coverage_pct,
            coverage_gap,
            role_str,
            func,
            transitive_coverage,
        )
    } else {
        generate_simple_function_recommendation(
            cyclomatic,
            coverage_pct,
            coverage_gap,
            role,
            func,
            transitive_coverage,
        )
    }
}

/// Generate recommendation for dead code debt type
fn generate_dead_code_recommendation(
    func: &FunctionMetrics,
    visibility: &FunctionVisibility,
    usage_hints: &[String],
    cyclomatic: u32,
    cognitive: u32,
) -> (String, String, Vec<String>) {
    let (action, rationale) =
        generate_dead_code_action(func, visibility, &func.name, &cyclomatic, &cognitive);
    let mut steps = generate_dead_code_steps(visibility);

    // Add usage hints to the steps
    for hint in usage_hints {
        steps.push(format!("Note: {hint}"));
    }

    (action, rationale, steps)
}

/// Generate recommendation for error swallowing debt
fn generate_error_swallowing_recommendation(
    pattern: &str,
    context: &Option<String>,
) -> (String, String, Vec<String>) {
    let primary_action = format!("Fix error swallowing: {}", pattern);

    let rationale = match context {
        Some(ctx) => format!("Error being silently ignored using '{}' pattern. Context: {}", pattern, ctx),
        None => format!("Error being silently ignored using '{}' pattern. This can hide critical failures in production", pattern),
    };

    let steps = vec![
        "Replace error swallowing with proper error handling".to_string(),
        "Log errors at minimum, even if they can't be handled".to_string(),
        "Consider propagating errors to caller with ?".to_string(),
        "Add context to errors using .context() or .with_context()".to_string(),
        "Test error paths explicitly".to_string(),
    ];

    (primary_action, rationale, steps)
}

/// Generate recommendation for test-specific debt types
fn generate_test_debt_recommendation(debt_type: &DebtType) -> (String, String, Vec<String>) {
    match debt_type {
        DebtType::TestComplexityHotspot {
            cyclomatic,
            cognitive,
            threshold
        } => (
            format!("Simplify test - complexity {} exceeds test threshold {}", cyclomatic.max(cognitive), threshold),
            format!("Test has high complexity (cyclo={cyclomatic}, cognitive={cognitive}) - consider splitting into smaller tests"),
            vec![
                "Break complex test into multiple smaller tests".to_string(),
                "Extract test setup into helper functions".to_string(),
                "Use parameterized tests for similar test cases".to_string(),
            ],
        ),
        DebtType::TestTodo { priority: _, reason } => (
            "Complete test TODO".to_string(),
            format!("Test contains TODO: {}", reason.as_ref().unwrap_or(&"No reason specified".to_string())),
            vec![
                "Address the TODO comment".to_string(),
                "Implement missing test logic".to_string(),
                "Remove TODO once completed".to_string(),
            ],
        ),
        DebtType::TestDuplication { instances, total_lines, similarity: _ } => (
            format!("Remove test duplication - {instances} similar test blocks"),
            format!("{instances} duplicated test blocks found across {total_lines} lines"),
            vec![
                "Extract common test logic into helper functions".to_string(),
                "Create parameterized tests for similar test cases".to_string(),
                "Use test fixtures for shared setup".to_string(),
            ],
        ),
        _ => unreachable!("Not a test debt type"),
    }
}

pub(super) fn generate_recommendation(
    func: &FunctionMetrics,
    debt_type: &DebtType,
    role: FunctionRole,
    _score: &UnifiedScore,
) -> ActionableRecommendation {
    generate_recommendation_with_data_flow(func, debt_type, role, _score, None)
}

fn generate_recommendation_with_data_flow(
    func: &FunctionMetrics,
    debt_type: &DebtType,
    role: FunctionRole,
    _score: &UnifiedScore,
    data_flow: Option<&crate::data_flow::DataFlowGraph>,
) -> ActionableRecommendation {
    generate_recommendation_with_coverage_and_data_flow(
        func, debt_type, role, _score, &None, data_flow,
    )
}

pub(super) fn generate_recommendation_with_coverage_and_data_flow(
    func: &FunctionMetrics,
    debt_type: &DebtType,
    role: FunctionRole,
    _score: &UnifiedScore,
    coverage: &Option<TransitiveCoverage>,
    data_flow: Option<&crate::data_flow::DataFlowGraph>,
) -> ActionableRecommendation {
    // Determine if we have actual coverage data (not just a None value)
    let has_coverage_data = coverage.is_some();

    // Create recommendation context using pure functions
    let recommendation_context =
        create_recommendation_context(func, debt_type, role, _score, coverage, has_coverage_data);

    // Generate recommendation using functional composition
    let (primary_action, rationale, steps) =
        generate_context_aware_recommendation(recommendation_context, data_flow);

    build_actionable_recommendation(primary_action, rationale, steps)
}

// Pure function to create recommendation context
fn create_recommendation_context(
    func: &FunctionMetrics,
    debt_type: &DebtType,
    role: FunctionRole,
    score: &UnifiedScore,
    coverage: &Option<TransitiveCoverage>,
    has_coverage_data: bool,
) -> RecommendationContext {
    RecommendationContext {
        function_info: FunctionInfo::from_metrics(func),
        debt_type: debt_type.clone(),
        role,
        score: score.clone(),
        coverage: coverage.clone(),
        is_rust_file: is_rust_file(&func.file),
        coverage_percent: extract_coverage_percent(coverage),
        has_coverage_data,
    }
}

// Data structure to hold recommendation context
struct RecommendationContext {
    function_info: FunctionInfo,
    debt_type: DebtType,
    role: FunctionRole,
    score: UnifiedScore,
    coverage: Option<TransitiveCoverage>,
    is_rust_file: bool,
    coverage_percent: f64,
    has_coverage_data: bool,
}

// Pure data structure for function information
struct FunctionInfo {
    file: std::path::PathBuf,
    name: String,
    line: usize,
    nesting: u32,
    length: usize,
    cognitive: u32,
    is_pure: bool,
    purity_confidence: f32,
}

impl FunctionInfo {
    fn from_metrics(func: &FunctionMetrics) -> Self {
        Self {
            file: func.file.clone(),
            name: func.name.clone(),
            line: func.line,
            nesting: func.nesting,
            length: func.length,
            cognitive: func.cognitive,
            is_pure: func.is_pure.unwrap_or(false),
            purity_confidence: func.purity_confidence.unwrap_or(0.0),
        }
    }
}

// Pure function to extract coverage percentage
fn extract_coverage_percent(coverage: &Option<TransitiveCoverage>) -> f64 {
    coverage.as_ref().map(|c| c.direct).unwrap_or(0.0)
}

// Pure function to generate context-aware recommendations
fn generate_context_aware_recommendation(
    context: RecommendationContext,
    data_flow: Option<&crate::data_flow::DataFlowGraph>,
) -> (String, String, Vec<String>) {
    match should_use_rust_specific_recommendation(&context) {
        Some(complexity) => generate_rust_complexity_recommendation(&context, complexity),
        None => generate_standard_recommendation_from_context(context, data_flow),
    }
}

// Pure function to determine if Rust-specific recommendation should be used
fn should_use_rust_specific_recommendation(context: &RecommendationContext) -> Option<u32> {
    if context.is_rust_file {
        if let DebtType::ComplexityHotspot { cyclomatic, .. } = &context.debt_type {
            return Some(*cyclomatic);
        }
    }
    None
}

// Pure function to generate Rust complexity recommendation
fn generate_rust_complexity_recommendation(
    context: &RecommendationContext,
    cyclomatic: u32,
) -> (String, String, Vec<String>) {
    let temp_item = create_temporary_debt_item(context);
    generate_rust_refactoring_recommendation(
        &temp_item,
        cyclomatic,
        context.coverage_percent,
        context.has_coverage_data,
    )
}

// Pure function to create temporary debt item for Rust recommendations
fn create_temporary_debt_item(context: &RecommendationContext) -> UnifiedDebtItem {
    UnifiedDebtItem {
        location: Location {
            file: context.function_info.file.clone(),
            function: context.function_info.name.clone(),
            line: context.function_info.line,
        },
        debt_type: context.debt_type.clone(),
        unified_score: context.score.clone(),
        function_role: context.role,
        recommendation: ActionableRecommendation {
            primary_action: String::new(),
            rationale: String::new(),
            implementation_steps: vec![],
            related_items: vec![],
        },
        expected_impact: ImpactMetrics {
            risk_reduction: 0.0,
            complexity_reduction: 0.0,
            coverage_improvement: 0.0,
            lines_reduction: 0,
        },
        transitive_coverage: context.coverage.clone(),
        upstream_dependencies: 0,
        downstream_dependencies: 0,
        upstream_callers: vec![],
        downstream_callees: vec![],
        nesting_depth: context.function_info.nesting,
        function_length: context.function_info.length,
        cyclomatic_complexity: extract_cyclomatic_complexity(&context.debt_type),
        cognitive_complexity: context.function_info.cognitive,
        entropy_details: None,
        is_pure: Some(context.function_info.is_pure),
        purity_confidence: Some(context.function_info.purity_confidence),
        god_object_indicators: None,
        tier: None,
    }
}

// Pure function to extract cyclomatic complexity from debt type
fn extract_cyclomatic_complexity(debt_type: &DebtType) -> u32 {
    match debt_type {
        DebtType::ComplexityHotspot { cyclomatic, .. } => *cyclomatic,
        _ => 0,
    }
}

// Function to generate standard recommendation from context
fn generate_standard_recommendation_from_context(
    context: RecommendationContext,
    data_flow: Option<&crate::data_flow::DataFlowGraph>,
) -> (String, String, Vec<String>) {
    // Convert context back to original parameters for compatibility
    let func = reconstruct_function_metrics(&context);
    generate_standard_recommendation(
        &func,
        &context.debt_type,
        context.role,
        &context.coverage,
        data_flow,
    )
}

// Helper function to reconstruct function metrics from context
fn reconstruct_function_metrics(context: &RecommendationContext) -> FunctionMetrics {
    FunctionMetrics {
        file: context.function_info.file.clone(),
        name: context.function_info.name.clone(),
        line: context.function_info.line,
        nesting: context.function_info.nesting,
        length: context.function_info.length,
        cognitive: context.function_info.cognitive,
        is_pure: Some(context.function_info.is_pure),
        purity_confidence: Some(context.function_info.purity_confidence),
        // Set reasonable defaults for other fields
        cyclomatic: extract_cyclomatic_complexity(&context.debt_type),
        is_test: false,
        visibility: None,
        is_trait_method: false,
        in_test_module: false,
        entropy_score: None,
        detected_patterns: None,
        upstream_callers: None,
        downstream_callees: None,
    }
}

// Pure function to build final actionable recommendation
fn build_actionable_recommendation(
    primary_action: String,
    rationale: String,
    steps: Vec<String>,
) -> ActionableRecommendation {
    ActionableRecommendation {
        primary_action,
        rationale,
        implementation_steps: steps,
        related_items: vec![],
    }
}

fn generate_standard_recommendation(
    func: &FunctionMetrics,
    debt_type: &DebtType,
    role: FunctionRole,
    coverage: &Option<TransitiveCoverage>,
    data_flow: Option<&crate::data_flow::DataFlowGraph>,
) -> (String, String, Vec<String>) {
    match debt_type {
        DebtType::DeadCode {
            visibility,
            usage_hints,
            cyclomatic,
            cognitive,
        } => generate_dead_code_recommendation(
            func,
            visibility,
            usage_hints,
            *cyclomatic,
            *cognitive,
        ),
        DebtType::TestingGap {
            coverage: coverage_val,
            cyclomatic,
            cognitive,
        } => generate_testing_gap_recommendation(
            *coverage_val,
            *cyclomatic,
            *cognitive,
            role,
            func,
            coverage,
        ),
        DebtType::ComplexityHotspot {
            cyclomatic,
            cognitive,
        } => {
            // Always try to use intelligent pattern-based recommendations
            // The DataFlowGraph is passed through but may still be None in some cases
            generate_complexity_recommendation_with_patterns_and_coverage(
                func,
                *cyclomatic,
                *cognitive,
                coverage,
                data_flow,
            )
        }
        DebtType::Duplication { .. } | DebtType::Risk { .. } => {
            generate_infrastructure_recommendation_with_coverage(debt_type, coverage)
        }
        DebtType::TestComplexityHotspot { .. }
        | DebtType::TestTodo { .. }
        | DebtType::TestDuplication { .. } => generate_test_debt_recommendation(debt_type),
        DebtType::ErrorSwallowing { pattern, context } => {
            generate_error_swallowing_recommendation(pattern, context)
        }
        // Security debt types
        // Resource Management debt types
        DebtType::AllocationInefficiency { pattern, impact } => {
            generate_resource_management_recommendation("allocation", pattern, impact)
        }
        DebtType::StringConcatenation {
            loop_type,
            iterations,
        } => generate_string_concat_recommendation(loop_type, iterations),
        DebtType::NestedLoops {
            depth,
            complexity_estimate,
        } => generate_nested_loops_recommendation(*depth, complexity_estimate),
        DebtType::BlockingIO { operation, context } => {
            generate_resource_management_recommendation("blocking_io", operation, context)
        }
        DebtType::SuboptimalDataStructure {
            current_type,
            recommended_type,
        } => generate_data_structure_recommendation(current_type, recommended_type),
        // Organization debt types
        DebtType::GodObject {
            responsibilities,
            god_object_score,
            ..
        } => generate_god_object_recommendation(*responsibilities, *god_object_score),
        DebtType::GodModule {
            functions,
            responsibilities,
            ..
        } => (
            "Split module into focused submodules".to_string(),
            format!(
                "Module with {} functions across {} responsibilities",
                functions, responsibilities
            ),
            vec![
                "Identify distinct responsibilities".to_string(),
                "Create separate modules for each responsibility".to_string(),
                "Move functions to appropriate modules".to_string(),
            ],
        ),
        DebtType::FeatureEnvy {
            external_class,
            usage_ratio,
        } => generate_feature_envy_recommendation(external_class, *usage_ratio),
        DebtType::PrimitiveObsession {
            primitive_type,
            domain_concept,
        } => generate_primitive_obsession_recommendation(primitive_type, domain_concept),
        DebtType::MagicValues { value, occurrences } => {
            generate_magic_values_recommendation(value, *occurrences)
        }
        // Testing quality debt types
        DebtType::AssertionComplexity {
            assertion_count,
            complexity_score,
        } => generate_assertion_complexity_recommendation(*assertion_count, *complexity_score),
        DebtType::FlakyTestPattern {
            pattern_type,
            reliability_impact,
        } => generate_flaky_test_recommendation(pattern_type, reliability_impact),
        // Resource management debt types
        DebtType::AsyncMisuse {
            pattern,
            performance_impact,
        } => generate_async_misuse_recommendation(pattern, performance_impact),
        DebtType::ResourceLeak {
            resource_type,
            cleanup_missing,
        } => generate_resource_leak_recommendation(resource_type, cleanup_missing),
        DebtType::CollectionInefficiency {
            collection_type,
            inefficiency_type,
        } => generate_collection_inefficiency_recommendation(collection_type, inefficiency_type),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_float_eq(left: f64, right: f64, epsilon: f64) {
        if (left - right).abs() > epsilon {
            panic!("assertion failed: `(left == right)`\n  left: `{}`,\n right: `{}`\n  diff: `{}`\nepsilon: `{}`", left, right, (left - right).abs(), epsilon);
        }
    }

    #[test]
    fn test_get_role_display_name() {
        assert_eq!(
            get_role_display_name(FunctionRole::PureLogic),
            "Business logic"
        );
        assert_eq!(
            get_role_display_name(FunctionRole::Orchestrator),
            "Orchestration"
        );
        assert_eq!(
            get_role_display_name(FunctionRole::IOWrapper),
            "I/O wrapper"
        );
        assert_eq!(
            get_role_display_name(FunctionRole::EntryPoint),
            "Entry point"
        );
        assert_eq!(
            get_role_display_name(FunctionRole::PatternMatch),
            "Pattern matching"
        );
        assert_eq!(get_role_display_name(FunctionRole::Unknown), "Function");
    }

    #[test]
    fn test_calculate_needed_test_cases_full_coverage() {
        // When coverage is 100%, no test cases needed
        assert_eq!(calculate_needed_test_cases(10, 1.0), 0);
        assert_eq!(calculate_needed_test_cases(25, 1.0), 0);
    }

    #[test]
    fn test_calculate_needed_test_cases_no_coverage() {
        // Cyclo=10 uses Simple tier (linear): 10 × 1.0 = 10
        assert_eq!(calculate_needed_test_cases(10, 0.0), 10);
        // Cyclo=25 uses Moderate tier (sqrt): sqrt(25) * 1.5 + 2 = 5 * 1.5 + 2 = 9.5 → ceil = 10
        assert_eq!(calculate_needed_test_cases(25, 0.0), 10);
    }

    #[test]
    fn test_classify_test_debt() {
        let test_func = FunctionMetrics {
            name: "test_something".to_string(),
            file: std::path::PathBuf::from("tests/test.rs"),
            line: 10,
            length: 20,
            cyclomatic: 4,
            cognitive: 6,
            nesting: 1,
            visibility: Some("pub".to_string()),
            is_test: true,
            is_trait_method: false,
            in_test_module: false,
            entropy_score: None,
            is_pure: Some(false),
            purity_confidence: Some(0.3),
            detected_patterns: None,
            upstream_callers: None,
            downstream_callees: None,
        };

        let debt = classify_test_debt(&test_func);
        match debt {
            DebtType::TestingGap {
                coverage,
                cyclomatic,
                cognitive,
            } => {
                assert_float_eq(coverage, 0.0, 0.01);
                assert_eq!(cyclomatic, 4);
                assert_eq!(cognitive, 6);
            }
            _ => panic!("Expected TestingGap debt type for test function"),
        }
    }

    #[test]
    fn test_calculate_needed_test_cases_partial_coverage() {
        // Cyclo=10 uses Simple tier (linear formula)
        // When 50% covered: 10 × 0.5 = 5
        assert_eq!(calculate_needed_test_cases(10, 0.5), 5);
        // When 80% covered: 10 × 0.2 = 2
        assert_eq!(calculate_needed_test_cases(10, 0.8), 2);
        // When 25% covered: 10 × 0.75 = 7.5 → ceil = 8
        assert_eq!(calculate_needed_test_cases(10, 0.25), 8);
    }

    #[test]
    fn test_calculate_simple_test_cases_minimum() {
        // Always returns at least 2 test cases
        assert_eq!(calculate_simple_test_cases(1, 0.5), 2);
        assert_eq!(calculate_simple_test_cases(1, 0.9), 2);
    }

    #[test]
    fn test_calculate_simple_test_cases_no_coverage() {
        // With no coverage, uses cyclomatic complexity (min 2)
        assert_eq!(calculate_simple_test_cases(5, 0.0), 5);
        assert_eq!(calculate_simple_test_cases(1, 0.0), 2);
    }

    #[test]
    fn test_calculate_simple_test_cases_partial_coverage() {
        // With partial coverage, calculates proportionally
        assert_eq!(calculate_simple_test_cases(10, 0.5), 5);
        assert_eq!(calculate_simple_test_cases(10, 0.8), 2);
    }

    #[test]
    fn test_generate_full_coverage_recommendation() {
        let (action, rationale, steps) =
            generate_full_coverage_recommendation(FunctionRole::PureLogic);
        assert_eq!(action, "Maintain test coverage");
        assert!(rationale.contains("Business logic"));
        assert!(rationale.contains("100% covered"));
        assert_eq!(steps.len(), 3);
        assert!(steps[0].contains("up to date"));
        assert!(steps[1].contains("property-based testing"));
        assert!(steps[2].contains("CI/CD"));
    }

    #[test]
    fn test_generate_full_coverage_recommendation_different_roles() {
        for role in [
            FunctionRole::Orchestrator,
            FunctionRole::IOWrapper,
            FunctionRole::EntryPoint,
            FunctionRole::PatternMatch,
            FunctionRole::Unknown,
        ] {
            let (action, rationale, _) = generate_full_coverage_recommendation(role);
            assert_eq!(action, "Maintain test coverage");
            assert!(rationale.contains("100% covered"));
        }
    }

    #[test]
    fn test_generate_testing_gap_recommendation_full_coverage() {
        // Test case for fully covered function
        let func = FunctionMetrics {
            name: "test_func".to_string(),
            file: "test.rs".into(),
            line: 1,
            cyclomatic: 5,
            cognitive: 8,
            nesting: 2,
            length: 20,
            is_test: false,
            visibility: None,
            is_trait_method: false,
            in_test_module: false,
            entropy_score: None,
            is_pure: Some(true),
            purity_confidence: Some(1.0),
            detected_patterns: None,
            upstream_callers: None,
            downstream_callees: None,
        };

        let (action, rationale, steps) = generate_testing_gap_recommendation(
            1.0, // 100% coverage
            5,   // cyclomatic
            8,   // cognitive
            FunctionRole::PureLogic,
            &func,
            &None,
        );

        assert_eq!(action, "Maintain test coverage");
        assert!(rationale.contains("Business logic function is currently 100% covered"));
        assert_eq!(steps.len(), 3);
        assert!(steps[0].contains("Keep tests up to date"));
    }

    #[test]
    fn test_generate_testing_gap_recommendation_complex_function_no_coverage() {
        // Test case for complex function with no coverage
        let func = FunctionMetrics {
            name: "complex_func".to_string(),
            file: "test.rs".into(),
            line: 1,
            cyclomatic: 25,
            cognitive: 41,
            nesting: 4,
            length: 117,
            is_test: false,
            visibility: None,
            is_trait_method: false,
            in_test_module: false,
            entropy_score: None,
            is_pure: Some(true),
            purity_confidence: Some(1.0),
            detected_patterns: None,
            upstream_callers: None,
            downstream_callees: None,
        };

        let (action, rationale, steps) = generate_testing_gap_recommendation(
            0.0, // 0% coverage
            25,  // cyclomatic
            41,  // cognitive
            FunctionRole::PureLogic,
            &func,
            &None,
        );

        // Should recommend adding tests and refactoring
        assert!(action.contains("Add"));
        assert!(action.contains("tests"));
        assert!(action.contains("100% coverage gap"));
        assert!(action.contains("refactor complexity"));
        assert!(rationale.contains("Complex"));
        assert!(rationale.contains("100% gap"));
        assert!(!steps.is_empty());
    }

    #[test]
    fn test_generate_testing_gap_recommendation_complex_function_partial_coverage() {
        // Test case for complex function with partial coverage
        let func = FunctionMetrics {
            name: "complex_func".to_string(),
            file: "test.rs".into(),
            line: 1,
            cyclomatic: 15,
            cognitive: 20,
            nesting: 3,
            length: 80,
            is_test: false,
            visibility: None,
            is_trait_method: false,
            in_test_module: false,
            entropy_score: None,
            is_pure: Some(false),
            purity_confidence: Some(0.8),
            detected_patterns: None,
            upstream_callers: None,
            downstream_callees: None,
        };

        let (action, rationale, steps) = generate_testing_gap_recommendation(
            0.6, // 60% coverage
            15,  // cyclomatic
            20,  // cognitive
            FunctionRole::Orchestrator,
            &func,
            &None,
        );

        // Should recommend adding tests for 40% gap and refactoring
        assert!(action.contains("40% coverage gap"));
        assert!(action.contains("refactor complexity"));
        assert!(rationale.contains("Complex"));
        assert!(rationale.contains("40% gap"));
        assert!(!steps.is_empty());
    }

    #[test]
    fn test_generate_testing_gap_recommendation_simple_function_no_coverage() {
        // Test case for simple function with no coverage
        let func = FunctionMetrics {
            name: "simple_func".to_string(),
            file: "test.rs".into(),
            line: 1,
            cyclomatic: 5,
            cognitive: 8,
            nesting: 2,
            length: 20,
            is_test: false,
            visibility: None,
            is_trait_method: false,
            in_test_module: false,
            entropy_score: None,
            is_pure: Some(false),
            purity_confidence: Some(0.9),
            detected_patterns: None,
            upstream_callers: None,
            downstream_callees: None,
        };

        let (action, rationale, steps) = generate_testing_gap_recommendation(
            0.0, // 0% coverage
            5,   // cyclomatic
            8,   // cognitive
            FunctionRole::IOWrapper,
            &func,
            &None,
        );

        // Should recommend adding tests for simple function
        assert!(action.contains("Add"));
        assert!(action.contains("test"));
        assert!(action.contains("100% coverage"));
        assert!(rationale.contains("I/O wrapper"));
        assert!(rationale.contains("100% coverage gap"));
        assert!(!steps.is_empty());
    }

    #[test]
    fn test_generate_testing_gap_recommendation_simple_function_partial_coverage() {
        // Test case for simple function with partial coverage
        let func = FunctionMetrics {
            name: "simple_func".to_string(),
            file: "test.rs".into(),
            line: 1,
            cyclomatic: 8,
            cognitive: 10,
            nesting: 2,
            length: 30,
            is_test: false,
            visibility: Some("pub".to_string()),
            is_trait_method: false,
            in_test_module: false,
            entropy_score: None,
            is_pure: Some(false),
            purity_confidence: Some(0.95),
            detected_patterns: None,
            upstream_callers: None,
            downstream_callees: None,
        };

        let (action, rationale, steps) = generate_testing_gap_recommendation(
            0.75, // 75% coverage
            8,    // cyclomatic
            10,   // cognitive
            FunctionRole::EntryPoint,
            &func,
            &None,
        );

        // Should recommend adding tests for 25% gap
        assert!(action.contains("Add"));
        assert!(action.contains("test"));
        assert!(action.contains("25% coverage"));
        assert!(rationale.contains("Entry point"));
        assert!(rationale.contains("25% coverage gap"));
        assert!(!steps.is_empty());
    }

    #[test]
    fn test_generate_testing_gap_recommendation_with_uncovered_lines() {
        // Test case with transitive coverage data
        let func = FunctionMetrics {
            name: "func_with_gaps".to_string(),
            file: "test.rs".into(),
            line: 10,
            cyclomatic: 6,
            cognitive: 9,
            nesting: 2,
            length: 25,
            is_test: false,
            visibility: None,
            is_trait_method: false,
            in_test_module: false,
            entropy_score: None,
            is_pure: Some(true),
            purity_confidence: Some(1.0),
            detected_patterns: None,
            upstream_callers: None,
            downstream_callees: None,
        };

        let transitive_cov = TransitiveCoverage {
            direct: 0.5,
            transitive: 0.5,
            propagated_from: vec![],
            uncovered_lines: vec![13, 14, 17, 18, 19],
        };

        let (action, _rationale, steps) = generate_testing_gap_recommendation(
            0.5, // 50% coverage
            6,   // cyclomatic
            9,   // cognitive
            FunctionRole::PureLogic,
            &func,
            &Some(transitive_cov),
        );

        // Should include uncovered lines analysis in steps
        assert!(action.contains("50% coverage"));
        assert!(!steps.is_empty());
        // The steps should include recommendations from analyze_uncovered_lines
    }

    #[test]
    fn test_generate_testing_gap_recommendation_edge_at_complexity_threshold() {
        // Test edge case right at complexity threshold
        let func = FunctionMetrics {
            name: "edge_func".to_string(),
            file: "test.rs".into(),
            line: 1,
            cyclomatic: 10,
            cognitive: 15,
            nesting: 3,
            length: 50,
            is_test: false,
            visibility: None,
            is_trait_method: false,
            in_test_module: false,
            entropy_score: None,
            is_pure: Some(true),
            purity_confidence: Some(0.85),
            detected_patterns: None,
            upstream_callers: None,
            downstream_callees: None,
        };

        // Test at cyclomatic=10 (not complex)
        let (action1, _, _) = generate_testing_gap_recommendation(
            0.3, // 30% coverage
            10,  // cyclomatic - at threshold
            14,  // cognitive - below threshold
            FunctionRole::PatternMatch,
            &func,
            &None,
        );

        // Should be treated as simple
        assert!(action1.contains("70% coverage"));
        assert!(!action1.contains("refactor complexity"));

        // Test at cyclomatic=11 (complex)
        let (action2, _, _) = generate_testing_gap_recommendation(
            0.3, // 30% coverage
            11,  // cyclomatic - above threshold
            14,  // cognitive - below threshold
            FunctionRole::PatternMatch,
            &func,
            &None,
        );

        // Should be treated as complex
        assert!(action2.contains("70% coverage gap"));
        assert!(action2.contains("refactor complexity"));
    }

    // Tests for extracted pure functions (spec 93)

    #[test]
    fn test_create_function_id() {
        use crate::priority::scoring::construction::create_function_id;

        let func = FunctionMetrics {
            name: "test_func".to_string(),
            file: "/path/to/file.rs".into(),
            line: 42,
            cyclomatic: 5,
            cognitive: 8,
            nesting: 2,
            length: 20,
            is_test: false,
            visibility: None,
            is_trait_method: false,
            in_test_module: false,
            entropy_score: None,
            is_pure: Some(true),
            purity_confidence: Some(0.9),
            detected_patterns: None,
            upstream_callers: None,
            downstream_callees: None,
        };

        let func_id = create_function_id(&func);
        assert_eq!(func_id.name, "test_func");
        assert_eq!(func_id.file.to_str().unwrap(), "/path/to/file.rs");
        assert_eq!(func_id.line, 42);
    }

    #[test]
    fn test_calculate_functions_to_extract() {
        // Test various complexity levels
        assert_eq!(calculate_functions_to_extract(5, 5), 2);
        assert_eq!(calculate_functions_to_extract(10, 10), 2);
        assert_eq!(calculate_functions_to_extract(15, 15), 3);
        assert_eq!(calculate_functions_to_extract(20, 20), 4);
        assert_eq!(calculate_functions_to_extract(25, 25), 5);
        assert_eq!(calculate_functions_to_extract(30, 30), 6);
        assert_eq!(calculate_functions_to_extract(50, 50), 10);

        // Test with different cyclomatic and cognitive values
        assert_eq!(calculate_functions_to_extract(10, 20), 4);
        assert_eq!(calculate_functions_to_extract(30, 15), 6);
    }

    #[test]
    fn test_format_complexity_display() {
        assert_eq!(format_complexity_display(&5, &8), "cyclo=5, cog=8");
        assert_eq!(format_complexity_display(&10, &15), "cyclo=10, cog=15");
        assert_eq!(format_complexity_display(&0, &0), "cyclo=0, cog=0");
    }

    #[test]
    fn test_format_role_description() {
        assert_eq!(
            format_role_description(FunctionRole::PureLogic),
            "business logic"
        );
        assert_eq!(
            format_role_description(FunctionRole::Orchestrator),
            "orchestration"
        );
        assert_eq!(
            format_role_description(FunctionRole::IOWrapper),
            "I/O wrapper"
        );
        assert_eq!(
            format_role_description(FunctionRole::EntryPoint),
            "entry point"
        );
        assert_eq!(
            format_role_description(FunctionRole::PatternMatch),
            "pattern matching"
        );
        assert_eq!(format_role_description(FunctionRole::Unknown), "function");
    }

    #[test]
    fn test_is_function_complex() {
        // Test not complex
        assert!(!is_function_complex(5, 10));
        assert!(!is_function_complex(10, 15));

        // Test complex based on cyclomatic
        assert!(is_function_complex(11, 10));
        assert!(is_function_complex(20, 5));

        // Test complex based on cognitive
        assert!(is_function_complex(5, 16));
        assert!(is_function_complex(10, 20));

        // Test complex based on both
        assert!(is_function_complex(15, 20));
    }

    #[test]
    fn test_calculate_risk_factor() {
        // Test various debt types
        assert_eq!(
            calculate_risk_factor(&DebtType::TestingGap {
                coverage: 0.5,
                cyclomatic: 10,
                cognitive: 15
            }),
            0.42
        );

        assert_eq!(
            calculate_risk_factor(&DebtType::ComplexityHotspot {
                cyclomatic: 20,
                cognitive: 25
            }),
            0.35
        );

        assert_eq!(
            calculate_risk_factor(&DebtType::ErrorSwallowing {
                pattern: "unwrap_or_default".to_string(),
                context: None
            }),
            0.35
        );

        assert_eq!(
            calculate_risk_factor(&DebtType::DeadCode {
                visibility: FunctionVisibility::Private,
                cyclomatic: 5,
                cognitive: 8,
                usage_hints: vec![]
            }),
            0.3
        );
    }

    #[test]
    fn test_calculate_coverage_improvement() {
        // Test simple function
        assert_float_eq(calculate_coverage_improvement(0.0, false), 100.0, 1e-10);
        assert_float_eq(calculate_coverage_improvement(0.5, false), 50.0, 1e-10);
        assert_float_eq(calculate_coverage_improvement(0.8, false), 20.0, 1e-10);
        assert_float_eq(calculate_coverage_improvement(1.0, false), 0.0, 1e-10);

        // Test complex function (50% reduction)
        assert_float_eq(calculate_coverage_improvement(0.0, true), 50.0, 1e-10);
        assert_float_eq(calculate_coverage_improvement(0.5, true), 25.0, 1e-10);
        assert_float_eq(calculate_coverage_improvement(0.8, true), 10.0, 1e-10);
        assert_float_eq(calculate_coverage_improvement(1.0, true), 0.0, 1e-10);
    }

    #[test]
    fn test_calculate_lines_reduction() {
        // Test dead code
        let dead_code = DebtType::DeadCode {
            visibility: FunctionVisibility::Private,
            cyclomatic: 10,
            cognitive: 15,
            usage_hints: vec![],
        };
        assert_eq!(calculate_lines_reduction(&dead_code), 25);

        // Test duplication
        let duplication = DebtType::Duplication {
            instances: 4,
            total_lines: 100,
        };
        assert_eq!(calculate_lines_reduction(&duplication), 75);

        // Test other types
        let complexity = DebtType::ComplexityHotspot {
            cyclomatic: 20,
            cognitive: 25,
        };
        assert_eq!(calculate_lines_reduction(&complexity), 0);
    }

    #[test]
    fn test_is_rust_file() {
        use std::path::Path;
        assert!(is_rust_file(Path::new("test.rs")));
        assert!(is_rust_file(Path::new("/path/to/file.rs")));
        assert!(!is_rust_file(Path::new("test.py")));
        assert!(!is_rust_file(Path::new("test.js")));
        assert!(!is_rust_file(Path::new("test")));
    }

    #[test]
    fn test_extract_coverage_percent() {
        // Test with coverage
        let coverage = TransitiveCoverage {
            direct: 0.75,
            transitive: 0.85,
            propagated_from: vec![],
            uncovered_lines: vec![],
        };
        assert_eq!(extract_coverage_percent(&Some(coverage)), 0.75);

        // Test without coverage
        assert_eq!(extract_coverage_percent(&None), 0.0);
    }

    #[test]
    fn test_extract_cyclomatic_complexity() {
        assert_eq!(
            extract_cyclomatic_complexity(&DebtType::ComplexityHotspot {
                cyclomatic: 15,
                cognitive: 20,
            }),
            15
        );

        assert_eq!(
            extract_cyclomatic_complexity(&DebtType::TestingGap {
                coverage: 0.5,
                cyclomatic: 10,
                cognitive: 12,
            }),
            0
        );

        assert_eq!(
            extract_cyclomatic_complexity(&DebtType::DeadCode {
                visibility: FunctionVisibility::Public,
                cyclomatic: 5,
                cognitive: 8,
                usage_hints: vec![],
            }),
            0
        );
    }

    #[test]
    fn test_build_actionable_recommendation() {
        let recommendation = build_actionable_recommendation(
            "Fix the issue".to_string(),
            "This is why it matters".to_string(),
            vec!["Step 1".to_string(), "Step 2".to_string()],
        );

        assert_eq!(recommendation.primary_action, "Fix the issue");
        assert_eq!(recommendation.rationale, "This is why it matters");
        assert_eq!(recommendation.implementation_steps.len(), 2);
        assert_eq!(recommendation.implementation_steps[0], "Step 1");
        assert_eq!(recommendation.implementation_steps[1], "Step 2");
        assert!(recommendation.related_items.is_empty());
    }

    #[test]
    fn test_create_recommendation_context() {
        let func = FunctionMetrics {
            name: "test_func".to_string(),
            file: "/test.rs".into(),
            line: 10,
            cyclomatic: 15,
            cognitive: 20,
            nesting: 3,
            length: 50,
            is_test: false,
            visibility: Some("pub".to_string()),
            is_trait_method: false,
            in_test_module: false,
            entropy_score: None,
            is_pure: Some(true),
            purity_confidence: Some(0.95),
            detected_patterns: None,
            upstream_callers: None,
            downstream_callees: None,
        };

        let debt_type = DebtType::ComplexityHotspot {
            cyclomatic: 15,
            cognitive: 20,
        };

        let score = UnifiedScore {
            complexity_factor: 7.5,
            coverage_factor: 6.0,
            dependency_factor: 2.0,
            role_multiplier: 1.2,
            final_score: 8.5,
        };

        let coverage = TransitiveCoverage {
            direct: 0.6,
            transitive: 0.7,
            propagated_from: vec![],
            uncovered_lines: vec![15, 16, 20],
        };

        let context = create_recommendation_context(
            &func,
            &debt_type,
            FunctionRole::PureLogic,
            &score,
            &Some(coverage.clone()),
            true, // has_coverage_data
        );

        assert_eq!(context.function_info.name, "test_func");
        assert_eq!(context.function_info.line, 10);
        assert!(context.is_rust_file);
        assert_eq!(context.coverage_percent, 0.6);
        assert!(context.has_coverage_data);
        assert_eq!(context.role, FunctionRole::PureLogic);
    }

    #[test]
    fn test_function_info_from_metrics() {
        let func = FunctionMetrics {
            name: "my_function".to_string(),
            file: "/src/lib.rs".into(),
            line: 100,
            cyclomatic: 8,
            cognitive: 12,
            nesting: 2,
            length: 35,
            is_test: false,
            visibility: None,
            is_trait_method: false,
            in_test_module: false,
            entropy_score: None,
            is_pure: Some(true),
            purity_confidence: Some(0.85),
            detected_patterns: None,
            upstream_callers: None,
            downstream_callees: None,
        };

        let info = FunctionInfo::from_metrics(&func);

        assert_eq!(info.name, "my_function");
        assert_eq!(info.file.to_str().unwrap(), "/src/lib.rs");
        assert_eq!(info.line, 100);
        assert_eq!(info.nesting, 2);
        assert_eq!(info.length, 35);
        assert_eq!(info.cognitive, 12);
        assert!(info.is_pure);
        assert_eq!(info.purity_confidence, 0.85);
    }
}
