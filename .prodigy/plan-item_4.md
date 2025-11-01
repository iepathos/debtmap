# Implementation Plan: Refactor parse_numpy_raises Function

## Problem Summary

**Location**: ./src/analyzers/python_exception_flow/analyzer.rs:parse_numpy_raises:805
**Priority Score**: 18.4275
**Debt Type**: ComplexityHotspot (cognitive: 63, cyclomatic: 21)
**Current Metrics**:
- Lines of Code: 120
- Cyclomatic Complexity: 21
- Cognitive Complexity: 63
- Nesting Depth: 5
- Function Role: PureLogic (should be pure but currently isn't)

**Issue**: Reduce complexity from 21 to ~10. High complexity 21/63 makes function hard to test and maintain.

**Root Causes**:
1. **Repeated exception-saving logic** - Same code block appears 5 times (lines 848-853, 861-866, 882-887, 892-898, 911-915)
2. **Multiple responsibilities** - Section detection, state tracking, content classification, and exception building all mixed
3. **Deep nesting** - 5 levels of nesting with complex conditionals
4. **Implicit state machine** - `in_raises` and `in_separator` flags make control flow hard to follow

## Target State

**Expected Impact** (from debtmap):
- Complexity Reduction: 10.5 (from 21 to ~10)
- Coverage Improvement: 0.0 (maintain existing coverage)
- Risk Reduction: 6.45

**Success Criteria**:
- [ ] Cyclomatic complexity reduced to ≤10
- [ ] No code duplication (DRY principle applied)
- [ ] Pure functions extracted for each responsibility
- [ ] Nesting depth reduced to ≤3 levels
- [ ] All existing tests continue to pass
- [ ] No clippy warnings
- [ ] Proper formatting with cargo fmt

## Implementation Phases

### Phase 1: Extract Exception Builder Helper

**Goal**: Eliminate the 5 instances of duplicated exception-saving code by extracting a pure helper function.

**Changes**:
- Create `fn save_exception(current_exception: Option<String>, description: String, exceptions: &mut Vec<DocumentedException>)` helper
- Replace all 5 occurrences of the duplication with calls to this helper
- This immediately reduces complexity by consolidating branching logic

**Testing**:
- Run `cargo test --lib` to verify existing tests pass
- Run `cargo clippy` to check for warnings
- Verify the function still correctly parses NumPy-style Raises sections

**Success Criteria**:
- [ ] Duplication eliminated (5 blocks → 1 function)
- [ ] Complexity reduced by ~3-4 points
- [ ] All tests pass
- [ ] Ready to commit

**Estimated Complexity Reduction**: 3-4 points (from 21 to ~17)

---

### Phase 2: Extract Line Classification Logic

**Goal**: Separate the complex logic that determines what type of line we're processing.

**Changes**:
- Create pure function `fn classify_line(trimmed: &str, indent_count: usize, known_sections: &[&str]) -> LineType`
  - Returns enum: `enum LineType { SectionHeader, ExceptionType, Description, Separator, Empty }`
- Extract the classification logic from lines 844-856, 859-868, 871-906
- Replace nested conditionals with match on `LineType`

**Testing**:
- Add unit tests for `classify_line` with various inputs
- Run `cargo test --lib` to verify existing tests pass
- Test edge cases: empty lines, various indentation levels, section headers

**Success Criteria**:
- [ ] Classification logic extracted to pure function
- [ ] New unit tests for `classify_line` added
- [ ] Main function uses match instead of nested ifs
- [ ] Complexity reduced by ~3-4 points
- [ ] All tests pass
- [ ] Ready to commit

**Estimated Complexity Reduction**: 3-4 points (from ~17 to ~13)

---

### Phase 3: Extract State Machine Logic

**Goal**: Make the parsing state machine explicit and easier to reason about.

**Changes**:
- Create enum `enum NumpyParseState { BeforeRaises, InRaisesHeader, ParsingExceptions, Done }`
- Create pure function `fn transition_state(current: NumpyParseState, line_type: LineType) -> NumpyParseState`
- Replace `in_raises` and `in_separator` booleans with state enum
- Use state transitions to control parsing flow

**Testing**:
- Add unit tests for state transitions
- Run `cargo test --lib` to verify existing tests pass
- Test state machine with various docstring formats

**Success Criteria**:
- [ ] State machine made explicit with enum
- [ ] State transition logic extracted to pure function
- [ ] Boolean flags removed
- [ ] Complexity reduced by ~2-3 points
- [ ] All tests pass
- [ ] Ready to commit

**Estimated Complexity Reduction**: 2-3 points (from ~13 to ~10)

---

### Phase 4: Final Cleanup and Optimization

**Goal**: Polish the refactored code and ensure it meets all quality standards.

**Changes**:
- Add comprehensive doc comments to new helper functions
- Ensure all extracted functions are truly pure (no side effects)
- Consider using iterator adapters instead of explicit loops where beneficial
- Run final formatting and linting

**Testing**:
- Run `just ci` - Full CI checks
- Run `cargo clippy -- -D warnings` - Ensure no warnings
- Run `cargo test --all` - All tests pass
- Manual review of complexity with debtmap

**Success Criteria**:
- [ ] All helper functions properly documented
- [ ] Code follows functional programming principles
- [ ] Final cyclomatic complexity ≤10
- [ ] All CI checks pass
- [ ] Ready for final commit

**Estimated Final Complexity**: ≤10

---

## Testing Strategy

**For each phase**:
1. Run `cargo test --lib` to verify existing tests pass
2. Run `cargo clippy` to check for warnings
3. Run `cargo fmt` to ensure proper formatting
4. For phases with new functions, add unit tests

**Existing test coverage**:
- `test_docstring_validation_google` - Tests Google-style parsing (related)
- Other tests in the module verify the analyzer's overall behavior
- The `parse_numpy_raises` function is called indirectly through the analyzer

**Final verification**:
1. `just ci` - Full CI checks
2. `cargo test --all` - All tests pass
3. `cargo clippy -- -D warnings` - No warnings
4. Manual verification that NumPy docstring parsing still works correctly

## Rollback Plan

If a phase fails:
1. Revert the phase with `git reset --hard HEAD~1`
2. Review the failure (test failures, compilation errors, logic bugs)
3. Adjust the implementation approach
4. Retry the phase with fixes

**Common failure modes**:
- **Test failures**: Likely logic error in extracted function - verify input/output matches original
- **Compilation errors**: Type mismatches or ownership issues - review function signatures
- **Clippy warnings**: Usually easy fixes - follow clippy suggestions

## Notes

**Why this approach**:
- **Phase 1** tackles the most obvious duplication first (quick win)
- **Phase 2** extracts the most complex logic (classification) into a testable pure function
- **Phase 3** makes implicit state explicit, improving readability
- **Phase 4** ensures quality and polish

**Functional programming alignment**:
- All extracted functions will be pure (no side effects)
- State transitions are explicit and testable
- Logic separated from I/O (already file-local, no I/O in this function)
- Each helper function has single responsibility

**Risk mitigation**:
- Incremental phases mean we can stop and commit at any point
- Each phase independently reduces complexity
- Existing tests provide safety net
- New unit tests for extracted functions increase coverage

**Expected effort**: ~3 hours total
- Phase 1: 45 minutes
- Phase 2: 1 hour
- Phase 3: 45 minutes
- Phase 4: 30 minutes
