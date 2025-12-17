use crate::core::{FunctionMetrics, PurityLevel};
use crate::data_flow::DataFlowGraph;
use crate::organization::GodObjectAnalysis;
use crate::priority::{
    call_graph::{CallGraph, FunctionId},
    coverage_propagation::TransitiveCoverage,
    debt_aggregator::{DebtAggregator, FunctionId as AggregatorFunctionId},
    score_types::Score0To100,
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

/// Purity spectrum classification for functions (spec 218).
/// Classifies functions on a spectrum from strictly pure to impure,
/// with score multipliers that reduce priority for purer functions.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum PuritySpectrum {
    /// Strictly pure - no mutations, no I/O, referentially transparent
    StrictlyPure,
    /// Locally pure - pure interface but uses local mutations internally
    LocallyPure,
    /// I/O isolated - I/O operations clearly separated from logic
    IOIsolated,
    /// I/O mixed - I/O mixed with business logic
    IOMixed,
    /// Impure - mutable state, side effects throughout
    Impure,
}

impl PuritySpectrum {
    /// Returns a score multiplier for this purity level.
    /// Lower multipliers reduce priority (less technical debt).
    /// Range: 0.0 (best - strictly pure) to 1.0 (worst - impure)
    pub fn score_multiplier(&self) -> f64 {
        match self {
            PuritySpectrum::StrictlyPure => 0.0,
            PuritySpectrum::LocallyPure => 0.3,
            PuritySpectrum::IOIsolated => 0.6,
            PuritySpectrum::IOMixed => 0.9,
            PuritySpectrum::Impure => 1.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedScore {
    pub complexity_factor: f64,   // 0-10, configurable weight (default 35%)
    pub coverage_factor: f64,     // 0-10, configurable weight (default 40%)
    pub dependency_factor: f64,   // 0-10, configurable weight (default 20%)
    pub role_multiplier: f64,     // 0.1-1.5x based on function role
    pub final_score: Score0To100, // Computed composite score (with scaling applied)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_score: Option<f64>, // Score before exponential scaling (spec 171)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exponential_factor: Option<f64>, // Exponent applied for scaling (1.0 = none, spec 171)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub risk_boost: Option<f64>, // Risk boost multiplier (1.0 = none, spec 171)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pre_adjustment_score: Option<f64>, // Score before orchestration adjustment (spec 110)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub adjustment_applied:
        Option<crate::priority::scoring::orchestration_adjustment::ScoreAdjustment>, // Orchestration adjustment details (spec 110)
    // Data flow factors (spec 218)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub purity_factor: Option<f64>, // Purity spectrum score (0.0-1.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refactorability_factor: Option<f64>, // Dead stores and escape analysis (1.0-1.5)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern_factor: Option<f64>, // Data flow vs business logic (0.7-1.0)
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
    /// **DEPRECATED (Spec 218)**: Use `entropy_analysis` instead.
    /// Kept for backward compatibility with existing code.
    pub entropy_details: Option<EntropyDetails>,
    /// Unified entropy analysis (Spec 218) - SINGLE SOURCE OF TRUTH.
    /// This is the canonical entropy type that flows through the entire pipeline.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entropy_analysis: Option<crate::complexity::EntropyAnalysis>,
    /// **DEPRECATED (Spec 218)**: Use `entropy_analysis.adjusted_complexity` instead.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entropy_adjusted_cognitive: Option<u32>,
    /// **DEPRECATED (Spec 218)**: Use `entropy_analysis.dampening_factor` instead.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entropy_dampening_factor: Option<f64>,
    pub is_pure: Option<bool>,          // Whether the function is pure
    pub purity_confidence: Option<f32>, // Confidence in purity detection
    #[serde(skip_serializing_if = "Option::is_none")]
    pub purity_level: Option<PurityLevel>, // Refined purity classification (spec 157)
    pub god_object_indicators: Option<GodObjectAnalysis>, // God object detection results
    #[serde(skip)]
    pub tier: Option<crate::priority::RecommendationTier>, // Recommendation tier for prioritization
    pub function_context: Option<crate::analysis::FunctionContext>, // Detected context (spec 122)
    pub context_confidence: Option<f64>, // Confidence in context detection (spec 122)
    pub contextual_recommendation: Option<crate::priority::scoring::ContextualRecommendation>, // Context-aware recommendation (spec 122)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern_analysis: Option<crate::output::PatternAnalysis>, // Pattern analysis for purity, frameworks, Rust patterns (spec 151)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_context: Option<crate::analysis::FileContext>, // File context for test file detection (spec 166)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_multiplier: Option<f64>, // Context-based dampening multiplier applied (spec 191)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_type: Option<crate::context::FileType>, // Detected file type for context dampening (spec 191)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language_specific: Option<crate::core::LanguageSpecificData>, // Language-specific pattern detection (spec 190)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detected_pattern: Option<crate::priority::detected_pattern::DetectedPattern>, // Detected complexity pattern (spec 204)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contextual_risk: Option<crate::risk::context::ContextualRisk>, // Git history and context provider risk data
    /// Cached line count for this item's file (spec 204).
    /// Populated during item creation to avoid re-reading files during calculate_total_impact.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_line_count: Option<usize>,
    /// Primary responsibility category for this function/module (spec 254).
    /// Derived from behavioral analysis of function name during analysis phase.
    /// Examples: "Data Access", "Validation", "Parsing", "Rendering"
    /// None if behavioral category cannot be inferred with high confidence (>= 0.7).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub responsibility_category: Option<String>,
    /// Count of error swallowing patterns detected in this function
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_swallowing_count: Option<u32>,
    /// Types of error swallowing patterns detected
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_swallowing_patterns: Option<Vec<String>>,
}

impl UnifiedDebtItem {
    /// Builder method to attach pattern analysis to this debt item (spec 151)
    pub fn with_pattern_analysis(
        mut self,
        pattern_analysis: crate::output::PatternAnalysis,
    ) -> Self {
        self.pattern_analysis = Some(pattern_analysis);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntropyDetails {
    pub entropy_score: f64,
    pub pattern_repetition: f64,
    pub original_complexity: u32,
    pub adjusted_complexity: u32,
    pub dampening_factor: f64,
    pub adjusted_cognitive: u32,
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
            3.0 + (go.god_object_score.value() / 50.0)
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
        final_score: Score0To100::new(base_score.final_score.value() * god_object_multiplier),
        base_score: base_score.base_score,
        exponential_factor: base_score.exponential_factor,
        risk_boost: base_score.risk_boost,
        pre_adjustment_score: base_score.pre_adjustment_score,
        adjustment_applied: base_score.adjustment_applied,
        purity_factor: base_score.purity_factor,
        refactorability_factor: base_score.refactorability_factor,
        pattern_factor: base_score.pattern_factor,
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

    // Delegate to the role-aware version (spec 205)
    calculate_unified_priority_with_role(
        func,
        &func_id,
        call_graph,
        coverage,
        debt_aggregator,
        has_coverage_data,
        role,
    )
}

/// Calculate unified priority score with a pre-computed function role (spec 205).
///
/// This function accepts a pre-computed FunctionRole to avoid redundant classification
/// when creating multiple debt items for the same function. The role is computed once
/// per function and reused across all debt types.
///
/// # Performance
///
/// For functions with multiple debt types (e.g., TestingGap AND ComplexityHotspot),
/// this eliminates redundant `classify_function_role()` calls, reducing scoring
/// overhead by ~30% on large codebases.
pub fn calculate_unified_priority_with_role(
    func: &FunctionMetrics,
    func_id: &FunctionId,
    call_graph: &CallGraph,
    coverage: Option<&LcovData>,
    debt_aggregator: Option<&DebtAggregator>,
    has_coverage_data: bool,
    role: FunctionRole,
) -> UnifiedScore {
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
            final_score: Score0To100::new(0.0),
            base_score: Some(0.0),
            exponential_factor: Some(1.0),
            risk_boost: Some(1.0),
            pre_adjustment_score: None,
            adjustment_applied: None,
            purity_factor: None,
            refactorability_factor: None,
            pattern_factor: None,
        };
    }

    // Detect if this is an orchestrator candidate for complexity weighting
    // Orchestrators typically have low cognitive complexity relative to cyclomatic
    let is_orchestrator_candidate = role == FunctionRole::Orchestrator;

    // Calculate entropy details if available (spec 214)
    let entropy_details = crate::priority::scoring::computation::calculate_entropy_details(func);

    // Calculate purity adjustment and apply to complexity metrics
    let purity_bonus = calculate_purity_adjustment(func);
    let (purity_adjusted_cyclomatic, purity_adjusted_cognitive) =
        apply_purity_adjustment(func.cyclomatic, func.cognitive, purity_bonus);

    let raw_complexity = normalize_complexity(
        purity_adjusted_cyclomatic,
        purity_adjusted_cognitive,
        entropy_details.as_ref(),
        is_orchestrator_candidate,
    );

    // Calculate complexity and dependency factors
    let complexity_factor = calculate_complexity_factor(raw_complexity);
    let upstream_count = call_graph.get_callers(func_id).len();
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

    // Apply structural quality adjustment based on nesting/cyclomatic ratio
    // High ratio = deeply nested relative to branches = bad structure = boost score
    // Low ratio = flat structure = good structure = reduce score
    let structural_multiplier =
        calculate_structural_quality_multiplier(func.nesting, func.cyclomatic);
    let structure_adjusted_score = role_adjusted_score * structural_multiplier;

    // Add debt-based adjustments
    let debt_adjustment = calculate_debt_adjustment(func, debt_aggregator);
    let debt_adjusted_score = structure_adjusted_score + debt_adjustment;

    // Normalize to 0-10 scale
    let normalized_score = normalize_final_score(debt_adjusted_score);

    // Apply orchestration score adjustment (spec 110) if this is an orchestrator
    let (final_normalized_score, pre_adjustment, adjustment) = apply_orchestration_adjustment(
        is_orchestrator_candidate,
        normalized_score,
        func_id,
        func,
        call_graph,
        &role,
    );

    UnifiedScore {
        complexity_factor,
        coverage_factor: (1.0 - coverage_pct) * 10.0, // Convert gap to 0-10 for display
        dependency_factor,
        role_multiplier,
        final_score: Score0To100::new(final_normalized_score),
        base_score: None,         // Set later in debt item construction (spec 171)
        exponential_factor: None, // Set later in debt item construction (spec 171)
        risk_boost: None,         // Set later in debt item construction (spec 171)
        pre_adjustment_score: pre_adjustment,
        adjustment_applied: adjustment,
        purity_factor: None, // Set by calculate_unified_priority_with_data_flow (spec 218)
        refactorability_factor: None, // Set by calculate_unified_priority_with_data_flow (spec 218)
        pattern_factor: None, // Set by calculate_unified_priority_with_data_flow (spec 218)
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

/// Calculates purity-based complexity adjustment.
///
/// Pure functions are easier to test and less risky, so they get a complexity bonus.
/// Now supports refined purity levels:
/// - StrictlyPure: 0.70-0.80 (best)
/// - LocallyPure: 0.75-0.85 (very good - uses local mutations)
/// - ReadOnly: 0.90 (good - reads but doesn't modify)
/// - Impure: 1.0 (no bonus)
fn calculate_purity_adjustment(func: &FunctionMetrics) -> f64 {
    // Try new purity_level field first
    if let Some(level) = func.purity_level {
        let confidence = func.purity_confidence.unwrap_or(0.0);

        return match level {
            PurityLevel::StrictlyPure => {
                if confidence > 0.8 {
                    0.70 // High confidence: 30% reduction
                } else {
                    0.80 // Medium confidence: 20% reduction
                }
            }
            PurityLevel::LocallyPure => {
                // NEW: Functionally pure with local mutations
                if confidence > 0.8 {
                    0.75 // High confidence: 25% reduction
                } else {
                    0.85 // Medium confidence: 15% reduction
                }
            }
            PurityLevel::ReadOnly => 0.90, // 10% reduction
            PurityLevel::Impure => 1.0,    // No reduction
        };
    }

    // Fallback to legacy is_pure field for backward compatibility
    if func.is_pure == Some(true) {
        // Old code path - treat as StrictlyPure
        if func.purity_confidence.unwrap_or(0.0) > 0.8 {
            0.70
        } else {
            0.85
        }
    } else {
        1.0 // Impure or unknown
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

/// Calculate structural quality multiplier based on nesting/cyclomatic ratio.
///
/// This captures how "deeply nested" code is relative to its branching complexity.
/// - High ratio (nesting/cyclo > 0.5) = deeply nested = bad structure = boost score (1.2-1.5x)
/// - Medium ratio (0.2-0.5) = moderate nesting = neutral (1.0x)
/// - Low ratio (< 0.2) = flat structure = good structure = reduce score (0.7-0.9x)
///
/// Examples:
/// - validate_and_transform_data: nesting=5, cyclo=8, ratio=0.625 → 1.3x (bad)
/// - process_user_data: nesting=1, cyclo=11, ratio=0.09 → 0.8x (good)
fn calculate_structural_quality_multiplier(nesting: u32, cyclomatic: u32) -> f64 {
    if cyclomatic == 0 {
        return 1.0;
    }

    let ratio = nesting as f64 / cyclomatic as f64;

    match ratio {
        r if r >= 0.6 => 1.5,  // Very deeply nested - major penalty
        r if r >= 0.5 => 1.3,  // Deeply nested - significant penalty
        r if r >= 0.4 => 1.15, // Moderately nested - minor penalty
        r if r >= 0.2 => 1.0,  // Normal structure - neutral
        r if r >= 0.1 => 0.85, // Flat structure - minor bonus
        _ => 0.7,              // Very flat structure - significant bonus
    }
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
///
/// Applies entropy-based dampening if entropy_details is provided and enabled in config (spec 214).
fn normalize_complexity(
    cyclomatic: u32,
    cognitive: u32,
    entropy_details: Option<&EntropyDetails>,
    is_orchestrator: bool,
) -> f64 {
    use crate::complexity::{ComplexityNormalization, ComplexityWeights, WeightedComplexity};
    #[allow(unused_imports)]
    use crate::priority::score_types::Score0To100;

    // Get configuration
    let config = crate::config::get_config();
    let entropy_config = crate::config::get_entropy_config();

    // Apply entropy dampening if available and enabled (spec 214)
    let (adjusted_cyclo, adjusted_cog) = if let Some(entropy) = entropy_details {
        if entropy_config.enabled {
            // Use entropy-adjusted complexity values
            (entropy.adjusted_complexity, entropy.adjusted_cognitive)
        } else {
            (cyclomatic, cognitive)
        }
    } else {
        // No entropy data available, use raw complexity
        (cyclomatic, cognitive)
    };

    // Get weights from configuration (spec 121)
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

    // Calculate weighted complexity score (0-100 scale) using entropy-adjusted values
    let weighted =
        WeightedComplexity::calculate(adjusted_cyclo, adjusted_cog, weights, &normalization);

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

// Data flow scoring functions (spec 218)

/// Calculate purity factor from data flow analysis (spec 218).
/// Returns a factor in range 0.0-1.0 where lower values reduce priority.
/// Based on PuritySpectrum classification derived from data flow graph.
fn calculate_purity_factor(func_id: &FunctionId, data_flow: &DataFlowGraph) -> f64 {
    // Get purity info from data flow analysis
    let purity_info = data_flow.get_purity_info(func_id);

    // Get mutation analysis
    let mutation_info = data_flow.get_mutation_info(func_id);

    // Get I/O operations
    let io_ops = data_flow.get_io_operations(func_id);

    // Classify on purity spectrum (spec 257: use binary signals)
    let spectrum = if let Some(purity) = purity_info {
        if purity.is_pure && purity.confidence > 0.8 {
            // Check if truly pure or just locally pure using binary signals
            if let Some(mutations) = mutation_info {
                if mutations.has_mutations {
                    // Has local mutations but doesn't escape
                    PuritySpectrum::LocallyPure
                } else {
                    // No mutations at all
                    PuritySpectrum::StrictlyPure
                }
            } else {
                PuritySpectrum::StrictlyPure
            }
        } else if purity.is_pure {
            // Lower confidence purity
            PuritySpectrum::LocallyPure
        } else {
            // Not pure - check I/O isolation
            classify_io_isolation(io_ops)
        }
    } else {
        // No purity info - assume impure
        PuritySpectrum::Impure
    };

    spectrum.score_multiplier()
}

/// Classify I/O isolation level based on I/O operations
fn classify_io_isolation(io_ops: Option<&Vec<crate::data_flow::IoOperation>>) -> PuritySpectrum {
    match io_ops {
        None => PuritySpectrum::Impure,
        Some(ops) if ops.is_empty() => PuritySpectrum::Impure,
        Some(ops) => {
            // If I/O operations are concentrated (few unique types), likely isolated
            let unique_types: HashSet<&String> = ops.iter().map(|op| &op.operation_type).collect();
            if unique_types.len() <= 2 && ops.len() <= 3 {
                PuritySpectrum::IOIsolated
            } else {
                PuritySpectrum::IOMixed
            }
        }
    }
}

/// Calculate refactorability factor (spec 218).
/// Returns a neutral factor of 1.0 (dead store analysis removed).
fn calculate_refactorability_factor(
    _func_id: &FunctionId,
    _data_flow: &DataFlowGraph,
    _config: &crate::config::DataFlowScoringConfig,
) -> f64 {
    // Dead store analysis has been removed as it produced too many false positives.
    // This function now returns a neutral factor.
    1.0
}

/// Calculate pattern factor to distinguish data flow from business logic (spec 218).
/// Returns a factor in range 0.7-1.0 where lower values reduce priority.
/// Pure data transformation pipelines get reduced priority (less debt).
fn calculate_pattern_factor(func_id: &FunctionId, data_flow: &DataFlowGraph) -> f64 {
    // Count data transformations
    let transform_count = count_data_transformations(func_id, data_flow);

    // Get variable dependencies
    let var_deps = data_flow.get_variable_dependencies(func_id);
    let dep_count = var_deps.map(|deps| deps.len()).unwrap_or(0);

    // Data flow functions have many transformations relative to dependencies
    if transform_count > 0 && dep_count > 0 {
        let transform_ratio = transform_count as f64 / dep_count as f64;

        if transform_ratio > 0.5 {
            // High transformation ratio - likely data flow pipeline
            0.7
        } else if transform_ratio > 0.3 {
            // Medium ratio - mixed
            0.85
        } else {
            // Low ratio - business logic
            1.0
        }
    } else {
        // No transformation data - assume business logic
        1.0
    }
}

/// Count data transformations involving this function
fn count_data_transformations(func_id: &FunctionId, data_flow: &DataFlowGraph) -> usize {
    // This is a simplified count - in reality we'd need to iterate through
    // all transformations in the graph to find those involving this function
    // For now, check if this function has any outgoing transformations
    let call_graph = data_flow.call_graph();
    let callees = call_graph.get_callees(func_id);

    // Count how many callees have data transformations
    callees
        .iter()
        .filter(|callee| data_flow.get_data_transformation(func_id, callee).is_some())
        .count()
}

/// Calculate unified priority with data flow analysis (spec 218).
/// This is the main entry point for data flow-aware scoring.
pub fn calculate_unified_priority_with_data_flow(
    func: &FunctionMetrics,
    call_graph: &CallGraph,
    data_flow: &DataFlowGraph,
    coverage: Option<&LcovData>,
    _organization_issues: Option<f64>, // Kept for API compatibility
    debt_aggregator: Option<&DebtAggregator>,
    config: &crate::config::DataFlowScoringConfig,
) -> UnifiedScore {
    let func_id = FunctionId::new(func.file.clone(), func.name.clone(), func.line);
    let role = classify_function_role(func, &func_id, call_graph);

    // Delegate to the role-aware version (spec 205)
    calculate_unified_priority_with_data_flow_and_role(
        func,
        &func_id,
        call_graph,
        data_flow,
        coverage,
        debt_aggregator,
        config,
        role,
    )
}

/// Calculate unified priority with data flow analysis and pre-computed role (spec 205, 218).
///
/// This is an optimized version that accepts a pre-computed function role to avoid
/// redundant classification when creating multiple debt items for the same function.
#[allow(clippy::too_many_arguments)]
pub fn calculate_unified_priority_with_data_flow_and_role(
    func: &FunctionMetrics,
    func_id: &FunctionId,
    call_graph: &CallGraph,
    data_flow: &DataFlowGraph,
    coverage: Option<&LcovData>,
    debt_aggregator: Option<&DebtAggregator>,
    config: &crate::config::DataFlowScoringConfig,
    role: FunctionRole,
) -> UnifiedScore {
    // Get base score from existing scoring with pre-computed role
    let has_coverage_data = coverage.is_some();
    let mut base_score = calculate_unified_priority_with_role(
        func,
        func_id,
        call_graph,
        coverage,
        debt_aggregator,
        has_coverage_data,
        role,
    );

    // If data flow scoring is disabled, return base score
    if !config.enabled {
        return base_score;
    }

    let purity_factor = calculate_purity_factor(func_id, data_flow);
    let refactorability_factor = calculate_refactorability_factor(func_id, data_flow, config);
    let pattern_factor = calculate_pattern_factor(func_id, data_flow);

    // Apply factors to final score with configurable weights
    let purity_adjustment = purity_factor * config.purity_weight;
    let refactorability_adjustment = refactorability_factor * config.refactorability_weight;
    let pattern_adjustment = pattern_factor * config.pattern_weight;

    // Combine adjustments (weighted average)
    let total_weight = config.purity_weight + config.refactorability_weight + config.pattern_weight;
    let combined_adjustment = if total_weight > 0.0 {
        (purity_adjustment + refactorability_adjustment + pattern_adjustment) / total_weight
    } else {
        1.0
    };

    // Apply adjustment to final score
    let adjusted_score = Score0To100::new(base_score.final_score.value() * combined_adjustment);

    // Update score with data flow factors
    base_score.final_score = adjusted_score;
    base_score.purity_factor = Some(purity_factor);
    base_score.refactorability_factor = Some(refactorability_factor);
    base_score.pattern_factor = Some(pattern_factor);

    base_score
}

#[cfg(test)]
mod tests;
