// Functions for creating UnifiedDebtItem instances

use crate::core::FunctionMetrics;
use crate::priority::unified_scorer::{
    calculate_unified_priority, calculate_unified_priority_with_debt, EntropyDetails,
};
use crate::priority::{
    call_graph::{CallGraph, FunctionId},
    coverage_propagation::calculate_transitive_coverage,
    debt_aggregator::DebtAggregator,
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
use crate::risk::lcov::LcovData;
use std::collections::HashSet;

/// Create a unified debt item with enhanced call graph analysis
pub fn create_unified_debt_item_enhanced(
    func: &FunctionMetrics,
    call_graph: &CallGraph,
    _enhanced_call_graph: Option<()>, // Placeholder for future enhanced call graph
    coverage: Option<&LcovData>,
) -> UnifiedDebtItem {
    let func_id = FunctionId {
        file: func.file.clone(),
        name: func.name.clone(),
        line: func.line,
    };

    // Security factor removed per spec 64
    // Organization factor removed per spec 58 - redundant with complexity factor

    let unified_score = calculate_unified_priority(
        func, call_graph, coverage, None, // Organization factor no longer used
    );
    let role = classify_function_role(func, &func_id, call_graph);

    let transitive_coverage =
        coverage.map(|cov| calculate_transitive_coverage(&func_id, call_graph, cov));

    // Use enhanced debt type classification
    let debt_type = classify_debt_type_enhanced(func, call_graph, &func_id);

    let recommendation = generate_recommendation(func, &debt_type, role, &unified_score);
    let expected_impact = calculate_expected_impact(func, &debt_type, &unified_score);

    // Get caller and callee names
    let upstream_callers = call_graph.get_callers(&func_id);
    let downstream_callees = call_graph.get_callees(&func_id);

    let upstream_caller_names: Vec<String> =
        upstream_callers.iter().map(|id| id.name.clone()).collect();
    let downstream_callee_names: Vec<String> = downstream_callees
        .iter()
        .map(|id| id.name.clone())
        .collect();

    UnifiedDebtItem {
        location: Location {
            file: func.file.clone(),
            function: func.name.clone(),
            line: func.line,
        },
        debt_type,
        unified_score,
        function_role: role,
        recommendation,
        expected_impact,
        transitive_coverage,
        upstream_dependencies: upstream_callers.len(),
        downstream_dependencies: downstream_callees.len(),
        upstream_callers: upstream_caller_names,
        downstream_callees: downstream_callee_names,
        nesting_depth: func.nesting,
        function_length: func.length,
        cyclomatic_complexity: func.cyclomatic,
        cognitive_complexity: func.cognitive,
        entropy_details: calculate_entropy_details(func),
        is_pure: func.is_pure,
        purity_confidence: func.purity_confidence,
        god_object_indicators: None,
    }
}

pub fn create_unified_debt_item_with_aggregator(
    func: &FunctionMetrics,
    call_graph: &CallGraph,
    coverage: Option<&LcovData>,
    framework_exclusions: &HashSet<FunctionId>,
    function_pointer_used_functions: Option<&HashSet<FunctionId>>,
    debt_aggregator: &DebtAggregator,
) -> UnifiedDebtItem {
    create_unified_debt_item_with_aggregator_and_data_flow(
        func,
        call_graph,
        coverage,
        framework_exclusions,
        function_pointer_used_functions,
        debt_aggregator,
        None, // DataFlowGraph will be provided by the new function
    )
}

// Pure function: Extract function ID creation
fn create_function_id(func: &FunctionMetrics) -> FunctionId {
    FunctionId {
        file: func.file.clone(),
        name: func.name.clone(),
        line: func.line,
    }
}

// Pure function: Calculate coverage data
fn calculate_coverage_data(
    func_id: &FunctionId,
    func: &FunctionMetrics,
    call_graph: &CallGraph,
    coverage: Option<&LcovData>,
) -> Option<TransitiveCoverage> {
    coverage.and_then(|lcov| {
        let end_line = func.line + func.length.saturating_sub(1);
        lcov.get_function_coverage_with_bounds(&func.file, &func.name, func.line, end_line)
            .map(|_| calculate_transitive_coverage(func_id, call_graph, lcov))
    })
}

// Pure function: Build debt analysis context
struct DebtAnalysisContext {
    #[allow(dead_code)]
    func_id: FunctionId,
    debt_type: DebtType,
    unified_score: UnifiedScore,
    function_role: FunctionRole,
    transitive_coverage: Option<TransitiveCoverage>,
    recommendation: ActionableRecommendation,
    expected_impact: ImpactMetrics,
}

// Pure function: Perform debt analysis
#[allow(clippy::too_many_arguments)]
fn analyze_debt(
    func: &FunctionMetrics,
    func_id: &FunctionId,
    call_graph: &CallGraph,
    coverage: Option<&LcovData>,
    framework_exclusions: &HashSet<FunctionId>,
    function_pointer_used_functions: Option<&HashSet<FunctionId>>,
    debt_aggregator: &DebtAggregator,
    data_flow: Option<&crate::data_flow::DataFlowGraph>,
) -> DebtAnalysisContext {
    // Calculate transitive coverage
    let transitive_coverage = calculate_coverage_data(func_id, func, call_graph, coverage);

    // Classify debt type
    let debt_type = classify_debt_type_with_exclusions(
        func,
        call_graph,
        func_id,
        framework_exclusions,
        function_pointer_used_functions,
        transitive_coverage.as_ref(),
    );

    // Calculate unified score
    let unified_score = calculate_unified_priority_with_debt(
        func,
        call_graph,
        coverage,
        None,
        Some(debt_aggregator),
    );

    // Determine function role
    let function_role = classify_function_role(func, func_id, call_graph);

    // Generate recommendation
    let recommendation = generate_recommendation_with_coverage_and_data_flow(
        func,
        &debt_type,
        function_role,
        &unified_score,
        &transitive_coverage,
        data_flow,
    );

    // Calculate expected impact
    let expected_impact = calculate_expected_impact(func, &debt_type, &unified_score);

    DebtAnalysisContext {
        func_id: func_id.clone(),
        debt_type,
        unified_score,
        function_role,
        transitive_coverage,
        recommendation,
        expected_impact,
    }
}

// Pure function: Extract dependency metrics
struct DependencyMetrics {
    upstream_count: usize,
    downstream_count: usize,
    upstream_names: Vec<String>,
    downstream_names: Vec<String>,
}

fn extract_dependency_metrics(func_id: &FunctionId, call_graph: &CallGraph) -> DependencyMetrics {
    let upstream = call_graph.get_callers(func_id);
    let downstream = call_graph.get_callees(func_id);

    DependencyMetrics {
        upstream_count: upstream.len(),
        downstream_count: downstream.len(),
        upstream_names: upstream.iter().map(|f| f.name.clone()).collect(),
        downstream_names: downstream.iter().map(|f| f.name.clone()).collect(),
    }
}

// Pure function: Build unified debt item from components
fn build_unified_debt_item(
    func: &FunctionMetrics,
    context: DebtAnalysisContext,
    deps: DependencyMetrics,
) -> UnifiedDebtItem {
    UnifiedDebtItem {
        location: Location {
            file: func.file.clone(),
            function: func.name.clone(),
            line: func.line,
        },
        debt_type: context.debt_type,
        unified_score: context.unified_score,
        function_role: context.function_role,
        recommendation: context.recommendation,
        expected_impact: context.expected_impact,
        transitive_coverage: context.transitive_coverage,
        upstream_dependencies: deps.upstream_count,
        downstream_dependencies: deps.downstream_count,
        upstream_callers: deps.upstream_names,
        downstream_callees: deps.downstream_names,
        nesting_depth: func.nesting,
        function_length: func.length,
        cyclomatic_complexity: func.cyclomatic,
        cognitive_complexity: func.cognitive,
        entropy_details: calculate_entropy_details(func),
        is_pure: func.is_pure,
        purity_confidence: func.purity_confidence,
        god_object_indicators: None,
    }
}

// Main function using functional composition
pub fn create_unified_debt_item_with_aggregator_and_data_flow(
    func: &FunctionMetrics,
    call_graph: &CallGraph,
    coverage: Option<&LcovData>,
    framework_exclusions: &HashSet<FunctionId>,
    function_pointer_used_functions: Option<&HashSet<FunctionId>>,
    debt_aggregator: &DebtAggregator,
    data_flow: Option<&crate::data_flow::DataFlowGraph>,
) -> UnifiedDebtItem {
    // Step 1: Create function ID (pure)
    let func_id = create_function_id(func);

    // Step 2: Analyze debt (pure)
    let context = analyze_debt(
        func,
        &func_id,
        call_graph,
        coverage,
        framework_exclusions,
        function_pointer_used_functions,
        debt_aggregator,
        data_flow,
    );

    // Step 3: Extract dependencies (pure)
    let deps = extract_dependency_metrics(&func_id, call_graph);

    // Step 4: Build final item (pure)
    build_unified_debt_item(func, context, deps)
}

pub fn create_unified_debt_item_with_exclusions(
    func: &FunctionMetrics,
    call_graph: &CallGraph,
    coverage: Option<&LcovData>,
    framework_exclusions: &HashSet<FunctionId>,
    function_pointer_used_functions: Option<&HashSet<FunctionId>>,
) -> UnifiedDebtItem {
    create_unified_debt_item_with_exclusions_and_data_flow(
        func,
        call_graph,
        coverage,
        framework_exclusions,
        function_pointer_used_functions,
        None,
    )
}

pub fn create_unified_debt_item_with_exclusions_and_data_flow(
    func: &FunctionMetrics,
    call_graph: &CallGraph,
    coverage: Option<&LcovData>,
    framework_exclusions: &HashSet<FunctionId>,
    function_pointer_used_functions: Option<&HashSet<FunctionId>>,
    data_flow: Option<&crate::data_flow::DataFlowGraph>,
) -> UnifiedDebtItem {
    let func_id = FunctionId {
        file: func.file.clone(),
        name: func.name.clone(),
        line: func.line,
    };

    // Calculate transitive coverage if direct coverage is available
    // Use exact AST boundaries for more accurate coverage matching
    let transitive_coverage = coverage.and_then(|lcov| {
        let end_line = func.line + func.length.saturating_sub(1);
        lcov.get_function_coverage_with_bounds(&func.file, &func.name, func.line, end_line)
            .map(|_direct| calculate_transitive_coverage(&func_id, call_graph, lcov))
    });

    // Use the enhanced debt type classification with framework exclusions
    let debt_type = classify_debt_type_with_exclusions(
        func,
        call_graph,
        &func_id,
        framework_exclusions,
        function_pointer_used_functions,
        transitive_coverage.as_ref(),
    );

    // Calculate unified score
    // Security factor removed per spec 64
    // Organization factor removed per spec 58 - redundant with complexity factor

    let unified_score = calculate_unified_priority(
        func, call_graph, coverage, None, // Organization factor no longer used
    );

    // Determine function role for more accurate analysis
    let function_role = classify_function_role(func, &func_id, call_graph);

    // Generate contextual recommendation based on debt type and metrics
    let recommendation = generate_recommendation_with_coverage_and_data_flow(
        func,
        &debt_type,
        function_role,
        &unified_score,
        &transitive_coverage,
        data_flow,
    );

    // Calculate expected impact
    let expected_impact = calculate_expected_impact(func, &debt_type, &unified_score);

    // Get dependency information
    let upstream = call_graph.get_callers(&func_id);
    let downstream = call_graph.get_callees(&func_id);

    UnifiedDebtItem {
        location: Location {
            file: func.file.clone(),
            function: func.name.clone(),
            line: func.line,
        },
        debt_type,
        unified_score,
        function_role,
        recommendation,
        expected_impact,
        transitive_coverage,
        upstream_dependencies: upstream.len(),
        downstream_dependencies: downstream.len(),
        upstream_callers: upstream.iter().map(|f| f.name.clone()).collect(),
        downstream_callees: downstream.iter().map(|f| f.name.clone()).collect(),
        nesting_depth: func.nesting,
        function_length: func.length,
        cyclomatic_complexity: func.cyclomatic,
        cognitive_complexity: func.cognitive,
        entropy_details: calculate_entropy_details(func),
        is_pure: func.is_pure,
        purity_confidence: func.purity_confidence,
        god_object_indicators: None,
    }
}

/// Create a unified debt item for a function
pub fn create_unified_debt_item(
    func: &FunctionMetrics,
    call_graph: &CallGraph,
    coverage: Option<&LcovData>,
) -> UnifiedDebtItem {
    create_unified_debt_item_with_data_flow(func, call_graph, coverage, None)
}

pub fn create_unified_debt_item_with_data_flow(
    func: &FunctionMetrics,
    call_graph: &CallGraph,
    coverage: Option<&LcovData>,
    data_flow: Option<&crate::data_flow::DataFlowGraph>,
) -> UnifiedDebtItem {
    let func_id = FunctionId {
        file: func.file.clone(),
        name: func.name.clone(),
        line: func.line,
    };

    // Security factor removed per spec 64
    // Organization factor removed per spec 58 - redundant with complexity factor

    let unified_score = calculate_unified_priority(
        func, call_graph, coverage, None, // Organization factor no longer used
    );
    let role = classify_function_role(func, &func_id, call_graph);

    let transitive_coverage =
        coverage.map(|cov| calculate_transitive_coverage(&func_id, call_graph, cov));

    let debt_type = determine_debt_type(func, &transitive_coverage, call_graph, &func_id);
    let recommendation = generate_recommendation_with_coverage_and_data_flow(
        func,
        &debt_type,
        role,
        &unified_score,
        &transitive_coverage,
        data_flow,
    );
    let expected_impact = calculate_expected_impact(func, &debt_type, &unified_score);

    // Get dependency counts and names from call graph
    let upstream_callers = call_graph.get_callers(&func_id);
    let downstream_callees = call_graph.get_callees(&func_id);

    let upstream_caller_names: Vec<String> =
        upstream_callers.iter().map(|id| id.name.clone()).collect();
    let downstream_callee_names: Vec<String> = downstream_callees
        .iter()
        .map(|id| id.name.clone())
        .collect();

    // Calculate entropy details
    let entropy_details = calculate_entropy_details(func);

    UnifiedDebtItem {
        location: Location {
            file: func.file.clone(),
            function: func.name.clone(),
            line: func.line,
        },
        debt_type,
        unified_score,
        function_role: role,
        recommendation,
        expected_impact,
        transitive_coverage,
        upstream_dependencies: upstream_callers.len(),
        downstream_dependencies: downstream_callees.len(),
        upstream_callers: upstream_caller_names,
        downstream_callees: downstream_callee_names,
        nesting_depth: func.nesting,
        function_length: func.length,
        cyclomatic_complexity: func.cyclomatic,
        cognitive_complexity: func.cognitive,
        entropy_details,
        is_pure: func.is_pure,
        purity_confidence: func.purity_confidence,
        god_object_indicators: None,
    }
}

// Helper functions

/// Helper function to calculate entropy details from FunctionMetrics
fn calculate_entropy_details(func: &FunctionMetrics) -> Option<EntropyDetails> {
    func.entropy_score.as_ref().map(|entropy_score| {
        // Use the new framework's dampening calculation
        let calculator = crate::complexity::entropy_core::UniversalEntropyCalculator::new(
            crate::complexity::entropy_core::EntropyConfig::default(),
        );
        let dampening_value = calculator.apply_dampening(entropy_score);
        let dampening_factor = (dampening_value / 2.0).clamp(0.5, 1.0); // Normalize to 0.5-1.0 range

        let adjusted_cyclomatic = (func.cyclomatic as f64 * dampening_factor) as u32;
        let _adjusted_cognitive = (func.cognitive as f64 * dampening_factor) as u32;

        EntropyDetails {
            entropy_score: entropy_score.token_entropy,
            pattern_repetition: entropy_score.pattern_repetition,
            original_complexity: func.cyclomatic,
            adjusted_complexity: adjusted_cyclomatic,
            dampening_factor,
        }
    })
}

fn determine_debt_type(
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

/// Pure function to check for testing gaps
fn check_testing_gap(
    func: &FunctionMetrics,
    coverage: &Option<TransitiveCoverage>,
) -> Option<DebtType> {
    coverage
        .as_ref()
        .filter(|cov| cov.direct < 0.2 && !func.is_test)
        .map(|cov| DebtType::TestingGap {
            coverage: cov.direct,
            cyclomatic: func.cyclomatic,
            cognitive: func.cognitive,
        })
}

/// Pure function to check for complexity hotspots
fn check_complexity_hotspot(func: &FunctionMetrics) -> Option<DebtType> {
    if func.cyclomatic > 10 || func.cognitive > 15 {
        Some(DebtType::ComplexityHotspot {
            cyclomatic: func.cyclomatic,
            cognitive: func.cognitive,
        })
    } else {
        None
    }
}

/// Pure function to check for dead code
fn check_dead_code(
    func: &FunctionMetrics,
    call_graph: &CallGraph,
    func_id: &FunctionId,
) -> Option<DebtType> {
    if is_dead_code(func, call_graph, func_id, None) {
        Some(DebtType::DeadCode {
            visibility: determine_visibility(func),
            cyclomatic: func.cyclomatic,
            cognitive: func.cognitive,
            usage_hints: generate_usage_hints(func, call_graph, func_id),
        })
    } else {
        None
    }
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

fn calculate_risk_score(func: &FunctionMetrics) -> f64 {
    // Better scaling for complexity risk (0-1 range)
    // Cyclomatic 10 = 0.33, 20 = 0.67, 30+ = 1.0
    let cyclo_risk = (func.cyclomatic as f64 / 30.0).min(1.0);

    // Cognitive complexity tends to be higher, so scale differently
    // Cognitive 15 = 0.33, 30 = 0.67, 45+ = 1.0
    let cognitive_risk = (func.cognitive as f64 / 45.0).min(1.0);

    // Length risk - functions over 100 lines are definitely risky
    let length_risk = (func.length as f64 / 100.0).min(1.0);

    // Average the three risk factors
    // Complexity is most important, then cognitive, then length
    let weighted_risk = cyclo_risk * 0.4 + cognitive_risk * 0.4 + length_risk * 0.2;

    // Scale to 0-10 range for final risk score
    // Note: Coverage is handled separately in the unified scoring system
    weighted_risk * 10.0
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
    // Check hardcoded exclusions (includes test functions, main, etc.)
    if is_excluded_from_dead_code_analysis(func) {
        return false;
    }

    // Check if function is definitely used through function pointers
    if let Some(fp_used) = function_pointer_used_functions {
        if fp_used.contains(func_id) {
            return false;
        }
    }

    // Check if function has incoming calls
    let callers = call_graph.get_callers(func_id);
    callers.is_empty()
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

// Pure function to check if function has testing gap
fn has_testing_gap(coverage: f64, is_test: bool) -> bool {
    coverage < 0.2 && !is_test
}

// Pure function to check if function is complexity hotspot based on metrics only
fn is_complexity_hotspot_by_metrics(cyclomatic: u32, cognitive: u32) -> bool {
    cyclomatic > 5 || cognitive > 8
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

/// Context structure for debt classification
struct ClassificationContext<'a> {
    func: &'a FunctionMetrics,
    call_graph: &'a CallGraph,
    func_id: &'a FunctionId,
    framework_exclusions: &'a HashSet<FunctionId>,
    function_pointer_used_functions: Option<&'a HashSet<FunctionId>>,
    coverage: Option<&'a TransitiveCoverage>,
}

/// Pure function to check for enhanced testing gaps
fn check_enhanced_testing_gap(context: &ClassificationContext) -> Option<DebtType> {
    context.coverage.and_then(|cov| {
        let has_gap = has_testing_gap(cov.direct, context.func.is_test)
            || (cov.direct < 0.8 && context.func.cyclomatic > 5 && !cov.uncovered_lines.is_empty());

        if has_gap {
            Some(DebtType::TestingGap {
                coverage: cov.direct,
                cyclomatic: context.func.cyclomatic,
                cognitive: context.func.cognitive,
            })
        } else {
            None
        }
    })
}

/// Pure function to check for enhanced complexity hotspots
fn check_enhanced_complexity_hotspot(func: &FunctionMetrics) -> Option<DebtType> {
    if is_complexity_hotspot_by_metrics(func.cyclomatic, func.cognitive) {
        Some(DebtType::ComplexityHotspot {
            cyclomatic: func.cyclomatic,
            cognitive: func.cognitive,
        })
    } else {
        None
    }
}

/// Pure function to check for enhanced dead code
fn check_enhanced_dead_code(context: &ClassificationContext) -> Option<DebtType> {
    if is_dead_code_with_exclusions(
        context.func,
        context.call_graph,
        context.func_id,
        context.framework_exclusions,
        context.function_pointer_used_functions,
    ) {
        Some(DebtType::DeadCode {
            visibility: determine_visibility(context.func),
            cyclomatic: context.func.cyclomatic,
            cognitive: context.func.cognitive,
            usage_hints: generate_usage_hints(context.func, context.call_graph, context.func_id),
        })
    } else {
        None
    }
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

fn is_excluded_from_dead_code_analysis(func: &FunctionMetrics) -> bool {
    // Entry points
    if func.name == "main" || func.name.starts_with("_start") {
        return true;
    }

    // Test functions
    if func.is_test
        || func.name.starts_with("test_")
        || func.file.to_string_lossy().contains("/tests/")
        || func.in_test_module
    // Helper functions in test modules
    {
        return true;
    }

    // Closures are part of their parent function - not independent dead code
    if func.name.contains("<closure@") {
        return true;
    }

    // Exported functions (likely FFI or API) - check for common patterns
    if func.name.contains("extern") || func.name.starts_with("__") {
        return true;
    }

    // Common framework patterns
    if is_framework_callback(func) {
        return true;
    }

    // Trait method implementations - these are called through trait objects
    // Use the new is_trait_method field for accurate detection
    if func.is_trait_method {
        return true;
    }

    // Also check for common trait method patterns as fallback
    if is_likely_trait_method(func) {
        return true;
    }

    false
}

fn is_likely_trait_method(func: &FunctionMetrics) -> bool {
    // Check if this is likely a trait method implementation based on:
    // 1. Public visibility + specific method names that are commonly trait methods
    // 2. Methods that are part of common trait implementations

    if func.visibility == Some("pub".to_string()) {
        // Common trait methods that should not be flagged as dead code
        let method_name = if let Some(pos) = func.name.rfind("::") {
            &func.name[pos + 2..]
        } else {
            &func.name
        };

        matches!(
            method_name,
            // Common trait methods from std library traits
            "write_results" | "write_risk_insights" |  // OutputWriter trait
            "fmt" | "clone" | "default" | "from" | "into" |
            "as_ref" | "as_mut" | "deref" | "deref_mut" |
            "drop" | "eq" | "ne" | "cmp" | "partial_cmp" |
            "hash" | "serialize" | "deserialize" |
            "try_from" | "try_into" | "to_string" |
            // Iterator trait methods
            "next" | "size_hint" | "count" | "last" | "nth" |
            // Async trait methods
            "poll" | "poll_next" | "poll_ready" | "poll_flush" |
            // Common custom trait methods
            "execute" | "run" | "process" | "handle" |
            "render" | "draw" | "update" | "tick" |
            "validate" | "is_valid" | "check" |
            "encode" | "decode" | "parse" | "format"
        )
    } else {
        false
    }
}

fn is_framework_callback(func: &FunctionMetrics) -> bool {
    // Common web framework handlers
    func.name.contains("handler") || 
    func.name.contains("route") ||
    func.name.contains("view") ||
    func.name.contains("controller") ||
    // Common async patterns
    func.name.starts_with("on_") ||
    func.name.starts_with("handle_") ||
    // Common trait implementations
    func.name == "new" ||
    func.name == "default" ||
    func.name == "fmt" ||
    func.name == "drop" ||
    func.name == "clone"
}

pub fn determine_visibility(func: &FunctionMetrics) -> FunctionVisibility {
    // Use the visibility field from FunctionMetrics if available
    match &func.visibility {
        Some(vis) if vis == "pub" => FunctionVisibility::Public,
        Some(vis) if vis == "pub(crate)" => FunctionVisibility::Crate,
        Some(vis) if vis.starts_with("pub(") => FunctionVisibility::Crate, // pub(super), pub(in ...), etc.
        _ => FunctionVisibility::Private,
    }
}

/// Helper to format complexity metrics for display
fn format_complexity_display(cyclomatic: &u32, cognitive: &u32) -> String {
    format!("cyclo={cyclomatic}, cog={cognitive}")
}

/// Helper to format role description
fn format_role_description(role: FunctionRole) -> &'static str {
    match role {
        FunctionRole::PureLogic => "business logic",
        FunctionRole::Orchestrator => "orchestration",
        FunctionRole::IOWrapper => "I/O wrapper",
        FunctionRole::EntryPoint => "entry point",
        FunctionRole::PatternMatch => "pattern matching",
        FunctionRole::Unknown => "function",
    }
}

/// Generate steps for dead code based on visibility
fn generate_dead_code_steps(visibility: &FunctionVisibility) -> Vec<String> {
    match visibility {
        FunctionVisibility::Private => vec![
            "Verify no dynamic calls or reflection usage".to_string(),
            "Remove function definition".to_string(),
            "Remove associated tests if any".to_string(),
            "Check if removal enables further cleanup".to_string(),
        ],
        FunctionVisibility::Crate => vec![
            "Check if function is intended as internal API".to_string(),
            "Add documentation if keeping for future use".to_string(),
            "Remove if truly unused".to_string(),
            "Consider making private if only locally needed".to_string(),
        ],
        FunctionVisibility::Public => vec![
            "Verify no external callers exist".to_string(),
            "Add comprehensive documentation if keeping".to_string(),
            "Mark as deprecated if phasing out".to_string(),
            "Consider adding usage examples or tests".to_string(),
        ],
    }
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

/// Generate steps for testing gap based on complexity
fn generate_testing_gap_steps(is_complex: bool) -> Vec<String> {
    if is_complex {
        vec![
            "Identify and extract pure functions (no side effects)".to_string(),
            "Add property-based tests for pure logic".to_string(),
            "Replace conditionals with pattern matching where possible".to_string(),
            "Convert loops to map/filter/fold operations".to_string(),
            "Push I/O to the boundaries".to_string(),
        ]
    } else {
        vec![
            "Test happy path scenarios".to_string(),
            "Add edge case tests".to_string(),
            "Cover error conditions".to_string(),
        ]
    }
}

/// Calculate number of functions to extract based on complexity
/// Algorithm: Divide max complexity by target (3-5) to get number of functions needed
/// to achieve manageable complexity per function
fn calculate_functions_to_extract(cyclomatic: u32, cognitive: u32) -> u32 {
    let max_complexity = cyclomatic.max(cognitive);
    // Target complexity per function is 3-5
    // Calculate how many functions needed to achieve this
    match max_complexity {
        0..=10 => 2,                      // Extract 2 functions: 10/2 = 5 complexity each
        11..=15 => 3,                     // Extract 3 functions: 15/3 = 5 complexity each
        16..=20 => 4,                     // Extract 4 functions: 20/4 = 5 complexity each
        21..=25 => 5,                     // Extract 5 functions: 25/5 = 5 complexity each
        26..=30 => 6,                     // Extract 6 functions: 30/6 = 5 complexity each
        _ => (max_complexity / 5).max(6), // For very high complexity, aim for ~5 per function
    }
}

/// Generate combined testing and refactoring steps for complex functions with low coverage
fn generate_combined_testing_refactoring_steps(
    cyclomatic: u32,
    cognitive: u32,
    coverage_pct: i32,
) -> Vec<String> {
    let functions_to_extract = calculate_functions_to_extract(cyclomatic, cognitive);
    let target_complexity = (cyclomatic / functions_to_extract).max(3);
    let uncovered_branches = ((100 - coverage_pct) as f32 / 100.0 * cyclomatic as f32) as u32;

    vec![
        format!(
            "Currently ~{} of {} branches are uncovered ({}% coverage)",
            uncovered_branches, cyclomatic, coverage_pct
        ),
        format!(
            "Write {} tests to cover critical uncovered branches first",
            uncovered_branches.min(cyclomatic / 2)
        ),
        format!(
            "Extract {} pure functions from {} branches:",
            functions_to_extract, cyclomatic
        ),
        format!(
            "  • Group ~{} related branches per function",
            cyclomatic / functions_to_extract.max(1)
        ),
        format!(
            "  • Target complexity ≤{} per extracted function",
            target_complexity
        ),
        "Extraction patterns to look for:".to_string(),
        "  • Validation logic → validate_input()".to_string(),
        "  • Complex calculations → calculate_result()".to_string(),
        "  • Error handling → handle_errors()".to_string(),
        format!("Write ~{} tests per extracted function", target_complexity),
        "Add property-based tests for complex logic".to_string(),
        format!(
            "Final goal: {}+ functions with ≤{} complexity each, 80%+ coverage",
            functions_to_extract, target_complexity
        ),
    ]
}

/// Generate recommendation for testing gap debt type
/// Get display name for a function role
fn get_role_display_name(role: FunctionRole) -> &'static str {
    match role {
        FunctionRole::PureLogic => "Business logic",
        FunctionRole::Orchestrator => "Orchestration",
        FunctionRole::IOWrapper => "I/O wrapper",
        FunctionRole::EntryPoint => "Entry point",
        FunctionRole::PatternMatch => "Pattern matching",
        FunctionRole::Unknown => "Function",
    }
}

/// Calculate test cases needed based on complexity and current coverage
/// A more realistic estimate: not every branch needs a separate test case
fn calculate_needed_test_cases(cyclomatic: u32, coverage_pct: f64) -> u32 {
    if coverage_pct >= 1.0 {
        return 0;
    }

    // More realistic: sqrt of cyclomatic complexity + 2 for edge cases
    // This accounts for the fact that tests often cover multiple paths
    let ideal_test_cases = ((cyclomatic as f64).sqrt() * 1.5 + 2.0).ceil() as u32;

    let current_test_cases = if coverage_pct > 0.0 {
        (ideal_test_cases as f64 * coverage_pct).ceil() as u32
    } else {
        0
    };

    ideal_test_cases
        .saturating_sub(current_test_cases)
        .min(cyclomatic)
}

/// Calculate approximate test cases for simple functions
fn calculate_simple_test_cases(cyclomatic: u32, coverage_pct: f64) -> u32 {
    ((cyclomatic.max(2) as f64 * (1.0 - coverage_pct)).ceil() as u32).max(2)
}

/// Add uncovered lines recommendations to steps
fn add_uncovered_lines_to_steps(
    steps: &mut Vec<String>,
    func: &FunctionMetrics,
    transitive_coverage: &Option<TransitiveCoverage>,
) {
    if let Some(cov) = transitive_coverage {
        if !cov.uncovered_lines.is_empty() {
            let uncovered_recommendations = analyze_uncovered_lines(func, &cov.uncovered_lines);
            for (i, rec) in uncovered_recommendations.into_iter().enumerate() {
                steps.insert(i, rec);
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

/// Analyze uncovered lines to provide specific testing recommendations
fn analyze_uncovered_lines(func: &FunctionMetrics, uncovered_lines: &[usize]) -> Vec<String> {
    let mut recommendations = Vec::new();

    if uncovered_lines.is_empty() {
        return recommendations;
    }

    // Group consecutive lines into ranges for better readability
    let mut ranges = Vec::new();
    let mut current_start = uncovered_lines[0];
    let mut current_end = uncovered_lines[0];

    for &line in &uncovered_lines[1..] {
        if line == current_end + 1 {
            current_end = line;
        } else {
            ranges.push((current_start, current_end));
            current_start = line;
            current_end = line;
        }
    }
    ranges.push((current_start, current_end));

    // Format line ranges
    let range_strings: Vec<String> = ranges
        .iter()
        .take(5)
        .map(|(start, end)| {
            if start == end {
                format!("{}", start)
            } else {
                format!("{}-{}", start, end)
            }
        })
        .collect();

    let more = if ranges.len() > 5 {
        format!(" and {} more ranges", ranges.len() - 5)
    } else {
        String::new()
    };

    recommendations.push(format!(
        "Add tests for uncovered lines: {}{}",
        range_strings.join(", "),
        more
    ));

    // Provide specific guidance based on the function characteristics
    if func.cyclomatic > 5 {
        recommendations.push(format!(
            "Focus on testing {} decision points to cover all {} execution paths",
            func.cyclomatic - 1,
            func.cyclomatic
        ));
    }

    recommendations
}

fn generate_recommendation(
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

fn generate_recommendation_with_coverage_and_data_flow(
    func: &FunctionMetrics,
    debt_type: &DebtType,
    role: FunctionRole,
    _score: &UnifiedScore,
    coverage: &Option<TransitiveCoverage>,
    data_flow: Option<&crate::data_flow::DataFlowGraph>,
) -> ActionableRecommendation {
    // Create recommendation context using pure functions
    let recommendation_context =
        create_recommendation_context(func, debt_type, role, _score, coverage);

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
) -> RecommendationContext {
    RecommendationContext {
        function_info: FunctionInfo::from_metrics(func),
        debt_type: debt_type.clone(),
        role,
        score: score.clone(),
        coverage: coverage.clone(),
        is_rust_file: is_rust_file(&func.file),
        coverage_percent: extract_coverage_percent(coverage),
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

// Pure function to determine if file is Rust
fn is_rust_file(file_path: &std::path::Path) -> bool {
    file_path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e == "rs")
        .unwrap_or(false)
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
    generate_rust_refactoring_recommendation(&temp_item, cyclomatic, context.coverage_percent)
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
            responsibility_count,
            complexity_score,
        } => generate_god_object_recommendation(*responsibility_count, *complexity_score),
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

/// Determines if a function is considered complex based on its metrics
fn is_function_complex(cyclomatic: u32, cognitive: u32) -> bool {
    cyclomatic > 10 || cognitive > 15
}

/// Calculates the risk reduction factor based on debt type
fn calculate_risk_factor(debt_type: &DebtType) -> f64 {
    match debt_type {
        DebtType::TestingGap { .. } => 0.42,
        DebtType::ComplexityHotspot { .. } => 0.35,
        DebtType::ErrorSwallowing { .. } => 0.35, // High risk - can hide critical failures
        DebtType::DeadCode { .. } => 0.3,
        DebtType::Duplication { .. } => 0.25,
        DebtType::Risk { .. } => 0.2,
        DebtType::TestComplexityHotspot { .. } => 0.15,
        DebtType::TestTodo { .. } | DebtType::TestDuplication { .. } => 0.1,
        // Resource Management debt types (medium risk)
        DebtType::BlockingIO { .. } => 0.45,
        DebtType::NestedLoops { .. } => 0.4,
        DebtType::AllocationInefficiency { .. } => 0.3,
        DebtType::StringConcatenation { .. } => 0.25,
        DebtType::SuboptimalDataStructure { .. } => 0.2,
        // Organization debt types (maintenance risk)
        DebtType::GodObject { .. } => 0.4,
        DebtType::FeatureEnvy { .. } => 0.25,
        DebtType::PrimitiveObsession { .. } => 0.2,
        DebtType::MagicValues { .. } => 0.15,
        // Testing quality debt types (low risk)
        DebtType::FlakyTestPattern { .. } => 0.3,
        DebtType::AssertionComplexity { .. } => 0.15,
        // Resource management debt types (medium risk)
        DebtType::ResourceLeak { .. } => 0.5,
        DebtType::AsyncMisuse { .. } => 0.4,
        DebtType::CollectionInefficiency { .. } => 0.2,
    }
}

/// Calculates coverage improvement potential for testing gaps
fn calculate_coverage_improvement(coverage: f64, is_complex: bool) -> f64 {
    let potential = 1.0 - coverage;
    if is_complex {
        potential * 50.0 // 50% of potential due to complexity
    } else {
        potential * 100.0 // Full coverage potential for simple functions
    }
}

/// Calculates lines that could be reduced through refactoring
fn calculate_lines_reduction(debt_type: &DebtType) -> u32 {
    match debt_type {
        DebtType::DeadCode {
            cyclomatic,
            cognitive,
            ..
        } => *cyclomatic + *cognitive,
        DebtType::Duplication {
            instances,
            total_lines,
        }
        | DebtType::TestDuplication {
            instances,
            total_lines,
            ..
        } => *total_lines - (*total_lines / instances),
        _ => 0,
    }
}

/// Calculates complexity reduction potential based on debt type
fn calculate_complexity_reduction(debt_type: &DebtType, is_complex: bool) -> f64 {
    match debt_type {
        DebtType::DeadCode {
            cyclomatic,
            cognitive,
            ..
        } => (*cyclomatic + *cognitive) as f64 * 0.5,
        DebtType::TestingGap { cyclomatic, .. } if is_complex => *cyclomatic as f64 * 0.3,
        DebtType::ComplexityHotspot { cyclomatic, .. } => *cyclomatic as f64 * 0.5,
        DebtType::TestComplexityHotspot { cyclomatic, .. } => *cyclomatic as f64 * 0.3,
        // Organization debt types - significant complexity reduction potential
        DebtType::GodObject {
            complexity_score, ..
        } => *complexity_score * 0.4,
        DebtType::NestedLoops { depth, .. } => (*depth as f64).powf(2.0) * 0.3, // Quadratic impact
        DebtType::FeatureEnvy { .. } => 2.0, // Modest improvement
        _ => 0.0,
    }
}

fn calculate_expected_impact(
    _func: &FunctionMetrics,
    debt_type: &DebtType,
    score: &UnifiedScore,
) -> ImpactMetrics {
    let risk_factor = calculate_risk_factor(debt_type);
    let risk_reduction = score.final_score * risk_factor;

    let (coverage_improvement, lines_reduction, complexity_reduction) = match debt_type {
        DebtType::TestingGap {
            coverage,
            cyclomatic,
            cognitive,
        } => {
            let is_complex = is_function_complex(*cyclomatic, *cognitive);
            (
                calculate_coverage_improvement(*coverage, is_complex),
                0,
                calculate_complexity_reduction(debt_type, is_complex),
            )
        }
        _ => (
            0.0,
            calculate_lines_reduction(debt_type),
            calculate_complexity_reduction(debt_type, false),
        ),
    };

    ImpactMetrics {
        coverage_improvement,
        lines_reduction,
        complexity_reduction,
        risk_reduction,
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
        // When coverage is 0%, we use sqrt formula: sqrt(10) * 1.5 + 2 ≈ 7
        assert_eq!(calculate_needed_test_cases(10, 0.0), 7);
        // sqrt(25) * 1.5 + 2 = 5 * 1.5 + 2 = 9.5 ≈ 10
        assert_eq!(calculate_needed_test_cases(25, 0.0), 10);
    }

    #[test]
    fn test_has_testing_gap() {
        // Coverage below 20% and not a test = testing gap
        assert!(has_testing_gap(0.1, false));
        assert!(has_testing_gap(0.19, false));

        // Coverage at or above 20% = no testing gap
        assert!(!has_testing_gap(0.2, false));
        assert!(!has_testing_gap(0.5, false));

        // Test functions never have testing gaps
        assert!(!has_testing_gap(0.0, true));
        assert!(!has_testing_gap(0.1, true));
    }

    #[test]
    fn test_is_complexity_hotspot_by_metrics() {
        // High cyclomatic complexity
        assert!(is_complexity_hotspot_by_metrics(6, 3));
        assert!(is_complexity_hotspot_by_metrics(10, 5));

        // High cognitive complexity
        assert!(is_complexity_hotspot_by_metrics(3, 9));
        assert!(is_complexity_hotspot_by_metrics(4, 15));

        // Both low = not a hotspot
        assert!(!is_complexity_hotspot_by_metrics(5, 8));
        assert!(!is_complexity_hotspot_by_metrics(3, 5));
        assert!(!is_complexity_hotspot_by_metrics(0, 0));
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
    fn test_check_enhanced_complexity_hotspot() {
        let complex_func = FunctionMetrics {
            name: "complex_func".to_string(),
            file: std::path::PathBuf::from("src/complex.rs"),
            line: 1,
            length: 50,
            cyclomatic: 10,
            cognitive: 12,
            nesting: 3,
            visibility: Some("pub".to_string()),
            is_test: false,
            is_trait_method: false,
            in_test_module: false,
            entropy_score: None,
            is_pure: Some(false),
            purity_confidence: Some(0.5),
            detected_patterns: None,
        };

        let debt = check_enhanced_complexity_hotspot(&complex_func);
        assert!(debt.is_some());

        match debt.unwrap() {
            DebtType::ComplexityHotspot {
                cyclomatic,
                cognitive,
            } => {
                assert_eq!(cyclomatic, 10);
                assert_eq!(cognitive, 12);
            }
            _ => panic!("Expected ComplexityHotspot debt type"),
        }
    }

    #[test]
    fn test_check_enhanced_complexity_hotspot_simple() {
        let simple_func = FunctionMetrics {
            name: "simple_func".to_string(),
            file: std::path::PathBuf::from("src/simple.rs"),
            line: 1,
            length: 10,
            cyclomatic: 2,
            cognitive: 3,
            nesting: 1,
            visibility: Some("pub".to_string()),
            is_test: false,
            is_trait_method: false,
            in_test_module: false,
            entropy_score: None,
            is_pure: Some(true),
            purity_confidence: Some(0.9),
            detected_patterns: None,
        };

        let debt = check_enhanced_complexity_hotspot(&simple_func);
        assert!(debt.is_none());
    }

    #[test]
    fn test_calculate_needed_test_cases_partial_coverage() {
        // When 50% covered, ideal is 7, current is 3.5 ≈ 4, needed is 3
        assert_eq!(calculate_needed_test_cases(10, 0.5), 3);
        // When 80% covered, ideal is 7, current is 5.6 ≈ 6, needed is 1
        assert_eq!(calculate_needed_test_cases(10, 0.8), 1);
        // When 25% covered, ideal is 7, current is 1.75 ≈ 2, needed is 5
        assert_eq!(calculate_needed_test_cases(10, 0.25), 5);
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
        );

        assert_eq!(context.function_info.name, "test_func");
        assert_eq!(context.function_info.line, 10);
        assert!(context.is_rust_file);
        assert_eq!(context.coverage_percent, 0.6);
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
