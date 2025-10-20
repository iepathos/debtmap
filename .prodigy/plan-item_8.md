# Implementation Plan: Reduce Complexity in write_priority_section

## Problem Summary

**Location**: ./src/io/writers/markdown/enhanced.rs:MarkdownWriter::write_priority_section:30
**Priority Score**: 23.78
**Debt Type**: ComplexityHotspot (Cognitive: 19, Cyclomatic: 15)
**Current Metrics**:
- Lines of Code: 45
- Function Length: 45
- Cyclomatic Complexity: 15
- Cognitive Complexity: 19
- Coverage: N/A (pure logic function)

**Issue**: Apply early returns to simplify control flow

The `write_priority_section` function mixes multiple responsibilities:
1. Writing section header
2. Fetching and validating top items
3. Formatting table headers and rows
4. Conditional verbosity handling
5. Direct I/O operations

This violates the functional programming principle of separating I/O from pure logic.

## Target State

**Expected Impact** (from debtmap):
- Complexity Reduction: 7.5 (from 15 to ~7-8)
- Coverage Improvement: 0.0
- Risk Reduction: 8.32

**Success Criteria**:
- [ ] Cyclomatic complexity reduced from 15 to ≤8
- [ ] Cognitive complexity reduced from 19 to ≤12
- [ ] Pure formatting logic extracted into testable functions
- [ ] I/O operations isolated to thin wrapper
- [ ] All existing tests continue to pass
- [ ] No clippy warnings
- [ ] Proper formatting

## Implementation Phases

### Phase 1: Extract Pure Table Formatting Logic

**Goal**: Separate table row formatting into a pure, testable function

**Changes**:
- Extract table row formatting logic from the loop (lines 48-63) into a pure function `format_priority_table_row`
- Function takes `(rank: usize, item: &UnifiedDebtItem)` and returns `String`
- Keep I/O in the main function, move formatting logic out

**Testing**:
- Add unit test for `format_priority_table_row` with sample data
- Verify existing integration tests still pass
- Run `cargo test --lib`

**Success Criteria**:
- [ ] Pure function `format_priority_table_row` created
- [ ] Function is unit tested
- [ ] Main function complexity reduced by ~2 points
- [ ] All tests pass
- [ ] No clippy warnings
- [ ] Ready to commit

### Phase 2: Extract Table Header Generation

**Goal**: Extract static table header generation into a pure function

**Changes**:
- Create pure function `format_priority_table_header(item_count: usize)` returning `String`
- Includes the "### Top N Priority Items" header and table headers
- Returns complete header section as String
- Replace lines 43-46 with call to this function

**Testing**:
- Add unit test for `format_priority_table_header`
- Verify output formatting is correct
- Run `cargo test --lib`

**Success Criteria**:
- [ ] Pure function `format_priority_table_header` created
- [ ] Function is unit tested
- [ ] Main function complexity reduced by ~1 point
- [ ] All tests pass
- [ ] Ready to commit

### Phase 3: Extract Complete Table Building Logic

**Goal**: Combine row and header formatting into a single pure table builder

**Changes**:
- Create pure function `build_priority_table(items: &[UnifiedDebtItem])` returning `String`
- This function orchestrates header and row formatting
- Returns complete table as String
- Main function becomes: get items, early return if empty, write table, conditionally write breakdown

**Testing**:
- Add unit test for `build_priority_table` with various input sizes
- Test empty input, single item, multiple items
- Run `cargo test --lib`

**Success Criteria**:
- [ ] Pure function `build_priority_table` created
- [ ] Comprehensive unit tests added
- [ ] Main function simplified to ~15-20 lines
- [ ] Cyclomatic complexity reduced to ~8
- [ ] All tests pass
- [ ] Ready to commit

### Phase 4: Apply Early Return Pattern

**Goal**: Simplify control flow using early returns and guard clauses

**Changes**:
- Restructure main function to use early returns
- Pattern: validate inputs → early return if invalid → process valid case
- Lines 37-41 already use early return (good!)
- Simplify the verbosity check at lines 68-71 to be more explicit

**Testing**:
- Verify behavior with empty items list
- Verify behavior with verbosity = 0 and verbosity > 0
- Run `cargo test --lib`

**Success Criteria**:
- [ ] Function uses guard clauses effectively
- [ ] Nesting depth reduced
- [ ] Cognitive complexity reduced to ≤12
- [ ] All tests pass
- [ ] Ready to commit

### Phase 5: Final Verification and Documentation

**Goal**: Ensure all improvements are complete and documented

**Changes**:
- Add doc comments to new pure functions
- Update module documentation if needed
- Run full CI suite
- Verify complexity metrics improved

**Testing**:
- `just ci` - Full CI checks
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --all-features`
- Visual inspection of generated markdown output

**Success Criteria**:
- [ ] All new functions have doc comments
- [ ] Full CI passes
- [ ] No clippy warnings
- [ ] Complexity reduced to target levels
- [ ] Code follows project functional programming style
- [ ] Ready for final commit

## Testing Strategy

**For each phase**:
1. Run `cargo test --lib` to verify existing tests pass
2. Run `cargo clippy` to check for warnings
3. Run phase-specific unit tests for new functions

**Final verification**:
1. `just ci` - Full CI checks
2. Manually test markdown output generation
3. Compare before/after complexity with `debtmap analyze`

## Rollback Plan

If a phase fails:
1. Revert the phase with `git reset --hard HEAD~1`
2. Review the failure cause
3. Adjust the implementation approach
4. Retry with fixes

## Notes

### Functional Programming Approach

This refactoring follows the functional core, imperative shell pattern:
- **Pure functions** (format_priority_table_row, build_priority_table): No side effects, easy to test
- **Imperative shell** (write_priority_section): Thin I/O wrapper around pure logic

### Why This Reduces Complexity

1. **Separation of Concerns**: Formatting logic separated from I/O operations
2. **Single Responsibility**: Each extracted function does one thing
3. **Testability**: Pure functions are trivially testable
4. **Reduced Nesting**: Early returns eliminate conditional nesting
5. **Function Composition**: Build complex behavior from simple, tested pieces

### Key Success Indicators

- Main function should be ~15-20 lines
- Each extracted function should be <10 lines
- Pure functions should have 100% test coverage
- No mutation in pure functions (immutable transformations only)
