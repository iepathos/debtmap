---
number: 157a
title: Add PurityLevel Enum (Foundation)
category: foundation
priority: critical
status: draft
dependencies: []
created: 2025-11-03
parent_spec: 157
---

# Specification 157a: Add PurityLevel Enum (Foundation)

**Category**: foundation
**Priority**: critical
**Status**: draft
**Dependencies**: None
**Parent Spec**: 157 - Local vs External Mutation Distinction

## Context

This is **Stage 1** of implementing local vs external mutation distinction (Spec 157). This stage adds the foundational types in a **fully backward-compatible** way that doesn't break any existing code.

## Objective

Add `PurityLevel` enum and optional field to `FunctionMetrics` without breaking any existing code or tests.

## Requirements

### Functional Requirements

1. **Add PurityLevel Enum** to `src/core/mod.rs`:
   ```rust
   #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
   pub enum PurityLevel {
       StrictlyPure,    // No mutations whatsoever
       LocallyPure,     // Only local mutations (NEW - functionally pure)
       ReadOnly,        // Reads external state but doesn't modify
       Impure,          // Modifies external state or performs I/O
   }
   ```

2. **Add Optional Field** to `FunctionMetrics`:
   ```rust
   pub struct FunctionMetrics {
       // ... existing fields ...

       // NEW: Refined purity classification (replaces is_pure eventually)
       #[serde(default, skip_serializing_if = "Option::is_none")]
       pub purity_level: Option<PurityLevel>,
   }
   ```

3. **Update FunctionMetrics::new()** to initialize as `None`:
   ```rust
   pub fn new(name: String, start_line: usize, end_line: usize) -> Self {
       Self {
           // ... existing fields ...
           purity_level: None,
       }
   }
   ```

### Non-Functional Requirements

- **Zero Breaking Changes**: All existing code must continue to compile
- **Backward Compatible Serialization**: `#[serde(default, skip_serializing_if = "Option::is_none")]`
- **No Test Changes Required**: Tests using `is_pure` continue to work
- **Quick Implementation**: <30 minutes

## Acceptance Criteria

- [x] `PurityLevel` enum added to `src/core/mod.rs`
- [x] `purity_level: Option<PurityLevel>` field added to `FunctionMetrics`
- [x] Field has `#[serde(default, skip_serializing_if = "Option::is_none")]` attribute
- [x] `FunctionMetrics::new()` initializes `purity_level: None`
- [x] `cargo build` succeeds
- [x] `cargo test` passes (all existing tests)
- [x] `cargo clippy` passes with no new warnings
- [x] `cargo fmt` applied

## Implementation Steps

1. **Add PurityLevel enum** before `FunctionMetrics` struct in `src/core/mod.rs`
2. **Add field to FunctionMetrics** with serde attributes
3. **Update FunctionMetrics::new()** to initialize the field
4. **Run cargo fmt**
5. **Run cargo clippy** and fix any warnings
6. **Run cargo test** to verify no regressions
7. **Commit changes**:
   ```
   feat: add PurityLevel enum for refined purity analysis

   Adds PurityLevel enum with four levels:
   - StrictlyPure: No mutations
   - LocallyPure: Only local mutations (functionally pure)
   - ReadOnly: Reads external state
   - Impure: Modifies external state

   This is stage 1 of spec 157 - foundation types only.
   Backward compatible - existing code unchanged.
   ```

## Testing Strategy

- Existing tests should all pass unchanged
- No new tests required (this is just adding types)
- Verify backward compatibility with serialization/deserialization

## Documentation

Add doc comments to the enum:

```rust
/// Refined purity classification that distinguishes local from external mutations.
///
/// This enum provides more nuanced purity analysis than the simple boolean `is_pure` field.
/// It enables better scoring for functions that use local mutations for efficiency but are
/// functionally pure (referentially transparent).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PurityLevel {
    /// No mutations whatsoever - pure mathematical functions
    StrictlyPure,

    /// Uses local mutations for efficiency but no external side effects
    /// (builder patterns, accumulators, owned `mut self`)
    LocallyPure,

    /// Reads external state but doesn't modify it (constants, `&self` methods)
    ReadOnly,

    /// Modifies external state or performs I/O (`&mut self`, statics, I/O)
    Impure,
}
```

## Estimated Effort

**Time**: 20-30 minutes
**Complexity**: Low
**Risk**: Very Low (additive only, fully backward compatible)

## Next Steps

After this spec is implemented, proceed to:
- **Spec 157b**: Implement ScopeTracker module
- **Spec 157c**: Integrate scope tracking into PurityDetector
- **Spec 157d**: Update scoring to use LocallyPure
