---
number: 196
title: Split io/effects.rs God Object into Focused Modules
category: optimization
priority: high
status: draft
dependencies: []
created: 2025-12-13
---

# Specification 196: Split io/effects.rs God Object into Focused Modules

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: None

## Context

`src/io/effects.rs` has been identified by debtmap as a God Object with:

- **740 lines of code** across 53 functions
- **6 distinct responsibilities** (file I/O, directory walking, cache operations, retry wrappers, composed operations)
- **High accumulated complexity**: cyclomatic 64, cognitive 83 (dampened to 41)
- **58.1% test coverage** (below target of 80%)
- **Max nesting depth**: 2 levels

This violates the Stillwater philosophy of "single responsibility per module" and the project's functional programming guidelines that recommend functions under 20 lines and clear separation of concerns.

### Current State Analysis

The file contains clearly marked sections:

1. **File Read Operations** (lines 44-105): `read_file_effect`, `read_file_bytes_effect`
2. **File Write Operations** (lines 107-142): `write_file_effect`
3. **File Existence Checks** (lines 144-173): `file_exists_effect`, `path_exists_effect`, `is_directory_effect`
4. **Directory Walking** (lines 175-251): `walk_dir_effect`, `walk_dir_with_config_effect`
5. **Cache Operations** (lines 253-364): `cache_get_effect`, `cache_set_effect`, `cache_invalidate_effect`, `cache_clear_effect`
6. **Retry-Wrapped Operations** (lines 366-444): retry versions of file/directory effects
7. **Composed Operations** (lines 446-576): `read_file_if_exists_effect`, `read_files_effect`, `walk_and_analyze_effect`, `walk_and_validate_effect`

### External Dependencies

Only **2 files** depend on this module:
1. `src/builders/effect_pipeline.rs` - Uses: `read_file_effect`, `cache_get_effect`, `cache_set_effect`, `walk_dir_with_config_effect`
2. `examples/effect_pipeline.rs` - Uses: `read_file_effect`

This minimal dependency footprint makes refactoring low-risk.

## Objective

Refactor `src/io/effects.rs` from a single 740-line file into a directory module with 5 focused sub-modules, following the established patterns from `io/writers/effects/` and `organization/god_object/`. Maintain full backward compatibility through re-exports.

## Requirements

### Functional Requirements

1. **Create effects directory module**
   - Convert `src/io/effects.rs` to `src/io/effects/` directory
   - Create `mod.rs` with comprehensive documentation and re-exports
   - Maintain all 19 public functions accessible via `crate::io::effects::`

2. **Split into 5 focused sub-modules**
   - `file.rs` - Basic file I/O (read, write, existence checks) - ~130 lines
   - `directory.rs` - Directory walking operations - ~80 lines
   - `cache.rs` - Cache get/set/invalidate/clear operations - ~120 lines
   - `retry.rs` - Retry-wrapped versions of operations - ~80 lines
   - `compose.rs` - Higher-level composed/batch operations - ~140 lines

3. **Co-locate tests with implementations**
   - Move existing tests to their respective modules
   - Each module contains its own `#[cfg(test)] mod tests`

4. **Add module documentation**
   - Each sub-module has `//!` doc comments explaining purpose
   - Main `mod.rs` documents the overall module organization
   - Follow Stillwater "Pure Core, Imperative Shell" documentation pattern

### Non-Functional Requirements

1. **Backward Compatibility**: All existing imports must continue to work unchanged
2. **Performance**: No runtime overhead from reorganization
3. **Test Coverage**: Maintain existing test coverage; aim for >80% in new modules
4. **Code Size**: Each module should be < 150 lines

## Acceptance Criteria

- [ ] `src/io/effects/` directory created with `mod.rs` and 5 sub-modules
- [ ] All 19 public functions accessible via `crate::io::effects::`
- [ ] `src/builders/effect_pipeline.rs` compiles without changes
- [ ] `examples/effect_pipeline.rs` compiles without changes
- [ ] All 10 existing tests pass (`cargo test io::effects`)
- [ ] `cargo clippy` reports no warnings
- [ ] `cargo fmt --check` passes
- [ ] Each sub-module is under 150 lines
- [ ] Module documentation follows Stillwater patterns
- [ ] Original `src/io/effects.rs` is deleted

## Technical Details

### Target Directory Structure

```
src/io/effects/
├── mod.rs          # ~50 lines - Module declaration and re-exports
├── file.rs         # ~130 lines - read_file_effect, read_file_bytes_effect,
│                   #              write_file_effect, file_exists_effect,
│                   #              path_exists_effect, is_directory_effect
├── directory.rs    # ~80 lines - walk_dir_effect, walk_dir_with_config_effect
├── cache.rs        # ~120 lines - cache_get_effect, cache_set_effect,
│                   #              cache_invalidate_effect, cache_clear_effect
├── retry.rs        # ~80 lines - read_file_with_retry_effect,
│                   #             read_file_bytes_with_retry_effect,
│                   #             walk_dir_with_retry_effect,
│                   #             write_file_with_retry_effect
└── compose.rs      # ~140 lines - read_file_if_exists_effect, read_files_effect,
                    #              walk_and_analyze_effect, walk_and_validate_effect
```

### Module Dependencies (Internal)

```
file.rs ─────────────────────────────────────────┐
                                                 │
directory.rs ────────────────────────────────────┼──> External crates
                                                 │    (stillwater, crate::*)
cache.rs ────────────────────────────────────────┤
                                                 │
retry.rs ──────> imports: file.rs, directory.rs ─┤
                                                 │
compose.rs ────> imports: file.rs, directory.rs ─┘
```

### mod.rs Structure

```rust
//! Effect-wrapped I/O operations for debtmap analysis.
//!
//! This module provides Effect-based wrappers around file system operations,
//! enabling pure functional composition while maintaining testability.
//!
//! # Module Organization
//!
//! The effects system is organized into focused sub-modules:
//!
//! - **file**: Basic file read/write/existence operations
//! - **directory**: Directory walking operations
//! - **cache**: Cache get/set/invalidate/clear operations
//! - **retry**: Retry-wrapped versions of I/O operations
//! - **compose**: Higher-level composed operations
//!
//! # Design Philosophy
//!
//! Following Stillwater's "Pure Core, Imperative Shell" pattern:
//! - All operations are wrapped in Effect types
//! - Effects defer execution until run with an environment
//! - Enables testing with mock environments
//! - Composes naturally with `and_then`, `map`, etc.

mod cache;
mod compose;
mod directory;
mod file;
mod retry;

// File operations
pub use file::{
    file_exists_effect, is_directory_effect, path_exists_effect,
    read_file_bytes_effect, read_file_effect, write_file_effect,
};

// Directory operations
pub use directory::{walk_dir_effect, walk_dir_with_config_effect};

// Cache operations
pub use cache::{
    cache_clear_effect, cache_get_effect, cache_invalidate_effect, cache_set_effect,
};

// Retry operations
pub use retry::{
    read_file_bytes_with_retry_effect, read_file_with_retry_effect,
    walk_dir_with_retry_effect, write_file_with_retry_effect,
};

// Composed operations
pub use compose::{
    read_file_if_exists_effect, read_files_effect,
    walk_and_analyze_effect, walk_and_validate_effect,
};
```

### Test Distribution

| Module | Tests Moved | Test Names |
|--------|-------------|------------|
| `file.rs` | 6 | `test_read_file_effect_success`, `test_read_file_effect_not_found`, `test_write_file_effect`, `test_file_exists_effect`, `test_is_directory_effect`, (part of) `test_read_file_if_exists_effect` |
| `directory.rs` | 1 | `test_walk_dir_effect` |
| `cache.rs` | 1 | `test_cache_operations` |
| `retry.rs` | 0 | (no existing tests) |
| `compose.rs` | 3 | `test_read_file_if_exists_effect`, `test_read_files_effect`, `test_read_files_effect_empty` |

### Implementation Approach

**Stage 1: Create Module Structure**
1. Create `src/io/effects/` directory
2. Create `mod.rs` with documentation and re-exports (referencing not-yet-existing modules)

**Stage 2: Extract file.rs**
1. Copy file read/write/existence functions from effects.rs
2. Add module documentation
3. Copy associated tests
4. Verify: `cargo check`

**Stage 3: Extract directory.rs**
1. Copy directory walking functions
2. Add module documentation
3. Copy associated test
4. Verify: `cargo check`

**Stage 4: Extract cache.rs**
1. Copy cache operations
2. Add module documentation
3. Copy associated test
4. Verify: `cargo check`

**Stage 5: Extract retry.rs**
1. Copy retry-wrapped functions
2. Add imports from sibling modules (`use super::file::*` etc.)
3. Add module documentation
4. Verify: `cargo check`

**Stage 6: Extract compose.rs**
1. Copy composed/batch operations
2. Add imports from sibling modules
3. Add module documentation
4. Copy associated tests
5. Verify: `cargo check`

**Stage 7: Delete Original and Verify**
1. Delete `src/io/effects.rs`
2. Run `cargo test`
3. Run `cargo clippy`
4. Run `cargo fmt --check`

## Dependencies

- **Prerequisites**: None
- **Affected Components**:
  - `src/io/effects.rs` → converted to `src/io/effects/`
  - `src/io/mod.rs` → may need update if it declares `pub mod effects;`
- **External Dependencies**: None changed

## Testing Strategy

### Unit Tests
- All existing tests are preserved and moved to appropriate modules
- Each module's tests run with `cargo test io::effects::file` etc.
- Shared test utilities (`create_test_env()`) moved to a test helper module or duplicated

### Integration Tests
- `src/builders/effect_pipeline.rs` serves as integration test (must compile unchanged)
- `examples/effect_pipeline.rs` serves as integration test (must compile unchanged)

### Regression Tests
- Full test suite must pass: `cargo test`
- No new clippy warnings: `cargo clippy`

## Documentation Requirements

- **Code Documentation**: Each sub-module has `//!` doc comments
- **API Documentation**: All public functions retain their existing doc comments
- **Examples**: Existing examples in doc comments are preserved

## Implementation Notes

### Import Adjustments in retry.rs and compose.rs

These modules depend on other effects modules:

```rust
// In retry.rs
use super::file::{read_file_effect, read_file_bytes_effect, write_file_effect};
use super::directory::walk_dir_effect;

// In compose.rs
use super::file::{file_exists_effect, read_file_effect};
use super::directory::walk_dir_with_config_effect;
```

### Shared Dependencies

All modules share these imports (add as needed per module):
```rust
use crate::effects::AnalysisEffect;
use crate::env::{AnalysisEnv, RealEnv};
use crate::errors::AnalysisError;
use stillwater::effect::prelude::*;
use std::path::PathBuf;
```

### Test Helper Duplication

The `create_test_env()` helper function may need to be:
1. Duplicated in each test module, OR
2. Extracted to a shared test utility module

Option 1 is simpler for this refactoring; Option 2 is better long-term but out of scope.

## Migration and Compatibility

### Breaking Changes
None. All public APIs are preserved through re-exports.

### Verification Steps
1. Ensure `use crate::io::effects::read_file_effect` still works
2. Ensure `use debtmap::io::effects::read_file_effect` still works (public API)
3. Ensure all 19 functions are accessible

## Estimated Impact

- **Lines refactored**: 740 (all of effects.rs)
- **Files created**: 6 (mod.rs + 5 sub-modules)
- **Files deleted**: 1 (effects.rs)
- **Risk**: Low - straightforward extraction with full backward compatibility
- **Complexity reduction**: Each module 80-140 lines vs original 740 lines
