# Implementation Plan: Refactor SharedCache God Object

## Problem Summary

**Location**: ./src/cache/shared_cache/mod.rs:file:0
**Priority Score**: 59.94
**Debt Type**: God Object (GodClass)

**Current Metrics**:
- Lines of Code: 1201
- Functions: 69 (61 impl methods, 4 module-level functions, 2 trait methods)
- Cyclomatic Complexity: 226 (avg 3.28, max 13)
- Coverage: 0.0%
- Responsibilities: 8 (Data Access, Utilities, Construction, Processing, Persistence, Computation, Filtering & Selection, Validation)
- God Object Score: 1.0 (confirmed god object)

**Issue**: The SharedCache struct has 61 methods handling 8 different responsibilities, making it difficult to maintain, test, and reason about. The debtmap analysis recommends splitting into focused modules with <30 functions each, organized by data flow: 1) Input/parsing functions 2) Core logic/transformation 3) Output/formatting.

## Target State

**Expected Impact** (from debtmap):
- Complexity Reduction: 45.2 points
- Maintainability Improvement: 5.99 points
- Test Effort: 120.1 hours

**Success Criteria**:
- [ ] Separate pruning logic into dedicated module (~40 methods)
- [ ] Extract construction/initialization logic into builder module (~6 methods)
- [ ] Reduce SharedCache impl to core coordination methods (<25 methods)
- [ ] All existing tests continue to pass
- [ ] No clippy warnings
- [ ] Proper formatting with `cargo fmt`
- [ ] Each extracted module has clear, single responsibility
- [ ] Public API remains unchanged (backward compatibility)

## Implementation Phases

### Phase 1: Extract Pruning Logic Module

**Goal**: Extract all pruning-related methods (40 methods) into a dedicated `pruning.rs` submodule.

**Changes**:
- Create `src/cache/shared_cache/pruning.rs` module
- Extract pure pruning functions:
  - `calculate_cache_projections`
  - `should_prune_based_on_projections`
  - `calculate_pruning_decision`
  - `should_remove_entry_by_age`
  - `filter_entries_by_age`
  - `calculate_max_age_duration`
  - `should_prune_after_insertion`
- Extract pruning decision functions:
  - `determine_pruning_config`
  - `determine_pruning_strategy`
  - `should_perform_post_insertion_pruning`
- Extract pruning execution methods (keep &self references):
  - `trigger_pruning_if_needed`
  - `trigger_pruning_if_needed_with_new_entry`
  - `trigger_pruning`
  - `execute_sync_pruning`
  - `execute_pruning_strategy`
  - `handle_pre_insertion_pruning`
  - `handle_post_insertion_pruning`
  - `execute_post_insertion_check`
  - `prune_with_strategy`
  - `clean_orphaned_entries`
  - `cleanup_old_entries`
- Extract pruning helper methods:
  - `calculate_entries_to_prune`
  - `remove_entries_from_index`
  - `delete_pruned_files`
  - `execute_fallback_cleanup`
  - `log_post_insertion_debug`
  - `log_config_if_test_environment`
- Extract stats creation methods:
  - `create_no_prune_stats`
  - `create_empty_prune_stats`
  - `create_prune_stats`
- Update `mod.rs` to:
  - Add `mod pruning;` declaration
  - Delegate to pruning module methods
  - Keep public API surface unchanged

**Testing**:
- Run `cargo test --lib` to verify existing tests pass
- Run `cargo clippy` to check for warnings
- Verify all pruning-related tests still work

**Success Criteria**:
- [ ] New `pruning.rs` module created with ~40 methods
- [ ] Pure functions (no &self) are top-level functions in pruning module
- [ ] Methods needing cache access accept SharedCache reference
- [ ] SharedCache delegates to pruning module
- [ ] All tests pass
- [ ] No clippy warnings
- [ ] Code compiles successfully

### Phase 2: Extract Builder/Construction Module

**Goal**: Extract construction and builder methods into a dedicated `builder.rs` submodule.

**Changes**:
- Create `src/cache/shared_cache/builder.rs` module
- Create `SharedCacheBuilder` struct with builder pattern
- Move construction methods to builder:
  - `new` -> `SharedCacheBuilder::build()`
  - `new_with_cache_dir` -> `SharedCacheBuilder::with_cache_dir()`
  - `new_with_location` -> `SharedCacheBuilder::with_location()`
  - `with_auto_pruning` -> `SharedCacheBuilder::with_auto_pruning()`
  - `with_auto_pruning_and_cache_dir` -> `SharedCacheBuilder::with_auto_pruning_and_cache_dir()`
- Implement builder methods:
  - `new()` - Start building
  - `cache_dir()` - Set custom cache directory
  - `auto_pruner()` - Configure auto-pruning
  - `max_cache_size()` - Set max size
  - `cleanup_threshold()` - Set cleanup threshold
  - `build()` - Construct SharedCache
- Update `SharedCache` to:
  - Keep `new()` as public API (delegates to builder)
  - Re-export builder for advanced use cases
- Update `mod.rs`:
  - Add `mod builder;` declaration
  - Add `pub use builder::SharedCacheBuilder;`
  - Update existing constructors to use builder internally

**Testing**:
- Run `cargo test --lib` to verify existing tests pass
- Verify all construction patterns still work
- Test builder pattern explicitly

**Success Criteria**:
- [ ] New `builder.rs` module created
- [ ] `SharedCacheBuilder` implements builder pattern
- [ ] All construction methods preserved (backward compatible)
- [ ] Builder provides flexible construction API
- [ ] All tests pass
- [ ] No clippy warnings
- [ ] Code compiles successfully

### Phase 3: Extract File Operations Module

**Goal**: Extract file management and cleanup methods into `file_ops.rs` submodule.

**Changes**:
- Create `src/cache/shared_cache/file_ops.rs` module
- Move file operation functions:
  - `classify_entry` (already module-level)
  - `build_dest_path` (already module-level)
  - `copy_file_entry` (already module-level)
  - `copy_dir_entry` (already module-level)
  - `get_cache_file_path` (make it accept `&CacheLocation`)
- Move file management methods:
  - `delete_cache_files`
  - `delete_cache_files_for_keys`
  - `delete_component_file`
  - `copy_dir_recursive`
  - `clear_component_files`
- Move cleanup methods:
  - `maybe_cleanup`
  - `cleanup`
  - `determine_keys_to_remove`
  - `select_keys_for_removal`
- Update `mod.rs`:
  - Add `mod file_ops;` declaration
  - Delegate to file_ops module
  - Re-export if needed for tests

**Testing**:
- Run `cargo test --lib` to verify tests pass
- Verify file operations still work correctly
- Check migration logic still functions

**Success Criteria**:
- [ ] New `file_ops.rs` module created
- [ ] File operations extracted and organized
- [ ] Cleanup logic extracted
- [ ] SharedCache delegates to file_ops
- [ ] All tests pass
- [ ] No clippy warnings
- [ ] Code compiles successfully

### Phase 4: Simplify Core SharedCache Coordination

**Goal**: Reduce SharedCache to pure coordination logic, delegating to specialized modules.

**Changes**:
- Keep only coordination methods in SharedCache:
  - `get()` - Delegates to reader
  - `put()` - Coordinates pruning + writer
  - `exists()` - Delegates to reader
  - `delete()` - Delegates to writer
  - `compute_cache_key()` - Delegates to reader
  - `get_stats()` - Delegates to index_manager
  - `get_full_stats()` - Delegates to index_manager
  - `save_index()` - Delegates to index_manager
  - `validate_version()` - Delegates to index_manager
  - `clear()` - Coordinates file_ops + index_manager
  - `clear_project()` - Coordinates file_ops + index_manager
  - `migrate_from_local()` - Delegates to file_ops
- Move remaining private helpers to appropriate modules:
  - `is_existing_entry()` -> inline or remove
  - `is_new_entry()` -> inline in pruning module
  - `put_with_config()` -> keep as coordination method
  - `execute_cache_storage()` -> inline into put_with_config
- Update documentation:
  - Add module-level docs explaining architecture
  - Document each submodule's responsibility
  - Add examples for common usage patterns

**Testing**:
- Run full test suite: `cargo test --lib`
- Run clippy: `cargo clippy --all-targets`
- Run formatter: `cargo fmt --all -- --check`
- Verify no functionality regressions

**Success Criteria**:
- [ ] SharedCache reduced to <25 methods
- [ ] Clear separation of concerns across modules
- [ ] All delegation properly implemented
- [ ] Module-level documentation complete
- [ ] All tests pass
- [ ] No clippy warnings
- [ ] Code properly formatted
- [ ] Public API unchanged (backward compatible)

### Phase 5: Add Tests and Final Verification

**Goal**: Ensure test coverage for extracted modules and verify overall quality.

**Changes**:
- Add unit tests for pure functions in pruning module:
  - `calculate_cache_projections` tests
  - `should_prune_based_on_projections` tests
  - `calculate_pruning_decision` tests
  - `should_remove_entry_by_age` tests
  - `filter_entries_by_age` tests
- Add unit tests for builder module:
  - Test various builder configurations
  - Test default values
  - Test error cases
- Add unit tests for file_ops module:
  - Test file classification
  - Test path building
  - Test cleanup logic
- Update integration tests in `tests.rs`:
  - Verify end-to-end workflows still work
  - Test pruning integration
  - Test builder integration
- Run full CI verification:
  - `cargo test --all-features`
  - `cargo clippy --all-targets --all-features`
  - `cargo fmt --all -- --check`
  - `cargo doc --no-deps`

**Testing**:
- Run full test suite with coverage: `cargo tarpaulin`
- Verify coverage improves from 0%
- Run full CI locally: `just ci`

**Success Criteria**:
- [ ] Unit tests added for all pure functions
- [ ] Builder module has comprehensive tests
- [ ] File ops module has tests
- [ ] Integration tests updated and passing
- [ ] Test coverage significantly improved from 0%
- [ ] All CI checks pass
- [ ] Documentation builds without warnings
- [ ] Ready for final commit

## Testing Strategy

**For each phase**:
1. Run `cargo test --lib` after each change
2. Run `cargo clippy` to ensure no warnings
3. Run `cargo fmt` to ensure proper formatting
4. Commit working code incrementally

**Integration testing**:
1. Run existing test suite in `src/cache/shared_cache/tests.rs`
2. Verify all cache operations still work
3. Test pruning behavior with various configurations
4. Test builder patterns

**Final verification**:
1. `just ci` - Full CI checks (build, test, clippy, fmt)
2. `cargo tarpaulin` - Coverage analysis (expect significant improvement from 0%)
3. `cargo doc --no-deps` - Verify documentation builds
4. Manual smoke testing of key workflows

## Rollback Plan

If a phase fails:
1. **Immediate rollback**: `git reset --hard HEAD~1`
2. **Review failure**: Check compiler errors, test failures, clippy warnings
3. **Adjust approach**:
   - If extraction is too aggressive, do smaller incremental changes
   - If tests fail, examine test assumptions and update carefully
   - If API breaks, ensure backward compatibility wrappers exist
4. **Retry**: Make smaller, more focused changes

## Notes

### Key Architectural Decisions

1. **Preserve Public API**: All existing public methods remain available for backward compatibility
2. **Progressive Extraction**: Extract modules one at a time, ensuring tests pass after each
3. **Pure Functions First**: Extract pure functions (no &self) before methods
4. **Delegation Pattern**: SharedCache becomes coordinator, delegating to specialized modules
5. **Module Organization**:
   - `pruning.rs` - All pruning/eviction logic (~40 methods)
   - `builder.rs` - Construction and initialization (~6 methods)
   - `file_ops.rs` - File management and cleanup (~15 methods)
   - `mod.rs` - Core coordination (<25 methods)
   - `reader.rs` - Already exists, handles reads
   - `writer.rs` - Already exists, handles writes

### Challenges and Mitigations

**Challenge**: SharedCache has extensive state (&self references everywhere)
**Mitigation**: Use delegation pattern - methods accept `&SharedCache` or needed fields

**Challenge**: Tests may depend on implementation details
**Mitigation**: Keep public API identical, only change internal organization

**Challenge**: Pruning logic is tightly coupled with cache operations
**Mitigation**: Use dependency injection pattern - pass what's needed to pruning functions

**Challenge**: 0% test coverage makes refactoring risky
**Mitigation**: Add property tests for pure functions as we extract them

### Expected Outcomes

After completion:
- **Maintainability**: 8 responsibilities → 4 focused modules
- **Testability**: Pure functions easily testable, coverage improves from 0%
- **Complexity**: 226 total complexity → distributed across modules (<100 per module)
- **Comprehension**: Each module has clear, single responsibility
- **Lines per file**: 1201 lines → ~300-400 per module (manageable size)

### Related Files to Watch

- `src/cache/shared_cache/tests.rs` - Integration tests
- `src/cache/shared_cache/reader.rs` - Reader module (already refactored)
- `src/cache/shared_cache/writer.rs` - Writer module (already refactored)
- `src/cache/auto_pruner.rs` - Auto-pruning configuration
- `src/cache/index_manager.rs` - Index management
- `src/cache/pruning.rs` - Pruning configuration types

These files should NOT be modified during this refactoring - we're only restructuring the SharedCache module itself.
