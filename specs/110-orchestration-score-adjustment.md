---
number: 110
title: Orchestration Pattern Score Adjustment
category: optimization
priority: high
status: draft
dependencies: [117]
created: 2025-10-16
---

# Specification 110: Orchestration Pattern Score Adjustment

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: [117 - Semantic Function Classification]

## Context

Building on spec 117's semantic role classification system, this specification implements score adjustments that reduce false positives for legitimate orchestrator functions while preserving full scoring for complex business logic.

**Current problem**: Even after identifying orchestrators (spec 117), they receive the same debt scores as tangled business logic, leading to:
- Orchestrator functions appearing as top debt priorities
- Discouraging functional composition patterns
- False alarms that reduce trust in debtmap recommendations

**Example**: `create_unified_analysis_with_exclusions` (complexity 17) that coordinates 6 pure functions should score lower than a monolithic function with complexity 17 from nested conditionals.

## Objective

Implement score adjustments for orchestrator functions based on composition quality metrics calculated from the call graph, reducing false positive debt scores by 30-50% while maintaining accurate scores for genuine technical debt.

## Requirements

### Functional Requirements

1. **Score Adjustment Algorithm**:
   - Calculate adjusted complexity based on role and composition quality
   - Apply graduated reductions based on delegation ratio
   - Cap maximum reduction at 30% to prevent over-optimization
   - Never reduce below delegation count × 2 (minimum inherent complexity)

2. **Orchestrator Scoring Rules**:
   - Base reduction: 20% for identified orchestrators (from spec 117)
   - Additional reduction based on composition quality (up to 10% more)
   - Reduction scales with: delegation ratio, callee count, and low local complexity
   - Maximum total reduction: 30%

3. **Role-Based Preservation**:
   - PureLogic: No adjustment (full scoring)
   - IOWrapper: No adjustment (already has multiplier from spec 117)
   - EntryPoint: No adjustment (already has multiplier from spec 117)
   - PatternMatch: No adjustment (already has multiplier from spec 117)

4. **Composition Quality Scoring** (calculated from CallGraph):
   - More callees → higher reduction
   - Higher delegation ratio → higher reduction
   - Lower local complexity → higher reduction
   - Composite quality score (0.0-1.0) multiplies base reduction

### Non-Functional Requirements

- **Transparency**: Adjustments logged in verbose mode
- **Configurability**: Thresholds adjustable via config
- **Auditability**: Store original and adjusted scores in output
- **Performance**: Adjustment calculation adds < 5% overhead
- **Consistency**: Same function always gets same adjustment

## Acceptance Criteria

### Functional Requirements
- [ ] `adjust_score()` pure function reduces scores for orchestrators
- [ ] Orchestrators receive base 20% reduction
- [ ] Additional up to 10% reduction based on composition quality
- [ ] Maximum total reduction: 30%
- [ ] Reduction never goes below callee_count × min_inherent_complexity_factor
- [ ] Zero-callee orchestrators receive no adjustment
- [ ] PureLogic functions receive no adjustment (full scoring)
- [ ] Composition quality factors (callee count, delegation ratio, complexity) influence adjustment
- [ ] Composition quality respects configurable minimum threshold

### Data & Configuration
- [ ] Original and adjusted scores stored in `UnifiedDebtItem`
- [ ] Configuration validation prevents invalid threshold values
- [ ] Configuration validation enforces base + quality ≤ max reduction
- [ ] Disabled configuration bypasses all adjustments
- [ ] Configuration allows tuning reduction percentages and quality thresholds

### Testing & Validation
- [ ] Unit tests verify adjustments for all role types
- [ ] Unit tests verify all edge cases (zero callees, disabled config, etc.)
- [ ] Property-based tests verify score invariants (never exceeds original, respects floor)
- [ ] Property-based tests verify quality bounds
- [ ] Integration tests with real codebases show 30-50% false positive reduction
- [ ] Performance benchmarks demonstrate < 5% overhead
- [ ] Tests verify determinism (same input → same output)

### Observability
- [ ] Verbose mode logs adjustment details per function
- [ ] JSON output includes adjustment metadata and reasoning
- [ ] Score calculation pipeline is documented and clear

## Technical Details

### Implementation Approach

**Phase 1: Core Adjustment Logic** (Week 1)
1. Create `src/priority/scoring/orchestration_adjustment.rs`
2. Define newtype wrapper (`ReductionPercent`) for type safety
3. Implement pure function for score adjustment calculation
4. Add pure function for composition quality scoring from CallGraph metrics
5. Add helper to extract metrics from CallGraph (callee count, delegation ratio)
6. Create adjustment configuration structure with validation
7. Write unit tests for all pure functions
8. Write property-based tests for invariants

**Phase 2: Integration with Scoring** (Week 1)
1. Integrate with existing scoring pipeline
2. Apply adjustments after base score calculation
3. Store both original and adjusted scores
4. Update recommendation generation to use adjusted scores

**Phase 3: Reporting and Validation** (Week 1)
1. Add verbose logging for adjustments
2. Include adjustment metadata in JSON output
3. Validate with real codebases
4. Tune thresholds based on results

### Architecture Changes

```rust
// src/priority/scoring/orchestration_adjustment.rs

use crate::priority::semantic_classifier::FunctionRole;
use crate::priority::call_graph::{CallGraph, FunctionId};
use crate::core::FunctionMetrics;
use anyhow::{ensure, Result};

/// Reduction percentage bounded between 0.0 and 1.0
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct ReductionPercent(f64);

impl ReductionPercent {
    pub fn new(value: f64) -> Result<Self> {
        ensure!(
            (0.0..=1.0).contains(&value),
            "Reduction percent must be between 0.0 and 1.0, got {}",
            value
        );
        Ok(Self(value))
    }

    pub fn value(&self) -> f64 {
        self.0
    }

    pub fn as_percent(&self) -> f64 {
        self.0 * 100.0
    }
}

/// Configuration for orchestration score adjustments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestrationAdjustmentConfig {
    pub enabled: bool,
    pub base_orchestrator_reduction: f64,    // Default: 0.20 (20%)
    pub max_quality_bonus: f64,              // Default: 0.10 (10% additional)
    pub max_total_reduction: f64,            // Default: 0.30 (30% cap)
    pub min_inherent_complexity_factor: f64, // Default: 2.0
    pub min_composition_quality: f64,        // Default: 0.5 (minimum quality multiplier)
}

impl Default for OrchestrationAdjustmentConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            base_orchestrator_reduction: 0.20,
            max_quality_bonus: 0.10,
            max_total_reduction: 0.30,
            min_inherent_complexity_factor: 2.0,
            min_composition_quality: 0.5,
        }
    }
}

impl OrchestrationAdjustmentConfig {
    /// Validate configuration values
    pub fn validate(&self) -> Result<()> {
        ensure!(
            (0.0..=1.0).contains(&self.base_orchestrator_reduction),
            "base_orchestrator_reduction must be between 0.0 and 1.0"
        );
        ensure!(
            (0.0..=1.0).contains(&self.max_quality_bonus),
            "max_quality_bonus must be between 0.0 and 1.0"
        );
        ensure!(
            (0.0..=1.0).contains(&self.max_total_reduction),
            "max_total_reduction must be between 0.0 and 1.0"
        );
        ensure!(
            self.base_orchestrator_reduction + self.max_quality_bonus <= self.max_total_reduction,
            "base_orchestrator_reduction + max_quality_bonus must be <= max_total_reduction"
        );
        ensure!(
            self.min_inherent_complexity_factor > 0.0,
            "min_inherent_complexity_factor must be positive"
        );
        ensure!(
            (0.0..=1.0).contains(&self.min_composition_quality),
            "min_composition_quality must be between 0.0 and 1.0"
        );
        Ok(())
    }
}

/// Metadata about score adjustment applied
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoreAdjustment {
    pub original_score: f64,
    pub adjusted_score: f64,
    pub reduction_percent: f64,
    pub adjustment_reason: String,
    pub quality_score: f64,  // Composition quality (0.0-1.0)
}

impl ScoreAdjustment {
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
// Pure Functions for Score Adjustment (Functional Programming Approach)
// ============================================================================

/// Composition metrics extracted from call graph
#[derive(Debug, Clone)]
pub struct CompositionMetrics {
    pub callee_count: usize,
    pub delegation_ratio: f64,
    pub local_complexity: u32,
}

/// Extract composition metrics from call graph (pure function)
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

/// Apply orchestration-aware score adjustment (pure function)
///
/// This is the main entry point for score adjustment. It dispatches to
/// role-specific adjustment functions based on the function's role classification.
///
/// # Score Calculation Pipeline
/// 1. Base complexity score (cyclomatic, cognitive, etc.)
/// 2. Entropy dampening (reduces noise in scattered complexity)
/// 3. Orchestration adjustment (reduces orchestrator false positives) ← This function
///
/// Formula: `final_score = adjust_orchestration(dampen_entropy(base_score))`
pub fn adjust_score(
    config: &OrchestrationAdjustmentConfig,
    base_score: f64,
    role: &FunctionRole,
    metrics: &CompositionMetrics,
) -> ScoreAdjustment {
    if !config.enabled {
        return ScoreAdjustment::no_adjustment(base_score);
    }

    match role {
        FunctionRole::Orchestrator => {
            adjust_orchestrator_score(config, base_score, metrics)
        },
        _ => {
            // Other roles (PureLogic, IOWrapper, EntryPoint, PatternMatch, Unknown)
            // already have multipliers applied via spec 117
            ScoreAdjustment::no_adjustment(base_score)
        },
    }
}

/// Adjust score for orchestrator functions (pure function)
///
/// Applies graduated reductions based on:
/// - Base reduction for being an orchestrator (20%)
/// - Composition quality bonus (up to 10% more)
/// - Minimum complexity floor (callee_count × factor)
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
/// Quality factors:
/// - Callee count: max +0.4 (more delegation is better)
/// - Delegation ratio: max +0.4 (higher ratio is better)
/// - Low local complexity: max +0.2 (simpler coordination is better)
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
```

### Integration with Scoring Pipeline

```rust
// src/priority/scoring/computation.rs

use crate::priority::scoring::orchestration_adjustment::{
    adjust_score, extract_composition_metrics, ScoreAdjustment
};

/// Calculate final score with all adjustments applied
///
/// # Score Calculation Pipeline
/// 1. Base complexity score (cyclomatic, cognitive, etc.)
/// 2. Role multiplier (from spec 117 - applied via get_role_multiplier)
/// 3. Entropy dampening (reduces noise in scattered complexity)
/// 4. Orchestration adjustment (reduces orchestrator false positives) ← NEW
///
/// Each step is a pure transformation of the score from the previous step.
pub fn calculate_final_score_with_adjustments(
    func_id: &FunctionId,
    function: &FunctionMetrics,
    call_graph: &CallGraph,
    config: &ScoringConfig,
) -> (f64, Option<ScoreAdjustment>) {
    // Step 1: Calculate base score (existing logic)
    let base_score = calculate_base_complexity_score(function);

    // Step 2: Apply role multiplier (spec 117 - existing)
    let role = classify_function_role(function, func_id, call_graph);
    let role_multiplier = get_role_multiplier(role);
    let after_role = base_score * role_multiplier;

    // Step 3: Apply entropy dampening (existing)
    let after_entropy = function.entropy_score
        .as_ref()
        .map(|entropy| apply_entropy_dampening(after_role as u32, entropy) as f64)
        .unwrap_or(after_role);

    // Step 4: Apply orchestration adjustment (new - only for orchestrators)
    let adjustment = if matches!(role, FunctionRole::Orchestrator) {
        let metrics = extract_composition_metrics(func_id, function, call_graph);
        let adj = adjust_score(
            &config.orchestration_adjustment,
            after_entropy,
            &role,
            &metrics,
        );
        (adj.adjusted_score, Some(adj))
    } else {
        (after_entropy, None)
    };

    adjustment
}

/// Functional composition approach (alternative implementation)
///
/// This demonstrates how the scoring pipeline can be expressed as
/// a composition of pure functions.
pub fn calculate_score_functional(
    func_id: &FunctionId,
    function: &FunctionMetrics,
    call_graph: &CallGraph,
    config: &ScoringConfig,
) -> (f64, Option<ScoreAdjustment>) {
    // Compose the scoring pipeline
    let base = calculate_base_complexity_score(function);

    // Role classification and multiplier
    let role = classify_function_role(function, func_id, call_graph);
    let with_role = base * get_role_multiplier(role);

    // Entropy dampening
    let with_entropy = function.entropy_score
        .as_ref()
        .map(|e| apply_entropy_dampening(with_role as u32, e) as f64)
        .unwrap_or(with_role);

    // Orchestration adjustment (only for orchestrators)
    if matches!(role, FunctionRole::Orchestrator) {
        let metrics = extract_composition_metrics(func_id, function, call_graph);
        let adj = adjust_score(&config.orchestration_adjustment, with_entropy, &role, &metrics);
        (adj.adjusted_score, Some(adj))
    } else {
        (with_entropy, None)
    }
}
```

### Data Structure Updates

```rust
// Add to UnifiedDebtItem
pub struct UnifiedDebtItem {
    // ... existing fields ...
    pub original_complexity: u32,
    pub adjusted_complexity: Option<f64>,
    pub score_adjustment: Option<ScoreAdjustment>,
}

// Add to UnifiedScore
pub struct UnifiedScore {
    // ... existing fields ...
    pub pre_adjustment_score: f64,
    pub adjustment_applied: Option<ScoreAdjustment>,
}
```

### Verbose Logging

```rust
// In scoring calculation
if verbose_mode {
    if let Some(adj) = &score_adjustment {
        eprintln!(
            "  📊 {} - {} → {} ({:.1}% reduction)",
            function.name,
            format_score(adj.original_score),
            format_score(adj.adjusted_score),
            adj.reduction_percent
        );
        eprintln!("     Reason: {}", adj.adjustment_reason);
    }
}
```

## Dependencies

- **Prerequisites**:
  - Spec 117 (Semantic Function Classification) - Already implemented
  - Call graph analysis - Existing functionality
- **Affected Components**:
  - `src/priority/scoring/` - Create new `orchestration_adjustment.rs` module
  - `src/priority/scoring/computation.rs` - Integrate adjustment into scoring pipeline
  - `src/priority/unified_scorer.rs` - Update to use adjusted scores
  - `src/config.rs` - Add `OrchestrationAdjustmentConfig` structure
- **External Dependencies**: None

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

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
        assert!(adjustment.reduction_percent >= 25.0 && adjustment.reduction_percent <= 30.0);
        assert!(adjustment.adjusted_score >= 70.0 && adjustment.adjusted_score <= 75.0);
        assert!(adjustment.quality_score >= 0.8); // High quality
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
        assert!(adjustment.adjusted_score >= 20.0); // 10 × 2 = 20
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

        // Should not adjust if callee_count = 0
        let adjustment = adjust_score(&config, 100.0, &role, &metrics);
        assert_eq!(adjustment.reduction_percent, 0.0);
        assert_eq!(adjustment.adjusted_score, 100.0);
        assert!(adjustment.adjustment_reason.contains("zero callees"));
    }

    // ========================================================================
    // Composition Quality Tests
    // ========================================================================

    #[test]
    fn test_composition_quality_calculation() {
        let config = OrchestrationAdjustmentConfig::default();

        // Excellent quality: 6+ callees, high delegation (0.5+), low complexity (≤2)
        let excellent = CompositionMetrics {
            callee_count: 8,
            delegation_ratio: 0.6,
            local_complexity: 2,
        };
        let quality = calculate_composition_quality(&config, &excellent);
        assert!(quality >= 0.9, "Excellent quality should be >= 0.9, got {}", quality);

        // Good quality: 4 callees, decent delegation (0.3), medium complexity (3)
        let good = CompositionMetrics {
            callee_count: 4,
            delegation_ratio: 0.3,
            local_complexity: 3,
        };
        let quality = calculate_composition_quality(&config, &good);
        assert!(quality >= 0.5 && quality < 0.9, "Good quality should be 0.5-0.9, got {}", quality);

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
        assert_eq!(quality, 0.6, "Quality should never go below configured minimum");
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

        // IOWrapper should not receive adjustment (already has multiplier from spec 117)
        let io_wrapper = FunctionRole::IOWrapper;
        let adj = adjust_score(&config, 100.0, &io_wrapper, &metrics);
        assert_eq!(adj.reduction_percent, 0.0);
        assert_eq!(adj.adjusted_score, 100.0);

        // EntryPoint should not receive adjustment
        let entry_point = FunctionRole::EntryPoint;
        let adj = adjust_score(&config, 100.0, &entry_point, &metrics);
        assert_eq!(adj.reduction_percent, 0.0);
        assert_eq!(adj.adjusted_score, 100.0);
    }

    // ========================================================================
    // Configuration Validation Tests
    // ========================================================================

    #[test]
    fn test_config_validation_valid() {
        let config = OrchestrationAdjustmentConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_validation_invalid_reduction_range() {
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
```

### Integration Tests

1. **Real codebase validation**:
   ```rust
   #[test]
   fn test_real_orchestrator_adjustment() {
       // Analyze a file with known orchestrators
       let analysis = analyze_file("src/priority/unified_scorer.rs");

       // Find orchestrator functions
       let orchestrators: Vec<_> = analysis.items.iter()
           .filter(|item| matches!(item.function_role, Some(FunctionRole::Orchestrator)))
           .collect();

       // Verify adjustments applied
       for item in orchestrators {
           assert!(item.score_adjustment.is_some());
           let adj = item.score_adjustment.as_ref().unwrap();
           assert!(adj.reduction_percent > 0.0);
           assert!(adj.adjusted_score < adj.original_score);
           assert!(adj.quality_score >= 0.0 && adj.quality_score <= 1.0);
       }
   }
   ```

2. **False positive reduction measurement**:
   ```rust
   #[test]
   fn test_false_positive_reduction_metrics() {
       // Analyze with and without adjustments
       let mut config_off = ScoringConfig::default();
       config_off.orchestration_adjustment.enabled = false;

       let mut config_on = ScoringConfig::default();
       config_on.orchestration_adjustment.enabled = true;

       let without = analyze_with_config(config_off);
       let with = analyze_with_config(config_on);

       // Count high-priority orchestrators
       let false_positives_before = count_high_priority_orchestrators(&without);
       let false_positives_after = count_high_priority_orchestrators(&with);

       // Verify 30-50% reduction (updated target)
       let reduction = (false_positives_before - false_positives_after) as f64
           / false_positives_before as f64;
       assert!(reduction >= 0.3 && reduction <= 0.5);
   }
   ```

3. **Performance benchmark**:
   ```rust
   #[bench]
   fn bench_adjustment_overhead(b: &mut Bencher) {
       let functions = load_test_functions(1000);
       let config = OrchestrationAdjustmentConfig::default();
       let call_graph = build_test_call_graph(&functions);

       b.iter(|| {
           for func in &functions {
               if matches!(func.role, FunctionRole::Orchestrator) {
                   let func_id = func.get_id();
                   let metrics = extract_composition_metrics(&func_id, &func, &call_graph);
                   let _ = adjust_score(
                       black_box(&config),
                       black_box(100.0),
                       black_box(&func.role),
                       black_box(&metrics),
                   );
               }
           }
       });
   }
   ```

## Documentation Requirements

### Code Documentation

Add comprehensive rustdoc comments explaining:
- Functional programming approach (pure functions, no side effects)
- Adjustment algorithm and formulas (with examples)
- Configuration options and their effects (with validation rules)
- Composition quality calculation (factor weights and rationale)
- Minimum complexity floor rationale (why callee_count × factor)
- Score calculation pipeline (order of transformations)
- Newtype wrapper for type safety (`ReductionPercent`)
- Edge cases and how they're handled
- Integration with spec 117's role multipliers

### User Documentation

```markdown
## Orchestration Score Adjustments

Debtmap automatically reduces complexity scores for legitimate orchestrator functions identified by spec 117's semantic classification:

### How It Works

1. **Role Detection**: Functions classified as orchestrators (spec 117)
2. **Base Reduction**: All orchestrators receive 20% base reduction
3. **Quality Bonus**: Up to 10% additional reduction based on composition quality
4. **Floor Protection**: Never reduce below minimum inherent complexity (callee_count × 2)

### Adjustment Formula

```
Base reduction: 20%
Quality bonus: 0-10% (based on composition quality)
Total reduction: min(base + bonus, 30%)
Final score: max(score × (1 - reduction), callees × 2)
```

### Quality Factors

Composition quality (0.0-1.0) influences the bonus reduction:
- **Callee count**: More delegated functions (max +0.4)
- **Delegation ratio**: Higher ratio of callees to function length (max +0.4)
- **Low local complexity**: Simpler coordination logic (max +0.2)

### Configuration

Customize in `.debtmap.toml`:

```toml
[orchestration_adjustment]
enabled = true
base_orchestrator_reduction = 0.20      # 20% base reduction
max_quality_bonus = 0.10                 # Up to 10% more
max_total_reduction = 0.30               # 30% cap
min_inherent_complexity_factor = 2.0     # Floor = callees × this
min_composition_quality = 0.5            # Minimum quality score
```

### Viewing Adjustments

Use verbose mode to see adjustment details:

```bash
debtmap analyze src -v
```

Output:
```
📊 create_unified_analysis_with_exclusions - 17.0 → 12.5 (26.5% reduction)
   Reason: Orchestrator (callees: 6, delegation: 30.0%, quality: 0.92)
```
```

## Implementation Notes

### Functional Programming Approach

This implementation follows functional programming principles:

1. **Pure Functions**: All adjustment logic is implemented as pure functions that take inputs and return outputs with no side effects
2. **Immutability**: No mutable state - all transformations create new values
3. **Composability**: Functions are designed to be easily composed in pipelines
4. **Type Safety**: Newtype wrappers (e.g., `Confidence`, `ReductionPercent`) prevent invalid values at compile time
5. **Testability**: Pure functions are trivially testable and deterministic
6. **Pipeline Architecture**: Score calculation flows through pure transformations:
   ```
   base_score → role_multiplier (spec 117) → entropy_dampening → orchestration_adjustment → final_score
   ```

### Formula Derivation

The adjustment formula balances several competing concerns:

1. **Base Reduction**: Flat 20% for all identified orchestrators (from spec 117)
2. **Quality Bonus**: 0-10% additional based on composition quality
3. **Floor**: Minimum complexity based on callee count
4. **Conservatism**: Cap at 30% to avoid over-reduction

```rust
base_reduction = 0.20
quality_score = calculate_composition_quality(metrics)  // 0.0-1.0
quality_bonus = max_quality_bonus * quality_score      // 0.0-0.10
total_reduction = min(base_reduction + quality_bonus, max_total_reduction)
adjusted = max(base_score × (1 - total_reduction), callee_count × min_inherent_complexity_factor)
```

Key insight: The formula is **additive** (base + quality bonus), which is simpler than the spec 109 multiplicative approach and easier to reason about.

### Tuning Process

1. Analyze 50+ real-world Rust projects
2. Hand-label 500+ functions as orchestrator vs worker
3. Measure false positive rates at different thresholds
4. Tune for optimal precision/recall tradeoff
5. Validate on held-out test set

### Edge Cases

1. **Zero Callees**: Functions with 0 callees receive no adjustment (explicit check)
2. **Extreme Delegators** (100% delegation): Still apply floor to prevent unrealistic low scores
3. **Non-Orchestrator Roles**: Only `FunctionRole::Orchestrator` receives adjustments (all other roles return early)
4. **Configuration Extremes**: Validate config values at load time (using `validate()` method)
5. **Base + Bonus Exceeds Max**: Validation ensures `base_reduction + max_quality_bonus ≤ max_total_reduction`
6. **Disabled Configuration**: Early return with no adjustment when `enabled = false`
7. **Quality Below Minimum**: Composition quality clamped to `min_composition_quality`
8. **Integration with Spec 117 Multipliers**: Orchestration adjustment is applied **after** the 0.8 role multiplier from spec 117

### Benefits of Functional Approach

1. **Testability**: Pure functions can be tested in isolation without setup/teardown
2. **Reasoning**: No hidden state makes it easy to understand what a function does
3. **Composability**: Functions can be combined in different ways for different use cases
4. **Performance**: No synchronization needed for parallel processing
5. **Determinism**: Same inputs always produce same outputs (critical for build reproducibility)
6. **Refactoring Safety**: Type system and tests catch breaking changes

## Migration and Compatibility

### Rollout Plan

1. **Week 1-2**: Deploy with feature disabled by default
2. **Week 3-4**: Enable for beta testers, gather feedback
3. **Week 5-6**: Tune thresholds based on feedback
4. **Week 7+**: Enable by default with opt-out option

### Backward Compatibility

- Optional fields in data structures (no breaking changes)
- Config defaults match current behavior (no adjustment)
- JSON output includes original scores for comparison

### A/B Testing

```rust
// Compare results with/without adjustments
if config.orchestration_adjustment.ab_test_mode {
    let (with_adj, without_adj) = calculate_both_versions(function);
    log_comparison(function.name, with_adj, without_adj);
}
```

## Success Metrics

- **False Positive Reduction**: 30-50% fewer orchestrators in top 10 debt items (updated target - more conservative than spec 109 approach)
- **Precision**: ≥85% of adjusted items are true orchestrators
- **Recall**: ≥75% of orchestrators receive adjustments
- **Performance**: < 5% overhead on analysis time
- **No Regressions**: Zero increase in false negatives (missing real debt)
- **Determinism**: Same input always produces same output (testable with property tests)
- **Integration**: Seamless integration with spec 117's existing role multipliers
