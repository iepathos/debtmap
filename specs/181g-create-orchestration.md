---
number: 181g
title: "God Object Refactor: Phase 7 - Create Orchestration Layer"
category: optimization
priority: high
status: draft
dependencies: [181f]
parent_spec: 181
phase: 7
estimated_time: "1 day"
created: 2025-12-03
updated: 2025-12-03
---

# Specification 181g: God Object Refactor - Phase 7: Create Orchestration Layer

**Category**: optimization
**Priority**: high
**Status**: draft
**Parent Spec**: 181
**Phase**: 7 of 9
**Estimated Time**: 1 day
**Dependencies**: 181f (Recommender extracted)

## Context

All pure functions have been extracted. This phase creates the orchestration layer that composes them into the analysis pipeline.

## Objective

Create `src/organization/god_object/detector.rs` (~250 lines) that:
1. Defines `GodObjectDetector` struct
2. Implements `OrganizationDetector` trait
3. Composes pure functions into analysis pipeline
4. Handles adapters for external dependencies (clustering, call graphs)

## Module Structure

```rust
//! # God Object Detector (Orchestration)
//!
//! Composes pure core functions into the detection pipeline.

use super::types::*;
use super::thresholds::*;
use super::scoring::*;
use super::classifier::*;
use super::recommender::*;
use super::ast_visitor::TypeVisitor;

/// God object detector that orchestrates analysis.
pub struct GodObjectDetector {
    thresholds: GodObjectThresholds,
}

impl GodObjectDetector {
    pub fn new() -> Self {
        Self {
            thresholds: GodObjectThresholds::default(),
        }
    }

    pub fn with_thresholds(thresholds: GodObjectThresholds) -> Self {
        Self { thresholds }
    }

    /// Main analysis pipeline - composes pure functions.
    pub fn analyze(&self, visitor: &TypeVisitor) -> EnhancedGodObjectAnalysis {
        // Compose pure functions:
        // 1. Extract metrics from visitor (data extraction)
        // 2. Calculate scores (scoring::calculate_god_object_score)
        // 3. Determine confidence (classifier::determine_confidence)
        // 4. Generate recommendations (recommender::suggest_module_splits_by_domain)
        // 5. Assemble EnhancedGodObjectAnalysis
    }
}

impl OrganizationDetector for GodObjectDetector {
    fn detect_anti_patterns(&self, file: &syn::File) -> Vec<OrganizationAntiPattern> {
        // Adapter: I/O → Pure Core → I/O
        let visitor = collect_type_data(file);
        let analysis = self.analyze(&visitor);
        convert_to_anti_patterns(analysis)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detector_pipeline() {
        let detector = GodObjectDetector::new();
        let visitor = create_test_visitor();
        let analysis = detector.analyze(&visitor);
        assert!(analysis.score >= 0.0);
    }
}
```

## Key Changes

### 1. Move GodObjectDetector

Extract `GodObjectDetector` struct and implementations from `god_object_detector.rs` to `detector.rs`.

### 2. Compose Pure Functions

Replace inline logic with calls to pure functions:
```rust
// Before (mixed logic)
fn analyze(&self, visitor: &TypeVisitor) -> Analysis {
    let score = visitor.method_count() as f64 / 15.0; // Inline math
    let confidence = if score > 2.0 { High } else { Low }; // Inline logic
    // ...
}

// After (composition)
fn analyze(&self, visitor: &TypeVisitor) -> Analysis {
    let score = scoring::calculate_god_object_score(
        visitor.method_count(),
        visitor.responsibility_count(),
        &self.thresholds
    );
    let confidence = classifier::determine_confidence(
        score,
        visitor.method_count(),
        visitor.responsibility_count()
    );
    // ...
}
```

### 3. Keep Adapters

Adapters for external dependencies stay in this module:
- `CallGraphAdapter` - Interface to call graph analysis
- `FieldAccessAdapter` - Interface to field access analysis
- Clustering integration helpers

## Acceptance Criteria

- [ ] `detector.rs` created (< 300 lines, target < 250)
- [ ] `GodObjectDetector` moved from god_object_detector.rs
- [ ] `OrganizationDetector` trait implementation complete
- [ ] Pipeline composes pure functions (no inline business logic)
- [ ] Adapters for external dependencies preserved
- [ ] All 6 test files pass unchanged
- [ ] Integration tests added for full pipeline
- [ ] Commit: "refactor(god-object): create orchestration layer (spec 181g)"

## Next Phase

**Phase 8** (Spec 181h): Update Public API & Cleanup

## References

- Parent: `specs/181-split-god-object-detector-module.md`
- Previous: `specs/181f-extract-recommender.md`
- Next: `specs/181h-update-public-api.md`
