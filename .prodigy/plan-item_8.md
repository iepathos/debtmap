# Implementation Plan: Add Tests and Refactor identify_unchanged_critical

## Problem Summary

**Location**: ./src/commands/compare_debtmap.rs:identify_unchanged_critical:420
**Priority Score**: 22.75
**Debt Type**: TestingGap
**Current Metrics**:
- Lines of Code: 48
- Functions: 1 (monolithic function)
- Cyclomatic Complexity: 7
- Cognitive Complexity: 23
- Coverage: 0.0% (100% gap)
- Nesting Depth: 5

**Issue**: Complex business logic with 100% coverage gap. Cyclomatic complexity of 7 requires at least 7 test cases for full path coverage. The function has nested conditionals and multiple responsibilities that need to be extracted into pure, testable functions.

**Function Role**: PureLogic (should be fully testable as it has no I/O operations)

## Target State

**Expected Impact**:
- Complexity Reduction: 2.1 (cyclomatic complexity from 7 → ~5)
- Coverage Improvement: 50.0% (from 0% to 50%+)
- Risk Reduction: 9.555

**Success Criteria**:
- [ ] 100% test coverage (all 24 uncovered lines covered)
- [ ] All 7 branches covered by tests
- [ ] Function refactored into 5 smaller pure functions
- [ ] Each extracted function has complexity ≤3
- [ ] All existing tests continue to pass
- [ ] No clippy warnings
- [ ] Proper formatting
- [ ] Cyclomatic complexity reduced to ≤5

## Implementation Phases

### Phase 1: Add Comprehensive Tests for Current Implementation

**Goal**: Achieve 100% coverage of the existing `identify_unchanged_critical` function before refactoring to ensure no regression during refactoring.

**Changes**:
- Add test module in `src/commands/compare_debtmap.rs`
- Create helper functions to build test data (before/after UnifiedJsonOutput)
- Write 7+ test cases covering all branches:
  1. Empty before/after (no items)
  2. No critical items (all scores < 8.0)
  3. Critical items resolved (in before, not in after)
  4. Critical items unchanged (same score ±0.5)
  5. Critical items improved significantly (score drops > 0.5)
  6. Critical items worsened (score increases but stays ≥ 8.0)
  7. Mixed scenario (some unchanged, some resolved, some improved)
- Verify all 24 uncovered lines are covered

**Testing**:
- Run `cargo test --lib test_identify_unchanged_critical` to verify all new tests pass
- Run `cargo tarpaulin --out Stdout -- --test test_identify_unchanged_critical` to verify coverage
- Run `cargo clippy` to ensure no warnings

**Success Criteria**:
- [ ] 7+ test cases written and passing
- [ ] 100% coverage of `identify_unchanged_critical` function
- [ ] All existing tests still pass
- [ ] Test code follows existing patterns in codebase
- [ ] Ready to commit

### Phase 2: Extract Pure Function - Build After Map

**Goal**: Extract the logic for building the after_map into a pure, testable function.

**Changes**:
- Create new function `build_function_map(items: &[DebtItem]) -> HashMap<(PathBuf, String), &FunctionMetrics>`
- Extract lines 428-437 into this new function
- Update `identify_unchanged_critical` to call this function
- Add 3 tests for `build_function_map`:
  1. Empty items list
  2. Mixed Function and File items (only Functions in map)
  3. Multiple functions with same file but different names

**Testing**:
- Run `cargo test --lib test_build_function_map` to verify new tests pass
- Run `cargo test --lib test_identify_unchanged_critical` to ensure no regression
- Verify cyclomatic complexity reduced

**Success Criteria**:
- [ ] `build_function_map` function created and tested
- [ ] All 10+ tests passing (7 from Phase 1 + 3 new)
- [ ] No regression in existing functionality
- [ ] Cyclomatic complexity of `identify_unchanged_critical` reduced by 1
- [ ] Ready to commit

### Phase 3: Extract Pure Function - Check if Critical

**Goal**: Extract the critical item check logic into a pure predicate function.

**Changes**:
- Create new function `is_critical(score: f64) -> bool` with threshold constant
- Replace inline checks `>= 8.0` with this function
- Update related functions (`create_summary`, `identify_new_items`) to use same function
- Add 3 tests for `is_critical`:
  1. Below threshold (7.9)
  2. At threshold (8.0)
  3. Above threshold (10.0)

**Testing**:
- Run `cargo test --lib test_is_critical` to verify new tests pass
- Run full test suite to ensure consistency across all usages
- Verify no magic numbers remain

**Success Criteria**:
- [ ] `is_critical` function created and tested
- [ ] All instances of `>= 8.0` replaced with function call
- [ ] All 13+ tests passing
- [ ] Code more maintainable (single source of truth for threshold)
- [ ] Ready to commit

### Phase 4: Extract Pure Function - Check Score Unchanged

**Goal**: Extract the logic for determining if a score is "unchanged" into a pure function.

**Changes**:
- Create new function `is_score_unchanged(before: f64, after: f64, tolerance: f64) -> bool`
- Extract the logic from line 447-450 into this function
- Default tolerance constant to 0.5
- Add 4 tests for `is_score_unchanged`:
  1. Exactly equal scores
  2. Within tolerance (change = 0.3)
  3. At boundary (change = 0.5)
  4. Outside tolerance (change = 0.6)

**Testing**:
- Run `cargo test --lib test_is_score_unchanged` to verify new tests pass
- Verify logic still correct in `identify_unchanged_critical`
- Check for edge cases

**Success Criteria**:
- [ ] `is_score_unchanged` function created and tested
- [ ] All 17+ tests passing
- [ ] Tolerance value documented and testable
- [ ] Cyclomatic complexity further reduced
- [ ] Ready to commit

### Phase 5: Extract Pure Function - Filter Critical Unchanged Items

**Goal**: Extract the core filtering logic into a pure, focused function.

**Changes**:
- Create new function:
  ```rust
  fn filter_unchanged_critical_items(
      before_items: &[DebtItem],
      after_map: &HashMap<(PathBuf, String), &FunctionMetrics>
  ) -> Vec<ItemInfo>
  ```
- Extract lines 439-461 into this function
- Update `identify_unchanged_critical` to be a thin wrapper:
  ```rust
  fn identify_unchanged_critical(...) -> UnchangedCritical {
      let after_map = build_function_map(&after.items);
      let items = filter_unchanged_critical_items(&before.items, &after_map);
      UnchangedCritical {
          count: items.len(),
          items,
      }
  }
  ```
- Add 5 tests for `filter_unchanged_critical_items`:
  1. Empty before items
  2. No matches in after_map (all resolved)
  3. All items improved (score change > 0.5)
  4. Mix of unchanged and improved
  5. Items that stay critical but change score within tolerance

**Testing**:
- Run `cargo test --lib test_filter_unchanged_critical_items` to verify new tests pass
- Verify `identify_unchanged_critical` still works correctly
- Check that nesting depth reduced to ≤2

**Success Criteria**:
- [ ] `filter_unchanged_critical_items` function created and tested
- [ ] `identify_unchanged_critical` simplified to 10-15 lines
- [ ] All 22+ tests passing
- [ ] Nesting depth ≤2 in all functions
- [ ] Each function has complexity ≤3
- [ ] Ready to commit

### Phase 6: Add Property-Based Tests

**Goal**: Add property-based tests to verify invariants and edge cases.

**Changes**:
- Add `proptest` to dev dependencies if not already present
- Create property tests for core invariants:
  1. `filter_unchanged_critical_items` always returns subset of input
  2. All returned items have scores ≥ 8.0
  3. All returned items exist in after_map
  4. Count equals length of items vector
- Add edge case tests:
  1. Very large before/after datasets (performance)
  2. Floating point edge cases (NaN, infinity)
  3. Unicode in function names

**Testing**:
- Run `cargo test --lib test_unchanged_critical_properties` to verify property tests pass
- Run full test suite with `cargo test --all-features`
- Verify coverage remains 100%

**Success Criteria**:
- [ ] 3+ property-based tests added and passing
- [ ] All 25+ tests passing
- [ ] Coverage ≥80% (target 100%)
- [ ] No clippy warnings
- [ ] Performance acceptable for large datasets
- [ ] Ready to commit

## Testing Strategy

**For each phase**:
1. Run `cargo test --lib` to verify existing tests pass
2. Run `cargo clippy` to check for warnings
3. Run `cargo fmt --all -- --check` to verify formatting
4. Run phase-specific tests to verify new functionality

**After Phase 1**:
- Run `cargo tarpaulin --out Stdout` to verify coverage improvement
- Verify all 24 uncovered lines now covered

**After Phase 5**:
- Verify cyclomatic complexity reduced from 7 to ≤5
- Verify each extracted function has complexity ≤3
- Run `cargo clippy -- -W clippy::cognitive_complexity` to check cognitive complexity

**Final verification**:
1. `cargo test --all-features` - All tests pass
2. `cargo clippy --all-targets --all-features -- -D warnings` - No warnings
3. `cargo fmt --all -- --check` - Proper formatting
4. `cargo tarpaulin --out Stdout` - Verify 80%+ coverage
5. `debtmap analyze` - Verify score improvement

## Rollback Plan

If a phase fails:
1. Revert the phase with `git reset --hard HEAD~1`
2. Review the test failure or compilation error
3. Adjust the implementation approach
4. Retry the phase

If tests become flaky:
1. Investigate test data setup
2. Check for hidden dependencies between tests
3. Ensure tests are deterministic (no random data, time dependencies)

## Notes

### Key Complexity Sources

The `identify_unchanged_critical` function has high complexity due to:
1. Nested iteration (for loop inside if let pattern matching)
2. Multiple conditional checks (score thresholds, tolerance checks)
3. HashMap lookup with conditional logic
4. Building ItemInfo structs conditionally

### Extraction Strategy

We're using a functional decomposition approach:
1. **Data structure building** → `build_function_map`
2. **Predicate functions** → `is_critical`, `is_score_unchanged`
3. **Core logic** → `filter_unchanged_critical_items`
4. **Wrapper/orchestration** → `identify_unchanged_critical`

This follows the "Pure Core, Imperative Shell" pattern from the project guidelines.

### Testing Philosophy

- **Phase 1**: Test existing behavior (safety net for refactoring)
- **Phases 2-5**: Test each extracted function in isolation (focused unit tests)
- **Phase 6**: Test invariants and edge cases (property-based testing)

### Coverage Target Justification

The function is marked as `PureLogic` with `is_pure: false` and `purity_confidence: 1.0`. This means:
- It should be pure logic (no side effects)
- It's currently implemented in an impure style (mutation, imperative)
- It has high confidence it can be made pure

Targeting 100% coverage is appropriate because:
1. Pure logic functions should be fully testable
2. Cyclomatic complexity of 7 requires 7+ test cases for full branch coverage
3. 0% → 100% coverage gives us confidence in the refactoring

### Avoiding Common Pitfalls

1. **Don't test implementation details**: Test behavior and contracts, not internal structure
2. **Keep test data realistic**: Use actual DebtItem structures, not simplified mocks
3. **Follow existing patterns**: Look at tests in `src/priority.rs` and `src/analysis/` for examples
4. **Preserve semantics**: The refactored code must behave identically to the original

### Performance Considerations

The function processes all items in the before list and performs HashMap lookups. This is O(n) which is acceptable. The extracted functions should maintain this performance profile.

### Related Code

Similar functions in this file that may benefit from the same predicates:
- `identify_resolved_items` (lines 250-287)
- `identify_improved_items` (lines 295-361)
- `identify_new_items` (lines 375-413)

After this refactoring, consider applying similar patterns to these functions in future debt fixes.
