---
number: 1
title: CI Infrastructure Improvements
category: foundation
priority: high
status: draft
dependencies: []
created: 2026-01-02
---

# Specification 001: CI Infrastructure Improvements

**Category**: foundation
**Priority**: high
**Status**: draft
**Dependencies**: None

## Context

Debtmap's current CI workflow lacks several infrastructure best practices that are standard in mature Rust projects. Comparing against cargo-cargofmt's CI reveals missing concurrency control, inefficient caching, and no aggregated status job. These gaps lead to:

- Wasted CI resources when multiple commits are pushed in quick succession
- Slower CI runs due to suboptimal caching
- Difficulty determining overall CI status when multiple jobs run
- Potential security issues from overly broad permissions

## Objective

Modernize debtmap's CI infrastructure by adding concurrency control, CI aggregation jobs, explicit permissions, optimized caching, and reduced debug info to match industry best practices.

## Requirements

### Functional Requirements

1. **Concurrency Control**: Cancel in-progress CI runs when new commits are pushed to the same branch
2. **CI Aggregation Job**: Single job that reports overall CI status based on all other jobs
3. **Optimized Caching**: Use Swatinem/rust-cache for faster, more efficient dependency caching
4. **Debug Info Optimization**: Reduce cached data size by limiting debug info

### Non-Functional Requirements

1. **Security**: Apply principle of least privilege with explicit permissions per job
2. **Performance**: CI runs should complete faster due to better caching
3. **Resource Efficiency**: Reduce wasted GitHub Actions minutes from redundant runs
4. **Maintainability**: Consistent configuration patterns across all workflow files

## Acceptance Criteria

- [ ] All workflow files include concurrency block with cancel-in-progress: true
- [ ] Main ci.yml has aggregation job that fails if any dependent job fails
- [ ] All workflows use Swatinem/rust-cache@v2 instead of manual actions/cache
- [ ] All workflows have explicit `permissions:` blocks with minimal required permissions
- [ ] CARGO_PROFILE_DEV_DEBUG environment variable set to line-tables-only
- [ ] CI passes on existing test suite after changes
- [ ] Redundant CI runs are cancelled when new commits are pushed

## Technical Details

### Implementation Approach

1. Add concurrency block to all workflow files:
   ```yaml
   concurrency:
     group: "${{ github.workflow }}-${{ github.ref }}"
     cancel-in-progress: true
   ```

2. Create CI aggregation job in ci.yml:
   ```yaml
   ci:
     name: CI
     needs: [test, lint, clippy, ...]
     runs-on: ubuntu-latest
     if: "always()"
     steps:
       - name: Failed
         run: exit 1
         if: "contains(needs.*.result, 'failure') || contains(needs.*.result, 'cancelled') || contains(needs.*.result, 'skipped')"
   ```

3. Replace manual caching with Swatinem/rust-cache:
   ```yaml
   - uses: Swatinem/rust-cache@v2
   ```

4. Add explicit permissions to each workflow:
   ```yaml
   permissions:
     contents: read
   ```

5. Add debug optimization environment variable:
   ```yaml
   env:
     CARGO_PROFILE_DEV_DEBUG: line-tables-only
   ```

### Architecture Changes

- ci.yml: Restructure to have separate jobs for test, lint, clippy with final aggregation
- coverage.yml: Replace actions-rs/toolchain with dtolnay/rust-toolchain, use Swatinem cache
- security.yml: Add explicit permissions, concurrency control
- All workflows: Consistent structure and environment variables

### Files to Modify

- `.github/workflows/ci.yml`
- `.github/workflows/coverage.yml`
- `.github/workflows/security.yml`
- `.github/workflows/release.yml`
- `.github/workflows/deploy-docs.yml`
- `.github/workflows/debtmap.yml`

## Dependencies

- **Prerequisites**: None
- **Affected Components**: All GitHub Actions workflows
- **External Dependencies**:
  - Swatinem/rust-cache@v2
  - dtolnay/rust-toolchain@stable

## Testing Strategy

- **Unit Tests**: N/A (workflow changes)
- **Integration Tests**: Push test commits to verify concurrency cancellation works
- **Manual Verification**:
  - Verify CI aggregation job correctly reports failures
  - Verify cache hits on subsequent runs
  - Verify redundant runs are cancelled

## Documentation Requirements

- **Code Documentation**: Add comments in workflow files explaining key configurations
- **User Documentation**: None required
- **Architecture Updates**: None required

## Implementation Notes

- Use actions/checkout@v5 (cargo-cargofmt) or v6 consistently
- dtolnay/rust-toolchain is preferred over deprecated actions-rs/toolchain
- The aggregation job pattern is standard in cargo ecosystem projects
- Swatinem/rust-cache automatically handles target directory and cargo registry

## Migration and Compatibility

- No breaking changes for users
- CI behavior change: redundant runs will now be cancelled
- Branch protection rules may need updating if they reference specific job names
