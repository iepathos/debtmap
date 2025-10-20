# Implementation Plan: Add Tests and Refactor find_functions_by_path

## Problem Summary

**Location**: ./src/risk/lcov.rs:find_functions_by_path:496
**Priority Score**: 23.7
**Debt Type**: TestingGap
**Current Metrics**:
- Lines of Code: 54
- Cyclomatic Complexity: 3
- Cognitive Complexity: 54
- Coverage: 0% (direct), 50% (transitive)
- Function Role: PureLogic
- Purity Confidence: 0.95

**Issue**: Complex business logic with 100% coverage gap. This is a pure function that implements path matching with 3 different strategies (suffix matching, reverse suffix matching, normalized path equality) and has separate parallel/sequential code paths. The function has significant code duplication between the parallel and sequential branches.

**Rationale**: Cyclomatic complexity of 3 requires at least 3 test cases for full path coverage. The function is marked as pure logic with 95% confidence, making it highly testable. Testing before refactoring ensures no regressions.

## Target State

**Expected Impact**:
- Complexity Reduction: 0.90 (from 3 to ~1-2 per extracted function)
- Coverage Improvement: 50.0% (from 0% to 50%+)
- Risk Reduction: 9.954

**Success Criteria**:
- [ ] 80%+ line coverage for `find_functions_by_path` and extracted functions
- [ ] All 3 matching strategies tested individually
- [ ] Code duplication eliminated between parallel/sequential paths
- [ ] All existing tests continue to pass
- [ ] No clippy warnings
- [ ] Proper formatting with `cargo fmt`
- [ ] Each extracted function has complexity ≤ 3

## Implementation Phases

### Phase 1: Add Comprehensive Tests for Current Implementation

**Goal**: Achieve 100% test coverage of the existing `find_functions_by_path` function before refactoring to ensure we understand its behavior and can detect regressions.

**Changes**:
- Create test module for `find_functions_by_path`
- Add tests for all 3 matching strategies:
  - Strategy 1: Query path ends with LCOV path (`query_path.ends_with(lcov_path)`)
  - Strategy 2: LCOV path ends with normalized query (`lcov_path.ends_with(normalized_query)`)
  - Strategy 3: Normalized paths are equal (`normalize_path(lcov_path) == normalized_query`)
- Test both parallel (>20 items) and sequential (≤20 items) code paths
- Test edge cases: empty maps, single item, boundary case (exactly 20 items)
- Test path normalization edge cases (leading ./, absolute vs relative paths)

**Testing**:
- Run `cargo test find_functions_by_path` to verify new tests pass
- Run `cargo tarpaulin --packages debtmap --lib` to verify coverage improvement
- Expected coverage: 80%+ for this function

**Success Criteria**:
- [ ] At least 8 test cases covering all strategies and both code paths
- [ ] All tests pass
- [ ] Coverage for `find_functions_by_path` reaches 80%+
- [ ] Tests document the expected behavior clearly
- [ ] Ready to commit: "Add comprehensive tests for find_functions_by_path"

### Phase 2: Extract Path Matching Strategy Functions

**Goal**: Eliminate code duplication by extracting the 3 matching strategies into separate pure functions. This reduces cognitive complexity and makes each strategy independently testable.

**Changes**:
- Extract `matches_suffix_strategy(query_path: &Path, lcov_path: &Path) -> bool`
  - Implements: `query_path.ends_with(lcov_path)`
- Extract `matches_reverse_suffix_strategy(query_path: &Path, lcov_path: &Path) -> bool`
  - Implements: `lcov_path.ends_with(normalize_path(query_path))`
- Extract `matches_normalized_equality_strategy(query_path: &Path, lcov_path: &Path) -> bool`
  - Implements: `normalize_path(lcov_path) == normalize_path(query_path)`
- Refactor `find_functions_by_path` to use these extracted functions
- Each strategy function should be 3-5 lines max

**Testing**:
- Add unit tests for each extracted strategy function (3-5 tests each)
- Verify all existing tests still pass
- Run `cargo clippy` to ensure no warnings

**Success Criteria**:
- [ ] 3 new pure functions extracted, each with complexity ≤ 2
- [ ] Each strategy function has 80%+ coverage
- [ ] Code duplication eliminated between parallel/sequential paths
- [ ] All existing tests still pass
- [ ] No clippy warnings
- [ ] Ready to commit: "Extract path matching strategies into pure functions"

### Phase 3: Refactor find_functions_by_path for Clarity

**Goal**: Simplify the main function by using a unified approach that conditionally chooses parallel vs sequential iteration, eliminating the branch duplication entirely.

**Changes**:
- Create a helper function `apply_matching_strategies` that takes an iterator (parallel or sequential) and applies all 3 strategies in order
- Refactor `find_functions_by_path` to:
  ```rust
  fn find_functions_by_path<'a>(
      functions: &'a HashMap<PathBuf, Vec<FunctionCoverage>>,
      query_path: &Path,
  ) -> Option<&'a Vec<FunctionCoverage>> {
      if functions.len() > 20 {
          apply_matching_strategies(functions.par_iter(), query_path)
      } else {
          apply_matching_strategies(functions.iter(), query_path)
      }
  }
  ```
- The `apply_matching_strategies` function chains the 3 strategies using `.or_else()`
- Complexity of main function should drop to 1-2

**Testing**:
- Verify all existing tests still pass (no behavior change)
- Run `cargo test --lib` for full test suite
- Run `cargo tarpaulin` to verify coverage maintained/improved

**Success Criteria**:
- [ ] `find_functions_by_path` reduced to ~10 lines
- [ ] Cyclomatic complexity ≤ 2
- [ ] All tests pass
- [ ] Coverage maintained at 80%+
- [ ] No clippy warnings
- [ ] Ready to commit: "Simplify find_functions_by_path using unified strategy application"

### Phase 4: Add Property-Based Tests

**Goal**: Add property-based tests to verify invariants and catch edge cases we might have missed.

**Changes**:
- Add `proptest` property-based tests for:
  - Path normalization idempotency: `normalize_path(normalize_path(p)) == normalize_path(p)`
  - Strategy transitivity: if A matches B and B matches C with same strategy, verify behavior
  - Verify that at least one strategy succeeds for semantically equivalent paths
- Add tests for large datasets (100+ items) to verify parallel path works correctly

**Testing**:
- Run `cargo test --lib` to verify property tests pass
- Run property tests multiple times to verify stability
- Check coverage with `cargo tarpaulin`

**Success Criteria**:
- [ ] At least 3 property-based tests added
- [ ] All property tests pass consistently
- [ ] Coverage improvement to 85%+ for the module
- [ ] All tests pass
- [ ] Ready to commit: "Add property-based tests for path matching strategies"

### Phase 5: Final Verification and Documentation

**Goal**: Verify the complete refactoring achieves the expected impact and document the improvements.

**Changes**:
- Add doc comments to all extracted functions with examples
- Add module-level documentation explaining the matching strategy approach
- Update any related documentation

**Testing**:
- Run full CI suite: `just ci` (if available) or:
  - `cargo test --all-features`
  - `cargo clippy --all-targets --all-features -- -D warnings`
  - `cargo fmt --all -- --check`
  - `cargo doc --no-deps`
- Run `cargo tarpaulin --packages debtmap --lib` to verify final coverage
- Run `debtmap analyze` to verify the debt item is resolved/improved

**Success Criteria**:
- [ ] All CI checks pass
- [ ] Coverage for affected code reaches 85%+
- [ ] Documentation is complete and accurate
- [ ] Debt score reduced by ~10 points (from 23.7)
- [ ] Cyclomatic complexity reduced from 3 to ≤2
- [ ] No regression in existing functionality
- [ ] Ready to commit: "Document path matching module and verify improvements"

## Testing Strategy

**For each phase**:
1. Run phase-specific tests first to verify new functionality
2. Run `cargo test --lib` to ensure no regressions
3. Run `cargo clippy` to check for warnings
4. Run `cargo fmt` to ensure consistent formatting
5. Use `cargo tarpaulin --packages debtmap --lib` to verify coverage improvements

**Final verification**:
1. `cargo test --all-features` - All tests pass
2. `cargo clippy --all-targets --all-features -- -D warnings` - No warnings
3. `cargo fmt --all -- --check` - Code is formatted
4. `cargo tarpaulin --packages debtmap --lib` - Coverage ≥85%
5. `debtmap analyze` - Verify debt score improvement

## Rollback Plan

If a phase fails:
1. Run `git status` to see uncommitted changes
2. Run `git diff` to review what changed
3. If tests fail and can't be fixed quickly:
   - Revert with `git restore <files>` for uncommitted changes
   - Or `git reset --hard HEAD~1` for committed changes
4. Review the failure:
   - Check test output for specific failures
   - Review clippy warnings if any
   - Check if assumptions about the code were incorrect
5. Adjust the approach:
   - For test failures: Review expected vs actual behavior
   - For refactoring issues: Consider smaller extraction steps
   - For complexity issues: Break down further or reconsider approach
6. Retry with adjusted plan

## Notes

### Code Duplication Pattern
The current implementation has ~30 lines of duplicated code between the parallel and sequential branches. The three matching strategies are repeated verbatim in both branches, which is the primary source of cognitive complexity.

### Why This Approach Works
1. **Test First**: Phase 1 establishes safety net before refactoring
2. **Extract Then Simplify**: Phase 2 removes duplication, Phase 3 simplifies structure
3. **Incremental**: Each phase is independently valuable and committable
4. **Measurable**: Coverage and complexity metrics track progress objectively

### Potential Challenges
- The function uses both `iter()` and `par_iter()`, which have different return types. The `apply_matching_strategies` function in Phase 3 will need to be generic over iterator type or use trait objects.
- Solution: Use a generic function with trait bounds: `where I: Iterator<Item = (&'a PathBuf, &'a Vec<FunctionCoverage>)>` and `where I: ParallelIterator<Item = (&'a PathBuf, &'a Vec<FunctionCoverage>)>`

### Alternative Approaches Considered
- **Extract-first approach**: Extract functions before testing. Rejected because we'd be refactoring code we don't understand yet.
- **Big-bang refactor**: Do all changes at once. Rejected because it's harder to isolate failures and not incrementally committable.
- **Extract 10 functions as suggested**: The debtmap recommendation suggests 10 functions, but analysis shows only 3-4 meaningful extractions (the 3 strategies + optional helper). Over-extraction would add complexity rather than reduce it.

### Success Metrics
- Initial state: 54 LOC, complexity 3, coverage 0%
- Target state: ~20-25 LOC main function, complexity ≤2, coverage 85%+, 3-4 well-tested pure functions
- Debt score reduction: 23.7 → ~13-15 (expecting ~10 point reduction)
