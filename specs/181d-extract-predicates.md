---
number: 181d
title: "God Object Refactor: Phase 4 - Extract Pure Predicates"
category: optimization
priority: high
status: draft
dependencies: [181c]
parent_spec: 181
phase: 4
estimated_time: "0.5 day"
created: 2025-12-03
updated: 2025-12-03
---

# Specification 181d: God Object Refactor - Phase 4: Extract Pure Predicates

**Category**: optimization
**Priority**: high
**Status**: draft
**Parent Spec**: 181
**Phase**: 4 of 9
**Estimated Time**: 0.5 day
**Dependencies**: 181c (Scoring functions extracted)

## Context

Phases 2-3 extracted types, thresholds, and scoring functions. This phase extracts pure detection predicates - boolean functions that make decisions based on data.

## Objective

Create `src/organization/god_object/predicates.rs` (~150 lines) containing all pure boolean detection functions.

## Requirements

### Functions to Extract

From `god_object_analysis.rs` and `god_object_detector.rs`:
- `is_god_object` - Main detection predicate
- `exceeds_method_threshold`
- `exceeds_field_threshold`
- `is_hybrid_god_module`
- `should_recommend_split`
- Any other boolean detection functions

### Module Structure

```rust
//! # God Object Detection Predicates (Pure Functions)
//!
//! Pure boolean functions for god object detection.
//!
//! All predicates are:
//! - Pure: No side effects, deterministic
//! - Composable: Can be combined with && and ||
//! - Testable: No mocks needed

use super::types::*;
use super::thresholds::*;

/// Check if method count exceeds threshold.
pub fn exceeds_method_threshold(count: usize, threshold: usize) -> bool {
    count > threshold
}

/// Check if counts indicate a god object.
pub fn is_god_object(
    method_count: usize,
    responsibility_count: usize,
    thresholds: &GodObjectThresholds,
) -> bool {
    method_count > thresholds.method_count_threshold
        && responsibility_count > thresholds.responsibility_threshold
}

// ... more predicates

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exceeds_method_threshold() {
        assert!(exceeds_method_threshold(20, 15));
        assert!(!exceeds_method_threshold(10, 15));
        assert!(!exceeds_method_threshold(15, 15)); // boundary
    }

    #[test]
    fn test_is_hybrid_god_module() {
        assert!(is_hybrid_god_module(60, 15));  // 60 > 15*3
        assert!(!is_hybrid_god_module(60, 25)); // 60 < 25*3
    }
}
```

### Testing Requirements

- Test all boolean combinations (true/false paths)
- Test boundary conditions
- Test predicate composition

## Acceptance Criteria

- [ ] `src/organization/god_object/predicates.rs` exists (< 200 lines)
- [ ] All predicates extracted and tested
- [ ] 100% branch coverage for predicates
- [ ] All 6 test files pass unchanged
- [ ] mod.rs updated with re-exports
- [ ] Commit: "refactor(god-object): extract detection predicates (spec 181d)"

## Next Phase

**Phase 5** (Spec 181e): Extract Classification Logic

## References

- Parent: `specs/181-split-god-object-detector-module.md`
- Previous: `specs/181c-extract-scoring.md`
- Next: `specs/181e-extract-classification.md`
