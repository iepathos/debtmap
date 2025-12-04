---
number: 181c
title: "God Object Refactor: Phase 3 - Extract Pure Scoring Functions"
category: optimization
priority: high
status: draft
dependencies: [181b]
parent_spec: 181
phase: 3
estimated_time: "1 day"
created: 2025-12-03
updated: 2025-12-03
---

# Specification 181c: God Object Refactor - Phase 3: Extract Pure Scoring Functions

**Category**: optimization
**Priority**: high
**Status**: draft
**Parent Spec**: 181
**Phase**: 3 of 9
**Estimated Time**: 1 day
**Dependencies**: 181b (Types & Thresholds extracted)

## Context

Phase 2 (181b) extracted all data structures to `types.rs` and constants to `thresholds.rs`. This phase extracts pure scoring algorithms - the easiest pure functions to extract since they're deterministic math with no side effects.

## Objective

Create `src/organization/god_object/scoring.rs` (~200 lines) containing all pure scoring calculation functions from `god_object_analysis.rs`.

## Requirements

### 1. Create scoring.rs

**Target**: < 200 lines (300 line absolute maximum)

**Functions to extract** (from `god_object_analysis.rs`):
- `calculate_god_object_score` - Core scoring function
- `calculate_god_object_score_weighted` - Weighted variant
- `calculate_complexity_weight` - Complexity weighting
- `calculate_purity_weight` - Purity weighting
- Any other score calculation helpers

**Module structure**:
```rust
//! # God Object Scoring (Pure Core)
//!
//! Pure functions for calculating god object scores and weights.
//!
//! ## Stillwater Architecture
//!
//! This is part of the **Pure Core** - deterministic math with no side effects.
//! All functions are:
//! - Deterministic: Same inputs → same outputs
//! - Side-effect free: No I/O, no mutations
//! - Composable: Can be chained together
//! - 100% testable: No mocks needed

use super::types::*;
use super::thresholds::*;

/// Calculate god object score from method and responsibility counts.
///
/// **Pure function** - deterministic, no side effects.
pub fn calculate_god_object_score(
    method_count: usize,
    responsibility_count: usize,
    thresholds: &GodObjectThresholds,
) -> f64 {
    // Implementation
}

// ... other scoring functions

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn test_scoring_deterministic() {
        let thresholds = GodObjectThresholds::default();
        let score1 = calculate_god_object_score(20, 5, &thresholds);
        let score2 = calculate_god_object_score(20, 5, &thresholds);
        assert_eq!(score1, score2);
    }

    proptest! {
        #[test]
        fn score_never_negative(
            method_count in 0..1000usize,
            resp_count in 0..100usize
        ) {
            let thresholds = GodObjectThresholds::default();
            let score = calculate_god_object_score(
                method_count,
                resp_count,
                &thresholds
            );
            prop_assert!(score >= 0.0);
        }

        #[test]
        fn score_monotonic_in_methods(
            base in 10..100usize,
            delta in 1..50usize
        ) {
            let thresholds = GodObjectThresholds::default();
            let score1 = calculate_god_object_score(base, 5, &thresholds);
            let score2 = calculate_god_object_score(base + delta, 5, &thresholds);
            prop_assert!(score2 >= score1);
        }
    }
}
```

### 2. Update god_object_analysis.rs

1. Add import: `use super::god_object::scoring::*;`
2. Delete extracted scoring functions
3. Update call sites to use imported functions (should be unchanged)
4. Verify compilation

### 3. Update mod.rs

1. Declare module: `mod scoring;`
2. Re-export: `pub use scoring::*;`

### 4. Add comprehensive unit tests

**Test coverage requirements**:
- [x] Determinism (same inputs → same outputs)
- [x] Non-negativity (scores always >= 0)
- [x] Monotonicity (more methods → higher score)
- [x] Edge cases (zero methods, zero responsibilities)
- [x] Threshold boundaries
- [x] Weighted vs unweighted consistency

### 5. Update benchmarks

Add benchmarks for scoring functions to `benches/god_object_bench.rs`:
```rust
fn bench_scoring(c: &mut Criterion) {
    let thresholds = GodObjectThresholds::default();

    c.bench_function("calculate_god_object_score", |b| {
        b.iter(|| {
            calculate_god_object_score(
                black_box(25),
                black_box(6),
                black_box(&thresholds)
            )
        })
    });

    c.bench_function("calculate_god_object_score_weighted", |b| {
        b.iter(|| {
            calculate_god_object_score_weighted(
                black_box(25.0),
                black_box(6),
                black_box(&thresholds)
            )
        })
    });
}
```

## Acceptance Criteria

### Files Created
- [ ] `src/organization/god_object/scoring.rs` exists (< 300 lines, target < 200)
- [ ] Module-level documentation complete
- [ ] All scoring functions extracted
- [ ] Comprehensive unit tests (determinism, properties, edge cases)
- [ ] Property-based tests with proptest

### Code Quality
- [ ] All functions are pure (no side effects)
- [ ] All functions documented with purity guarantees
- [ ] Functions < 20 lines each
- [ ] Cyclomatic complexity < 5 per function
- [ ] `cargo clippy` passes with no warnings

### Integration
- [ ] god_object_analysis.rs imports from scoring module
- [ ] mod.rs re-exports scoring functions
- [ ] All 6 test files pass unchanged
- [ ] `cargo test god_object` passes
- [ ] `cargo build` succeeds

### Performance
- [ ] Benchmarks added for all public scoring functions
- [ ] No performance regression (< 5% acceptable)
- [ ] Benchmark comparison vs baseline (from 181a)

### Commit
- [ ] Commit message: "refactor(god-object): extract pure scoring functions (spec 181c)"
- [ ] Commit includes test results
- [ ] Commit includes benchmark comparison

## Testing Strategy

### Unit Tests (Required)
```rust
#[test]
fn test_scoring_deterministic()
fn test_scoring_zero_methods()
fn test_scoring_zero_responsibilities()
fn test_scoring_threshold_boundary()
fn test_weighted_vs_unweighted_consistency()
```

### Property Tests (Required)
```rust
proptest! {
    fn score_never_negative(...)
    fn score_monotonic_in_methods(...)
    fn score_monotonic_in_responsibilities(...)
    fn weighted_score_reasonable_bounds(...)
}
```

### Integration Tests
- All existing tests must pass unchanged
- No modifications to test files required

## Success Metrics

### Quantitative
- ✅ scoring.rs < 200 lines
- ✅ 100% test coverage for scoring functions
- ✅ 10+ property tests
- ✅ All tests pass
- ✅ < 5% performance regression

### Qualitative
- ✅ Clear, focused module
- ✅ All functions pure (documented)
- ✅ Testable without mocks
- ✅ Well-documented with examples

## Next Phase

**Phase 4** (Spec 181d): Extract Pure Predicates
- Create `predicates.rs`
- Extract detection predicates (boolean functions)
- Add unit tests

## References

- Parent spec: `specs/181-split-god-object-detector-module.md`
- Previous phase: `specs/181b-extract-types-thresholds.md`
- Next phase: `specs/181d-extract-predicates.md`
