---
number: 4
title: Scheduled CI Workflows
category: testing
priority: medium
status: draft
dependencies: [1, 2]
created: 2026-01-02
---

# Specification 004: Scheduled CI Workflows

**Category**: testing
**Priority**: medium
**Status**: draft
**Dependencies**: Spec 001 (CI Infrastructure), Spec 002 (Testing Matrix)

## Context

Debtmap's CI runs only on push and PR events. This misses opportunities to:

- Catch upcoming Rust breakages before they affect development
- Test with latest dependency versions proactively
- Optimize security audits to run only when relevant files change

cargo-cargofmt demonstrates effective use of scheduled workflows for proactive testing and path-filtered triggers for efficient resource usage.

## Objective

Add scheduled CI workflows to test against upcoming Rust versions (beta/nightly) and latest dependencies, plus implement path-based triggers for security audits to reduce unnecessary CI runs.

## Requirements

### Functional Requirements

1. **Rust-Next Workflow**: Monthly scheduled tests against stable, beta, and nightly Rust
2. **Latest Dependencies Testing**: Scheduled job to test with cargo update --workspace
3. **Path-Filtered Security**: Security audits only run when Cargo.toml or Cargo.lock change

### Non-Functional Requirements

1. **Proactive Testing**: Catch Rust ecosystem issues before they block development
2. **Resource Efficiency**: Avoid running expensive jobs unnecessarily
3. **Early Warning**: Surface upcoming breaking changes early
4. **CI Budget**: Reduce wasted CI minutes on unchanged dependencies

## Acceptance Criteria

- [ ] rust-next.yml workflow runs monthly on schedule
- [ ] rust-next tests stable, beta, and nightly on all platforms
- [ ] rust-next tests with latest dependencies (cargo update)
- [ ] beta/nightly failures use continue-on-error (non-blocking)
- [ ] Security audit only triggers on Cargo.toml/Cargo.lock changes
- [ ] Security audit still runs on push to master for baseline
- [ ] Scheduled workflows send notifications on failure (if configured)

## Technical Details

### Implementation Approach

1. **Rust-Next Workflow** (new file: `.github/workflows/rust-next.yml`):
   ```yaml
   name: rust-next

   permissions:
     contents: read

   on:
     schedule:
     - cron: '0 0 1 * *'  # First of each month at midnight UTC

   env:
     RUST_BACKTRACE: 1
     CARGO_TERM_COLOR: always

   concurrency:
     group: "${{ github.workflow }}-${{ github.ref }}"
     cancel-in-progress: true

   jobs:
     test:
       name: Test
       strategy:
         matrix:
           os: ["ubuntu-latest", "windows-latest", "macos-latest"]
           rust: ["stable", "beta"]
           include:
           - os: ubuntu-latest
             rust: "nightly"
       continue-on-error: ${{ matrix.rust != 'stable' }}
       runs-on: ${{ matrix.os }}
       env:
         CARGO_PROFILE_DEV_DEBUG: line-tables-only
       steps:
       - uses: actions/checkout@v5
       - uses: dtolnay/rust-toolchain@master
         with:
           toolchain: ${{ matrix.rust }}
       - uses: Swatinem/rust-cache@v2
       - uses: taiki-e/install-action@cargo-hack
       - name: Build
         run: cargo test --workspace --no-run
       - name: Test
         run: cargo hack test --each-feature --workspace

     latest:
       name: "Check latest dependencies"
       runs-on: ubuntu-latest
       env:
         CARGO_RESOLVER_INCOMPATIBLE_RUST_VERSIONS: allow
       steps:
       - uses: actions/checkout@v5
       - uses: dtolnay/rust-toolchain@stable
       - uses: Swatinem/rust-cache@v2
       - uses: taiki-e/install-action@cargo-hack
       - name: Update dependencies
         run: cargo update
       - name: Build
         run: cargo test --workspace --no-run
       - name: Test
         run: cargo hack test --each-feature --workspace
   ```

2. **Path-Filtered Security Audit** (modify `.github/workflows/security.yml`):
   ```yaml
   on:
     pull_request:
       paths:
         - '**/Cargo.toml'
         - '**/Cargo.lock'
     push:
       branches: [master]
     schedule:
       - cron: '0 2 * * 1'  # Keep weekly scheduled run
     workflow_dispatch:
   ```

### Architecture Changes

- New workflow file: rust-next.yml
- Modified security.yml with path filters

### Files to Create

- `.github/workflows/rust-next.yml`

### Files to Modify

- `.github/workflows/security.yml` (add path filters)

### Schedule Considerations

- Monthly rust-next: Catches breakages without excessive runs
- Weekly security: Catches new advisories promptly
- Path filtering: Runs on relevant changes only

## Dependencies

- **Prerequisites**:
  - Spec 001 (concurrency patterns)
  - Spec 002 (testing matrix, cargo-hack)
- **Affected Components**: CI workflows
- **External Dependencies**: Same as Spec 002

## Testing Strategy

- **Unit Tests**: N/A (workflow changes)
- **Integration Tests**: N/A (scheduled workflows)
- **Manual Verification**:
  - Manually trigger rust-next workflow to verify it runs
  - Verify security audit doesn't run on non-Cargo changes
  - Verify security audit does run on Cargo.toml changes

## Documentation Requirements

- **Code Documentation**: Add comments explaining schedule cron expressions
- **User Documentation**: Document CI schedule in README or CONTRIBUTING
- **Architecture Updates**: None required

## Implementation Notes

- `continue-on-error: true` for beta/nightly prevents blocking on known issues
- `CARGO_RESOLVER_INCOMPATIBLE_RUST_VERSIONS: allow` permits newer deps for latest test
- dtolnay/rust-toolchain@master with `toolchain` param for flexible version
- GitHub sends workflow failure notifications to repo watchers by default
- Consider adding workflow_dispatch trigger for manual testing

## Migration and Compatibility

- No breaking changes
- Existing PRs will see fewer security audit runs (efficiency improvement)
- Scheduled workflows don't affect PR merge requirements
- First rust-next run will occur on next scheduled date (or manual trigger)
