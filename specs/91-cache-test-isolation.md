---
number: 91
title: Cache Integration Test Isolation
category: testing
priority: high
status: draft
dependencies: []
created: 2025-09-03
---

# Specification 91: Cache Integration Test Isolation

**Category**: testing
**Priority**: high
**Status**: draft
**Dependencies**: None

## Context

The cache integration tests in `tests/cache_integration.rs` currently experience race conditions when run in parallel. Tests that pass when executed sequentially with `--test-threads=1` fail when run in parallel, specifically:
- `test_analysis_cache_uses_shared_backend`
- `test_cache_migration_from_local`

This occurs because multiple tests share the same cache directories when the `DEBTMAP_CACHE_DIR` environment variable is set, causing interference between simultaneously running tests. The current test setup uses a simple `TempDir` for cache directories, but this doesn't provide sufficient isolation when tests manipulate the same project IDs or cache paths.

Running tests sequentially is not a sustainable solution as it:
- Significantly slows down the test suite
- Prevents effective use of multi-core processors
- Makes CI/CD pipelines slower and more expensive
- Reduces developer productivity during local testing

## Objective

Implement proper test isolation for cache integration tests to enable reliable parallel execution without race conditions or shared state interference, while maintaining test clarity and performance.

## Requirements

### Functional Requirements
- Each test must have its own isolated cache directory structure
- Tests must not interfere with each other when run in parallel
- Environment variables must be isolated per test
- Project IDs must be unique per test to prevent cache key collisions
- Test cleanup must be reliable and not affect other tests
- Cache location resolution must be deterministic within each test
- Tests must pass consistently regardless of execution order

### Non-Functional Requirements
- No performance degradation compared to current sequential execution
- Test code should remain readable and maintainable
- Minimal changes to existing test logic
- Must work on all supported platforms (Linux, macOS, Windows)
- No leftover test artifacts after test completion
- Clear error messages when isolation fails

## Acceptance Criteria

- [ ] All cache integration tests pass when run with default parallel execution
- [ ] Tests pass consistently across multiple runs (no flaky failures)
- [ ] No test artifacts remain in temp directories after test completion
- [ ] Each test uses a unique cache directory verified by assertion
- [ ] Project IDs are unique per test execution
- [ ] Environment variable changes in one test don't affect others
- [ ] Tests can be debugged individually without special configuration
- [ ] Performance is equal or better than sequential execution
- [ ] CI pipeline runs cache tests in parallel by default

## Technical Details

### Implementation Approach

1. **Unique Test Identifiers**
   - Generate unique test IDs using a combination of:
     - Test function name
     - Thread ID
     - Timestamp or UUID
   - Use these IDs to create isolated namespaces

2. **Directory Isolation**
   - Create unique temp directories per test with guaranteed uniqueness
   - Structure: `/tmp/debtmap-test-{test_id}/cache`
   - Ensure directories are created with proper permissions

3. **Environment Variable Isolation**
   - Use thread-local storage for environment variables where possible
   - Implement a test-specific environment wrapper that:
     - Captures current environment state
     - Applies test-specific overrides
     - Restores original state on drop
   - Consider using `serial_test` crate for tests that must modify global state

4. **Project ID Uniqueness**
   - Override project ID generation in tests
   - Use test name + unique suffix for project IDs
   - Ensure cache paths include test-specific components

### Architecture Changes

1. **Test Helper Module**
   ```rust
   // tests/helpers/cache_isolation.rs
   pub struct IsolatedCacheTest {
       test_id: String,
       cache_dir: TempDir,
       project_dir: TempDir,
       env_guard: EnvGuard,
   }
   
   impl IsolatedCacheTest {
       pub fn new(test_name: &str) -> Self { ... }
       pub fn cache_path(&self) -> &Path { ... }
       pub fn project_path(&self) -> &Path { ... }
   }
   ```

2. **Enhanced EnvGuard**
   - Make EnvGuard thread-safe using Mutex
   - Add scoped environment modifications
   - Implement proper cleanup on panic

### Data Structures

```rust
// Test isolation context
pub struct TestContext {
    id: String,
    cache_dir: PathBuf,
    project_dir: PathBuf,
    original_env: HashMap<String, Option<String>>,
}

// Thread-safe environment manager
pub struct IsolatedEnv {
    modifications: Arc<Mutex<HashMap<String, Option<String>>>>,
}
```

### APIs and Interfaces

```rust
// New test helper functions
pub fn with_isolated_cache<F, R>(test_name: &str, f: F) -> R 
where
    F: FnOnce(&TestContext) -> R;

pub fn create_isolated_test_env(test_name: &str) -> (TempDir, TempDir, EnvGuard);

pub fn ensure_unique_project_id(base: &str) -> String;
```

## Dependencies

- **Prerequisites**: None
- **Affected Components**: 
  - `tests/cache_integration.rs` - All test functions
  - `src/cache/cache_location.rs` - May need test-mode awareness
  - `src/cache/shared_cache.rs` - May need test helper methods
- **External Dependencies**: 
  - Consider adding `serial_test` crate for tests requiring serialization
  - Consider `uuid` crate for unique ID generation

## Testing Strategy

- **Unit Tests**: 
  - Test isolation helper functions work correctly
  - Verify unique ID generation
  - Test environment isolation
  - Validate cleanup behavior

- **Integration Tests**: 
  - Run all cache tests in parallel multiple times
  - Verify no interference between tests
  - Check for resource leaks
  - Stress test with many parallel tests

- **Performance Tests**: 
  - Measure test execution time in parallel vs sequential
  - Monitor resource usage during parallel execution
  - Ensure no performance regression

- **Platform Testing**:
  - Verify on Linux, macOS, and Windows
  - Test with different filesystem types
  - Verify temp directory cleanup

## Documentation Requirements

- **Code Documentation**: 
  - Document isolation strategy in test module
  - Add comments explaining test helper usage
  - Document any platform-specific behavior

- **Test Documentation**: 
  - Update test README with isolation approach
  - Document how to debug isolated tests
  - Explain environment variable handling

- **Developer Guide**:
  - Best practices for writing isolated cache tests
  - Common pitfalls and how to avoid them
  - Debugging techniques for parallel tests

## Implementation Notes

1. **Gradual Migration**
   - Start with the failing tests
   - Migrate other cache tests incrementally
   - Keep backward compatibility during migration

2. **Error Handling**
   - Clear error messages when isolation fails
   - Include test name in all error contexts
   - Log cache paths for debugging

3. **Platform Considerations**
   - Windows may have different temp directory behavior
   - File locking semantics vary by platform
   - Path length limitations on some systems

4. **Performance Optimizations**
   - Reuse temp directory creation where safe
   - Lazy initialization of test resources
   - Parallel directory cleanup after tests

5. **Debugging Support**
   - Environment variable to keep test artifacts
   - Verbose mode to log isolation details
   - Test replay capability with saved state

## Migration and Compatibility

During prototype phase, breaking changes to test infrastructure are acceptable. Focus on:
- Correct isolation over backward compatibility
- Clean, maintainable test code
- Performance and reliability
- Clear migration path for existing tests

Tests can be migrated incrementally:
1. Fix currently failing tests first
2. Migrate high-value tests next
3. Convert remaining tests as time permits
4. Remove old test helpers once migration complete