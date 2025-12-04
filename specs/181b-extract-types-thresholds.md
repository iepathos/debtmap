---
number: 181b
title: "God Object Refactor: Phase 2 - Extract Types & Thresholds"
category: optimization
priority: high
status: draft
dependencies: [181a]
parent_spec: 181
phase: 2
estimated_time: "0.5 day"
created: 2025-12-03
updated: 2025-12-03
---

# Specification 181b: God Object Refactor - Phase 2: Extract Types & Thresholds

**Category**: optimization
**Priority**: high
**Status**: draft
**Parent Spec**: 181 (Refactor God Object Detection Module)
**Phase**: 2 of 9
**Estimated Time**: 0.5 day
**Dependencies**: 181a (Foundation & Analysis)

## Context

This is **Phase 2** of the God Object Detection Module refactoring. Phase 1 (181a) completed the analysis and created `REFACTORING_PLAN.md`.

This phase extracts the foundation: all data structures and constants. This is the **lowest risk** phase since types have no logic or side effects.

## Objective

Create two new modules with zero behavioral changes:
1. `src/organization/god_object/types.rs` - All data structures
2. `src/organization/god_object/thresholds.rs` - All constants and configuration

Update existing files to use these new modules. All tests must continue to pass.

## Requirements

### 1. Create types.rs

**Target**: < 200 lines (300 line absolute maximum)

**Extract all type definitions from**:
- `src/organization/god_object_analysis.rs`
- `src/organization/god_object_detector.rs`

**Types to extract** (based on spec 181, lines 481-509):
```rust
// From god_object_analysis.rs
pub struct GodObjectAnalysis { ... }
pub struct EnhancedGodObjectAnalysis { ... }
pub enum DetectionType { GodClass, GodFile, GodModule }
pub struct ModuleSplit { ... }
pub struct StructMetrics { ... }
pub enum GodObjectConfidence { High, Medium, Low }
pub struct PurityDistribution { ... }
pub struct FunctionVisibilityBreakdown { ... }
pub enum Priority { Critical, High, Medium, Low }
pub enum SplitAnalysisMethod { ... }
pub enum RecommendationSeverity { ... }
pub struct ClassificationResult { ... }
pub enum SignalType { ... }
pub enum GodObjectType { ... }
pub struct InterfaceEstimate { ... }
pub struct MergeRecord { ... }
pub enum MetricInconsistency { ... }
pub enum StageType { ... }
pub struct StructWithMethods { ... }

// From god_object_detector.rs (if any)
pub struct GodObjectClassificationParams<'a> { ... }
pub struct DomainAnalysisParams<'a> { ... }
// Note: Adapters may stay in detector.rs (Phase 7)
```

**Requirements**:
- Include all derive macros (Debug, Clone, Serialize, Deserialize, etc.)
- Include all struct field documentation
- Keep all visibility modifiers (pub, pub(crate), etc.)
- Only include types (structs, enums), not functions
- Add module-level documentation explaining Stillwater principles

**Module Documentation Template**:
```rust
//! # God Object Types (Pure Data)
//!
//! Core data structures for god object detection.
//!
//! ## Stillwater Architecture
//!
//! This module is part of the **Pure Core** - data structures with no behavior.
//! Following Stillwater principles:
//! - Types are pure data (no methods with side effects)
//! - Validation and computation are separate functions (in other modules)
//! - No I/O operations
//!
//! ## Organization
//!
//! - Analysis types: `GodObjectAnalysis`, `EnhancedGodObjectAnalysis`
//! - Configuration types: `DetectionType`, `GodObjectConfidence`, `Priority`
//! - Metric types: `StructMetrics`, `PurityDistribution`, etc.
//! - Recommendation types: `ModuleSplit`, `ClassificationResult`
```

### 2. Create thresholds.rs

**Target**: < 150 lines

**Extract all constants and configuration**:
```rust
pub struct GodObjectThresholds {
    pub method_count_threshold: usize,
    pub field_count_threshold: usize,
    pub responsibility_threshold: usize,
    pub confidence_high_threshold: f64,
    pub confidence_medium_threshold: f64,
    // ... all threshold fields
}

impl Default for GodObjectThresholds {
    fn default() -> Self {
        Self {
            method_count_threshold: 15,
            field_count_threshold: 10,
            // ... all defaults
        }
    }
}

// Constants
pub const HYBRID_STANDALONE_THRESHOLD: usize = 50;
pub const HYBRID_DOMINANCE_RATIO: usize = 3;
pub const MIN_METHODS_FOR_GOD_CLASS: usize = 15;
pub const MIN_RESPONSIBILITIES_FOR_GOD: usize = 3;
// ... all other constants
```

**Requirements**:
- All constants from both god_object_*.rs files
- All threshold configuration
- Default implementations
- Module-level documentation

### 3. Update god_object_analysis.rs

**Changes**:
1. Add imports at top:
   ```rust
   use super::god_object::types::*;
   use super::god_object::thresholds::*;
   ```
2. Remove all type definitions (now in types.rs)
3. Remove all threshold constants (now in thresholds.rs)
4. Keep all functions (will be moved in later phases)
5. Verify file compiles

**Verification**:
- File still compiles
- All tests pass
- No behavioral changes

### 4. Update god_object_detector.rs

**Changes**:
1. Add imports at top:
   ```rust
   use super::god_object::types::*;
   use super::god_object::thresholds::*;
   ```
2. Remove any type definitions (now in types.rs)
3. Remove any threshold constants (now in thresholds.rs)
4. Keep all functions (will be moved in later phases)
5. Verify file compiles

### 5. Update mod.rs

**Changes**:
1. Declare new modules:
   ```rust
   mod types;
   mod thresholds;
   ```
2. Re-export types:
   ```rust
   pub use types::*;
   pub use thresholds::*;
   ```
3. Keep all existing re-exports (backward compatibility)

**Verification**:
- All public API remains accessible
- Tests continue to work without modification

## Acceptance Criteria

### Files Created
- [ ] `src/organization/god_object/types.rs` exists
- [ ] `src/organization/god_object/thresholds.rs` exists
- [ ] Both files < 300 lines (target: types.rs < 200, thresholds.rs < 150)
- [ ] Both files have comprehensive module-level documentation

### Code Quality
- [ ] All types extracted from god_object_analysis.rs
- [ ] All types extracted from god_object_detector.rs
- [ ] All constants extracted to thresholds.rs
- [ ] All derive macros preserved
- [ ] All documentation preserved
- [ ] No duplicated code

### Integration
- [ ] god_object_analysis.rs imports from new modules
- [ ] god_object_detector.rs imports from new modules
- [ ] mod.rs re-exports all types
- [ ] No circular dependencies
- [ ] `cargo build` succeeds
- [ ] `cargo clippy --all-targets -- -D warnings` passes
- [ ] `cargo fmt --all -- --check` passes

### Testing
- [ ] All 6 test files pass unchanged:
  - `tests/god_object_metrics_test.rs`
  - `tests/god_object_struct_recommendations.rs`
  - `tests/god_object_type_based_clustering_test.rs`
  - `tests/god_object_confidence_classification_test.rs`
  - `tests/god_object_detection_test.rs`
  - `tests/god_object_config_rs_test.rs`
- [ ] `cargo test god_object` passes
- [ ] No test modifications required

### Documentation
- [ ] types.rs has module-level docs explaining Stillwater principles
- [ ] thresholds.rs has module-level docs explaining configuration
- [ ] All public types documented
- [ ] README or CLAUDE.md updated if needed

### Commit
- [ ] Single commit with message: "refactor(god-object): extract types and thresholds (spec 181b)"
- [ ] Commit message explains what was extracted and why
- [ ] Commit includes verification that tests pass

## Implementation Steps

### Step 1: Create types.rs skeleton
```bash
# Create file with module documentation
touch src/organization/god_object/types.rs
```

### Step 2: Extract type definitions
1. Use `rg "^pub (struct|enum)" src/organization/god_object_analysis.rs` to find all types
2. For each type:
   - Copy full definition (including derives, docs, fields)
   - Paste into types.rs
   - Group by category (analysis types, metric types, etc.)
3. Repeat for god_object_detector.rs
4. Add necessary imports to types.rs

### Step 3: Create thresholds.rs
1. Create file with module documentation
2. Extract `GodObjectThresholds` struct
3. Extract all `const` declarations
4. Implement `Default` trait
5. Add documentation for each threshold

### Step 4: Update existing files
1. Add imports to god_object_analysis.rs
2. Delete extracted types (verify with git diff)
3. Add imports to god_object_detector.rs
4. Delete extracted types
5. Verify both files still compile: `cargo build`

### Step 5: Update mod.rs
1. Add module declarations
2. Add re-exports
3. Verify public API: `cargo doc --no-deps`

### Step 6: Run tests
```bash
cargo test god_object
cargo clippy --all-targets -- -D warnings
cargo fmt --all -- --check
```

### Step 7: Commit
```bash
git add src/organization/god_object/
git commit -m "refactor(god-object): extract types and thresholds (spec 181b)

Extract pure data structures and constants into focused modules:
- types.rs: All data structures (GodObjectAnalysis, EnhancedGodObjectAnalysis, etc.)
- thresholds.rs: Configuration and constants

Part of god object module refactoring (spec 181, phase 2/9).
Backward compatible - all tests pass unchanged.
"
```

## Testing Strategy

### Pre-implementation
1. Run baseline tests: `cargo test god_object` (should pass)
2. Note any flaky tests

### During implementation
1. After creating types.rs: `cargo check`
2. After creating thresholds.rs: `cargo check`
3. After updating god_object_analysis.rs: `cargo build`
4. After updating god_object_detector.rs: `cargo build`
5. After updating mod.rs: `cargo test god_object`

### Post-implementation
1. Full test suite: `cargo test`
2. Clippy: `cargo clippy --all-targets -- -D warnings`
3. Format check: `cargo fmt --all -- --check`
4. Doc build: `cargo doc --no-deps`

### Verification Commands
```bash
# Line count verification
tokei src/organization/god_object/types.rs
tokei src/organization/god_object/thresholds.rs

# Ensure < 300 lines each

# Public API verification
cargo doc --no-deps --open
# Navigate to organization::god_object and verify all types visible

# Dependency check
cargo tree --edges normal --package debtmap | grep god_object
# Should show no circular dependencies
```

## Rollback Plan

If issues arise:
```bash
# Rollback the commit
git reset --hard HEAD~1

# Or revert specific files
git checkout HEAD~1 -- src/organization/god_object/

# Fix issues and re-apply
```

This is the safest phase to rollback since it's just type extraction.

## Success Metrics

### Quantitative
- ✅ types.rs: < 200 lines (300 max)
- ✅ thresholds.rs: < 150 lines
- ✅ All 6 tests pass
- ✅ Zero clippy warnings
- ✅ Reduction in god_object_analysis.rs and god_object_detector.rs line counts

### Qualitative
- ✅ Types are well-organized and documented
- ✅ Clear module responsibilities
- ✅ No circular dependencies
- ✅ Easy to find type definitions (all in one place)
- ✅ Stillwater principles evident in structure

## Next Phase

**Phase 3** (Spec 181c): Extract Pure Scoring Functions
- Create `scoring.rs` with pure scoring algorithms
- Move all `calculate_god_object_score*` functions
- Add unit tests for determinism
- Commit and verify

## References

- Parent spec: `specs/181-split-god-object-detector-module.md`
- Previous phase: `specs/181a-foundation-analysis.md`
- Next phase: `specs/181c-extract-scoring.md`
- Refactoring plan: `REFACTORING_PLAN.md` (created in 181a)
