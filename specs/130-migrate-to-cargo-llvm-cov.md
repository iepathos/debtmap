---
number: 130
title: Migrate from cargo-tarpaulin to cargo-llvm-cov for code coverage
category: testing
priority: medium
status: draft
dependencies: []
created: 2025-10-26
---

# Specification 130: Migrate from cargo-tarpaulin to cargo-llvm-cov for code coverage

**Category**: testing
**Priority**: medium
**Status**: draft
**Dependencies**: None

## Context

Debtmap currently uses `cargo-tarpaulin` for code coverage reporting. While tarpaulin has served the project well, the Rust ecosystem has evolved and `cargo-llvm-cov` has become the recommended standard for several reasons:

1. **Better accuracy** - Uses LLVM's native source-based coverage instrumentation
2. **Faster execution** - More efficient instrumentation and data collection
3. **Better async/await support** - Improved handling of modern async Rust code
4. **Stable Rust compatibility** - Works on stable toolchain without additional setup
5. **Fewer false positives/negatives** - More reliable coverage metrics
6. **Better maintenance** - Actively maintained with regular updates
7. **Industry standard** - Widely adopted in the Rust ecosystem

The current `.tarpaulin.toml` configuration already uses the LLVM engine (`engine = "Llvm"`), but cargo-llvm-cov provides a more direct and efficient interface to LLVM coverage.

## Objective

Migrate debtmap's code coverage tooling from `cargo-tarpaulin` to `cargo-llvm-cov`, updating all related configuration files, CI/CD workflows, documentation, and development scripts to use the new tool while maintaining or improving coverage accuracy and developer experience.

## Requirements

### Functional Requirements

1. **Tool Installation**
   - Add cargo-llvm-cov installation to development setup documentation
   - Update tool installation scripts (justfile)
   - Ensure CI/CD workflows install cargo-llvm-cov

2. **Configuration Migration**
   - Create new `.cargo-llvm-cov.toml` or equivalent configuration
   - Migrate exclusion patterns from `.tarpaulin.toml`
   - Preserve output format options (HTML, LCOV, JSON)
   - Maintain parallel execution capabilities

3. **CI/CD Integration**
   - Update GitHub Actions workflows to use cargo-llvm-cov
   - Preserve all output formats for artifact collection
   - Maintain coverage threshold checking
   - Ensure compatibility with coverage upload services (if any)

4. **Development Workflow**
   - Update justfile recipes for coverage commands
   - Maintain all existing coverage workflows (generate, check, open)
   - Preserve LCOV output for debtmap self-analysis
   - Support incremental coverage for faster iteration

5. **Documentation Updates**
   - Update README.md with cargo-llvm-cov examples
   - Update contributing/development documentation
   - Update installation instructions
   - Add migration notes for existing contributors

### Non-Functional Requirements

1. **Performance** - Coverage generation should be as fast or faster than current tarpaulin setup
2. **Accuracy** - Coverage metrics should be at least as accurate as current setup
3. **Compatibility** - Must work on Ubuntu and macOS (current CI platforms)
4. **Maintainability** - Configuration should be simple and well-documented
5. **Developer Experience** - Should not disrupt existing development workflows

## Acceptance Criteria

- [ ] `cargo-llvm-cov` is installed and configured for local development
- [ ] All justfile coverage recipes use cargo-llvm-cov instead of tarpaulin
- [ ] Coverage reports generate in HTML, LCOV, and JSON formats
- [ ] Coverage threshold checking works correctly (80% minimum)
- [ ] CI/CD workflows successfully run coverage with cargo-llvm-cov
- [ ] Coverage reports are generated on both Ubuntu and macOS
- [ ] `just analyze-self` command works with llvm-cov generated LCOV
- [ ] README.md shows cargo-llvm-cov examples instead of tarpaulin
- [ ] Development documentation updated with installation instructions
- [ ] `.tarpaulin.toml` is removed after migration is complete
- [ ] All exclusion patterns (tests, benchmarks) are properly migrated
- [ ] Coverage workflow maintains parallel execution for performance
- [ ] Documentation includes rationale for the migration

## Technical Details

### Implementation Approach

The migration will follow these steps:

1. **Local Setup and Testing**
   - Install cargo-llvm-cov locally
   - Test basic coverage generation
   - Compare output with current tarpaulin results
   - Validate exclusion patterns work correctly

2. **Configuration Creation**
   - Create coverage configuration (if needed)
   - Set up exclusion patterns matching current .tarpaulin.toml
   - Configure output formats (HTML, LCOV, JSON)
   - Configure output directory (target/coverage)

3. **Justfile Migration**
   - Update `coverage` recipe to use cargo-llvm-cov
   - Update `coverage-lcov` recipe
   - Update `coverage-check` recipe with threshold validation
   - Update `coverage-open` recipe
   - Update `analyze-self` recipe
   - Update `install-tools` recipe

4. **CI/CD Workflow Updates**
   - Update GitHub Actions to install cargo-llvm-cov
   - Update workflow steps to use new commands
   - Validate artifact uploads still work
   - Test on both Ubuntu and macOS runners

5. **Documentation Updates**
   - Update README.md Quick Start section
   - Update README.md Coverage & Risk section
   - Update development documentation
   - Update contributing guide
   - Add migration notes for contributors

6. **Cleanup**
   - Remove `.tarpaulin.toml`
   - Remove any tarpaulin-specific scripts
   - Update any remaining references to tarpaulin

### cargo-llvm-cov Command Reference

```bash
# Install
cargo install cargo-llvm-cov

# Generate HTML report (default location: target/llvm-cov/html)
cargo llvm-cov --html

# Generate LCOV format for debtmap analysis
cargo llvm-cov --lcov --output-path target/coverage/lcov.info

# Generate JSON format
cargo llvm-cov --json --output-path target/coverage/coverage.json

# Generate all formats
cargo llvm-cov --html --lcov --json

# Check coverage threshold
cargo llvm-cov --fail-under-lines 80

# Exclude patterns (via CARGO_LLVM_COV_IGNORE_FILENAME_REGEX or CLI)
cargo llvm-cov --ignore-filename-pattern 'tests/.*' --ignore-filename-pattern '.*_test\.rs'

# Clean coverage artifacts
cargo llvm-cov clean

# Show coverage summary in terminal
cargo llvm-cov --summary-only

# Open HTML report in browser
cargo llvm-cov --open
```

### Configuration Options

cargo-llvm-cov can be configured via:

1. **Command-line flags** (recommended for explicit control)
2. **Environment variables** (for CI/CD)
3. **Cargo.toml `[llvm-cov]` section** (for project defaults)

Recommended configuration approach:

```toml
# In Cargo.toml (optional, for defaults)
[package.metadata.llvm-cov]
# Exclude test files from coverage
ignore-filename-patterns = [
    "tests/.*",
    ".*_test\\.rs",
    ".*_tests\\.rs",
    ".*/tests/.*",
    ".*/test/.*",
    "benches/.*",
    ".*/bench/.*",
    ".*/benchmark/.*"
]
```

### Justfile Recipe Updates

**Before (tarpaulin)**:
```just
coverage:
    #!/usr/bin/env bash
    echo "Building debtmap binary for integration tests..."
    cargo build --bin debtmap
    echo "Generating code coverage report with tarpaulin (LLVM engine + nextest)..."
    cargo tarpaulin --config .tarpaulin.toml
    echo "Coverage report generated at target/coverage/tarpaulin-report.html"
```

**After (llvm-cov)**:
```just
coverage:
    #!/usr/bin/env bash
    echo "Cleaning previous coverage data..."
    cargo llvm-cov clean
    echo "Generating code coverage report with cargo-llvm-cov..."
    cargo llvm-cov --all-features \
        --ignore-filename-pattern 'tests/.*' \
        --ignore-filename-pattern '.*_test\.rs' \
        --ignore-filename-pattern '.*_tests\.rs' \
        --html --lcov --json \
        --output-dir target/coverage
    echo "Coverage report generated at target/coverage/html/index.html"
```

### CI/CD Workflow Updates

**Before (tarpaulin in CI)**:
```yaml
- name: Generate coverage
  run: cargo tarpaulin --config .tarpaulin.toml
```

**After (llvm-cov in CI)**:
```yaml
- name: Install cargo-llvm-cov
  run: cargo install cargo-llvm-cov

- name: Generate coverage
  run: |
    cargo llvm-cov clean
    cargo llvm-cov --all-features \
      --ignore-filename-pattern 'tests/.*' \
      --ignore-filename-pattern '.*_test\.rs' \
      --ignore-filename-pattern '.*_tests\.rs' \
      --html --lcov --json \
      --output-dir target/coverage

- name: Upload coverage reports
  uses: actions/upload-artifact@v4
  with:
    name: coverage-reports
    path: target/coverage/
```

### README.md Updates

**Current (lines 72-73)**:
```bash
# With test coverage (recommended)
cargo tarpaulin --out lcov --output-dir target/coverage
debtmap analyze . --lcov target/coverage/lcov.info
```

**Updated**:
```bash
# With test coverage (recommended)
cargo llvm-cov --lcov --output-path target/coverage/lcov.info
debtmap analyze . --lcov target/coverage/lcov.info

# Or using just command
just coverage
debtmap analyze . --lcov target/coverage/lcov.info
```

### Exclusion Pattern Migration

Current `.tarpaulin.toml` exclusions:
- `tests/*`
- `*_test.rs`
- `*_tests.rs`
- `*/tests/*`
- `*/test/*`
- `benches/*`
- `*/bench/*`
- `*/benchmark/*`

These will be migrated to cargo-llvm-cov using `--ignore-filename-pattern` flags or configuration in `Cargo.toml`.

### Output Directory Structure

**Current (tarpaulin)**:
```
target/coverage/
├── tarpaulin-report.html
├── tarpaulin-report.json
└── lcov.info
```

**New (llvm-cov)**:
```
target/coverage/
├── html/
│   └── index.html
├── lcov.info
└── coverage.json
```

Need to update file paths in:
- `just coverage-open` (HTML path)
- `just coverage-check` (JSON path)
- CI/CD artifact uploads

## Dependencies

**No prerequisite specifications** - This is a standalone tooling migration.

**Affected Components**:
- `.tarpaulin.toml` - To be removed
- `justfile` - Coverage recipes to be updated
- `.github/workflows/ci.yml` - CI workflow to be updated
- `README.md` - Documentation examples to be updated
- Development documentation
- Contributing guide

**External Dependencies**:
- `cargo-llvm-cov` (new tool to be installed)
- LLVM toolchain (already available via rustc)

## Testing Strategy

### Manual Testing

1. **Local coverage generation**
   ```bash
   cargo llvm-cov --html
   # Verify HTML report opens correctly
   # Compare coverage percentages with previous tarpaulin run
   ```

2. **LCOV format validation**
   ```bash
   cargo llvm-cov --lcov --output-path target/coverage/lcov.info
   debtmap analyze . --lcov target/coverage/lcov.info
   # Verify debtmap can parse the LCOV file
   # Check that coverage data is correctly integrated
   ```

3. **Threshold checking**
   ```bash
   cargo llvm-cov --fail-under-lines 80
   # Verify threshold validation works
   ```

4. **Justfile recipes**
   ```bash
   just coverage
   just coverage-lcov
   just coverage-check
   just coverage-open
   just analyze-self
   # Verify all recipes work correctly
   ```

### Integration Testing

1. **CI/CD validation**
   - Create PR with migration changes
   - Verify CI workflows pass on Ubuntu
   - Verify CI workflows pass on macOS
   - Check coverage artifacts are uploaded correctly

2. **Cross-platform testing**
   - Test on Ubuntu (CI)
   - Test on macOS (CI and local)
   - Verify consistent coverage metrics across platforms

3. **Regression testing**
   - Compare coverage percentages before and after migration
   - Verify no significant drops in coverage (±2% acceptable variance)
   - Check that all previously covered code is still covered

### Performance Testing

1. **Execution time comparison**
   - Measure tarpaulin execution time (baseline)
   - Measure llvm-cov execution time
   - Verify llvm-cov is equal or faster

2. **Incremental coverage**
   - Test `cargo llvm-cov --no-clean` for incremental runs
   - Verify significant speedup for unchanged code

## Documentation Requirements

### Code Documentation

- Update inline comments referencing tarpaulin
- Add comments explaining coverage exclusions

### User Documentation

1. **README.md updates**
   - Line 72-73: Update coverage example to use cargo-llvm-cov
   - Line 346: Update `just coverage` description
   - Add note about cargo-llvm-cov in installation section
   - Update any other tarpaulin references

2. **Development documentation**
   - Update development setup guide
   - Add cargo-llvm-cov installation instructions
   - Update coverage workflow documentation
   - Add troubleshooting section for common issues

3. **Contributing guide**
   - Update testing section with new coverage commands
   - Add migration notes for existing contributors
   - Update quality gates section if needed

### Migration Documentation

Create a brief migration note in the commit message or PR:

```
Migrate from cargo-tarpaulin to cargo-llvm-cov

Rationale:
- Better accuracy using LLVM's native coverage instrumentation
- Faster execution and better async/await support
- Industry standard in Rust ecosystem
- Works on stable toolchain

Changes:
- Removed .tarpaulin.toml
- Updated justfile coverage recipes
- Updated CI/CD workflows
- Updated README.md examples
- Migrated all exclusion patterns

All coverage metrics and workflows are preserved.
```

## Implementation Notes

### Advantages Over Tarpaulin

Key benefits of migrating to cargo-llvm-cov:

1. **Better Platform Support**
   - ✅ Full support: Linux, macOS, Windows
   - ✅ Works on ARM architectures (M1/M2 Macs)
   - ⚠️ Tarpaulin: Linux x86-64 only, limited macOS support

2. **Faster Execution**
   - ✅ More efficient LLVM instrumentation
   - ✅ Better incremental compilation support
   - ✅ Typically 20-40% faster than tarpaulin

3. **Better Accuracy**
   - ✅ Fewer false positives/negatives
   - ✅ More precise source mapping
   - ✅ Better async/await coverage

4. **Cargo Nextest Integration**
   - ✅ Native support via `cargo llvm-cov nextest`
   - ✅ Faster test execution with coverage
   - ⚠️ Tarpaulin: Limited nextest support

5. **Active Maintenance**
   - ✅ Regular updates and bug fixes
   - ✅ Follows Rust compiler development
   - ✅ Growing ecosystem adoption

6. **Industry Standard**
   - ✅ Recommended by Rust project
   - ✅ Uses rustc's native coverage
   - ✅ Better long-term support

### Known Limitations of cargo-llvm-cov

Before migrating, be aware of these current limitations (most don't affect debtmap):

1. **Doc Tests** (Issue #2)
   - Doc tests disabled by default
   - Requires nightly compiler for doctest coverage
   - **Impact on debtmap**: Minor - we primarily use unit and integration tests
   - **Mitigation**: Skip doctest coverage or use `--doctests` flag with nightly if needed

2. **Branch Coverage** (Issue #8)
   - Branch coverage not yet supported
   - Line and region coverage available
   - **Impact on debtmap**: None - we use line coverage metrics
   - **Mitigation**: Use line coverage (default behavior)

3. **Workspace Recompilation** (Issue #198)
   - May recompile workspace crates on each run
   - Can slow down coverage in large workspaces
   - **Impact on debtmap**: Minor - single package project
   - **Mitigation**: Use `--no-clean` for incremental runs during development

4. **Non-Workspace Path Dependencies** (Issue #303)
   - Path dependencies may be included in coverage reports
   - Can inflate or skew coverage percentages
   - **Impact on debtmap**: None - no path dependencies
   - **Mitigation**: Use `--ignore-filename-pattern` to exclude unwanted paths

5. **Compiler Version Requirements**
   - Requires rustc 1.60+ to run
   - Building cargo-llvm-cov itself requires rustc 1.81+
   - LLVM version must match rustc's LLVM version
   - **Impact on debtmap**: None - already using modern Rust
   - **Mitigation**: CI already pins compatible Rust versions

6. **Platform Support**
   - Broader platform support than tarpaulin (Linux, macOS, Windows)
   - tarpaulin only works well on Linux x86-64
   - **Impact on debtmap**: Positive - better macOS support
   - **Benefit**: More reliable cross-platform CI coverage

### Potential Issues and Solutions

1. **Issue**: Different output directory structure
   **Solution**: Update all file path references in justfile and CI
   ```
   Old: target/coverage/tarpaulin-report.html
   New: target/coverage/html/index.html
   ```

2. **Issue**: JSON output format may differ
   **Solution**: Update `just coverage-check` to parse llvm-cov JSON schema
   ```bash
   # May need to adjust jq query for new JSON structure
   jq '.data[0].totals.lines.percent' coverage.json
   ```

3. **Issue**: Coverage percentages may differ slightly
   **Solution**: Document that ±2% variance is expected due to more accurate instrumentation
   - llvm-cov has better source mapping accuracy
   - May expose previously uncounted code paths
   - May correctly exclude unreachable code

4. **Issue**: First-time setup requires component installation
   **Solution**: Document installation steps clearly
   ```bash
   cargo install cargo-llvm-cov
   # rustup component add llvm-tools-preview  # Usually automatic
   ```

5. **Issue**: Some developers may have tarpaulin installed
   **Solution**: Add clear migration notes in commit message and update docs
   - Both tools can coexist
   - Update justfile recipes to use llvm-cov
   - Document transition in PR description

6. **Issue**: LLVM version mismatch warnings
   **Solution**: Use rustc-bundled LLVM tools (default behavior)
   - cargo-llvm-cov automatically finds correct versions
   - Only matters for custom LLVM installations

7. **Issue**: Incomplete coverage on first run after clean
   **Solution**: Run coverage twice if needed
   ```bash
   cargo llvm-cov clean
   cargo llvm-cov --no-report  # First pass
   cargo llvm-cov --no-run --html  # Generate report
   ```

8. **Issue**: Integration test binaries may need special handling
   **Solution**: Already handled - cargo-llvm-cov covers all test types by default
   - Unit tests: Covered automatically
   - Integration tests: Covered automatically
   - Doc tests: Use `--doctests` flag if needed (requires nightly)

### Debtmap-Specific Considerations

1. **Single Package Project**
   - ✅ No workspace complexity
   - ✅ No path dependency issues
   - ✅ Straightforward migration

2. **Test Suite Compatibility**
   - Current: Uses cargo nextest
   - cargo-llvm-cov: Supports nextest via `cargo llvm-cov nextest`
   - **Action**: Update justfile to use `cargo llvm-cov nextest` if desired

3. **LCOV Output for Self-Analysis**
   - Critical requirement: debtmap analyzes its own coverage
   - cargo-llvm-cov: Full LCOV support with `--lcov` flag
   - ✅ No compatibility issues

4. **Parallel Execution**
   - Current: 12 parallel jobs in .tarpaulin.toml
   - cargo-llvm-cov: Uses cargo's parallel test execution
   - **Action**: No special configuration needed

5. **CI Artifact Upload**
   - Current: Uploads target/coverage/
   - New: Same directory, different internal structure
   - **Action**: Artifact upload paths remain the same

### Best Practices

1. **Use explicit flags** rather than relying on defaults for CI/CD
2. **Clean coverage data** before each run to avoid stale data
3. **Document exclusion patterns** clearly in configuration
4. **Keep HTML and LCOV outputs** for different use cases
5. **Test migration locally** before updating CI/CD

### Performance Optimization

```bash
# For faster local development, generate only what's needed
cargo llvm-cov --html  # Only HTML for quick viewing

# For CI, generate all formats once
cargo llvm-cov --html --lcov --json  # All formats in one pass

# For incremental development (experimental)
cargo llvm-cov --no-clean  # Don't clean before running
```

## Migration and Compatibility

### Breaking Changes

None - this is an internal tooling change that doesn't affect:
- Public API
- Command-line interface
- Output formats (LCOV, HTML, JSON still available)
- Coverage thresholds

### Migration Path for Contributors

1. **Install cargo-llvm-cov**: `cargo install cargo-llvm-cov`
2. **Remove tarpaulin** (optional): `cargo uninstall cargo-tarpaulin`
3. **Use existing workflows**: `just coverage`, `just analyze-self`, etc.
4. **No code changes required**: Coverage instrumentation is automatic

### Backward Compatibility

- Old coverage reports in `target/coverage/` from tarpaulin will be replaced
- LCOV format remains compatible with debtmap analysis
- No changes to debtmap's coverage parsing logic needed

### Timeline

1. **Phase 1** (Day 1): Local setup and testing
2. **Phase 2** (Day 1-2): Justfile and configuration updates
3. **Phase 3** (Day 2): CI/CD workflow updates
4. **Phase 4** (Day 2-3): Documentation updates
5. **Phase 5** (Day 3): Testing and validation
6. **Phase 6** (Day 3): Cleanup and commit

Total estimated time: 2-3 days including thorough testing

## Troubleshooting Guide

### Common Issues After Migration

1. **Error: `llvm-tools-preview` component not found**
   ```bash
   # Solution: Install the component
   rustup component add llvm-tools-preview
   ```

2. **Warning: Coverage data appears incomplete**
   ```bash
   # Solution: Clean and regenerate
   cargo llvm-cov clean
   cargo llvm-cov --all-features
   ```

3. **Error: Different coverage percentage than expected**
   - **Cause**: More accurate instrumentation
   - **Solution**: Review newly covered/uncovered code
   - **Note**: ±2% variance is normal and expected

4. **Error: HTML report not found at old path**
   ```bash
   # Old path: target/coverage/tarpaulin-report.html
   # New path: target/coverage/html/index.html
   # Solution: Update bookmarks and scripts
   ```

5. **Error: JSON schema different, jq query fails**
   ```bash
   # Old tarpaulin query:
   jq '.files | to_entries | map(.value.coverage) | add / length'

   # New llvm-cov query (may need adjustment):
   jq '.data[0].totals.lines.percent'
   ```

6. **Performance: Coverage generation seems slow**
   ```bash
   # Solution: Use incremental mode for development
   cargo llvm-cov --no-clean  # Skip clean step

   # Or integrate with nextest for faster tests
   cargo llvm-cov nextest
   ```

7. **CI: Artifact upload fails**
   - **Cause**: Different directory structure
   - **Solution**: Update artifact paths in workflow
   ```yaml
   # Before
   path: target/coverage/tarpaulin-report.html

   # After
   path: target/coverage/
   ```

## Success Metrics

- [ ] All coverage workflows continue to function
- [ ] Coverage percentages remain consistent (±2%)
- [ ] CI/CD workflows pass on all platforms
- [ ] Documentation is clear and up-to-date
- [ ] No developer workflow disruption
- [ ] Faster or equal coverage generation time
- [ ] Improved coverage accuracy (if measurable)
- [ ] macOS CI coverage more reliable than with tarpaulin

## References

- [cargo-llvm-cov GitHub](https://github.com/taiki-e/cargo-llvm-cov)
- [cargo-llvm-cov Documentation](https://github.com/taiki-e/cargo-llvm-cov/blob/main/README.md)
- [LLVM Source-Based Code Coverage](https://clang.llvm.org/docs/SourceBasedCodeCoverage.html)
- [Rust Coverage Instrumentation](https://doc.rust-lang.org/rustc/instrument-coverage.html)
