//! Construction module - Functions for creating UnifiedDebtItem instances
//!
//! This module contains all the construction and builder functions for creating
//! UnifiedDebtItem instances from various sources with different configurations.

use crate::analysis::ContextDetector;
use crate::config::{get_context_multipliers, get_data_flow_scoring_config};
use crate::context::{detect_file_type, FileType};
use crate::core::FunctionMetrics;
use crate::priority::score_types::Score0To100;
use crate::priority::scoring::ContextRecommendationEngine;
use crate::priority::unified_scorer::{
    calculate_unified_priority, calculate_unified_priority_with_data_flow,
    calculate_unified_priority_with_debt,
};
use crate::priority::{
    call_graph::{CallGraph, FunctionId},
    coverage_propagation::calculate_transitive_coverage,
    debt_aggregator::DebtAggregator,
    scoring::debt_item::{
        calculate_entropy_details, calculate_expected_impact, classify_debt_type_enhanced,
        classify_debt_type_with_exclusions, generate_recommendation,
        generate_recommendation_with_coverage_and_data_flow,
    },
    semantic_classifier::classify_function_role,
    ActionableRecommendation, DebtType, FunctionRole, ImpactMetrics, Location, TransitiveCoverage,
    UnifiedDebtItem, UnifiedScore,
};
use crate::risk::lcov::LcovData;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

/// Type alias for file line count cache (spec 195).
pub type FileLineCountCache = HashMap<PathBuf, usize>;

/// Look up cached file line count (pure function, spec 195).
///
/// This is a pure O(1) lookup from the pre-built cache.
/// Falls back to reading file if not in cache (defensive coding).
fn get_file_line_count(file_path: &Path, cache: &FileLineCountCache) -> Option<usize> {
    cache
        .get(file_path)
        .copied()
        .or_else(|| calculate_file_line_count_from_disk(file_path))
}

/// Calculate file line count by reading from disk (fallback for cache miss).
/// Returns None if file cannot be read.
fn calculate_file_line_count_from_disk(file_path: &Path) -> Option<usize> {
    use crate::metrics::LocCounter;
    let loc_counter = LocCounter::default();
    loc_counter
        .count_file(file_path)
        .ok()
        .map(|count| count.physical_lines)
}

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
    score.final_score = Score0To100::new(score.final_score.value() * multiplier);
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

/// Apply contextual risk multiplier to a UnifiedScore (spec 255)
///
/// Adjusts the final score based on git context analysis (churn, recency, etc.).
/// The multiplier is calculated as contextual_risk / base_risk.
/// For example, if contextual_risk is 2x the base_risk, the score is doubled.
pub fn apply_contextual_risk_to_score(
    mut score: UnifiedScore,
    contextual_risk: &crate::risk::context::ContextualRisk,
) -> UnifiedScore {
    // Calculate multiplier from contextual risk
    // If base_risk is 0, no adjustment (avoid division by zero)
    if contextual_risk.base_risk <= 0.0 {
        return score;
    }

    let risk_multiplier = contextual_risk.contextual_risk / contextual_risk.base_risk;

    // Apply multiplier to final_score (clamped to 0-100 by Score0To100)
    let adjusted_final = score.final_score.value() * risk_multiplier;
    score.final_score = Score0To100::new(adjusted_final);

    // Also record the pre-contextual score for transparency
    if score.base_score.is_none() {
        score.base_score = Some(score.final_score.value() / risk_multiplier);
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

    // Use pre-populated call graph data from FunctionMetrics if available,
    // otherwise fall back to querying the call graph directly
    let (upstream_caller_names, downstream_callee_names) =
        if func.upstream_callers.is_some() || func.downstream_callees.is_some() {
            (
                func.upstream_callers.clone().unwrap_or_default(),
                func.downstream_callees.clone().unwrap_or_default(),
            )
        } else {
            // Fallback: query call graph directly
            let upstream_callers = call_graph.get_callers(&func_id);
            let downstream_callees = call_graph.get_callees(&func_id);
            (
                upstream_callers.iter().map(|id| id.name.clone()).collect(),
                downstream_callees
                    .iter()
                    .map(|id| id.name.clone())
                    .collect(),
            )
        };

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
            unified_score.final_score.value(),
        ))
    } else {
        None
    };

    // Detect complexity pattern once during construction (spec 204)
    let detected_pattern =
        crate::priority::detected_pattern::DetectedPattern::detect(&func.language_specific);

    // Calculate entropy details once for efficiency (spec 214)
    let entropy_details = calculate_entropy_details(func);

    // Calculate file line count (this function doesn't use the cache since it's a standalone API)
    let file_line_count = calculate_file_line_count_from_disk(&func.file);

    // Analyze responsibility category during construction (spec 254)
    let responsibility_category =
        crate::organization::god_object::analyze_function_responsibility(&func.name);

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
        upstream_dependencies: upstream_caller_names.len(),
        downstream_dependencies: downstream_callee_names.len(),
        upstream_callers: upstream_caller_names,
        downstream_callees: downstream_callee_names,
        nesting_depth: func.nesting,
        function_length: func.length,
        cyclomatic_complexity: func.cyclomatic,
        cognitive_complexity: func.cognitive,
        entropy_details: entropy_details.clone(),
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
        file_line_count,         // Cached line count (spec 204)
        responsibility_category, // Behavioral responsibility (spec 254)
        error_swallowing_count: func.error_swallowing_count,
        error_swallowing_patterns: func.error_swallowing_patterns.clone(),
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
) -> Vec<UnifiedDebtItem> {
    use std::path::Path;
    // Create empty cache for backward compatibility (will use fallback reads)
    let empty_cache = FileLineCountCache::new();
    // Create detectors for backward compatibility (spec 196: ideally shared at higher level)
    let context_detector = ContextDetector::new();
    let recommendation_engine = ContextRecommendationEngine::new();
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
        &empty_cache,   // Empty cache for backward compatibility
        &context_detector,
        &recommendation_engine,
    )
}

// Pure function: Extract function ID creation
pub(crate) fn create_function_id(func: &FunctionMetrics) -> FunctionId {
    FunctionId::new(func.file.clone(), func.name.clone(), func.line)
}

// Pure function: Calculate coverage data (Spec 203)
// ALWAYS returns Some when coverage is provided, never None
fn calculate_coverage_data(
    func_id: &FunctionId,
    func: &FunctionMetrics,
    call_graph: &CallGraph,
    coverage: Option<&LcovData>,
) -> Option<TransitiveCoverage> {
    coverage.map(|lcov| {
        let end_line = func.line + func.length.saturating_sub(1);
        // get_function_coverage_with_bounds now returns Some(0.0) when not found
        let _direct_coverage =
            lcov.get_function_coverage_with_bounds(&func.file, &func.name, func.line, end_line);
        calculate_transitive_coverage(func_id, call_graph, lcov)
    })
}

// Pure function: Build debt analysis context (spec 228: multi-debt support)
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

// Pure function: Analyze a single debt type and create context (spec 228)
/// Returns None if the debt pattern doesn't warrant a recommendation (e.g., clean dispatcher)
#[allow(clippy::too_many_arguments)]
fn analyze_single_debt_type(
    func: &FunctionMetrics,
    func_id: &FunctionId,
    debt_type: DebtType,
    call_graph: &CallGraph,
    coverage: Option<&LcovData>,
    debt_aggregator: &DebtAggregator,
    data_flow: Option<&crate::data_flow::DataFlowGraph>,
    transitive_coverage: Option<&TransitiveCoverage>,
) -> Option<DebtAnalysisContext> {
    // Calculate unified score for this specific debt type
    let has_coverage_data = coverage.is_some();
    let unified_score = if let Some(df) = data_flow {
        // Use data flow scoring when available (spec 218)
        let config = get_data_flow_scoring_config();
        calculate_unified_priority_with_data_flow(
            func,
            call_graph,
            df,
            coverage,
            None,
            Some(debt_aggregator),
            &config,
        )
    } else {
        calculate_unified_priority_with_debt(
            func,
            call_graph,
            coverage,
            None,
            Some(debt_aggregator),
            has_coverage_data,
        )
    };

    // Determine function role
    let function_role = classify_function_role(func, func_id, call_graph);

    // Clone transitive coverage for the recommendation function
    let coverage_for_rec = transitive_coverage.cloned();

    // Generate recommendation (spec 201: may return None for clean dispatchers)
    let recommendation = generate_recommendation_with_coverage_and_data_flow(
        func,
        &debt_type,
        function_role,
        &unified_score,
        &coverage_for_rec,
        data_flow,
    )?;

    // Calculate expected impact
    let expected_impact = calculate_expected_impact(func, &debt_type, &unified_score);

    Some(DebtAnalysisContext {
        func_id: func_id.clone(),
        debt_type,
        unified_score,
        function_role,
        transitive_coverage: transitive_coverage.cloned(),
        recommendation,
        expected_impact,
    })
}

// Pure function: Extract dependency metrics
#[derive(Clone)]
struct DependencyMetrics {
    upstream_count: usize,
    downstream_count: usize,
    upstream_names: Vec<String>,
    downstream_names: Vec<String>,
}

fn extract_dependency_metrics(
    func: &FunctionMetrics,
    func_id: &FunctionId,
    call_graph: &CallGraph,
) -> DependencyMetrics {
    // Use pre-populated call graph data from FunctionMetrics if available
    let (upstream_names, downstream_names) =
        if func.upstream_callers.is_some() || func.downstream_callees.is_some() {
            (
                func.upstream_callers.clone().unwrap_or_default(),
                func.downstream_callees.clone().unwrap_or_default(),
            )
        } else {
            // Fallback: query call graph directly
            let upstream = call_graph.get_callers(func_id);
            let downstream = call_graph.get_callees(func_id);
            (
                upstream.iter().map(|f| f.name.clone()).collect(),
                downstream.iter().map(|f| f.name.clone()).collect(),
            )
        };

    DependencyMetrics {
        upstream_count: upstream_names.len(),
        downstream_count: downstream_names.len(),
        upstream_names,
        downstream_names,
    }
}

// Apply exponential scaling and risk boosting to a debt item (spec 171)
fn apply_score_scaling(mut item: UnifiedDebtItem) -> UnifiedDebtItem {
    use crate::priority::scoring::scaling::{calculate_final_score, ScalingConfig};

    let config = ScalingConfig::default();
    let base_score = item.unified_score.final_score.value();

    // Calculate final score with scaling
    let (final_score, exponent, boost) =
        calculate_final_score(base_score, &item.debt_type, &item, &config);

    // Update the unified score with scaling information
    item.unified_score.base_score = Some(base_score);
    item.unified_score.exponential_factor = Some(exponent);
    item.unified_score.risk_boost = Some(boost);
    item.unified_score.final_score = Score0To100::new(final_score);

    item
}

// Pure function: Build unified debt item from components (spec 195: uses cache, spec 196: shared detectors)
fn build_unified_debt_item(
    func: &FunctionMetrics,
    mut context: DebtAnalysisContext,
    deps: DependencyMetrics,
    file_line_counts: &FileLineCountCache,
    context_detector: &ContextDetector,
    recommendation_engine: &ContextRecommendationEngine,
) -> UnifiedDebtItem {
    // Apply context-aware dampening (spec 191)
    let (context_multiplier, context_type) = calculate_context_multiplier(&func.file);
    context.unified_score =
        apply_context_multiplier_to_score(context.unified_score, context_multiplier);

    // Detect function context (spec 122) - using shared detector (spec 196)
    let context_analysis = context_detector.detect_context(func, &func.file);

    // Generate contextual recommendation if confidence is high enough (spec 122)
    // Using shared engine (spec 196)
    let contextual_recommendation = if context_analysis.confidence > 0.6 {
        Some(recommendation_engine.generate_recommendation(
            func,
            context_analysis.context,
            context_analysis.confidence,
            context.unified_score.final_score.value(),
        ))
    } else {
        None
    };

    // Detect complexity pattern once during construction (spec 204)
    let detected_pattern =
        crate::priority::detected_pattern::DetectedPattern::detect(&func.language_specific);

    // Calculate entropy details once for efficiency (spec 214)
    let entropy_details = calculate_entropy_details(func);

    // Look up file line count from cache (spec 195: O(1) lookup instead of file read)
    let file_line_count = get_file_line_count(&func.file, file_line_counts);

    // Analyze responsibility category during construction (spec 254)
    let responsibility_category =
        crate::organization::god_object::analyze_function_responsibility(&func.name);

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
        file_line_count,         // Cached line count (spec 204)
        responsibility_category, // Behavioral responsibility (spec 254)
        error_swallowing_count: func.error_swallowing_count,
        error_swallowing_patterns: func.error_swallowing_patterns.clone(),
    }
}

// Main function using functional composition (spec 201, spec 228: multi-debt, spec 195: cache, spec 196: parallel)
/// Returns `Vec<UnifiedDebtItem>` - one per debt type found (spec 228)
///
/// # Parallelism (spec 196)
///
/// This function accepts shared `context_detector` and `recommendation_engine` references
/// to enable parallel processing. When called from `process_metrics_to_debt_items`,
/// these are created once and shared across all threads via immutable references.
///
/// # Thread Safety
///
/// All shared references are to `Sync` types:
/// - `ContextDetector`: Compiled regexes (read-only)
/// - `ContextRecommendationEngine`: Static recommendations (read-only)
/// - `FileLineCountCache`: HashMap (read-only)
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
    file_line_counts: &FileLineCountCache,
    context_detector: &ContextDetector,
    recommendation_engine: &ContextRecommendationEngine,
) -> Vec<UnifiedDebtItem> {
    // Step 1: Create function ID (pure)
    let func_id = create_function_id(func);

    // Step 2: Calculate coverage data once (reused for all debt types)
    let transitive_coverage = calculate_coverage_data(&func_id, func, call_graph, coverage);

    // Step 3: Get all debt types for this function (spec 228)
    let debt_types = classify_debt_type_with_exclusions(
        func,
        call_graph,
        &func_id,
        framework_exclusions,
        function_pointer_used_functions,
        transitive_coverage.as_ref(),
    );

    // Step 4: Extract dependencies once (shared across all debt items)
    let deps = extract_dependency_metrics(func, &func_id, call_graph);

    // Step 5: Create one UnifiedDebtItem per debt type (functional transformation)
    debt_types
        .into_iter()
        .filter_map(|debt_type| {
            // Analyze this specific debt type
            let context = analyze_single_debt_type(
                func,
                &func_id,
                debt_type,
                call_graph,
                coverage,
                debt_aggregator,
                data_flow,
                transitive_coverage.as_ref(),
            )?;

            // Build debt item (spec 195: uses cached file line counts, spec 196: shared detectors)
            let mut item = build_unified_debt_item(
                func,
                context,
                deps.clone(),
                file_line_counts,
                context_detector,
                recommendation_engine,
            );

            // Analyze contextual risk if risk analyzer is provided (spec 202)
            if let Some(analyzer) = risk_analyzer {
                let complexity_metrics = crate::core::ComplexityMetrics::from_function(func);
                let func_coverage = coverage.and_then(|cov| {
                    cov.get_function_coverage_with_line(&func.file, &func.name, func.line)
                });

                let (_, contextual_risk) = analyzer.analyze_function_with_context(
                    func.file.clone(),
                    func.name.clone(),
                    (func.line, func.line + func.length),
                    &complexity_metrics,
                    func_coverage,
                    func.is_test,
                    project_path.to_path_buf(),
                );

                // Apply contextual risk to score (spec 255)
                if let Some(ref ctx_risk) = contextual_risk {
                    item.unified_score =
                        apply_contextual_risk_to_score(item.unified_score, ctx_risk);
                }

                item.contextual_risk = contextual_risk;
            }

            // Apply exponential scaling and risk boosting (spec 171)
            Some(apply_score_scaling(item))
        })
        .collect()
}

pub fn create_unified_debt_item_with_exclusions(
    func: &FunctionMetrics,
    call_graph: &CallGraph,
    coverage: Option<&LcovData>,
    framework_exclusions: &HashSet<FunctionId>,
    function_pointer_used_functions: Option<&HashSet<FunctionId>>,
) -> Vec<UnifiedDebtItem> {
    // Create empty cache for backward compatibility (will use fallback reads)
    let empty_cache = FileLineCountCache::new();
    create_unified_debt_item_with_exclusions_and_data_flow(
        func,
        call_graph,
        coverage,
        framework_exclusions,
        function_pointer_used_functions,
        None,
        &empty_cache,
    )
}

pub fn create_unified_debt_item_with_exclusions_and_data_flow(
    func: &FunctionMetrics,
    call_graph: &CallGraph,
    coverage: Option<&LcovData>,
    framework_exclusions: &HashSet<FunctionId>,
    function_pointer_used_functions: Option<&HashSet<FunctionId>>,
    data_flow: Option<&crate::data_flow::DataFlowGraph>,
    file_line_counts: &FileLineCountCache,
) -> Vec<UnifiedDebtItem> {
    let func_id = FunctionId::new(func.file.clone(), func.name.clone(), func.line);

    // Calculate transitive coverage if coverage file is provided (Spec 203)
    // Use exact AST boundaries for more accurate coverage matching
    // ALWAYS return Some when coverage is provided, never None (eliminates Cov:N/A)
    let transitive_coverage = coverage.map(|lcov| {
        let end_line = func.line + func.length.saturating_sub(1);
        // get_function_coverage_with_bounds now returns Some(0.0) when not found
        // So we always get a value, even if it's 0%
        let _direct_coverage =
            lcov.get_function_coverage_with_bounds(&func.file, &func.name, func.line, end_line);
        calculate_transitive_coverage(&func_id, call_graph, lcov)
    });

    // Use the enhanced debt type classification with framework exclusions (spec 228)
    let debt_types = classify_debt_type_with_exclusions(
        func,
        call_graph,
        &func_id,
        framework_exclusions,
        function_pointer_used_functions,
        transitive_coverage.as_ref(),
    );

    // Pre-calculate shared data (extracted once, reused for all debt items)
    let mut unified_score = calculate_unified_priority(
        func, call_graph, coverage, None, // Organization factor no longer used
    );

    // Apply context-aware dampening (spec 191)
    let (context_multiplier, context_type) = calculate_context_multiplier(&func.file);
    unified_score = apply_context_multiplier_to_score(unified_score, context_multiplier);

    // Pre-extract dependencies (shared across all debt items)
    let (upstream_caller_names, downstream_callee_names) =
        if func.upstream_callers.is_some() || func.downstream_callees.is_some() {
            (
                func.upstream_callers.clone().unwrap_or_default(),
                func.downstream_callees.clone().unwrap_or_default(),
            )
        } else {
            // Fallback: query call graph directly
            let upstream = call_graph.get_callers(&func_id);
            let downstream = call_graph.get_callees(&func_id);
            (
                upstream.iter().map(|f| f.name.clone()).collect(),
                downstream.iter().map(|f| f.name.clone()).collect(),
            )
        };

    // Pre-calculate shared context data
    let context_detector = crate::analysis::ContextDetector::new();
    let context_analysis = context_detector.detect_context(func, &func.file);

    let function_role = classify_function_role(func, &func_id, call_graph);

    let contextual_recommendation = if context_analysis.confidence > 0.6 {
        let engine = crate::priority::scoring::ContextRecommendationEngine::new();
        Some(engine.generate_recommendation(
            func,
            context_analysis.context,
            context_analysis.confidence,
            unified_score.final_score.value(),
        ))
    } else {
        None
    };

    let detected_pattern =
        crate::priority::detected_pattern::DetectedPattern::detect(&func.language_specific);
    let entropy_details = calculate_entropy_details(func);
    // Look up file line count from cache (spec 195: O(1) lookup instead of file read)
    let file_line_count = get_file_line_count(&func.file, file_line_counts);

    // Create one UnifiedDebtItem per debt type (spec 228)
    debt_types
        .into_iter()
        .filter_map(|debt_type| {
            // Generate debt-type-specific recommendation
            let recommendation = generate_recommendation_with_coverage_and_data_flow(
                func,
                &debt_type,
                function_role,
                &unified_score,
                &transitive_coverage,
                data_flow,
            )?;

            // Calculate debt-type-specific impact
            let expected_impact = calculate_expected_impact(func, &debt_type, &unified_score);

            Some(UnifiedDebtItem {
                location: Location {
                    file: func.file.clone(),
                    function: func.name.clone(),
                    line: func.line,
                },
                debt_type,
                unified_score: unified_score.clone(),
                function_role,
                recommendation,
                expected_impact,
                transitive_coverage: transitive_coverage.clone(),
                upstream_dependencies: upstream_caller_names.len(),
                downstream_dependencies: downstream_callee_names.len(),
                upstream_callers: upstream_caller_names.clone(),
                downstream_callees: downstream_callee_names.clone(),
                nesting_depth: func.nesting,
                function_length: func.length,
                cyclomatic_complexity: func.cyclomatic,
                cognitive_complexity: func.cognitive,
                entropy_details: entropy_details.clone(),
                entropy_adjusted_cognitive: entropy_details.as_ref().map(|e| e.adjusted_cognitive),
                entropy_dampening_factor: entropy_details.as_ref().map(|e| e.dampening_factor),
                is_pure: func.is_pure,
                purity_confidence: func.purity_confidence,
                purity_level: None,
                god_object_indicators: None,
                tier: None,
                function_context: Some(context_analysis.context),
                context_confidence: Some(context_analysis.confidence),
                contextual_recommendation: contextual_recommendation.clone(),
                pattern_analysis: None,
                file_context: None,
                context_multiplier: Some(context_multiplier),
                context_type: Some(context_type),
                language_specific: func.language_specific.clone(),
                detected_pattern: detected_pattern.clone(),
                contextual_risk: None,
                file_line_count,
                responsibility_category:
                    crate::organization::god_object::analyze_function_responsibility(&func.name),
                error_swallowing_count: func.error_swallowing_count,
                error_swallowing_patterns: func.error_swallowing_patterns.clone(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::FileType;
    use crate::priority::score_types::Score0To100;
    use std::path::PathBuf;

    #[test]
    fn test_calculate_context_multiplier_for_example() {
        // Context dampening is now opt-in (default disabled)
        // File type is still detected, but multiplier defaults to 1.0
        let path = PathBuf::from("examples/demo.rs");
        let (multiplier, file_type) = calculate_context_multiplier(&path);

        assert_eq!(file_type, FileType::Example);
        assert_eq!(multiplier, 1.0); // No dampening by default (opt-in)
    }

    #[test]
    fn test_calculate_context_multiplier_for_test() {
        // Context dampening is now opt-in (default disabled)
        let path = PathBuf::from("tests/integration_test.rs");
        let (multiplier, file_type) = calculate_context_multiplier(&path);

        assert_eq!(file_type, FileType::Test);
        assert_eq!(multiplier, 1.0); // No dampening by default (opt-in)
    }

    #[test]
    fn test_calculate_context_multiplier_for_benchmark() {
        // Context dampening is now opt-in (default disabled)
        let path = PathBuf::from("benches/perf.rs");
        let (multiplier, file_type) = calculate_context_multiplier(&path);

        assert_eq!(file_type, FileType::Benchmark);
        assert_eq!(multiplier, 1.0); // No dampening by default (opt-in)
    }

    #[test]
    fn test_calculate_context_multiplier_for_build_script() {
        // Context dampening is now opt-in (default disabled)
        let path = PathBuf::from("build.rs");
        let (multiplier, file_type) = calculate_context_multiplier(&path);

        assert_eq!(file_type, FileType::BuildScript);
        assert_eq!(multiplier, 1.0); // No dampening by default (opt-in)
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
            final_score: Score0To100::new(24.0),
            base_score: Some(20.0),
            exponential_factor: None,
            risk_boost: None,
            pre_adjustment_score: Some(22.0),
            adjustment_applied: None,
            purity_factor: None,
            refactorability_factor: None,
            pattern_factor: None,
        };

        let adjusted = apply_context_multiplier_to_score(original_score, 0.1);

        // All scores should be multiplied by 0.1 (use approximate comparison for floats)
        assert!((adjusted.final_score.value() - 2.4).abs() < 0.0001);
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
            final_score: Score0To100::new(15.0),
            base_score: None,
            exponential_factor: None,
            risk_boost: None,
            pre_adjustment_score: None,
            adjustment_applied: None,
            purity_factor: None,
            refactorability_factor: None,
            pattern_factor: None,
        };

        // Test with all file types
        for multiplier in &[0.1, 0.2, 0.3, 1.0] {
            let adjusted = apply_context_multiplier_to_score(original_score.clone(), *multiplier);
            assert!(adjusted.final_score <= original_score.final_score);
        }
    }
}
