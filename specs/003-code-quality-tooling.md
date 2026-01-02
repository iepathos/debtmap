---
number: 3
title: Code Quality Tooling
category: testing
priority: medium
status: draft
dependencies: [1]
created: 2026-01-02
---

# Specification 003: Code Quality Tooling

**Category**: testing
**Priority**: medium
**Status**: draft
**Dependencies**: Spec 001 (CI Infrastructure Improvements)

## Context

Debtmap's CI currently runs clippy but doesn't integrate results with GitHub's security tab. The project also lacks:

- Commit message linting to enforce conventional commits
- Automated spell checking for code and documentation
- Pre-commit hook CI validation

These quality tools help maintain consistent code standards and catch issues early in the development process.

## Objective

Add code quality tooling to debtmap's CI including Clippy SARIF integration for GitHub security tab, commit message linting with crate-ci/committed, spell checking with crate-ci/typos, and pre-commit CI validation.

## Requirements

### Functional Requirements

1. **Clippy SARIF Integration**: Upload clippy results to GitHub Security tab for inline PR annotations
2. **Commit Message Linting**: Validate commit messages follow conventional commit format
3. **Spell Checking**: Automated spell checking of code and documentation
4. **Pre-commit CI**: Run pre-commit hooks in CI to validate they pass

### Non-Functional Requirements

1. **Developer Experience**: Inline clippy warnings in PR diffs
2. **Consistency**: Enforce commit message standards automatically
3. **Documentation Quality**: Catch typos before they reach production
4. **Workflow Integration**: Pre-commit hooks work locally and in CI

## Acceptance Criteria

- [ ] Clippy results appear in GitHub Security tab for each PR
- [ ] PRs show inline clippy annotations on affected lines
- [ ] Commit message linting runs on all PRs
- [ ] Commits with invalid format are flagged (not necessarily blocking)
- [ ] Spell checking runs on all PRs
- [ ] typos.toml configuration file exists with project-specific words
- [ ] Pre-commit workflow runs pre-commit hooks in CI
- [ ] .pre-commit-config.yaml is validated in CI

## Technical Details

### Implementation Approach

1. **Clippy SARIF Integration**:
   ```yaml
   clippy:
     runs-on: ubuntu-latest
     permissions:
       security-events: write
     steps:
     - uses: actions/checkout@v5
     - uses: dtolnay/rust-toolchain@stable
       with:
         components: clippy
     - uses: Swatinem/rust-cache@v2
     - name: Install SARIF tools
       run: cargo install clippy-sarif sarif-fmt --locked
     - name: Run clippy
       run: >
         cargo clippy --workspace --all-features --all-targets --message-format=json
         | clippy-sarif
         | tee clippy-results.sarif
         | sarif-fmt
       continue-on-error: true
     - name: Upload SARIF
       uses: github/codeql-action/upload-sarif@v4
       with:
         sarif_file: clippy-results.sarif
         wait-for-processing: true
     - name: Check for errors
       run: cargo clippy --workspace --all-features --all-targets -- -D warnings
   ```

2. **Commit Message Linting** (new file: `.github/workflows/committed.yml`):
   ```yaml
   name: Lint Commits
   on: [pull_request]

   permissions:
     contents: read

   concurrency:
     group: "${{ github.workflow }}-${{ github.ref }}"
     cancel-in-progress: true

   jobs:
     committed:
       runs-on: ubuntu-latest
       steps:
       - uses: actions/checkout@v5
         with:
           fetch-depth: 0
       - uses: crate-ci/committed@master
   ```

3. **Spell Checking** (new file: `.github/workflows/spelling.yml`):
   ```yaml
   name: Spelling
   on: [pull_request]

   permissions:
     contents: read

   concurrency:
     group: "${{ github.workflow }}-${{ github.ref }}"
     cancel-in-progress: true

   jobs:
     spelling:
       runs-on: ubuntu-latest
       steps:
       - uses: actions/checkout@v5
       - uses: crate-ci/typos@master
   ```

4. **Pre-commit CI** (new file: `.github/workflows/pre-commit.yml`):
   ```yaml
   name: pre-commit
   on:
     pull_request:
     push:
       branches: [master]

   permissions:
     contents: read

   concurrency:
     group: "${{ github.workflow }}-${{ github.ref }}"
     cancel-in-progress: true

   jobs:
     pre-commit:
       runs-on: ubuntu-latest
       steps:
       - uses: actions/checkout@v5
       - uses: actions/setup-python@v5
         with:
           python-version: '3.x'
       - uses: pre-commit/action@v3.0.1
   ```

5. **Typos Configuration** (new file: `typos.toml`):
   ```toml
   [default.extend-words]
   # Project-specific words that aren't typos

   [files]
   extend-exclude = [
     "*.lock",
     "target/",
   ]
   ```

### Files to Create

- `.github/workflows/committed.yml`
- `.github/workflows/spelling.yml`
- `.github/workflows/pre-commit.yml`
- `typos.toml`
- `.committed.toml` (optional, for commit message configuration)

### Files to Modify

- `.github/workflows/ci.yml` (add SARIF integration to clippy)

## Dependencies

- **Prerequisites**: Spec 001 (for concurrency and permissions patterns)
- **Affected Components**: CI workflows
- **External Dependencies**:
  - crate-ci/committed@master
  - crate-ci/typos@master
  - pre-commit/action@v3.0.1
  - github/codeql-action/upload-sarif@v4
  - clippy-sarif, sarif-fmt cargo tools

## Testing Strategy

- **Unit Tests**: N/A (workflow changes)
- **Integration Tests**:
  - Submit PR with intentional typo to verify spelling check
  - Submit PR with bad commit message to verify linting
- **Manual Verification**:
  - Verify clippy annotations appear in PR diff
  - Verify SARIF results in Security tab

## Documentation Requirements

- **Code Documentation**: None required
- **User Documentation**: Document commit message format expectations in CONTRIBUTING.md
- **Architecture Updates**: None required

## Implementation Notes

- committed uses conventional commit format by default
- typos has good defaults but may need project-specific word list
- SARIF upload requires `security-events: write` permission
- Pre-commit CI requires .pre-commit-config.yaml to exist
- Consider making commit linting non-blocking initially (continue-on-error: true)

## Migration and Compatibility

- No breaking changes
- May flag existing typos in codebase (fix them)
- Historical commits won't be validated, only new PRs
- Team should agree on commit message format expectations
