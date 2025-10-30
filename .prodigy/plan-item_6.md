# Implementation Plan: Split Large Test File into Focused Modules

## Problem Summary

**Location**: ./src/priority/semantic_classifier.rs:file:0
**Priority Score**: 58.53
**Debt Type**: File-level complexity (God Object - Large Test File)
**Current Metrics**:
- Lines of Code: 2021
- Functions: 86 (27 implementation + 59 test functions)
- Cyclomatic Complexity: 187 total, 2.17 avg, 13 max
- Coverage: 0% (this is test code itself)

**Issue**: This test file has grown too large with 2021 lines and 86 functions. It contains production code (classification logic and helper functions) mixed with extensive test coverage. The file should be split into focused modules for better maintainability.

## Target State

**Expected Impact**:
- Complexity Reduction: 37.4
- Maintainability Improvement: 5.85
- Test Effort: 202.1

**Success Criteria**:
- [ ] Production code separated from test code
- [ ] Test file reduced to <500 lines with clear organization
- [ ] Each module has <30 functions
- [ ] All existing tests continue to pass
- [ ] No clippy warnings
- [ ] Proper formatting with `cargo fmt`

## Implementation Phases

### Phase 1: Extract Pattern Matching Helpers

**Goal**: Extract pattern matching helper functions (debug, accessor, constructor name matching) into a dedicated module.

**Changes**:
- Create `src/priority/semantic_classifier/pattern_matchers.rs`
- Move pattern matching functions:
  - `matches_debug_pattern`
  - `matches_output_io_pattern`
  - `matches_accessor_name`
  - `is_entry_point_by_name`
  - `is_orchestrator_by_name`
- Update imports in `semantic_classifier.rs`

**Testing**:
- Run `cargo test --lib semantic_classifier` to verify tests pass
- Run `cargo clippy` to check for warnings

**Success Criteria**:
- [ ] Pattern matchers extracted to separate module
- [ ] All tests pass
- [ ] Main file reduced by ~150 lines
- [ ] Ready to commit

### Phase 2: Extract Classification Rules

**Goal**: Extract the core classification rule functions into a focused module.

**Changes**:
- Create `src/priority/semantic_classifier/classifiers.rs`
- Move classification functions:
  - `is_debug_function`
  - `has_diagnostic_characteristics`
  - `is_simple_constructor`
  - `is_constructor_enhanced`
  - `is_enum_converter_enhanced`
  - `is_accessor_method`
  - `is_pattern_matching_function`
  - `is_orchestrator`
  - `is_io_wrapper`
  - `is_io_orchestration`
  - `calculate_delegation_ratio`
  - `delegates_to_tested_functions`
  - `contains_io_patterns`
- Update imports in `semantic_classifier.rs`

**Testing**:
- Run `cargo test --lib semantic_classifier` to verify tests pass
- Run `cargo clippy` to check for warnings

**Success Criteria**:
- [ ] Classification rules extracted to separate module
- [ ] All tests pass
- [ ] Main file reduced by ~400 lines
- [ ] Ready to commit

### Phase 3: Extract AST Analysis Helpers

**Goal**: Extract AST-specific analysis functions into a dedicated module.

**Changes**:
- Create `src/priority/semantic_classifier/ast_analysis.rs`
- Move AST analysis functions:
  - `is_simple_accessor_body`
  - `is_simple_accessor_expr`
  - `is_simple_accessor_method`
  - `has_immutable_self_receiver`
  - `is_simple_binding_pattern`
- Update imports in `classifiers.rs` and `semantic_classifier.rs`

**Testing**:
- Run `cargo test --lib semantic_classifier` to verify tests pass
- Run `cargo clippy` to check for warnings

**Success Criteria**:
- [ ] AST analysis helpers extracted to separate module
- [ ] All tests pass
- [ ] Main file reduced by ~150 lines
- [ ] Ready to commit

### Phase 4: Reorganize Tests by Feature

**Goal**: Split the test module into focused test files by feature area.

**Changes**:
- Create test submodules within `src/priority/semantic_classifier/`:
  - `tests/entry_point_tests.rs` - Entry point classification tests
  - `tests/orchestrator_tests.rs` - Orchestrator classification tests
  - `tests/io_wrapper_tests.rs` - I/O wrapper classification tests
  - `tests/constructor_tests.rs` - Constructor detection tests
  - `tests/enum_converter_tests.rs` - Enum converter detection tests
  - `tests/accessor_tests.rs` - Accessor method detection tests
  - `tests/debug_tests.rs` - Debug function detection tests
  - `tests/integration_tests.rs` - Integration and helper function tests
- Move test helper function `create_test_metrics` to shared test utilities
- Update module structure to support test submodules

**Testing**:
- Run `cargo test --lib semantic_classifier` to verify all tests pass
- Run `cargo clippy` to check for warnings

**Success Criteria**:
- [ ] Tests split into 8 focused files, each <200 lines
- [ ] All 59 tests pass
- [ ] Main test file removed
- [ ] Test organization follows feature areas
- [ ] Ready to commit

### Phase 5: Convert to Module Directory Structure

**Goal**: Finalize the module structure and ensure clean public API.

**Changes**:
- Convert `semantic_classifier.rs` to `semantic_classifier/mod.rs`
- Keep only public API in `mod.rs`:
  - `FunctionRole` enum
  - `classify_function_role` function
  - `get_role_multiplier` function
  - `classify_by_rules` (if needed publicly)
- Use `mod` declarations to organize submodules
- Ensure proper visibility modifiers (`pub(crate)` vs `pub`)
- Add module-level documentation

**Testing**:
- Run `cargo test --lib semantic_classifier` to verify tests pass
- Run `just ci` for full CI checks
- Run `cargo doc` to verify documentation builds

**Success Criteria**:
- [ ] Module converted to directory structure
- [ ] Clean public API with only 3-4 public items
- [ ] All tests pass
- [ ] Documentation builds without warnings
- [ ] Ready to commit

## Testing Strategy

**For each phase**:
1. Run `cargo test --lib semantic_classifier` to verify existing tests pass
2. Run `cargo clippy` to check for warnings
3. Run `cargo fmt` to ensure proper formatting
4. Verify imports are correct and no unused imports remain

**Final verification**:
1. `just ci` - Full CI checks including all tests
2. `cargo doc --no-deps` - Verify documentation builds
3. Run full test suite: `cargo test --all-features`
4. Verify no regressions in functionality

## Rollback Plan

If a phase fails:
1. Revert the phase with `git reset --hard HEAD~1`
2. Review the failure:
   - Check for missing imports
   - Verify function visibility modifiers
   - Check for circular dependencies
3. Adjust the plan based on the issue
4. Retry with corrections

Common issues to watch for:
- **Circular dependencies**: Ensure dependency graph flows in one direction (patterns → ast_analysis → classifiers → mod)
- **Visibility**: Helper functions should be `pub(crate)`, only API functions `pub`
- **Test organization**: Each test file should be independent and not depend on other test files

## Notes

**Module Dependency Order**:
```
semantic_classifier/mod.rs (public API)
  ├── pattern_matchers.rs (pure pattern matching, no dependencies)
  ├── ast_analysis.rs (depends on syn only)
  └── classifiers.rs (depends on pattern_matchers, ast_analysis)
```

**Functional Programming Approach**:
- All extracted functions are already pure (take inputs, return outputs)
- No mutable state to worry about
- Can be tested independently
- Composition through function calls

**Why This Split**:
- **Pattern matchers**: Pure string/name-based pattern matching (simplest, no dependencies)
- **AST analysis**: Pure AST inspection functions (depends only on syn)
- **Classifiers**: Business logic combining patterns and AST (depends on both)
- **Tests**: Organized by feature for easier maintenance

**Line Count Reduction**:
- Phase 1: ~150 lines (pattern matchers)
- Phase 2: ~400 lines (classifiers)
- Phase 3: ~150 lines (AST analysis)
- Phase 4: ~1100 lines (tests moved to separate files)
- Final main file: ~200 lines (public API only)
