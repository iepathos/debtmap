# Implementation Plan: Refactor JavaScript Testing Detector Module

## Problem Summary

**Location**: ./src/analyzers/javascript/detectors/testing.rs:file:0
**Priority Score**: 66.90186491047315
**Debt Type**: God Object (File-level)
**Current Metrics**:
- Lines of Code: 2034
- Functions: 87 (82 private, 1 public API, and test functions)
- Cyclomatic Complexity: 204 total, 2.34 average
- Max Complexity: 9
- Coverage: 0% (test code itself, not production code)

**Issue**: This file is a God Object with 2034 lines and 87 functions. It mixes multiple responsibilities:
1. **Pattern Detection** - Main detection functions for different anti-patterns
2. **Query Building** - Tree-sitter query construction
3. **Node Extraction** - AST node extraction helpers
4. **Text Parsing** - Test name and text parsing utilities
5. **Validation** - Helper predicates and checks
6. **Issue Creation** - Anti-pattern issue construction

The recommendation is to split by data flow: 1) Input/parsing functions 2) Core logic/transformation 3) Output/formatting.

## Target State

**Expected Impact** (from debtmap):
- Complexity Reduction: 40.8 points
- Maintainability Improvement: 6.69 points
- Test Effort Reduction: 203.4

**Success Criteria**:
- [x] Split into 3-4 focused modules with <30 functions each
- [x] Each module has a single, clear responsibility
- [x] Public API surface remains unchanged
- [x] All existing tests continue to pass
- [x] No clippy warnings
- [x] Proper formatting with rustfmt

## Implementation Phases

### Phase 1: Create Query and Extraction Utilities Module

**Goal**: Extract Tree-sitter query building and node extraction helpers into a separate module for parsing/input handling.

**Changes**:
- Create `src/analyzers/javascript/detectors/testing/queries.rs`
- Move these functions to the new module:
  - `build_async_test_query()` - Query construction
  - `extract_test_function_name()` - Node extraction
  - `extract_test_name()` - Node extraction
  - `extract_test_body()` - Node extraction
  - `parse_test_name()` - Text parsing
- Add module declaration in parent module or create `mod.rs`
- Make functions public within the module
- Update imports in main testing module

**Testing**:
- Run `cargo test --lib -- testing::queries` to verify query functions work
- Run `cargo test --lib -- javascript::detectors::testing` to ensure main tests pass
- Run `cargo clippy -- -D warnings` to check for issues

**Success Criteria**:
- [x] New module compiles successfully
- [x] 5 functions extracted (reduces main file by ~80 lines)
- [x] All tests pass
- [x] No clippy warnings

### Phase 2: Create Validation and Helper Module

**Goal**: Extract pure validation predicates and helper functions into a separate module.

**Changes**:
- Create `src/analyzers/javascript/detectors/testing/validators.rs`
- Move these helper functions:
  - `is_test_file()` - Path validation
  - `is_test_function()` - Function name validation
  - `is_snapshot_method()` - Method name validation
  - `has_assertions()` - Body content validation
  - `detect_timing_dependency()` - Timing pattern detection
  - `contains_async_operations()` - Async pattern detection
  - `calculate_test_complexity()` - Complexity calculation
  - `count_snapshot_methods()` - Snapshot counting
- Make functions public within module
- Update imports in main testing module

**Testing**:
- Run `cargo test --lib -- testing::validators` to verify validators work
- Run `cargo test --lib -- javascript::detectors::testing` for integration
- Verify all test helper tests still pass

**Success Criteria**:
- [x] New module compiles successfully
- [x] 8 helper functions extracted (reduces main file by ~150 lines)
- [x] All tests pass
- [x] Functions are pure and easily testable

### Phase 3: Create Pattern Detection Module

**Goal**: Extract individual pattern detection functions into a focused module for core detection logic.

**Changes**:
- Create `src/analyzers/javascript/detectors/testing/detectors.rs`
- Move pattern detection functions:
  - `detect_missing_assertions()` - Missing assertion detection
  - `detect_complex_tests()` - Complexity detection
  - `detect_timing_dependent_tests()` - Timing dependency detection
  - `detect_react_test_issues()` - React cleanup detection
  - `detect_async_test_issues()` - Async handling detection
  - `detect_snapshot_overuse()` - Snapshot overuse detection
  - `create_async_test_issue()` - Issue creation helper
- These functions use the validators and query modules
- Make functions public within module
- Update imports in main testing module

**Testing**:
- Run `cargo test --lib -- testing::detectors` to verify detection logic
- Run full test suite: `cargo test --lib -- javascript::detectors::testing`
- Verify each detector function has passing tests

**Success Criteria**:
- [x] New module compiles successfully
- [x] 7 detection functions extracted (reduces main file by ~350 lines)
- [x] All tests pass
- [x] Clear separation of detection concerns

### Phase 4: Reorganize Main Module and Create Module Structure

**Goal**: Create a clean module structure with proper re-exports and a minimal main file.

**Changes**:
- Create `src/analyzers/javascript/detectors/testing/mod.rs` as the module root
- Keep in main module (now `testing/types.rs`):
  - `TestingAntiPattern` enum (40 lines)
  - `TestingAntiPattern::to_debt_item()` impl (74 lines)
  - `detect_testing_patterns()` orchestration function (19 lines)
- Create proper module structure:
  ```
  testing/
  ├── mod.rs           - Public API and re-exports
  ├── types.rs         - TestingAntiPattern enum and conversion
  ├── queries.rs       - Query building and node extraction
  ├── validators.rs    - Helper predicates and validation
  └── detectors.rs     - Pattern detection implementations
  ```
- Move all tests to appropriate modules using `#[cfg(test)]`
- Update `src/analyzers/javascript/detectors/mod.rs` to reference new structure

**Testing**:
- Run `cargo test --lib -- javascript::detectors::testing`
- Run `cargo clippy --all-targets` to verify no warnings
- Run `cargo fmt --all -- --check` to verify formatting
- Verify public API unchanged: `detect_testing_patterns()` still callable

**Success Criteria**:
- [x] Clean module hierarchy with 4-5 focused files
- [x] Main module reduced to ~133 lines (types + orchestration)
- [x] All 87 functions properly distributed
- [x] All tests passing and properly organized
- [x] No clippy warnings or formatting issues

### Phase 5: Final Verification and Documentation

**Goal**: Ensure the refactoring achieves the expected impact and add module documentation.

**Changes**:
- Add module-level documentation to each file explaining its purpose:
  - `mod.rs` - Overview of testing pattern detection
  - `types.rs` - TestingAntiPattern enum and DebtItem conversion
  - `queries.rs` - Tree-sitter query utilities for AST analysis
  - `validators.rs` - Pure validation and helper functions
  - `detectors.rs` - Individual pattern detection implementations
- Add examples in documentation where helpful
- Verify test coverage is maintained
- Run full CI checks: `just ci`
- Generate coverage report: `cargo tarpaulin --lib`
- Analyze with debtmap: `debtmap analyze`

**Testing**:
- Full test suite: `cargo test --all-features`
- Clippy strict mode: `cargo clippy --all-targets --all-features -- -D warnings`
- Documentation build: `cargo doc --no-deps --document-private-items`
- Format check: `cargo fmt --all -- --check`

**Success Criteria**:
- [x] All module documentation clear and helpful
- [x] Full CI passes with no warnings
- [x] Test coverage maintained or improved
- [x] Debtmap shows improvement in metrics
- [x] All phases verified and committed

## Testing Strategy

**For each phase**:
1. After code changes, run targeted tests: `cargo test --lib -- <module_path>`
2. Run full test suite: `cargo test --all-features`
3. Check for warnings: `cargo clippy --all-targets -- -D warnings`
4. Verify formatting: `cargo fmt --all -- --check`
5. Commit with clear message describing the phase

**Final verification**:
1. `just ci` - Full CI checks (tests, clippy, fmt, deny)
2. `cargo tarpaulin --lib` - Coverage report
3. `debtmap analyze --output .prodigy/debt-after-refactor.json` - Verify improvement
4. Compare metrics: complexity should drop by ~40 points, file should be split

## Rollback Plan

If a phase fails:
1. Identify the specific error from test output or clippy
2. Fix the issue in the current phase (don't move to next phase)
3. If unfixable after 3 attempts:
   - Document the blocker in this plan
   - Revert with `git reset --hard HEAD~1`
   - Reassess the approach
   - Consider alternative extraction strategy

## Module Size Targets

After refactoring, expected file sizes:
- `types.rs`: ~120 lines (enum + impl + tests)
- `queries.rs`: ~100 lines (5 functions + tests)
- `validators.rs`: ~200 lines (8 functions + tests)
- `detectors.rs`: ~400 lines (7 detection functions + tests)
- `mod.rs`: ~30 lines (module declarations + re-exports)
- Total: ~850 lines of production code + tests distributed

This represents splitting the original 2034-line file into 5 focused modules, with the largest module having <30 functions and clear single responsibility.

## Notes

**Key Design Principles**:
- **Functional separation**: Each module has a single, clear purpose
- **Data flow**: queries → validators → detectors → types
- **Pure functions**: Validators and queries have no side effects
- **Minimal coupling**: Modules depend only on what they need
- **Testability**: Each module can be tested independently

**Migration Strategy**:
- Use `pub(crate)` or `pub(super)` for inter-module visibility
- Keep `detect_testing_patterns()` as the only public API
- Tests remain in `#[cfg(test)]` blocks within each module
- No changes to external callers required

**Risks and Mitigations**:
- **Risk**: Breaking test utilities used across modules
  - **Mitigation**: Move shared test utilities to a `test_utils` module if needed
- **Risk**: Complex interdependencies between functions
  - **Mitigation**: Extract pure functions first, then functions that depend on them
- **Risk**: Test failures due to import changes
  - **Mitigation**: Update imports incrementally, run tests after each change

This refactoring transforms a 2034-line God Object into a well-organized module hierarchy following functional programming principles and the project's architecture guidelines.
