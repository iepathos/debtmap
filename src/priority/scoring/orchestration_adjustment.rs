//! Orchestration Score Adjustment Module (Spec 110)
//!
//! This module implements score adjustments for orchestrator functions identified
//! by spec 117's semantic classification. It reduces false positives by applying
//! graduated reductions based on composition quality metrics.
//!
//! # Design Philosophy
//!
//! This module follows functional programming principles:
//! - All functions are pure (no side effects)
//! - Immutable data structures
//! - Clear, composable transformations
//! - Type-safe newtypes prevent invalid values at compile time
//!
//! # Score Calculation Pipeline
//!
//! ```text
//! base_score → role_multiplier (spec 117) → entropy_dampening → orchestration_adjustment → final_score
//! ```
//!
//! This module implements the final step: orchestration adjustment.

use crate::core::FunctionMetrics;
use crate::priority::call_graph::{CallGraph, FunctionId};
use crate::priority::semantic_classifier::FunctionRole;
use anyhow::{ensure, Result};
use serde::{Deserialize, Serialize};

// ============================================================================
// Type-Safe Newtypes
// ============================================================================

/// Reduction percentage bounded between 0.0 and 1.0
///
/// This newtype wrapper ensures that reduction percentages are always valid
/// at compile time, preventing invalid values from being used in calculations.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct ReductionPercent(f64);

impl ReductionPercent {
    /// Create a new ReductionPercent, validating that the value is between 0.0 and 1.0
    pub fn new(value: f64) -> Result<Self> {
        ensure!(
            (0.0..=1.0).contains(&value),
            "Reduction percent must be between 0.0 and 1.0, got {}",
            value
        );
        Ok(Self(value))
    }

    /// Get the raw value (0.0-1.0)
    pub fn value(&self) -> f64 {
        self.0
    }

    /// Get the value as a percentage (0.0-100.0)
    pub fn as_percent(&self) -> f64 {
        self.0 * 100.0
    }
}

// ============================================================================
// Configuration
// ============================================================================

/// Configuration for orchestration score adjustments
///
/// This configuration allows tuning of the adjustment algorithm to match
/// project-specific needs while maintaining sensible defaults.
///
/// # Example
///
/// ```toml
/// [orchestration_adjustment]
/// enabled = true
/// base_orchestrator_reduction = 0.20      # 20% base reduction
/// max_quality_bonus = 0.10                # Up to 10% more
/// max_total_reduction = 0.30              # 30% cap
/// min_inherent_complexity_factor = 2.0    # Floor = callees × this
/// min_composition_quality = 0.5           # Minimum quality score
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestrationAdjustmentConfig {
    /// Whether orchestration adjustments are enabled
    pub enabled: bool,

    /// Base reduction for all orchestrators (default: 0.20 = 20%)
    pub base_orchestrator_reduction: f64,

    /// Additional reduction based on composition quality (default: 0.10 = 10% max)
    pub max_quality_bonus: f64,

    /// Maximum total reduction allowed (default: 0.30 = 30% cap)
    pub max_total_reduction: f64,

    /// Minimum inherent complexity factor (default: 2.0)
    /// Final score never goes below: callee_count × this factor
    pub min_inherent_complexity_factor: f64,

    /// Minimum composition quality multiplier (default: 0.5)
    /// Quality scores are clamped to [min, 1.0]
    pub min_composition_quality: f64,
}

impl Default for OrchestrationAdjustmentConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            base_orchestrator_reduction: 0.20,
            max_quality_bonus: 0.10,
            max_total_reduction: 0.31, // Set slightly higher to avoid floating point precision issues
            min_inherent_complexity_factor: 2.0,
            min_composition_quality: 0.5,
        }
    }
}

impl OrchestrationAdjustmentConfig {
    /// Validate configuration values
    ///
    /// Ensures that:
    /// - All reduction percentages are in [0.0, 1.0]
    /// - base + max_quality_bonus ≤ max_total_reduction
    /// - min_inherent_complexity_factor > 0
    /// - min_composition_quality ∈ [0.0, 1.0]
    pub fn validate(&self) -> Result<()> {
        ensure!(
            (0.0..=1.0).contains(&self.base_orchestrator_reduction),
            "base_orchestrator_reduction must be between 0.0 and 1.0, got {}",
            self.base_orchestrator_reduction
        );
        ensure!(
            (0.0..=1.0).contains(&self.max_quality_bonus),
            "max_quality_bonus must be between 0.0 and 1.0, got {}",
            self.max_quality_bonus
        );
        ensure!(
            (0.0..=1.0).contains(&self.max_total_reduction),
            "max_total_reduction must be between 0.0 and 1.0, got {}",
            self.max_total_reduction
        );
        ensure!(
            self.base_orchestrator_reduction + self.max_quality_bonus <= self.max_total_reduction,
            "base_orchestrator_reduction ({}) + max_quality_bonus ({}) must be <= max_total_reduction ({})",
            self.base_orchestrator_reduction,
            self.max_quality_bonus,
            self.max_total_reduction
        );
        ensure!(
            self.min_inherent_complexity_factor > 0.0,
            "min_inherent_complexity_factor must be positive, got {}",
            self.min_inherent_complexity_factor
        );
        ensure!(
            (0.0..=1.0).contains(&self.min_composition_quality),
            "min_composition_quality must be between 0.0 and 1.0, got {}",
            self.min_composition_quality
        );
        Ok(())
    }
}

// ============================================================================
// Adjustment Metadata
// ============================================================================

/// Metadata about score adjustment applied to a function
///
/// This structure captures all information about how a score was adjusted,
/// enabling auditability and transparency in the scoring system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoreAdjustment {
    /// Original score before adjustment
    pub original_score: f64,

    /// Final score after adjustment
    pub adjusted_score: f64,

    /// Percentage reduction applied (0.0-100.0)
    pub reduction_percent: f64,

    /// Human-readable explanation of the adjustment
    pub adjustment_reason: String,

    /// Composition quality score (0.0-1.0) that influenced the adjustment
    pub quality_score: f64,
}

impl ScoreAdjustment {
    /// Create a "no adjustment" record for functions that don't receive adjustments
    pub fn no_adjustment(score: f64) -> Self {
        Self {
            original_score: score,
            adjusted_score: score,
            reduction_percent: 0.0,
            adjustment_reason: "No adjustment applied".to_string(),
            quality_score: 0.0,
        }
    }
}

// ============================================================================
// Composition Metrics
// ============================================================================

/// Composition metrics extracted from call graph
///
/// These metrics describe how a function composes smaller functions:
/// - More callees indicates more delegation
/// - Higher delegation ratio means higher proportion of the function is coordination
/// - Lower local complexity suggests cleaner orchestration
#[derive(Debug, Clone)]
pub struct CompositionMetrics {
    /// Number of functions called by this function
    pub callee_count: usize,

    /// Ratio of function calls to total statements (callees / length)
    pub delegation_ratio: f64,

    /// Cyclomatic complexity of the function itself
    pub local_complexity: u32,
}

/// Extract composition metrics from call graph (pure function)
///
/// This function analyzes the call graph to determine how much a function
/// delegates to other functions versus implementing logic itself.
///
/// # Formula
///
/// ```text
/// delegation_ratio = callee_count / function_length
/// ```
///
/// A higher delegation ratio indicates better orchestration.
pub fn extract_composition_metrics(
    func_id: &FunctionId,
    func: &FunctionMetrics,
    call_graph: &CallGraph,
) -> CompositionMetrics {
    let callees = call_graph.get_callees(func_id);
    let callee_count = callees.len();

    // Delegation ratio: callees / function length
    let delegation_ratio = if func.length > 0 {
        callee_count as f64 / func.length as f64
    } else {
        0.0
    };

    // Local complexity is the cyclomatic complexity
    let local_complexity = func.cyclomatic;

    CompositionMetrics {
        callee_count,
        delegation_ratio,
        local_complexity,
    }
}

// ============================================================================
// Score Adjustment Algorithm (Pure Functions)
// ============================================================================

/// Apply orchestration-aware score adjustment (pure function)
///
/// This is the main entry point for score adjustment. It dispatches to
/// role-specific adjustment functions based on the function's role classification.
///
/// # Score Calculation Pipeline
///
/// 1. Base complexity score (cyclomatic, cognitive, etc.)
/// 2. Role multiplier (from spec 117)
/// 3. Entropy dampening (reduces noise in scattered complexity)
/// 4. **Orchestration adjustment** (reduces orchestrator false positives) ← This function
///
/// # Adjustment Rules
///
/// - **Orchestrator**: Receives graduated reduction based on composition quality
/// - **Other roles**: No adjustment (PureLogic, IOWrapper, EntryPoint, PatternMatch already have multipliers from spec 117)
///
/// # Example
///
/// ```text
/// Input:  base_score=100.0, role=Orchestrator, quality_score=0.9
/// Output: ScoreAdjustment { adjusted_score=72.0, reduction_percent=28.0, ... }
/// ```
pub fn adjust_score(
    config: &OrchestrationAdjustmentConfig,
    base_score: f64,
    role: &FunctionRole,
    metrics: &CompositionMetrics,
) -> ScoreAdjustment {
    // Early return if adjustments are disabled
    if !config.enabled {
        return ScoreAdjustment::no_adjustment(base_score);
    }

    // Only adjust orchestrators; other roles already have multipliers from spec 117
    match role {
        FunctionRole::Orchestrator => adjust_orchestrator_score(config, base_score, metrics),
        _ => ScoreAdjustment::no_adjustment(base_score),
    }
}

/// Adjust score for orchestrator functions (pure function)
///
/// Applies graduated reductions based on:
/// - Base reduction for being an orchestrator (20% by default)
/// - Composition quality bonus (up to 10% more)
/// - Minimum complexity floor (callee_count × factor)
///
/// # Algorithm
///
/// ```text
/// base_reduction = 0.20
/// quality_score = calculate_composition_quality(metrics)  // 0.0-1.0
/// quality_bonus = max_quality_bonus * quality_score       // 0.0-0.10
/// total_reduction = min(base_reduction + quality_bonus, max_total_reduction)
/// adjusted = max(base_score × (1 - total_reduction), callee_count × min_inherent_complexity_factor)
/// ```
///
/// # Edge Cases
///
/// - Zero callees: No adjustment (not a real orchestrator)
/// - High quality (0.9+): ~30% reduction (base 20% + quality bonus 10%)
/// - Low quality (0.5): ~25% reduction (base 20% + quality bonus 5%)
fn adjust_orchestrator_score(
    config: &OrchestrationAdjustmentConfig,
    base_score: f64,
    metrics: &CompositionMetrics,
) -> ScoreAdjustment {
    // Handle zero-callee edge case
    if metrics.callee_count == 0 {
        return ScoreAdjustment {
            original_score: base_score,
            adjusted_score: base_score,
            reduction_percent: 0.0,
            adjustment_reason: "Orchestrator with zero callees (no adjustment)".to_string(),
            quality_score: 0.0,
        };
    }

    // Base reduction for all orchestrators
    let base_reduction = config.base_orchestrator_reduction;

    // Calculate composition quality score
    let quality_score = calculate_composition_quality(config, metrics);

    // Apply quality bonus (scaled by quality score)
    let quality_bonus = config.max_quality_bonus * quality_score;

    // Total reduction (capped at max)
    let total_reduction = (base_reduction + quality_bonus).min(config.max_total_reduction);

    // Calculate adjusted score
    let reduction_factor = 1.0 - total_reduction;
    let adjusted = base_score * reduction_factor;

    // Apply minimum complexity floor (never reduce below inherent coordination complexity)
    let min_complexity = metrics.callee_count as f64 * config.min_inherent_complexity_factor;
    let final_score = adjusted.max(min_complexity);

    let actual_reduction = (base_score - final_score) / base_score;

    ScoreAdjustment {
        original_score: base_score,
        adjusted_score: final_score,
        reduction_percent: actual_reduction * 100.0,
        adjustment_reason: format!(
            "Orchestrator (callees: {}, delegation: {:.1}%, quality: {:.2})",
            metrics.callee_count,
            metrics.delegation_ratio * 100.0,
            quality_score
        ),
        quality_score,
    }
}

/// Calculate composition quality score (pure function)
///
/// Returns a quality score between config.min_composition_quality and 1.0.
/// Higher quality (more callees, higher delegation, lower complexity) increases
/// the quality bonus applied to the base reduction.
///
/// # Quality Factors
///
/// - **Callee count**: max +0.4 (more delegation is better)
///   - 0-1 callees: 0.0
///   - 2 callees: 0.1
///   - 3 callees: 0.2
///   - 4 callees: 0.3
///   - 5 callees: 0.35
///   - 6+ callees: 0.4
///
/// - **Delegation ratio**: max +0.4 (higher ratio is better)
///   - <0.2: 0.0
///   - 0.2-0.5: scales linearly to 0.4
///   - ≥0.5: 0.4
///
/// - **Low local complexity**: max +0.2 (simpler coordination is better)
///   - 0-2: 0.2
///   - 3: 0.15
///   - 4: 0.1
///   - 5: 0.05
///   - 6+: 0.0
///
/// # Example
///
/// ```text
/// Excellent: 8 callees, 60% delegation, complexity 2
/// → callee_quality(0.4) + delegation_quality(0.4) + complexity_quality(0.2) = 1.0
///
/// Good: 4 callees, 30% delegation, complexity 3
/// → callee_quality(0.3) + delegation_quality(0.13) + complexity_quality(0.15) = 0.58
///
/// Poor: 2 callees, 10% delegation, complexity 8
/// → callee_quality(0.1) + delegation_quality(0.0) + complexity_quality(0.0) = 0.1 → clamped to min (0.5)
/// ```
fn calculate_composition_quality(
    config: &OrchestrationAdjustmentConfig,
    metrics: &CompositionMetrics,
) -> f64 {
    // More callees = better orchestration (max +0.4)
    // Scale from 0.0 at 2 callees to 0.4 at 6+ callees
    let callee_quality = match metrics.callee_count {
        0..=1 => 0.0,
        2 => 0.1,
        3 => 0.2,
        4 => 0.3,
        5 => 0.35,
        _ => 0.4, // 6+ callees
    };

    // High delegation ratio = better orchestration (max +0.4)
    // Scale from 0.0 at ratio=0.2 to 0.4 at ratio=0.5+
    let delegation_quality = if metrics.delegation_ratio >= 0.5 {
        0.4
    } else if metrics.delegation_ratio >= 0.2 {
        (metrics.delegation_ratio - 0.2) / 0.3 * 0.4
    } else {
        0.0
    };

    // Low local complexity = cleaner orchestration (max +0.2)
    // Lower is better for orchestrators
    let complexity_quality = match metrics.local_complexity {
        0..=2 => 0.2,
        3 => 0.15,
        4 => 0.1,
        5 => 0.05,
        _ => 0.0,
    };

    // Combine quality factors
    let quality = callee_quality + delegation_quality + complexity_quality;

    // Clamp to [min_composition_quality, 1.0]
    quality.min(1.0).max(config.min_composition_quality)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    // ========================================================================
    // Configuration Tests
    // ========================================================================

    #[test]
    fn test_default_config_is_valid() {
        let config = OrchestrationAdjustmentConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_validation_invalid_base_reduction() {
        let mut config = OrchestrationAdjustmentConfig::default();
        config.base_orchestrator_reduction = 1.5;
        assert!(config.validate().is_err());

        config.base_orchestrator_reduction = -0.1;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validation_base_plus_bonus_exceeds_max() {
        let mut config = OrchestrationAdjustmentConfig::default();
        config.base_orchestrator_reduction = 0.25;
        config.max_quality_bonus = 0.15;
        config.max_total_reduction = 0.30;
        // 0.25 + 0.15 = 0.40 > 0.30 (max)
        assert!(config.validate().is_err(), "base + bonus must be <= max");
    }

    #[test]
    fn test_config_validation_min_complexity_factor() {
        let mut config = OrchestrationAdjustmentConfig::default();
        config.min_inherent_complexity_factor = 0.0;
        assert!(config.validate().is_err());

        config.min_inherent_complexity_factor = -1.0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_disabled_config() {
        let mut config = OrchestrationAdjustmentConfig::default();
        config.enabled = false;

        let role = FunctionRole::Orchestrator;
        let metrics = CompositionMetrics {
            callee_count: 8,
            delegation_ratio: 0.5,
            local_complexity: 2,
        };

        let adjustment = adjust_score(&config, 100.0, &role, &metrics);
        assert_eq!(adjustment.reduction_percent, 0.0);
        assert_eq!(adjustment.adjusted_score, 100.0);
    }

    // ========================================================================
    // Orchestrator Adjustment Tests
    // ========================================================================

    #[test]
    fn test_high_quality_orchestrator_max_reduction() {
        let config = OrchestrationAdjustmentConfig::default();
        let role = FunctionRole::Orchestrator;

        // Excellent quality: 6 callees, 50% delegation, complexity 2
        let metrics = CompositionMetrics {
            callee_count: 6,
            delegation_ratio: 0.5,
            local_complexity: 2,
        };

        let adjustment = adjust_score(&config, 100.0, &role, &metrics);

        // Base 20% + quality bonus (up to 10%) = ~30% reduction
        assert!(
            adjustment.reduction_percent >= 25.0 && adjustment.reduction_percent <= 30.0,
            "Expected reduction between 25-30%, got {}",
            adjustment.reduction_percent
        );
        assert!(
            adjustment.adjusted_score >= 70.0 && adjustment.adjusted_score <= 75.0,
            "Expected adjusted score between 70-75, got {}",
            adjustment.adjusted_score
        );
        assert!(
            adjustment.quality_score >= 0.8,
            "Expected high quality score, got {}",
            adjustment.quality_score
        );
    }

    #[test]
    fn test_minimum_complexity_floor() {
        let config = OrchestrationAdjustmentConfig::default();
        let role = FunctionRole::Orchestrator;

        // High quality metrics but low base score
        let metrics = CompositionMetrics {
            callee_count: 10,
            delegation_ratio: 0.6,
            local_complexity: 2,
        };

        // Even with high reduction, should not go below callee_count × 2
        let adjustment = adjust_score(&config, 30.0, &role, &metrics);
        assert!(
            adjustment.adjusted_score >= 20.0,
            "Score should not go below 20 (10 callees × 2), got {}",
            adjustment.adjusted_score
        );
    }

    #[test]
    fn test_zero_callee_edge_case() {
        let config = OrchestrationAdjustmentConfig::default();
        let role = FunctionRole::Orchestrator;

        let metrics = CompositionMetrics {
            callee_count: 0,
            delegation_ratio: 0.0,
            local_complexity: 5,
        };

        let adjustment = adjust_score(&config, 100.0, &role, &metrics);
        assert_eq!(adjustment.reduction_percent, 0.0);
        assert_eq!(adjustment.adjusted_score, 100.0);
        assert!(adjustment.adjustment_reason.contains("zero callees"));
    }

    // ========================================================================
    // Composition Quality Tests
    // ========================================================================

    #[test]
    fn test_composition_quality_excellent() {
        let config = OrchestrationAdjustmentConfig::default();

        // Excellent quality: 6+ callees, high delegation (0.5+), low complexity (≤2)
        let excellent = CompositionMetrics {
            callee_count: 8,
            delegation_ratio: 0.6,
            local_complexity: 2,
        };
        let quality = calculate_composition_quality(&config, &excellent);
        assert!(
            quality >= 0.9,
            "Excellent quality should be >= 0.9, got {}",
            quality
        );
    }

    #[test]
    fn test_composition_quality_good() {
        let config = OrchestrationAdjustmentConfig::default();

        // Good quality: 4 callees, decent delegation (0.3), medium complexity (3)
        let good = CompositionMetrics {
            callee_count: 4,
            delegation_ratio: 0.3,
            local_complexity: 3,
        };
        let quality = calculate_composition_quality(&config, &good);
        assert!(
            quality >= 0.5 && quality < 0.9,
            "Good quality should be 0.5-0.9, got {}",
            quality
        );
    }

    #[test]
    fn test_composition_quality_poor() {
        let config = OrchestrationAdjustmentConfig::default();

        // Poor quality: 2 callees, low delegation (0.1), high complexity (8)
        let poor = CompositionMetrics {
            callee_count: 2,
            delegation_ratio: 0.1,
            local_complexity: 8,
        };
        let quality = calculate_composition_quality(&config, &poor);
        assert!(
            quality >= config.min_composition_quality && quality < 0.5,
            "Poor quality should be between min and 0.5, got {}",
            quality
        );
    }

    #[test]
    fn test_composition_quality_respects_config_minimum() {
        let mut config = OrchestrationAdjustmentConfig::default();
        config.min_composition_quality = 0.6;

        // Worst possible metrics
        let worst = CompositionMetrics {
            callee_count: 0,
            delegation_ratio: 0.0,
            local_complexity: 20,
        };

        let quality = calculate_composition_quality(&config, &worst);
        assert_eq!(
            quality, 0.6,
            "Quality should never go below configured minimum"
        );
    }

    // ========================================================================
    // Non-Orchestrator Role Tests
    // ========================================================================

    #[test]
    fn test_non_orchestrator_roles_no_adjustment() {
        let config = OrchestrationAdjustmentConfig::default();
        let metrics = CompositionMetrics {
            callee_count: 5,
            delegation_ratio: 0.3,
            local_complexity: 3,
        };

        // PureLogic should not receive adjustment
        let pure_logic = FunctionRole::PureLogic;
        let adj = adjust_score(&config, 100.0, &pure_logic, &metrics);
        assert_eq!(adj.reduction_percent, 0.0);
        assert_eq!(adj.adjusted_score, 100.0);

        // IOWrapper should not receive adjustment
        let io_wrapper = FunctionRole::IOWrapper;
        let adj = adjust_score(&config, 100.0, &io_wrapper, &metrics);
        assert_eq!(adj.reduction_percent, 0.0);
        assert_eq!(adj.adjusted_score, 100.0);

        // EntryPoint should not receive adjustment
        let entry_point = FunctionRole::EntryPoint;
        let adj = adjust_score(&config, 100.0, &entry_point, &metrics);
        assert_eq!(adj.reduction_percent, 0.0);
        assert_eq!(adj.adjusted_score, 100.0);

        // PatternMatch should not receive adjustment
        let pattern_match = FunctionRole::PatternMatch;
        let adj = adjust_score(&config, 100.0, &pattern_match, &metrics);
        assert_eq!(adj.reduction_percent, 0.0);
        assert_eq!(adj.adjusted_score, 100.0);

        // Unknown should not receive adjustment
        let unknown = FunctionRole::Unknown;
        let adj = adjust_score(&config, 100.0, &unknown, &metrics);
        assert_eq!(adj.reduction_percent, 0.0);
        assert_eq!(adj.adjusted_score, 100.0);
    }

    // ========================================================================
    // Property-Based Tests
    // ========================================================================

    proptest! {
        #[test]
        fn prop_adjusted_score_never_exceeds_original(
            base_score in 1.0f64..1000.0,
            callee_count in 1usize..50,
            delegation_ratio in 0.0f64..1.0,
            complexity in 0u32..20
        ) {
            let config = OrchestrationAdjustmentConfig::default();
            let role = FunctionRole::Orchestrator;
            let metrics = CompositionMetrics {
                callee_count,
                delegation_ratio,
                local_complexity: complexity,
            };

            let adjustment = adjust_score(&config, base_score, &role, &metrics);

            prop_assert!(adjustment.adjusted_score <= adjustment.original_score);
        }

        #[test]
        fn prop_adjusted_score_respects_minimum_floor(
            base_score in 10.0f64..1000.0,
            callee_count in 1usize..50
        ) {
            let config = OrchestrationAdjustmentConfig::default();
            let role = FunctionRole::Orchestrator;
            // Use high-quality metrics to maximize reduction
            let metrics = CompositionMetrics {
                callee_count,
                delegation_ratio: 0.8,
                local_complexity: 2,
            };

            let adjustment = adjust_score(&config, base_score, &role, &metrics);
            let min_floor = callee_count as f64 * config.min_inherent_complexity_factor;

            prop_assert!(
                adjustment.adjusted_score >= min_floor,
                "Adjusted score {} must be >= floor {}",
                adjustment.adjusted_score,
                min_floor
            );
        }

        #[test]
        fn prop_composition_quality_bounded(
            callee_count in 0usize..20,
            delegation_ratio in 0.0f64..1.0,
            complexity in 0u32..20
        ) {
            let config = OrchestrationAdjustmentConfig::default();
            let metrics = CompositionMetrics {
                callee_count,
                delegation_ratio,
                local_complexity: complexity,
            };

            let quality = calculate_composition_quality(&config, &metrics);

            prop_assert!(
                quality >= config.min_composition_quality && quality <= 1.0,
                "Quality {} must be in [{}, 1.0]",
                quality,
                config.min_composition_quality
            );
        }
    }
}
