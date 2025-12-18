---
number: 261
title: Remove Score Clamping
category: optimization
priority: medium
status: draft
dependencies: []
created: 2025-12-18
---

# Specification 261: Remove Score Clamping

**Category**: optimization
**Priority**: medium
**Status**: draft
**Dependencies**: None

## Context

Debtmap currently enforces a hard clamp of scores to the 0-100 range via the `Score0To100` type. This clamping happens automatically in `Score0To100::new()` and `normalize_final_score()`, preventing scores from exceeding 100 regardless of how severe the technical debt actually is.

This clamping destroys critical prioritization information at the top of the list:
- A function with raw score 250 and one with 150 both appear as 100
- The relative severity difference is lost
- Items that should be clearly distinguished appear identical

Scores are inherently relative - they represent comparative priority, not absolute values on a fixed scale. Removing clamping allows:
1. True relative severity to be visible
2. Easier balancing and calibration of scoring formulas
3. Clear distinction between "bad" and "catastrophically bad" debt

## Objective

Remove the upper bound clamping from score calculations, allowing scores to naturally reflect actual debt severity without artificial limits.

## Requirements

### Functional Requirements

1. **Remove upper bound clamping**
   - `Score0To100::new()` should not clamp to 100
   - `normalize_final_score()` should not clamp to 100
   - Scores can naturally exceed 100 based on debt severity

2. **Keep lower bound at 0**
   - Negative scores should still be clamped to 0
   - Zero represents "no debt"

3. **Maintain relative ordering**
   - Higher scores = higher priority
   - Sorting behavior unchanged

### Non-Functional Requirements

- Minimal code changes
- No performance impact
- Clear documentation update

## Acceptance Criteria

- [ ] Scores can exceed 100 in output
- [ ] Scores are still clamped at 0 (no negative scores)
- [ ] Sorting by score works correctly with values > 100
- [ ] TUI displays scores > 100 correctly
- [ ] JSON output contains unclamped scores
- [ ] Tests updated to reflect new behavior

## Technical Details

### Implementation Approach

#### 1. Modify Score0To100 Type

In `src/priority/score_types.rs`:

```rust
impl Score0To100 {
    /// Create a new score, clamping negative values to 0.
    /// No upper bound - scores can exceed 100 for severe debt.
    pub fn new(value: f64) -> Self {
        Self(value.max(0.0))
    }
}
```

Note: The type name `Score0To100` becomes a misnomer, but renaming would require touching 60+ files. The type can be renamed in a future refactor if desired.

#### 2. Modify normalize_final_score()

In `src/priority/scoring/calculation.rs`:

```rust
/// Normalize final score (no upper clamping, only floor at 0)
pub fn normalize_final_score(raw_score: f64) -> f64 {
    raw_score.max(0.0)
}
```

#### 3. Update Tests

Tests that assert scores are <= 100 need updating:
- Property tests asserting `score <= 100.0`
- Unit tests checking clamping behavior

## Dependencies

- **Prerequisites**: None
- **Affected Components**:
  - `src/priority/score_types.rs` - Score type definition
  - `src/priority/scoring/calculation.rs` - Normalization function
  - Various test files

## Testing Strategy

### Unit Tests

```rust
#[test]
fn score_allows_values_above_100() {
    let score = Score0To100::new(150.0);
    assert_eq!(score.value(), 150.0);
}

#[test]
fn score_still_clamps_negative_to_zero() {
    let score = Score0To100::new(-10.0);
    assert_eq!(score.value(), 0.0);
}

#[test]
fn normalize_preserves_high_scores() {
    assert_eq!(normalize_final_score(250.0), 250.0);
    assert_eq!(normalize_final_score(-5.0), 0.0);
}
```

## Documentation Requirements

- **Code Documentation**: Update docstrings to reflect no upper clamping
- **User Documentation**: Note that scores represent relative priority, not percentages

## Implementation Notes

### Type Naming

`Score0To100` is now a misnomer since values can exceed 100. Options:
1. Keep the name (minimal change, document the caveat)
2. Rename to `DebtScore` or `PriorityScore` (extensive refactor)

Recommended: Keep the name for now, add a doc comment explaining it's historical.

### Display Considerations

- Severity tiers (Critical/High/Medium/Low) still work based on score ranges
- TUI progress bars may need adjustment if they assume 0-100 scale
- Consider updating tier thresholds if needed

## Migration and Compatibility

### Breaking Changes

- Scores in output may exceed 100
- Scripts that validate `score <= 100` will fail

### Migration Path

No migration needed. Output format unchanged, just values differ.
