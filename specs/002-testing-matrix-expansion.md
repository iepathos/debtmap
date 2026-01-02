---
number: 2
title: Testing Matrix Expansion
category: testing
priority: high
status: draft
dependencies: [1]
created: 2026-01-02
---

# Specification 002: Testing Matrix Expansion

**Category**: testing
**Priority**: high
**Status**: draft
**Dependencies**: Spec 001 (CI Infrastructure Improvements)

## Context

Debtmap currently tests only on Ubuntu and macOS, missing Windows entirely. Additionally, the CI lacks:

- MSRV (Minimum Supported Rust Version) validation
- Minimal-versions testing to catch implicit dependency requirements
- Feature flag combination testing

These gaps can lead to broken releases on Windows, unexpected MSRV breakage, and feature flag incompatibilities that only surface for users.

## Objective

Expand debtmap's test matrix to include Windows, implement MSRV checking, add minimal-versions testing, and use cargo-hack for comprehensive feature flag testing.

## Requirements

### Functional Requirements

1. **Windows Testing**: Add Windows to the CI test matrix
2. **MSRV Validation**: Automatically verify code compiles with declared MSRV
3. **Minimal-Versions Testing**: Test with oldest compatible dependency versions
4. **Feature Flag Testing**: Test all feature combinations using cargo-hack

### Non-Functional Requirements

1. **Compatibility**: Ensure debtmap works across all major platforms
2. **Stability**: Catch dependency issues before they affect users
3. **Reliability**: Prevent accidental MSRV breakage
4. **Coverage**: Test all supported feature combinations

## Acceptance Criteria

- [ ] CI test matrix includes ubuntu-latest, windows-latest, macos-latest
- [ ] All tests pass on Windows
- [ ] MSRV is declared in Cargo.toml (rust-version field)
- [ ] CI validates MSRV using cargo-hack --rust-version
- [ ] Minimal-versions job generates lockfile with -Z minimal-versions and compiles successfully
- [ ] cargo-hack tests each feature combination
- [ ] CI fails if any feature combination doesn't compile or test

## Technical Details

### Implementation Approach

1. **Expand Test Matrix**:
   ```yaml
   strategy:
     matrix:
       os: ["ubuntu-latest", "windows-latest", "macos-latest"]
       rust: ["stable"]
   ```

2. **Add MSRV to Cargo.toml**:
   ```toml
   [package]
   rust-version = "1.75"  # Or appropriate version
   ```

3. **MSRV Checking Job**:
   ```yaml
   msrv:
     name: "Check MSRV"
     runs-on: ubuntu-latest
     steps:
     - uses: actions/checkout@v5
     - uses: dtolnay/rust-toolchain@stable
     - uses: Swatinem/rust-cache@v2
     - uses: taiki-e/install-action@cargo-hack
     - name: Check MSRV
       run: cargo hack check --each-feature --locked --rust-version --workspace --all-targets --keep-going
   ```

4. **Minimal-Versions Testing**:
   ```yaml
   minimal-versions:
     name: Minimal versions
     runs-on: ubuntu-latest
     steps:
     - uses: actions/checkout@v5
     - uses: dtolnay/rust-toolchain@stable
     - name: Install nightly
       uses: dtolnay/rust-toolchain@nightly
     - name: Generate minimal lockfile
       run: cargo +nightly generate-lockfile -Z minimal-versions
     - name: Check with minimal versions
       run: cargo +stable check --workspace --all-features --locked --keep-going
   ```

5. **Feature Flag Testing**:
   ```yaml
   - uses: taiki-e/install-action@cargo-hack
   - name: Test each feature
     run: cargo hack test --each-feature --workspace
   ```

### Architecture Changes

- ci.yml: Split into separate jobs (test, msrv, minimal-versions, lockfile)
- Cargo.toml: Add rust-version field

### Files to Modify

- `.github/workflows/ci.yml`
- `Cargo.toml` (add rust-version)

### Windows-Specific Considerations

- Ensure any shell commands work on Windows (use bash explicitly if needed)
- Handle path separators correctly in any file operations
- Test any platform-specific code paths

## Dependencies

- **Prerequisites**: Spec 001 (for CI infrastructure patterns)
- **Affected Components**: CI workflows, Cargo.toml
- **External Dependencies**:
  - taiki-e/install-action@cargo-hack
  - cargo-hack tool

## Testing Strategy

- **Unit Tests**: Existing test suite runs on all platforms
- **Integration Tests**: Feature combination testing via cargo-hack
- **MSRV Tests**: Compilation check with declared MSRV
- **Minimal Versions**: Compilation check with oldest dependencies

## Documentation Requirements

- **Code Documentation**: Document MSRV in README if not already present
- **User Documentation**: Update installation instructions if Windows was previously unsupported
- **Architecture Updates**: None required

## Implementation Notes

- Determine appropriate MSRV by checking:
  - Current Rust edition requirements
  - Minimum versions of key dependencies (anyhow, clap, etc.)
  - Any Rust features used (e.g., let-else, inline const)
- cargo-hack --each-feature tests the powerset of features
- Minimal-versions requires nightly for -Z flag but checks with stable
- Windows CI may be slower; consider making it optional for PRs

## Migration and Compatibility

- MSRV declaration may affect users on older Rust versions
- Should announce MSRV policy in documentation
- Consider setting MSRV conservatively (e.g., Rust 1.70+)
