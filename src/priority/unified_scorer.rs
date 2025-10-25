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
            || role == FunctionRole::Debug
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
            pre_adjustment_score: None,
            adjustment_applied: None,
        };
    }

    // Detect if this is an orchestrator candidate for complexity weighting
    // Orchestrators typically have low cognitive complexity relative to cyclomatic
    let is_orchestrator_candidate = role == FunctionRole::Orchestrator;

    // Calculate complexity factor (normalized to 0-10)
    // Apply purity bonus first (pure functions are easier to understand and test)
    let purity_adjusted_cyclomatic = (func.cyclomatic as f64 * purity_bonus) as u32;
    let purity_adjusted_cognitive = (func.cognitive as f64 * purity_bonus) as u32;

    let raw_complexity = normalize_complexity(
        purity_adjusted_cyclomatic,
        purity_adjusted_cognitive,
        is_orchestrator_candidate,
    );

    // Get actual coverage percentage
    let coverage_pct = if func.is_test {
        1.0 // Test functions have 100% coverage by definition
    } else if let Some(cov) = coverage {
        // get_function_coverage already returns 0-1 fraction, no need to divide again
        cov.get_function_coverage(&func.file, &func.name)
            .unwrap_or(0.0)
    } else {
        0.0 // No coverage data - assume worst case
    };

    // Calculate complexity and dependency factors
    let complexity_factor = calculate_complexity_factor(raw_complexity);
    let upstream_count = call_graph.get_callers(&func_id).len();
    let dependency_factor = calculate_dependency_factor(upstream_count);

    // Get role multiplier - adjusted for better differentiation
    let role_multiplier: f64 = match role {
        FunctionRole::EntryPoint => 1.5,
        FunctionRole::PureLogic if raw_complexity > 5.0 => 1.3, // Complex core logic
        FunctionRole::PureLogic => 1.0,
        FunctionRole::Orchestrator => 0.8,
        FunctionRole::IOWrapper => 0.5,
        FunctionRole::PatternMatch => 0.6,
        FunctionRole::Debug => 0.3,
        _ => 1.0,
    };

    // Get role-based coverage weight multiplier (spec 110)
    let role_coverage_weights = crate::config::get_role_coverage_weights();
    let coverage_weight_multiplier = match role {
        FunctionRole::EntryPoint => role_coverage_weights.entry_point,
        FunctionRole::Orchestrator => role_coverage_weights.orchestrator,
        FunctionRole::PureLogic => role_coverage_weights.pure_logic,
        FunctionRole::IOWrapper => role_coverage_weights.io_wrapper,
        FunctionRole::PatternMatch => role_coverage_weights.pattern_match,
        FunctionRole::Debug => role_coverage_weights.pattern_match, // Same as pattern_match
        _ => role_coverage_weights.unknown,
    };

    // Calculate base score with coverage as multiplier (spec 122)
    let base_score = if has_coverage_data {
        // With coverage: use multiplier approach (coverage dampens complexity+deps score)
        let coverage_multiplier = if func.is_test {
            0.0 // Test functions get maximum dampening (near-zero score)
        } else {
            // Apply role-based coverage weight adjustment (spec 110)
            let adjusted_coverage_pct = 1.0 - ((1.0 - coverage_pct) * coverage_weight_multiplier);
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
    };

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
    // NOTE: This is separate from role-based coverage weight adjustment (applied earlier at line 236).
    // Coverage weight adjusts coverage expectations by role (entry points need less unit coverage).
    // This multiplier provides a final adjustment for role importance (default 0.3-1.8x range).
    // The two-stage approach ensures they don't interfere with each other.
    let role_config = crate::config::get_role_multiplier_config();
    let clamped_role_multiplier = if role_config.enable_clamping {
        role_multiplier.clamp(role_config.clamp_min, role_config.clamp_max)
    } else {
        role_multiplier
    };
    let role_adjusted_score = base_score * clamped_role_multiplier;

    // Add debt-based adjustments additively (not multiplicatively)
    let debt_adjustment = if let Some(aggregator) = debt_aggregator {
        let agg_func_id =
            AggregatorFunctionId::new(func.file.clone(), func.name.clone(), func.line);
        let debt_scores = aggregator.calculate_debt_scores(&agg_func_id);

        // Add small additive adjustments for other debt types
        (debt_scores.testing / 50.0)
            + (debt_scores.resource / 50.0)
            + (debt_scores.duplication / 50.0)
    } else {
        0.0
    };

    let debt_adjusted_score = role_adjusted_score + debt_adjustment;

    // Remove entropy dampening from scoring - it should not affect prioritization
    // Entropy is already factored into complexity metrics
    let final_score = debt_adjusted_score;

    // Normalize to 0-10 scale with better distribution
    let normalized_score = normalize_final_score(final_score);

    // Apply orchestration score adjustment (spec 110) if this is an orchestrator
    let (final_normalized_score, pre_adjustment, adjustment) = if is_orchestrator_candidate {
        let config = crate::config::get_orchestration_adjustment_config();
        if config.enabled {
            // Extract composition metrics from call graph
            let composition_metrics =
                crate::priority::scoring::orchestration_adjustment::extract_composition_metrics(
                    &func_id, func, call_graph,
                );

            // Apply the adjustment
            let adjustment = crate::priority::scoring::orchestration_adjustment::adjust_score(
                &config,
                normalized_score,
                &role,
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
        } else {
            (normalized_score, None, None)
        }
    } else {
        (normalized_score, None, None)
    };

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
/// Normalize complexity metrics with cognitive complexity weighting for orchestrators
///
/// For orchestrators (coordination functions), cognitive complexity receives higher weight (70%)
/// because sequential delegation has lower mental overhead than nested algorithmic logic,
/// even with similar cyclomatic complexity from error handling.
///
/// For non-orchestrators, both metrics are weighted equally (50%/50%).
fn normalize_complexity(cyclomatic: u32, cognitive: u32, is_orchestrator: bool) -> f64 {
    // Apply cognitive complexity weighting based on function role
    let combined = if is_orchestrator {
        // Weight cognitive more heavily for orchestrators (70%)
        // Sequential calls = low cognitive, high cyclomatic from error handling
        (cyclomatic as f64 * 0.3) + (cognitive as f64 * 0.7)
    } else {
        // Equal weight for pure logic (50%/50%)
        (cyclomatic + cognitive) as f64 / 2.0
    };

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
