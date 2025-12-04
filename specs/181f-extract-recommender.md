---
number: 181f
title: "God Object Refactor: Phase 6 - Extract Recommendation Logic"
category: optimization
priority: high
status: draft
dependencies: [181e]
parent_spec: 181
phase: 6
estimated_time: "1 day"
created: 2025-12-03
updated: 2025-12-03
---

# Specification 181f: God Object Refactor - Phase 6: Extract Recommendation Logic

**Category**: optimization
**Priority**: high
**Status**: draft
**Parent Spec**: 181
**Phase**: 6 of 9
**Estimated Time**: 1 day
**Dependencies**: 181e (Classification extracted)

## Context

Extract pure recommendation generation functions that suggest how to split god objects.

## Objective

Create `src/organization/god_object/recommender.rs` (~250 lines) with pure recommendation logic.

## Functions to Extract

From `god_object_detector.rs`:
- `suggest_module_splits_by_domain` - Generate domain-based split suggestions
- `recommend_module_splits` - Basic split recommendations
- `recommend_module_splits_enhanced` - Enhanced recommendations
- `generate_split_rationale` - Create justification text
- Related recommendation helpers

## Module Structure

```rust
//! # God Object Recommendation Generation (Pure Functions)
//!
//! Pure functions for generating refactoring recommendations.

use super::types::*;
use super::classifier::*;

/// Suggest module splits based on domain analysis.
pub fn suggest_module_splits_by_domain(
    metrics: &[StructMetrics]
) -> Vec<ModuleSplit> {
    // Pure recommendation logic
}

/// Generate enhanced recommendations with confidence and rationale.
pub fn recommend_module_splits_enhanced(
    analysis: &GodObjectAnalysis,
    metrics: &[StructMetrics],
) -> Vec<ModuleSplit> {
    // Compose classifier and recommendation logic
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_domain_based_splits() {
        let metrics = vec![/* test data */];
        let splits = suggest_module_splits_by_domain(&metrics);
        assert!(!splits.is_empty());
    }
}
```

## Acceptance Criteria

- [ ] `recommender.rs` created (< 300 lines, target < 250)
- [ ] All recommendation functions extracted
- [ ] Unit tests for recommendation generation
- [ ] All 6 test files pass unchanged
- [ ] Commit: "refactor(god-object): extract recommendation logic (spec 181f)"

## Next Phase

**Phase 7** (Spec 181g): Create Orchestration Layer

## References

- Parent: `specs/181-split-god-object-detector-module.md`
- Previous: `specs/181e-extract-classification.md`
- Next: `specs/181g-create-orchestration.md`
