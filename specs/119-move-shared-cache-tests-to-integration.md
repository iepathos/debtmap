---
number: 119
title: Move SharedCache Tests to Integration Test File
category: testing
priority: low
status: draft
dependencies: []
created: 2025-10-18
---

# Specification 119: Move SharedCache Tests to Integration Test File

**Category**: testing
**Priority**: low
**Status**: draft
**Dependencies**: None

## Context

The `shared_cache.rs` file contains 818 lines of tests (37% of the 2196 total lines), which contributes to it being flagged as a potential god object. While the implementation itself is well-designed with proper functional decomposition, the co-located tests inflate the file size metrics.

**Current Structure**:
```
src/cache/shared_cache.rs (2196 lines)
├── Implementation: lines 1-1377 (63%)
└── Tests (#[cfg(test)]): lines 1378-2196 (37%)
    ├── 22 test functions
    ├── Test utilities and helpers
    └── Comprehensive integration scenarios
```

**Impact on Metrics**:
- Line count: 2196 (inflated by 818 test lines)
- Contributes to false positive god object detection
- Makes file harder to navigate (must scroll past 1377 lines to reach tests)

**Why This Matters**:
- Rust convention: Integration tests belong in `tests/` directory
- Unit tests can stay with implementation, but these are **integration tests** (use TempDir, test full workflows)
- Separating reduces noise in god object detection
- Improves file organization and discoverability

**Not a High Priority**:
This is a **minor organizational improvement**, not a fundamental design issue. The code is well-designed regardless of test location. Priority is low because:
- Specs 117-118 will fix the false positive issue algorithmically
- This is cosmetic cleanup, not architectural improvement
- No functional benefit, purely organizational

## Objective

Move the 22 integration tests from `src/cache/shared_cache.rs` to `tests/cache/shared_cache_test.rs`, reducing the source file to ~1377 lines and following Rust testing conventions.

## Requirements

### Functional Requirements

1. **Create Integration Test File**
   - Create `tests/cache/` directory structure
   - Create `tests/cache/shared_cache_test.rs`
   - Move all 22 test functions from `#[cfg(test)] mod tests { ... }`
   - Preserve all test functionality exactly as-is

2. **Test Migration**

   Tests to move (all 22):
   ```rust
   test_shared_cache_operations
   test_cache_stats
   test_age_calculation_pure_functions
   test_filter_entries_by_age
   test_put_with_config_test_environment
   test_put_with_config_sync_pruning_enabled
   test_put_with_config_auto_prune_disabled
   test_put_with_config_multiple_entries
   test_put_with_config_overwrites_existing
   test_cache_version_validation
   test_cache_clear
   test_compute_cache_key_with_file
   test_compute_cache_key_without_file
   test_put_with_config_large_data
   test_cleanup_removes_oldest_entries
   test_cleanup_target_size_calculation
   test_cleanup_handles_empty_cache
   test_cleanup_removes_files_from_all_components
   test_cleanup_updates_index_correctly
   test_cleanup_handles_concurrent_file_access
   test_cleanup_preserves_entries_under_target_size
   test_cleanup_pure_functions_behavior
   ```

3. **Update Imports**
   - Tests will need to import from `debtmap::cache::shared_cache::*`
   - Add `use debtmap::cache::*` for dependent types
   - Import test utilities: `tempfile::TempDir`, `std::collections::HashMap`

4. **Preserve Test Coverage**
   - All tests must pass after migration
   - Coverage metrics should remain identical
   - No change in test execution or behavior

### Non-Functional Requirements

1. **Zero Functional Changes**: Tests run identically before and after
2. **Backward Compatibility**: No API changes to `SharedCache`
3. **Build Performance**: No impact on compilation times
4. **Documentation**: Update CLAUDE.md conventions if needed

## Acceptance Criteria

- [ ] `tests/cache/` directory created
- [ ] `tests/cache/shared_cache_test.rs` created with all 22 tests
- [ ] `src/cache/shared_cache.rs` `#[cfg(test)]` module removed
- [ ] `src/cache/shared_cache.rs` reduced to ~1377 lines (63% reduction)
- [ ] All 22 tests pass: `cargo test --test shared_cache_test`
- [ ] Full test suite passes: `cargo test`
- [ ] Code coverage unchanged (run `cargo tarpaulin` before/after)
- [ ] God object detection no longer flags file size (when specs 117-118 implemented)
- [ ] No clippy warnings introduced
- [ ] Git history shows clean move operation

## Technical Details

### Implementation Approach

**Phase 1: Create Test File Structure**

```bash
# Create directory
mkdir -p tests/cache

# Create test file with proper structure
touch tests/cache/shared_cache_test.rs
```

**Phase 2: Migrate Test Code**

```rust
// tests/cache/shared_cache_test.rs

// Import the module under test
use debtmap::cache::shared_cache::{SharedCache, CacheStats, FullCacheStats};
use debtmap::cache::index_manager::CacheMetadata;
use debtmap::cache::pruning::PruningConfig;

// Test utilities
use std::collections::HashMap;
use std::time::{Duration, SystemTime};
use tempfile::TempDir;

// Migrate all 22 test functions
#[test]
fn test_shared_cache_operations() {
    let temp_dir = TempDir::new().unwrap();
    // ... (exact copy from original)
}

#[test]
fn test_cache_stats() {
    // ... (exact copy from original)
}

// ... all other tests ...
```

**Phase 3: Remove Original Tests**

```rust
// src/cache/shared_cache.rs

// Remove entire #[cfg(test)] block:
// DELETE lines 1378-2196

// File now ends at line 1377 with:
impl std::fmt::Display for FullCacheStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // ...
    }
}
// EOF
```

**Phase 4: Verification**

```bash
# Run only the moved tests
cargo test --test shared_cache_test

# Run all tests to ensure nothing broke
cargo test

# Check coverage (should be identical)
cargo tarpaulin --out Stdout

# Verify no clippy warnings
cargo clippy --all-targets --all-features
```

### Architecture Changes

**Before**:
```
src/cache/shared_cache.rs (2196 lines)
├── use statements
├── SharedCache struct
├── Implementation (76 functions)
├── CacheStats, FullCacheStats
└── #[cfg(test)] mod tests {
    └── 22 test functions (818 lines)
}
```

**After**:
```
src/cache/shared_cache.rs (1377 lines)
├── use statements
├── SharedCache struct
├── Implementation (76 functions)
└── CacheStats, FullCacheStats

tests/cache/shared_cache_test.rs (830 lines)
├── use debtmap::cache::*
└── 22 test functions (moved from src/)
```

**Benefits**:
- ✅ Source file 37% smaller (1377 vs 2196 lines)
- ✅ Follows Rust convention (integration tests in `tests/`)
- ✅ Better file organization
- ✅ Easier navigation (implementation not mixed with tests)

### Data Structures

No changes to data structures. Tests access public API only.

### APIs and Interfaces

**No API Changes**. Tests use existing public interface:
```rust
impl SharedCache {
    pub fn new(repo_path: Option<&Path>) -> Result<Self>;
    pub fn new_with_cache_dir(...) -> Result<Self>;
    pub fn get(&self, ...) -> Result<Vec<u8>>;
    pub fn put(&self, ...) -> Result<()>;
    // ... all existing public methods
}
```

## Dependencies

- **Prerequisites**: None
- **Affected Components**:
  - `src/cache/shared_cache.rs` (remove tests)
  - `tests/cache/shared_cache_test.rs` (new file)
- **External Dependencies**: None (tests use existing `tempfile` crate)

## Testing Strategy

### Pre-Migration Validation

```bash
# Capture baseline
cargo test 2>&1 | tee /tmp/tests-before.log
cargo tarpaulin --out Stdout 2>&1 | tee /tmp/coverage-before.log
```

### Post-Migration Validation

```bash
# Run moved tests
cargo test --test shared_cache_test

# Verify all tests still pass
cargo test 2>&1 | tee /tmp/tests-after.log

# Verify coverage unchanged
cargo tarpaulin --out Stdout 2>&1 | tee /tmp/coverage-after.log

# Compare results
diff /tmp/tests-before.log /tmp/tests-after.log
diff /tmp/coverage-before.log /tmp/coverage-after.log
```

### Success Criteria

- [ ] All 22 tests pass in new location
- [ ] Total test count unchanged
- [ ] Coverage percentage identical (±0.1%)
- [ ] No new clippy warnings
- [ ] `cargo build` succeeds without warnings

## Documentation Requirements

### Code Documentation

Add comment to `src/cache/shared_cache.rs`:

```rust
/// Thread-safe shared cache implementation
///
/// # Testing
///
/// Integration tests are located in `tests/cache/shared_cache_test.rs`.
/// Run with: `cargo test --test shared_cache_test`
pub struct SharedCache {
    // ...
}
```

### User Documentation

Update `CLAUDE.md` if it mentions test organization:

```markdown
## Testing Guidelines

### Test Organization

- **Unit tests**: Co-locate with implementation using `#[cfg(test)]`
- **Integration tests**: Place in `tests/` directory
- **Distinction**:
  - Unit tests: Test individual functions in isolation
  - Integration tests: Test full workflows, use TempDir, external resources

### Example: SharedCache

```
src/cache/shared_cache.rs    - Implementation
tests/cache/shared_cache_test.rs - Integration tests
```
```

### Architecture Updates

No changes to `ARCHITECTURE.md` needed. This is internal reorganization only.

## Implementation Notes

### Gotchas

1. **Module Visibility**: Integration tests cannot access `pub(crate)` items
   - If tests fail due to visibility, either:
     - Make the item `pub` (if it should be public)
     - Keep as unit test if it requires `pub(crate)` access

2. **Relative Paths**: Integration tests cannot use `super::*`
   - Use `use debtmap::cache::shared_cache::*` instead

3. **Environment Variables**: Tests set `DEBTMAP_CACHE_DIR` env vars
   - Ensure cleanup in test teardown to avoid interference

4. **Parallel Execution**: Integration tests may run in parallel
   - Verify tests don't share mutable state
   - Each test uses isolated `TempDir`

### Best Practices

1. **Atomic Migration**: Move all tests in single commit
2. **Preserve Git History**: Use `git mv` if possible (though test code is new file)
3. **Run Tests First**: Ensure all tests pass before migration
4. **Incremental Verification**: Test after each phase

### Migration Script

```bash
#!/bin/bash
set -e

echo "=== Phase 1: Pre-migration verification ==="
cargo test
cargo clippy --all-targets

echo "=== Phase 2: Create test directory ==="
mkdir -p tests/cache

echo "=== Phase 3: Extract test module ==="
# Extract lines 1378-2196 to new file
sed -n '1378,2196p' src/cache/shared_cache.rs > /tmp/tests.rs

# Transform module to integration test
# - Remove #[cfg(test)] and mod tests {
# - Add proper imports
# - Remove closing }

echo "=== Phase 4: Create integration test file ==="
cat > tests/cache/shared_cache_test.rs << 'EOF'
use debtmap::cache::shared_cache::{SharedCache, CacheStats, FullCacheStats};
use debtmap::cache::index_manager::CacheMetadata;
use debtmap::cache::pruning::PruningConfig;
use std::collections::HashMap;
use std::time::{Duration, SystemTime};
use tempfile::TempDir;

EOF

# Append test functions (manual step - remove module wrapper)

echo "=== Phase 5: Remove old tests ==="
# Keep lines 1-1377 only
sed -i.bak '1378,2196d' src/cache/shared_cache.rs

echo "=== Phase 6: Verify ==="
cargo test --test shared_cache_test
cargo test
cargo clippy --all-targets

echo "=== Migration complete! ==="
```

## Migration and Compatibility

### Breaking Changes

**None**. This is purely internal reorganization. Public API unchanged.

### Migration Path

**Single-step migration**:
1. Create `tests/cache/shared_cache_test.rs` with all tests
2. Remove `#[cfg(test)] mod tests` from `src/cache/shared_cache.rs`
3. Commit both changes together
4. Verify with `cargo test`

### Compatibility Considerations

- **Cargo.toml**: No changes needed (integration tests auto-discovered)
- **CI/CD**: No changes needed (`cargo test` runs all tests)
- **Coverage Tools**: Should automatically include integration tests
- **IDE Support**: Most IDEs recognize `tests/` directory

### Rollback Plan

If issues arise:
```bash
git revert <migration-commit-sha>
cargo test  # Verify rollback successful
```

## Success Metrics

- [ ] File size reduced from 2196 → 1377 lines (37% reduction)
- [ ] All 22 tests pass in new location
- [ ] Zero functional changes
- [ ] God object detector shows lower line count
- [ ] Improved code navigation (implementation separate from tests)

## Timeline Estimate

- **Phase 1** (Create structure): 5 minutes
- **Phase 2** (Copy tests): 10 minutes
- **Phase 3** (Update imports): 15 minutes
- **Phase 4** (Remove old tests): 5 minutes
- **Phase 5** (Verification): 10 minutes
- **Total**: ~45 minutes

## Alternative Approaches Considered

### Alternative 1: Keep Tests Co-located

**Pros**:
- Current Rust convention for unit tests
- Zero migration effort

**Cons**:
- File remains large (2196 lines)
- Inflates god object metrics
- Tests are actually integration tests (use TempDir, full workflows)

**Decision**: Rejected. Tests are integration tests, should be in `tests/`.

### Alternative 2: Split into Multiple Test Files

**Pros**:
- Could group by functionality (cleanup tests, stats tests, etc.)

**Cons**:
- Over-engineering for 22 tests
- Adds unnecessary complexity
- Harder to discover related tests

**Decision**: Rejected. Single test file is sufficient.

### Alternative 3: Leave Until Specs 117-118 Implemented

**Pros**:
- Specs 117-118 will fix false positive algorithmically
- No migration needed

**Cons**:
- Still doesn't follow Rust convention
- File remains harder to navigate

**Decision**: Keep as option. This spec is low priority and can be deferred.

## Recommendation

**Implement after Specs 117-118** are complete. This is a nice-to-have organizational improvement, not a critical fix. The false positive issue is better solved algorithmically (complexity weighting + purity analysis) than by moving tests.

**Priority**: Low (cosmetic improvement)
**Effort**: Low (~45 minutes)
**Risk**: Very low (pure reorganization, no logic changes)
