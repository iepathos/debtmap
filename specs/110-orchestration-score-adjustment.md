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

- [ ] `apply_orchestration_adjustment()` function reduces scores for orchestrators
- [ ] High-confidence orchestrators (≥0.7) receive up to 30% reduction
- [ ] Medium-confidence orchestrators (0.5-0.7) receive up to 20% reduction
- [ ] Low-confidence orchestrators (<0.5) receive up to 10% reduction
- [ ] Reduction never goes below delegation_count × 2
- [ ] Worker functions receive no orchestration adjustment
- [ ] Pure workers receive 10% reduction
- [ ] Deep entry points (≥3 depth) receive 15% reduction
- [ ] Composition quality factors (pure calls, depth) influence adjustment
- [ ] Original and adjusted scores stored in `UnifiedDebtItem`
- [ ] Verbose mode logs adjustment details per function
- [ ] Configuration allows tuning reduction percentages
- [ ] Tests verify adjustments for all role types
- [ ] Real codebase tests show 40-60% false positive reduction
- [ ] Performance overhead < 5%

## Technical Details

### Implementation Approach

**Phase 1: Core Adjustment Logic** (Week 1)
1. Create `src/priority/scoring/orchestration_adjustment.rs`
2. Implement score adjustment calculation
3. Add composition quality scoring
4. Create adjustment configuration structure
5. Write unit tests for adjustment formulas

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
        }
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

pub struct OrchestrationAdjuster {
    config: OrchestrationAdjustmentConfig,
}

impl OrchestrationAdjuster {
    pub fn new(config: OrchestrationAdjustmentConfig) -> Self {
        Self { config }
    }

    /// Apply orchestration-aware score adjustment
    pub fn adjust_score(
        &self,
        base_score: f64,
        complexity: u32,
        role: &FunctionRole,
        metrics: &RoleMetrics,
    ) -> ScoreAdjustment {
        if !self.config.enabled {
            return ScoreAdjustment::no_adjustment(base_score);
        }

        match role {
            FunctionRole::Orchestrator { coordinates, confidence } => {
                self.adjust_orchestrator_score(base_score, complexity, *coordinates, *confidence, metrics)
            },
            FunctionRole::Worker { is_pure, .. } => {
                self.adjust_worker_score(base_score, *is_pure)
            },
            FunctionRole::EntryPoint { downstream_depth } => {
                self.adjust_entry_point_score(base_score, *downstream_depth)
            },
            FunctionRole::Utility => {
                ScoreAdjustment::no_adjustment(base_score)
            },
        }
    }

    /// Adjust score for orchestrator functions
    fn adjust_orchestrator_score(
        &self,
        base_score: f64,
        complexity: u32,
        coordinates: usize,
        confidence: f64,
        metrics: &RoleMetrics,
    ) -> ScoreAdjustment {
        // Determine base reduction percentage based on confidence
        let base_reduction = self.calculate_base_reduction(confidence);

        // Apply composition quality multiplier
        let quality_multiplier = self.calculate_composition_quality(metrics);
        let final_reduction = base_reduction * quality_multiplier;

        // Calculate adjusted score
        let reduction_factor = 1.0 - final_reduction;
        let adjusted = base_score * reduction_factor;

        // Apply minimum complexity floor
        let min_complexity = (coordinates as f64 * self.config.min_inherent_complexity_factor) as f64;
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

    /// Calculate base reduction percentage based on confidence
    fn calculate_base_reduction(&self, confidence: f64) -> f64 {
        if confidence >= 0.7 {
            self.config.high_confidence_reduction
        } else if confidence >= 0.5 {
            // Graduated between low and medium
            let t = (confidence - 0.5) / 0.2; // 0.0 at 0.5, 1.0 at 0.7
            self.config.low_confidence_reduction
                + t * (self.config.medium_confidence_reduction - self.config.low_confidence_reduction)
        } else {
            // Graduated below 0.5
            let t = confidence / 0.5; // 0.0 at 0.0, 1.0 at 0.5
            t * self.config.low_confidence_reduction
        }
    }

    /// Calculate composition quality multiplier (0.0-1.0)
    fn calculate_composition_quality(&self, metrics: &RoleMetrics) -> f64 {
        let mut quality = 0.0;

        // Pure function calls increase quality (max +0.3)
        let pure_ratio = if metrics.callee_count > 0 {
            metrics.pure_callee_count as f64 / metrics.callee_count as f64
        } else {
            0.0
        };
        quality += pure_ratio * 0.3;

        // Shallow call depth increases quality (max +0.3)
        let depth_quality = match metrics.avg_call_depth {
            0..=1 => 0.3,
            2 => 0.2,
            3 => 0.1,
            _ => 0.0,
        };
        quality += depth_quality;

        // High delegation ratio increases quality (max +0.4)
        let delegation_quality = (metrics.delegation_ratio - 0.5).max(0.0) * 0.8; // 0.0 at 0.5, 0.4 at 1.0
        quality += delegation_quality;

        // Quality ranges from 0.0 (poor) to 1.0 (excellent)
        quality.min(1.0).max(0.7) // Minimum quality 0.7 to avoid over-penalizing
    }

    /// Adjust score for worker functions
    fn adjust_worker_score(&self, base_score: f64, is_pure: bool) -> ScoreAdjustment {
        if is_pure {
            let reduction_factor = 1.0 - self.config.pure_worker_reduction;
            let adjusted = base_score * reduction_factor;

            ScoreAdjustment {
                original_score: base_score,
                adjusted_score: adjusted,
                reduction_percent: self.config.pure_worker_reduction * 100.0,
                adjustment_reason: "Pure worker function".to_string(),
                confidence: 1.0,
            }
        } else {
            // No adjustment for impure workers
            ScoreAdjustment::no_adjustment(base_score)
        }
    }

    /// Adjust score for entry point functions
    fn adjust_entry_point_score(&self, base_score: f64, depth: u32) -> ScoreAdjustment {
        if depth >= 3 {
            let reduction_factor = 1.0 - self.config.entry_point_reduction;
            let adjusted = base_score * reduction_factor;

            ScoreAdjustment {
                original_score: base_score,
                adjusted_score: adjusted,
                reduction_percent: self.config.entry_point_reduction * 100.0,
                adjustment_reason: format!("Entry point with depth {}", depth),
                confidence: 0.8,
            }
        } else {
            ScoreAdjustment::no_adjustment(base_score)
        }
    }
}

impl ScoreAdjustment {
    fn no_adjustment(score: f64) -> Self {
        Self {
            original_score: score,
            adjusted_score: score,
            reduction_percent: 0.0,
            adjustment_reason: "No adjustment applied".to_string(),
            confidence: 1.0,
        }
    }
}
```

### Integration with Scoring Pipeline

```rust
// src/priority/scoring/computation.rs

use crate::priority::scoring::orchestration_adjustment::{OrchestrationAdjuster, ScoreAdjustment};

pub fn calculate_final_score_with_adjustments(
    function: &FunctionMetrics,
    call_graph: &CallGraph,
    config: &ScoringConfig,
) -> (f64, Option<ScoreAdjustment>) {
    // Step 1: Calculate base score (existing logic)
    let base_score = calculate_base_complexity_score(function);

    // Step 2: Apply entropy dampening (existing)
    let after_entropy = if let Some(entropy) = &function.entropy_score {
        apply_entropy_dampening(base_score as u32, entropy) as f64
    } else {
        base_score
    };

    // Step 3: NEW - Apply orchestration adjustment
    if let Some(role) = &function.function_role {
        if let Some(metrics) = &function.role_metrics {
            let adjuster = OrchestrationAdjuster::new(config.orchestration_adjustment.clone());
            let adjustment = adjuster.adjust_score(
                after_entropy,
                function.cyclomatic,
                role,
                metrics,
            );

            return (adjustment.adjusted_score, Some(adjustment));
        }
    }

    (after_entropy, None)
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

    #[test]
    fn test_high_confidence_orchestrator_max_reduction() {
        let config = OrchestrationAdjustmentConfig::default();
        let adjuster = OrchestrationAdjuster::new(config);

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

        let adjustment = adjuster.adjust_score(100.0, 17, &role, &metrics);

        // High confidence (0.9) + excellent quality should give ~30% reduction
        assert!(adjustment.reduction_percent >= 25.0 && adjustment.reduction_percent <= 30.0);
        assert!(adjustment.adjusted_score >= 70.0 && adjustment.adjusted_score <= 75.0);
    }

    #[test]
    fn test_minimum_complexity_floor() {
        let config = OrchestrationAdjustmentConfig::default();
        let adjuster = OrchestrationAdjuster::new(config);

        let role = FunctionRole::Orchestrator {
            coordinates: 10,
            confidence: 0.95,
        };

        let metrics = create_high_quality_metrics(10);

        // Even with high reduction, should not go below coordinates × 2
        let adjustment = adjuster.adjust_score(30.0, 15, &role, &metrics);
        assert!(adjustment.adjusted_score >= 20.0); // 10 × 2 = 20
    }

    #[test]
    fn test_composition_quality_calculation() {
        let adjuster = OrchestrationAdjuster::new(OrchestrationAdjustmentConfig::default());

        // Excellent quality: all pure, shallow depth, high delegation
        let excellent = RoleMetrics {
            delegation_ratio: 0.9,
            pure_callee_count: 10,
            callee_count: 10,
            avg_call_depth: 1,
            local_complexity: 2,
            caller_count: 2,
        };
        let quality = adjuster.calculate_composition_quality(&excellent);
        assert!(quality >= 0.9);

        // Poor quality: few pure, deep, low delegation
        let poor = RoleMetrics {
            delegation_ratio: 0.6,
            pure_callee_count: 2,
            callee_count: 10,
            avg_call_depth: 5,
            local_complexity: 8,
            caller_count: 2,
        };
        let quality = adjuster.calculate_composition_quality(&poor);
        assert!(quality >= 0.7 && quality < 0.8); // Never below 0.7
    }

    #[test]
    fn test_graduated_confidence_reductions() {
        let config = OrchestrationAdjustmentConfig::default();
        let adjuster = OrchestrationAdjuster::new(config);

        // High confidence: 0.8
        let high = adjuster.calculate_base_reduction(0.8);
        assert_eq!(high, 0.30);

        // Medium confidence: 0.6
        let medium = adjuster.calculate_base_reduction(0.6);
        assert!(medium > 0.10 && medium < 0.30);

        // Low confidence: 0.4
        let low = adjuster.calculate_base_reduction(0.4);
        assert!(low < 0.10);
    }

    #[test]
    fn test_pure_worker_reduction() {
        let config = OrchestrationAdjustmentConfig::default();
        let adjuster = OrchestrationAdjuster::new(config);

        let adjustment = adjuster.adjust_worker_score(100.0, true);
        assert_eq!(adjustment.reduction_percent, 10.0);
        assert_eq!(adjustment.adjusted_score, 90.0);

        let no_adjustment = adjuster.adjust_worker_score(100.0, false);
        assert_eq!(no_adjustment.reduction_percent, 0.0);
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
- Adjustment algorithm and formulas
- Configuration options and their effects
- Composition quality calculation
- Minimum complexity floor rationale

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
high_confidence_reduction = 0.30  # 30%
medium_confidence_reduction = 0.20  # 20%
low_confidence_reduction = 0.10  # 10%
pure_worker_reduction = 0.10  # 10%
entry_point_reduction = 0.15  # 15%
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

### Formula Derivation

The adjustment formula balances several competing concerns:

1. **Confidence**: Higher confidence → higher reduction
2. **Quality**: Better composition → higher reduction
3. **Floor**: Minimum complexity based on coordination count
4. **Conservatism**: Cap at 30% to avoid over-reduction

```rust
reduction = base_reduction(confidence) × composition_quality(metrics)
adjusted = max(base_score × (1 - reduction), coordinates × 2)
```

### Tuning Process

1. Analyze 50+ real-world Rust projects
2. Hand-label 500+ functions as orchestrator vs worker
3. Measure false positive rates at different thresholds
4. Tune for optimal precision/recall tradeoff
5. Validate on held-out test set

### Edge Cases

1. **Extreme Delegators** (100% delegation): Still apply floor to prevent zero scores
2. **Recursive Orchestrators**: Call depth calculation prevents infinite recursion
3. **Mixed Patterns**: Confidence scoring handles ambiguous cases
4. **Configuration Extremes**: Validate config values at load time

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
- **User Satisfaction**: ≥80% of users report improved recommendations
- **No Regressions**: Zero increase in false negatives (missing real debt)
