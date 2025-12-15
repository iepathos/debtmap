---
number: 201
title: Consolidate Nesting Implementations to Single Source of Truth
category: foundation
priority: high
status: draft
dependencies: [198]
created: 2025-12-15
---

# Specification 201: Consolidate Nesting Implementations to Single Source of Truth

**Category**: foundation
**Priority**: high
**Status**: draft
**Dependencies**: Spec 198 (else-if fix must be applied to chosen implementation)

## Context

Debtmap has **6+ different nesting calculation implementations**, each with different bugs or inconsistencies. This causes:
- Inconsistent complexity metrics across different analysis paths
- Maintenance burden when fixing bugs (must fix in multiple places)
- Confusion about which implementation is "correct"

### Current Implementations

| Location | Implementation | Status |
|----------|----------------|--------|
| `src/complexity/pure.rs:576-622` | `calculate_max_nesting_depth` | **CORRECT** - treats else-if as flat |
| `src/extraction/extractor.rs:815-857` | `NestingVisitor` | **BUGGY** - else-if counted as nested |
| `src/analyzers/rust_complexity_calculation.rs:86-159` | `NestingVisitor` | **BUGGY** - same |
| `src/complexity/entropy.rs:457-499` | `StructureAnalyzer` | **BUGGY** - same |
| `src/complexity/languages/rust.rs:282-294` | `NestingCalculator` | **BUGGY** - counts all blocks |
| `src/organization/struct_initialization.rs:397-457` | `measure_nesting_depth` | **INCOMPLETE** - misses else/loops |

### The Correct Implementation

`src/complexity/pure.rs` has the correct implementation:

```rust
fn calculate_expr_nesting_depth(expr: &Expr, current_depth: u32) -> u32 {
    match expr {
        Expr::If(if_expr) => {
            let new_depth = current_depth + 1;
            let then_depth = calculate_block_nesting_depth(&if_expr.then_branch, new_depth);
            let else_depth = if_expr
                .else_branch
                .as_ref()
                .map(|(_, e)| calculate_expr_nesting_depth(e, current_depth))  // CORRECT!
                .unwrap_or(current_depth);
            then_depth.max(else_depth)
        }
        // ... handles While, ForLoop, Loop, Match correctly
    }
}
```

Key insight: The else branch is evaluated at `current_depth`, not `new_depth`, so `else if` chains stay flat.

## Objective

Consolidate all nesting calculations to use `complexity::pure::calculate_max_nesting_depth` as the single source of truth, eliminating duplicate implementations and ensuring consistent metrics across the codebase.

## Requirements

### Functional Requirements

1. **Single implementation**: Only `complexity::pure::calculate_max_nesting_depth` should contain nesting logic
2. **All callers migrate**: Every place that calculates nesting must call the pure implementation
3. **Delete duplicates**: Remove all other nesting implementations after migration
4. **Consistent results**: All analysis paths must produce identical nesting values for same input

### Non-Functional Requirements

1. **No performance regression**: Pure implementation is already efficient
2. **Backward compatibility for tests**: Update test expectations where values change
3. **Clear public API**: Export the function from appropriate module

## Acceptance Criteria

- [ ] `complexity::pure::calculate_max_nesting_depth` is the only nesting calculation
- [ ] `src/extraction/extractor.rs` uses pure implementation
- [ ] `src/analyzers/rust_complexity_calculation.rs` uses pure implementation
- [ ] `src/complexity/entropy.rs` uses pure implementation
- [ ] `src/complexity/languages/rust.rs` uses pure implementation
- [ ] `src/organization/struct_initialization.rs` uses pure implementation
- [ ] All duplicate `NestingVisitor` structs are deleted
- [ ] All tests pass with updated expectations
- [ ] Running same code through different analysis paths yields identical nesting

## Technical Details

### Implementation Approach

#### Step 1: Ensure pure.rs is exported

```rust
// src/complexity/mod.rs
pub use pure::{calculate_max_nesting_depth, calculate_nesting_depth};
```

#### Step 2: Update extractor.rs

```rust
// src/extraction/extractor.rs

// Remove NestingVisitor struct (lines 815-857)

// Update calculate_max_nesting method
fn calculate_max_nesting(&self, block: &syn::Block) -> u32 {
    crate::complexity::pure::calculate_max_nesting_depth(block)
}
```

#### Step 3: Update rust_complexity_calculation.rs

```rust
// src/analyzers/rust_complexity_calculation.rs

// Remove NestingVisitor struct (lines 86-159)

pub fn calculate_nesting(block: &syn::Block) -> u32 {
    crate::complexity::pure::calculate_max_nesting_depth(block)
}
```

#### Step 4: Update entropy.rs

```rust
// src/complexity/entropy.rs

fn analyze_code_structure(&self, block: &Block) -> (usize, u32) {
    // Keep unique variable counting logic
    let unique_vars = self.count_unique_variables(block);

    // Use pure implementation for nesting
    let max_nesting = crate::complexity::pure::calculate_max_nesting_depth(block);

    (unique_vars, max_nesting)
}
```

#### Step 5: Update languages/rust.rs

```rust
// src/complexity/languages/rust.rs

// Remove NestingCalculator struct (lines 282-294)

fn calculate_max_nesting(&self, item_fn: &ItemFn) -> u32 {
    crate::complexity::pure::calculate_max_nesting_depth(&item_fn.block)
}
```

#### Step 6: Update struct_initialization.rs

```rust
// src/organization/struct_initialization.rs

// Remove measure_nesting_depth and measure_depth_recursive functions

fn measure_nesting_depth(block: &syn::Block) -> (f64, usize) {
    let max_depth = crate::complexity::pure::calculate_max_nesting_depth(block) as usize;

    // For average depth, we still need custom logic or accept max as proxy
    // Option 1: Use max as proxy (simpler)
    let avg_depth = max_depth as f64;

    // Option 2: Keep separate logic for average (more accurate but adds complexity)

    (avg_depth, max_depth)
}
```

### Files to Delete Code From

| File | Lines to Remove |
|------|-----------------|
| `src/extraction/extractor.rs` | 815-857 (NestingVisitor) |
| `src/analyzers/rust_complexity_calculation.rs` | 86-159 (NestingVisitor + calculate_nesting body) |
| `src/complexity/entropy.rs` | 473-494 (StructureAnalyzer nesting logic) |
| `src/complexity/languages/rust.rs` | 282-294 (NestingCalculator) |
| `src/organization/struct_initialization.rs` | 419-457 (measure_depth_recursive) |

### Test Updates Required

Tests that assert specific nesting values may need updates:
- `tests/nesting_calculation_test.rs`
- `tests/cognitive_complexity_tests.rs`
- `tests/entropy_tests.rs`
- Any tests in `src/*/tests` modules

## Dependencies

- **Prerequisites**: Spec 198 must be complete (pure.rs already correct, but spec documents the behavior)
- **Affected Components**: All 6 files listed above
- **Supersedes**: Specs 199, 200 (those fixes become part of this consolidation)

## Testing Strategy

- **Unit Tests**: Verify pure implementation handles all edge cases
- **Consistency Tests**: Add test that runs same code through all analysis paths and asserts identical nesting
- **Regression Tests**: Ensure existing functionality unchanged (with updated expectations)
- **Integration Tests**: Verify end-to-end complexity metrics are consistent

### New Test Case

```rust
#[test]
fn test_nesting_consistency_across_analysis_paths() {
    let code = r#"
    fn test() {
        if a {
            if b {
                for i in items {
                    match x {
                        A => {}
                        B => {}
                    }
                }
            }
        } else if c {
            while d {
                e();
            }
        }
    }
    "#;

    // All these should return identical values
    let pure_nesting = /* via pure::calculate_max_nesting_depth */;
    let extractor_nesting = /* via UnifiedFileExtractor */;
    let rust_calc_nesting = /* via rust_complexity_calculation::calculate_nesting */;

    assert_eq!(pure_nesting, extractor_nesting);
    assert_eq!(pure_nesting, rust_calc_nesting);
}
```

## Documentation Requirements

- **Code Documentation**: Update docstrings to reference single implementation
- **Architecture Updates**: Document that `complexity::pure` is the authoritative source

## Implementation Notes

- Complete this spec BEFORE implementing specs 199 and 200 (they become obsolete)
- The pure implementation is already correct; this is primarily a cleanup/consolidation
- Be careful with the average nesting in struct_initialization - may need separate logic

## Migration and Compatibility

- **Breaking Changes**: Nesting values will change for code with else-if chains or deep blocks
- **User Impact**: Complexity metrics will be more accurate but may shift rankings
- **Recommendation**: Release as part of a version bump with changelog entry
