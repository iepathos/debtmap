---
number: 1
title: Upgrade Stillwater to 0.15
category: foundation
priority: high
status: draft
dependencies: []
created: 2025-12-20
---

# Specification 001: Upgrade Stillwater to 0.15

**Category**: foundation
**Priority**: high
**Status**: draft
**Dependencies**: none

## Context

Debtmap currently uses stillwater 0.13 for its effect system, validation, and reader patterns. Stillwater 0.15 introduces several new features that would benefit debtmap:

- **Writer Effect**: Accumulate logs/metrics alongside computation without threading state
- **Sink Effect**: Stream output with O(1) memory for large reports
- **Bracket Builder**: Cleaner resource management syntax
- **Enhanced error context**: Improved context chaining

This upgrade is a prerequisite for implementing the Writer and Sink effects in subsequent specifications.

## Objective

Upgrade the stillwater dependency from 0.13 to 0.15, ensuring all existing functionality continues to work and enabling access to new features.

## Requirements

### Functional Requirements
- Update Cargo.toml to reference stillwater 0.15
- Verify all existing effect system code compiles without changes
- Ensure existing validation patterns continue to function
- Confirm reader pattern helpers remain operational
- Verify retry patterns work correctly

### Non-Functional Requirements
- Zero runtime performance regression
- No increase in compile time beyond expected dependency update
- Maintain existing API compatibility in debtmap's effect module

## Acceptance Criteria

- [ ] Cargo.toml updated to `stillwater = { version = "0.15", features = ["async"] }`
- [ ] `cargo build` succeeds without errors
- [ ] `cargo test` passes all existing tests
- [ ] `cargo clippy` reports no new warnings
- [ ] New Writer Effect types are accessible: `WriterEffect`, `tell`, `tell_one`, `tap_tell`
- [ ] New Sink Effect types are accessible: `emit`, `emit_many`, `run_with_sink`
- [ ] Bracket builder API is available: `Bracket::<R>::new()`

## Technical Details

### Implementation Approach

1. Update Cargo.toml dependency version
2. Run `cargo build` to identify any breaking changes
3. Address any API changes between 0.13 and 0.15
4. Run full test suite to verify compatibility
5. Update any deprecated API usages

### API Changes from 0.13 to 0.15

Based on stillwater changelog, the following are additive changes (no breaking changes expected):

**New in 0.14**:
- Compile-time resource tracking with `Bracket` pattern
- Resource markers: `FileRes`, `DbRes`, `LockRes`, `TxRes`, `SocketRes`

**New in 0.15**:
- Writer Effect: `WriterEffect` trait, `tell()`, `tell_one()`, `tap_tell()`, `listen()`, `censor()`
- Sink Effect: `emit()`, `emit_many()`, `run_with_sink()`, `run_collecting()`
- Monoid implementations for aggregation

### Affected Files

- `Cargo.toml` - dependency version update
- `src/effects/core.rs` - verify compatibility, add new imports

## Dependencies

- **Prerequisites**: none
- **Affected Components**: `src/effects/core.rs`, all modules using stillwater types
- **External Dependencies**: stillwater 0.15 crate

## Testing Strategy

- **Unit Tests**: Run existing test suite (`cargo test`)
- **Integration Tests**: Verify effect composition patterns work
- **Compilation Tests**: Ensure all modules compile
- **Smoke Test**: Run debtmap on sample codebase

## Documentation Requirements

- **Code Documentation**: Update module docs to reference 0.15 features
- **User Documentation**: none required for this spec

## Implementation Notes

The upgrade from 0.13 to 0.15 should be backwards compatible. Stillwater follows semantic versioning and these are additive releases. However, verify:

1. Any re-exported types haven't changed paths
2. Trait bounds remain compatible
3. Generic constraints are still satisfied

If any issues arise, consult stillwater CHANGELOG.md for migration guidance.

## Migration and Compatibility

No breaking changes expected. This is an additive upgrade that unlocks new capabilities without changing existing behavior.
