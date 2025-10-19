---
number: 119
title: Separate SharedCache Unit Tests into Dedicated File
category: testing
priority: low
status: draft
dependencies: []
created: 2025-10-18
---

# Specification 119: Separate SharedCache Unit Tests into Dedicated File

**Category**: testing
**Priority**: low
**Status**: draft
**Dependencies**: None

## Context

The `shared_cache.rs` file contains 818 lines of unit tests (37% of the 2196 total lines), which contributes to it being flagged as a potential god object. While the implementation itself is well-designed with proper functional decomposition, the co-located tests inflate the file size metrics.

**Current Structure**:
```
src/cache/shared_cache.rs (2196 lines)
├── Implementation: lines 1-1377 (63%)
└── Tests (#[cfg(test)]): lines 1378-2196 (37%)
    ├── 22 unit test functions
    ├── Tests private helper methods
    └── Tests public API workflows
```

**Why Tests Can't Move to `tests/`**:
The tests access **private implementation details**:
```rust
// Tests private static methods
SharedCache::calculate_max_age_duration(0);      // Private!
SharedCache::select_keys_for_removal(...);        // Private!
SharedCache::should_prune_after_insertion(...);   // Private!

// Tests internal implementation
cache.location.ensure_directories().unwrap();     // Field access
```

These are **unit tests**, not integration tests. They must remain in the `src/` tree to access private methods.

**Impact on Metrics**:
- Line count: 2196 (inflated by 818 test lines)
- Contributes to false positive god object detection
- Makes file harder to navigate (must scroll past 1377 lines to reach tests)

**Why This Matters**:
- Rust convention: Large modules can split tests into separate files
- Unit tests can be in `module_name/tests.rs` instead of inline
- Separating reduces noise in god object detection
- Improves file organization and discoverability

**Not a High Priority**:
This is a **minor organizational improvement**, not a fundamental design issue. The code is well-designed regardless of test location. Priority is low because:
- Specs 117-118 will fix the false positive issue algorithmically
- This is cosmetic cleanup, not architectural improvement
- No functional benefit, purely organizational

## Objective

Refactor `src/cache/shared_cache.rs` into a module directory with implementation in `mod.rs` and unit tests in `tests.rs`, reducing the main implementation file to ~1377 lines while preserving all test functionality and private method access.

## Requirements

### Functional Requirements

1. **Convert File to Module Directory**
   - Rename `src/cache/shared_cache.rs` → `src/cache/shared_cache/mod.rs`
   - Create `src/cache/shared_cache/tests.rs` for unit tests
   - Move all 22 test functions from `#[cfg(test)] mod tests { ... }`
   - Preserve all test functionality exactly as-is
   - Maintain access to private methods via `use super::*;`

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

3. **Preserve Test Capabilities**
   - Tests remain `#[cfg(test)]` - still unit tests
   - Can access private methods via `super::*`
   - Can test private static functions
   - Can access struct fields directly
   - All tests must pass after migration
   - Coverage metrics should remain identical

### Non-Functional Requirements

1. **Zero Functional Changes**: Tests run identically before and after
2. **Backward Compatibility**: No API changes to `SharedCache`
3. **Module Import**: `use crate::cache::shared_cache::*` still works
4. **Build Performance**: No impact on compilation times
5. **Documentation**: Update comments if needed

## Acceptance Criteria

- [ ] `src/cache/shared_cache/` directory created
- [ ] `src/cache/shared_cache/mod.rs` created with implementation (1377 lines)
- [ ] `src/cache/shared_cache/tests.rs` created with all 22 tests (~830 lines)
- [ ] Original `src/cache/shared_cache.rs` deleted
- [ ] All 22 tests pass: `cargo test shared_cache`
- [ ] Tests can access private methods (verified by tests passing)
- [ ] Full test suite passes: `cargo test`
- [ ] Code coverage unchanged (run `cargo tarpaulin` before/after)
- [ ] God object detection shows reduced line count for mod.rs
- [ ] No clippy warnings introduced
- [ ] External imports still work: `use debtmap::cache::shared_cache::SharedCache`

## Technical Details

### Implementation Approach

**Phase 1: Create Module Directory Structure**

```bash
# Create module directory
mkdir -p src/cache/shared_cache

# Move implementation to mod.rs
mv src/cache/shared_cache.rs src/cache/shared_cache/mod.rs
```

**Phase 2: Extract Tests to Separate File**

```rust
// src/cache/shared_cache/tests.rs

#![cfg(test)]

use super::*;  // Access private methods from mod.rs
use std::collections::HashMap;
use std::time::{Duration, SystemTime};
use tempfile::TempDir;

#[test]
fn test_shared_cache_operations() {
    let temp_dir = TempDir::new().unwrap();

    std::env::set_var("DEBTMAP_CACHE_DIR", temp_dir.path().to_str().unwrap());
    std::env::set_var("DEBTMAP_CACHE_AUTO_PRUNE", "false");

    let cache = SharedCache::new_with_cache_dir(None, temp_dir.path().to_path_buf()).unwrap();

    // ... rest of test (exact copy from original)
}

#[test]
fn test_age_calculation_pure_functions() {
    // CAN ACCESS PRIVATE METHODS via super::*
    let max_age_0_days = SharedCache::calculate_max_age_duration(0);
    let max_age_1_day = SharedCache::calculate_max_age_duration(1);

    assert_eq!(max_age_0_days, Duration::from_secs(0));
    assert_eq!(max_age_1_day, Duration::from_secs(86400));

    // ... rest of test
}

// ... all 22 tests (exact copies from original)
```

**Phase 3: Remove Tests from mod.rs**

```rust
// src/cache/shared_cache/mod.rs

// ... all implementation code ...

impl std::fmt::Display for FullCacheStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Cache Statistics:")?;
        writeln!(f, "  Strategy: {:?}", self.strategy)?;
        writeln!(f, "  Location: {}", self.cache_location.display())?;
        writeln!(f, "  Project ID: {}", self.project_id)?;
        writeln!(f, "  Total entries: {}", self.total_entries)?;
        writeln!(f, "  Total size: {} MB", self.total_size / (1024 * 1024))?;
        Ok(())
    }
}

// Add module declaration to include tests
#[cfg(test)]
mod tests;

// EOF (no inline tests)
```

**Phase 4: Verification**

```bash
# Run only shared_cache tests
cargo test shared_cache

# Run all tests to ensure nothing broke
cargo test

# Check coverage (should be identical)
cargo tarpaulin --out Stdout | grep shared_cache

# Verify no clippy warnings
cargo clippy --all-targets --all-features
```

### Architecture Changes

**Before**:
```
src/cache/
├── shared_cache.rs (2196 lines)
│   ├── Implementation (1377 lines)
│   └── #[cfg(test)] mod tests { ... } (818 lines)
├── index_manager.rs
├── auto_pruner.rs
└── ...
```

**After**:
```
src/cache/
├── shared_cache/
│   ├── mod.rs (1385 lines - implementation + mod tests; declaration)
│   └── tests.rs (830 lines - #[cfg(test)] unit tests)
├── index_manager.rs
├── auto_pruner.rs
└── ...
```

**Benefits**:
- ✅ Main implementation file 37% smaller (1385 vs 2196 lines)
- ✅ Follows Rust convention for large modules
- ✅ Better file organization (implementation separate from tests)
- ✅ Easier navigation (no 1400-line scroll to find tests)
- ✅ Tests retain full access to private implementation
- ✅ External API unchanged (`use debtmap::cache::shared_cache::*` still works)

### Data Structures

No changes to data structures. Tests access same private and public methods.

### APIs and Interfaces

**No API Changes**. Module structure changes but public interface identical:

```rust
// External usage - UNCHANGED
use debtmap::cache::shared_cache::{SharedCache, CacheStats, FullCacheStats};

let cache = SharedCache::new(repo_path)?;
cache.put("key", "component", data)?;
```

**Internal Module Structure**:
```rust
// mod.rs exports everything as before
pub struct SharedCache { ... }
pub struct CacheStats { ... }
pub struct FullCacheStats { ... }

// tests.rs imports from parent module
use super::*;  // Gets everything from mod.rs
```

## Dependencies

- **Prerequisites**: None
- **Affected Components**:
  - `src/cache/shared_cache.rs` → `src/cache/shared_cache/mod.rs`
  - New: `src/cache/shared_cache/tests.rs`
- **External Dependencies**: None
- **Import Changes**: None (module path unchanged)

## Testing Strategy

### Pre-Migration Validation

```bash
# Capture baseline
cargo test shared_cache 2>&1 | tee /tmp/tests-before.log
cargo tarpaulin --out Stdout 2>&1 | grep -A 20 "shared_cache" | tee /tmp/coverage-before.log
```

### Post-Migration Validation

```bash
# Run moved tests
cargo test shared_cache 2>&1 | tee /tmp/tests-after.log

# Verify all tests still pass
cargo test

# Verify coverage unchanged
cargo tarpaulin --out Stdout 2>&1 | grep -A 20 "shared_cache" | tee /tmp/coverage-after.log

# Compare results
diff /tmp/tests-before.log /tmp/tests-after.log
diff /tmp/coverage-before.log /tmp/coverage-after.log

# Verify external imports work
cargo check --all-targets
```

### Success Criteria

- [ ] All 22 tests pass in new location
- [ ] Total test count unchanged
- [ ] Coverage percentage identical (±0.1%)
- [ ] No new clippy warnings
- [ ] `cargo build` succeeds without warnings
- [ ] Private method tests still work (e.g., `calculate_max_age_duration`)

## Documentation Requirements

### Code Documentation

Update comment in `mod.rs`:

```rust
// src/cache/shared_cache/mod.rs

/// Thread-safe shared cache implementation
///
/// # Module Organization
///
/// - Implementation: `src/cache/shared_cache/mod.rs` (this file)
/// - Unit tests: `src/cache/shared_cache/tests.rs`
///
/// # Testing
///
/// Run unit tests with: `cargo test shared_cache`
pub struct SharedCache {
    // ...
}
```

### User Documentation

No user-facing documentation changes needed. This is internal reorganization.

### Architecture Updates

Optional update to `ARCHITECTURE.md`:

```markdown
### Cache Module

**Location**: `src/cache/`

**SharedCache** (`src/cache/shared_cache/`):
- `mod.rs` - Main implementation (~1377 lines)
- `tests.rs` - Unit tests (~830 lines, #[cfg(test)])

The shared cache follows Rust convention of separating large test modules
into dedicated files while maintaining access to private implementation.
```

## Implementation Notes

### Gotchas

1. **Module Declaration**: Must add `#[cfg(test)] mod tests;` to end of `mod.rs`
   - This makes `tests.rs` part of the module
   - Tests compile only in test mode
   - Tests can use `super::*` to access private items

2. **File-Level `#[cfg(test)]`**: The `tests.rs` file needs `#![cfg(test)]` at the top
   - This is different from inline `#[cfg(test)] mod tests { }`
   - Entire file is conditionally compiled for tests

3. **Git Move**: Use `git mv` to preserve file history
   ```bash
   git mv src/cache/shared_cache.rs src/cache/shared_cache/mod.rs
   # Then create tests.rs with extracted tests
   ```

4. **Import Paths**: External imports don't change
   - `use debtmap::cache::shared_cache::SharedCache` still works
   - Rust treats `module_name/mod.rs` identically to `module_name.rs`

### Best Practices

1. **Atomic Commit**: All changes in single commit
2. **Preserve History**: Use `git mv` for mod.rs
3. **Test First**: Run tests before migration to establish baseline
4. **Incremental Verification**: Test after each phase

### Migration Script

```bash
#!/bin/bash
set -e

echo "=== Phase 1: Pre-migration verification ==="
cargo test shared_cache
TEST_COUNT=$(cargo test shared_cache 2>&1 | grep "test result" | awk '{print $4}')
echo "Baseline: $TEST_COUNT tests passing"

echo "=== Phase 2: Create module directory ==="
mkdir -p src/cache/shared_cache

echo "=== Phase 3: Move implementation to mod.rs ==="
git mv src/cache/shared_cache.rs src/cache/shared_cache/mod.rs

echo "=== Phase 4: Extract tests to tests.rs ==="
# Extract test module (lines 1378-2196)
sed -n '1378,2196p' src/cache/shared_cache/mod.rs > /tmp/test_content.rs

# Create tests.rs with proper header
cat > src/cache/shared_cache/tests.rs << 'EOF'
#![cfg(test)]

use super::*;
use std::collections::HashMap;
use std::time::{Duration, SystemTime};
use tempfile::TempDir;

EOF

# Remove #[cfg(test)] mod tests { wrapper and closing }
# Keep test functions
sed '1d; $d' /tmp/test_content.rs | sed '1,4d' >> src/cache/shared_cache/tests.rs

echo "=== Phase 5: Remove tests from mod.rs ==="
# Keep lines 1-1377, delete the rest
sed -i.bak '1378,2196d' src/cache/shared_cache/mod.rs

# Add module declaration at end of mod.rs
echo "" >> src/cache/shared_cache/mod.rs
echo "#[cfg(test)]" >> src/cache/shared_cache/mod.rs
echo "mod tests;" >> src/cache/shared_cache/mod.rs

echo "=== Phase 6: Verify ==="
cargo test shared_cache
NEW_TEST_COUNT=$(cargo test shared_cache 2>&1 | grep "test result" | awk '{print $4}')

if [ "$TEST_COUNT" = "$NEW_TEST_COUNT" ]; then
    echo "✅ Test count matches: $TEST_COUNT"
else
    echo "❌ Test count changed: $TEST_COUNT → $NEW_TEST_COUNT"
    exit 1
fi

cargo clippy --all-targets --all-features

echo "=== Phase 7: Cleanup ==="
rm src/cache/shared_cache/mod.rs.bak
rm /tmp/test_content.rs

echo "=== Migration complete! ==="
echo "Commit changes with: git add src/cache/shared_cache/"
```

## Migration and Compatibility

### Breaking Changes

**None**. This is purely internal reorganization. Public API unchanged.

External code continues to work without modification:
```rust
use debtmap::cache::shared_cache::SharedCache;  // Still works
```

### Migration Path

**Single-step migration**:
1. Move `shared_cache.rs` → `shared_cache/mod.rs`
2. Extract tests to `shared_cache/tests.rs`
3. Add `#[cfg(test)] mod tests;` to `mod.rs`
4. Delete test block from `mod.rs`
5. Commit all changes together
6. Verify with `cargo test shared_cache`

### Compatibility Considerations

- **Module Path**: Unchanged - `debtmap::cache::shared_cache`
- **Public API**: Identical exports
- **Cargo.toml**: No changes needed
- **CI/CD**: No changes needed
- **Coverage Tools**: Should automatically track both files
- **IDE Support**: Most IDEs recognize module directory pattern

### Rollback Plan

If issues arise:
```bash
# Rollback is simple - reverse the file moves
git revert <migration-commit-sha>
cargo test shared_cache  # Verify rollback successful
```

## Success Metrics

- [ ] Main file reduced from 2196 → ~1385 lines (37% reduction)
- [ ] All 22 tests pass in new location
- [ ] Zero functional changes
- [ ] Private method tests still work
- [ ] God object detector shows lower line count for mod.rs
- [ ] Improved code navigation (implementation separate from tests)
- [ ] External imports unchanged

## Timeline Estimate

- **Phase 1** (Verify baseline): 5 minutes
- **Phase 2** (Create directory): 1 minute
- **Phase 3** (Move to mod.rs): 2 minutes
- **Phase 4** (Extract tests): 15 minutes
- **Phase 5** (Update mod.rs): 5 minutes
- **Phase 6** (Verification): 10 minutes
- **Total**: ~40 minutes

## Alternative Approaches Considered

### Alternative 1: Keep Tests Co-located (Current State)

**Pros**:
- Zero migration effort
- Standard Rust pattern for small modules

**Cons**:
- File remains large (2196 lines)
- Inflates god object metrics
- Harder to navigate

**Decision**: Rejected for large modules. Rust convention allows splitting tests for modules >1000 lines.

### Alternative 2: Move to Integration Tests (`tests/` directory)

**Pros**:
- Cleaner separation

**Cons**:
- ❌ **Cannot access private methods** - tests would break
- ❌ Tests call `SharedCache::calculate_max_age_duration()` (private static)
- ❌ Tests call `SharedCache::select_keys_for_removal()` (private)
- ❌ Would need to make methods `pub` or rewrite tests

**Decision**: Rejected. Tests require access to private implementation.

### Alternative 3: Split into Multiple Test Files by Topic

**Pros**:
- Could group by functionality (cleanup, stats, pruning, etc.)

**Cons**:
- Over-engineering for 22 tests
- Adds unnecessary complexity
- Harder to discover related tests

**Decision**: Rejected. Single test file is sufficient.

### Alternative 4: Leave Until Specs 117-118 Implemented

**Pros**:
- Specs 117-118 will fix false positive algorithmically
- No migration needed

**Cons**:
- File remains harder to navigate
- Doesn't follow Rust convention for large modules

**Decision**: Keep as option. This spec is low priority and can be deferred.

## Recommendation

**Implement after Specs 117-118** are complete. This is a nice-to-have organizational improvement, not a critical fix. The false positive issue is better solved algorithmically (complexity weighting + purity analysis) than by reorganizing files.

**Priority**: Low (cosmetic improvement)
**Effort**: Low (~40 minutes)
**Risk**: Very low (pure reorganization, preserves all functionality)
**Benefits**: Better navigation, reduced file size, follows Rust conventions
