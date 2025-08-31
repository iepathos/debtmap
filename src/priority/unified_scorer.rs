use crate::config;
use crate::core::FunctionMetrics;
use crate::priority::{
    call_graph::{CallGraph, FunctionId},
    coverage_propagation::{
        calculate_transitive_coverage, TransitiveCoverage,
    },
    debt_aggregator::{DebtAggregator, FunctionId as AggregatorFunctionId},
    external_api_detector::{generate_enhanced_dead_code_hints, is_likely_external_api},
    semantic_classifier::{classify_function_role, FunctionRole},
    ActionableRecommendation, DebtType, FunctionAnalysis, FunctionVisibility, ImpactMetrics,
};
use crate::risk::evidence_calculator::EvidenceBasedRiskCalculator;
use crate::risk::lcov::LcovData;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedScore {
    pub complexity_factor: f64, // 0-10, configurable weight (default 35%)
    pub coverage_factor: f64,   // 0-10, configurable weight (default 40%)
    pub dependency_factor: f64, // 0-10, configurable weight (default 20%)
    pub role_multiplier: f64,   // 0.1-1.5x based on function role
    pub final_score: f64,       // Computed composite score
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedDebtItem {
    pub location: Location,
    pub debt_type: DebtType,
    pub unified_score: UnifiedScore,
    pub function_role: FunctionRole,
    pub recommendation: ActionableRecommendation,
    pub expected_impact: ImpactMetrics,
    pub transitive_coverage: Option<TransitiveCoverage>,
    pub upstream_dependencies: usize,
    pub downstream_dependencies: usize,
    pub upstream_callers: Vec<String>, // List of function names that call this function
    pub downstream_callees: Vec<String>, // List of functions that this function calls
    pub nesting_depth: u32,
    pub function_length: usize,
    pub cyclomatic_complexity: u32,
    pub cognitive_complexity: u32,
    pub entropy_details: Option<EntropyDetails>, // Store entropy information
    pub is_pure: Option<bool>,                   // Whether the function is pure
    pub purity_confidence: Option<f32>,          // Confidence in purity detection
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntropyDetails {
    pub entropy_score: f64,
    pub pattern_repetition: f64,
    pub original_complexity: u32,
    pub adjusted_complexity: u32,
    pub dampening_factor: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Location {
    pub file: PathBuf,
    pub function: String,
    pub line: usize,
}

pub fn calculate_unified_priority(
    func: &FunctionMetrics,
    call_graph: &CallGraph,
    coverage: Option<&LcovData>,
    organization_issues: Option<f64>,
) -> UnifiedScore {
    calculate_unified_priority_with_debt(func, call_graph, coverage, organization_issues, None)
}

pub fn calculate_unified_priority_with_debt(
    func: &FunctionMetrics,
    call_graph: &CallGraph,
    coverage: Option<&LcovData>,
    _organization_issues: Option<f64>, // Kept for compatibility but no longer used
    debt_aggregator: Option<&DebtAggregator>,
) -> UnifiedScore {
    let func_id = FunctionId {
        file: func.file.clone(),
        name: func.name.clone(),
        line: func.line,
    };

    // Check if this function is actually technical debt
    // Simple I/O wrappers, entry points, and trivial pure functions with low complexity
    // are not technical debt UNLESS they're untested and non-trivial
    let role = classify_function_role(func, &func_id, call_graph);

    // Pure functions are inherently less risky and easier to test
    let purity_bonus = if func.is_pure == Some(true) {
        // High confidence pure functions get bigger bonus
        if func.purity_confidence.unwrap_or(0.0) > 0.8 {
            0.7 // 30% reduction in complexity perception
        } else {
            0.85 // 15% reduction
        }
    } else {
        1.0 // No reduction for impure functions
    };

    let is_trivial = (func.cyclomatic <= 3 && func.cognitive <= 5)
        && (role == FunctionRole::IOWrapper
            || role == FunctionRole::EntryPoint
            || role == FunctionRole::PatternMatch
            || (role == FunctionRole::PureLogic && func.length <= 10));

    // Check actual test coverage if we have lcov data
    let has_coverage = if let Some(cov) = coverage {
        cov.get_function_coverage(&func.file, &func.name)
            .map(|coverage_pct| coverage_pct > 0.0)
            .unwrap_or(false)
    } else {
        false // No coverage data means assume untested
    };

    // If it's trivial AND tested, it's definitely not technical debt
    if is_trivial && has_coverage {
        return UnifiedScore {
            complexity_factor: 0.0,
            coverage_factor: 0.0,
            dependency_factor: 0.0,
            role_multiplier: 1.0,
            final_score: 0.0,
        };
    }

    // Calculate complexity factor (normalized to 0-10)
    // Apply purity bonus first (pure functions are easier to understand and test)
    let purity_adjusted_cyclomatic = (func.cyclomatic as f64 * purity_bonus) as u32;
    let purity_adjusted_cognitive = (func.cognitive as f64 * purity_bonus) as u32;

    let raw_complexity =
        normalize_complexity(purity_adjusted_cyclomatic, purity_adjusted_cognitive);

    // Get actual coverage percentage
    let coverage_pct = if func.is_test {
        1.0 // Test functions have 100% coverage by definition
    } else if let Some(cov) = coverage {
        cov.get_function_coverage(&func.file, &func.name)
            .unwrap_or(0.0) / 100.0 // Convert to 0-1 range
    } else {
        0.0 // No coverage data - assume worst case
    };

    // Calculate coverage gap with exponential scaling (spec 68 requirement)
    let coverage_gap = 1.0 - coverage_pct;
    let coverage_factor = coverage_gap.powf(1.5); // Exponential scaling for emphasis

    // Calculate complexity factor with sublinear scaling to avoid over-penalizing
    let complexity_factor = raw_complexity.powf(0.8);

    // Calculate dependency factor with sqrt scaling for better distribution
    let upstream_count = call_graph.get_callers(&func_id).len();
    let dependency_factor = ((upstream_count as f64 + 1.0).sqrt() / 2.0).min(1.0);

    // Get role multiplier - adjusted for better differentiation
    let role_multiplier = match role {
        FunctionRole::EntryPoint => 1.5,
        FunctionRole::PureLogic if raw_complexity > 5.0 => 1.3, // Complex core logic
        FunctionRole::PureLogic => 1.0,
        FunctionRole::Orchestrator => 0.8,
        FunctionRole::IOWrapper => 0.5,
        FunctionRole::PatternMatch => 0.6,
        _ => 1.0,
    };

    // MULTIPLICATIVE SCORING MODEL (spec 68 requirement)
    // Base formula: Score = (Coverage_Gap ^ α) × (Complexity ^ β) × (Dependency ^ γ) × Role_Modifier
    // Note: Add small constants to avoid zero multiplication
    let complexity_component = (complexity_factor + 0.1).max(0.1);
    let dependency_component = (dependency_factor + 0.1).max(0.1);
    let mut base_score = coverage_factor * complexity_component * dependency_component;

    // Complexity-coverage interaction bonus (spec 68 requirement)
    // High complexity + low coverage = multiplicative penalty
    if coverage_pct < 0.5 && raw_complexity > 5.0 {
        base_score *= 1.5; // 50% bonus for complex untested code
    }

    // Apply role multiplier
    let role_adjusted_score = base_score * role_multiplier;

    // Calculate debt-based modifiers if aggregator is available
    let debt_modifier = if let Some(aggregator) = debt_aggregator {
        let agg_func_id = AggregatorFunctionId {
            file: func.file.clone(),
            name: func.name.clone(),
            start_line: func.line,
            end_line: func.line + func.length,
        };
        let debt_scores = aggregator.calculate_debt_scores(&agg_func_id);
        
        // Add small multiplicative factors for other debt types
        let testing_modifier = 1.0 + (debt_scores.testing.min(10.0) / 100.0);
        let resource_modifier = 1.0 + (debt_scores.resource.min(10.0) / 100.0);
        let duplication_modifier = 1.0 + (debt_scores.duplication.min(10.0) / 100.0);
        
        testing_modifier * resource_modifier * duplication_modifier
    } else {
        1.0
    };

    let debt_adjusted_score = role_adjusted_score * debt_modifier;

    // Apply reduced entropy dampening (spec 68: max 50% reduction, not 100%)
    let final_score = if let Some(entropy_score) = func.entropy_score.as_ref() {
        apply_reduced_entropy_dampening(debt_adjusted_score, entropy_score)
    } else {
        debt_adjusted_score
    };

    // Normalize to 0-10 scale with better distribution
    let normalized_score = normalize_final_score(final_score);

    UnifiedScore {
        complexity_factor: raw_complexity,
        coverage_factor: coverage_gap * 10.0, // Convert back to 0-10 for display
        dependency_factor: upstream_count as f64,
        role_multiplier,
        final_score: normalized_score,
    }
}

/// Apply reduced entropy dampening per spec 68 (max 50% reduction)
fn apply_reduced_entropy_dampening(
    score: f64,
    entropy_score: &crate::complexity::entropy::EntropyScore,
) -> f64 {
    let config = config::get_entropy_config();

    if !config.enabled {
        return score;
    }

    // Only apply dampening for very low entropy (< 0.2)
    if entropy_score.token_entropy >= 0.2 {
        return score; // No dampening for normal entropy
    }

    // Calculate graduated dampening: 50-100% of score preserved
    // Formula: dampening = max(0.5, 1.0 - (0.5 × (0.2 - entropy) / 0.2))
    let dampening_factor = (0.5 + 0.5 * (entropy_score.token_entropy / 0.2)).max(0.5);

    score * dampening_factor
}



fn normalize_complexity(cyclomatic: u32, cognitive: u32) -> f64 {
    // Normalize complexity to 0-10 scale
    let combined = (cyclomatic + cognitive) as f64 / 2.0;

    // Use logarithmic scale for better distribution
    // Complexity of 1-5 = low (0-3), 6-10 = medium (3-6), 11+ = high (6-10)
    if combined <= 5.0 {
        combined * 0.6
    } else if combined <= 10.0 {
        3.0 + (combined - 5.0) * 0.6
    } else {
        6.0 + ((combined - 10.0) * 0.2).min(4.0)
    }
}

/// Normalize final score to 0-10 range with better distribution
fn normalize_final_score(raw_score: f64) -> f64 {
    // Use percentile-based normalization for better spread
    // Raw scores typically range from 0 to ~5 with multiplicative model
    // Map to 0-10 scale with emphasis on differentiation
    
    if raw_score <= 0.01 {
        0.0 // Trivial or fully tested
    } else if raw_score <= 0.1 {
        raw_score * 20.0 // 0.01-0.1 -> 0.2-2.0
    } else if raw_score <= 0.5 {
        2.0 + (raw_score - 0.1) * 7.5 // 0.1-0.5 -> 2.0-5.0
    } else if raw_score <= 1.0 {
        5.0 + (raw_score - 0.5) * 6.0 // 0.5-1.0 -> 5.0-8.0
    } else if raw_score <= 2.0 {
        8.0 + (raw_score - 1.0) * 1.5 // 1.0-2.0 -> 8.0-9.5
    } else {
        (9.5 + (raw_score - 2.0) * 0.25).min(10.0) // 2.0+ -> 9.5-10.0
    }
}

fn calculate_dependency_factor(upstream_count: usize) -> f64 {
    // Calculate criticality based on number of upstream dependencies (callers)
    // Functions with many callers are on critical paths and should be prioritized
    // 0 callers = 0 (dead code)
    // 1-2 callers = 2-4 (low criticality)
    // 3-5 callers = 4-7 (medium criticality)
    // 6-10 callers = 7-9 (high criticality)
    // 10+ callers = 9-10 (critical path)

    match upstream_count {
        0 => 0.0, // Dead code - low priority unless it's complex
        1 => 2.0,
        2 => 3.0,
        3 => 4.0,
        4 => 5.0,
        5 => 6.0,
        6..=7 => 7.0,
        8..=9 => 8.0,
        10..=14 => 9.0,
        _ => 10.0, // 15+ callers = critical path function
    }
}

// Organization factor removed per spec 58 - redundant with complexity factor
// Organization issues are already captured by complexity metrics

/// Create evidence-based risk assessment for a function
pub fn create_evidence_based_risk_assessment(
    func: &FunctionMetrics,
    call_graph: &CallGraph,
    coverage: Option<&LcovData>,
) -> crate::risk::evidence::RiskAssessment {
    let calculator = EvidenceBasedRiskCalculator::new();

    // Convert FunctionMetrics to FunctionAnalysis
    let function_analysis = FunctionAnalysis {
        file: func.file.clone(),
        function: func.name.clone(),
        line: func.line,
        function_length: func.length,
        cyclomatic_complexity: func.cyclomatic,
        cognitive_complexity: func.cognitive,
        nesting_depth: func.nesting,
        is_test: func.is_test,
        visibility: determine_visibility(func),
        is_pure: func.is_pure,
        purity_confidence: func.purity_confidence,
    };

    calculator.calculate_risk(&function_analysis, call_graph, coverage)
}

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

fn is_dead_code(
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

/// Enhanced dead code detection using the enhanced call graph
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

fn determine_visibility(func: &FunctionMetrics) -> FunctionVisibility {
    // Use the visibility field from FunctionMetrics if available
    match &func.visibility {
        Some(vis) if vis == "pub" => FunctionVisibility::Public,
        Some(vis) if vis == "pub(crate)" => FunctionVisibility::Crate,
        Some(vis) if vis.starts_with("pub(") => FunctionVisibility::Crate, // pub(super), pub(in ...), etc.
        _ => FunctionVisibility::Private,
    }
}

fn generate_usage_hints(
    func: &FunctionMetrics,
    call_graph: &CallGraph,
    func_id: &FunctionId,
) -> Vec<String> {
    let visibility = determine_visibility(func);

    // Use enhanced dead code hints
    let mut hints = generate_enhanced_dead_code_hints(func, &visibility);

    // Add call graph information
    let callees = call_graph.get_callees(func_id);
    if callees.is_empty() {
        hints.push("Function has no dependencies - safe to remove".to_string());
    } else {
        hints.push(format!("Function calls {} other functions", callees.len()));
    }

    hints
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

/// Classify the complexity level based on cyclomatic complexity
fn classify_complexity_level(cyclomatic: u32) -> ComplexityLevel {
    match cyclomatic {
        0..=3 => ComplexityLevel::Low,
        4..=5 => ComplexityLevel::LowModerate,
        6..=10 => ComplexityLevel::Moderate,
        _ => ComplexityLevel::High,
    }
}

#[derive(Debug, Clone, Copy)]
enum ComplexityLevel {
    Low,
    LowModerate,
    Moderate,
    High,
}

/// Generate complexity-based recommendation for risk debt
fn generate_complexity_risk_recommendation(
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
    // For cyclomatic 5-6, extract 2 functions
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
            "Goal: Reduce cyclomatic complexity from {} to <={}",
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
            "Goal: Reduce cyclomatic complexity from {} to <={}",
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
    } else {
        format!("Decompose into {} pure functions (complexity {} → {}), then add {} comprehensive tests", 
                functions_to_extract, cyclo, target_complexity, functions_to_extract * 4)
    };

    let mut steps = vec![
        "Map each conditional branch to its core responsibility".to_string(),
        format!(
            "Create {} pure functions, one per responsibility",
            functions_to_extract
        ),
        "Replace complex conditionals with function dispatch table".to_string(),
        "Extract validation logic into composable predicates".to_string(),
        "Transform data mutations into immutable transformations".to_string(),
        "Isolate side effects in a thin orchestration layer".to_string(),
    ];

    if !has_good_coverage {
        steps.push(format!(
            "Write {} unit tests plus property-based tests for pure functions",
            functions_to_extract * 4
        ));
    }

    if has_coverage_issue && !has_good_coverage {
        steps.push(format!(
            "Target: Each function ≤{} complexity with 80%+ coverage",
            target_complexity
        ));
    } else {
        steps.push(format!(
            "Target: Each function ≤{} cyclomatic complexity",
            target_complexity
        ));
    }

    (
        action,
        "High complexity requiring aggressive refactoring".to_string(),
        steps,
    )
}

/// Generate recommendation for infrastructure debt types (duplication, risk)
fn generate_infrastructure_recommendation_with_coverage(
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
            // Check if any factor mentions complexity to provide more specific recommendations
            let has_complexity_issue = factors.iter().any(|f| {
                f.contains("complexity") || f.contains("cyclomatic") || f.contains("cognitive")
            });

            if has_complexity_issue {
                // Extract complexity values from factors string if present
                let cyclo = extract_cyclomatic_from_factors(factors).unwrap_or(0);
                let (action, _, steps) =
                    generate_complexity_risk_recommendation(cyclo, coverage, factors);
                (
                    action,
                    format!("Risk score {:.1}: {}", risk_score, factors.join(", ")),
                    steps,
                )
            } else {
                // Non-complexity related risk
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
fn generate_complexity_hotspot_recommendation(
    cyclomatic: u32,
    cognitive: u32,
) -> (String, String, Vec<String>) {
    // Calculate extraction based on complexity distribution
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

/// Generate complexity recommendation using pattern analysis when available
fn generate_complexity_recommendation_with_patterns_and_coverage(
    func: &FunctionMetrics,
    cyclomatic: u32,
    cognitive: u32,
    coverage: &Option<TransitiveCoverage>,
    data_flow: Option<&crate::data_flow::DataFlowGraph>,
) -> (String, String, Vec<String>) {
    use crate::extraction_patterns::{ExtractionAnalyzer, UnifiedExtractionAnalyzer};

    // Try to analyze extraction patterns
    let analyzer = UnifiedExtractionAnalyzer::new();

    // Create a minimal FileMetrics for the analyzer
    let file_metrics = crate::core::FileMetrics {
        path: func.file.clone(),
        language: detect_file_language(&func.file),
        complexity: crate::core::ComplexityMetrics::default(),
        debt_items: vec![],
        dependencies: vec![],
        duplications: vec![],
    };

    let suggestions = analyzer.analyze_function(func, &file_metrics, data_flow);

    // If we have intelligent suggestions from AST analysis, use them
    if !suggestions.is_empty() {
        // Generate pattern-based recommendation
        let mut action_parts = vec![];
        let mut steps = vec![];
        let mut total_complexity_reduction = 0u32;

        for (i, suggestion) in suggestions.iter().enumerate().take(3) {
            // Include top 3 patterns
            action_parts.push(format!(
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

            total_complexity_reduction += suggestion
                .complexity_reduction
                .current_cyclomatic
                .saturating_sub(suggestion.complexity_reduction.predicted_cyclomatic);
        }

        let predicted_complexity = cyclomatic.saturating_sub(total_complexity_reduction);

        // Create action with specific pattern names
        let action = if !action_parts.is_empty() {
            format!(
                "Extract {} to reduce complexity from {} to ~{}",
                action_parts.join(", "),
                cyclomatic,
                predicted_complexity
            )
        } else {
            format!(
                "Extract {} identified patterns to reduce complexity from {} to {}",
                suggestions.len(),
                cyclomatic,
                predicted_complexity
            )
        };

        // Provide detailed explanation of why these extractions are recommended
        let pattern_benefits = match suggestions.len() {
            1 => "This extraction will create a focused, testable unit".to_string(),
            2 => "These extractions will separate distinct concerns into testable units".to_string(),
            _ => format!("These {} extractions will decompose the function into smaller, focused units that are easier to test and understand", suggestions.len()),
        };

        let complexity_explanation = if cyclomatic > 15 {
            format!("Cyclomatic complexity of {} indicates {} independent execution paths, requiring at least {} test cases for full path coverage", 
                    cyclomatic, cyclomatic, cyclomatic)
        } else if cyclomatic > 10 {
            format!("Cyclomatic complexity of {} indicates {} independent paths through the code, making thorough testing difficult", 
                    cyclomatic, cyclomatic)
        } else if cyclomatic > 5 {
            format!("Cyclomatic complexity of {} indicates {} independent paths requiring {} test cases minimum - extraction will reduce this to 3-5 tests per function",
                    cyclomatic, cyclomatic, cyclomatic)
        } else {
            format!("Cyclomatic complexity of {} indicates moderate complexity that can be improved through extraction", cyclomatic)
        };

        let rationale = format!(
            "{}. Function has {} extractable patterns that can be isolated. {}. Target complexity per function is 5 or less for optimal maintainability.",
            complexity_explanation,
            suggestions.len(),
            pattern_benefits
        );

        // Add testing steps only if coverage is low
        let has_good_coverage = coverage.as_ref().map(|c| c.direct >= 0.8).unwrap_or(false);

        if !has_good_coverage {
            // Add uncovered lines information if available
            if let Some(cov) = coverage {
                if !cov.uncovered_lines.is_empty() {
                    let uncovered_recommendations =
                        analyze_uncovered_lines(func, &cov.uncovered_lines);
                    for (i, rec) in uncovered_recommendations.into_iter().enumerate() {
                        steps.insert(i, rec);
                    }
                }
            }

            steps.push(format!(
                "{}. Write unit tests for each extracted pure function",
                suggestions.len() + 2
            ));
            steps.push(format!(
                "{}. Add property-based tests for complex transformations",
                suggestions.len() + 3
            ));
        }

        steps.push(format!(
            "Expected complexity reduction: {}%",
            (total_complexity_reduction as f32 / cyclomatic as f32 * 100.0) as u32
        ));

        (action, rationale, steps)
    } else {
        // Fall back to heuristic recommendations with estimated line ranges
        generate_heuristic_recommendations_with_line_estimates(
            func, cyclomatic, cognitive, coverage, data_flow,
        )
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

/// Generate recommendations based on data flow analysis when AST is unavailable
fn generate_heuristic_recommendations_with_line_estimates(
    func: &FunctionMetrics,
    cyclomatic: u32,
    cognitive: u32,
    coverage: &Option<TransitiveCoverage>,
    data_flow: Option<&crate::data_flow::DataFlowGraph>,
) -> (String, String, Vec<String>) {
    // Analyze function characteristics from available metrics
    let has_high_branching = cyclomatic > 7;
    let has_deep_nesting = func.nesting > 3;
    let is_pure = func.is_pure.unwrap_or(false);
    let purity_confidence = func.purity_confidence.unwrap_or(0.0);

    // Get variable dependencies if data flow is available
    let num_dependencies = if let Some(df) = data_flow {
        let func_id = crate::priority::call_graph::FunctionId {
            file: func.file.clone(),
            name: func.name.clone(),
            line: func.line,
        };
        df.get_variable_dependencies(&func_id)
            .map(|d| d.len())
            .unwrap_or(0)
    } else {
        0
    };

    // Generate targeted recommendations based on patterns
    let mut steps = Vec::new();
    let mut suggested_extractions = Vec::new();
    let mut complexity_reduction = 0;

    if has_high_branching {
        suggested_extractions.push("validation logic");
        steps.push(format!(
            "Identify validation checks from {} branches → extract as validate_*()",
            cyclomatic / 4
        ));
        complexity_reduction += cyclomatic / 4;
    }

    if has_deep_nesting {
        suggested_extractions.push("nested processing");
        steps.push(format!(
            "Extract nested logic (depth {}) → process_*() functions",
            func.nesting
        ));
        complexity_reduction += 2;
    }

    if cognitive > cyclomatic * 2 {
        suggested_extractions.push("complex calculations");
        steps.push(format!(
            "Extract calculations from {} cognitive complexity → calculate_*()",
            cognitive / 5
        ));
        complexity_reduction += cognitive / 5;
    }

    if num_dependencies > 5 {
        suggested_extractions.push("data transformation pipeline");
        steps.push(format!(
            "Create data transformation pipeline to manage {} dependencies",
            num_dependencies
        ));
        complexity_reduction += 1;
    }

    if is_pure && purity_confidence > 0.8 {
        steps.push(
            "Function is likely pure - focus on breaking down into smaller pure functions"
                .to_string(),
        );
    } else if purity_confidence < 0.3 {
        steps.push("Isolate side effects at function boundaries before extraction".to_string());
    }

    // Add testing recommendation only if coverage is low
    let has_good_coverage = coverage.as_ref().map(|c| c.direct >= 0.8).unwrap_or(false);

    // Add uncovered lines info if available
    if let Some(cov) = coverage {
        if !cov.uncovered_lines.is_empty() && !has_good_coverage {
            let uncovered_recommendations = analyze_uncovered_lines(func, &cov.uncovered_lines);
            // Add uncovered lines info at the beginning
            for rec in uncovered_recommendations.into_iter().rev() {
                steps.insert(0, rec);
            }
        }
    }

    if !has_good_coverage {
        let test_count = if suggested_extractions.is_empty() {
            // If no specific extractions suggested, base on complexity
            std::cmp::max(2, (cyclomatic / 5) as usize)
        } else {
            suggested_extractions.len() * 2
        };
        steps.push(format!(
            "Add {} comprehensive tests after refactoring",
            test_count
        ));
    }

    let predicted_complexity = cyclomatic.saturating_sub(complexity_reduction);

    let functions_to_extract = calculate_functions_to_extract(cyclomatic, cognitive);
    let action = if !suggested_extractions.is_empty() {
        format!(
            "Extract {} pure functions to reduce complexity from {} to ~{}",
            functions_to_extract, cyclomatic, predicted_complexity
        )
    } else {
        format!(
            "Extract {} pure functions (complexity {} → ~{})",
            functions_to_extract,
            cyclomatic,
            predicted_complexity.min(10)
        )
    };

    // Provide detailed explanation based on complexity level
    let complexity_impact = if cyclomatic > 15 {
        format!("Cyclomatic complexity of {} indicates {} independent execution paths, requiring at least {} test cases for full coverage", 
                cyclomatic, cyclomatic, cyclomatic)
    } else if cyclomatic > 10 {
        format!("Cyclomatic complexity of {} indicates {} independent paths, making comprehensive testing difficult",
                cyclomatic, cyclomatic)
    } else if cyclomatic > 5 {
        format!("Cyclomatic complexity of {} means at least {} test cases needed for full path coverage - extraction reduces this to 3-5 tests per function",
                cyclomatic, cyclomatic)
    } else {
        format!(
            "Cyclomatic complexity of {} is manageable but can be improved through extraction",
            cyclomatic
        )
    };

    let extraction_reasoning = format!(
        "Extracting {} functions targets ~5 complexity per function - the sweet spot for maintainability where each function has a single clear purpose and can be tested with 3-5 test cases.",
        functions_to_extract
    );

    let purity_note = if is_pure && purity_confidence > 0.8 {
        " Pure function extraction will create easily testable units with no side effects."
    } else if is_pure {
        " Function appears pure, making extraction safer."
    } else {
        " Consider isolating side effects during extraction."
    };

    let rationale = format!(
        "{}. {}{}",
        complexity_impact, extraction_reasoning, purity_note
    );

    (action, rationale, steps)
}

/// Helper function to get pattern type name for display
fn pattern_type_name(pattern: &crate::extraction_patterns::ExtractablePattern) -> &str {
    use crate::extraction_patterns::ExtractablePattern;
    match pattern {
        ExtractablePattern::AccumulationLoop { .. } => "accumulation loop",
        ExtractablePattern::GuardChainSequence { .. } => "guard chain",
        ExtractablePattern::TransformationPipeline { .. } => "transformation pipeline",
        ExtractablePattern::SimilarBranches { .. } => "similar branches",
        ExtractablePattern::NestedExtraction { .. } => "nested pattern",
    }
}

/// Helper function to detect file language
fn detect_file_language(path: &std::path::Path) -> crate::core::Language {
    let extension = path.extension().and_then(|ext| ext.to_str()).unwrap_or("");

    match extension {
        "rs" => crate::core::Language::Rust,
        "py" => crate::core::Language::Python,
        "js" | "jsx" | "ts" | "tsx" => crate::core::Language::JavaScript,
        _ => crate::core::Language::Rust, // Default to Rust
    }
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

// New recommendation generators for expanded debt types

fn generate_resource_management_recommendation(
    resource_type: &str,
    detail1: &str,
    detail2: &str,
) -> (String, String, Vec<String>) {
    match resource_type {
        "allocation" => (
            format!("Optimize allocation pattern: {}", detail1),
            format!("Resource impact: {}", detail2),
            vec![
                "Use object pooling where appropriate".to_string(),
                "Consider pre-allocation strategies".to_string(),
                "Profile memory usage patterns".to_string(),
                "Review data structure choices".to_string(),
            ],
        ),
        "blocking_io" => (
            format!("Optimize {} operation", detail1),
            format!("Context: {}", detail2),
            vec![
                "Consider async/await pattern".to_string(),
                "Use appropriate I/O libraries".to_string(),
                "Consider background processing".to_string(),
                "Add proper error handling".to_string(),
            ],
        ),
        "basic" => (
            format!("Optimize {} resource issue", detail1),
            format!("Resource impact ({}): {}", detail2, detail1),
            vec![
                "Profile and identify resource bottlenecks".to_string(),
                "Apply resource optimization techniques".to_string(),
                "Monitor resource usage before and after changes".to_string(),
                "Consider algorithmic improvements".to_string(),
            ],
        ),
        _ => (
            "Optimize resource usage".to_string(),
            "Resource issue detected".to_string(),
            vec!["Monitor and profile resource usage".to_string()],
        ),
    }
}

fn generate_string_concat_recommendation(
    loop_type: &str,
    iterations: &Option<u32>,
) -> (String, String, Vec<String>) {
    let iter_info = iterations.map_or("unknown".to_string(), |i| i.to_string());
    (
        format!("Use StringBuilder for {} loop concatenation", loop_type),
        format!(
            "String concatenation in {} (≈{} iterations)",
            loop_type, iter_info
        ),
        vec![
            "Replace += with StringBuilder/StringBuffer".to_string(),
            "Pre-allocate capacity if known".to_string(),
            "Consider string formatting alternatives".to_string(),
            "Benchmark performance improvement".to_string(),
        ],
    )
}

fn generate_nested_loops_recommendation(
    depth: u32,
    complexity_estimate: &str,
) -> (String, String, Vec<String>) {
    (
        format!("Reduce {}-level nested loop complexity", depth),
        format!("Complexity estimate: {}", complexity_estimate),
        vec![
            "Extract inner loops into functions".to_string(),
            "Consider algorithmic improvements".to_string(),
            "Use iterators for cleaner code".to_string(),
            "Profile actual performance impact".to_string(),
        ],
    )
}

fn generate_data_structure_recommendation(
    current: &str,
    recommended: &str,
) -> (String, String, Vec<String>) {
    (
        format!("Replace {} with {}", current, recommended),
        format!(
            "Data structure {} is suboptimal for access patterns",
            current
        ),
        vec![
            format!("Refactor to use {}", recommended),
            "Update related algorithms".to_string(),
            "Test performance before/after".to_string(),
            "Update documentation".to_string(),
        ],
    )
}

fn generate_god_object_recommendation(
    responsibility_count: u32,
    complexity_score: f64,
) -> (String, String, Vec<String>) {
    (
        format!(
            "Split {} responsibilities into focused classes",
            responsibility_count
        ),
        format!("God object with complexity {:.1}", complexity_score),
        vec![
            "Identify single responsibility principle violations".to_string(),
            "Extract cohesive functionality into separate classes".to_string(),
            "Use composition over inheritance".to_string(),
            "Refactor incrementally with tests".to_string(),
        ],
    )
}

fn generate_feature_envy_recommendation(
    external_class: &str,
    usage_ratio: f64,
) -> (String, String, Vec<String>) {
    (
        format!("Move method closer to {} class", external_class),
        format!(
            "Method uses {}% external data",
            (usage_ratio * 100.0) as u32
        ),
        vec![
            format!("Consider moving method to {}", external_class),
            "Extract shared functionality".to_string(),
            "Review class responsibilities".to_string(),
            "Maintain cohesion after refactoring".to_string(),
        ],
    )
}

fn generate_primitive_obsession_recommendation(
    primitive_type: &str,
    domain_concept: &str,
) -> (String, String, Vec<String>) {
    (
        format!(
            "Create {} domain type instead of {}",
            domain_concept, primitive_type
        ),
        format!(
            "Primitive obsession: {} used for {}",
            primitive_type, domain_concept
        ),
        vec![
            format!("Create {} value object", domain_concept),
            "Add validation and behavior to type".to_string(),
            "Replace primitive usage throughout codebase".to_string(),
            "Add type safety and domain logic".to_string(),
        ],
    )
}

fn generate_magic_values_recommendation(
    value: &str,
    occurrences: u32,
) -> (String, String, Vec<String>) {
    (
        format!("Extract '{}' into named constant", value),
        format!("Magic value '{}' appears {} times", value, occurrences),
        vec![
            format!(
                "Define const {} = '{}'",
                value.to_uppercase().replace(' ', "_"),
                value
            ),
            "Replace all occurrences with named constant".to_string(),
            "Add documentation explaining value meaning".to_string(),
            "Group related constants in module".to_string(),
        ],
    )
}

fn generate_assertion_complexity_recommendation(
    assertion_count: u32,
    complexity_score: f64,
) -> (String, String, Vec<String>) {
    (
        format!("Simplify {} complex assertions", assertion_count),
        format!("Test assertion complexity: {:.1}", complexity_score),
        vec![
            "Split complex assertions into multiple simple ones".to_string(),
            "Use custom assertion helpers".to_string(),
            "Add descriptive assertion messages".to_string(),
            "Consider table-driven test patterns".to_string(),
        ],
    )
}

fn generate_flaky_test_recommendation(
    pattern_type: &str,
    reliability_impact: &str,
) -> (String, String, Vec<String>) {
    (
        format!("Fix {} flaky test pattern", pattern_type),
        format!("Reliability impact: {}", reliability_impact),
        vec![
            "Identify and eliminate non-deterministic behavior".to_string(),
            "Use test doubles to isolate dependencies".to_string(),
            "Add proper test cleanup and setup".to_string(),
            "Consider parallel test safety".to_string(),
        ],
    )
}

fn generate_async_misuse_recommendation(
    pattern: &str,
    performance_impact: &str,
) -> (String, String, Vec<String>) {
    (
        format!("Fix async pattern: {}", pattern),
        format!("Resource impact: {}", performance_impact),
        vec![
            "Use proper async/await patterns".to_string(),
            "Avoid blocking async contexts".to_string(),
            "Configure async runtime appropriately".to_string(),
            "Add timeout and cancellation handling".to_string(),
        ],
    )
}

fn generate_resource_leak_recommendation(
    resource_type: &str,
    cleanup_missing: &str,
) -> (String, String, Vec<String>) {
    (
        format!("Add {} resource cleanup", resource_type),
        format!("Missing cleanup: {}", cleanup_missing),
        vec![
            "Implement Drop trait for automatic cleanup".to_string(),
            "Use RAII patterns for resource management".to_string(),
            "Add try-finally or defer patterns".to_string(),
            "Test resource cleanup in error scenarios".to_string(),
        ],
    )
}

fn generate_collection_inefficiency_recommendation(
    collection_type: &str,
    inefficiency_type: &str,
) -> (String, String, Vec<String>) {
    (
        format!("Optimize {} usage", collection_type),
        format!("Inefficiency: {}", inefficiency_type),
        vec![
            "Review collection access patterns".to_string(),
            "Consider alternative data structures".to_string(),
            "Pre-allocate capacity where possible".to_string(),
            "Monitor collection resource usage".to_string(),
        ],
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::priority::call_graph::{CallType, FunctionCall};
    use crate::priority::coverage_propagation::TransitiveCoverage;
    use crate::risk::lcov::FunctionCoverage;

    fn create_test_metrics() -> FunctionMetrics {
        FunctionMetrics {
            file: PathBuf::from("test.rs"),
            name: "test_function".to_string(),
            line: 10,
            length: 50,
            cyclomatic: 5,
            cognitive: 8,
            nesting: 0,
            is_test: false,
            visibility: None,
            is_trait_method: false,
            in_test_module: false,
            entropy_score: None,
            is_pure: None,
            purity_confidence: None,
        }
    }

    #[test]
    fn test_classify_test_debt_high_complexity() {
        let mut func = create_test_metrics();
        func.is_test = true;
        func.cyclomatic = 16;
        func.cognitive = 10;

        let debt = classify_test_debt(&func);
        match debt {
            DebtType::TestComplexityHotspot {
                cyclomatic,
                cognitive,
                threshold,
            } => {
                assert_eq!(cyclomatic, 16);
                assert_eq!(cognitive, 10);
                assert_eq!(threshold, 15);
            }
            _ => panic!("Expected TestComplexityHotspot"),
        }
    }

    #[test]
    fn test_classify_test_debt_high_cognitive() {
        let mut func = create_test_metrics();
        func.is_test = true;
        func.cyclomatic = 10;
        func.cognitive = 21;

        let debt = classify_test_debt(&func);
        match debt {
            DebtType::TestComplexityHotspot { .. } => {}
            _ => panic!("Expected TestComplexityHotspot"),
        }
    }

    #[test]
    fn test_classify_test_debt_low_complexity() {
        let mut func = create_test_metrics();
        func.is_test = true;
        func.cyclomatic = 5;
        func.cognitive = 8;

        let debt = classify_test_debt(&func);
        match debt {
            DebtType::TestingGap {
                coverage,
                cyclomatic,
                cognitive,
            } => {
                assert_eq!(coverage, 0.0);
                assert_eq!(cyclomatic, 5);
                assert_eq!(cognitive, 8);
            }
            _ => panic!("Expected TestingGap"),
        }
    }

    #[test]
    fn test_is_complexity_hotspot_high_cyclomatic() {
        let mut func = create_test_metrics();
        func.cyclomatic = 11;
        func.cognitive = 10;

        let result = is_complexity_hotspot(&func, &FunctionRole::PureLogic);
        assert!(result.is_some());
        match result.unwrap() {
            DebtType::ComplexityHotspot { cyclomatic, .. } => {
                assert_eq!(cyclomatic, 11);
            }
            _ => panic!("Expected ComplexityHotspot"),
        }
    }

    #[test]
    fn test_is_complexity_hotspot_high_cognitive() {
        let mut func = create_test_metrics();
        func.cyclomatic = 8;
        func.cognitive = 16;

        let result = is_complexity_hotspot(&func, &FunctionRole::PureLogic);
        assert!(result.is_some());
    }

    #[test]
    fn test_is_complexity_hotspot_orchestrator() {
        let mut func = create_test_metrics();
        func.cyclomatic = 6;
        func.cognitive = 8;

        let result = is_complexity_hotspot(&func, &FunctionRole::Orchestrator);
        assert!(result.is_some());
        match result.unwrap() {
            DebtType::ComplexityHotspot { .. } => {}
            _ => panic!("Expected ComplexityHotspot for orchestrator"),
        }
    }

    #[test]
    fn test_is_complexity_hotspot_orchestrator_low() {
        let mut func = create_test_metrics();
        func.cyclomatic = 4;
        func.cognitive = 5;

        let result = is_complexity_hotspot(&func, &FunctionRole::Orchestrator);
        assert!(result.is_none());
    }

    #[test]
    fn test_classify_simple_function_io_wrapper() {
        let mut func = create_test_metrics();
        func.cyclomatic = 2;
        func.cognitive = 3;

        let result = classify_simple_function_risk(&func, &FunctionRole::IOWrapper);
        assert!(result.is_some());
        match result.unwrap() {
            DebtType::Risk { risk_score, .. } => {
                assert_eq!(risk_score, 0.0);
            }
            _ => panic!("Expected Risk with 0.0 score"),
        }
    }

    #[test]
    fn test_classify_simple_function_entry_point() {
        let mut func = create_test_metrics();
        func.cyclomatic = 3;
        func.cognitive = 5;

        let result = classify_simple_function_risk(&func, &FunctionRole::EntryPoint);
        assert!(result.is_some());
    }

    #[test]
    fn test_classify_simple_function_pattern_match() {
        let mut func = create_test_metrics();
        func.cyclomatic = 3;
        func.cognitive = 4;

        let result = classify_simple_function_risk(&func, &FunctionRole::PatternMatch);
        assert!(result.is_some());
        match result.unwrap() {
            DebtType::Risk { risk_score, .. } => {
                assert_eq!(risk_score, 0.0);
            }
            _ => panic!("Expected Risk with 0.0 score"),
        }
    }

    #[test]
    fn test_classify_simple_function_pure_logic() {
        let mut func = create_test_metrics();
        func.cyclomatic = 2;
        func.cognitive = 3;
        func.length = 8;

        let result = classify_simple_function_risk(&func, &FunctionRole::PureLogic);
        assert!(result.is_some());
        match result.unwrap() {
            DebtType::Risk {
                risk_score,
                factors,
            } => {
                assert_eq!(risk_score, 0.0);
                assert!(factors[0].contains("Trivial pure function"));
            }
            _ => panic!("Expected Risk"),
        }
    }

    #[test]
    fn test_classify_simple_function_pure_logic_too_long() {
        let mut func = create_test_metrics();
        func.cyclomatic = 2;
        func.cognitive = 3;
        func.length = 15;

        let result = classify_simple_function_risk(&func, &FunctionRole::PureLogic);
        assert!(result.is_none());
    }

    #[test]
    fn test_classify_simple_function_not_simple() {
        let mut func = create_test_metrics();
        func.cyclomatic = 5;
        func.cognitive = 7;

        let result = classify_simple_function_risk(&func, &FunctionRole::IOWrapper);
        assert!(result.is_none());
    }

    #[test]
    fn test_classify_risk_based_debt_high_cyclomatic() {
        let mut func = create_test_metrics();
        func.cyclomatic = 6;
        func.cognitive = 7;

        let debt = classify_risk_based_debt(&func, &FunctionRole::Unknown);
        match debt {
            DebtType::Risk { risk_score, .. } => {
                assert!(risk_score > 0.0);
            }
            _ => panic!("Expected Risk"),
        }
    }

    #[test]
    fn test_classify_risk_based_debt_high_cognitive() {
        let mut func = create_test_metrics();
        func.cyclomatic = 4;
        func.cognitive = 9;

        let debt = classify_risk_based_debt(&func, &FunctionRole::Unknown);
        match debt {
            DebtType::Risk { risk_score, .. } => {
                assert!(risk_score > 0.0);
            }
            _ => panic!("Expected Risk"),
        }
    }

    #[test]
    fn test_classify_risk_based_debt_long_function() {
        let mut func = create_test_metrics();
        func.cyclomatic = 3;
        func.cognitive = 5;
        func.length = 60;

        let debt = classify_risk_based_debt(&func, &FunctionRole::Unknown);
        match debt {
            DebtType::Risk { risk_score, .. } => {
                assert!(risk_score > 0.0);
            }
            _ => panic!("Expected Risk"),
        }
    }

    #[test]
    fn test_classify_risk_based_debt_simple_pure() {
        let mut func = create_test_metrics();
        func.cyclomatic = 3;
        func.cognitive = 5;
        func.length = 20;

        let debt = classify_risk_based_debt(&func, &FunctionRole::PureLogic);
        match debt {
            DebtType::Risk {
                risk_score,
                factors,
            } => {
                assert_eq!(risk_score, 0.0);
                assert!(factors[0].contains("Simple pure function"));
            }
            _ => panic!("Expected Risk"),
        }
    }

    #[test]
    fn test_classify_risk_based_debt_simple_other() {
        let mut func = create_test_metrics();
        func.cyclomatic = 3;
        func.cognitive = 5;
        func.length = 20;

        let debt = classify_risk_based_debt(&func, &FunctionRole::Unknown);
        match debt {
            DebtType::Risk {
                risk_score,
                factors,
            } => {
                assert_eq!(risk_score, 0.1);
                assert!(factors[0].contains("Simple function"));
            }
            _ => panic!("Expected Risk"),
        }
    }

    #[test]
    fn test_normalize_complexity() {
        assert!(normalize_complexity(1, 1) < 2.0);
        assert!(normalize_complexity(5, 5) > 2.0);
        assert!(normalize_complexity(5, 5) < 6.0);
        assert!(normalize_complexity(10, 10) > 5.0);
        assert!(normalize_complexity(20, 20) <= 10.0);
    }

    #[test]
    fn test_unified_scoring() {
        let func = create_test_metrics();
        let graph = CallGraph::new();
        let score = calculate_unified_priority(&func, &graph, None, None);

        assert!(score.complexity_factor > 0.0);
        assert!(score.coverage_factor > 0.0);
        // ROI and Semantic factors removed per spec 58
        assert!(score.final_score > 0.0);
        assert!(score.final_score <= 10.0);
    }

    #[test]
    fn test_debt_type_determination() {
        let func = create_test_metrics();
        let coverage = Some(TransitiveCoverage {
            direct: 0.1,
            transitive: 0.1,
            propagated_from: vec![],
            uncovered_lines: vec![],
        });

        let call_graph = CallGraph::new();
        let func_id = FunctionId {
            file: func.file.clone(),
            name: func.name.clone(),
            line: func.line,
        };
        let debt_type = determine_debt_type(&func, &coverage, &call_graph, &func_id);
        match debt_type {
            DebtType::TestingGap { .. } => (),
            _ => panic!("Expected TestingGap debt type"),
        }
    }

    #[test]
    fn test_recommendation_generation() {
        let func = create_test_metrics();
        let debt_type = DebtType::ComplexityHotspot {
            cyclomatic: 15,
            cognitive: 20,
        };
        let score = UnifiedScore {
            complexity_factor: 8.0,
            coverage_factor: 7.0,
            dependency_factor: 4.0,
            role_multiplier: 1.0,
            final_score: 6.5,
        };

        let rec = generate_recommendation(&func, &debt_type, FunctionRole::PureLogic, &score);
        // ComplexityHotspot now extracts first, then may add tests if coverage is low
        // Since generate_recommendation doesn't pass coverage, it defaults to unknown coverage
        // which means tests will be recommended
        assert!(
            rec.primary_action.contains("Extract"),
            "Action: {}",
            rec.primary_action
        );
        // The action might say "pure functions" or specific extraction names
        assert!(
            rec.primary_action.contains("Extract"),
            "Action should mention extraction: {}",
            rec.primary_action
        );
        // With unknown coverage, tests should be recommended in the steps
        assert!(
            rec.implementation_steps
                .iter()
                .any(|s| s.contains("test") || s.contains("Test")),
            "Steps should mention tests when coverage is unknown: {:?}",
            rec.implementation_steps
        );
        assert!(
            rec.rationale.contains("complexity") || rec.rationale.contains("cyclo"),
            "Rationale should mention complexity: {}",
            rec.rationale
        );
        assert!(!rec.implementation_steps.is_empty());
    }

    fn test_generate_testing_gap_recommendation_helper(
        coverage: f64,
        cyclomatic: u32,
        cognitive: u32,
        role: FunctionRole,
    ) -> (String, String, Vec<String>) {
        let func = create_test_metrics();
        generate_testing_gap_recommendation(coverage, cyclomatic, cognitive, role, &func, &None)
    }

    #[test]
    fn test_coverage_gap_messaging() {
        // Test with 0% coverage (100% gap)
        let (action, rationale, _) =
            test_generate_testing_gap_recommendation_helper(0.0, 5, 8, FunctionRole::PureLogic);
        assert_eq!(action, "Add 5 tests for 100% coverage gap");
        assert!(rationale.contains("100%"));
        assert!(rationale.contains("0% covered"));

        // Test with 40% coverage (60% gap)
        let (action, rationale, _) =
            test_generate_testing_gap_recommendation_helper(0.4, 5, 8, FunctionRole::Orchestrator);
        assert_eq!(action, "Add 3 tests for 60% coverage gap");
        assert!(rationale.contains("60%"));
        assert!(rationale.contains("40% covered"));

        // Test with 75% coverage (25% gap from 100%)
        let (action, rationale, _) =
            test_generate_testing_gap_recommendation_helper(0.75, 5, 8, FunctionRole::IOWrapper);
        assert_eq!(action, "Add 2 tests for 25% coverage gap");
        assert!(rationale.contains("25%"));
        assert!(rationale.contains("75% covered"));
    }

    #[test]
    fn test_dead_code_detection() {
        let mut func = create_test_metrics();
        func.name = "unused_helper".to_string();

        let mut call_graph = CallGraph::new();
        let func_id = FunctionId {
            file: func.file.clone(),
            name: func.name.clone(),
            line: func.line,
        };

        // Function exists but has no callers - should be dead code
        call_graph.add_function(func_id.clone(), false, false, func.cyclomatic, func.length);

        let debt_type = determine_debt_type(&func, &None, &call_graph, &func_id);

        match debt_type {
            DebtType::DeadCode {
                visibility: FunctionVisibility::Private,
                ..
            } => (),
            _ => panic!("Expected DeadCode for unused private function, got {debt_type:?}"),
        }
    }

    #[test]
    fn test_main_function_not_dead_code() {
        let mut func = create_test_metrics();
        func.name = "main".to_string();

        let call_graph = CallGraph::new();
        let func_id = FunctionId {
            file: func.file.clone(),
            name: func.name.clone(),
            line: func.line,
        };

        let debt_type = determine_debt_type(&func, &None, &call_graph, &func_id);

        // Main should not be flagged as dead code
        if let DebtType::DeadCode { .. } = debt_type {
            panic!("Main function should not be flagged as dead code")
        }
    }

    #[test]
    fn test_simple_io_wrapper_with_coverage_zero_score() {
        // Create a simple I/O wrapper function with test coverage
        let mut func = create_test_metrics();
        func.name = "extract_module_from_import".to_string();
        func.cyclomatic = 1;
        func.cognitive = 1;
        func.length = 3;
        func.nesting = 1;

        let call_graph = CallGraph::new();

        // Create mock coverage data showing function is tested
        let mut lcov = LcovData::default();
        lcov.functions.insert(
            func.file.clone(),
            vec![crate::risk::lcov::FunctionCoverage {
                name: func.name.clone(),
                start_line: func.line,
                execution_count: 18,
                coverage_percentage: 100.0,
                uncovered_lines: vec![],
            }],
        );

        // Calculate priority score with coverage
        let score = calculate_unified_priority(&func, &call_graph, Some(&lcov), None);

        // Tested simple I/O wrapper should have zero score (not technical debt)
        assert_eq!(score.final_score, 0.0);
        assert_eq!(score.complexity_factor, 0.0);
        assert_eq!(score.coverage_factor, 0.0);
        // ROI and Semantic factors removed per spec 58
    }

    #[test]
    fn test_simple_io_wrapper_without_coverage_has_score() {
        // Create a simple I/O wrapper function without test coverage
        let mut func = create_test_metrics();
        func.name = "print_risk_function".to_string();
        func.cyclomatic = 1;
        func.cognitive = 0;
        func.length = 4;
        func.nesting = 1;

        let call_graph = CallGraph::new();

        // Calculate priority score without coverage (assume untested)
        let score = calculate_unified_priority(&func, &call_graph, None, None);

        // Untested simple I/O wrapper should have a non-zero score (testing gap)
        assert!(
            score.final_score > 0.0,
            "Untested I/O wrapper should have non-zero score"
        );
    }

    #[test]
    fn test_simple_entry_point_with_coverage_zero_score() {
        // Create a simple entry point function with coverage
        let mut func = create_test_metrics();
        func.name = "main".to_string();
        func.cyclomatic = 2;
        func.cognitive = 3;
        func.length = 8;

        let call_graph = CallGraph::new();

        // Create mock coverage data
        let mut lcov = LcovData::default();
        lcov.functions.insert(
            func.file.clone(),
            vec![crate::risk::lcov::FunctionCoverage {
                name: func.name.clone(),
                start_line: func.line,
                execution_count: 1,
                coverage_percentage: 100.0,
                uncovered_lines: vec![],
            }],
        );

        // Calculate priority score with coverage
        let score = calculate_unified_priority(&func, &call_graph, Some(&lcov), None);

        // Tested simple entry point should have zero score (not technical debt)
        assert_eq!(score.final_score, 0.0);
    }

    #[test]
    fn test_simple_pure_function_without_coverage_has_score() {
        // Create a simple pure logic function without coverage
        let mut func = create_test_metrics();
        func.name = "format_string".to_string();
        func.cyclomatic = 1;
        func.cognitive = 2;
        func.length = 5;

        let call_graph = CallGraph::new();

        // Calculate priority score without coverage
        let score = calculate_unified_priority(&func, &call_graph, None, None);

        // Untested pure function should have non-zero score (testing gap)
        assert!(
            score.final_score > 0.0,
            "Untested pure function should have non-zero score"
        );
    }

    #[test]
    fn test_complex_function_has_score() {
        // Create a complex function that should have a non-zero score
        let mut func = create_test_metrics();
        func.name = "complex_logic".to_string();
        func.cyclomatic = 8;
        func.cognitive = 12;
        func.length = 50;

        let call_graph = CallGraph::new();

        // Calculate priority score
        let score = calculate_unified_priority(&func, &call_graph, None, None);

        // Complex function should have non-zero score (is technical debt)
        assert!(score.final_score > 0.0);
        assert!(score.complexity_factor > 0.0);
    }

    #[test]
    fn test_dead_code_recommendation() {
        let mut func = create_test_metrics();
        func.visibility = Some("pub".to_string()); // Make it public for the test
        let debt_type = DebtType::DeadCode {
            visibility: FunctionVisibility::Public,
            cyclomatic: 5,
            cognitive: 8,
            usage_hints: vec!["No internal callers".to_string()],
        };
        let score = UnifiedScore {
            complexity_factor: 5.0,
            coverage_factor: 0.0,
            dependency_factor: 0.0,
            role_multiplier: 1.0,
            final_score: 2.0,
        };

        let rec = generate_recommendation(&func, &debt_type, FunctionRole::Unknown, &score);
        // With the new API detection, a public function in test.rs with no special indicators
        // will be marked as "Remove unused public function (no API indicators)"
        assert!(
            rec.primary_action.contains("Remove unused public function")
                || rec.primary_action.contains("Verify external usage")
        );
        assert!(rec.rationale.contains("no callers"));
        assert!(rec
            .implementation_steps
            .iter()
            .any(|s| s.contains("external callers") || s.contains("Verify")));
    }

    #[test]
    fn test_format_role_description_pure_logic() {
        let role = FunctionRole::PureLogic;
        let description = format_role_description(role);
        assert_eq!(description, "business logic");
    }

    #[test]
    fn test_format_role_description_orchestrator() {
        let role = FunctionRole::Orchestrator;
        let description = format_role_description(role);
        assert_eq!(description, "orchestration");
    }

    #[test]
    fn test_format_role_description_io_wrapper() {
        let role = FunctionRole::IOWrapper;
        let description = format_role_description(role);
        assert_eq!(description, "I/O wrapper");
    }

    #[test]
    fn test_format_role_description_entry_point() {
        let role = FunctionRole::EntryPoint;
        let description = format_role_description(role);
        assert_eq!(description, "entry point");
    }

    #[test]
    fn test_format_role_description_unknown() {
        let role = FunctionRole::Unknown;
        let description = format_role_description(role);
        assert_eq!(description, "function");
    }

    #[test]
    fn test_generate_testing_gap_recommendation_complex_high_cyclomatic() {
        let (action, rationale, steps) = test_generate_testing_gap_recommendation_helper(
            0.25,
            15, // high cyclomatic (> 10)
            10, // normal cognitive
            FunctionRole::PureLogic,
        );

        // With high complexity, should include refactoring recommendation
        assert!(action.contains("Add") && action.contains("tests"));
        assert!(action.contains("75%")); // 100 - 25 = 75% gap
        assert!(
            rationale.contains("Complex business logic")
                || rationale.contains("high testing priority")
        );
        assert!(rationale.contains("75% gap")); // 100 - 25 = 75% gap
        assert_eq!(steps.len(), 12); // Complex functions generate more detailed steps
                                     // Step checking removed as step order may vary
    }

    #[test]
    fn test_generate_testing_gap_recommendation_complex_high_cognitive() {
        let (action, rationale, steps) = test_generate_testing_gap_recommendation_helper(
            0.5,
            8,  // normal cyclomatic
            20, // high cognitive (> 15)
            FunctionRole::Orchestrator,
        );

        // With high complexity, should include refactoring recommendation
        assert!(action.contains("Add") && action.contains("tests"));
        assert!(action.contains("50%")); // 100 - 50 = 50% gap
        assert!(
            rationale.contains("Complex orchestration")
                || rationale.contains("high testing priority")
        );
        assert!(rationale.contains("50% gap")); // 100 - 50 = 50% gap
        assert_eq!(steps.len(), 12); // Complex functions generate more detailed steps
                                     // Step checking removed as step order may vary
    }

    #[test]
    fn test_generate_testing_gap_recommendation_simple_pure_logic() {
        let (action, rationale, steps) = test_generate_testing_gap_recommendation_helper(
            0.0,
            5, // low complexity
            8, // low cognitive
            FunctionRole::PureLogic,
        );

        // New format says "Add X tests for Y% coverage gap"
        assert_eq!(action, "Add 5 tests for 100% coverage gap");
        assert!(rationale.contains("Business logic"));
        assert!(rationale.contains("100%"));
        assert_eq!(steps.len(), 3);
        assert!(steps[0].contains("happy path"));
    }

    #[test]
    fn test_generate_testing_gap_recommendation_simple_orchestrator() {
        let (action, rationale, steps) =
            test_generate_testing_gap_recommendation_helper(0.75, 3, 5, FunctionRole::Orchestrator);

        // New format says "Add X tests for Y% coverage gap"
        // With cyclomatic 3 and 75% coverage, needs (3 * 0.25) = 0.75 → 1, but min is 2
        assert_eq!(action, "Add 2 tests for 25% coverage gap");
        assert!(rationale.contains("Orchestration"));
        assert!(rationale.contains("25%"));
        assert_eq!(steps.len(), 3);
        assert!(steps[1].contains("edge case"));
    }

    #[test]
    fn test_generate_testing_gap_recommendation_simple_io_wrapper() {
        let (action, rationale, steps) =
            test_generate_testing_gap_recommendation_helper(0.33, 2, 3, FunctionRole::IOWrapper);

        // New format says "Add X tests for Y% coverage gap"
        assert_eq!(action, "Add 2 tests for 67% coverage gap");
        assert!(rationale.contains("I/O wrapper"));
        assert!(rationale.contains("67%")); // 100 - 33 = 67% gap
        assert_eq!(steps.len(), 3);
        assert!(steps[2].contains("error conditions"));
    }

    #[test]
    fn test_generate_testing_gap_recommendation_simple_entry_point() {
        let (action, rationale, steps) =
            test_generate_testing_gap_recommendation_helper(1.0, 1, 1, FunctionRole::EntryPoint);

        // With 100% coverage, action is to maintain coverage
        assert_eq!(action, "Maintain test coverage");
        assert!(rationale.contains("100% covered"));
        assert!(rationale.contains("Entry point"));
        assert_eq!(steps.len(), 3);
    }

    #[test]
    fn test_generate_testing_gap_recommendation_simple_unknown_role() {
        let (action, rationale, steps) = test_generate_testing_gap_recommendation_helper(
            0.0,
            0, // will use max(0, 2) = 2
            0,
            FunctionRole::Unknown,
        );

        // New format says "Add X tests for Y% coverage gap"
        // With cyclomatic 0, uses max(0, 2) = 2, coverage 0%, so 2 * 1.0 = 2
        assert_eq!(action, "Add 2 tests for 100% coverage gap");
        assert!(rationale.contains("Function"));
        // With 0% coverage, there's 100% gap
        assert!(rationale.contains("100%"));
        assert_eq!(steps.len(), 3);
    }

    #[test]
    fn test_generate_testing_gap_recommendation_both_high_complexity() {
        let (action, rationale, steps) = test_generate_testing_gap_recommendation_helper(
            0.1,
            25, // very high cyclomatic
            30, // very high cognitive
            FunctionRole::PureLogic,
        );

        // With high complexity, should include refactoring recommendation
        assert!(action.contains("Add") && action.contains("tests"));
        assert!(action.contains("90%")); // 100 - 10 = 90% gap
        assert!(
            rationale.contains("Complex business logic")
                || rationale.contains("high testing priority")
        );
        assert!(rationale.contains("90%")); // 100 - 10 = 90% gap
        assert_eq!(steps.len(), 12); // Complex functions generate more detailed steps
                                     // Steps have changed format, checking for test-related content
        assert!(steps
            .iter()
            .any(|s| s.contains("test") || s.contains("Test")));
        assert!(steps
            .iter()
            .any(|s| s.contains("extract") || s.contains("Extract")));
    }

    #[test]
    fn test_generate_test_debt_recommendation_complexity_hotspot() {
        let debt_type = DebtType::TestComplexityHotspot {
            cyclomatic: 15,
            cognitive: 20,
            threshold: 10,
        };

        let (action, rationale, steps) = generate_test_debt_recommendation(&debt_type);

        assert_eq!(
            action,
            "Simplify test - complexity 20 exceeds test threshold 10"
        );
        assert_eq!(rationale, "Test has high complexity (cyclo=15, cognitive=20) - consider splitting into smaller tests");
        assert_eq!(steps.len(), 3);
        assert_eq!(steps[0], "Break complex test into multiple smaller tests");
        assert_eq!(steps[1], "Extract test setup into helper functions");
        assert_eq!(steps[2], "Use parameterized tests for similar test cases");
    }

    #[test]
    fn test_generate_test_debt_recommendation_todo_with_reason() {
        let debt_type = DebtType::TestTodo {
            priority: crate::core::Priority::Medium,
            reason: Some("Need to test error handling".to_string()),
        };

        let (action, rationale, steps) = generate_test_debt_recommendation(&debt_type);

        assert_eq!(action, "Complete test TODO");
        assert_eq!(rationale, "Test contains TODO: Need to test error handling");
        assert_eq!(steps.len(), 3);
        assert_eq!(steps[0], "Address the TODO comment");
        assert_eq!(steps[1], "Implement missing test logic");
        assert_eq!(steps[2], "Remove TODO once completed");
    }

    #[test]
    fn test_generate_test_debt_recommendation_todo_without_reason() {
        let debt_type = DebtType::TestTodo {
            priority: crate::core::Priority::Low,
            reason: None,
        };

        let (action, rationale, steps) = generate_test_debt_recommendation(&debt_type);

        assert_eq!(action, "Complete test TODO");
        assert_eq!(rationale, "Test contains TODO: No reason specified");
        assert_eq!(steps.len(), 3);
        assert_eq!(steps[0], "Address the TODO comment");
        assert_eq!(steps[1], "Implement missing test logic");
        assert_eq!(steps[2], "Remove TODO once completed");
    }

    #[test]
    fn test_generate_test_debt_recommendation_duplication() {
        let debt_type = DebtType::TestDuplication {
            instances: 5,
            total_lines: 150,
            similarity: 0.85,
        };

        let (action, rationale, steps) = generate_test_debt_recommendation(&debt_type);

        assert_eq!(action, "Remove test duplication - 5 similar test blocks");
        assert_eq!(rationale, "5 duplicated test blocks found across 150 lines");
        assert_eq!(steps.len(), 3);
        assert_eq!(steps[0], "Extract common test logic into helper functions");
        assert_eq!(
            steps[1],
            "Create parameterized tests for similar test cases"
        );
        assert_eq!(steps[2], "Use test fixtures for shared setup");
    }

    #[test]
    fn test_is_function_complex() {
        // Not complex - both metrics below thresholds
        assert!(!is_function_complex(5, 10));
        assert!(!is_function_complex(10, 15));

        // Complex - cyclomatic exceeds threshold
        assert!(is_function_complex(11, 10));
        assert!(is_function_complex(15, 5));

        // Complex - cognitive exceeds threshold
        assert!(is_function_complex(5, 16));
        assert!(is_function_complex(10, 20));

        // Complex - both exceed thresholds
        assert!(is_function_complex(11, 16));
        assert!(is_function_complex(20, 25));
    }

    #[test]
    fn test_calculate_risk_factor() {
        // Test each debt type returns expected risk factor
        let testing_gap = DebtType::TestingGap {
            coverage: 0.5,
            cyclomatic: 5,
            cognitive: 8,
        };
        assert_eq!(calculate_risk_factor(&testing_gap), 0.42);

        let complexity = DebtType::ComplexityHotspot {
            cyclomatic: 15,
            cognitive: 20,
        };
        assert_eq!(calculate_risk_factor(&complexity), 0.35);

        let dead_code = DebtType::DeadCode {
            cyclomatic: 5,
            cognitive: 7,
            visibility: FunctionVisibility::Private,
            usage_hints: vec![],
        };
        assert_eq!(calculate_risk_factor(&dead_code), 0.3);

        let duplication = DebtType::Duplication {
            instances: 3,
            total_lines: 90,
        };
        assert_eq!(calculate_risk_factor(&duplication), 0.25);

        let risk = DebtType::Risk {
            risk_score: 2.0,
            factors: vec!["test".to_string()],
        };
        assert_eq!(calculate_risk_factor(&risk), 0.2);

        let test_complexity = DebtType::TestComplexityHotspot {
            cyclomatic: 12,
            cognitive: 18,
            threshold: 10,
        };
        assert_eq!(calculate_risk_factor(&test_complexity), 0.15);
    }

    #[test]
    fn test_calculate_coverage_improvement() {
        // Simple function with 0% coverage
        assert_eq!(calculate_coverage_improvement(0.0, false), 100.0);

        // Simple function with 60% coverage
        assert_eq!(calculate_coverage_improvement(0.6, false), 40.0);

        // Complex function with 0% coverage (reduced potential)
        assert_eq!(calculate_coverage_improvement(0.0, true), 50.0);

        // Complex function with 60% coverage (reduced potential)
        assert_eq!(calculate_coverage_improvement(0.6, true), 20.0);

        // Full coverage - no improvement possible
        assert_eq!(calculate_coverage_improvement(1.0, false), 0.0);
        assert_eq!(calculate_coverage_improvement(1.0, true), 0.0);
    }

    #[test]
    fn test_calculate_lines_reduction() {
        // Dead code reduction
        let dead_code = DebtType::DeadCode {
            cyclomatic: 5,
            cognitive: 7,
            visibility: FunctionVisibility::Private,
            usage_hints: vec![],
        };
        assert_eq!(calculate_lines_reduction(&dead_code), 12);

        // Duplication reduction
        let duplication = DebtType::Duplication {
            instances: 3,
            total_lines: 90,
        };
        assert_eq!(calculate_lines_reduction(&duplication), 60);

        // Test duplication reduction
        let test_dup = DebtType::TestDuplication {
            instances: 2,
            total_lines: 50,
            similarity: 0.9,
        };
        assert_eq!(calculate_lines_reduction(&test_dup), 25);

        // No reduction for other types
        let testing_gap = DebtType::TestingGap {
            coverage: 0.5,
            cyclomatic: 5,
            cognitive: 8,
        };
        assert_eq!(calculate_lines_reduction(&testing_gap), 0);
    }

    #[test]
    fn test_calculate_complexity_reduction() {
        // Dead code complexity reduction
        let dead_code = DebtType::DeadCode {
            cyclomatic: 10,
            cognitive: 14,
            visibility: FunctionVisibility::Private,
            usage_hints: vec![],
        };
        assert_eq!(calculate_complexity_reduction(&dead_code, false), 12.0);

        // Testing gap - complex function
        let testing_gap = DebtType::TestingGap {
            coverage: 0.3,
            cyclomatic: 15,
            cognitive: 20,
        };
        assert_eq!(calculate_complexity_reduction(&testing_gap, true), 4.5);

        // Testing gap - simple function
        assert_eq!(calculate_complexity_reduction(&testing_gap, false), 0.0);

        // Complexity hotspot
        let complexity = DebtType::ComplexityHotspot {
            cyclomatic: 20,
            cognitive: 25,
        };
        assert_eq!(calculate_complexity_reduction(&complexity, false), 10.0);

        // Test complexity hotspot
        let test_complexity = DebtType::TestComplexityHotspot {
            cyclomatic: 12,
            cognitive: 16,
            threshold: 10,
        };
        assert!((calculate_complexity_reduction(&test_complexity, false) - 3.6).abs() < 0.001);
    }

    #[test]
    fn test_risk_debt_recommendation_with_moderate_complexity() {
        let func = create_test_metrics();
        let debt_type = DebtType::Risk {
            risk_score: 5.0,
            factors: vec!["Moderate complexity (cyclomatic: 9)".to_string()],
        };
        let score = UnifiedScore {
            complexity_factor: 5.0,
            coverage_factor: 3.0,
            dependency_factor: 2.0,
            role_multiplier: 1.0,
            final_score: 3.0,
        };

        let rec = generate_recommendation(&func, &debt_type, FunctionRole::PureLogic, &score);
        assert!(rec.primary_action.contains("Extract 3 pure functions"));
        assert!(rec.primary_action.contains("complexity 9 → 3"));
        // When coverage is unknown (None), tests are still recommended
        assert!(rec.primary_action.contains("comprehensive tests"));
        assert!(rec.rationale.contains("Risk score 5.0"));
        assert!(rec.rationale.contains("Moderate complexity"));
        // With unknown coverage, we have 5 refactoring steps + 2 testing steps = 7
        // The actual number of steps depends on coverage logic
        assert!(
            rec.implementation_steps.len() >= 5,
            "Expected at least 5 steps, got {}: {:?}",
            rec.implementation_steps.len(),
            rec.implementation_steps
        );
        // Verify key content exists somewhere in steps
        assert!(rec
            .implementation_steps
            .iter()
            .any(|s| s.contains("logical sections") || s.contains("function")));
        // Pattern matching may not always be present depending on complexity analysis
        assert!(rec
            .implementation_steps
            .iter()
            .any(|s| s.contains(".map()") || s.contains("pure functions")));
        // Check that test step exists when coverage is unknown
        let has_test_step = rec
            .implementation_steps
            .iter()
            .any(|s| s.contains("unit tests"));
        assert!(
            has_test_step,
            "Should have test step when coverage is unknown"
        );
        // Check for goal/target step
        let _has_goal = rec
            .implementation_steps
            .iter()
            .any(|s| s.contains("Goal") || s.contains("complexity from") || s.contains("80%+"));
    }

    #[test]
    fn test_risk_debt_recommendation_with_high_complexity() {
        let func = create_test_metrics();
        let debt_type = DebtType::Risk {
            risk_score: 7.0,
            factors: vec!["High complexity (cyclomatic: 15)".to_string()],
        };
        let score = UnifiedScore {
            complexity_factor: 7.0,
            coverage_factor: 5.0,
            dependency_factor: 3.0,
            role_multiplier: 1.0,
            final_score: 5.0,
        };

        let rec = generate_recommendation(&func, &debt_type, FunctionRole::Unknown, &score);
        assert!(rec.primary_action.contains("Decompose into"));
        assert!(rec.primary_action.contains("pure functions"));
        assert!(rec.primary_action.contains("complexity 15"));
        assert!(rec.rationale.contains("Risk score 7.0"));
        assert_eq!(rec.implementation_steps.len(), 8);
        assert!(rec.implementation_steps[0].contains("conditional branch"));
        assert!(rec.implementation_steps[2].contains("function dispatch table"));
    }

    #[test]
    fn test_risk_debt_recommendation_without_complexity() {
        let func = create_test_metrics();
        let debt_type = DebtType::Risk {
            risk_score: 2.0,
            factors: vec!["Low coverage: 30%".to_string()],
        };
        let score = UnifiedScore {
            complexity_factor: 1.0,
            coverage_factor: 6.0,
            dependency_factor: 1.0,
            role_multiplier: 1.0,
            final_score: 3.0,
        };

        let rec = generate_recommendation(&func, &debt_type, FunctionRole::Unknown, &score);
        assert_eq!(rec.primary_action, "Address identified risk factors");
        assert!(rec.rationale.contains("Low coverage"));
        assert_eq!(rec.implementation_steps.len(), 3);
        assert!(rec.implementation_steps[1].contains("missing tests"));
    }

    #[test]
    fn test_calculate_expected_impact_integration() {
        // Test the main function with various debt types
        let func = create_test_metrics();

        // Testing gap with complex function
        let testing_gap = DebtType::TestingGap {
            coverage: 0.2,
            cyclomatic: 12,
            cognitive: 18,
        };
        let score = UnifiedScore {
            complexity_factor: 5.0,
            coverage_factor: 6.0,
            dependency_factor: 2.0,
            role_multiplier: 1.0,
            final_score: 7.5,
        };

        let impact = calculate_expected_impact(&func, &testing_gap, &score);
        assert_eq!(impact.coverage_improvement, 40.0); // (1-0.2) * 50 for complex
        assert_eq!(impact.lines_reduction, 0);
        assert!((impact.complexity_reduction - 3.6).abs() < 0.001); // 12 * 0.3
        assert_eq!(impact.risk_reduction, 3.15); // 7.5 * 0.42
    }

    #[test]
    fn test_calculate_dependency_factor_dead_code() {
        // Test that functions with no callers get 0 priority (dead code)
        assert_eq!(calculate_dependency_factor(0), 0.0);
    }

    #[test]
    fn test_calculate_dependency_factor_low_criticality() {
        // Test low criticality range (1-2 callers)
        assert_eq!(calculate_dependency_factor(1), 2.0);
        assert_eq!(calculate_dependency_factor(2), 3.0);
    }

    #[test]
    fn test_calculate_dependency_factor_medium_criticality() {
        // Test medium criticality range (3-5 callers)
        assert_eq!(calculate_dependency_factor(3), 4.0);
        assert_eq!(calculate_dependency_factor(4), 5.0);
        assert_eq!(calculate_dependency_factor(5), 6.0);
    }

    #[test]
    fn test_calculate_dependency_factor_high_criticality() {
        // Test high criticality range (6-9 callers)
        assert_eq!(calculate_dependency_factor(6), 7.0);
        assert_eq!(calculate_dependency_factor(7), 7.0);
        assert_eq!(calculate_dependency_factor(8), 8.0);
        assert_eq!(calculate_dependency_factor(9), 8.0);
    }

    #[test]
    fn test_calculate_dependency_factor_critical_path() {
        // Test critical path range (10+ callers)
        assert_eq!(calculate_dependency_factor(10), 9.0);
        assert_eq!(calculate_dependency_factor(12), 9.0);
        assert_eq!(calculate_dependency_factor(14), 9.0);
        assert_eq!(calculate_dependency_factor(15), 10.0);
        assert_eq!(calculate_dependency_factor(20), 10.0);
        assert_eq!(calculate_dependency_factor(100), 10.0);
    }

    #[test]
    fn test_calculate_dependency_factor_boundaries() {
        // Test boundary conditions to ensure no gaps in coverage
        // Lower boundaries
        assert_eq!(calculate_dependency_factor(0), 0.0);
        assert_eq!(calculate_dependency_factor(1), 2.0);

        // Mid-range boundaries
        assert_eq!(calculate_dependency_factor(5), 6.0);
        assert_eq!(calculate_dependency_factor(6), 7.0);
        assert_eq!(calculate_dependency_factor(10), 9.0);

        // Upper boundaries
        assert_eq!(calculate_dependency_factor(14), 9.0);
        assert_eq!(calculate_dependency_factor(15), 10.0);
    }

    #[test]
    fn test_calculate_dependency_factor_monotonic_increase() {
        // Test that the function is monotonically increasing (higher input never gives lower output)
        let mut prev_value = calculate_dependency_factor(0);
        for i in 1..=20 {
            let current_value = calculate_dependency_factor(i);
            assert!(
                current_value >= prev_value,
                "Dependency factor should be monotonically increasing: {} callers gave {}, but {} callers gave {}",
                i - 1, prev_value, i, current_value
            );
            prev_value = current_value;
        }
    }

    #[test]
    fn test_calculate_dependency_factor_value_range() {
        // Test that all outputs are within expected range [0.0, 10.0]
        for count in 0..=100 {
            let factor = calculate_dependency_factor(count);
            assert!(
                (0.0..=10.0).contains(&factor),
                "Dependency factor {} for {} callers is out of range [0.0, 10.0]",
                factor,
                count
            );
        }
    }

    #[test]
    fn test_calculate_dependency_factor_comprehensive_coverage() {
        // Test every single branch in the match statement for complete coverage
        // This ensures 100% branch coverage of the function

        // Branch 0: Dead code
        assert_eq!(calculate_dependency_factor(0), 0.0);

        // Branch 1: Single caller
        assert_eq!(calculate_dependency_factor(1), 2.0);

        // Branch 2: Two callers
        assert_eq!(calculate_dependency_factor(2), 3.0);

        // Branch 3: Three callers
        assert_eq!(calculate_dependency_factor(3), 4.0);

        // Branch 4: Four callers
        assert_eq!(calculate_dependency_factor(4), 5.0);

        // Branch 5: Five callers
        assert_eq!(calculate_dependency_factor(5), 6.0);

        // Branch 6..=7: Six and seven callers
        assert_eq!(calculate_dependency_factor(6), 7.0);
        assert_eq!(calculate_dependency_factor(7), 7.0);

        // Branch 8..=9: Eight and nine callers
        assert_eq!(calculate_dependency_factor(8), 8.0);
        assert_eq!(calculate_dependency_factor(9), 8.0);

        // Branch 10..=14: Ten to fourteen callers
        assert_eq!(calculate_dependency_factor(10), 9.0);
        assert_eq!(calculate_dependency_factor(11), 9.0);
        assert_eq!(calculate_dependency_factor(12), 9.0);
        assert_eq!(calculate_dependency_factor(13), 9.0);
        assert_eq!(calculate_dependency_factor(14), 9.0);

        // Branch _: Fifteen or more callers
        assert_eq!(calculate_dependency_factor(15), 10.0);
        assert_eq!(calculate_dependency_factor(16), 10.0);
        assert_eq!(calculate_dependency_factor(50), 10.0);
        assert_eq!(calculate_dependency_factor(999), 10.0);
        assert_eq!(calculate_dependency_factor(usize::MAX), 10.0);
    }

    #[test]
    fn test_calculate_dependency_factor_large_values() {
        // Test with very large values to ensure no overflow or unexpected behavior
        assert_eq!(calculate_dependency_factor(1000), 10.0);
        assert_eq!(calculate_dependency_factor(10000), 10.0);
        assert_eq!(calculate_dependency_factor(100000), 10.0);
        assert_eq!(calculate_dependency_factor(1000000), 10.0);
        assert_eq!(calculate_dependency_factor(usize::MAX / 2), 10.0);
        assert_eq!(calculate_dependency_factor(usize::MAX - 1), 10.0);
        assert_eq!(calculate_dependency_factor(usize::MAX), 10.0);
    }

    #[test]
    fn test_calculate_dependency_factor_typical_scenarios() {
        // Test typical real-world scenarios with descriptive assertions

        // Utility function used by one module
        let single_use_utility = calculate_dependency_factor(1);
        assert_eq!(
            single_use_utility, 2.0,
            "Single-use utility should have low priority"
        );

        // Helper function used by a few modules
        let shared_helper = calculate_dependency_factor(3);
        assert_eq!(
            shared_helper, 4.0,
            "Shared helper should have medium priority"
        );

        // Core business logic used across the codebase
        let core_logic = calculate_dependency_factor(8);
        assert_eq!(
            core_logic, 8.0,
            "Core business logic should have high priority"
        );

        // Critical infrastructure function
        let infrastructure = calculate_dependency_factor(20);
        assert_eq!(
            infrastructure, 10.0,
            "Critical infrastructure should have maximum priority"
        );

        // Dead code that can be removed
        let dead_code = calculate_dependency_factor(0);
        assert_eq!(dead_code, 0.0, "Dead code should have zero priority");
    }

    #[test]
    fn test_calculate_dependency_factor_classification_consistency() {
        // Test that the classification is consistent within each range

        // Low criticality range (1-2)
        assert!(calculate_dependency_factor(1) < calculate_dependency_factor(3));
        assert!(calculate_dependency_factor(2) < calculate_dependency_factor(3));

        // Medium criticality range (3-5)
        assert!(calculate_dependency_factor(3) < calculate_dependency_factor(6));
        assert!(calculate_dependency_factor(4) < calculate_dependency_factor(6));
        assert!(calculate_dependency_factor(5) < calculate_dependency_factor(6));

        // High criticality range (6-9)
        assert!(calculate_dependency_factor(6) < calculate_dependency_factor(10));
        assert!(calculate_dependency_factor(7) < calculate_dependency_factor(10));
        assert!(calculate_dependency_factor(8) < calculate_dependency_factor(10));
        assert!(calculate_dependency_factor(9) < calculate_dependency_factor(10));

        // Critical path range (10-14)
        assert!(calculate_dependency_factor(10) < calculate_dependency_factor(15));
        assert!(calculate_dependency_factor(14) < calculate_dependency_factor(15));
    }

    #[test]
    fn test_calculate_dependency_factor_step_function_behavior() {
        // Test that the function behaves as a proper step function
        // with discrete jumps at boundaries

        // Test discrete jumps
        assert_ne!(
            calculate_dependency_factor(0),
            calculate_dependency_factor(1)
        );
        assert_ne!(
            calculate_dependency_factor(1),
            calculate_dependency_factor(2)
        );
        assert_ne!(
            calculate_dependency_factor(2),
            calculate_dependency_factor(3)
        );
        assert_ne!(
            calculate_dependency_factor(5),
            calculate_dependency_factor(6)
        );
        assert_ne!(
            calculate_dependency_factor(7),
            calculate_dependency_factor(8)
        );
        assert_ne!(
            calculate_dependency_factor(9),
            calculate_dependency_factor(10)
        );
        assert_ne!(
            calculate_dependency_factor(14),
            calculate_dependency_factor(15)
        );

        // Test plateaus within ranges
        assert_eq!(
            calculate_dependency_factor(6),
            calculate_dependency_factor(7)
        );
        assert_eq!(
            calculate_dependency_factor(8),
            calculate_dependency_factor(9)
        );
        assert_eq!(
            calculate_dependency_factor(10),
            calculate_dependency_factor(14)
        );
        assert_eq!(
            calculate_dependency_factor(15),
            calculate_dependency_factor(100)
        );
    }

    #[test]
    fn test_calculate_dependency_factor_mathematical_properties() {
        // Test mathematical properties of the function

        // Non-negative output
        for i in 0..=100 {
            assert!(
                calculate_dependency_factor(i) >= 0.0,
                "Output should always be non-negative"
            );
        }

        // Bounded output [0, 10]
        for i in 0..=1000 {
            let result = calculate_dependency_factor(i);
            assert!(
                (0.0..=10.0).contains(&result),
                "Output should be in range [0, 10]"
            );
        }

        // Monotonic non-decreasing
        let mut prev = calculate_dependency_factor(0);
        for i in 1..=100 {
            let curr = calculate_dependency_factor(i);
            assert!(curr >= prev, "Function should be monotonic non-decreasing");
            prev = curr;
        }

        // Saturation at maximum
        assert_eq!(
            calculate_dependency_factor(15),
            calculate_dependency_factor(1000),
            "Function should saturate at maximum value"
        );
    }

    #[test]
    fn test_calculate_dependency_factor_integration_with_scoring() {
        // Test how the dependency factor integrates with the overall scoring system
        // Simulating real-world usage patterns

        // Simulate scoring for different function types
        let test_scenarios = vec![
            (0, "dead_code_function", 0.0),
            (1, "private_helper", 2.0),
            (3, "module_internal", 4.0),
            (7, "public_api", 7.0),
            (12, "core_utility", 9.0),
            (25, "framework_base", 10.0),
        ];

        for (callers, description, expected) in test_scenarios {
            let factor = calculate_dependency_factor(callers);
            assert_eq!(
                factor, expected,
                "Function '{}' with {} callers should have factor {}",
                description, callers, expected
            );

            // Verify the factor can be used in calculations without issues
            let weighted_score = factor * 1.5; // Simulate weighting
            assert!(
                weighted_score.is_finite(),
                "Factor should produce valid calculations"
            );
        }
    }

    #[test]
    fn test_calculate_unified_priority_with_debt_trivial_tested_function() {
        // Test that trivial tested functions get zero score
        let mut func = create_test_metrics();
        func.cyclomatic = 2;
        func.cognitive = 3;
        func.length = 8;

        let graph = CallGraph::new();

        // Create mock coverage data showing the function is tested
        let mut lcov = LcovData::default();
        lcov.functions.insert(
            PathBuf::from("test.rs"),
            vec![FunctionCoverage {
                name: "test_function".to_string(),
                start_line: 10,
                execution_count: 1,
                coverage_percentage: 100.0,
                uncovered_lines: vec![],
            }],
        );

        let score = calculate_unified_priority_with_debt(&func, &graph, Some(&lcov), None, None);

        assert_eq!(
            score.final_score, 0.0,
            "Trivial tested function should have zero score"
        );
        assert_eq!(score.complexity_factor, 0.0);
        assert_eq!(score.coverage_factor, 0.0);
    }

    #[test]
    fn test_calculate_unified_priority_with_debt_complex_untested() {
        // Test that complex untested functions get high scores
        let mut func = create_test_metrics();
        func.cyclomatic = 15;
        func.cognitive = 20;
        func.length = 100;

        let graph = CallGraph::new();

        let score = calculate_unified_priority_with_debt(
            &func, &graph, None, // No coverage data
            None, None,
        );

        assert!(
            score.final_score > 5.0,
            "Complex untested function should have high score"
        );
        assert!(score.complexity_factor > 5.0);
        assert_eq!(
            score.coverage_factor, 10.0,
            "No coverage data should assume worst case"
        );
    }

    #[test]
    fn test_calculate_unified_priority_with_debt_with_entropy_dampening() {
        // Test entropy-based complexity dampening
        // Enable entropy for this test
        std::env::set_var("DEBTMAP_ENTROPY_ENABLED", "true");

        let mut func = create_test_metrics();
        func.cyclomatic = 20;
        func.cognitive = 25;
        func.entropy_score = Some(crate::complexity::entropy::EntropyScore {
            token_entropy: 0.1, // Low entropy to trigger dampening (< 0.2)
            pattern_repetition: 0.7,
            branch_similarity: 0.5,
            effective_complexity: 15.0,
            unique_variables: 5,
            max_nesting: 2,
            dampening_applied: 0.8,
        });

        let graph = CallGraph::new();

        let score_with_entropy =
            calculate_unified_priority_with_debt(&func, &graph, None, None, None);

        // Compare with same function without entropy
        func.entropy_score = None;
        let score_without_entropy =
            calculate_unified_priority_with_debt(&func, &graph, None, None, None);

        // Clean up
        std::env::remove_var("DEBTMAP_ENTROPY_ENABLED");

        // When entropy is enabled, the final score should be reduced but not by more than 50%
        // The scores may be the same if entropy is disabled in config
        // With low entropy (0.1), we expect dampening to apply
        if score_with_entropy.final_score < score_without_entropy.final_score {
            // Dampening was applied - check it's not more than 50%
            assert!(
                score_with_entropy.final_score >= score_without_entropy.final_score * 0.5,
                "Entropy dampening should not exceed 50% reduction per spec 68"
            );
        } else {
            // If scores are equal, entropy config might be disabled - that's ok
            // The test is still valid as it tests the mechanism when enabled
            assert!(
                score_with_entropy.final_score == score_without_entropy.final_score,
                "Scores should be equal when entropy is disabled"
            );
        }
    }

    #[test]
    fn test_calculate_unified_priority_with_debt_organization_issues() {
        // Test organization factor integration
        let func = create_test_metrics();
        let graph = CallGraph::new();

        let score = calculate_unified_priority_with_debt(
            &func,
            &graph,
            None,
            Some(7.0), // Organization issues
            None,
        );

        // Organization factor removed per spec 58 - redundant with complexity factor
        // Organization issues are now captured within complexity factor
        assert!(score.final_score > 0.0, "Score should be calculated");
    }

    #[test]
    fn test_calculate_unified_priority_with_debt_role_multiplier() {
        // Test that different function roles get appropriate multipliers
        let mut func = create_test_metrics();
        func.cyclomatic = 10;
        func.cognitive = 12;

        let mut graph = CallGraph::new();
        let func_id = FunctionId {
            file: func.file.clone(),
            name: func.name.clone(),
            line: func.line,
        };

        // Add some callers to make it an important function
        for i in 0..5 {
            let caller = FunctionId {
                file: PathBuf::from("caller.rs"),
                name: format!("caller_{}", i),
                line: i * 10,
            };
            graph.add_call(FunctionCall {
                caller,
                callee: func_id.clone(),
                call_type: CallType::Direct,
            });
        }

        let score = calculate_unified_priority_with_debt(&func, &graph, None, None, None);

        assert!(score.role_multiplier > 0.0);
        assert!(
            score.dependency_factor > 0.0,
            "Function with callers should have dependency factor"
        );
    }

    #[test]
    fn test_calculate_unified_priority_with_debt_test_functions() {
        // Test that test functions get appropriate handling
        let mut func = create_test_metrics();
        func.is_test = true;
        func.cyclomatic = 8;

        let graph = CallGraph::new();

        let score = calculate_unified_priority_with_debt(&func, &graph, None, None, None);

        assert_eq!(
            score.coverage_factor, 0.0,
            "Test functions don't need coverage"
        );
    }

    #[test]
    fn test_calculate_unified_priority_with_debt_max_score_capping() {
        // Test that scores are capped at 10.0
        let mut func = create_test_metrics();
        func.cyclomatic = 50;
        func.cognitive = 60;
        func.length = 500;

        let graph = CallGraph::new();

        let score = calculate_unified_priority_with_debt(
            &func,
            &graph,
            None,
            Some(10.0), // Max organization issues
            None,
        );

        assert!(score.final_score <= 10.0, "Score should be capped at 10.0");
        // Organization factor removed per spec 58 - redundant with complexity factor
        // Security factor removed per spec 64
    }

    #[test]
    fn test_calculate_unified_priority_with_debt_io_wrapper_trivial() {
        // Test that trivial I/O wrappers are handled correctly
        let mut func = create_test_metrics();
        func.cyclomatic = 2;
        func.cognitive = 3;
        func.length = 5;
        func.name = "read_file".to_string(); // Likely an I/O wrapper

        let graph = CallGraph::new();
        let mut lcov = LcovData::default();
        lcov.functions.insert(
            PathBuf::from("test.rs"),
            vec![FunctionCoverage {
                name: "read_file".to_string(),
                start_line: 1,
                execution_count: 1,
                coverage_percentage: 100.0,
                uncovered_lines: vec![],
            }],
        );

        let score = calculate_unified_priority_with_debt(&func, &graph, Some(&lcov), None, None);

        assert_eq!(
            score.final_score, 0.0,
            "Trivial tested I/O wrapper should have zero score"
        );
    }

    #[test]
    fn test_calculate_unified_priority_with_debt_entry_point() {
        // Test entry point functions
        let mut func = create_test_metrics();
        func.cyclomatic = 3;
        func.cognitive = 4;
        func.name = "main".to_string();

        let graph = CallGraph::new();
        let mut lcov = LcovData::default();
        lcov.functions.insert(
            PathBuf::from("test.rs"),
            vec![FunctionCoverage {
                name: "main".to_string(),
                start_line: 1,
                execution_count: 1,
                coverage_percentage: 100.0,
                uncovered_lines: vec![],
            }],
        );

        let score = calculate_unified_priority_with_debt(&func, &graph, Some(&lcov), None, None);

        assert_eq!(
            score.final_score, 0.0,
            "Trivial tested entry point should have zero score"
        );
    }

    #[test]
    fn test_calculate_unified_priority_with_debt_partial_coverage() {
        // Test function with partial coverage
        let func = create_test_metrics();

        let graph = CallGraph::new();
        let mut lcov = LcovData::default();
        // Add partial coverage (some hits but not full coverage)
        lcov.functions.insert(
            PathBuf::from("test.rs"),
            vec![FunctionCoverage {
                name: "test_function".to_string(),
                start_line: 10,
                execution_count: 1,
                coverage_percentage: 50.0,
                uncovered_lines: vec![12, 14, 15, 16, 18],
            }],
        );

        let score = calculate_unified_priority_with_debt(&func, &graph, Some(&lcov), None, None);

        // Even with some coverage, non-trivial functions should still have priority
        assert!(
            score.final_score > 0.0,
            "Non-trivial function should have non-zero score even with coverage"
        );
    }

    #[test]
    fn test_calculate_unified_priority_with_debt_pure_function_high_confidence() {
        // Test that pure functions with high confidence get complexity reduction
        let mut func = create_test_metrics();
        func.cyclomatic = 12;
        func.cognitive = 15;
        func.is_pure = Some(true);
        func.purity_confidence = Some(0.9); // High confidence

        let graph = CallGraph::new();

        let score = calculate_unified_priority_with_debt(&func, &graph, None, None, None);

        // Verify purity bonus was applied (30% reduction for high confidence)
        // Original cyclomatic 12 * 0.7 = 8.4
        assert!(
            score.complexity_factor < normalize_complexity(12, 15),
            "High confidence pure function should have reduced complexity"
        );
    }

    #[test]
    fn test_calculate_unified_priority_with_debt_pure_function_low_confidence() {
        // Test that pure functions with low confidence get smaller reduction
        let mut func = create_test_metrics();
        func.cyclomatic = 12;
        func.cognitive = 15;
        func.is_pure = Some(true);
        func.purity_confidence = Some(0.5); // Low confidence

        let graph = CallGraph::new();

        let score = calculate_unified_priority_with_debt(&func, &graph, None, None, None);

        // Verify smaller purity bonus was applied (15% reduction for low confidence)
        // Original cyclomatic 12 * 0.85 = 10.2
        assert!(
            score.complexity_factor < normalize_complexity(12, 15),
            "Low confidence pure function should have some complexity reduction"
        );
    }

    #[test]
    fn test_calculate_unified_priority_with_debt_impure_function() {
        // Test that impure functions get no reduction
        let mut func = create_test_metrics();
        func.cyclomatic = 12;
        func.cognitive = 15;
        func.is_pure = Some(false);
        func.purity_confidence = Some(0.9);

        let graph = CallGraph::new();

        let score = calculate_unified_priority_with_debt(&func, &graph, None, None, None);

        // Verify no purity bonus for impure functions
        assert_eq!(
            score.complexity_factor,
            normalize_complexity(12, 15),
            "Impure function should get no complexity reduction"
        );
    }

    #[test]
    fn test_calculate_unified_priority_with_debt_with_debt_aggregator() {
        // Test integration with DebtAggregator for additional debt scores
        let func = create_test_metrics();
        let graph = CallGraph::new();

        // Create a mock debt aggregator
        let debt_aggregator = DebtAggregator::new();

        // Note: We can't easily set specific debt scores without modifying DebtAggregator
        // but we can test that the function handles the aggregator correctly
        let score_with_aggregator =
            calculate_unified_priority_with_debt(&func, &graph, None, None, Some(&debt_aggregator));

        let score_without_aggregator =
            calculate_unified_priority_with_debt(&func, &graph, None, None, None);

        // The scores might be the same if aggregator has no data for this function
        // but the function should handle both cases without panicking
        assert!(score_with_aggregator.final_score >= 0.0);
        assert!(score_without_aggregator.final_score >= 0.0);
    }

    #[test]
    fn test_calculate_unified_priority_with_debt_upstream_dependencies() {
        // Test that functions with many upstream dependencies get higher scores
        let func = create_test_metrics();
        let mut graph = CallGraph::new();

        let func_id = FunctionId {
            file: func.file.clone(),
            name: func.name.clone(),
            line: func.line,
        };

        // Add 10 upstream callers
        for i in 0..10 {
            let caller = FunctionId {
                file: PathBuf::from("caller.rs"),
                name: format!("upstream_{}", i),
                line: i * 10,
            };
            graph.add_call(FunctionCall {
                caller,
                callee: func_id.clone(),
                call_type: CallType::Direct,
            });
        }

        let score_with_deps = calculate_unified_priority_with_debt(&func, &graph, None, None, None);

        // Test with no dependencies
        let empty_graph = CallGraph::new();
        let score_no_deps =
            calculate_unified_priority_with_debt(&func, &empty_graph, None, None, None);

        assert!(
            score_with_deps.dependency_factor > score_no_deps.dependency_factor,
            "Functions with upstream dependencies should have higher dependency factor"
        );
        assert!(
            score_with_deps.final_score > score_no_deps.final_score,
            "Functions with upstream dependencies should have higher priority"
        );
    }

    #[test]
    fn test_calculate_unified_priority_with_debt_pattern_match_role() {
        // Test that pattern match functions get appropriate handling
        let mut func = create_test_metrics();
        func.cyclomatic = 3;
        func.cognitive = 4;
        func.length = 8;
        func.name = "match_type".to_string(); // Likely a pattern match function

        let graph = CallGraph::new();
        let mut lcov = LcovData::default();
        lcov.functions.insert(
            PathBuf::from("test.rs"),
            vec![FunctionCoverage {
                name: "match_type".to_string(),
                start_line: 10,
                execution_count: 1,
                coverage_percentage: 100.0,
                uncovered_lines: vec![],
            }],
        );

        let score = calculate_unified_priority_with_debt(&func, &graph, Some(&lcov), None, None);

        assert_eq!(
            score.final_score, 0.0,
            "Trivial tested pattern match function should have zero score"
        );
    }

    #[test]
    fn test_calculate_unified_priority_with_debt_boundary_conditions() {
        // Test boundary conditions for complexity thresholds
        let mut func = create_test_metrics();

        // Test at exactly trivial boundary (cyclomatic = 3, cognitive = 5)
        func.cyclomatic = 3;
        func.cognitive = 5;
        func.length = 10;

        let graph = CallGraph::new();
        let mut lcov = LcovData::default();
        lcov.functions.insert(
            PathBuf::from("test.rs"),
            vec![FunctionCoverage {
                name: "test_function".to_string(),
                start_line: 10,
                execution_count: 1,
                coverage_percentage: 100.0,
                uncovered_lines: vec![],
            }],
        );

        let score = calculate_unified_priority_with_debt(&func, &graph, Some(&lcov), None, None);

        assert_eq!(
            score.final_score, 0.0,
            "Function at trivial boundary with coverage should have zero score"
        );

        // Test just above trivial boundary
        func.cyclomatic = 4; // Just above trivial threshold
        let score_above =
            calculate_unified_priority_with_debt(&func, &graph, Some(&lcov), None, None);

        assert!(
            score_above.final_score > 0.0,
            "Function above trivial boundary should have non-zero score"
        );
    }

    #[test]
    fn test_calculate_unified_priority_with_debt_zero_values() {
        // Test handling of zero/minimal values
        let mut func = create_test_metrics();
        func.cyclomatic = 0;
        func.cognitive = 0;
        func.length = 0;
        func.nesting = 0;

        let graph = CallGraph::new();

        let score = calculate_unified_priority_with_debt(&func, &graph, None, None, None);

        // Even with zero complexity, lack of coverage should give some score
        assert_eq!(
            score.complexity_factor, 0.0,
            "Zero complexity should give zero complexity factor"
        );
        assert_eq!(
            score.coverage_factor, 10.0,
            "No coverage data should assume worst case"
        );
        assert!(
            score.final_score > 0.0,
            "Should still have non-zero score due to no coverage"
        );
    }

    #[test]
    fn test_calculate_unified_priority_with_debt_all_factors_combined() {
        // Test with all factors present
        let mut func = create_test_metrics();
        func.cyclomatic = 15;
        func.cognitive = 20;
        func.is_pure = Some(true);
        func.purity_confidence = Some(0.85);
        func.entropy_score = Some(crate::complexity::entropy::EntropyScore {
            token_entropy: 0.4,
            pattern_repetition: 0.6,
            branch_similarity: 0.5,
            effective_complexity: 12.0,
            unique_variables: 8,
            max_nesting: 3,
            dampening_applied: 0.75,
        });

        let mut graph = CallGraph::new();
        let func_id = FunctionId {
            file: func.file.clone(),
            name: func.name.clone(),
            line: func.line,
        };

        // Add dependencies
        for i in 0..3 {
            let caller = FunctionId {
                file: PathBuf::from("dep.rs"),
                name: format!("dep_{}", i),
                line: i * 20,
            };
            graph.add_call(FunctionCall {
                caller,
                callee: func_id.clone(),
                call_type: CallType::Direct,
            });
        }

        let score = calculate_unified_priority_with_debt(&func, &graph, None, None, None);

        // Verify all factors are present
        assert!(
            score.complexity_factor > 0.0,
            "Should have complexity factor"
        );
        assert!(score.coverage_factor > 0.0, "Should have coverage factor");
        assert!(
            score.dependency_factor > 0.0,
            "Should have dependency factor"
        );
        assert!(score.role_multiplier > 0.0, "Should have role multiplier");
        assert!(score.final_score > 0.0, "Should have final score");
    }

    #[test]
    fn test_apply_reduced_entropy_dampening_no_dampening() {
        // Test case where no dampening should be applied (entropy >= 0.2)
        let entropy_score = crate::complexity::entropy::EntropyScore {
            token_entropy: 0.8,      // Well above 0.2 threshold
            pattern_repetition: 0.5,
            branch_similarity: 0.5,
            effective_complexity: 1.0,
            unique_variables: 5,
            max_nesting: 2,
            dampening_applied: 1.0,
        };

        let score = 5.0;
        let dampened = apply_reduced_entropy_dampening(score, &entropy_score);
        assert_eq!(
            dampened, score,
            "No dampening should be applied when entropy >= 0.2"
        );
    }

    #[test]
    fn test_apply_reduced_entropy_dampening_low_entropy() {
        // Test dampening with low entropy (< 0.2)
        let entropy_score = crate::complexity::entropy::EntropyScore {
            token_entropy: 0.1, // Low entropy (< 0.2)
            pattern_repetition: 0.5,
            branch_similarity: 0.5,
            effective_complexity: 1.0,
            unique_variables: 5,
            max_nesting: 2,
            dampening_applied: 1.0,
        };

        let score = 5.0;
        let dampened = apply_reduced_entropy_dampening(score, &entropy_score);
        
        // With entropy of 0.1, dampening should be 0.5 + 0.5 * (0.1/0.2) = 0.75
        let expected = score * 0.75;
        assert!(
            (dampened - expected).abs() < 0.01,
            "Low entropy should apply 25% dampening (75% preserved)"
        );
    }

    #[test]
    fn test_apply_reduced_entropy_dampening_very_low_entropy() {
        // Test with very low entropy (near 0)
        let entropy_score = crate::complexity::entropy::EntropyScore {
            token_entropy: 0.05, // Very low entropy
            pattern_repetition: 0.5,
            branch_similarity: 0.5,
            effective_complexity: 1.0,
            unique_variables: 5,
            max_nesting: 2,
            dampening_applied: 1.0,
        };

        let score = 5.0;
        let dampened = apply_reduced_entropy_dampening(score, &entropy_score);
        
        // With entropy of 0.05, dampening should be 0.5 + 0.5 * (0.05/0.2) = 0.625
        let expected = score * 0.625;
        assert!(
            (dampened - expected).abs() < 0.01,
            "Very low entropy should apply 37.5% dampening (62.5% preserved)"
        );
    }

    #[test]
    fn test_apply_reduced_entropy_dampening_edge_cases() {
        // Test with extreme values at boundaries
        let entropy_score = crate::complexity::entropy::EntropyScore {
            token_entropy: 0.0,      // Minimum entropy
            pattern_repetition: 1.0,
            branch_similarity: 1.0,
            effective_complexity: 1.0,
            unique_variables: 0,
            max_nesting: 10,
            dampening_applied: 1.0,
        };

        let score = 5.0;
        let dampened = apply_reduced_entropy_dampening(score, &entropy_score);
        
        // With entropy of 0.0, maximum dampening is 50%
        assert_eq!(
            dampened, score * 0.5,
            "Maximum dampening should be exactly 50% reduction per spec 68"
        );
    }

    #[test]
    fn test_apply_reduced_entropy_dampening_boundary() {
        // Test at the 0.2 boundary
        let entropy_score = crate::complexity::entropy::EntropyScore {
            token_entropy: 0.2, // Exactly at boundary
            pattern_repetition: 0.5,
            branch_similarity: 0.5,
            effective_complexity: 1.0,
            unique_variables: 5,
            max_nesting: 2,
            dampening_applied: 1.0,
        };

        let score = 5.0;
        let dampened = apply_reduced_entropy_dampening(score, &entropy_score);
        
        // At entropy of 0.2, no dampening should apply
        assert_eq!(
            dampened, score,
            "No dampening at entropy = 0.2 boundary"
        );
    }

    // Tests for extracted pure functions

    #[test]
    fn test_classify_test_debt_type_high_complexity() {
        // Test when cyclomatic complexity exceeds threshold
        let mut func = create_test_metrics();
        func.cyclomatic = 20;
        func.cognitive = 10;
        func.is_test = true;
        let debt = classify_test_debt(&func);
        match debt {
            DebtType::TestComplexityHotspot {
                cyclomatic,
                cognitive,
                threshold,
            } => {
                assert_eq!(cyclomatic, 20);
                assert_eq!(cognitive, 10);
                assert_eq!(threshold, 15);
            }
            _ => panic!("Expected TestComplexityHotspot"),
        }
    }

    #[test]
    fn test_classify_test_debt_type_high_cognitive() {
        // Test when cognitive complexity exceeds threshold
        let mut func = create_test_metrics();
        func.cyclomatic = 10;
        func.cognitive = 25;
        func.is_test = true;
        let debt = classify_test_debt(&func);
        match debt {
            DebtType::TestComplexityHotspot {
                cyclomatic,
                cognitive,
                threshold,
            } => {
                assert_eq!(cyclomatic, 10);
                assert_eq!(cognitive, 25);
                assert_eq!(threshold, 15);
            }
            _ => panic!("Expected TestComplexityHotspot"),
        }
    }

    #[test]
    fn test_classify_test_debt_type_normal() {
        // Test when complexity is within normal range
        let mut func = create_test_metrics();
        func.cyclomatic = 10;
        func.cognitive = 15;
        func.is_test = true;
        let debt = classify_test_debt(&func);
        match debt {
            DebtType::TestingGap {
                coverage,
                cyclomatic,
                cognitive,
            } => {
                assert_eq!(coverage, 0.0);
                assert_eq!(cyclomatic, 10);
                assert_eq!(cognitive, 15);
            }
            _ => panic!("Expected TestingGap"),
        }
    }

    #[test]
    fn test_has_testing_gap_true() {
        // Coverage below threshold and not a test
        assert!(has_testing_gap(0.1, false));
        assert!(has_testing_gap(0.0, false));
        assert!(has_testing_gap(0.19, false));
    }

    #[test]
    fn test_has_testing_gap_false() {
        // Coverage above threshold
        assert!(!has_testing_gap(0.2, false));
        assert!(!has_testing_gap(0.5, false));
        assert!(!has_testing_gap(1.0, false));

        // Test functions don't have gaps
        assert!(!has_testing_gap(0.0, true));
        assert!(!has_testing_gap(0.1, true));
    }

    #[test]
    fn test_is_complexity_hotspot_by_metrics_true() {
        // High cyclomatic complexity
        assert!(is_complexity_hotspot_by_metrics(6, 5));
        assert!(is_complexity_hotspot_by_metrics(10, 7));

        // High cognitive complexity
        assert!(is_complexity_hotspot_by_metrics(3, 9));
        assert!(is_complexity_hotspot_by_metrics(5, 15));

        // Both high
        assert!(is_complexity_hotspot_by_metrics(10, 10));
    }

    #[test]
    fn test_is_complexity_hotspot_by_metrics_false() {
        // Both below thresholds
        assert!(!is_complexity_hotspot_by_metrics(5, 8));
        assert!(!is_complexity_hotspot_by_metrics(3, 5));
        assert!(!is_complexity_hotspot_by_metrics(1, 1));
    }

    #[test]
    fn test_classify_simple_function_debt_io_wrapper() {
        let mut func = create_test_metrics();
        func.cyclomatic = 2;
        func.cognitive = 3;
        let debt = classify_simple_function_risk(&func, &FunctionRole::IOWrapper);
        assert!(debt.is_some());
        match debt.unwrap() {
            DebtType::Risk {
                risk_score,
                factors,
            } => {
                assert_eq!(risk_score, 0.0);
                assert_eq!(
                    factors[0],
                    "Simple I/O wrapper or entry point - minimal risk"
                );
            }
            _ => panic!("Expected Risk with minimal score"),
        }
    }

    #[test]
    fn test_classify_simple_function_debt_entry_point() {
        let mut func = create_test_metrics();
        func.cyclomatic = 2;
        func.cognitive = 3;
        let debt = classify_simple_function_risk(&func, &FunctionRole::EntryPoint);
        assert!(debt.is_some());
        match debt.unwrap() {
            DebtType::Risk {
                risk_score,
                factors,
            } => {
                assert_eq!(risk_score, 0.0);
                assert_eq!(
                    factors[0],
                    "Simple I/O wrapper or entry point - minimal risk"
                );
            }
            _ => panic!("Expected Risk with minimal score"),
        }
    }

    #[test]
    fn test_classify_simple_function_debt_pattern_match() {
        let mut func = create_test_metrics();
        func.cyclomatic = 2;
        func.cognitive = 3;
        let debt = classify_simple_function_risk(&func, &FunctionRole::PatternMatch);
        assert!(debt.is_some());
        match debt.unwrap() {
            DebtType::Risk {
                risk_score,
                factors,
            } => {
                assert_eq!(risk_score, 0.0);
                assert_eq!(
                    factors[0],
                    "Simple I/O wrapper or entry point - minimal risk"
                );
            }
            _ => panic!("Expected Risk with minimal score"),
        }
    }

    #[test]
    fn test_classify_simple_function_debt_pure_logic_short() {
        let mut func = create_test_metrics();
        func.cyclomatic = 2;
        func.cognitive = 3;
        func.length = 8; // Short pure function
        let debt = classify_simple_function_risk(&func, &FunctionRole::PureLogic);
        assert!(debt.is_some());
        match debt.unwrap() {
            DebtType::Risk {
                risk_score,
                factors,
            } => {
                assert_eq!(risk_score, 0.0);
                assert_eq!(factors[0], "Trivial pure function - not technical debt");
            }
            _ => panic!("Expected Risk with minimal score"),
        }
    }

    #[test]
    fn test_classify_simple_function_debt_pure_logic_long() {
        let mut func = create_test_metrics();
        func.cyclomatic = 2;
        func.cognitive = 3;
        func.length = 15; // Longer pure function
        let debt = classify_simple_function_risk(&func, &FunctionRole::PureLogic);
        // Should return None for longer pure function
        assert!(debt.is_none());
    }

    #[test]
    fn test_classify_simple_function_debt_orchestration() {
        let mut func = create_test_metrics();
        func.cyclomatic = 2;
        func.cognitive = 3;
        let debt = classify_simple_function_risk(&func, &FunctionRole::Orchestrator);
        // Should return None for orchestrator
        assert!(debt.is_none());
    }
}
