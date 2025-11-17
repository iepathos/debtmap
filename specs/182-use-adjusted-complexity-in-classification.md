---
number: 182
title: Use Adjusted Complexity in Debt Classification
category: foundation
priority: high
status: draft
dependencies: []
created: 2025-11-16
---

# Specification 182: Use Adjusted Complexity in Debt Classification

**Category**: foundation
**Priority**: high
**Status**: draft
**Dependencies**: None

## Context

Debtmap calculates entropy-adjusted complexity scores to dampen cyclomatic complexity for functions with low entropy (predictable, repetitive structure). The `FunctionMetrics.adjusted_complexity` field stores this dampened value, which better reflects the actual cognitive load of understanding the function.

**Current Problem**: The debt classification logic in `src/priority/scoring/classification.rs` ignores `adjusted_complexity` and uses raw `cyclomatic` complexity for:
1. Threshold comparison (determining if function is a complexity hotspot)
2. Storing complexity values in `DebtType::ComplexityHotspot`

**Example from `state_reconciliation.rs:81` (`reconcile_state` function)**:
- Raw cyclomatic complexity: 9
- Entropy-adjusted complexity: 4.15 (dampened by factor 0.51 due to low entropy 0.28)
- Current behavior: Flagged as complexity hotspot because `cognitive=16 > 15`
- Expected behavior: Should NOT be flagged because adjusted complexity (4.15) < threshold (10)

This causes false positives for functions with repetitive, predictable structure that are actually easier to understand than their raw cyclomatic complexity suggests.

## Objective

Modify debt classification logic to use entropy-adjusted complexity scores when available, ensuring that functions with low entropy are not incorrectly flagged as complexity hotspots.

## Requirements

### Functional Requirements

1. **Threshold Comparison Using Adjusted Complexity**
   - `check_complexity_hotspot()` must use `func.adjusted_complexity` if available
   - Fall back to `func.cyclomatic` if `adjusted_complexity` is None
   - Apply same threshold logic (cyclomatic > 10 || cognitive > 15)

2. **Store Adjusted Complexity in DebtType**
   - When creating `DebtType::ComplexityHotspot`, store adjusted complexity
   - Preserve both raw and adjusted values for transparency
   - Update `DebtType::ComplexityHotspot` enum to include adjusted complexity field

3. **Backward Compatibility**
   - Functions without entropy analysis should use raw cyclomatic (None case)
   - Existing debt items should continue to work
   - No changes to external API or serialization format (if applicable)

### Non-Functional Requirements

- **Performance**: No measurable performance impact (< 1% overhead)
- **Correctness**: All existing tests must pass
- **Transparency**: Adjusted complexity usage must be visible in debug output
- **Maintainability**: Code changes must be minimal and localized

## Acceptance Criteria

- [ ] `check_complexity_hotspot()` uses `adjusted_complexity` when available
- [ ] `DebtType::ComplexityHotspot` includes both raw and adjusted complexity
- [ ] Functions with low entropy (< 0.35) and adjusted complexity < 10 are NOT flagged
- [ ] Functions without entropy analysis continue to use raw cyclomatic
- [ ] All existing tests pass without modification
- [ ] New test validates adjusted complexity threshold logic
- [ ] Documentation updated to explain adjusted complexity usage

## Technical Details

### Implementation Approach

#### 1. Update `DebtType::ComplexityHotspot` Enum

**File**: `src/core/mod.rs` or `src/priority/mod.rs` (wherever `DebtType` is defined)

```rust
pub enum DebtType {
    // ... other variants ...

    ComplexityHotspot {
        cyclomatic: u32,           // Raw cyclomatic complexity
        cognitive: u32,            // Cognitive complexity
        adjusted_cyclomatic: Option<u32>, // Entropy-adjusted cyclomatic (spec 182)
    },

    // ... other variants ...
}
```

**Rationale**: Storing both raw and adjusted values provides transparency and allows recommendations to show both metrics.

#### 2. Modify `check_complexity_hotspot()` Function

**File**: `src/priority/scoring/classification.rs:56-61`

**Before**:
```rust
fn check_complexity_hotspot(func: &FunctionMetrics) -> Option<DebtType> {
    (func.cyclomatic > 10 || func.cognitive > 15).then_some(DebtType::ComplexityHotspot {
        cyclomatic: func.cyclomatic,
        cognitive: func.cognitive,
    })
}
```

**After**:
```rust
fn check_complexity_hotspot(func: &FunctionMetrics) -> Option<DebtType> {
    // Use adjusted complexity if available (spec 182)
    let effective_cyclomatic = func.adjusted_complexity
        .map(|adj| adj.round() as u32)
        .unwrap_or(func.cyclomatic);

    (effective_cyclomatic > 10 || func.cognitive > 15).then_some(DebtType::ComplexityHotspot {
        cyclomatic: func.cyclomatic,
        cognitive: func.cognitive,
        adjusted_cyclomatic: func.adjusted_complexity.map(|adj| adj.round() as u32),
    })
}
```

**Key Changes**:
1. Calculate `effective_cyclomatic` using adjusted complexity when available
2. Use `effective_cyclomatic` in threshold comparison
3. Store both raw and adjusted values in `DebtType::ComplexityHotspot`

#### 3. Update All Pattern Matches on `DebtType::ComplexityHotspot`

**Files to Update**: All files that pattern match on `DebtType::ComplexityHotspot`

Search pattern: `DebtType::ComplexityHotspot \{`

Update pattern matches to handle new `adjusted_cyclomatic` field:

```rust
// Old pattern
DebtType::ComplexityHotspot { cyclomatic, cognitive } => { ... }

// New pattern (use adjusted if available)
DebtType::ComplexityHotspot { cyclomatic, cognitive, adjusted_cyclomatic } => {
    let effective_cyclomatic = adjusted_cyclomatic.unwrap_or(*cyclomatic);
    // Use effective_cyclomatic in calculations...
}
```

**Affected Files** (approximate, needs verification):
- `src/priority/scoring/concise_recommendation.rs`
- `src/priority/scoring/computation.rs`
- `src/priority/scoring/scaling.rs`
- `src/priority/scoring/debt_item.rs`
- `src/priority/formatter*.rs`
- `src/output/*.rs`
- Various test files

#### 4. Handle Backward Compatibility

**Serialization**: If `DebtType` is serialized (e.g., for caching or API responses), ensure:
- `adjusted_cyclomatic: None` for old data
- Serde defaults handle missing field gracefully

```rust
#[derive(Serialize, Deserialize)]
pub enum DebtType {
    ComplexityHotspot {
        cyclomatic: u32,
        cognitive: u32,
        #[serde(default)]  // Handle missing field in old data
        adjusted_cyclomatic: Option<u32>,
    },
}
```

### Architecture Changes

**Modified Modules**:
1. `src/core/mod.rs` or `src/priority/mod.rs` - Update `DebtType` enum
2. `src/priority/scoring/classification.rs` - Update `check_complexity_hotspot()`
3. All modules pattern matching `DebtType::ComplexityHotspot` - Update patterns

**Data Flow**:
```
FunctionMetrics (with adjusted_complexity)
    ↓
check_complexity_hotspot()
    ↓ (uses adjusted_complexity if available)
DebtType::ComplexityHotspot { cyclomatic, cognitive, adjusted_cyclomatic }
    ↓
Scoring/Recommendation (uses adjusted_cyclomatic)
    ↓
Output (displays both raw and adjusted)
```

### Example Behavior Change

**Input**:
```rust
FunctionMetrics {
    name: "reconcile_state",
    cyclomatic: 9,
    cognitive: 16,
    adjusted_complexity: Some(4.15),
    entropy_score: Some(EntropyScore {
        token_entropy: 0.28,
        dampening_factor: 0.51,
        adjusted_complexity: 4,
    }),
    // ... other fields
}
```

**Before Spec 182**:
```rust
// check_complexity_hotspot() compares: 9 > 10 || 16 > 15 → true
DebtType::ComplexityHotspot {
    cyclomatic: 9,
    cognitive: 16,
}
// Result: Flagged as complexity hotspot (FALSE POSITIVE)
```

**After Spec 182**:
```rust
// check_complexity_hotspot() compares: 4 > 10 || 16 > 15 → true (cognitive still high)
DebtType::ComplexityHotspot {
    cyclomatic: 9,
    cognitive: 16,
    adjusted_cyclomatic: Some(4),
}
// Result: Still flagged due to high cognitive complexity, but adjusted complexity stored
```

**Note**: In this specific case, the function is still flagged because `cognitive=16 > 15`. This is correct behavior - cognitive complexity is not entropy-adjusted, so high cognitive complexity still indicates a hotspot. The key improvement is that downstream recommendations will use `adjusted_cyclomatic=4` instead of `cyclomatic=9`.

## Dependencies

**Prerequisites**: None (uses existing `FunctionMetrics.adjusted_complexity` field)

**Affected Components**:
- `DebtType` enum definition
- `check_complexity_hotspot()` function
- All pattern matches on `DebtType::ComplexityHotspot`
- Recommendation generation (see spec 183)

**Related Specifications**:
- **Spec 183**: Use adjusted complexity in recommendations (depends on this spec)
- **Spec 178**: Fix moderate complexity recommendation logic (related)
- **Spec 177**: Role-aware complexity recommendations (related)

## Testing Strategy

### Unit Tests

**File**: `src/priority/scoring/classification.rs`

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_complexity_hotspot_uses_adjusted_complexity() {
        // Function with low entropy, adjusted complexity below threshold
        let func = FunctionMetrics {
            name: "test_func".to_string(),
            cyclomatic: 9,  // Above threshold (raw)
            cognitive: 12,  // Below threshold
            adjusted_complexity: Some(4.15),  // Below threshold (adjusted)
            // ... other fields with test defaults
        };

        let result = check_complexity_hotspot(&func);
        assert!(result.is_none(), "Should NOT flag - adjusted complexity is below threshold");
    }

    #[test]
    fn check_complexity_hotspot_falls_back_to_raw_when_no_adjustment() {
        // Function without entropy analysis
        let func = FunctionMetrics {
            name: "test_func".to_string(),
            cyclomatic: 11,  // Above threshold
            cognitive: 12,
            adjusted_complexity: None,  // No adjustment available
            // ... other fields
        };

        let result = check_complexity_hotspot(&func);
        assert!(result.is_some(), "Should flag - raw cyclomatic above threshold");
    }

    #[test]
    fn check_complexity_hotspot_stores_both_raw_and_adjusted() {
        let func = FunctionMetrics {
            name: "test_func".to_string(),
            cyclomatic: 15,
            cognitive: 20,
            adjusted_complexity: Some(8.5),
            // ... other fields
        };

        if let Some(DebtType::ComplexityHotspot {
            cyclomatic,
            cognitive,
            adjusted_cyclomatic,
        }) = check_complexity_hotspot(&func)
        {
            assert_eq!(cyclomatic, 15, "Raw cyclomatic should be stored");
            assert_eq!(cognitive, 20, "Cognitive should be stored");
            assert_eq!(adjusted_cyclomatic, Some(9), "Adjusted cyclomatic should be rounded and stored");
        } else {
            panic!("Expected ComplexityHotspot");
        }
    }

    #[test]
    fn check_complexity_hotspot_high_cognitive_still_flags() {
        // Even with low adjusted cyclomatic, high cognitive should flag
        let func = FunctionMetrics {
            name: "reconcile_state".to_string(),
            cyclomatic: 9,
            cognitive: 16,  // Above threshold
            adjusted_complexity: Some(4.15),  // Below threshold
            // ... other fields
        };

        let result = check_complexity_hotspot(&func);
        assert!(result.is_some(), "Should flag due to high cognitive complexity");

        if let Some(DebtType::ComplexityHotspot { adjusted_cyclomatic, .. }) = result {
            assert_eq!(adjusted_cyclomatic, Some(4), "Should store adjusted complexity");
        }
    }
}
```

### Integration Tests

**File**: `tests/adjusted_complexity_classification_test.rs`

```rust
#[test]
fn classify_low_entropy_function_uses_adjusted_complexity() {
    // Simulate analysis of state_reconciliation.rs:81
    let source = r#"
        pub fn reconcile_state(current: State, target: State) -> Result<Vec<Action>> {
            let mut actions = vec![];
            if current.mode != target.mode {
                if current.has_active_connections() {
                    if target.mode == Mode::Offline {
                        actions.push(drain_connections());
                        if current.has_pending_writes() {
                            actions.push(flush_writes());
                        }
                    }
                } else if target.allows_reconnect() {
                    actions.push(establish_connections());
                }
            }
            Ok(actions)
        }
    "#;

    let result = analyze_rust_code(source, "test.rs");
    let func = &result.functions[0];

    // Verify entropy adjustment happened
    assert!(func.adjusted_complexity.is_some());
    assert!(func.adjusted_complexity.unwrap() < func.cyclomatic as f64);

    // Verify classification uses adjusted complexity
    let debt_type = determine_debt_type(func, &None, &CallGraph::new(), &func_id);

    // If cognitive > 15, should still be flagged, but with adjusted complexity stored
    if let DebtType::ComplexityHotspot { cyclomatic, adjusted_cyclomatic, .. } = debt_type {
        assert_eq!(cyclomatic, func.cyclomatic);
        assert!(adjusted_cyclomatic.is_some());
        assert!(adjusted_cyclomatic.unwrap() < cyclomatic);
    }
}
```

### Regression Tests

- Run full test suite to ensure no breaking changes
- Verify existing debt item serialization still works
- Check that functions without entropy analysis are unaffected

## Documentation Requirements

### Code Documentation

1. **Update `check_complexity_hotspot()` docstring**:
   ```rust
   /// Determine if a function is a complexity hotspot.
   ///
   /// Uses entropy-adjusted cyclomatic complexity when available (spec 182).
   /// This prevents false positives for functions with repetitive, predictable structure.
   ///
   /// # Thresholds
   /// - Cyclomatic (or adjusted): > 10
   /// - Cognitive: > 15
   ///
   /// # Returns
   /// `Some(DebtType::ComplexityHotspot)` if function exceeds thresholds.
   ```

2. **Document `DebtType::ComplexityHotspot` fields**:
   ```rust
   ComplexityHotspot {
       /// Raw cyclomatic complexity (unadjusted)
       cyclomatic: u32,
       /// Cognitive complexity
       cognitive: u32,
       /// Entropy-adjusted cyclomatic complexity (spec 182)
       /// None if entropy analysis was not performed
       adjusted_cyclomatic: Option<u32>,
   },
   ```

### User Documentation

**Update relevant sections in**:
- `README.md` - Mention adjusted complexity in complexity analysis section
- `book/src/entropy-analysis.md` - Explain how entropy affects classification
- `book/src/scoring-strategies.md` - Document adjusted complexity usage

**Example addition to `book/src/entropy-analysis.md`**:

```markdown
## Entropy-Adjusted Classification

Functions with low entropy (< 0.35) have their cyclomatic complexity dampened
before classification. This prevents false positives for functions with
repetitive, predictable structure.

**Example**:
- Raw cyclomatic: 9
- Entropy: 0.28 (low - predictable pattern)
- Dampening factor: 0.51
- Adjusted cyclomatic: 4.15

The adjusted complexity (4.15) is used for threshold comparison, so this
function would NOT be flagged as a complexity hotspot (threshold: 10).

However, cognitive complexity is NOT adjusted, so functions with high
cognitive complexity will still be flagged even if their adjusted cyclomatic
complexity is low.
```

## Implementation Notes

### Handling Edge Cases

1. **Functions without entropy analysis**:
   - `adjusted_complexity` will be `None`
   - Fall back to raw `cyclomatic`
   - No behavior change for these functions

2. **Functions with high cognitive but low adjusted cyclomatic**:
   - Still flagged as complexity hotspot (cognitive > 15)
   - `adjusted_cyclomatic` stored for downstream use
   - Recommendations can provide more nuanced guidance

3. **Rounding adjusted complexity**:
   - Round to nearest integer for threshold comparison
   - Prevents edge cases like 10.49 vs 10.51

### Migration Path

1. **Phase 1**: Update `DebtType` enum (backward compatible with `#[serde(default)]`)
2. **Phase 2**: Update `check_complexity_hotspot()` logic
3. **Phase 3**: Update all pattern matches incrementally
4. **Phase 4**: Update documentation and tests

### Rollback Plan

If issues arise:
1. Revert `check_complexity_hotspot()` to use raw cyclomatic
2. Keep `adjusted_cyclomatic` field (with `#[serde(default)]`)
3. Wait for spec 183 to provide full solution

## Success Metrics

### Quantitative

- **False positive reduction**: >= 20% reduction in flagged complexity hotspots
- **Precision improvement**: >= 15% increase in true positive rate
- **Performance**: < 1% analysis time overhead
- **Test coverage**: >= 95% for modified code paths

### Qualitative

- Developers report fewer irrelevant complexity warnings
- Low-entropy functions no longer incorrectly flagged
- Recommendations are more accurate and actionable

### Validation

1. **Benchmark suite**: Run on 50+ open-source Rust projects
2. **False positive analysis**: Compare before/after flagged functions
3. **Manual review**: Sample 20 flagged functions to verify correctness
4. **User feedback**: Survey 5+ users on recommendation quality

## Open Questions

1. **Should cognitive complexity also be entropy-adjusted?**
   - Current approach: No adjustment (cognitive measures mental load)
   - Alternative: Apply similar dampening for low-entropy functions
   - Decision: Keep cognitive unadjusted (spec 182), revisit in future spec if needed

2. **What threshold should be used for adjusted complexity?**
   - Current approach: Same threshold (10) as raw cyclomatic
   - Alternative: Lower threshold (e.g., 8) since adjusted is already dampened
   - Decision: Use same threshold for consistency, monitor false negatives

3. **Should we store dampening factor in DebtType?**
   - Current approach: Only store adjusted_cyclomatic
   - Alternative: Store both adjusted value and dampening factor
   - Decision: Store only adjusted value (simplicity), factor available in FunctionMetrics

## Future Enhancements

1. **Configurable thresholds for adjusted complexity** (separate from raw)
2. **Entropy-adjusted cognitive complexity** (if research supports it)
3. **Visual indication of adjustment in output** (e.g., "cyclomatic: 9 (adj: 4)")
4. **Historical trend analysis** of false positive reduction

## Related Work

- **Spec 183**: Use adjusted complexity in recommendations (depends on this spec)
- **Spec 178**: Fix moderate complexity recommendation logic
- **Spec 177**: Role-aware complexity recommendations
- **Spec 176**: Pattern-based complexity recommendations
- **Entropy analysis documentation**: `book/src/entropy-analysis.md`

## References

- McCabe, T. J. (1976). "A Complexity Measure". IEEE Transactions on Software Engineering.
- Shannon, C. E. (1948). "A Mathematical Theory of Communication".
- Debtmap entropy implementation: `src/complexity/entropy_core.rs`
