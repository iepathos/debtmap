//! Construction module - Functions for creating UnifiedDebtItem instances
//!
//! This module contains all the construction and builder functions for creating
//! UnifiedDebtItem instances from various sources with different configurations.

use crate::config::get_context_multipliers;
use crate::context::{detect_file_type, FileType};
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
use std::path::Path;

/// Calculate context-aware multiplier for a file path (spec 191)
///
/// Returns a tuple of (multiplier, file_type) based on the detected file type.
/// Non-production code (examples, tests, benchmarks) gets dampened multipliers.
fn calculate_context_multiplier(file_path: &Path) -> (f64, FileType) {
    let file_type = detect_file_type(file_path);
    let config = get_context_multipliers();

    // If context dampening is disabled, return 1.0 for all files
    if !config.enable_context_dampening {
        return (1.0, file_type);
    }

    let multiplier = match file_type {
        FileType::Example => config.examples,
        FileType::Test => config.tests,
        FileType::Benchmark => config.benchmarks,
        FileType::BuildScript => config.build_scripts,
        FileType::Documentation => config.documentation,
        FileType::Production | FileType::Configuration => 1.0, // No dampening for production code
    };

    (multiplier, file_type)
}

/// Apply context multiplier to a UnifiedScore (spec 191)
fn apply_context_multiplier_to_score(mut score: UnifiedScore, multiplier: f64) -> UnifiedScore {
    // Apply multiplier to final_score and all contributing factors
    score.final_score *= multiplier;
    score.complexity_factor *= multiplier;
    score.coverage_factor *= multiplier;
    score.dependency_factor *= multiplier;

    // Also apply to base_score if present
    if let Some(base) = score.base_score {
        score.base_score = Some(base * multiplier);
    }

    // Apply to pre_adjustment_score if present
    if let Some(pre_adj) = score.pre_adjustment_score {
        score.pre_adjustment_score = Some(pre_adj * multiplier);
    }

    score
}

/// Create a unified debt item with enhanced call graph analysis (spec 201)
/// Returns None if the debt pattern doesn't warrant a recommendation (e.g., clean dispatcher)
pub fn create_unified_debt_item_enhanced(
    func: &FunctionMetrics,
    call_graph: &CallGraph,
    _enhanced_call_graph: Option<()>, // Placeholder for future enhanced call graph
    coverage: Option<&LcovData>,
) -> Option<UnifiedDebtItem> {
    let func_id = FunctionId::new(func.file.clone(), func.name.clone(), func.line);

    // Security factor removed per spec 64
    // Organization factor removed per spec 58 - redundant with complexity factor

    let mut unified_score = calculate_unified_priority(
        func, call_graph, coverage, None, // Organization factor no longer used
    );

    // Apply context-aware dampening (spec 191)
    let (context_multiplier, context_type) = calculate_context_multiplier(&func.file);
    unified_score = apply_context_multiplier_to_score(unified_score, context_multiplier);

    let role = classify_function_role(func, &func_id, call_graph);

    let transitive_coverage =
        coverage.map(|cov| calculate_transitive_coverage(&func_id, call_graph, cov));

    // Use enhanced debt type classification
    let debt_type = classify_debt_type_enhanced(func, call_graph, &func_id);

    // Generate recommendation (spec 201: may return None for clean dispatchers)
    let recommendation = generate_recommendation(func, &debt_type, role, &unified_score)?;
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

    // Detect complexity pattern once during construction (spec 204)
    let detected_pattern =
        crate::priority::detected_pattern::DetectedPattern::detect(&func.language_specific);

    // Calculate entropy details once for efficiency (spec 214)
    let entropy_details = calculate_entropy_details(func);

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
        entropy_details: entropy_details.clone(),
        entropy_adjusted_cyclomatic: entropy_details.as_ref().map(|e| e.adjusted_complexity),
        entropy_adjusted_cognitive: entropy_details.as_ref().map(|e| e.adjusted_cognitive),
        entropy_dampening_factor: entropy_details.as_ref().map(|e| e.dampening_factor),
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
        context_multiplier: Some(context_multiplier), // Context dampening multiplier (spec 191)
        context_type: Some(context_type),             // Detected file type (spec 191)
        language_specific: func.language_specific.clone(), // State machine/coordinator signals (spec 190)
        detected_pattern,                                  // Detected complexity pattern (spec 204)
        contextual_risk: None,
    };

    // Apply exponential scaling and risk boosting (spec 171)
    Some(apply_score_scaling(item))
}

pub fn create_unified_debt_item_with_aggregator(
    func: &FunctionMetrics,
    call_graph: &CallGraph,
    coverage: Option<&LcovData>,
    framework_exclusions: &HashSet<FunctionId>,
    function_pointer_used_functions: Option<&HashSet<FunctionId>>,
    debt_aggregator: &DebtAggregator,
) -> Option<UnifiedDebtItem> {
    use std::path::Path;
    create_unified_debt_item_with_aggregator_and_data_flow(
        func,
        call_graph,
        coverage,
        framework_exclusions,
        function_pointer_used_functions,
        debt_aggregator,
        None,           // DataFlowGraph will be provided by the new function
        None,           // No risk analyzer in wrapper function
        Path::new("."), // Default project path
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

// Pure function: Perform debt analysis (spec 201)
/// Returns None if the debt pattern doesn't warrant a recommendation (e.g., clean dispatcher)
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
) -> Option<DebtAnalysisContext> {
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

    // Generate recommendation (spec 201: may return None for clean dispatchers)
    let recommendation = generate_recommendation_with_coverage_and_data_flow(
        func,
        &debt_type,
        function_role,
        &unified_score,
        &transitive_coverage,
        data_flow,
    )?;

    // Calculate expected impact
    let expected_impact = calculate_expected_impact(func, &debt_type, &unified_score);

    Some(DebtAnalysisContext {
        func_id: func_id.clone(),
        debt_type,
        unified_score,
        function_role,
        transitive_coverage,
        recommendation,
        expected_impact,
    })
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
    mut context: DebtAnalysisContext,
    deps: DependencyMetrics,
) -> UnifiedDebtItem {
    // Apply context-aware dampening (spec 191)
    let (context_multiplier, context_type) = calculate_context_multiplier(&func.file);
    context.unified_score =
        apply_context_multiplier_to_score(context.unified_score, context_multiplier);

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

    // Detect complexity pattern once during construction (spec 204)
    let detected_pattern =
        crate::priority::detected_pattern::DetectedPattern::detect(&func.language_specific);

    // Calculate entropy details once for efficiency (spec 214)
    let entropy_details = calculate_entropy_details(func);

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
        entropy_details: entropy_details.clone(),
        entropy_adjusted_cyclomatic: entropy_details.as_ref().map(|e| e.adjusted_complexity),
        entropy_adjusted_cognitive: entropy_details.as_ref().map(|e| e.adjusted_cognitive),
        entropy_dampening_factor: entropy_details.as_ref().map(|e| e.dampening_factor),
        is_pure: func.is_pure,
        purity_confidence: func.purity_confidence,
        purity_level: None,
        god_object_indicators: None,
        tier: None,
        function_context: Some(context_analysis.context),
        context_confidence: Some(context_analysis.confidence),
        contextual_recommendation,
        pattern_analysis: None, // Pattern analysis added in spec 151, populated when available
        context_multiplier: Some(context_multiplier),
        context_type: Some(context_type),
        language_specific: func.language_specific.clone(), // State machine/coordinator signals (spec 190)
        detected_pattern,                                  // Detected complexity pattern (spec 204)
        contextual_risk: None,
    }
}

// Main function using functional composition (spec 201)
/// Returns None if the debt pattern doesn't warrant a recommendation (e.g., clean dispatcher)
#[allow(clippy::too_many_arguments)]
pub fn create_unified_debt_item_with_aggregator_and_data_flow(
    func: &FunctionMetrics,
    call_graph: &CallGraph,
    coverage: Option<&LcovData>,
    framework_exclusions: &HashSet<FunctionId>,
    function_pointer_used_functions: Option<&HashSet<FunctionId>>,
    debt_aggregator: &DebtAggregator,
    data_flow: Option<&crate::data_flow::DataFlowGraph>,
    risk_analyzer: Option<&crate::risk::RiskAnalyzer>,
    project_path: &Path,
) -> Option<UnifiedDebtItem> {
    // Step 1: Create function ID (pure)
    let func_id = create_function_id(func);

    // Step 2: Analyze debt (pure) - may return None for clean dispatchers (spec 201)
    let context = analyze_debt(
        func,
        &func_id,
        call_graph,
        coverage,
        framework_exclusions,
        function_pointer_used_functions,
        debt_aggregator,
        data_flow,
    )?;

    // Step 3: Extract dependencies (pure)
    let deps = extract_dependency_metrics(&func_id, call_graph);

    // Step 4: Build final item (pure)
    let mut item = build_unified_debt_item(func, context, deps);

    // Step 4.5: Analyze contextual risk if risk analyzer is provided (spec 202)
    if let Some(analyzer) = risk_analyzer {
        let complexity_metrics = crate::core::ComplexityMetrics::from_function(func);
        let func_coverage = coverage
            .and_then(|cov| cov.get_function_coverage_with_line(&func.file, &func.name, func.line));

        // Call analyze_function_with_context to get contextual risk
        let (_, contextual_risk) = analyzer.analyze_function_with_context(
            func.file.clone(),
            func.name.clone(),
            (func.line, func.line + func.length),
            &complexity_metrics,
            func_coverage,
            func.is_test,
            project_path.to_path_buf(),
        );

        item.contextual_risk = contextual_risk;
    }

    // Step 5: Apply exponential scaling and risk boosting (spec 171)
    Some(apply_score_scaling(item))
}

pub fn create_unified_debt_item_with_exclusions(
    func: &FunctionMetrics,
    call_graph: &CallGraph,
    coverage: Option<&LcovData>,
    framework_exclusions: &HashSet<FunctionId>,
    function_pointer_used_functions: Option<&HashSet<FunctionId>>,
) -> Option<UnifiedDebtItem> {
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
) -> Option<UnifiedDebtItem> {
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

    let mut unified_score = calculate_unified_priority(
        func, call_graph, coverage, None, // Organization factor no longer used
    );

    // Apply context-aware dampening (spec 191)
    let (context_multiplier, context_type) = calculate_context_multiplier(&func.file);
    unified_score = apply_context_multiplier_to_score(unified_score, context_multiplier);

    // Determine function role for more accurate analysis
    let function_role = classify_function_role(func, &func_id, call_graph);

    // Generate contextual recommendation based on debt type and metrics (spec 201)
    // May return None for clean dispatchers
    let recommendation = generate_recommendation_with_coverage_and_data_flow(
        func,
        &debt_type,
        function_role,
        &unified_score,
        &transitive_coverage,
        data_flow,
    )?;

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

    // Detect complexity pattern once during construction (spec 204)
    let detected_pattern =
        crate::priority::detected_pattern::DetectedPattern::detect(&func.language_specific);

    // Calculate entropy details once for efficiency (spec 214)
    let entropy_details = calculate_entropy_details(func);

    Some(UnifiedDebtItem {
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
        entropy_details: entropy_details.clone(),
        entropy_adjusted_cyclomatic: entropy_details.as_ref().map(|e| e.adjusted_complexity),
        entropy_adjusted_cognitive: entropy_details.as_ref().map(|e| e.adjusted_cognitive),
        entropy_dampening_factor: entropy_details.as_ref().map(|e| e.dampening_factor),
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
        context_multiplier: Some(context_multiplier),
        context_type: Some(context_type),
        language_specific: func.language_specific.clone(), // State machine/coordinator signals (spec 190)
        detected_pattern,                                  // Detected complexity pattern (spec 204)
        contextual_risk: None,
    })
}

/// Create a unified debt item for a function (spec 201)
/// Returns None if the debt pattern doesn't warrant a recommendation (e.g., clean dispatcher)
pub fn create_unified_debt_item(
    func: &FunctionMetrics,
    call_graph: &CallGraph,
    coverage: Option<&LcovData>,
) -> Option<UnifiedDebtItem> {
    create_unified_debt_item_with_data_flow(func, call_graph, coverage, None)
}

pub fn create_unified_debt_item_with_data_flow(
    func: &FunctionMetrics,
    call_graph: &CallGraph,
    coverage: Option<&LcovData>,
    data_flow: Option<&crate::data_flow::DataFlowGraph>,
) -> Option<UnifiedDebtItem> {
    let func_id = FunctionId::new(func.file.clone(), func.name.clone(), func.line);

    // Security factor removed per spec 64
    // Organization factor removed per spec 58 - redundant with complexity factor

    let mut unified_score = calculate_unified_priority(
        func, call_graph, coverage, None, // Organization factor no longer used
    );

    // Apply context-aware dampening (spec 191)
    let (context_multiplier, context_type) = calculate_context_multiplier(&func.file);
    unified_score = apply_context_multiplier_to_score(unified_score, context_multiplier);

    let role = classify_function_role(func, &func_id, call_graph);

    let transitive_coverage =
        coverage.map(|cov| calculate_transitive_coverage(&func_id, call_graph, cov));

    let debt_type = determine_debt_type(func, &transitive_coverage, call_graph, &func_id);
    // Generate recommendation (spec 201: may return None for clean dispatchers)
    let recommendation = generate_recommendation_with_coverage_and_data_flow(
        func,
        &debt_type,
        role,
        &unified_score,
        &transitive_coverage,
        data_flow,
    )?;
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

    // Detect complexity pattern once during construction (spec 204)
    let detected_pattern =
        crate::priority::detected_pattern::DetectedPattern::detect(&func.language_specific);

    Some(UnifiedDebtItem {
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
        entropy_details: entropy_details.clone(),
        entropy_adjusted_cyclomatic: entropy_details.as_ref().map(|e| e.adjusted_complexity),
        entropy_adjusted_cognitive: entropy_details.as_ref().map(|e| e.adjusted_cognitive),
        entropy_dampening_factor: entropy_details.as_ref().map(|e| e.dampening_factor),
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
        context_multiplier: Some(context_multiplier),
        context_type: Some(context_type),
        language_specific: func.language_specific.clone(), // State machine/coordinator signals (spec 190)
        detected_pattern,                                  // Detected complexity pattern (spec 204)
        contextual_risk: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::FileType;
    use std::path::PathBuf;

    #[test]
    fn test_calculate_context_multiplier_for_example() {
        let path = PathBuf::from("examples/demo.rs");
        let (multiplier, file_type) = calculate_context_multiplier(&path);

        assert_eq!(file_type, FileType::Example);
        assert_eq!(multiplier, 0.1); // 90% reduction
    }

    #[test]
    fn test_calculate_context_multiplier_for_test() {
        let path = PathBuf::from("tests/integration_test.rs");
        let (multiplier, file_type) = calculate_context_multiplier(&path);

        assert_eq!(file_type, FileType::Test);
        assert_eq!(multiplier, 0.2); // 80% reduction
    }

    #[test]
    fn test_calculate_context_multiplier_for_benchmark() {
        let path = PathBuf::from("benches/perf.rs");
        let (multiplier, file_type) = calculate_context_multiplier(&path);

        assert_eq!(file_type, FileType::Benchmark);
        assert_eq!(multiplier, 0.3); // 70% reduction
    }

    #[test]
    fn test_calculate_context_multiplier_for_build_script() {
        let path = PathBuf::from("build.rs");
        let (multiplier, file_type) = calculate_context_multiplier(&path);

        assert_eq!(file_type, FileType::BuildScript);
        assert_eq!(multiplier, 0.3); // 70% reduction
    }

    #[test]
    fn test_calculate_context_multiplier_for_production() {
        let path = PathBuf::from("src/main.rs");
        let (multiplier, file_type) = calculate_context_multiplier(&path);

        assert_eq!(file_type, FileType::Production);
        assert_eq!(multiplier, 1.0); // No reduction
    }

    #[test]
    fn test_apply_context_multiplier_to_score() {
        let original_score = UnifiedScore {
            complexity_factor: 8.0,
            coverage_factor: 10.0,
            dependency_factor: 6.0,
            role_multiplier: 1.0,
            final_score: 24.0,
            base_score: Some(20.0),
            exponential_factor: None,
            risk_boost: None,
            pre_adjustment_score: Some(22.0),
            adjustment_applied: None,
        };

        let adjusted = apply_context_multiplier_to_score(original_score, 0.1);

        // All scores should be multiplied by 0.1 (use approximate comparison for floats)
        assert!((adjusted.final_score - 2.4).abs() < 0.0001);
        assert!((adjusted.complexity_factor - 0.8).abs() < 0.0001);
        assert!((adjusted.coverage_factor - 1.0).abs() < 0.0001);
        assert!((adjusted.dependency_factor - 0.6).abs() < 0.0001);
        assert!(adjusted.base_score.is_some());
        assert!((adjusted.base_score.unwrap() - 2.0).abs() < 0.0001);
        assert!(adjusted.pre_adjustment_score.is_some());
        assert!((adjusted.pre_adjustment_score.unwrap() - 2.2).abs() < 0.0001);
    }

    #[test]
    fn test_context_multiplier_never_increases_score() {
        let original_score = UnifiedScore {
            complexity_factor: 5.0,
            coverage_factor: 5.0,
            dependency_factor: 5.0,
            role_multiplier: 1.0,
            final_score: 15.0,
            base_score: None,
            exponential_factor: None,
            risk_boost: None,
            pre_adjustment_score: None,
            adjustment_applied: None,
        };

        // Test with all file types
        for multiplier in &[0.1, 0.2, 0.3, 1.0] {
            let adjusted = apply_context_multiplier_to_score(original_score.clone(), *multiplier);
            assert!(adjusted.final_score <= original_score.final_score);
        }
    }
}
