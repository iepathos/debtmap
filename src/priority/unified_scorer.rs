use crate::core::FunctionMetrics;
use crate::priority::{
    call_graph::{CallGraph, FunctionId},
    coverage_propagation::TransitiveCoverage,
    debt_aggregator::{DebtAggregator, FunctionId as AggregatorFunctionId},
    scoring::calculation::{
        apply_interaction_bonus, calculate_base_score, calculate_complexity_factor,
        calculate_coverage_factor, calculate_dependency_factor, normalize_final_score,
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
            .unwrap_or(0.0)
            / 100.0 // Convert to 0-1 range
    } else {
        0.0 // No coverage data - assume worst case
    };

    // Use pure functions for calculation (easier to test and debug)
    let coverage_factor = calculate_coverage_factor(coverage_pct);
    let complexity_factor = calculate_complexity_factor(raw_complexity);
    let upstream_count = call_graph.get_callers(&func_id).len();
    let dependency_factor = calculate_dependency_factor(upstream_count);

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

    // Calculate multiplicative base score using pure function
    let mut base_score =
        calculate_base_score(coverage_factor, complexity_factor, dependency_factor);

    // Apply interaction bonus using pure function
    base_score = apply_interaction_bonus(base_score, coverage_pct, raw_complexity);

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

    // Apply entropy dampening (spec 68: max 50% reduction)
    let final_score = if let Some(entropy_score) = func.entropy_score.as_ref() {
        // Apply dampening as a multiplier to the score
        // Use the new framework's dampening calculation
        let calculator = crate::complexity::entropy_core::UniversalEntropyCalculator::new(
            crate::complexity::entropy_core::EntropyConfig::default()
        );
        let dampening_factor = calculator.apply_dampening(entropy_score) / 100.0;
        debt_adjusted_score * dampening_factor.min(1.0).max(0.5) // Ensure max 50% reduction
    } else {
        debt_adjusted_score
    };

    // Normalize to 0-10 scale with better distribution
    let normalized_score = normalize_final_score(final_score);

    // Debug: Log ALL scores to find where 5.05 comes from
    if std::env::var("DEBUG_ALL_SCORES").is_ok() {
        eprintln!(
            "SCORE DEBUG: {} - raw={:.4}, norm={:.2}, cov={:.2}, cplx={:.2}, deps={}",
            func.name, final_score, normalized_score, coverage_pct, raw_complexity, upstream_count
        );
    }

    UnifiedScore {
        complexity_factor: raw_complexity,
        coverage_factor: (1.0 - coverage_pct) * 10.0, // Convert gap to 0-10 for display
        dependency_factor: upstream_count as f64,
        role_multiplier,
        final_score: normalized_score,
    }
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
