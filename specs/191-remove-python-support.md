---
number: 191
title: Remove Python Language Support
category: foundation
priority: high
status: draft
dependencies: []
created: 2025-11-30
---

# Specification 191: Remove Python Language Support

**Category**: foundation
**Priority**: high
**Status**: draft
**Dependencies**: None

## Context

Debtmap currently provides partial Python analysis support via `rustpython-parser`. However, maintaining multiple language analyzers distracts from the primary goal of being the best Rust code analyzer. To focus development efforts and perfect Rust analysis before expanding to other languages, Python support needs to be removed from the codebase.

## Objective

Remove all Python language analysis functionality from debtmap, including parser integration, analyzers, detectors, tests, benchmarks, and documentation references, while maintaining codebase stability and test coverage for remaining Rust-focused features.

## Requirements

### Functional Requirements

- Remove Python analyzer modules and all Python-specific analysis code
- Remove Python parser dependency (`rustpython-parser`) from Cargo.toml
- Remove Python-specific test files and test cases
- Remove Python-specific benchmarks
- Remove Python language variant from Language enum (or mark as unsupported)
- Update analyzer factory to not return Python analyzers
- Clean up any Python-specific configuration options

### Non-Functional Requirements

- All remaining tests must pass after removal
- No broken references or imports in remaining code
- Cargo build must succeed with no warnings
- Code coverage should not decrease for Rust analysis features
- Documentation must be updated to reflect Rust-only support

## Acceptance Criteria

- [ ] All Python analyzer files removed from `src/analyzers/python*`
- [ ] All Python-specific modules removed from `src/analysis/python_*`
- [ ] All Python testing modules removed from `src/testing/python/`
- [ ] All Python resource tracking removed from `src/resource/python/`
- [ ] All Python organization modules removed from `src/organization/python/`
- [ ] All Python complexity patterns removed from `src/complexity/languages/python/` and `src/complexity/python_*`
- [ ] All Python debt patterns removed from `src/debt/python_*`
- [ ] All Python extraction patterns removed from `src/extraction_patterns/language_specific/python_*`
- [ ] `rustpython-parser` dependency removed from Cargo.toml
- [ ] Python test files removed from `tests/python_*.rs`
- [ ] Python benchmarks removed from `benches/python_*.rs`
- [ ] Language::Python variant either removed or returns NullAnalyzer
- [ ] `get_analyzer()` function no longer creates Python analyzer
- [ ] All imports of Python modules removed from remaining files
- [ ] `cargo build` completes successfully with no warnings
- [ ] `cargo test` passes all remaining tests
- [ ] `cargo clippy` reports no issues
- [ ] No references to Python in error messages or help text (except historical context)

## Technical Details

### Implementation Approach

**Phase 1: Inventory and Dependencies**
1. Identify all Python-related files using grep/glob patterns
2. Analyze import dependencies to understand impact on remaining code
3. Identify any shared utilities that need to be preserved
4. Create backup branch for reference

**Phase 2: Remove Python Modules**
1. Delete Python analyzer files:
   - `src/analyzers/python.rs`
   - `src/analyzers/python_*.rs`
2. Delete Python analysis modules:
   - `src/analysis/python_*` directories and files
3. Delete Python testing modules:
   - `src/testing/python/` directory
4. Delete Python resource tracking:
   - `src/resource/python/` directory
5. Delete Python organization modules:
   - `src/organization/python/` directory
6. Delete Python complexity patterns:
   - `src/complexity/languages/python/` directory
   - `src/complexity/python_*.rs` files
7. Delete Python debt patterns:
   - `src/debt/python_*.rs` files
8. Delete Python extraction patterns:
   - `src/extraction_patterns/language_specific/python_*.rs`

**Phase 3: Update Core Files**
1. Update `src/analyzers/mod.rs`:
   - Remove Python module declarations
   - Remove Python from `get_analyzer()` factory or return NullAnalyzer
2. Update `src/core/mod.rs` or language definitions:
   - Remove Python language variant or mark as unsupported
3. Update any multi-language test detectors to remove Python patterns

**Phase 4: Remove Dependencies**
1. Remove from Cargo.toml:
   - `rustpython-parser = "0.4"`
2. Update Cargo.lock with `cargo update`

**Phase 5: Clean Up Tests and Benchmarks**
1. Delete Python test files:
   - `tests/python_*.rs`
   - Any Python-specific test cases in other test files
2. Delete Python benchmarks:
   - `benches/python_*.rs`
3. Remove Python benchmark entries from Cargo.toml `[[bench]]` sections

**Phase 6: Validate**
1. Run `cargo build` - must succeed with no warnings
2. Run `cargo test` - all remaining tests must pass
3. Run `cargo clippy` - no new warnings
4. Check for dead code warnings indicating missed cleanup
5. Grep for remaining "python" references (case-insensitive)

### Architecture Changes

- `Language` enum: Python variant removed or returns NullAnalyzer
- Analyzer factory: Python analyzer creation removed
- Module structure: All `python*` modules removed

### Data Structures

No changes to data structures - only removals.

### APIs and Interfaces

- `get_analyzer()` function signature unchanged, but Python language returns NullAnalyzer or error
- `Analyzer` trait unchanged
- No breaking changes to public API for remaining Rust analysis

## Dependencies

- **Prerequisites**: None
- **Affected Components**:
  - All Python analyzer modules
  - Language enum
  - Analyzer factory
  - Test suite
  - Documentation
- **External Dependencies**: Will remove `rustpython-parser` from Cargo.toml

## Testing Strategy

- **Unit Tests**: Remove all Python-specific unit tests
- **Integration Tests**: Remove Python integration tests
- **Regression Tests**: Ensure all remaining Rust tests pass
- **Build Tests**: Verify clean build with no warnings

## Documentation Requirements

- **Code Documentation**: Remove inline docs referencing Python support
- **README.md**: Will be updated in spec 193
- **Architecture Docs**: Will be updated in spec 193
- **Changelog**: Document removal and rationale

## Implementation Notes

- Preserve any language-agnostic patterns that were in Python modules
- Check for shared utilities between Python and other analyzers
- Be thorough with grep searches to catch all references
- Use git grep for accurate searches: `git grep -i python`
- Check for Python in:
  - Error messages
  - Help text
  - Comments
  - Documentation strings
  - Test assertions

## Migration and Compatibility

**Breaking Changes**:
- Users attempting to analyze Python files will receive error or warning
- Any saved analysis results for Python files become invalid
- Configuration files with Python-specific settings will have those settings ignored

**Communication**:
- Clear message in release notes about Python removal
- Explanation of strategic focus on Rust
- Promise to revisit multi-language support after perfecting Rust analysis

**Recommended Error Message**:
```
Error: Python analysis is not currently supported
Debtmap is focusing exclusively on Rust analysis to perfect the core features.
Python support will be reconsidered in future releases once Rust analysis is mature.
```
