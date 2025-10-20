# Implementation Plan: Reduce Complexity in DependencyInjectionRecognizer::detect

## Problem Summary

**Location**: ./src/analysis/patterns/dependency_injection.rs:DependencyInjectionRecognizer::detect:115
**Priority Score**: 27.3
**Debt Type**: ComplexityHotspot (cognitive: 44, cyclomatic: 7)
**Current Metrics**:
- Lines of Code: 78
- Functions: 1 (the detect method)
- Cyclomatic Complexity: 7
- Cognitive Complexity: 44
- Nesting Depth: 4

**Issue**: The `detect` function has high cognitive complexity (44) due to deep nesting and multiple responsibilities. While cyclomatic complexity (7) is manageable, the cognitive load makes the function harder to understand and maintain. The function handles detection logic, decorator counting, confidence calculation, implementation collection, and pattern building all in one place.

## Target State

**Expected Impact**:
- Complexity Reduction: 3.5 (target cognitive complexity ~40 or lower)
- Coverage Improvement: 0.0 (maintain existing coverage)
- Risk Reduction: 9.555

**Success Criteria**:
- [ ] Cognitive complexity reduced to ~35-40 or lower
- [ ] Nesting depth reduced from 4 to 2 or less
- [ ] Function broken into smaller, testable pure functions
- [ ] All existing tests continue to pass
- [ ] No clippy warnings
- [ ] Code properly formatted with `cargo fmt`
- [ ] Clear separation of concerns (detection, counting, building)

## Implementation Phases

### Phase 1: Extract Decorator Counting Logic

**Goal**: Extract the complex decorator counting logic into a pure, testable function

**Changes**:
- Create new pure function `count_injection_decorators(class: &ClassDef) -> usize`
- Move lines 125-135 into this new function
- Replace inline counting logic with function call
- Add unit tests for the new function

**Testing**:
- Add test cases for counting decorators:
  - Class with no decorators
  - Class with class-level decorators only
  - Class with method-level decorators only
  - Class with both class and method decorators
- Run `cargo test --lib` to verify all tests pass
- Run `cargo clippy` to check for warnings

**Success Criteria**:
- [ ] New `count_injection_decorators` function created
- [ ] Function is pure (no side effects)
- [ ] At least 4 unit tests added for edge cases
- [ ] All existing tests pass
- [ ] Nesting depth in `detect` reduced by 1 level
- [ ] Ready to commit

### Phase 2: Extract Implementation Collection Logic

**Goal**: Extract the implementation collection logic into a pure function

**Changes**:
- Create new pure function `collect_injection_implementations(class: &ClassDef, file_path: &PathBuf) -> Vec<Implementation>`
- Move lines 146-162 into this new function
- Replace inline collection logic with function call
- Add unit tests for the new function

**Testing**:
- Add test cases for collecting implementations:
  - Class with `__init__` method
  - Class with methods having injection decorators
  - Class with both
  - Class with neither
- Run `cargo test --lib` to verify all tests pass
- Run `cargo clippy` to check for warnings

**Success Criteria**:
- [ ] New `collect_injection_implementations` function created
- [ ] Function is pure and independently testable
- [ ] At least 4 unit tests added
- [ ] All existing tests pass
- [ ] Further reduction in nesting depth
- [ ] Ready to commit

### Phase 3: Extract Pattern Building Logic

**Goal**: Extract the pattern instance building logic into a pure function

**Changes**:
- Create new pure function `build_pattern_instance(class: &ClassDef, has_constructor: bool, has_decorator: bool, has_setter: bool, decorator_count: usize, implementations: Vec<Implementation>, confidence: f32) -> PatternInstance`
- Move lines 164-186 into this new function
- Replace inline pattern building with function call
- Add unit tests for the new function

**Testing**:
- Add test cases for building patterns:
  - Pattern with constructor injection only
  - Pattern with decorator injection only
  - Pattern with setter injection only
  - Pattern with multiple injection types
- Run `cargo test --lib` to verify all tests pass
- Run `cargo clippy` to check for warnings

**Success Criteria**:
- [ ] New `build_pattern_instance` function created
- [ ] Function is pure with clear inputs/outputs
- [ ] At least 4 unit tests added
- [ ] All existing tests pass
- [ ] `detect` function significantly simplified
- [ ] Ready to commit

### Phase 4: Simplify Main Detection Logic with Guard Clauses

**Goal**: Reduce nesting in the main `detect` function using early returns and guard clauses

**Changes**:
- Add guard clause at line 118: return early if `classes` is None
- Restructure the main loop to use early continue for non-matching classes
- Move the condition check (line 137) earlier with early continue
- Ensure all helper functions are properly called
- Verify cognitive complexity reduction

**Testing**:
- Run full test suite: `cargo test --lib`
- Run `cargo clippy` to ensure no new warnings
- Run `cargo fmt` to ensure proper formatting
- Manually verify that the refactored logic matches original behavior

**Success Criteria**:
- [ ] Guard clauses added for early returns
- [ ] Nesting depth reduced to 2 or less
- [ ] Main loop is clear and readable
- [ ] All existing tests pass
- [ ] No clippy warnings
- [ ] Code properly formatted
- [ ] Cognitive complexity target achieved (~35-40)
- [ ] Ready to commit

### Phase 5: Final Verification and Documentation

**Goal**: Verify all improvements and add documentation for the refactored functions

**Changes**:
- Add doc comments to all new helper functions
- Run full CI checks with `just ci`
- Regenerate coverage with `cargo tarpaulin` to ensure no regression
- Run `debtmap analyze` to verify complexity reduction
- Update any relevant documentation

**Testing**:
- Full CI suite: `just ci`
- Coverage check: `cargo tarpaulin --lib`
- Complexity analysis: `debtmap analyze`
- Manual code review of all changes

**Success Criteria**:
- [ ] All new functions have doc comments with examples
- [ ] Full CI passes
- [ ] Test coverage maintained or improved
- [ ] Debtmap shows complexity reduction
- [ ] Code review confirms quality improvements
- [ ] Ready for final commit

## Testing Strategy

**For each phase**:
1. Run `cargo test --lib` to verify existing tests pass
2. Run `cargo clippy --all-targets --all-features -- -D warnings` to check for warnings
3. Run `cargo fmt --all -- --check` to verify formatting
4. Add phase-specific unit tests as outlined above

**Final verification**:
1. `just ci` - Full CI checks (all tests, clippy, formatting)
2. `cargo tarpaulin --lib` - Regenerate coverage report
3. `debtmap analyze` - Verify complexity reduction achieved
4. Manual review of cognitive complexity metrics

## Rollback Plan

If a phase fails:
1. Identify the specific failure (test failure, clippy warning, compilation error)
2. Review the error messages and test output
3. If the issue is fixable within the phase:
   - Fix the issue
   - Re-run tests
   - Continue with the phase
4. If the issue is fundamental to the approach:
   - Revert the phase with `git reset --hard HEAD~1`
   - Reassess the approach
   - Adjust the plan
   - Retry with modified approach

**Critical**: Do NOT proceed to the next phase if the current phase is not complete and stable.

## Notes

### Key Principles for This Refactoring:

1. **Pure Functions First**: Each extracted function should be pure (no side effects) and independently testable
2. **Single Responsibility**: Each new function does exactly one thing
3. **Preserve Behavior**: The refactoring should not change any existing behavior or test outcomes
4. **Incremental Progress**: Each phase results in working, committable code
5. **Test Coverage**: Maintain or improve test coverage throughout

### Functional Programming Patterns to Apply:

- **Extract Pure Logic**: All helper functions should be pure transformations
- **Iterator Chains**: Use `.iter().filter().map().collect()` patterns for clarity
- **Immutability**: No mutable state in helper functions
- **Function Composition**: Build the final result by composing smaller functions

### Gotchas to Watch For:

1. **Ownership**: Be careful with `PathBuf` cloning in helper functions
2. **Iterator Efficiency**: Avoid unnecessary clones in iterator chains
3. **Edge Cases**: Ensure empty collections are handled correctly
4. **Test Patterns**: Follow existing test patterns in the file (lines 239-377)

### Expected Cognitive Complexity Reduction:

- **Phase 1**: ~5-7 points (decorator counting extraction)
- **Phase 2**: ~5-7 points (implementation collection extraction)
- **Phase 3**: ~3-5 points (pattern building extraction)
- **Phase 4**: ~5-8 points (guard clauses and nesting reduction)
- **Total Target**: From 44 to ~35-40 or better

The key to success is maintaining small, incremental changes that each improve the code while preserving correctness.
