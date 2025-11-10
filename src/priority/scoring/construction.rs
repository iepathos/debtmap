//! Construction module - Functions for creating UnifiedDebtItem instances
//!
//! This module contains all the construction and builder functions for creating
//! UnifiedDebtItem instances from various sources with different configurations.

use crate::core::FunctionMetrics;
use crate::priority::unified_scorer::{
    calculate_unified_priority, calculate_unified_priority_with_debt,
};
use crate::priority::{
    call_graph::{CallGraph, FunctionId},
    coverage_propagation::calculate_transitive_coverage,
    debt_aggregator::DebtAggregator,
    scoring::debt_item::{
        calculate_entropy_details, calculate_expected_impact, classify_debt_type_enhanced,
        classify_debt_type_with_exclusions, determine_debt_type, generate_recommendation,
        generate_recommendation_with_coverage_and_data_flow,
    },
    semantic_classifier::classify_function_role,
    ActionableRecommendation, DebtType, FunctionRole, ImpactMetrics, Location, TransitiveCoverage,
    UnifiedDebtItem, UnifiedScore,
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
    let func_id = FunctionId::new(func.file.clone(), func.name.clone(), func.line);

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

    // Detect function context (spec 122)
    let context_detector = crate::analysis::ContextDetector::new();
    let context_analysis = context_detector.detect_context(func, &func.file);

    // Generate contextual recommendation if confidence is high enough (spec 122)
    let contextual_recommendation = if context_analysis.confidence > 0.6 {
        let engine = crate::priority::scoring::ContextRecommendationEngine::new();
        Some(engine.generate_recommendation(
            func,
            context_analysis.context,
            context_analysis.confidence,
            unified_score.final_score,
        ))
    } else {
        None
    };

    let item = UnifiedDebtItem {
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
        purity_level: None,
        god_object_indicators: None,
        tier: None,
        function_context: Some(context_analysis.context),
        context_confidence: Some(context_analysis.confidence),
        contextual_recommendation,
        pattern_analysis: None, // Pattern analysis added in spec 151, populated when available
        file_context: None,
    };

    // Apply exponential scaling and risk boosting (spec 171)
    apply_score_scaling(item)
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
pub(crate) fn create_function_id(func: &FunctionMetrics) -> FunctionId {
    FunctionId::new(func.file.clone(), func.name.clone(), func.line)
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
    let has_coverage_data = coverage.is_some();
    let unified_score = calculate_unified_priority_with_debt(
        func,
        call_graph,
        coverage,
        None,
        Some(debt_aggregator),
        has_coverage_data,
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

// Apply exponential scaling and risk boosting to a debt item (spec 171)
fn apply_score_scaling(mut item: UnifiedDebtItem) -> UnifiedDebtItem {
    use crate::priority::scoring::scaling::{calculate_final_score, ScalingConfig};

    let config = ScalingConfig::default();
    let base_score = item.unified_score.final_score;

    // Calculate final score with scaling
    let (final_score, exponent, boost) =
        calculate_final_score(base_score, &item.debt_type, &item, &config);

    // Update the unified score with scaling information
    item.unified_score.base_score = Some(base_score);
    item.unified_score.exponential_factor = Some(exponent);
    item.unified_score.risk_boost = Some(boost);
    item.unified_score.final_score = final_score;

    item
}

// Pure function: Build unified debt item from components
fn build_unified_debt_item(
    func: &FunctionMetrics,
    context: DebtAnalysisContext,
    deps: DependencyMetrics,
) -> UnifiedDebtItem {
    // Detect function context (spec 122)
    let context_detector = crate::analysis::ContextDetector::new();
    let context_analysis = context_detector.detect_context(func, &func.file);

    // Generate contextual recommendation if confidence is high enough (spec 122)
    let contextual_recommendation = if context_analysis.confidence > 0.6 {
        let engine = crate::priority::scoring::ContextRecommendationEngine::new();
        Some(engine.generate_recommendation(
            func,
            context_analysis.context,
            context_analysis.confidence,
            context.unified_score.final_score,
        ))
    } else {
        None
    };

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
        file_context: None,
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
        purity_level: None,
        god_object_indicators: None,
        tier: None,
        function_context: Some(context_analysis.context),
        context_confidence: Some(context_analysis.confidence),
        contextual_recommendation,
        pattern_analysis: None, // Pattern analysis added in spec 151, populated when available
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
    let item = build_unified_debt_item(func, context, deps);

    // Step 5: Apply exponential scaling and risk boosting (spec 171)
    apply_score_scaling(item)
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
    let func_id = FunctionId::new(func.file.clone(), func.name.clone(), func.line);

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

    // Detect function context (spec 122)
    let context_detector = crate::analysis::ContextDetector::new();
    let context_analysis = context_detector.detect_context(func, &func.file);

    // Generate contextual recommendation if confidence is high enough (spec 122)
    let contextual_recommendation = if context_analysis.confidence > 0.6 {
        let engine = crate::priority::scoring::ContextRecommendationEngine::new();
        Some(engine.generate_recommendation(
            func,
            context_analysis.context,
            context_analysis.confidence,
            unified_score.final_score,
        ))
    } else {
        None
    };

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
        purity_level: None,
        god_object_indicators: None,
        tier: None,
        function_context: Some(context_analysis.context),
        context_confidence: Some(context_analysis.confidence),
        contextual_recommendation,
        pattern_analysis: None, // Pattern analysis added in spec 151, populated when available
        file_context: None,
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
    let func_id = FunctionId::new(func.file.clone(), func.name.clone(), func.line);

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

    // Detect function context (spec 122)
    let context_detector = crate::analysis::ContextDetector::new();
    let context_analysis = context_detector.detect_context(func, &func.file);

    // Generate contextual recommendation if confidence is high enough (spec 122)
    let contextual_recommendation = if context_analysis.confidence > 0.6 {
        let engine = crate::priority::scoring::ContextRecommendationEngine::new();
        Some(engine.generate_recommendation(
            func,
            context_analysis.context,
            context_analysis.confidence,
            unified_score.final_score,
        ))
    } else {
        None
    };

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
        purity_level: None,
        god_object_indicators: None,
        tier: None,
        function_context: Some(context_analysis.context),
        context_confidence: Some(context_analysis.confidence),
        contextual_recommendation,
        pattern_analysis: None, // Pattern analysis added in spec 151, populated when available
        file_context: None,
    }
}
