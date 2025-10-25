# Implementation Plan: Add Tests and Refactor detect_react_test_issues

## Problem Summary

**Location**: ./src/analyzers/javascript/detectors/testing.rs:detect_react_test_issues:293
**Priority Score**: 25.025
**Debt Type**: TestingGap (0% coverage, cyclomatic complexity 10, cognitive complexity 27)

**Current Metrics**:
- Lines of Code: 63
- Cyclomatic Complexity: 10
- Cognitive Complexity: 27
- Coverage: 0.0%
- Nesting Depth: 4

**Issue**: Complex business logic with 100% testing gap. Cyclomatic complexity of 10 requires at least 10 test cases for full path coverage. After extracting 6 functions, each will need only 3-5 tests. Testing before refactoring ensures no regressions.

**Uncovered Lines**: 293, 300-304, 306-311, 314-315, 317-318, 320, 324-325, 327-328, 334, 338-339, 341-342, 348-352 (31 total lines)

## Target State

**Expected Impact** (from debtmap):
- Complexity Reduction: 3.0
- Coverage Improvement: 50.0%
- Risk Reduction: 10.5105

**Success Criteria**:
- [ ] Achieve 80%+ test coverage for detect_react_test_issues and extracted functions
- [ ] Reduce cyclomatic complexity from 10 to ≤7 through function extraction
- [ ] Extract 4-6 pure functions with complexity ≤3 each
- [ ] All existing tests continue to pass
- [ ] No clippy warnings
- [ ] Proper formatting with cargo fmt

## Implementation Phases

### Phase 1: Add Comprehensive Tests for Current Implementation

**Goal**: Establish baseline test coverage before refactoring to ensure no regressions

**Changes**:
- Create test module for `detect_react_test_issues` function
- Add tests covering all 10 branches:
  1. Test with no render calls → no issues
  2. Test with render but cleanup present → no issues
  3. Test with render and missing cleanup → reports MissingCleanup
  4. Test with multiple renders and insufficient cleanups → reports issue
  5. Test with "mount" calls (render alias) → counts as render
  6. Test with "unmount" in cleanup → counts as cleanup
  7. Test with member expression cleanup calls → counts as cleanup
  8. Test with invalid query syntax → handles gracefully
  9. Test with empty source code → no issues
  10. Test with render count equal to cleanup count → no issues

**Testing**:
```bash
cargo test detect_react_test_issues --lib
cargo tarpaulin --lib --packages debtmap --output-dir coverage --skip-clean -- detect_react_test_issues
```

**Success Criteria**:
- [ ] 10+ test cases covering all branches
- [ ] Tests verify both positive (issue detected) and negative (no issue) cases
- [ ] Coverage for detect_react_test_issues reaches 80%+
- [ ] All tests pass
- [ ] Ready to commit

### Phase 2: Extract Query Building Logic

**Goal**: Extract pure query string construction and query object creation into testable functions

**Changes**:
- Extract `build_render_query()` - returns the render query string
- Extract `build_cleanup_query()` - returns the cleanup query string
- Extract `create_query(language: &Language, query_str: &str) -> Result<Query, QueryError>` - creates Query from string
- Update `detect_react_test_issues` to use extracted functions

**Extracted Functions**:
```rust
fn build_render_query() -> &'static str {
    r#"(call_expression function: (identifier) @func) @render_call"#
}

fn build_cleanup_query() -> &'static str {
    r#"(call_expression function: [...]) @cleanup_call"#
}

fn create_query(language: &Language, query_str: &str) -> Result<Query, QueryError> {
    Query::new(language, query_str)
}
```

**Testing**:
- Test `build_render_query()` returns correct query string
- Test `build_cleanup_query()` returns correct query string
- Test `create_query()` with valid and invalid query strings
- Ensure original tests still pass

**Success Criteria**:
- [ ] 3 new pure functions extracted
- [ ] Each function has complexity ≤2
- [ ] 6+ new unit tests for extracted functions
- [ ] All existing tests pass
- [ ] No clippy warnings
- [ ] Ready to commit

### Phase 3: Extract Function Name Validation Logic

**Goal**: Extract conditional logic for validating render and cleanup function names

**Changes**:
- Extract `is_render_function(name: &str) -> bool` - checks if name is render/mount
- Extract `is_cleanup_function(name: &str) -> bool` - checks if name is cleanup/unmount
- Update match processing logic to use these functions

**Extracted Functions**:
```rust
fn is_render_function(name: &str) -> bool {
    name == "render" || name == "mount"
}

fn is_cleanup_function(name: &str) -> bool {
    name == "cleanup" || name == "unmount" || name.contains("unmount")
}
```

**Testing**:
- Test `is_render_function()` with "render", "mount", "other" → true, true, false
- Test `is_cleanup_function()` with "cleanup", "unmount", "componentWillUnmount", "render" → true, true, true, false
- Ensure original detection tests still pass

**Success Criteria**:
- [ ] 2 new pure functions extracted
- [ ] Each function has complexity ≤2
- [ ] 8+ unit tests covering edge cases
- [ ] All existing tests pass
- [ ] Complexity of detect_react_test_issues reduced by ~2
- [ ] Ready to commit

### Phase 4: Extract Query Match Processing Logic

**Goal**: Extract the complex query matching and counting logic into separate functions

**Changes**:
- Extract `count_render_calls(root: Node, source: &str, language: &Language) -> usize`
- Extract `count_cleanup_calls(root: Node, source: &str, language: &Language) -> usize`
- Update `detect_react_test_issues` to use these counting functions

**Extracted Functions**:
```rust
fn count_render_calls(root: Node, source: &str, language: &Language) -> usize {
    // Contains query creation, cursor management, and counting logic
}

fn count_cleanup_calls(root: Node, source: &str, language: &Language) -> usize {
    // Contains query creation, cursor management, and counting logic
}
```

**Testing**:
- Test `count_render_calls()` with various AST structures
- Test `count_cleanup_calls()` with various AST structures
- Test with empty/invalid nodes
- Ensure original detection tests still pass

**Success Criteria**:
- [ ] 2 new functions extracted with single responsibility
- [ ] Each function has complexity ≤4
- [ ] 10+ integration tests with realistic AST inputs
- [ ] All existing tests pass
- [ ] Complexity of detect_react_test_issues reduced to ≤7
- [ ] Ready to commit

### Phase 5: Final Cleanup and Validation

**Goal**: Ensure all quality gates pass and verify improvement metrics

**Changes**:
- Add documentation for all extracted functions
- Run full test suite and coverage analysis
- Verify complexity metrics with debtmap
- Update any related documentation

**Testing**:
```bash
# Full CI validation
just ci

# Coverage verification (should be 80%+)
cargo tarpaulin --lib --output-dir coverage --skip-clean

# Complexity verification
debtmap analyze --format json > after.json
jq '.items[] | select(.location.file == "src/analyzers/javascript/detectors/testing.rs" and .location.function == "detect_react_test_issues")' after.json
```

**Success Criteria**:
- [ ] Overall coverage for module reaches 80%+
- [ ] Cyclomatic complexity reduced from 10 to ≤7
- [ ] 4-6 pure functions extracted with complexity ≤3 each
- [ ] All 30+ tests pass
- [ ] No clippy warnings
- [ ] Properly formatted
- [ ] Documentation complete
- [ ] Debtmap shows improvement in unified score

## Testing Strategy

**For each phase**:
1. Write tests FIRST (TDD approach)
2. Run `cargo test --lib -- detect_react_test_issues` to verify phase tests
3. Run `cargo test --lib` to verify no regressions
4. Run `cargo clippy --all-targets --all-features -- -D warnings`
5. Run `cargo fmt --all -- --check`

**Phase-specific testing**:
- **Phase 1**: Focus on branch coverage - ensure all 10 branches covered
- **Phase 2**: Test pure query building functions in isolation
- **Phase 3**: Test validation functions with edge cases (empty strings, special chars)
- **Phase 4**: Test counting functions with realistic tree-sitter AST structures
- **Phase 5**: Full integration testing and coverage analysis

**Final verification**:
1. `just ci` - Full CI checks must pass
2. `cargo tarpaulin --lib` - Regenerate coverage (target 80%+)
3. `debtmap analyze` - Verify improvement in metrics

## Rollback Plan

If a phase fails:
1. Review the test failures carefully - they indicate regressions
2. If tests can be fixed quickly (< 10 minutes), fix and continue
3. If fundamental issue discovered:
   - Revert the phase with `git reset --hard HEAD~1`
   - Document what went wrong
   - Adjust the plan
   - Retry with modified approach

If stuck after 3 attempts on any phase:
1. Document the blocker
2. Research alternative approaches
3. Consider breaking the phase into smaller sub-phases
4. Ask for guidance if needed

## Notes

**Key Insights**:
- This function has 0% coverage despite being business logic (marked as PureLogic role)
- High cognitive complexity (27) suggests nested conditionals that should be extracted
- Pattern repetition score of 0.8 indicates some duplicate logic that can be shared
- Function uses tree-sitter queries - need to understand query syntax for testing
- Need realistic tree-sitter AST fixtures for integration tests

**Gotchas**:
- Tree-sitter Query objects require valid syntax - invalid queries return Err
- QueryCursor is stateful - ensure proper initialization in tests
- Node text extraction depends on source bytes - need valid source in tests
- Member expression matching is more complex than identifier matching

**Testing Approach**:
- Phase 1 establishes comprehensive test coverage on existing code
- Phases 2-4 incrementally extract and test pure functions
- Each extraction maintains test coverage while reducing complexity
- TDD approach: write tests first, then refactor

**Functional Programming Alignment**:
- Extract pure query builders (static strings)
- Extract pure validators (string -> bool)
- Separate I/O concerns (query execution) from logic (validation)
- Final structure: pure functions + thin orchestration layer
