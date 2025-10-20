# Implementation Plan: Add Tests and Refactor detect_async_test_issues

## Problem Summary

**Location**: ./src/analyzers/javascript/detectors/testing.rs:detect_async_test_issues:357
**Priority Score**: 24.0
**Debt Type**: TestingGap (cognitive: 28, coverage: 0.0%, cyclomatic: 7)
**Current Metrics**:
- Lines of Code: 52
- Cyclomatic Complexity: 7
- Cognitive Complexity: 28
- Coverage: 0.0% (direct), 42.86% (transitive)
- Nesting Depth: 6
- Downstream Dependencies: 7

**Issue**: Add 7 tests for 100% coverage gap, then refactor complexity 7 into 6 functions. Complex business logic with 100% gap. Cyclomatic complexity of 7 requires at least 7 test cases for full path coverage. After extracting 6 functions, each will need only 3-5 tests. Testing before refactoring ensures no regressions.

## Target State

**Expected Impact** (from debtmap):
- Complexity Reduction: 2.1
- Coverage Improvement: 50.0%
- Risk Reduction: 10.08

**Success Criteria**:
- [ ] Function has 80%+ test coverage (up from 0%)
- [ ] Cyclomatic complexity reduced from 7 to ≤5
- [ ] Cognitive complexity reduced from 28 to ≤15
- [ ] At least 6 pure functions extracted
- [ ] Each extracted function has complexity ≤3
- [ ] All existing tests continue to pass
- [ ] No clippy warnings
- [ ] Proper formatting with cargo fmt

## Implementation Phases

### Phase 1: Add Comprehensive Tests for Current Implementation

**Goal**: Achieve 100% test coverage for the existing `detect_async_test_issues` function before refactoring to ensure no regressions.

**Changes**:
- Create test module for `detect_async_test_issues`
- Add test for successful path: test function with async operations (fetch) and no await/done
- Add test for test function with axios async operation, no await/done
- Add test for test function with Promise without await/done
- Add test for test function with .then() call without await/done
- Add test for negative case: test function with async operations but has await
- Add test for negative case: test function with async operations but has done() callback
- Add test for non-test function (should not trigger)
- Add test for query parse failure
- Add test for test function without body
- Add edge case tests: empty body, nested async operations, multiple async patterns

**Testing**:
```bash
cargo test --lib test_detect_async_test_issues
cargo test --lib detect_async_test_issues -- --nocapture
cargo tarpaulin --out Html --output-dir coverage -- test_detect_async_test_issues
```

**Success Criteria**:
- [ ] At least 7 test cases covering all branches
- [ ] Tests cover all uncovered lines: 357, 363-370, 373-374, 376, 380-381, 385-387, 395-400
- [ ] All tests pass
- [ ] Coverage for `detect_async_test_issues` reaches 100%
- [ ] Ready to commit

### Phase 2: Extract Pure Query Construction Function

**Goal**: Extract tree-sitter query creation into a pure function with error handling.

**Changes**:
- Create `build_async_test_query() -> Result<Query, QueryError>` function
- Move query string and Query::new() call into this function
- Add test for query construction success
- Add test for invalid query string (error path)
- Update `detect_async_test_issues` to use the extracted function
- Keep function under 10 lines

**Testing**:
```bash
cargo test --lib test_build_async_test_query
cargo test --lib # Ensure no regressions
```

**Success Criteria**:
- [ ] `build_async_test_query` is pure and independently testable
- [ ] Function complexity ≤2
- [ ] All tests pass including new unit tests
- [ ] No clippy warnings
- [ ] Ready to commit

### Phase 3: Extract Capture Node Extraction Logic

**Goal**: Extract the logic that finds and extracts captures from query matches into pure functions.

**Changes**:
- Create `extract_test_function_name(match_: &QueryMatch) -> Option<&Node>` (gets capture index 0)
- Create `extract_test_name(match_: &QueryMatch) -> Option<&Node>` (gets capture index 1)
- Create `extract_test_body(match_: &QueryMatch) -> Option<&Node>` (gets capture index 2)
- Add tests for each extraction function with valid and invalid indices
- Update `detect_async_test_issues` to use these helper functions
- Reduce nesting depth from 6 to ≤4

**Testing**:
```bash
cargo test --lib test_extract_test_function_name
cargo test --lib test_extract_test_name
cargo test --lib test_extract_test_body
cargo test --lib # Ensure no regressions
```

**Success Criteria**:
- [ ] Three pure extraction functions with complexity ≤2 each
- [ ] Nesting depth reduced to 4
- [ ] All tests pass
- [ ] Code is more readable and maintainable
- [ ] Ready to commit

### Phase 4: Extract Test Name Parsing Logic

**Goal**: Extract string trimming and test name parsing into a pure function.

**Changes**:
- Create `parse_test_name(node: Node, source: &str) -> String` function
- Move quote trimming logic into this function
- Add tests for various quote styles: double quotes, single quotes, backticks
- Add tests for strings without quotes
- Add tests for empty strings
- Update main function to use the parser

**Testing**:
```bash
cargo test --lib test_parse_test_name
cargo test --lib # Ensure no regressions
```

**Success Criteria**:
- [ ] `parse_test_name` is pure and handles all quote styles
- [ ] Function complexity ≤2
- [ ] All edge cases tested
- [ ] All tests pass
- [ ] Ready to commit

### Phase 5: Extract Async Issue Creation Logic

**Goal**: Extract the issue creation logic into a pure function.

**Changes**:
- Create `create_async_test_issue(body_node: Node, test_name: String) -> TestingAntiPattern` function
- Move the TestingAntiPattern::AsyncTestIssue construction into this function
- Add tests for issue creation with various inputs
- Update main function to use the creator
- Reduce cyclomatic complexity of main function

**Testing**:
```bash
cargo test --lib test_create_async_test_issue
cargo test --lib # Ensure no regressions
```

**Success Criteria**:
- [ ] `create_async_test_issue` is pure with complexity ≤1
- [ ] Main function complexity reduced to ≤4
- [ ] All tests pass
- [ ] Ready to commit

### Phase 6: Final Refactoring and Validation

**Goal**: Complete refactoring, verify all metrics improved, and ensure comprehensive test coverage.

**Changes**:
- Extract any remaining complex conditional logic
- Ensure main function is now a clean pipeline: query → match → extract → validate → create issue
- Add integration test that runs the full function with a complete test file
- Run full test suite and coverage analysis
- Document the refactored functions
- Add inline comments explaining the flow

**Testing**:
```bash
cargo test --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt --all -- --check
cargo tarpaulin --out Html --output-dir coverage
debtmap analyze --format json > .prodigy/after-refactor.json
```

**Success Criteria**:
- [ ] Main function cyclomatic complexity ≤5 (reduced from 7)
- [ ] Main function cognitive complexity ≤15 (reduced from 28)
- [ ] Overall test coverage for module ≥80%
- [ ] All 6+ extracted functions have complexity ≤3
- [ ] All tests pass
- [ ] No clippy warnings
- [ ] Code is formatted
- [ ] Debtmap shows improvement in metrics
- [ ] Ready to commit

## Testing Strategy

**For each phase**:
1. Write tests first (TDD approach)
2. Run `cargo test --lib` to verify tests fail appropriately
3. Implement the change
4. Run `cargo test --lib` to verify tests pass
5. Run `cargo clippy` to check for warnings
6. Run `cargo fmt` to format code
7. Review coverage with `cargo tarpaulin`

**Final verification**:
1. `cargo test --all-features` - All tests pass
2. `cargo clippy --all-targets --all-features -- -D warnings` - No warnings
3. `cargo fmt --all -- --check` - Code is formatted
4. `cargo tarpaulin --out Html` - Coverage ≥80% for the module
5. `debtmap analyze` - Verify improvement in all metrics

## Rollback Plan

If a phase fails:
1. Review test failures and error messages
2. If the issue is fundamental, revert with `git reset --hard HEAD~1`
3. Re-analyze the approach
4. Adjust the phase plan
5. Retry with updated approach

If tests consistently fail after 3 attempts:
1. Document the issue in comments
2. Seek alternative approach (maybe extract different functions)
3. Consider if the original function structure needs different decomposition

## Notes

**Key Refactoring Targets**:
- The nested if-let chain (lines 380-403) is the main complexity source
- Deep nesting (6 levels) makes code hard to test and understand
- Query parsing, node extraction, and issue creation are separate concerns
- Each concern should be a pure function for easy testing

**Functional Programming Approach**:
- Extract pure functions that take inputs and return outputs
- Keep I/O (tree-sitter queries, node traversal) separate from logic (validation, parsing)
- Use Option and Result types for error handling
- Chain operations functionally rather than nesting

**Coverage Strategy**:
- Test each extracted function independently (unit tests)
- Test the main function with realistic JavaScript test code (integration tests)
- Focus on branch coverage: ensure every if/match arm is tested
- Use property-based testing if patterns emerge

**Dependencies to Preserve**:
- Keep using existing helper functions: `get_node_text`, `is_test_function`, `contains_async_operations`
- Maintain compatibility with `TestingAntiPattern::AsyncTestIssue` enum variant
- Don't change the public API of `detect_async_test_issues`
