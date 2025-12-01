---
number: 192
title: Remove JavaScript and TypeScript Language Support
category: foundation
priority: high
status: draft
dependencies: [191]
created: 2025-11-30
---

# Specification 192: Remove JavaScript and TypeScript Language Support

**Category**: foundation
**Priority**: high
**Status**: draft
**Dependencies**: [191]

## Context

Similar to Python, debtmap provides partial JavaScript and TypeScript analysis support via tree-sitter. To maintain focus on becoming the best Rust code analyzer, JavaScript and TypeScript support needs to be removed from the codebase. This spec should be executed after Python removal (spec 191) to avoid conflicts and allow learning from that process.

## Objective

Remove all JavaScript and TypeScript language analysis functionality from debtmap, including tree-sitter integration, analyzers, detectors, tests, benchmarks, and documentation references, while maintaining codebase stability.

## Requirements

### Functional Requirements

- Remove JavaScript/TypeScript analyzer modules and all JS/TS-specific analysis code
- Remove tree-sitter dependencies (`tree-sitter`, `tree-sitter-javascript`, `tree-sitter-typescript`) from Cargo.toml
- Remove JavaScript/TypeScript-specific test files and test cases
- Remove JavaScript/TypeScript-specific benchmarks if any exist
- Remove JavaScript/TypeScript language variants from Language enum (or mark as unsupported)
- Update analyzer factory to not return JS/TS analyzers
- Clean up any JS/TS-specific configuration options

### Non-Functional Requirements

- All remaining tests must pass after removal
- No broken references or imports in remaining code
- Cargo build must succeed with no warnings
- Code coverage should not decrease for Rust analysis features
- Documentation must be updated to reflect Rust-only support

## Acceptance Criteria

- [ ] All JavaScript analyzer files removed from `src/analyzers/javascript/`
- [ ] JavaScript module removed from `src/analyzers/mod.rs`
- [ ] JavaScript organization module removed from `src/organization/javascript/`
- [ ] JavaScript complexity patterns removed from `src/complexity/languages/javascript.rs`
- [ ] Tree-sitter dependencies removed from Cargo.toml:
  - `tree-sitter`
  - `tree-sitter-javascript`
  - `tree-sitter-typescript`
- [ ] JavaScript/TypeScript test files removed if they exist
- [ ] JavaScript/TypeScript benchmarks removed if they exist
- [ ] Language::JavaScript and Language::TypeScript variants either removed or return NullAnalyzer
- [ ] `get_analyzer()` function no longer creates JavaScript/TypeScript analyzers
- [ ] All imports of JavaScript modules removed from remaining files
- [ ] `cargo build` completes successfully with no warnings
- [ ] `cargo test` passes all remaining tests
- [ ] `cargo clippy` reports no issues
- [ ] No references to JavaScript/TypeScript in error messages or help text (except historical context)

## Technical Details

### Implementation Approach

**Phase 1: Inventory and Dependencies**
1. Identify all JavaScript/TypeScript-related files using grep/glob patterns
2. Analyze import dependencies to understand impact on remaining code
3. Identify tree-sitter usage scope
4. Reference approach from Python removal (spec 191)

**Phase 2: Remove JavaScript/TypeScript Modules**
1. Delete JavaScript analyzer directory:
   - `src/analyzers/javascript/` (entire directory)
2. Delete JavaScript organization module:
   - `src/organization/javascript/` directory
3. Delete JavaScript complexity patterns:
   - `src/complexity/languages/javascript.rs`
4. Check for any JS/TS extraction patterns:
   - `src/extraction_patterns/language_specific/javascript_*.rs` (if exists)

**Phase 3: Update Core Files**
1. Update `src/analyzers/mod.rs`:
   - Remove JavaScript module declaration
   - Remove JavaScript/TypeScript from `get_analyzer()` factory or return NullAnalyzer
   - Remove `create_js_analyzer()` helper function
2. Update language definitions:
   - Remove JavaScript/TypeScript language variants or mark as unsupported

**Phase 4: Remove Dependencies**
1. Remove from Cargo.toml:
   - `tree-sitter = "0.25"`
   - `tree-sitter-javascript = "0.25"`
   - `tree-sitter-typescript = "0.23"`
2. Update Cargo.lock with `cargo update`

**Phase 5: Clean Up Tests and Benchmarks**
1. Search for and delete any JavaScript/TypeScript test files:
   - `tests/javascript_*.rs`
   - `tests/typescript_*.rs`
2. Remove any JavaScript/TypeScript test cases in multi-language test files
3. Delete any JavaScript/TypeScript benchmarks if they exist

**Phase 6: Validate**
1. Run `cargo build` - must succeed with no warnings
2. Run `cargo test` - all remaining tests must pass
3. Run `cargo clippy` - no new warnings
4. Check for dead code warnings indicating missed cleanup
5. Grep for remaining "javascript" and "typescript" references (case-insensitive)

### Architecture Changes

- `Language` enum: JavaScript and TypeScript variants removed or return NullAnalyzer
- Analyzer factory: JavaScript/TypeScript analyzer creation removed
- Module structure: All `javascript*` and `typescript*` modules removed
- tree-sitter integration completely removed

### Data Structures

No changes to data structures - only removals.

### APIs and Interfaces

- `get_analyzer()` function signature unchanged, but JavaScript/TypeScript languages return NullAnalyzer or error
- `Analyzer` trait unchanged
- No breaking changes to public API for remaining Rust analysis

## Dependencies

- **Prerequisites**: Spec 191 (Remove Python Support) should be completed first
- **Affected Components**:
  - All JavaScript/TypeScript analyzer modules
  - Language enum
  - Analyzer factory
  - Test suite
  - Documentation
- **External Dependencies**: Will remove tree-sitter dependencies from Cargo.toml

## Testing Strategy

- **Unit Tests**: Remove all JavaScript/TypeScript-specific unit tests
- **Integration Tests**: Remove JavaScript/TypeScript integration tests
- **Regression Tests**: Ensure all remaining Rust tests pass
- **Build Tests**: Verify clean build with no warnings

## Documentation Requirements

- **Code Documentation**: Remove inline docs referencing JavaScript/TypeScript support
- **README.md**: Will be updated in spec 193
- **Architecture Docs**: Will be updated in spec 193
- **Changelog**: Document removal and rationale

## Implementation Notes

- Follow similar process to Python removal (spec 191)
- tree-sitter may have been used only for JS/TS - confirm before removing
- Check if any language-agnostic code depends on tree-sitter
- Be thorough with grep searches to catch all references
- Use git grep for accurate searches: `git grep -i javascript`, `git grep -i typescript`
- Check for JavaScript/TypeScript in:
  - Error messages
  - Help text
  - Comments
  - Documentation strings
  - Test assertions
  - Example code

## Migration and Compatibility

**Breaking Changes**:
- Users attempting to analyze JavaScript/TypeScript files will receive error or warning
- Any saved analysis results for JavaScript/TypeScript files become invalid
- Configuration files with JavaScript/TypeScript-specific settings will have those settings ignored

**Communication**:
- Clear message in release notes about JavaScript/TypeScript removal
- Explanation of strategic focus on Rust
- Promise to revisit multi-language support after perfecting Rust analysis

**Recommended Error Message**:
```
Error: JavaScript/TypeScript analysis is not currently supported
Debtmap is focusing exclusively on Rust analysis to perfect the core features.
JavaScript and TypeScript support will be reconsidered in future releases once Rust analysis is mature.
```
