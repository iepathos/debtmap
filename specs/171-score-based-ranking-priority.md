---
number: 171
title: Score-Based Ranking Priority
category: optimization
priority: high
status: draft
dependencies: []
created: 2025-11-09
---

# Specification 171: Score-Based Ranking Priority

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: None

## Context

Debtmap currently uses a tier-based prioritization system that ranks items by tier first (T1 > T2 > T3 > T4), then by score within each tier. This was implemented to surface architectural issues above testing gaps.

However, this creates confusing output where lower-scored items appear higher in the ranking than higher-scored items:

```
#2 SCORE: 11.5 [ERROR] UNTESTED [CRITICAL]
#6 SCORE: 2.83 [ERROR] UNTESTED [LOW]
#7 SCORE: 8.97 [ERROR] UNTESTED [CRITICAL]
```

In this example:
- Item #6 (score 2.83) ranks higher than #7 (score 8.97)
- The tier assignments (T2 vs T3) are completely invisible to users
- The sophisticated scoring system is effectively ignored by the ranking

### Current Implementation

The sorting logic in `src/priority/unified_analysis_queries.rs:70-91`:

```rust
all_items.sort_by(|a, b| {
    // Get tier for comparison
    let tier_a = match a {
        DebtItem::Function(f) => f.tier.unwrap_or(RecommendationTier::T4Maintenance),
        DebtItem::File(_) => RecommendationTier::T1CriticalArchitecture,
    };
    let tier_b = match b {
        DebtItem::Function(f) => f.tier.unwrap_or(RecommendationTier::T4Maintenance),
        DebtItem::File(_) => RecommendationTier::T1CriticalArchitecture,
    };

    // Primary sort: by tier (lower enum value = higher priority)
    match tier_a.cmp(&tier_b) {
        std::cmp::Ordering::Equal => {
            // Secondary sort: by score within tier (higher score = higher priority)
            b.score()
                .partial_cmp(&a.score())
                .unwrap_or(std::cmp::Ordering::Equal)
        }
        other => other,
    }
});
```

### Problems

1. **Transparency**: Users cannot see why items are ranked the way they are
2. **Score Meaningfulness**: The sophisticated scoring system is undermined
3. **User Confusion**: Rankings appear arbitrary without tier context
4. **Trust Issues**: Users lose confidence in the prioritization algorithm
5. **Debugging Difficulty**: Hard to validate ranking decisions

## Objective

Replace tier-based ranking with pure score-based ranking using exponential scaling and risk-based boosting to naturally separate high-priority items from low-priority ones, ensuring transparent and understandable prioritization.

## Requirements

### Functional Requirements

1. **Score-Based Ranking**: Items must be sorted by score in descending order (highest first)
2. **Exponential Scaling**: High-priority debt types use exponential scaling to naturally amplify their scores
3. **Risk-Based Boosting**: Discrete boosts applied for measurable risk factors (dependencies, entry points, etc.)
4. **Transparent Scoring**: Any prioritization logic must be visible in the final score
5. **Backward Compatibility**: Existing base score calculation functions remain unchanged
6. **Configuration**: Users can adjust scaling exponents and boost factors through configuration

### Non-Functional Requirements

1. **Performance**: Sorting must remain O(n log n) with minimal overhead
2. **Clarity**: The ranking logic must be immediately understandable
3. **Predictability**: Higher scores must always rank higher, no exceptions
4. **Testability**: Ranking behavior must be easily tested and verified

## Acceptance Criteria

- [ ] Items are sorted strictly by final score (highest to lowest)
- [ ] Exponential scaling applied to architectural and complexity debt types
- [ ] Risk-based boosts applied for high dependencies, entry points, and other risk factors
- [ ] Higher-scored items never rank below lower-scored items
- [ ] Tier information is still available for filtering/grouping if needed
- [ ] All existing tests pass with updated ranking logic
- [ ] New tests verify strict score-based ordering
- [ ] Property tests verify no score inversions
- [ ] Performance benchmarks show no regression (target: within 5%)
- [ ] Documentation explains exponential scaling and risk boosting approach

## Technical Details

### Implementation Approach

**Phase 1: Remove Tier-Based Sorting**

Simplify the sorting logic in `src/priority/unified_analysis_queries.rs`:

```rust
// Old (tier-based)
all_items.sort_by(|a, b| {
    let tier_a = ...; // tier comparison first
    let tier_b = ...;
    match tier_a.cmp(&tier_b) {
        std::cmp::Ordering::Equal => b.score().partial_cmp(&a.score())...,
        other => other,
    }
});

// New (score-based)
all_items.sort_by(|a, b| {
    b.score()
        .partial_cmp(&a.score())
        .unwrap_or(std::cmp::Ordering::Equal)
});
```

**Phase 2: Implement Exponential Scaling + Risk Boosting**

Create new score transformation module in `src/priority/scoring/scaling.rs`:

```rust
use crate::priority::{DebtType, UnifiedDebtItem, FunctionRole};

/// Configuration for exponential scaling and risk boosting
#[derive(Debug, Clone)]
pub struct ScalingConfig {
    // Exponential scaling exponents
    pub god_object_exponent: f64,      // Default: 1.4
    pub god_module_exponent: f64,      // Default: 1.4
    pub high_complexity_exponent: f64, // Default: 1.2 (cyclomatic > 30)
    pub moderate_complexity_exponent: f64, // Default: 1.1 (cyclomatic > 15)

    // Risk boost factors
    pub high_dependency_boost: f64,    // Default: 1.2 (total deps > 15)
    pub entry_point_boost: f64,        // Default: 1.15
    pub complex_untested_boost: f64,   // Default: 1.25 (complexity > 20 + untested)
}

impl Default for ScalingConfig {
    fn default() -> Self {
        Self {
            god_object_exponent: 1.4,
            god_module_exponent: 1.4,
            high_complexity_exponent: 1.2,
            moderate_complexity_exponent: 1.1,
            high_dependency_boost: 1.2,
            entry_point_boost: 1.15,
            complex_untested_boost: 1.25,
        }
    }
}

/// Apply exponential scaling based on debt type
fn apply_exponential_scaling(
    base_score: f64,
    debt_type: &DebtType,
    config: &ScalingConfig,
) -> f64 {
    match debt_type {
        // Architectural issues get strong exponential scaling
        DebtType::GodObject { .. } => base_score.powf(config.god_object_exponent),
        DebtType::GodModule { .. } => base_score.powf(config.god_module_exponent),

        // High complexity gets moderate exponential scaling
        DebtType::ComplexityHotspot { cyclomatic, .. } if *cyclomatic > 30 => {
            base_score.powf(config.high_complexity_exponent)
        }
        DebtType::ComplexityHotspot { cyclomatic, .. } if *cyclomatic > 15 => {
            base_score.powf(config.moderate_complexity_exponent)
        }

        // Complex untested code gets slight exponential boost
        DebtType::TestingGap { cyclomatic, .. } if *cyclomatic > 20 => {
            base_score.powf(config.moderate_complexity_exponent)
        }

        // Everything else stays linear
        _ => base_score,
    }
}

/// Apply discrete risk-based boosts
fn apply_risk_boosts(
    score: f64,
    item: &UnifiedDebtItem,
    config: &ScalingConfig,
) -> f64 {
    let mut boost = 1.0;

    // High dependency count indicates central, critical code
    let total_deps = item.upstream_dependencies + item.downstream_dependencies;
    if total_deps > 15 {
        boost *= config.high_dependency_boost;
    }

    // Entry points are critical paths
    if matches!(item.function_role, FunctionRole::EntryPoint) {
        boost *= config.entry_point_boost;
    }

    // Complex + untested is particularly risky
    if is_untested(item) && item.cyclomatic_complexity > 20 {
        boost *= config.complex_untested_boost;
    }

    score * boost
}

/// Check if item is untested
fn is_untested(item: &UnifiedDebtItem) -> bool {
    matches!(
        item.debt_type,
        DebtType::TestingGap { coverage, .. } if coverage < 0.1
    )
}

/// Calculate final score with exponential scaling and risk boosting
pub fn calculate_final_score(
    base_score: f64,
    debt_type: &DebtType,
    item: &UnifiedDebtItem,
    config: &ScalingConfig,
) -> f64 {
    // Step 1: Apply exponential scaling based on debt type
    let scaled = apply_exponential_scaling(base_score, debt_type, config);

    // Step 2: Apply discrete risk boosts
    apply_risk_boosts(scaled, item, config)
}
```

**Phase 3: Integrate into Score Calculation**

Update the unified scorer to use the new scaling system:

```rust
// In src/priority/unified_scorer.rs or score_debt_item function
use crate::priority::scoring::scaling::{calculate_final_score, ScalingConfig};

let base_score = calculate_base_score(...);
let scaling_config = ScalingConfig::default(); // Or from user config
let final_score = calculate_final_score(base_score, &debt_type, &item, &scaling_config);
```

**Phase 4: Update Score Components**

Ensure `UnifiedScore` captures the scaling information:

```rust
pub struct UnifiedScore {
    pub complexity_factor: f64,
    pub coverage_factor: f64,
    pub dependency_factor: f64,
    pub role_multiplier: f64,
    pub base_score: f64,             // NEW: Score before scaling
    pub exponential_factor: f64,     // NEW: Exponent applied (1.0 = none)
    pub risk_boost: f64,             // NEW: Risk boost multiplier
    pub final_score: f64,            // base ^ exponential * risk_boost
    pub pre_adjustment_score: Option<f64>,
    pub adjustment_applied: Option<String>,
}
```

### Architecture Changes

**Before (Two-Stage Ranking)**:
```
Score Calculation → Tier Classification → Sort by (Tier, Score)
                                          ↑ Tier overrides score!
```

**After (Exponential + Risk Boosting)**:
```
Score Calculation → Exponential Scaling → Risk Boosting → Final Score → Sort by Score
                    (debt type)           (dependencies,
                                          entry points)
```

### Data Flow

1. Calculate base score from complexity, coverage, dependencies
2. Classify debt type (GodObject, ComplexityHotspot, TestingGap, etc.)
3. Apply exponential scaling based on debt type (1.0x to 1.4x exponent)
4. Apply risk boosts based on dependencies, entry points, etc. (1.0x to 1.5x multiplier)
5. Store final score: `base^exponent * risk_boost`
6. Sort all items by `final_score` descending

### Configuration

New `ScalingConfig` for exponential scaling and risk boosting:

```rust
pub struct ScalingConfig {
    // Exponential scaling exponents
    pub god_object_exponent: f64,      // Default: 1.4
    pub god_module_exponent: f64,      // Default: 1.4
    pub high_complexity_exponent: f64, // Default: 1.2 (cyclomatic > 30)
    pub moderate_complexity_exponent: f64, // Default: 1.1 (cyclomatic > 15)

    // Risk boost factors
    pub high_dependency_boost: f64,    // Default: 1.2 (total deps > 15)
    pub entry_point_boost: f64,        // Default: 1.15
    pub complex_untested_boost: f64,   // Default: 1.25 (complexity > 20 + untested)
}
```

These settings control how much high-priority items are boosted **through score amplification**, not sorting overrides.

### Example Comparison

**Current System (Tier-Based Ranking)**:
```
Item A: base=10, tier=T3 → Ranks 5th (T3 tier, even with high score)
Item B: base=5, tier=T2 → Ranks 2nd (T2 tier overrides score)
Result: Lower score ranks higher (confusing!)
```

**New System (Exponential + Risk Boosting)**:
```
Item A: base=10, GodObject → 10^1.4 = 25.1, high_deps → 25.1 * 1.2 = 30.1 → Ranks 1st
Item B: base=5, TestingGap → 5^1.0 = 5.0, entry_point → 5.0 * 1.15 = 5.75 → Ranks 3rd
Item C: base=12, SimpleIssue → 12^1.0 = 12.0, no_boost → 12.0 * 1.0 = 12.0 → Ranks 2nd
Result: Scores strictly ordered (30.1 > 12.0 > 5.75), transparent!
```

### Why Exponential Scaling Works Better

**Linear Multipliers (Tier Weights)**:
- God Object: 10 * 1.5 = 15
- Testing Gap: 10 * 0.7 = 7
- Separation: 15 - 7 = 8 (fixed gap)

**Exponential Scaling**:
- God Object: 10^1.4 ≈ 25.1
- Testing Gap: 10^1.0 = 10.0
- Separation: 25.1 - 10.0 = 15.1 (larger gap!)

At higher base scores:
- God Object: 50^1.4 ≈ 247
- Testing Gap: 50^1.0 = 50
- Separation: 197 (much larger!)

**Key insight**: Exponential scaling naturally creates more separation at higher scores, which is exactly what we want - serious architectural issues with high base scores get amplified significantly.

## Dependencies

### Prerequisites
None - this is a refactoring of existing functionality

### Affected Components
- `src/priority/unified_analysis_queries.rs` - Sorting logic (simplified)
- `src/priority/scoring/scaling.rs` - NEW: Exponential scaling and risk boosting
- `src/priority/scoring/computation.rs` - Integration point for scaling
- `src/priority/unified_scorer.rs` - UnifiedScore structure (new fields)
- `src/priority/tiers.rs` - Tier classification (kept for filtering/grouping only)

### External Dependencies
None

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exponential_scaling_god_object() {
        let config = ScalingConfig::default();

        // Low base score: 10^1.4 ≈ 25.1
        assert!((apply_exponential_scaling(10.0, &DebtType::GodObject {..}, &config) - 25.1).abs() < 0.1);

        // High base score: 50^1.4 ≈ 247
        assert!((apply_exponential_scaling(50.0, &DebtType::GodObject {..}, &config) - 247.0).abs() < 1.0);
    }

    #[test]
    fn test_exponential_scaling_creates_separation() {
        let config = ScalingConfig::default();

        // Same base score for different debt types
        let base = 20.0;

        let god_object_score = apply_exponential_scaling(base, &DebtType::GodObject {..}, &config);
        let testing_gap_score = apply_exponential_scaling(base, &DebtType::TestingGap {..}, &config);

        // God object should score significantly higher
        assert!(god_object_score > testing_gap_score * 1.5);
        // 20^1.4 ≈ 66 vs 20^1.0 = 20, ratio ~3.3x
    }

    #[test]
    fn test_risk_boosts_multiply() {
        let config = ScalingConfig::default();
        let mut item = create_test_item();
        item.upstream_dependencies = 10;  // High deps
        item.downstream_dependencies = 10;
        item.function_role = FunctionRole::EntryPoint;
        item.debt_type = DebtType::TestingGap { coverage: 0.0, cyclomatic: 25, cognitive: 30 };

        let boosted = apply_risk_boosts(10.0, &item, &config);

        // Should apply: high_deps (1.2) * entry_point (1.15) * complex_untested (1.25)
        // 10.0 * 1.2 * 1.15 * 1.25 ≈ 17.25
        assert!((boosted - 17.25).abs() < 0.1);
    }

    #[test]
    fn test_no_score_inversions() {
        // Property: For all items, if score_a > score_b, then rank_a < rank_b
        let items = generate_random_items(100);
        let ranked = get_top_mixed_priorities(items, 100);

        for i in 0..ranked.len()-1 {
            assert!(
                ranked[i].score() >= ranked[i+1].score(),
                "Score inversion at position {}: {} < {}",
                i, ranked[i].score(), ranked[i+1].score()
            );
        }
    }

    #[test]
    fn test_score_based_sorting_strict_ordering() {
        let items = vec![
            create_item(base: 10.0, debt_type: GodObject),      // 10^1.4 ≈ 25.1
            create_item(base: 15.0, debt_type: TestingGap),     // 15^1.0 = 15.0
            create_item(base: 8.0, debt_type: ComplexityHotspot), // 8^1.2 ≈ 10.6
        ];

        let ranked = get_top_mixed_priorities(items, 10);

        // Should be ordered by final score
        assert!(ranked[0].score() > 25.0);  // God object
        assert!(ranked[1].score() > 14.0);  // Testing gap
        assert!(ranked[2].score() > 10.0);  // Complexity hotspot
    }

    #[test]
    fn test_architectural_issues_surface_naturally() {
        // Even with slightly lower base scores, architectural issues should surface
        let god_object = create_item(base: 30.0, debt_type: GodObject);  // 30^1.4 ≈ 108
        let simple_gap = create_item(base: 50.0, debt_type: TestingGap); // 50^1.0 = 50

        assert!(god_object.final_score > simple_gap.final_score);
        // Exponential scaling ensures architectural issues rank higher
    }
}
```

### Integration Tests

```rust
#[test]
fn test_real_codebase_ranking_consistency() {
    // Run analysis on debtmap itself
    let results = analyze_project(".");
    let ranked = results.get_top_mixed_priorities(10);

    // Verify strict score ordering
    for i in 0..ranked.len()-1 {
        assert!(
            ranked[i].score() >= ranked[i+1].score(),
            "Score inversion in real codebase: rank {} ({}) > rank {} ({})",
            i, ranked[i].score(), i+1, ranked[i+1].score()
        );
    }

    // Verify architectural issues surface naturally through exponential scaling
    let god_objects: Vec<_> = ranked.iter()
        .filter(|i| matches!(i.debt_type, DebtType::GodObject {..}))
        .collect();

    let simple_gaps: Vec<_> = ranked.iter()
        .filter(|i| matches!(i.debt_type, DebtType::TestingGap { cyclomatic, .. } if cyclomatic < 10))
        .collect();

    // God objects should generally rank higher due to exponential scaling
    if !god_objects.is_empty() && !simple_gaps.is_empty() {
        let avg_god_rank = god_objects.iter().map(|i| ranked.iter().position(|r| r == *i).unwrap()).sum::<usize>() / god_objects.len();
        let avg_gap_rank = simple_gaps.iter().map(|i| ranked.iter().position(|r| r == *i).unwrap()).sum::<usize>() / simple_gaps.len();
        assert!(avg_god_rank < avg_gap_rank, "God objects should rank higher on average");
    }
}

#[test]
fn test_score_components_transparency() {
    let results = analyze_project(".");
    let item = &results.items[0];

    // Verify score components are accessible
    assert!(item.unified_score.base_score > 0.0);
    assert!(item.unified_score.exponential_factor >= 1.0);
    assert!(item.unified_score.risk_boost >= 1.0);

    // Verify final score calculation is transparent
    let expected_final = item.unified_score.base_score.powf(item.unified_score.exponential_factor)
                        * item.unified_score.risk_boost;
    assert!((item.unified_score.final_score - expected_final).abs() < 0.01);
}
```

### Performance Tests

```rust
#[bench]
fn bench_score_based_sorting(b: &mut Bencher) {
    let items = generate_items(1000);
    b.iter(|| {
        let mut items_copy = items.clone();
        items_copy.sort_by(|a, b| {
            b.score().partial_cmp(&a.score()).unwrap_or(Ordering::Equal)
        });
    });
}
```

### User Acceptance

1. Run `debtmap analyze .` on debtmap repository
2. Verify ranking makes intuitive sense
3. Confirm higher scores rank higher
4. Validate that architectural issues still surface near the top
5. Check that output is more understandable

## Documentation Requirements

### Code Documentation

1. Document the tier weight application in `UnifiedScore`
2. Add rustdoc examples showing tier weight calculation
3. Comment the simplified sorting logic
4. Explain tier weight configuration in `TierConfig`

### User Documentation

Update `book/src/` documentation:

1. **Prioritization Explanation**:
   ```markdown
   ## How Debtmap Ranks Technical Debt

   Debtmap ranks items strictly by score (highest first). The score uses a two-stage amplification:

   ### Stage 1: Exponential Scaling (by debt type)
   High-priority debt types get exponential amplification:
   - **God Objects/Modules**: score^1.4 (strong amplification)
   - **High Complexity** (cyclomatic > 30): score^1.2
   - **Moderate Complexity** (cyclomatic > 15): score^1.1
   - **Simple Issues**: score^1.0 (no amplification)

   ### Stage 2: Risk Boosting (by context)
   Additional multipliers based on risk factors:
   - **High Dependencies** (>15 total): ×1.2
   - **Entry Points**: ×1.15
   - **Complex Untested** (complexity > 20, uncovered): ×1.25

   ### Example
   ```
   God Object: base=30 → 30^1.4 ≈ 108, high_deps → 108 × 1.2 = 129.6
   Simple Gap: base=30 → 30^1.0 = 30, no boosts → 30 × 1.0 = 30.0
   ```

   This ensures architectural issues naturally rank higher while maintaining
   transparent, predictable score-based ranking.
   ```

2. **Configuration Guide**:
   ```markdown
   ## Customizing Score Scaling

   Adjust exponential scaling and risk boosts in your config:

   ```toml
   [scaling]
   # Exponential scaling exponents
   god_object_exponent = 1.5        # Default: 1.4
   high_complexity_exponent = 1.3   # Default: 1.2

   # Risk boost multipliers
   high_dependency_boost = 1.3      # Default: 1.2
   entry_point_boost = 1.2          # Default: 1.15
   complex_untested_boost = 1.4     # Default: 1.25
   ```

   **Note**: Higher exponents create more separation. An exponent of 1.0 means
   linear scaling (no boost). Exponents above 2.0 can create extreme scores.
   ```

### Architecture Updates

Update `ARCHITECTURE.md`:

```markdown
## Prioritization System

Debtmap uses **pure score-based ranking** with exponential scaling and risk boosting:

### Score Calculation Pipeline
1. **Base Score**: Calculate from complexity, coverage, dependencies
2. **Exponential Scaling**: Apply debt-type-specific exponent (1.0-1.4)
3. **Risk Boosting**: Apply context-based multipliers (1.0-1.5x)
4. **Final Score**: `base^exponent × risk_boost`
5. **Ranking**: Sort by final score (highest first)

### Why Exponential Scaling?
Linear multipliers create fixed gaps, regardless of base score:
- God Object: 10 × 1.5 = 15 vs Testing Gap: 10 × 0.7 = 7 (gap: 8)
- God Object: 50 × 1.5 = 75 vs Testing Gap: 50 × 0.7 = 35 (gap: 40)

Exponential scaling creates proportional separation:
- God Object: 10^1.4 ≈ 25 vs Testing Gap: 10^1.0 = 10 (ratio: 2.5x)
- God Object: 50^1.4 ≈ 247 vs Testing Gap: 50^1.0 = 50 (ratio: 4.9x)

High-severity issues naturally amplify more as their base scores increase.

### Implementation
- **Module**: `src/priority/scoring/scaling.rs`
- **Config**: `ScalingConfig` with tunable exponents and boosts
- **Transparency**: All score components tracked in `UnifiedScore`
```

## Implementation Notes

### Migration Path

1. **Create scaling module**: `src/priority/scoring/scaling.rs` with exponential/risk functions
2. **Update UnifiedScore**: Add `base_score`, `exponential_factor`, `risk_boost` fields
3. **Integrate scaling**: Modify score calculation to use `calculate_final_score()`
4. **Simplify sorting**: Remove tier-based comparison in `unified_analysis_queries.rs`
5. **Update tests**: Verify score ordering and exponential behavior
6. **Run benchmarks**: Ensure performance within 5% of current
7. **Update docs**: Explain exponential scaling and risk boosting
8. **Test on real codebase**: Validate rankings make intuitive sense

### Edge Cases

1. **Equal scores**: Use stable sort to preserve discovery order
2. **NaN scores**: Handle gracefully with `unwrap_or(Ordering::Equal)`
3. **Zero base score**: Apply minimum base score of 1.0 to avoid zero^exponent = 0
4. **Very high scores**: Cap exponential scaling at reasonable limits (e.g., 10000) to prevent overflow
5. **Negative scores**: Should not occur, but clamp to minimum of 1.0 if encountered

### Performance Considerations

- **Sorting**: Remains O(n log n), simpler comparison than tier-based
- **Exponential scaling**: `powf()` is O(1) per item, ~20-30 CPU cycles
- **Risk boosting**: Simple multiplication, O(1) per item
- **Memory**: Three additional f64 fields per item (~24 bytes)
- **Overall**: Expected to be within 5% of current performance, possibly faster due to simpler sort

### Backward Compatibility

- **Base score calculation**: Unchanged
- **Tier classification**: Kept for filtering/grouping, not for ranking
- **New fields**: `base_score`, `exponential_factor`, `risk_boost` added to `UnifiedScore`
- **JSON output**: Includes new score components for transparency
- **Breaking change**: Ranking order will change significantly for some items

## Migration and Compatibility

### Breaking Changes

**Ranking Order**: Items will be ranked differently. Users expecting tier-based sorting will see changes.

**Mitigation**:
- Document change in release notes
- Explain rationale (transparency, score meaningfulness)
- Provide tier weight configuration for customization
- Show before/after examples

### Rollback Plan

If needed, tier-based sorting can be restored via configuration flag:

```rust
pub struct RankingConfig {
    pub use_tier_based_sorting: bool,  // Default: false (score-based)
}
```

However, **score-based sorting is the recommended default** for transparency.

### Testing Migration

1. **Baseline**: Capture current rankings for debtmap repository
2. **Apply change**: Implement score-based sorting with tier weights
3. **Compare**: Analyze ranking differences
4. **Validate**: Ensure changes make intuitive sense
5. **Adjust weights**: Tune tier weights if needed to preserve architectural emphasis

### Communication

**Release Notes**:
```markdown
## Breaking Change: Exponential Score-Based Ranking

Debtmap now ranks items strictly by score using exponential scaling and risk boosting.

**Why**: The previous tier-based ranking was confusing - items with score 2.83 could
rank higher than items with score 8.97 due to invisible tier assignments that overrode
the score.

**What Changed**:
- Ranking is now **always** by score (highest first)
- High-priority debt types get exponential amplification (e.g., score^1.4 for God Objects)
- Risk factors (dependencies, entry points) apply additional boosts
- All scoring is transparent and visible in the output

**Impact**: Rankings will change significantly, but will now be predictable and explainable.

**Examples**:
```
Before (tier-based):
  #6: handle_analyze_command (score 2.83, T2) ranks above
  #7: format_module_structure (score 8.97, T3) ← confusing!

After (exponential + risk):
  #3: format_module_structure (base=8.97, exp=1.1, final≈10.4)
  #6: handle_analyze_command (base=2.83, boost=1.15, final≈3.25) ✓
```

**Customization**: Adjust exponents and boosts in `[scaling]` config section.
```

## Success Metrics

- [ ] 100% of ranking decisions explainable by score alone
- [ ] Zero score inversions (higher score = higher rank)
- [ ] User confusion reports decrease
- [ ] Tier influence still visible in top 10 (architectural issues prominent)
- [ ] Performance within 5% of current implementation
- [ ] Test coverage maintains 85%+

## Future Enhancements

1. **Score Breakdown UI**: Show score components in output
2. **Customizable Weights**: Per-project tier weight profiles
3. **Interactive Tuning**: CLI tool to experiment with weights
4. **Score Visualization**: Graph showing score distribution
5. **Explanation Mode**: `--explain-ranking` flag to show tier weight application

## References

- Current implementation: `src/priority/unified_analysis_queries.rs:70-91`
- Tier classification: `src/priority/tiers.rs`
- Score calculation: `src/priority/scoring/computation.rs`
- Tier config: `src/priority/tiers.rs:60-128`
