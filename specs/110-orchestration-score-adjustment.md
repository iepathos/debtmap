---
number: 110
title: Orchestration Pattern Score Adjustment
category: optimization
priority: high
status: draft
dependencies: [109]
created: 2025-10-16
---

# Specification 110: Orchestration Pattern Score Adjustment

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: [109 - Call Graph Role Classification]

## Context

Building on spec 109's role classification system, this specification implements score adjustments that reduce false positives for legitimate orchestrator functions while preserving full scoring for complex business logic.

**Current problem**: Even after identifying orchestrators (spec 109), they receive the same debt scores as tangled business logic, leading to:
- Orchestrator functions appearing as top debt priorities
- Discouraging functional composition patterns
- False alarms that reduce trust in debtmap recommendations

**Example**: `create_unified_analysis_with_exclusions` (complexity 17) that coordinates 6 pure functions should score lower than a monolithic function with complexity 17 from nested conditionals.

## Objective

Implement graduated score adjustments for orchestrator functions based on confidence level and composition quality, reducing false positive debt scores by 40-60% while maintaining accurate scores for genuine technical debt.

## Requirements

### Functional Requirements

1. **Score Adjustment Algorithm**:
   - Calculate adjusted complexity based on role and confidence
   - Apply graduated reductions (not fixed percentages)
   - Cap maximum reduction at 30% to prevent over-optimization
   - Never reduce below delegation count × 2 (minimum inherent complexity)

2. **Orchestrator Scoring Rules**:
   - High confidence (≥0.7): Up to 30% reduction
   - Medium confidence (0.5-0.7): Up to 20% reduction
   - Low confidence (<0.5): Up to 10% reduction
   - Reduction scales with composition quality (pure calls, shallow depth)

3. **Worker Function Preservation**:
   - Full scoring applied to workers (no reduction)
   - Pure workers get 10% reduction (encourage pure functions)
   - Impure workers with high complexity remain high priority

4. **Entry Point Handling**:
   - Shallow entry points (<3 depth): No adjustment
   - Deep entry points (≥3 depth): 15% reduction
   - Recognize that entry points naturally coordinate

5. **Composition Quality Scoring**:
   - More pure function calls → higher reduction
   - Shallower call depth → higher reduction
   - Higher delegation ratio → higher reduction
   - Composite score determines final adjustment

### Non-Functional Requirements

- **Transparency**: Adjustments logged in verbose mode
- **Configurability**: Thresholds adjustable via config
- **Auditability**: Store original and adjusted scores in output
- **Performance**: Adjustment calculation adds < 5% overhead
- **Consistency**: Same function always gets same adjustment

## Acceptance Criteria

### Functional Requirements
- [ ] `adjust_score()` pure function reduces scores for orchestrators
- [ ] High-confidence orchestrators (≥0.7) receive up to 30% reduction
- [ ] Medium-confidence orchestrators (0.5-0.7) receive up to 20% reduction
- [ ] Low-confidence orchestrators (<0.5) receive up to 10% reduction
- [ ] Reduction never goes below delegation_count × min_inherent_complexity_factor
- [ ] Zero-coordination orchestrators receive no adjustment
- [ ] Worker functions receive no orchestration adjustment
- [ ] Pure workers receive 10% reduction
- [ ] Deep entry points (≥3 depth) receive 15% reduction
- [ ] Composition quality factors (pure calls, depth, delegation) influence adjustment
- [ ] Composition quality respects configurable minimum threshold

### Data & Configuration
- [ ] Original and adjusted scores stored in `UnifiedDebtItem`
- [ ] Configuration validation prevents invalid threshold values
- [ ] Configuration validation enforces reduction ordering (high ≥ medium ≥ low)
- [ ] Disabled configuration bypasses all adjustments
- [ ] Configuration allows tuning all reduction percentages and thresholds

### Testing & Validation
- [ ] Unit tests verify adjustments for all role types
- [ ] Unit tests verify all edge cases (zero coordination, disabled config, etc.)
- [ ] Property-based tests verify score invariants (never exceeds original, respects floor)
- [ ] Property-based tests verify quality bounds
- [ ] Integration tests with real codebases show 40-60% false positive reduction
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
2. Define newtype wrappers (`Confidence`, `ReductionPercent`) for type safety
3. Implement pure function for score adjustment calculation
4. Add pure function for composition quality scoring
5. Add pure function for base reduction calculation
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

use crate::priority::call_graph::roles::{FunctionRole, RoleMetrics};
use anyhow::{ensure, Result};

/// Confidence score bounded between 0.0 and 1.0
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Confidence(f64);

impl Confidence {
    pub fn new(value: f64) -> Result<Self> {
        ensure!(
            (0.0..=1.0).contains(&value),
            "Confidence must be between 0.0 and 1.0, got {}",
            value
        );
        Ok(Self(value))
    }

    pub fn value(&self) -> f64 {
        self.0
    }
}

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
    pub high_confidence_reduction: f64,    // Default: 0.30 (30%)
    pub medium_confidence_reduction: f64,  // Default: 0.20 (20%)
    pub low_confidence_reduction: f64,     // Default: 0.10 (10%)
    pub pure_worker_reduction: f64,        // Default: 0.10 (10%)
    pub entry_point_reduction: f64,        // Default: 0.15 (15%)
    pub min_inherent_complexity_factor: f64, // Default: 2.0
    pub min_composition_quality: f64,      // Default: 0.5 (minimum quality multiplier)
}

impl Default for OrchestrationAdjustmentConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            high_confidence_reduction: 0.30,
            medium_confidence_reduction: 0.20,
            low_confidence_reduction: 0.10,
            pure_worker_reduction: 0.10,
            entry_point_reduction: 0.15,
            min_inherent_complexity_factor: 2.0,
            min_composition_quality: 0.5,
        }
    }
}

impl OrchestrationAdjustmentConfig {
    /// Validate configuration values
    pub fn validate(&self) -> Result<()> {
        ensure!(
            (0.0..=1.0).contains(&self.high_confidence_reduction),
            "high_confidence_reduction must be between 0.0 and 1.0"
        );
        ensure!(
            (0.0..=1.0).contains(&self.medium_confidence_reduction),
            "medium_confidence_reduction must be between 0.0 and 1.0"
        );
        ensure!(
            (0.0..=1.0).contains(&self.low_confidence_reduction),
            "low_confidence_reduction must be between 0.0 and 1.0"
        );
        ensure!(
            self.high_confidence_reduction >= self.medium_confidence_reduction,
            "high_confidence_reduction must be >= medium_confidence_reduction"
        );
        ensure!(
            self.medium_confidence_reduction >= self.low_confidence_reduction,
            "medium_confidence_reduction must be >= low_confidence_reduction"
        );
        ensure!(
            (0.0..=1.0).contains(&self.pure_worker_reduction),
            "pure_worker_reduction must be between 0.0 and 1.0"
        );
        ensure!(
            (0.0..=1.0).contains(&self.entry_point_reduction),
            "entry_point_reduction must be between 0.0 and 1.0"
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
    pub confidence: f64,
}

impl ScoreAdjustment {
    pub fn no_adjustment(score: f64) -> Self {
        Self {
            original_score: score,
            adjusted_score: score,
            reduction_percent: 0.0,
            adjustment_reason: "No adjustment applied".to_string(),
            confidence: 1.0,
        }
    }
}

// ============================================================================
// Pure Functions for Score Adjustment (Functional Programming Approach)
// ============================================================================

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
    complexity: u32,
    role: &FunctionRole,
    metrics: &RoleMetrics,
) -> ScoreAdjustment {
    if !config.enabled {
        return ScoreAdjustment::no_adjustment(base_score);
    }

    match role {
        FunctionRole::Orchestrator { coordinates, confidence } => {
            adjust_orchestrator_score(config, base_score, complexity, *coordinates, *confidence, metrics)
        },
        FunctionRole::Worker { is_pure, .. } => {
            adjust_worker_score(config, base_score, *is_pure)
        },
        FunctionRole::EntryPoint { downstream_depth } => {
            adjust_entry_point_score(config, base_score, *downstream_depth)
        },
        FunctionRole::Utility => {
            ScoreAdjustment::no_adjustment(base_score)
        },
    }
}

/// Adjust score for orchestrator functions (pure function)
///
/// Applies graduated reductions based on:
/// - Confidence level (how sure we are this is an orchestrator)
/// - Composition quality (purity, depth, delegation ratio)
/// - Minimum complexity floor (coordinates × factor)
fn adjust_orchestrator_score(
    config: &OrchestrationAdjustmentConfig,
    base_score: f64,
    complexity: u32,
    coordinates: usize,
    confidence: f64,
    metrics: &RoleMetrics,
) -> ScoreAdjustment {
    // Handle zero-coordination edge case
    if coordinates == 0 {
        return ScoreAdjustment {
            original_score: base_score,
            adjusted_score: base_score,
            reduction_percent: 0.0,
            adjustment_reason: "Orchestrator with zero coordination (no adjustment)".to_string(),
            confidence,
        };
    }

    // Determine base reduction percentage based on confidence
    let base_reduction = calculate_base_reduction(config, confidence);

    // Apply composition quality multiplier
    let quality_multiplier = calculate_composition_quality(config, metrics);
    let final_reduction = base_reduction * quality_multiplier;

    // Calculate adjusted score
    let reduction_factor = 1.0 - final_reduction;
    let adjusted = base_score * reduction_factor;

    // Apply minimum complexity floor (never reduce below inherent coordination complexity)
    let min_complexity = coordinates as f64 * config.min_inherent_complexity_factor;
    let final_score = adjusted.max(min_complexity);

    let actual_reduction = (base_score - final_score) / base_score;

    ScoreAdjustment {
        original_score: base_score,
        adjusted_score: final_score,
        reduction_percent: actual_reduction * 100.0,
        adjustment_reason: format!(
            "Orchestrator (confidence: {:.1}%, coordinates: {}, quality: {:.2})",
            confidence * 100.0,
            coordinates,
            quality_multiplier
        ),
        confidence,
    }
}

/// Calculate base reduction percentage based on confidence (pure function)
///
/// Returns a graduated reduction percentage:
/// - ≥0.7: High confidence reduction
/// - 0.5-0.7: Graduated between low and medium
/// - <0.5: Graduated below low
fn calculate_base_reduction(config: &OrchestrationAdjustmentConfig, confidence: f64) -> f64 {
    if confidence >= 0.7 {
        config.high_confidence_reduction
    } else if confidence >= 0.5 {
        // Graduated between low and medium
        let t = (confidence - 0.5) / 0.2; // 0.0 at 0.5, 1.0 at 0.7
        config.low_confidence_reduction
            + t * (config.medium_confidence_reduction - config.low_confidence_reduction)
    } else {
        // Graduated below 0.5
        let t = confidence / 0.5; // 0.0 at 0.0, 1.0 at 0.5
        t * config.low_confidence_reduction
    }
}

/// Calculate composition quality multiplier (pure function)
///
/// Returns a quality score between config.min_composition_quality and 1.0.
/// Higher quality (more pure calls, shallower depth, higher delegation) increases
/// the effectiveness of the base reduction.
///
/// Quality factors:
/// - Pure function ratio: max +0.3
/// - Call depth: max +0.3
/// - Delegation ratio: max +0.4
fn calculate_composition_quality(config: &OrchestrationAdjustmentConfig, metrics: &RoleMetrics) -> f64 {
    // Pure function calls increase quality (max +0.3)
    let pure_ratio = if metrics.callee_count > 0 {
        metrics.pure_callee_count as f64 / metrics.callee_count as f64
    } else {
        0.0
    };
    let pure_quality = pure_ratio * 0.3;

    // Shallow call depth increases quality (max +0.3)
    let depth_quality = match metrics.avg_call_depth {
        0..=1 => 0.3,
        2 => 0.2,
        3 => 0.1,
        _ => 0.0,
    };

    // High delegation ratio increases quality (max +0.4)
    // Scale from 0.0 at delegation_ratio=0.5 to 0.4 at delegation_ratio=1.0
    let delegation_quality = (metrics.delegation_ratio - 0.5).max(0.0) * 0.8;

    // Combine quality factors
    let quality = pure_quality + depth_quality + delegation_quality;

    // Clamp to [min_composition_quality, 1.0]
    quality.min(1.0).max(config.min_composition_quality)
}

/// Adjust score for worker functions (pure function)
fn adjust_worker_score(config: &OrchestrationAdjustmentConfig, base_score: f64, is_pure: bool) -> ScoreAdjustment {
    if is_pure {
        let reduction_factor = 1.0 - config.pure_worker_reduction;
        let adjusted = base_score * reduction_factor;

        ScoreAdjustment {
            original_score: base_score,
            adjusted_score: adjusted,
            reduction_percent: config.pure_worker_reduction * 100.0,
            adjustment_reason: "Pure worker function".to_string(),
            confidence: 1.0,
        }
    } else {
        // No adjustment for impure workers
        ScoreAdjustment::no_adjustment(base_score)
    }
}

/// Adjust score for entry point functions (pure function)
fn adjust_entry_point_score(config: &OrchestrationAdjustmentConfig, base_score: f64, depth: u32) -> ScoreAdjustment {
    if depth >= 3 {
        let reduction_factor = 1.0 - config.entry_point_reduction;
        let adjusted = base_score * reduction_factor;

        ScoreAdjustment {
            original_score: base_score,
            adjusted_score: adjusted,
            reduction_percent: config.entry_point_reduction * 100.0,
            adjustment_reason: format!("Entry point with depth {}", depth),
            confidence: 0.8,
        }
    } else {
        ScoreAdjustment::no_adjustment(base_score)
    }
}
```

### Integration with Scoring Pipeline

```rust
// src/priority/scoring/computation.rs

use crate::priority::scoring::orchestration_adjustment::{adjust_score, ScoreAdjustment};

/// Calculate final score with all adjustments applied
///
/// # Score Calculation Pipeline
/// 1. Base complexity score (cyclomatic, cognitive, etc.)
/// 2. Entropy dampening (reduces noise in scattered complexity)
/// 3. Orchestration adjustment (reduces orchestrator false positives)
///
/// Each step is a pure transformation of the score from the previous step.
pub fn calculate_final_score_with_adjustments(
    function: &FunctionMetrics,
    call_graph: &CallGraph,
    config: &ScoringConfig,
) -> (f64, Option<ScoreAdjustment>) {
    // Step 1: Calculate base score (existing logic)
    let base_score = calculate_base_complexity_score(function);

    // Step 2: Apply entropy dampening (existing)
    let after_entropy = function.entropy_score
        .as_ref()
        .map(|entropy| apply_entropy_dampening(base_score as u32, entropy) as f64)
        .unwrap_or(base_score);

    // Step 3: Apply orchestration adjustment (new)
    let adjustment = match (&function.function_role, &function.role_metrics) {
        (Some(role), Some(metrics)) => {
            let adj = adjust_score(
                &config.orchestration_adjustment,
                after_entropy,
                function.cyclomatic,
                role,
                metrics,
            );
            (adj.adjusted_score, Some(adj))
        }
        _ => (after_entropy, None),
    };

    adjustment
}

/// Functional composition approach (alternative implementation)
///
/// This demonstrates how the scoring pipeline can be expressed as
/// a composition of pure functions.
pub fn calculate_score_functional(
    function: &FunctionMetrics,
    config: &ScoringConfig,
) -> (f64, Option<ScoreAdjustment>) {
    // Compose the scoring pipeline
    let score_pipeline = |base: f64| -> (f64, Option<ScoreAdjustment>) {
        let with_entropy = function.entropy_score
            .as_ref()
            .map(|e| apply_entropy_dampening(base as u32, e) as f64)
            .unwrap_or(base);

        match (&function.function_role, &function.role_metrics) {
            (Some(role), Some(metrics)) => {
                let adj = adjust_score(&config.orchestration_adjustment, with_entropy, function.cyclomatic, role, metrics);
                (adj.adjusted_score, Some(adj))
            }
            _ => (with_entropy, None),
        }
    };

    let base = calculate_base_complexity_score(function);
    score_pipeline(base)
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
  - Spec 109 (Call Graph Role Classification) - Must be implemented first
- **Affected Components**:
  - `src/priority/scoring/computation.rs` - Score calculation
  - `src/priority/scoring/debt_item.rs` - Debt item creation
  - `src/priority/unified_scorer.rs` - Unified scoring
  - `src/config.rs` - Add adjustment configuration
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
    fn test_high_confidence_orchestrator_max_reduction() {
        let config = OrchestrationAdjustmentConfig::default();

        let role = FunctionRole::Orchestrator {
            coordinates: 8,
            confidence: 0.9,
        };

        let metrics = RoleMetrics {
            delegation_ratio: 0.85,
            pure_callee_count: 7,
            callee_count: 8,
            avg_call_depth: 1,
            local_complexity: 2,
            caller_count: 3,
        };

        let adjustment = adjust_score(&config, 100.0, 17, &role, &metrics);

        // High confidence (0.9) + excellent quality should give ~30% reduction
        assert!(adjustment.reduction_percent >= 25.0 && adjustment.reduction_percent <= 30.0);
        assert!(adjustment.adjusted_score >= 70.0 && adjustment.adjusted_score <= 75.0);
    }

    #[test]
    fn test_minimum_complexity_floor() {
        let config = OrchestrationAdjustmentConfig::default();

        let role = FunctionRole::Orchestrator {
            coordinates: 10,
            confidence: 0.95,
        };

        let metrics = create_high_quality_metrics(10);

        // Even with high reduction, should not go below coordinates × 2
        let adjustment = adjust_score(&config, 30.0, 15, &role, &metrics);
        assert!(adjustment.adjusted_score >= 20.0); // 10 × 2 = 20
    }

    #[test]
    fn test_zero_coordination_edge_case() {
        let config = OrchestrationAdjustmentConfig::default();

        let role = FunctionRole::Orchestrator {
            coordinates: 0,
            confidence: 0.9,
        };

        let metrics = RoleMetrics::default();

        // Should not adjust if coordinates = 0
        let adjustment = adjust_score(&config, 100.0, 10, &role, &metrics);
        assert_eq!(adjustment.reduction_percent, 0.0);
        assert_eq!(adjustment.adjusted_score, 100.0);
        assert!(adjustment.adjustment_reason.contains("zero coordination"));
    }

    // ========================================================================
    // Composition Quality Tests
    // ========================================================================

    #[test]
    fn test_composition_quality_calculation() {
        let config = OrchestrationAdjustmentConfig::default();

        // Excellent quality: all pure, shallow depth, high delegation
        let excellent = RoleMetrics {
            delegation_ratio: 0.9,
            pure_callee_count: 10,
            callee_count: 10,
            avg_call_depth: 1,
            local_complexity: 2,
            caller_count: 2,
        };
        let quality = calculate_composition_quality(&config, &excellent);
        assert!(quality >= 0.9, "Excellent quality should be >= 0.9, got {}", quality);

        // Good quality: mostly pure, shallow depth, decent delegation
        let good = RoleMetrics {
            delegation_ratio: 0.75,
            pure_callee_count: 7,
            callee_count: 10,
            avg_call_depth: 2,
            local_complexity: 3,
            caller_count: 2,
        };
        let quality = calculate_composition_quality(&config, &good);
        assert!(quality >= 0.6 && quality < 0.9, "Good quality should be 0.6-0.9, got {}", quality);

        // Poor quality: few pure, deep, low delegation
        let poor = RoleMetrics {
            delegation_ratio: 0.6,
            pure_callee_count: 2,
            callee_count: 10,
            avg_call_depth: 5,
            local_complexity: 8,
            caller_count: 2,
        };
        let quality = calculate_composition_quality(&config, &poor);
        assert!(
            quality >= config.min_composition_quality && quality < 0.7,
            "Poor quality should be between min and 0.7, got {}",
            quality
        );
    }

    #[test]
    fn test_composition_quality_respects_config_minimum() {
        let mut config = OrchestrationAdjustmentConfig::default();
        config.min_composition_quality = 0.6;

        // Worst possible metrics
        let worst = RoleMetrics {
            delegation_ratio: 0.0,
            pure_callee_count: 0,
            callee_count: 10,
            avg_call_depth: 10,
            local_complexity: 20,
            caller_count: 1,
        };

        let quality = calculate_composition_quality(&config, &worst);
        assert_eq!(quality, 0.6, "Quality should never go below configured minimum");
    }

    // ========================================================================
    // Confidence Reduction Tests
    // ========================================================================

    #[test]
    fn test_graduated_confidence_reductions() {
        let config = OrchestrationAdjustmentConfig::default();

        // High confidence: 0.8
        let high = calculate_base_reduction(&config, 0.8);
        assert_eq!(high, 0.30, "Confidence >= 0.7 should use high reduction");

        // Medium confidence: 0.6
        let medium = calculate_base_reduction(&config, 0.6);
        assert!(
            medium > 0.10 && medium < 0.30,
            "Confidence 0.5-0.7 should graduate between low and medium, got {}",
            medium
        );

        // Low confidence: 0.4
        let low = calculate_base_reduction(&config, 0.4);
        assert!(low < 0.10, "Confidence < 0.5 should be < low reduction, got {}", low);

        // Edge cases
        let at_boundary = calculate_base_reduction(&config, 0.5);
        assert_eq!(at_boundary, config.low_confidence_reduction);

        let zero_confidence = calculate_base_reduction(&config, 0.0);
        assert_eq!(zero_confidence, 0.0, "Zero confidence should give zero reduction");
    }

    // ========================================================================
    // Worker Function Tests
    // ========================================================================

    #[test]
    fn test_pure_worker_reduction() {
        let config = OrchestrationAdjustmentConfig::default();

        let pure_adjustment = adjust_worker_score(&config, 100.0, true);
        assert_eq!(pure_adjustment.reduction_percent, 10.0);
        assert_eq!(pure_adjustment.adjusted_score, 90.0);

        let impure_adjustment = adjust_worker_score(&config, 100.0, false);
        assert_eq!(impure_adjustment.reduction_percent, 0.0);
        assert_eq!(impure_adjustment.adjusted_score, 100.0);
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
        config.high_confidence_reduction = 1.5;
        assert!(config.validate().is_err());

        config.high_confidence_reduction = -0.1;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validation_reduction_ordering() {
        let mut config = OrchestrationAdjustmentConfig::default();
        config.high_confidence_reduction = 0.1;
        config.medium_confidence_reduction = 0.2;
        config.low_confidence_reduction = 0.3;
        assert!(config.validate().is_err(), "High must be >= medium >= low");
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

        let role = FunctionRole::Orchestrator {
            coordinates: 8,
            confidence: 0.9,
        };
        let metrics = create_high_quality_metrics(8);

        let adjustment = adjust_score(&config, 100.0, 17, &role, &metrics);
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
            coordinates in 1usize..50,
            confidence in 0.0f64..1.0
        ) {
            let config = OrchestrationAdjustmentConfig::default();
            let role = FunctionRole::Orchestrator { coordinates, confidence };
            let metrics = create_high_quality_metrics(coordinates);

            let adjustment = adjust_score(&config, base_score, 10, &role, &metrics);

            prop_assert!(adjustment.adjusted_score <= adjustment.original_score);
        }

        #[test]
        fn prop_adjusted_score_respects_minimum_floor(
            base_score in 10.0f64..1000.0,
            coordinates in 1usize..50,
            confidence in 0.7f64..1.0  // High confidence
        ) {
            let config = OrchestrationAdjustmentConfig::default();
            let role = FunctionRole::Orchestrator { coordinates, confidence };
            let metrics = create_high_quality_metrics(coordinates);

            let adjustment = adjust_score(&config, base_score, 10, &role, &metrics);
            let min_floor = coordinates as f64 * config.min_inherent_complexity_factor;

            prop_assert!(
                adjustment.adjusted_score >= min_floor,
                "Adjusted score {} must be >= floor {}",
                adjustment.adjusted_score,
                min_floor
            );
        }

        #[test]
        fn prop_composition_quality_bounded(
            delegation_ratio in 0.0f64..1.0,
            pure_count in 0usize..20,
            total_count in 1usize..20,
            depth in 0u32..10
        ) {
            let config = OrchestrationAdjustmentConfig::default();
            let metrics = RoleMetrics {
                delegation_ratio,
                pure_callee_count: pure_count.min(total_count),
                callee_count: total_count,
                avg_call_depth: depth,
                local_complexity: 5,
                caller_count: 2,
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

    // ========================================================================
    // Helper Functions
    // ========================================================================

    fn create_high_quality_metrics(coordinates: usize) -> RoleMetrics {
        RoleMetrics {
            delegation_ratio: 0.9,
            pure_callee_count: coordinates,
            callee_count: coordinates,
            avg_call_depth: 1,
            local_complexity: 2,
            caller_count: 3,
        }
    }
}
```

### Integration Tests

1. **Real codebase validation**:
   ```rust
   #[test]
   fn test_shared_cache_orchestrator_adjustment() {
       // Analyze src/cache/shared_cache.rs
       let analysis = analyze_file("src/cache/shared_cache.rs");

       // Find orchestrator functions
       let orchestrators: Vec<_> = analysis.items.iter()
           .filter(|item| matches!(item.function_role, FunctionRole::Orchestrator { .. }))
           .collect();

       // Verify adjustments applied
       for item in orchestrators {
           assert!(item.score_adjustment.is_some());
           let adj = item.score_adjustment.as_ref().unwrap();
           assert!(adj.reduction_percent > 0.0);
           assert!(adj.adjusted_score < adj.original_score);
       }
   }
   ```

2. **False positive reduction measurement**:
   ```rust
   #[test]
   fn test_false_positive_reduction_metrics() {
       // Analyze with and without adjustments
       let without = analyze_with_config(config_no_adjustment());
       let with = analyze_with_config(config_with_adjustment());

       // Count high-priority orchestrators
       let false_positives_before = count_high_priority_orchestrators(&without);
       let false_positives_after = count_high_priority_orchestrators(&with);

       // Verify 40-60% reduction
       let reduction = (false_positives_before - false_positives_after) as f64
           / false_positives_before as f64;
       assert!(reduction >= 0.4 && reduction <= 0.6);
   }
   ```

3. **Performance benchmark**:
   ```rust
   #[bench]
   fn bench_adjustment_overhead(b: &mut Bencher) {
       let functions = load_test_functions(1000);
       let adjuster = OrchestrationAdjuster::new(Default::default());

       b.iter(|| {
           for func in &functions {
               let _ = adjuster.adjust_score(
                   black_box(100.0),
                   black_box(func.cyclomatic),
                   black_box(&func.role),
                   black_box(&func.metrics),
               );
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
- Minimum complexity floor rationale (why coordinates × factor)
- Score calculation pipeline (order of transformations)
- Newtype wrappers for type safety (`Confidence`, `ReductionPercent`)
- Edge cases and how they're handled

### User Documentation

```markdown
## Orchestration Score Adjustments

Debtmap automatically reduces complexity scores for legitimate orchestrator functions:

### How It Works

1. **Role Detection**: Functions classified as orchestrators (spec 109)
2. **Confidence Scoring**: Multi-factor confidence calculation
3. **Adjustment Calculation**: Graduated reductions based on confidence and quality
4. **Floor Protection**: Never reduce below minimum inherent complexity

### Adjustment Levels

- **High Confidence (≥70%)**: Up to 30% score reduction
- **Medium Confidence (50-70%)**: Up to 20% score reduction
- **Low Confidence (<50%)**: Up to 10% score reduction

### Quality Factors

Composition quality influences adjustment amount:
- Pure function calls (more is better)
- Call depth (shallower is better)
- Delegation ratio (higher is better)

### Configuration

Customize in `.debtmap.toml`:

```toml
[orchestration_adjustment]
enabled = true
high_confidence_reduction = 0.30  # 30% (must be >= medium)
medium_confidence_reduction = 0.20  # 20% (must be >= low)
low_confidence_reduction = 0.10  # 10%
pure_worker_reduction = 0.10  # 10%
entry_point_reduction = 0.15  # 15%
min_inherent_complexity_factor = 2.0  # Floor = coordinates × this
min_composition_quality = 0.5  # Minimum quality multiplier
```

### Viewing Adjustments

Use verbose mode to see adjustment details:

```bash
debtmap analyze src -v
```

Output:
```
📊 create_unified_analysis_with_exclusions - 17.0 → 12.5 (26.5% reduction)
   Reason: Orchestrator (confidence: 85.0%, coordinates: 6, quality: 0.92)
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
   base_score → entropy_dampening → orchestration_adjustment → final_score
   ```

### Formula Derivation

The adjustment formula balances several competing concerns:

1. **Confidence**: Higher confidence → higher reduction
2. **Quality**: Better composition → higher reduction
3. **Floor**: Minimum complexity based on coordination count
4. **Conservatism**: Cap at 30% to avoid over-reduction

```rust
reduction = base_reduction(confidence) × composition_quality(metrics)
adjusted = max(base_score × (1 - reduction), coordinates × min_inherent_complexity_factor)
```

Key insight: The formula is **multiplicative** (confidence × quality), not additive, which naturally handles edge cases where either factor is low.

### Tuning Process

1. Analyze 50+ real-world Rust projects
2. Hand-label 500+ functions as orchestrator vs worker
3. Measure false positive rates at different thresholds
4. Tune for optimal precision/recall tradeoff
5. Validate on held-out test set

### Edge Cases

1. **Zero Coordination**: Functions with 0 coordinates receive no adjustment (explicit check)
2. **Extreme Delegators** (100% delegation): Still apply floor to prevent unrealistic low scores
3. **Recursive Orchestrators**: Call depth calculation prevents infinite recursion
4. **Mixed Patterns**: Confidence scoring and quality metrics handle ambiguous cases
5. **Configuration Extremes**: Validate config values at load time (using `validate()` method)
6. **Disabled Configuration**: Early return with no adjustment when `enabled = false`
7. **Quality Below Minimum**: Composition quality clamped to `min_composition_quality`

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

- **False Positive Reduction**: 40-60% fewer orchestrators in top 10 debt items
- **Precision**: ≥90% of adjusted items are true orchestrators
- **Recall**: ≥80% of orchestrators receive adjustments
- **Performance**: < 5% overhead on analysis time
- **No Regressions**: Zero increase in false negatives (missing real debt)
- **Determinism**: Same input always produces same output (testable with property tests)
