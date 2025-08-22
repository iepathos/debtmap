---
number: 61
title: Test Performance Optimization
category: optimization
priority: high
status: implemented
dependencies: []
created: 2025-01-22
---

# Specification 61: Test Performance Optimization

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: None

## Context

The debtmap test suite has grown to over 1800 tests across 160 files, with some test files exceeding 1000 lines. Test execution time has become a bottleneck for development velocity, exceeding the 2-minute timeout limits for automated tools. This impacts developer productivity and CI/CD pipeline efficiency.

Current issues:
- Test suite takes longer than 2 minutes to complete
- Tests run sequentially by default, underutilizing available CPU cores
- No optimization in test compilation profiles
- Potential redundant test setup and teardown operations

## Objective

Reduce test suite execution time by 30-50% through the adoption of cargo-nextest, test profile optimizations, and improved parallelization strategies. This will improve developer experience and enable faster feedback cycles.

## Requirements

### Functional Requirements
- Integrate cargo-nextest as the primary test runner
- Configure optimized test compilation profiles
- Maintain all existing test functionality and coverage
- Preserve test isolation and reproducibility

### Non-Functional Requirements
- Achieve 30-50% reduction in test execution time
- Support both local development and CI environments
- Maintain backward compatibility with standard cargo test
- Zero impact on test reliability or accuracy

## Acceptance Criteria

- [x] cargo-nextest installed and configured in project
- [ ] Test profile optimizations added to Cargo.toml
- [x] Documentation updated with new test commands (Justfile updated)
- [ ] CI pipeline updated to use cargo-nextest
- [ ] 30% or greater reduction in test execution time verified
- [x] All existing tests pass with new configuration
- [ ] Developer documentation includes performance testing guidelines

## Technical Details

### Implementation Approach

1. **Phase 1: cargo-nextest Integration**
   - Add cargo-nextest to project dependencies
   - Create nextest configuration file (.config/nextest.toml)
   - Define test profiles for local and CI environments
   - Configure retry policies for flaky tests

2. **Phase 2: Cargo.toml Optimizations**
   ```toml
   [profile.test]
   opt-level = 2      # Enable optimizations in test builds
   debug = 1          # Reduced debug info for faster builds
   
   [profile.dev]
   opt-level = 1      # Some optimization for development
   debug = 2          # Full debug info for development
   
   [profile.ci]
   inherits = "test"
   lto = "thin"       # Link-time optimization for CI
   codegen-units = 1  # Better optimization at cost of parallel compilation
   ```

3. **Phase 3: Parallelization Configuration**
   - Configure optimal thread count based on available cores
   - Set RUST_TEST_THREADS environment variable
   - Create test groups for better parallel execution
   - Implement test sharding for CI environments

### Architecture Changes

No architectural changes required. This is purely a build and test infrastructure optimization.

### Data Structures

No new data structures required.

### APIs and Interfaces

No API changes. Test execution interface remains compatible with existing commands.

## Dependencies

- **Prerequisites**: None
- **Affected Components**: 
  - Test suite execution
  - CI/CD pipeline configuration
  - Developer documentation
- **External Dependencies**: 
  - cargo-nextest (new dependency)

## Testing Strategy

- **Unit Tests**: Verify all unit tests pass with new runner
- **Integration Tests**: Confirm integration tests work correctly
- **Performance Tests**: Benchmark execution time before and after
- **CI Validation**: Ensure CI pipeline works with nextest

## Documentation Requirements

- **Code Documentation**: Document any test-specific configurations
- **User Documentation**: 
  - Update README with new test commands
  - Add performance testing guide
  - Document nextest configuration options
- **Architecture Updates**: None required

## Implementation Notes

### cargo-nextest Configuration Example

```toml
# .config/nextest.toml
[profile.default]
retries = 0
slow-timeout = { period = "30s", terminate-after = 2 }
test-threads = "num-cpus"

[profile.ci]
retries = 2
failure-output = "immediate-final"
fail-fast = false
test-threads = 8

[profile.local]
retries = 0
failure-output = "immediate"
fail-fast = true
test-threads = "num-cpus"
```

### Usage Examples

```bash
# Install nextest
cargo install cargo-nextest

# Run all tests (replaces cargo test)
cargo nextest run

# Run with specific profile
cargo nextest run --profile ci

# Run specific test
cargo nextest run test_name

# Show test output
cargo nextest run --nocapture

# Run in parallel with explicit thread count
RUST_TEST_THREADS=8 cargo nextest run
```

### Performance Monitoring

Track and document:
- Baseline test execution time
- Time after nextest adoption
- Time after profile optimizations
- CI pipeline execution time improvements

## Migration and Compatibility

During prototype phase, we can make breaking changes if needed. However, this optimization should be transparent to existing workflows:

1. **Fallback Support**: Maintain compatibility with `cargo test` for developers who haven't installed nextest
2. **CI Migration**: Update CI configuration to use nextest
3. **Local Development**: Provide setup script for developers
4. **Documentation**: Clear migration guide for team members

### Migration Steps

1. Install cargo-nextest locally
2. Run test suite to establish baseline timing
3. Add Cargo.toml optimizations
4. Configure nextest profiles
5. Update CI configuration
6. Document new commands and best practices
7. Monitor and tune based on results

### Rollback Plan

If issues arise:
1. Remove nextest configuration files
2. Revert CI configuration
3. Continue using standard cargo test
4. All tests remain compatible with both runners

## Implementation Status

### Completed (2025-01-22)

1. **cargo-nextest installed**: Successfully installed via `cargo install cargo-nextest --locked`
2. **Justfile updated**: All test commands now use `cargo nextest run` instead of `cargo test`
3. **Tests passing**: All 1050+ tests pass with nextest

### Pending

1. **Cargo.toml optimizations**: Profile optimizations not yet added
2. **CI configuration**: GitHub Actions workflow needs updating
3. **Performance benchmarking**: Need to measure actual speedup achieved
4. **Coverage compatibility**: cargo-llvm-cov has compatibility issues with current setup

### Coverage Tool Compatibility Notes

**cargo-tarpaulin**: Works normally, use for coverage reporting
- Continue using: `cargo tarpaulin --out Html --out Lcov`
- Not compatible with nextest (both need to control test execution)

**cargo-llvm-cov**: Has version compatibility issues on macOS
- Would provide nextest compatibility: `cargo llvm-cov nextest`
- Requires LLVM tools setup or nightly Rust
- Deferred due to toolchain complexity

### Recommendation

Use dual approach:
- **Development/CI testing**: `cargo nextest run` (fast, parallel)
- **Coverage reporting**: `cargo tarpaulin` (accurate, established)