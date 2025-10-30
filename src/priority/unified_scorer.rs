use crate::core::FunctionMetrics;
use crate::organization::GodObjectAnalysis;
use crate::priority::{
    call_graph::{CallGraph, FunctionId},
    coverage_propagation::TransitiveCoverage,
    debt_aggregator::{DebtAggregator, FunctionId as AggregatorFunctionId},
    scoring::calculation::{
        calculate_base_score_no_coverage, calculate_base_score_with_coverage_multiplier,
        calculate_complexity_factor, calculate_coverage_factor, calculate_coverage_multiplier,
        calculate_dependency_factor, normalize_final_score,
    },
    scoring::debt_item::{determine_visibility, is_dead_code},
    semantic_classifier::{classify_function_role, FunctionRole},
    ActionableRecommendation, DebtType, FunctionAnalysis, ImpactMetrics,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pre_adjustment_score: Option<f64>, // Score before orchestration adjustment (spec 110)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub adjustment_applied:
        Option<crate::priority::scoring::orchestration_adjustment::ScoreAdjustment>, // Orchestration adjustment details (spec 110)
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
    pub god_object_indicators: Option<GodObjectAnalysis>, // God object detection results
    #[serde(skip)]
    pub tier: Option<crate::priority::RecommendationTier>, // Recommendation tier for prioritization
    pub function_context: Option<crate::analysis::FunctionContext>, // Detected context (spec 122)
    pub context_confidence: Option<f64>,         // Confidence in context detection (spec 122)
    pub contextual_recommendation: Option<crate::priority::scoring::ContextualRecommendation>, // Context-aware recommendation (spec 122)
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
    let has_coverage_data = coverage.is_some();
    calculate_unified_priority_with_debt(
        func,
        call_graph,
        coverage,
        organization_issues,
        None,
        has_coverage_data,
    )
}

pub fn calculate_unified_score_with_patterns(
    func: &FunctionMetrics,
    god_object: Option<&GodObjectAnalysis>,
    coverage: Option<&LcovData>,
    call_graph: &CallGraph,
) -> UnifiedScore {
    let has_coverage_data = coverage.is_some();
    let base_score = calculate_unified_priority_with_debt(
        func,
        call_graph,
        coverage,
        None,
        None,
        has_coverage_data,
    );

    // Apply god object multiplier
    let god_object_multiplier = if let Some(go) = god_object {
        if go.is_god_object {
            // Massive boost for functions in god objects
            3.0 + (go.god_object_score / 50.0)
        } else {
            1.0
        }
    } else {
        1.0
    };

    UnifiedScore {
        complexity_factor: base_score.complexity_factor * god_object_multiplier,
        coverage_factor: base_score.coverage_factor,
        dependency_factor: base_score.dependency_factor,
        role_multiplier: base_score.role_multiplier,
        final_score: base_score.final_score * god_object_multiplier,
        pre_adjustment_score: base_score.pre_adjustment_score,
        adjustment_applied: base_score.adjustment_applied,
    }
}

pub fn calculate_unified_priority_with_debt(
    func: &FunctionMetrics,
    call_graph: &CallGraph,
    coverage: Option<&LcovData>,
    _organization_issues: Option<f64>, // Kept for compatibility but no longer used
    debt_aggregator: Option<&DebtAggregator>,
    has_coverage_data: bool,
) -> UnifiedScore {
    let func_id = FunctionId::new(func.file.clone(), func.name.clone(), func.line);

    // Check if this function is actually technical debt
    // Simple I/O wrappers, entry points, and trivial pure functions with low complexity
    // are not technical debt UNLESS they're untested and non-trivial
    let role = classify_function_role(func, &func_id, call_graph);

    // Check if function is trivial based on complexity and role
    let is_trivial = is_trivial_function(func, role);

    // Check actual test coverage
    let coverage_pct = get_function_coverage(func, coverage);
    let has_coverage = coverage_pct > 0.0;

    // If it's trivial AND tested, it's definitely not technical debt
    if should_skip_as_non_debt(is_trivial, has_coverage) {
        return UnifiedScore {
            complexity_factor: 0.0,
            coverage_factor: 0.0,
            dependency_factor: 0.0,
            role_multiplier: 1.0,
            final_score: 0.0,
            pre_adjustment_score: None,
            adjustment_applied: None,
        };
    }

    // Detect if this is an orchestrator candidate for complexity weighting
    // Orchestrators typically have low cognitive complexity relative to cyclomatic
    let is_orchestrator_candidate = role == FunctionRole::Orchestrator;

    // Calculate purity adjustment and apply to complexity metrics
    let purity_bonus = calculate_purity_adjustment(func);
    let (purity_adjusted_cyclomatic, purity_adjusted_cognitive) =
        apply_purity_adjustment(func.cyclomatic, func.cognitive, purity_bonus);

    let raw_complexity = normalize_complexity(
        purity_adjusted_cyclomatic,
        purity_adjusted_cognitive,
        is_orchestrator_candidate,
    );

    // Calculate complexity and dependency factors
    let complexity_factor = calculate_complexity_factor(raw_complexity);
    let upstream_count = call_graph.get_callers(&func_id).len();
    let dependency_factor = calculate_dependency_factor(upstream_count);

    // Get role-based values
    let role_multiplier = calculate_role_multiplier(role, raw_complexity);
    let coverage_weight = get_role_coverage_weight(role);

    // Calculate base score
    let base_score = calculate_base_score(
        has_coverage_data,
        func.is_test,
        coverage_pct,
        coverage_weight,
        complexity_factor,
        dependency_factor,
    );

    // Store coverage_factor for display purposes (kept for backward compatibility)
    let _coverage_factor = if has_coverage_data {
        if func.is_test {
            0.1
        } else {
            calculate_coverage_factor(coverage_pct)
        }
    } else {
        0.0
    };

    // Apply role adjustment with configurable clamping (spec 119)
    let role_config = crate::config::get_role_multiplier_config();
    let clamped_role_multiplier = if role_config.enable_clamping {
        role_multiplier.clamp(role_config.clamp_min, role_config.clamp_max)
    } else {
        role_multiplier
    };
    let role_adjusted_score = base_score * clamped_role_multiplier;

    // Add debt-based adjustments
    let debt_adjustment = calculate_debt_adjustment(func, debt_aggregator);
    let debt_adjusted_score = role_adjusted_score + debt_adjustment;

    // Normalize to 0-10 scale
    let normalized_score = normalize_final_score(debt_adjusted_score);

    // Apply orchestration score adjustment (spec 110) if this is an orchestrator
    let (final_normalized_score, pre_adjustment, adjustment) = apply_orchestration_adjustment(
        is_orchestrator_candidate,
        normalized_score,
        &func_id,
        func,
        call_graph,
        &role,
    );

    UnifiedScore {
        complexity_factor,
        coverage_factor: (1.0 - coverage_pct) * 10.0, // Convert gap to 0-10 for display
        dependency_factor,
        role_multiplier,
        final_score: final_normalized_score,
        pre_adjustment_score: pre_adjustment,
        adjustment_applied: adjustment,
    }
}

// Helper functions for calculate_unified_priority_with_debt

/// Determine if a function is trivial based on complexity and role.
///
/// Trivial functions are simple enough that they're not considered technical debt.
fn is_trivial_function(func: &FunctionMetrics, role: FunctionRole) -> bool {
    (func.cyclomatic <= 3 && func.cognitive <= 5)
        && (role == FunctionRole::IOWrapper
            || role == FunctionRole::EntryPoint
            || role == FunctionRole::PatternMatch
            || role == FunctionRole::Debug
            || (role == FunctionRole::PureLogic && func.length <= 10))
}

/// Get the coverage percentage for a function.
///
/// Returns 1.0 for test functions, or the actual coverage from lcov data.
fn get_function_coverage(func: &FunctionMetrics, coverage: Option<&LcovData>) -> f64 {
    if func.is_test {
        1.0 // Test functions have 100% coverage by definition
    } else if let Some(cov) = coverage {
        cov.get_function_coverage(&func.file, &func.name)
            .unwrap_or(0.0)
    } else {
        0.0 // No coverage data - assume worst case
    }
}

/// Determine if a function should be skipped as non-debt.
///
/// Functions that are trivial AND tested are not technical debt.
fn should_skip_as_non_debt(is_trivial: bool, has_coverage: bool) -> bool {
    is_trivial && has_coverage
}

/// Calculate the purity adjustment multiplier.
///
/// Pure functions get a bonus (lower multiplier) because they're inherently less risky and easier to test.
fn calculate_purity_adjustment(func: &FunctionMetrics) -> f64 {
    if func.is_pure == Some(true) {
        // High confidence pure functions get bigger bonus
        if func.purity_confidence.unwrap_or(0.0) > 0.8 {
            0.7 // 30% reduction in complexity perception
        } else {
            0.85 // 15% reduction
        }
    } else {
        1.0 // No reduction for impure functions
    }
}

/// Apply purity adjustment to complexity metrics.
///
/// Returns adjusted cyclomatic and cognitive complexity values.
fn apply_purity_adjustment(cyclomatic: u32, cognitive: u32, adjustment: f64) -> (u32, u32) {
    (
        (cyclomatic as f64 * adjustment) as u32,
        (cognitive as f64 * adjustment) as u32,
    )
}

/// Calculate the role multiplier based on function role and complexity.
///
/// Different roles have different impacts on technical debt priority.
fn calculate_role_multiplier(role: FunctionRole, raw_complexity: f64) -> f64 {
    match role {
        FunctionRole::EntryPoint => 1.5,
        FunctionRole::PureLogic if raw_complexity > 5.0 => 1.3, // Complex core logic
        FunctionRole::PureLogic => 1.0,
        FunctionRole::Orchestrator => 0.8,
        FunctionRole::IOWrapper => 0.5,
        FunctionRole::PatternMatch => 0.6,
        FunctionRole::Debug => 0.3,
        _ => 1.0,
    }
}

/// Get the coverage weight multiplier for a function role.
///
/// Spec 110: Different roles have different coverage expectations.
fn get_role_coverage_weight(role: FunctionRole) -> f64 {
    let role_coverage_weights = crate::config::get_role_coverage_weights();
    match role {
        FunctionRole::EntryPoint => role_coverage_weights.entry_point,
        FunctionRole::Orchestrator => role_coverage_weights.orchestrator,
        FunctionRole::PureLogic => role_coverage_weights.pure_logic,
        FunctionRole::IOWrapper => role_coverage_weights.io_wrapper,
        FunctionRole::PatternMatch => role_coverage_weights.pattern_match,
        FunctionRole::Debug => role_coverage_weights.pattern_match, // Same as pattern_match
        _ => role_coverage_weights.unknown,
    }
}

/// Calculate the base score using either coverage multiplier or no-coverage approach.
///
/// This handles the two different scoring paths depending on coverage data availability.
fn calculate_base_score(
    has_coverage_data: bool,
    is_test: bool,
    coverage_pct: f64,
    coverage_weight: f64,
    complexity_factor: f64,
    dependency_factor: f64,
) -> f64 {
    if has_coverage_data {
        // With coverage: use multiplier approach (coverage dampens complexity+deps score)
        let coverage_multiplier = if is_test {
            0.0 // Test functions get maximum dampening (near-zero score)
        } else {
            // Apply role-based coverage weight adjustment (spec 110)
            let adjusted_coverage_pct = 1.0 - ((1.0 - coverage_pct) * coverage_weight);
            calculate_coverage_multiplier(adjusted_coverage_pct)
        };
        calculate_base_score_with_coverage_multiplier(
            coverage_multiplier,
            complexity_factor,
            dependency_factor,
        )
    } else {
        // Without coverage: adjusted weights (50% complexity, 25% deps, 25% debt)
        calculate_base_score_no_coverage(complexity_factor, dependency_factor)
    }
}

/// Calculate debt-based adjustment to the score.
///
/// Adds small additive adjustments for various debt types.
fn calculate_debt_adjustment(
    func: &FunctionMetrics,
    debt_aggregator: Option<&DebtAggregator>,
) -> f64 {
    if let Some(aggregator) = debt_aggregator {
        let agg_func_id =
            AggregatorFunctionId::new(func.file.clone(), func.name.clone(), func.line);
        let debt_scores = aggregator.calculate_debt_scores(&agg_func_id);

        // Add small additive adjustments for other debt types
        (debt_scores.testing / 50.0)
            + (debt_scores.resource / 50.0)
            + (debt_scores.duplication / 50.0)
    } else {
        0.0
    }
}

/// Apply orchestration adjustment if enabled.
///
/// Returns (final_score, pre_adjustment_score, adjustment_details).
fn apply_orchestration_adjustment(
    is_orchestrator: bool,
    normalized_score: f64,
    func_id: &FunctionId,
    func: &FunctionMetrics,
    call_graph: &CallGraph,
    role: &FunctionRole,
) -> (
    f64,
    Option<f64>,
    Option<crate::priority::scoring::orchestration_adjustment::ScoreAdjustment>,
) {
    if !is_orchestrator {
        return (normalized_score, None, None);
    }

    let config = crate::config::get_orchestration_adjustment_config();
    if !config.enabled {
        return (normalized_score, None, None);
    }

    // Extract composition metrics from call graph
    let composition_metrics =
        crate::priority::scoring::orchestration_adjustment::extract_composition_metrics(
            func_id, func, call_graph,
        );

    // Apply the adjustment
    let adjustment = crate::priority::scoring::orchestration_adjustment::adjust_score(
        &config,
        normalized_score,
        role,
        &composition_metrics,
    );

    // Log adjustment details for observability (spec 110)
    log::debug!(
        "Orchestration adjustment applied to {}:{} - Original: {:.2}, Adjusted: {:.2}, Reduction: {:.1}%, Reason: {}",
        func.file.display(),
        func.name,
        adjustment.original_score,
        adjustment.adjusted_score,
        adjustment.reduction_percent,
        adjustment.adjustment_reason
    );

    (
        adjustment.adjusted_score,
        Some(normalized_score),
        Some(adjustment),
    )
}

/// Normalize complexity to 0-10 scale using weighted complexity (spec 121).
///
/// Uses configurable weights for cyclomatic and cognitive complexity.
/// Default: 30% cyclomatic, 70% cognitive (research shows cognitive correlates better with bugs).
/// For orchestrators, cognitive weight may be increased further.
fn normalize_complexity(cyclomatic: u32, cognitive: u32, is_orchestrator: bool) -> f64 {
    use crate::complexity::{ComplexityNormalization, ComplexityWeights, WeightedComplexity};

    // Get weights from configuration (spec 121)
    let config = crate::config::get_config();
    let weights = if let Some(weights_config) = config.complexity_weights.as_ref() {
        ComplexityWeights {
            cyclomatic: weights_config.cyclomatic,
            cognitive: weights_config.cognitive,
        }
    } else {
        // For orchestrators, increase cognitive weight further
        if is_orchestrator {
            ComplexityWeights {
                cyclomatic: 0.25,
                cognitive: 0.75,
            }
        } else {
            ComplexityWeights::default()
        }
    };

    // Get normalization parameters from configuration
    let normalization = if let Some(weights_config) = config.complexity_weights.as_ref() {
        ComplexityNormalization {
            max_cyclomatic: weights_config.max_cyclomatic,
            max_cognitive: weights_config.max_cognitive,
        }
    } else {
        ComplexityNormalization::default()
    };

    // Calculate weighted complexity score (0-100 scale)
    let weighted = WeightedComplexity::calculate(cyclomatic, cognitive, weights, &normalization);

    // Convert from 0-100 scale to 0-10 scale for backward compatibility
    weighted.weighted_score / 10.0
}

// Pure functions for scoring calculation (spec 68)

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

#[cfg(test)]
mod tests;
