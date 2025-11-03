---
number: 157d
title: Update Scoring to Use LocallyPure
category: foundation
priority: critical
status: draft
dependencies: [157a, 157b, 157c]
created: 2025-11-03
parent_spec: 157
---

# Specification 157d: Update Scoring to Use LocallyPure

**Category**: foundation
**Priority**: critical
**Status**: draft
**Dependencies**: 157a, 157b, 157c (PurityLevel fully implemented)
**Parent Spec**: 157 - Local vs External Mutation Distinction

## Context

This is **Stage 4 (FINAL)** of implementing local vs external mutation distinction (Spec 157). This stage updates the scoring system to use the new `purity_level` field and give appropriate multipliers to LocallyPure functions.

## Objective

Update `calculate_purity_adjustment()` in `unified_scorer.rs` to handle all four purity levels with backward-compatible fallback to `is_pure`.

## Requirements

### Functional Requirements

1. **Update calculate_purity_adjustment()** to handle `purity_level`:
   ```rust
   fn calculate_purity_adjustment(func: &FunctionMetrics) -> f64 {
       // Try new field first
       if let Some(level) = func.purity_level {
           return match level {
               PurityLevel::StrictlyPure => {
                   if func.purity_confidence.unwrap_or(0.0) > 0.8 {
                       0.70  // 30% reduction
                   } else {
                       0.80  // 20% reduction
                   }
               }
               PurityLevel::LocallyPure => {
                   // NEW: Local mutations still quite testable
                   if func.purity_confidence.unwrap_or(0.0) > 0.8 {
                       0.75  // 25% reduction
                   } else {
                       0.85  // 15% reduction
                   }
               }
               PurityLevel::ReadOnly => 0.90,  // 10% reduction
               PurityLevel::Impure => 1.0,     // No reduction
           };
       }

       // Fallback to old field for backward compatibility
       if func.is_pure == Some(true) {
           if func.purity_confidence.unwrap_or(0.0) > 0.8 {
               0.70
           } else {
               0.85
           }
       } else {
           1.0
       }
   }
   ```

2. **Add Integration Test** for scoring:
   ```rust
   #[test]
   fn test_locally_pure_gets_reduced_multiplier() {
       let code = r#"
           fn calculate_totals(items: &[Item]) -> Vec<f64> {
               let mut totals = Vec::new();
               for item in items {
                   totals.push(item.price * item.quantity);
               }
               totals
           }
       "#;

       let debt_item = analyze_and_score(code).unwrap();
       assert_eq!(debt_item.purity_level, Some(PurityLevel::LocallyPure));

       // Should get 0.75x multiplier (high confidence)
       // Verify score is lower than impure but higher than strictly pure
   }
   ```

### Non-Functional Requirements

- **Backward Compatible**: Old data using `is_pure` still scores correctly
- **No Regressions**: Existing scores don't worsen
- **Clear Logic**: Easy to understand purity-to-multiplier mapping

## Acceptance Criteria

- [x] `calculate_purity_adjustment()` handles all four PurityLevel values
- [x] Backward-compatible fallback to `is_pure` field works
- [x] Integration test verifies LocallyPure scoring
- [x] All existing scoring tests still pass
- [x] `cargo build` succeeds
- [x] `cargo test` passes
- [x] `cargo clippy` passes
- [x] `cargo fmt` applied
- [x] Documentation updated with new multipliers

## Implementation Details

### Scoring Multipliers

| Purity Level | High Confidence (>0.8) | Medium Confidence |
|--------------|------------------------|-------------------|
| StrictlyPure | 0.70 (30% reduction)   | 0.80 (20% reduction) |
| LocallyPure  | 0.75 (25% reduction)   | 0.85 (15% reduction) |
| ReadOnly     | 0.90 (10% reduction)   | 0.90 (10% reduction) |
| Impure       | 1.00 (no reduction)    | 1.00 (no reduction) |

**Rationale**:
- **LocallyPure** functions are easier to test than Impure (no external dependencies)
- **LocallyPure** slightly harder to reason about than StrictlyPure (internal mutation)
- **ReadOnly** functions are deterministic but depend on external state
- **Impure** functions get no benefit

### Updated Function

```rust
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
                    0.70  // High confidence: 30% reduction
                } else {
                    0.80  // Medium confidence: 20% reduction
                }
            }
            PurityLevel::LocallyPure => {
                // NEW: Functionally pure with local mutations
                if confidence > 0.8 {
                    0.75  // High confidence: 25% reduction
                } else {
                    0.85  // Medium confidence: 15% reduction
                }
            }
            PurityLevel::ReadOnly => 0.90,  // 10% reduction
            PurityLevel::Impure => 1.0,      // No reduction
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
        1.0  // Impure or unknown
    }
}
```

## Testing Strategy

### Unit Test

```rust
#[test]
fn test_purity_adjustment_levels() {
    let mut func = FunctionMetrics::new("test".to_string(), 1, 10);

    // StrictlyPure with high confidence
    func.purity_level = Some(PurityLevel::StrictlyPure);
    func.purity_confidence = Some(0.9);
    assert_eq!(calculate_purity_adjustment(&func), 0.70);

    // LocallyPure with high confidence
    func.purity_level = Some(PurityLevel::LocallyPure);
    func.purity_confidence = Some(0.9);
    assert_eq!(calculate_purity_adjustment(&func), 0.75);

    // LocallyPure with medium confidence
    func.purity_confidence = Some(0.7);
    assert_eq!(calculate_purity_adjustment(&func), 0.85);

    // ReadOnly
    func.purity_level = Some(PurityLevel::ReadOnly);
    assert_eq!(calculate_purity_adjustment(&func), 0.90);

    // Impure
    func.purity_level = Some(PurityLevel::Impure);
    assert_eq!(calculate_purity_adjustment(&func), 1.0);
}

#[test]
fn test_backward_compatibility_with_is_pure() {
    let mut func = FunctionMetrics::new("test".to_string(), 1, 10);

    // Old code using is_pure field only
    func.purity_level = None;  // Not set
    func.is_pure = Some(true);
    func.purity_confidence = Some(0.9);

    // Should still work with old logic
    assert_eq!(calculate_purity_adjustment(&func), 0.70);
}
```

### Integration Test

```rust
#[test]
fn test_end_to_end_locally_pure_scoring() {
    let code = r#"
        fn build_list(items: Vec<i32>) -> Vec<i32> {
            let mut result = Vec::with_capacity(items.len());
            for item in items {
                result.push(item * 2);
            }
            result
        }
    "#;

    // Parse and analyze
    let file_metrics = analyze_rust_code(code, Path::new("test.rs")).unwrap();
    let func = &file_metrics.complexity.functions[0];

    // Should be classified as LocallyPure
    assert_eq!(func.purity_level, Some(PurityLevel::LocallyPure));

    // Calculate score
    let score = calculate_unified_score(func);

    // Verify purity adjustment was applied (0.75x or 0.85x)
    // Score should be lower than impure equivalent
}
```

## Documentation Updates

Update `docs/purity-analysis.md` with:

```markdown
## Purity Levels and Scoring

Debtmap uses a four-level purity classification:

### Strictly Pure (Best: 0.70-0.80x multiplier)
No mutations whatsoever. Pure mathematical functions.
```rust
fn add(a: i32, b: i32) -> i32 {
    a + b
}
```

### Locally Pure (Very Good: 0.75-0.85x multiplier)
Uses local mutations for efficiency but no external side effects.
Functionally pure - same inputs always produce same outputs.
```rust
fn calculate_totals(items: &[Item]) -> Vec<f64> {
    let mut totals = Vec::new();  // Local mutation only
    for item in items {
        totals.push(item.price * item.quantity);
    }
    totals  // Functionally pure!
}
```

### Read-Only (Good: 0.90x multiplier)
Reads external state but doesn't modify it.
```rust
const MAX: i32 = 100;
fn is_valid(x: i32) -> bool {
    x < MAX  // Reads constant
}
```

### Impure (No benefit: 1.0x multiplier)
Modifies external state or performs I/O.
```rust
impl Counter {
    fn increment(&mut self) {
        self.count += 1;  // External mutation
    }
}
```
```

## Estimated Effort

**Time**: 1-2 hours
**Complexity**: Low
**Risk**: Low (isolated scoring change, backward compatible)

## Success Metrics

After implementation:
- Functions with local mutations get 0.75x-0.85x multiplier (instead of 1.0x)
- Estimated 20-30% of previously "impure" functions reclassified as LocallyPure
- Lower false negative rate (<5% vs. previous 20-30%)
- All existing tests still pass
- Scoring only improves (never worsens)

## Final Validation

This completes Spec 157. After implementation:
1. Run full test suite
2. Analyze a sample Rust project and verify:
   - Builder patterns get LocallyPure classification
   - Accumulator patterns get LocallyPure classification
   - &mut self methods still get Impure classification
3. Compare before/after debt scores on real codebase
4. Verify false negative rate reduction

## Next Steps

After this spec is implemented, Spec 157 is **COMPLETE**. Optional follow-up:
- Spec 158: Enhanced closure analysis for upvalue mutations
- Performance benchmarking and optimization if needed
