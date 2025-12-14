---
number: 232
title: Fix Dampened Complexity Calculation
category: foundation
priority: high
status: draft
dependencies: []
created: 2025-12-13
---

# Specification 232: Fix Dampened Complexity Calculation

**Category**: foundation
**Priority**: high
**Status**: draft
**Dependencies**: None

## Context

The `adjusted_complexity` output contains a calculation error. When `dampening_factor: 1.0`, `dampened_cyclomatic` should equal `cyclomatic_complexity`, but instead it equals `cognitive_complexity`.

### Evidence from debtmap.json

```json
{
  "metrics": {
    "cyclomatic_complexity": 11,
    "cognitive_complexity": 23,
    // ...
  },
  "adjusted_complexity": {
    "dampened_cyclomatic": 23.0,  // BUG: Should be 11.0 when factor is 1.0
    "dampening_factor": 1.0
  }
}
```

### Expected Behavior

When `dampening_factor = 1.0`:
- No dampening should occur
- `dampened_cyclomatic` should equal original `cyclomatic_complexity`
- Result: `dampened_cyclomatic: 11.0` (not 23.0)

### Impact

- **Incorrect scoring**: Dampened complexity feeds into debt scoring
- **Misleading output**: Users see wrong values
- **Broken invariant**: `damped = original * factor` when factor = 1.0 should preserve original

## Objective

Fix the dampened complexity calculation so that:
1. `dampened_cyclomatic` correctly reflects dampened cyclomatic complexity
2. When `dampening_factor = 1.0`, output equals original cyclomatic
3. The calculation formula is documented

## Requirements

### Functional Requirements

1. **Fix Calculation**: Correct the `dampened_cyclomatic` value:
   - Should be `cyclomatic_complexity * dampening_factor` (or similar formula)
   - Must not use cognitive_complexity as input

2. **Document Formula**: Clearly document the dampening algorithm:
   - What is being dampened and why
   - The exact formula
   - When dampening is applied

3. **Validate Output**: Ensure invariants hold:
   - `dampening_factor = 1.0` → `dampened_cyclomatic = cyclomatic_complexity`
   - `dampening_factor < 1.0` → `dampened_cyclomatic < cyclomatic_complexity`

### Non-Functional Requirements

- Calculation is pure and deterministic
- No performance impact
- Backward compatible output structure

## Acceptance Criteria

- [ ] `dampened_cyclomatic` correctly calculated from cyclomatic complexity
- [ ] When `dampening_factor = 1.0`, `dampened_cyclomatic` equals original cyclomatic
- [ ] Unit tests verify calculation correctness
- [ ] Documentation explains dampening formula
- [ ] Integration test validates output values

## Technical Details

### Investigation: Find the Bug

First, locate where `adjusted_complexity` is calculated:

```bash
rg "dampened_cyclomatic|adjusted_complexity" src/
```

Likely location: `src/output/unified.rs` or `src/complexity/entropy.rs`

### Current (Buggy) Code

Based on the output, the bug is likely:

```rust
// BUGGY: Using cognitive_complexity instead of cyclomatic
AdjustedComplexity {
    dampened_cyclomatic: e.entropy_score * cognitive as f64,  // Wrong!
    dampening_factor: e.dampening_factor,
}
```

Or:

```rust
// BUGGY: Using adjusted_complexity (which might be cognitive-based)
AdjustedComplexity {
    dampened_cyclomatic: e.adjusted_complexity as f64,  // If this is cognitive
    dampening_factor: e.dampening_factor,
}
```

### Correct Implementation

```rust
// src/output/unified.rs

/// Adjusted complexity based on entropy analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdjustedComplexity {
    /// Cyclomatic complexity adjusted by entropy dampening factor
    /// Formula: cyclomatic_complexity * dampening_factor
    pub dampened_cyclomatic: f64,
    /// Factor applied to dampen complexity (0.0 - 1.0)
    /// 1.0 = no dampening, <1.0 = reduced complexity weight
    pub dampening_factor: f64,
}

impl FunctionDebtItemOutput {
    fn from_function_item(item: &UnifiedDebtItem, include_scoring_details: bool) -> Self {
        // ...

        // Correct calculation using cyclomatic_complexity
        let adjusted_complexity = item.entropy_details.as_ref().map(|e| {
            let cyclomatic = item.cyclomatic_complexity as f64;
            AdjustedComplexity {
                dampened_cyclomatic: cyclomatic * e.dampening_factor,
                dampening_factor: e.dampening_factor,
            }
        });

        // ...
    }
}
```

### Verify Calculation in Entropy Module

Check if `adjusted_complexity` in `EntropyDetails` is the source:

```rust
// src/complexity/entropy.rs

pub struct EntropyDetails {
    pub entropy_score: f64,
    pub dampening_factor: f64,
    /// This might be cognitive-based, which is the bug source
    pub adjusted_complexity: u32,
}
```

If `adjusted_complexity` is cognitive-based by design, rename it for clarity:

```rust
pub struct EntropyDetails {
    pub entropy_score: f64,
    pub dampening_factor: f64,
    /// Adjusted cognitive complexity (dampened)
    pub adjusted_cognitive: u32,
}
```

And calculate dampened cyclomatic separately in output:

```rust
adjusted_complexity: item.entropy_details.as_ref().map(|e| AdjustedComplexity {
    dampened_cyclomatic: item.cyclomatic_complexity as f64 * e.dampening_factor,
    dampening_factor: e.dampening_factor,
}),
```

### Test Implementation

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dampening_factor_one_preserves_cyclomatic() {
        let item = create_test_item_with_complexity(
            cyclomatic: 11,
            cognitive: 23,
            dampening_factor: 1.0,
        );

        let output = FunctionDebtItemOutput::from_function_item(&item, false);

        let adjusted = output.adjusted_complexity.unwrap();
        assert_eq!(adjusted.dampening_factor, 1.0);
        assert_eq!(adjusted.dampened_cyclomatic, 11.0);  // Must equal cyclomatic!
    }

    #[test]
    fn test_dampening_reduces_cyclomatic() {
        let item = create_test_item_with_complexity(
            cyclomatic: 20,
            cognitive: 40,
            dampening_factor: 0.5,
        );

        let output = FunctionDebtItemOutput::from_function_item(&item, false);

        let adjusted = output.adjusted_complexity.unwrap();
        assert_eq!(adjusted.dampening_factor, 0.5);
        assert_eq!(adjusted.dampened_cyclomatic, 10.0);  // 20 * 0.5
    }

    #[test]
    fn test_dampened_cyclomatic_independent_of_cognitive() {
        // Two items with same cyclomatic but different cognitive
        let item1 = create_test_item_with_complexity(
            cyclomatic: 15,
            cognitive: 10,
            dampening_factor: 0.8,
        );
        let item2 = create_test_item_with_complexity(
            cyclomatic: 15,
            cognitive: 50,
            dampening_factor: 0.8,
        );

        let output1 = FunctionDebtItemOutput::from_function_item(&item1, false);
        let output2 = FunctionDebtItemOutput::from_function_item(&item2, false);

        // Same dampened cyclomatic regardless of cognitive
        assert_eq!(
            output1.adjusted_complexity.unwrap().dampened_cyclomatic,
            output2.adjusted_complexity.unwrap().dampened_cyclomatic
        );
        assert_eq!(
            output1.adjusted_complexity.unwrap().dampened_cyclomatic,
            12.0  // 15 * 0.8
        );
    }
}
```

### Architecture Changes

- Modified: `src/output/unified.rs` (fix calculation)
- Possibly modified: `src/complexity/entropy.rs` (clarify naming)
- No structural changes to output format

## Dependencies

- **Prerequisites**: None
- **Affected Components**:
  - `src/output/unified.rs`
  - `src/complexity/entropy.rs` (if renaming fields)
- **External Dependencies**: None

## Testing Strategy

- **Unit Tests**:
  - Verify formula: `dampened = cyclomatic * factor`
  - Verify factor=1.0 preserves original
  - Verify independence from cognitive complexity

- **Integration Tests**:
  - Run debtmap, verify adjusted_complexity values
  - Spot-check items where factor=1.0

- **Regression Tests**:
  - Add test case with known cyclomatic/cognitive values

## Documentation Requirements

- **Code Documentation**: Document the dampening formula in rustdoc
- **User Documentation**: Explain what adjusted_complexity means
- **Architecture Updates**: Document entropy-based dampening algorithm

## Implementation Notes

1. **Root Cause Investigation**: Before fixing, confirm the bug source by tracing the calculation path.

2. **Field Naming**: If `adjusted_complexity` in `EntropyDetails` is intentionally cognitive-based, rename it to `adjusted_cognitive` for clarity.

3. **Formula Documentation**: Whatever the formula, document it clearly so future maintainers understand the intent.

## Migration and Compatibility

- **Output format unchanged**: Same JSON structure
- **Values will change**: `dampened_cyclomatic` values will be different (correct)
- **No breaking changes**: Consumers shouldn't depend on buggy values
