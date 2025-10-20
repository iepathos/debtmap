# Implementation Plan: Add Tests and Refactor detect_timing_dependent_tests

## Problem Summary

**Location**: ./src/analyzers/javascript/detectors/testing.rs:detect_timing_dependent_tests:245
**Priority Score**: 24.0
**Debt Type**: TestingGap (0% direct coverage, 42.86% transitive coverage)

**Current Metrics**:
- Lines of Code: 47
- Cyclomatic Complexity: 7
- Cognitive Complexity: 28
- Nesting Depth: 6
- Coverage: 0% (direct), 42.86% (transitive)
- Uncovered Lines: 245, 251-256, 258-259, 261, 265-266, 270-272, 279

**Issue**: Complex business logic with 100% direct coverage gap. Function has high cognitive complexity (28) due to deep nesting and multiple responsibilities. Cyclomatic complexity of 7 requires at least 7 test cases for full path coverage.

**Recommendation**: Add 7 tests for 100% coverage gap, then refactor complexity 7 into 6 functions. Testing before refactoring ensures no regressions.

## Target State

**Expected Impact**:
- Complexity Reduction: 2.1
- Coverage Improvement: 50.0%
- Risk Reduction: 10.08

**Success Criteria**:
- [ ] Direct coverage increases from 0% to 80%+
- [ ] Cyclomatic complexity reduced from 7 to ≤3 per function
- [ ] Cognitive complexity reduced from 28 to ≤10 per function
- [ ] 6+ pure functions extracted with clear single responsibilities
- [ ] All existing tests continue to pass
- [ ] No clippy warnings
- [ ] Proper formatting (cargo fmt)

## Implementation Phases

### Phase 1: Add Comprehensive Test Coverage

**Goal**: Achieve 80%+ coverage of the current function before refactoring to ensure no regressions during refactoring.

**Changes**:
- Add test module for `detect_timing_dependent_tests` function
- Write 7+ test cases covering all branches:
  1. Test with setTimeout dependency detected
  2. Test with setInterval dependency detected
  3. Test with Date.now() dependency detected
  4. Test with Math.random() dependency detected
  5. Test with performance.now() dependency detected
  6. Test with no timing dependencies (negative case)
  7. Test with invalid query (error handling path)
  8. Test with non-test function (early exit path)
- Follow existing test patterns in the file (using tree_sitter::Parser)

**Testing**:
```bash
cargo test --lib detect_timing_dependent_tests
cargo tarpaulin --out Stdout --include-tests -- detect_timing_dependent_tests
```

**Success Criteria**:
- [ ] All 7+ tests pass
- [ ] Coverage for detect_timing_dependent_tests ≥ 80%
- [ ] Tests are clear and self-documenting
- [ ] No clippy warnings
- [ ] Ready to commit

### Phase 2: Extract Query Building Pure Function

**Goal**: Separate query construction into a pure, testable function to reduce complexity.

**Changes**:
- Extract query string building into `build_test_query() -> &'static str`
- This is a pure function with zero complexity
- Update `detect_timing_dependent_tests` to call this function
- Reduces cognitive load by removing nested string literal

**Testing**:
```bash
cargo test --lib build_test_query
cargo test --lib detect_timing_dependent_tests  # Ensure no regression
```

**Success Criteria**:
- [ ] New function is pure (no side effects)
- [ ] All existing tests pass
- [ ] Complexity reduced slightly
- [ ] Ready to commit

### Phase 3: Extract Test Function Matching Logic

**Goal**: Extract the logic for finding test function calls into a pure function.

**Changes**:
- Create `find_test_function_capture<'a>(captures: &'a [Capture]) -> Option<&'a Capture>`
- Extracts the `.find(|c| c.index == 0)` logic
- This is a pure function focused on a single responsibility
- Update main function to use this helper
- Add 3-5 unit tests for this pure function

**Testing**:
```bash
cargo test --lib find_test_function_capture
cargo test --lib detect_timing_dependent_tests
```

**Success Criteria**:
- [ ] New function has cyclomatic complexity ≤ 2
- [ ] New function has 80%+ coverage
- [ ] All existing tests pass
- [ ] Reduced nesting in main function
- [ ] Ready to commit

### Phase 4: Extract Test Name and Body Extraction

**Goal**: Separate the logic for extracting test name and body into a focused function.

**Changes**:
- Create `extract_test_details<'a>(captures: &'a [Capture], source: &'a str) -> Option<(String, String)>`
- Extracts the logic for finding name (index 1) and body (index 2) captures
- Handles text extraction and trimming
- Returns tuple of (test_name, body_text) or None
- Add 5+ unit tests covering:
  - Both name and body found
  - Missing name
  - Missing body
  - Various quote styles (", ')

**Testing**:
```bash
cargo test --lib extract_test_details
cargo test --lib detect_timing_dependent_tests
```

**Success Criteria**:
- [ ] New function has cyclomatic complexity ≤ 3
- [ ] New function has 85%+ coverage
- [ ] All existing tests pass
- [ ] Main function is less nested
- [ ] Ready to commit

### Phase 5: Extract Timing Issue Creation Logic

**Goal**: Separate the creation of TestingAntiPattern instances into a pure function.

**Changes**:
- Create `create_timing_issue(body_node: Node, test_name: String, timing_type: String) -> TestingAntiPattern`
- Pure function that constructs the enum variant
- Single responsibility: data construction
- Add 3 unit tests with different timing types

**Testing**:
```bash
cargo test --lib create_timing_issue
cargo test --lib detect_timing_dependent_tests
```

**Success Criteria**:
- [ ] New function is pure with complexity = 1
- [ ] New function has 100% coverage
- [ ] All existing tests pass
- [ ] Main function focuses on orchestration only
- [ ] Ready to commit

### Phase 6: Final Refactoring - Functional Pipeline

**Goal**: Transform the main function into a functional pipeline that orchestrates pure functions.

**Changes**:
- Refactor `detect_timing_dependent_tests` into a clean functional pipeline:
  1. Build query
  2. Execute query
  3. Map over matches
  4. Filter to test functions
  5. Extract test details
  6. Detect timing issues
  7. Collect results
- Main function becomes orchestration with minimal branching
- Target cyclomatic complexity ≤ 3
- All business logic delegated to pure functions

**Final Structure**:
```rust
fn detect_timing_dependent_tests(
    root: Node,
    source: &str,
    language: &tree_sitter::Language,
    issues: &mut Vec<TestingAntiPattern>,
) {
    let query = build_test_query();

    if let Ok(query) = Query::new(language, query) {
        let matches = execute_query(&query, root, source);

        for match_ in matches {
            if let Some(issue) = process_test_match(&match_, source) {
                issues.push(issue);
            }
        }
    }
}

// Helper that chains the pure functions
fn process_test_match(match_: &QueryMatch, source: &str) -> Option<TestingAntiPattern> {
    find_test_function_capture(&match_.captures)
        .filter(|c| is_test_function(get_node_text(c.node, source)))
        .and_then(|_| extract_test_details(&match_.captures, source))
        .and_then(|(name, body)| {
            detect_timing_dependency(&body)
                .map(|timing| create_timing_issue(match_.captures[2].node, name, timing))
        })
}
```

**Testing**:
```bash
cargo test --lib detect_timing_dependent_tests
cargo test --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt --all -- --check
```

**Success Criteria**:
- [ ] Main function cyclomatic complexity ≤ 3
- [ ] Main function cognitive complexity ≤ 10
- [ ] All tests pass (including original integration tests)
- [ ] Coverage maintained at 80%+
- [ ] No clippy warnings
- [ ] Code is more readable and maintainable
- [ ] Ready to commit

## Testing Strategy

**For each phase**:
1. Run `cargo test --lib` to verify existing tests pass
2. Run `cargo clippy` to check for warnings
3. Run phase-specific tests to verify new functionality
4. Run `cargo fmt` to ensure proper formatting

**Final verification**:
1. `cargo test --all-features` - All tests pass
2. `cargo clippy --all-targets --all-features -- -D warnings` - No warnings
3. `cargo fmt --all -- --check` - Properly formatted
4. `cargo tarpaulin --out Stdout` - Verify coverage improvement
5. Compare before/after metrics:
   - Coverage: 0% → 80%+
   - Cyclomatic: 7 → ≤3 per function
   - Cognitive: 28 → ≤10 per function

## Rollback Plan

If a phase fails:
1. Revert the phase with `git reset --hard HEAD~1`
2. Review the failure and test output
3. Identify the issue (logic error, missed edge case, etc.)
4. Adjust the implementation approach
5. Retry the phase with corrections

If tests start failing unexpectedly:
1. Check if tree-sitter parsing changed
2. Verify test data is correct
3. Ensure helper functions match expected signatures
4. Review any dependency changes

## Notes

### Key Refactoring Principles

1. **Test First**: Write comprehensive tests before refactoring to catch regressions
2. **Extract Pure Functions**: Each extracted function should be pure with no side effects
3. **Single Responsibility**: Each function does exactly one thing
4. **Functional Composition**: Chain pure functions together rather than nesting
5. **Minimal Complexity**: Target cyclomatic complexity ≤ 3 per function

### Current Function Analysis

The function has several responsibilities mixed together:
- Query construction (lines 251-259)
- Query execution (lines 261-263)
- Match iteration (line 265)
- Test function identification (lines 266-269)
- Test details extraction (lines 270-277)
- Timing dependency detection (line 279)
- Issue creation (lines 280-284)

Each of these should be a separate, testable function.

### Edge Cases to Test

- Empty source code
- Malformed test syntax
- Tests with multiple timing dependencies
- Nested test calls (describe within describe)
- TypeScript vs JavaScript differences
- Various quote styles (" vs ')
- Tests without bodies
- Non-test functions with similar patterns

### Performance Considerations

- Query parsing happens once per call (acceptable)
- Match iteration is lazy (good)
- No unnecessary allocations in hot path
- Pure functions enable potential memoization

### Dependencies

This function depends on:
- `tree_sitter` for AST querying
- `get_node_text` helper (line 4)
- `is_test_function` helper (line 451)
- `detect_timing_dependency` helper (line 495)
- `SourceLocation::from_node` (from mod.rs)

All dependencies are stable and well-tested.
