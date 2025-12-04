---
number: 181h
title: "God Object Refactor: Phase 8 - Update Public API & Cleanup"
category: optimization
priority: high
status: draft
dependencies: [181g]
parent_spec: 181
phase: 8
estimated_time: "0.5 day"
created: 2025-12-03
updated: 2025-12-03
---

# Specification 181h: God Object Refactor - Phase 8: Update Public API & Cleanup

**Category**: optimization
**Priority**: high
**Status**: draft
**Parent Spec**: 181
**Phase**: 8 of 9
**Estimated Time**: 0.5 day
**Dependencies**: 181g (Orchestration created)

## Context

All new modules have been created (types, thresholds, predicates, scoring, classifier, recommender, detector). The old files still exist. This phase updates the public API and marks old files as deprecated.

## Objective

1. Update `mod.rs` to re-export all functionality from new modules
2. Mark old files (`god_object_detector.rs`, `god_object_analysis.rs`) as deprecated
3. Verify all tests pass
4. Run full quality checks

## Requirements

### 1. Update src/organization/god_object/mod.rs

**Complete module structure**:
```rust
//! God Object Detection Module
//!
//! Refactored following Stillwater principles (Pure Core, Imperative Shell).
//!
//! ## Architecture
//!
//! **Pure Core** (business logic):
//! - `types` - Data structures
//! - `thresholds` - Configuration
//! - `predicates` - Detection predicates
//! - `scoring` - Scoring algorithms
//! - `classifier` - Classification logic
//! - `recommender` - Recommendation generation
//!
//! **Orchestration**:
//! - `detector` - Composes pure functions into pipeline
//!
//! **I/O Shell**:
//! - `ast_visitor` - AST traversal

// New modules
mod types;
mod thresholds;
mod predicates;
mod scoring;
mod classifier;
mod recommender;
mod detector;
mod ast_visitor;

// Re-exports for public API
pub use types::*;
pub use thresholds::*;
pub use scoring::{
    calculate_god_object_score,
    calculate_god_object_score_weighted,
};
pub use classifier::{
    determine_confidence,
    group_methods_by_responsibility,
    classify_god_object,
};
pub use recommender::{
    suggest_module_splits_by_domain,
    recommend_module_splits,
    recommend_module_splits_enhanced,
};
pub use detector::GodObjectDetector;
pub use ast_visitor::TypeVisitor;
```

### 2. Add Deprecation Warnings to Old Files

**src/organization/god_object_detector.rs**:
```rust
#![deprecated(
    since = "0.8.0",
    note = "Use src/organization/god_object/detector.rs instead. \
            This file will be removed in 0.9.0"
)]

// Keep existing code for one release cycle
```

**src/organization/god_object_analysis.rs**:
```rust
#![deprecated(
    since = "0.8.0",
    note = "Use src/organization/god_object/{scoring,classifier,recommender}.rs instead. \
            This file will be removed in 0.9.0"
)]

// Keep existing code for one release cycle
```

### 3. Update src/organization/mod.rs

Ensure re-exports from god_object module are correct:
```rust
pub use god_object::{
    GodObjectDetector,
    GodObjectAnalysis,
    EnhancedGodObjectAnalysis,
    calculate_god_object_score,
    determine_confidence,
    // ... all other public exports
};
```

### 4. Verify Tests

Run all test suites:
```bash
# All god object tests
cargo test god_object

# Full test suite
cargo test

# Verify no warnings about deprecated items in tests
cargo test 2>&1 | grep -i deprecat
```

### 5. Run Quality Checks

```bash
# Clippy
cargo clippy --all-targets --all-features -- -D warnings

# Format
cargo fmt --all -- --check

# Documentation
cargo doc --no-deps --document-private-items

# Benchmarks (verify < 5% regression)
cargo bench --bench god_object_bench
```

## Acceptance Criteria

### Public API
- [ ] mod.rs updated with all re-exports
- [ ] All public functions accessible from `debtmap::organization::god_object`
- [ ] Backward compatibility maintained
- [ ] No breaking changes

### Deprecation
- [ ] Old files marked as deprecated
- [ ] Deprecation messages include migration path
- [ ] Planned removal version documented (0.9.0)

### Testing
- [ ] All 6 test files pass unchanged:
  - `tests/god_object_metrics_test.rs`
  - `tests/god_object_struct_recommendations.rs`
  - `tests/god_object_type_based_clustering_test.rs`
  - `tests/god_object_confidence_classification_test.rs`
  - `tests/god_object_detection_test.rs`
  - `tests/god_object_config_rs_test.rs`
- [ ] `cargo test` passes with 0 failures
- [ ] No new warnings introduced

### Quality
- [ ] `cargo clippy --all-targets -- -D warnings` passes
- [ ] `cargo fmt --all -- --check` passes
- [ ] `cargo doc --no-deps` builds without warnings
- [ ] Benchmarks show < 5% regression

### Documentation
- [ ] ARCHITECTURE.md updated (if exists)
- [ ] CLAUDE.md updated with new structure
- [ ] Module-level docs complete for all new modules

### Commit
- [ ] Commit message: "refactor(god-object): update public API and deprecate old files (spec 181h)"
- [ ] Commit includes test results
- [ ] Commit includes benchmark results

## Testing Strategy

### Pre-commit Checks
```bash
# All tests must pass
cargo test

# No clippy warnings
cargo clippy --all-targets -- -D warnings

# Code properly formatted
cargo fmt --all -- --check

# Documentation builds
cargo doc --no-deps

# Benchmarks acceptable
cargo bench --bench god_object_bench > benchmark_results.txt
# Compare with baseline from Phase 1
```

### Verification Checklist
- [ ] Can import `GodObjectDetector` from `debtmap::organization`
- [ ] Can import `calculate_god_object_score` from `debtmap::organization`
- [ ] Can import all types from `debtmap::organization`
- [ ] Old paths still work (deprecated but functional)
- [ ] New paths work without deprecation warnings

## Success Metrics

### Quantitative
- ✅ 100% test pass rate
- ✅ 0 clippy warnings
- ✅ 0 new deprecation warnings in tests
- ✅ < 5% performance regression
- ✅ Public API fully backward compatible

### Qualitative
- ✅ Clear module organization visible in docs
- ✅ Easy to find functionality
- ✅ Deprecation path clear for users
- ✅ Ready for final cleanup (Phase 9)

## Next Phase

**Phase 9** (Spec 181i): Delete Old Files

This is the final phase where deprecated files are deleted and the refactoring is complete.

## References

- Parent: `specs/181-split-god-object-detector-module.md`
- Previous: `specs/181g-create-orchestration.md`
- Next: `specs/181i-delete-old-files.md`
