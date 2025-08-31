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

pub fn create_unified_debt_item_with_aggregator_and_data_flow(
    func: &FunctionMetrics,
    call_graph: &CallGraph,
    coverage: Option<&LcovData>,
    framework_exclusions: &HashSet<FunctionId>,
    function_pointer_used_functions: Option<&HashSet<FunctionId>>,
    debt_aggregator: &DebtAggregator,
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

    // Calculate unified score with debt aggregator
    let unified_score = calculate_unified_priority_with_debt(
        func,
        call_graph,
        coverage,
        None, // Let the aggregator provide organization factor
        Some(debt_aggregator),
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
    }
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
    }
}

// Helper functions

/// Helper function to calculate entropy details from FunctionMetrics
fn calculate_entropy_details(func: &FunctionMetrics) -> Option<EntropyDetails> {
    func.entropy_score.as_ref().map(|entropy_score| {
        let adjusted_cyclomatic =
            crate::complexity::entropy::apply_entropy_dampening(func.cyclomatic, entropy_score);
        let _adjusted_cognitive =
            crate::complexity::entropy::apply_entropy_dampening(func.cognitive, entropy_score);
        let dampening_factor = if func.cyclomatic > 0 {
            adjusted_cyclomatic as f64 / func.cyclomatic as f64
        } else {
            1.0
        };

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
    // Determine primary debt type based on metrics
    if let Some(cov) = coverage {
        // Any untested function (< 20% coverage) that isn't a test itself is a testing gap
        // Even simple functions need basic tests
        if cov.direct < 0.2 && !func.is_test {
            return DebtType::TestingGap {
                coverage: cov.direct,
                cyclomatic: func.cyclomatic,
                cognitive: func.cognitive,
            };
        }
    }

    if func.cyclomatic > 10 || func.cognitive > 15 {
        return DebtType::ComplexityHotspot {
            cyclomatic: func.cyclomatic,
            cognitive: func.cognitive,
        };
    }

    // Check for dead code before falling back to generic risk
    if is_dead_code(func, call_graph, func_id, None) {
        return DebtType::DeadCode {
            visibility: determine_visibility(func),
            cyclomatic: func.cyclomatic,
            cognitive: func.cognitive,
            usage_hints: generate_usage_hints(func, call_graph, func_id),
        };
    }

    // Get role for later checks
    let role = classify_function_role(func, func_id, call_graph);

    // Low complexity functions that are I/O wrappers or entry points
    // should not be flagged as technical debt
    if func.cyclomatic <= 3 && func.cognitive <= 5 {
        // Check if it's an I/O wrapper or entry point
        if role == FunctionRole::IOWrapper
            || role == FunctionRole::EntryPoint
            || role == FunctionRole::PatternMatch
        {
            // These are acceptable patterns, not debt
            return DebtType::Risk {
                risk_score: 0.0,
                factors: vec!["Simple I/O wrapper or entry point - minimal risk".to_string()],
            };
        }

        // Pure logic functions that are very simple are not debt
        if role == FunctionRole::PureLogic && func.length <= 10 {
            // Simple pure functions like formatters don't need to be flagged
            // Return minimal risk to indicate no real debt
            return DebtType::Risk {
                risk_score: 0.0,
                factors: vec!["Trivial pure function - not technical debt".to_string()],
            };
        }
    }

    // Only flag as risk-based debt if there's actual complexity or other indicators
    if func.cyclomatic > 5 || func.cognitive > 8 || func.length > 50 {
        DebtType::Risk {
            risk_score: calculate_risk_score(func),
            factors: identify_risk_factors(func, coverage),
        }
    } else {
        // Simple functions with cyclomatic <= 5 and cognitive <= 8 and length <= 50
        // Simple functions are not debt in themselves
        if role == FunctionRole::PureLogic {
            // Simple pure functions are not debt - return minimal risk
            DebtType::Risk {
                risk_score: 0.0,
                factors: vec!["Simple pure function - minimal risk".to_string()],
            }
        } else {
            // Other simple functions - minimal risk
            DebtType::Risk {
                risk_score: 0.1,
                factors: vec!["Simple function with low complexity".to_string()],
            }
        }
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
    // Test functions are special debt cases
    if func.is_test {
        return classify_test_debt(func);
    }

    // Check for testing gaps first (like in determine_debt_type)
    if let Some(cov) = coverage {
        if has_testing_gap(cov.direct, func.is_test) {
            return DebtType::TestingGap {
                coverage: cov.direct,
                cyclomatic: func.cyclomatic,
                cognitive: func.cognitive,
            };
        }
    }

    // Check for complexity hotspots - include moderate complexity functions
    if is_complexity_hotspot_by_metrics(func.cyclomatic, func.cognitive) {
        return DebtType::ComplexityHotspot {
            cyclomatic: func.cyclomatic,
            cognitive: func.cognitive,
        };
    }

    // Check for dead code with framework exclusions
    if is_dead_code_with_exclusions(
        func,
        call_graph,
        func_id,
        framework_exclusions,
        function_pointer_used_functions,
    ) {
        return DebtType::DeadCode {
            visibility: determine_visibility(func),
            cyclomatic: func.cyclomatic,
            cognitive: func.cognitive,
            usage_hints: generate_usage_hints(func, call_graph, func_id),
        };
    }

    // Get role for later checks
    let role = classify_function_role(func, func_id, call_graph);

    // Low complexity functions that are I/O wrappers or entry points
    // should not be flagged as technical debt
    if func.cyclomatic <= 3 && func.cognitive <= 5 {
        if let Some(debt) = classify_simple_function_risk(func, &role) {
            return debt;
        }
    }

    // At this point, we have simple functions (cyclo <= 5, cog <= 8)
    // These are not technical debt - return minimal risk
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
fn generate_testing_gap_recommendation(
    coverage_pct: f64,
    cyclomatic: u32,
    cognitive: u32,
    role: FunctionRole,
    func: &FunctionMetrics,
    transitive_coverage: &Option<TransitiveCoverage>,
) -> (String, String, Vec<String>) {
    let is_complex = cyclomatic > 10 || cognitive > 15;
    let coverage_pct_int = (coverage_pct * 100.0) as i32;
    let role_str = format_role_description(role);
    let coverage_gap = 100 - coverage_pct_int;

    // If function is fully covered, no testing gap exists
    if coverage_gap == 0 {
        let role_display = match role {
            FunctionRole::PureLogic => "Business logic",
            FunctionRole::Orchestrator => "Orchestration",
            FunctionRole::IOWrapper => "I/O wrapper",
            FunctionRole::EntryPoint => "Entry point",
            FunctionRole::PatternMatch => "Pattern matching",
            FunctionRole::Unknown => "Function",
        };

        return (
            "Maintain test coverage".to_string(),
            format!("{} function is currently 100% covered", role_display),
            vec![
                "Keep tests up to date with code changes".to_string(),
                "Consider property-based testing for edge cases".to_string(),
                "Monitor coverage in CI/CD pipeline".to_string(),
            ],
        );
    }

    if is_complex {
        let functions_to_extract = calculate_functions_to_extract(cyclomatic, cognitive);

        // Calculate test cases needed
        let current_test_cases = if coverage_pct_int > 0 {
            (cyclomatic as f64 * coverage_pct).ceil() as u32
        } else {
            0
        };
        let needed_test_cases = cyclomatic.saturating_sub(current_test_cases);

        // Explain why both testing and refactoring are needed
        let complexity_explanation = format!(
            "Cyclomatic complexity of {} requires at least {} test cases for full path coverage. After extracting {} functions, each will need only 3-5 tests",
            cyclomatic, cyclomatic, functions_to_extract
        );

        // Add uncovered lines info if available
        let mut steps =
            generate_combined_testing_refactoring_steps(cyclomatic, cognitive, coverage_pct_int);
        if let Some(cov) = transitive_coverage {
            if !cov.uncovered_lines.is_empty() {
                let uncovered_recommendations = analyze_uncovered_lines(func, &cov.uncovered_lines);
                // Insert uncovered lines info at the beginning of steps
                for (i, rec) in uncovered_recommendations.into_iter().enumerate() {
                    steps.insert(i, rec);
                }
            }
        }

        (
            format!("Add {} tests for {}% coverage gap, then refactor complexity {} into {} functions", 
                   needed_test_cases, coverage_gap, cyclomatic, functions_to_extract),
            format!("Complex {role_str} with {coverage_gap}% gap. {}. Testing before refactoring ensures no regressions",
                   complexity_explanation),
            steps,
        )
    } else {
        let role_display = match role {
            FunctionRole::PureLogic => "Business logic",
            FunctionRole::Orchestrator => "Orchestration",
            FunctionRole::IOWrapper => "I/O wrapper",
            FunctionRole::EntryPoint => "Entry point",
            FunctionRole::PatternMatch => "Pattern matching",
            FunctionRole::Unknown => "Function",
        };

        // Calculate approximate test cases needed (minimum 2 for basic happy/error paths)
        let test_cases_needed =
            ((cyclomatic.max(2) as f64 * (1.0 - coverage_pct)).ceil() as u32).max(2);

        let coverage_explanation = if coverage_pct_int == 0 {
            format!("{role_display} with {coverage_gap}% coverage gap, currently {coverage_pct_int}% covered. Needs {} test cases to cover all {} execution paths",
                   test_cases_needed, cyclomatic.max(2))
        } else {
            format!("{role_display} with {coverage_gap}% coverage gap, currently {coverage_pct_int}% covered. Needs {} more test cases",
                   test_cases_needed)
        };

        // Add uncovered lines info if available
        let mut steps = generate_testing_gap_steps(false);
        if let Some(cov) = transitive_coverage {
            if !cov.uncovered_lines.is_empty() {
                let uncovered_recommendations = analyze_uncovered_lines(func, &cov.uncovered_lines);
                // Insert uncovered lines info at the beginning of steps
                for (i, rec) in uncovered_recommendations.into_iter().enumerate() {
                    steps.insert(i, rec);
                }
            }
        }

        (
            format!(
                "Add {} tests for {}% coverage gap",
                test_cases_needed, coverage_gap
            ),
            coverage_explanation,
            steps,
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
    let (primary_action, rationale, steps) = match debt_type {
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
    };

    ActionableRecommendation {
        primary_action,
        rationale,
        implementation_steps: steps,
        related_items: vec![],
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
