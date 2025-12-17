---
number: 260
title: Score Calculation Transparency
category: optimization
priority: medium
status: draft
dependencies: [171, 110, 191]
created: 2025-12-17
---

# Specification 260: Score Calculation Transparency

**Category**: optimization
**Priority**: medium
**Status**: draft
**Dependencies**: spec 171 (exponential scaling), spec 110 (orchestration adjustment), spec 191 (context dampening)

## Context

The current score breakdown display shows a gap between the formula-calculated value and the stored `base_score` that is labeled as "debt patterns, normalization" without explaining what specific adjustments were applied. For example:

```
calculation steps
  1. weighted base          30.50
  2. × role                 30.50 × 1.30 = 39.65
  3. × struct               39.65 × 1.30 = 51.55
  4. + adjustments          51.55 → 100.00 (debt patterns, normalization)
  final (clamped)           100.0
```

Users cannot understand why there's a ~48 point gap between 51.55 and 100.00. The actual score calculation pipeline includes several adjustments that aren't currently tracked:

1. **Debt aggregator adjustment** - Adds points for testing, resource, and duplication patterns
2. **Normalization** - Clamping to 0-100 range
3. **Orchestration adjustment** (spec 110) - Reduces score for clean orchestrators
4. **Context multiplier** (spec 191) - Dampens score for example/test/benchmark files

While specs 110 and 191 store their adjustments in `UnifiedScore`, the debt aggregator adjustment is calculated but never stored, making it impossible to explain in the display.

## Objective

Provide complete transparency into score calculation by tracking all intermediate values and adjustments in `UnifiedScore`, enabling the TUI and other outputs to show exactly what factors contributed to the final score.

## Requirements

### Functional Requirements

1. **Track debt aggregator adjustment**
   - Store the additive debt adjustment value in `UnifiedScore`
   - Track breakdown of debt components (testing, resource, duplication)
   - Enable display to show: "debt adjustment: +12.5 (testing: +5.0, resource: +4.5, duplication: +3.0)"

2. **Track pre-normalization score**
   - Store the score before `normalize_final_score()` is called
   - Enable display to show clamping when it occurs: "clamped: 115.2 → 100.0"

3. **Track structural multiplier details**
   - Store the structural quality multiplier value
   - Currently calculated but not stored in `UnifiedScore`

4. **Complete calculation audit trail**
   - All intermediate values should be traceable from raw inputs to final score
   - Display should be able to reconstruct the exact calculation pipeline

### Non-Functional Requirements

- Minimal memory impact (only store values when they differ from defaults)
- Backward compatible JSON serialization (use `skip_serializing_if`)
- No performance regression in scoring calculation

## Acceptance Criteria

- [ ] `UnifiedScore` has new optional field `debt_adjustment: Option<DebtAdjustmentDetails>`
- [ ] `DebtAdjustmentDetails` struct captures testing, resource, and duplication components
- [ ] `UnifiedScore` has new optional field `pre_normalization_score: Option<f64>`
- [ ] `UnifiedScore` has new optional field `structural_multiplier: Option<f64>`
- [ ] Score calculation pipeline populates all new fields
- [ ] TUI score breakdown shows each adjustment step with actual values
- [ ] When clamping occurs, display shows original value and "clamped to 100"
- [ ] JSON output includes all score tracking fields for debugging
- [ ] All existing tests continue to pass
- [ ] New unit tests validate score tracking completeness

## Technical Details

### Implementation Approach

#### 1. New Data Structures

```rust
/// Detailed breakdown of debt aggregator adjustments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebtAdjustmentDetails {
    /// Total additive adjustment
    pub total: f64,
    /// Testing debt component (testing_score / 50.0)
    pub testing: f64,
    /// Resource debt component (resource_score / 50.0)
    pub resource: f64,
    /// Duplication debt component (duplication_score / 50.0)
    pub duplication: f64,
}
```

#### 2. UnifiedScore Extensions

Add to `UnifiedScore`:

```rust
/// Debt aggregator adjustment details (spec 260)
#[serde(skip_serializing_if = "Option::is_none")]
pub debt_adjustment: Option<DebtAdjustmentDetails>,

/// Score before normalization/clamping was applied (spec 260)
#[serde(skip_serializing_if = "Option::is_none")]
pub pre_normalization_score: Option<f64>,

/// Structural quality multiplier applied (spec 260)
#[serde(skip_serializing_if = "Option::is_none")]
pub structural_multiplier: Option<f64>,
```

#### 3. Calculation Pipeline Changes

In `calculate_unified_priority_with_role()`:

```rust
// Calculate debt adjustment with tracking
let (debt_adjustment, debt_details) = calculate_debt_adjustment_with_details(func, debt_aggregator);
let debt_adjusted_score = structure_adjusted_score + debt_adjustment;

// Track pre-normalization score
let pre_normalization = debt_adjusted_score;
let normalized_score = normalize_final_score(debt_adjusted_score);

// Store in UnifiedScore
UnifiedScore {
    // ... existing fields ...
    debt_adjustment: Some(debt_details),
    pre_normalization_score: if (pre_normalization - normalized_score).abs() > 0.1 {
        Some(pre_normalization)
    } else {
        None
    },
    structural_multiplier: Some(structural_multiplier),
}
```

#### 4. Display Updates

In `score_breakdown.rs`, replace the vague "adjustments" step:

```rust
// Step 4: Show debt adjustment if applied
if let Some(debt) = &item.unified_score.debt_adjustment {
    if debt.total.abs() > 0.01 {
        let after_debt = current_value + debt.total;
        add_label_value(
            &mut lines,
            &format!("{}. + debt", step_num),
            format!(
                "{:.2} + {:.2} = {:.2} (test:{:.1}, res:{:.1}, dup:{:.1})",
                current_value, debt.total, after_debt,
                debt.testing, debt.resource, debt.duplication
            ),
            theme,
            width,
        );
        current_value = after_debt;
        step_num += 1;
    }
}

// Step N: Show clamping if applied
if let Some(pre_norm) = item.unified_score.pre_normalization_score {
    if pre_norm > 100.0 {
        add_label_value(
            &mut lines,
            &format!("{}. clamped", step_num),
            format!("{:.2} → 100.00 (max score)", pre_norm),
            theme,
            width,
        );
        step_num += 1;
    }
}
```

### Architecture Changes

- `UnifiedScore` struct gains 3 new optional fields
- `calculate_debt_adjustment()` becomes `calculate_debt_adjustment_with_details()` returning both value and breakdown
- Score breakdown display logic becomes data-driven from stored values

### APIs and Interfaces

No external API changes. Internal scoring API changes are backward compatible through optional fields.

## Dependencies

- **Prerequisites**:
  - Spec 171 (exponential scaling) - already implements `base_score` tracking pattern
  - Spec 110 (orchestration adjustment) - already implements `adjustment_applied` pattern
  - Spec 191 (context dampening) - already implements `context_multiplier` tracking

- **Affected Components**:
  - `src/priority/unified_scorer.rs` - score calculation
  - `src/priority/scoring/calculation.rs` - debt adjustment calculation
  - `src/tui/results/detail_pages/score_breakdown.rs` - display logic
  - `src/io/writers/` - JSON/markdown output formatters

- **External Dependencies**: None

## Testing Strategy

- **Unit Tests**:
  - Test `DebtAdjustmentDetails` correctly captures component breakdown
  - Test `pre_normalization_score` is populated only when clamping occurs
  - Test `structural_multiplier` is correctly stored
  - Test backward compatibility with existing tests

- **Integration Tests**:
  - Verify complete audit trail from raw metrics to final score
  - Test TUI display shows all adjustment steps
  - Test JSON output includes all tracking fields

- **Regression Tests**:
  - All existing scoring tests must continue to pass
  - Score values should not change (only tracking added)

## Documentation Requirements

- **Code Documentation**: Docstrings for new fields and functions
- **User Documentation**: Update score breakdown explanation in README
- **Architecture Updates**: Document complete scoring pipeline in ARCHITECTURE.md

## Implementation Notes

### Backward Compatibility

All new fields use `Option<T>` with `#[serde(skip_serializing_if = "Option::is_none")]`:
- Old code reading new output will ignore unknown fields
- New code reading old output will see `None` for new fields
- No migration required

### Memory Optimization

Only store values when they provide information:
- `debt_adjustment`: Only when `total > 0.01`
- `pre_normalization_score`: Only when clamping actually occurred
- `structural_multiplier`: Always store (small overhead, high value)

### Display Calculation Accuracy

The display should use stored values, not recalculate:
- Current approach: Recalculate from formula, show gap
- New approach: Use stored intermediate values directly
- Benefit: Display exactly matches what calculation did

## Migration and Compatibility

- No breaking changes
- No migration required
- New fields default to `None` for items created by older code
- Existing JSON output remains valid

## Future Considerations

This spec establishes a pattern for score audit trails. Future specs could:
- Add score history tracking for trend analysis
- Enable "explain this score" interactive feature
- Provide score comparison between analysis runs
- Support score debugging mode with full trace output
