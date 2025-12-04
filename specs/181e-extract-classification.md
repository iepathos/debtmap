---
number: 181e
title: "God Object Refactor: Phase 5 - Extract Classification Logic"
category: optimization
priority: high
status: draft
dependencies: [181d]
parent_spec: 181
phase: 5
estimated_time: "1 day"
created: 2025-12-03
updated: 2025-12-03
---

# Specification 181e: God Object Refactor - Phase 5: Extract Classification Logic

**Category**: optimization
**Priority**: high
**Status**: draft
**Parent Spec**: 181
**Phase**: 5 of 9
**Estimated Time**: 1 day
**Dependencies**: 181d (Predicates extracted)

## Context

Extract pure classification and grouping functions that transform data into categorizations.

## Objective

Create `src/organization/god_object/classifier.rs` (~200 lines) with pure classification logic.

## Functions to Extract

From `god_object_analysis.rs`:
- `determine_confidence` - Map scores to confidence levels
- `classify_god_object` - Categorize god object types
- `group_methods_by_responsibility` - Group methods by domain
- `infer_responsibility_with_confidence` - Classify individual methods
- `classify_detection_type` - Determine GodClass vs GodFile vs GodModule
- Related classification helpers

## Module Structure

```rust
//! # God Object Classification (Pure Transformations)
//!
//! Pure functions for classifying and grouping god objects.

use super::types::*;
use super::scoring::*;

/// Determine confidence level from score and metrics.
pub fn determine_confidence(
    score: f64,
    method_count: usize,
    responsibility_count: usize,
) -> GodObjectConfidence {
    // Pure classification logic
}

/// Group methods by inferred responsibility domain.
pub fn group_methods_by_responsibility(
    methods: &[String]
) -> HashMap<String, Vec<String>> {
    // Pure grouping logic
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_confidence_mapping() {
        assert_eq!(
            determine_confidence(2.5, 30, 6),
            GodObjectConfidence::High
        );
    }

    proptest! {
        #[test]
        fn classification_idempotent(method_name: String) {
            let r1 = infer_responsibility_with_confidence(&method_name);
            let r2 = infer_responsibility_with_confidence(&method_name);
            prop_assert_eq!(r1.category, r2.category);
        }
    }
}
```

## Acceptance Criteria

- [ ] `classifier.rs` created (< 300 lines, target < 200)
- [ ] All classification functions extracted
- [ ] Unit tests for all functions
- [ ] Property tests for idempotence
- [ ] All 6 test files pass unchanged
- [ ] Commit: "refactor(god-object): extract classification logic (spec 181e)"

## Next Phase

**Phase 6** (Spec 181f): Extract Recommendation Logic

## References

- Parent: `specs/181-split-god-object-detector-module.md`
- Previous: `specs/181d-extract-predicates.md`
- Next: `specs/181f-extract-recommender.md`
