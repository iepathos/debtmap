# Implementation Plan: Reduce Complexity in get_categorized_debt

## Problem Summary

**Location**: ./src/priority/unified_analysis_queries.rs:UnifiedAnalysis::get_categorized_debt:194
**Priority Score**: 11.35
**Debt Type**: ComplexityHotspot (Cyclomatic: 22, Cognitive: 46)

**Current Metrics**:
- Function Length: 97 lines
- Cyclomatic Complexity: 22
- Cognitive Complexity: 46
- Nesting Depth: 4 levels
- Function Role: PureLogic (but marked as not pure)

**Issue**: High complexity 22/46 makes function hard to test and maintain. The function has multiple responsibilities: categorizing debt items, computing category summaries, estimating effort, and identifying cross-category dependencies.

## Target State

**Expected Impact** (from debtmap):
- Complexity Reduction: 11.0 (target cyclomatic complexity ~10)
- Coverage Improvement: 0.0
- Risk Reduction: 3.97

**Success Criteria**:
- [ ] Cyclomatic complexity reduced from 22 to ≤10
- [ ] Cognitive complexity reduced from 46 to ≤20
- [ ] Function length reduced from 97 to ≤30 lines
- [ ] Nesting depth reduced from 4 to ≤2 levels
- [ ] All extracted functions are pure and independently testable
- [ ] All existing tests continue to pass
- [ ] No clippy warnings
- [ ] Proper formatting with rustfmt

## Implementation Phases

### Phase 1: Extract Item Categorization Logic

**Goal**: Extract the logic that determines which category a debt item belongs to into a pure, testable function.

**Changes**:
- Create new pure function `categorize_debt_item(item: &DebtItem) -> DebtCategory`
- Move lines 200-212 into this function
- Replace inline categorization with function call
- Add unit tests for all categorization scenarios

**Testing**:
- Test function-level items with each DebtType variant
- Test file-level items with god object indicators
- Test file-level items with low coverage
- Test file-level items with normal code quality
- Run `cargo test --lib` to verify existing tests pass

**Success Criteria**:
- [ ] New `categorize_debt_item` function is pure (no side effects)
- [ ] Function has comprehensive unit tests
- [ ] Cyclomatic complexity of main function reduced by ~6
- [ ] All tests pass
- [ ] Ready to commit

### Phase 2: Extract Effort Estimation Logic

**Goal**: Extract the nested effort estimation logic into a pure, testable function.

**Changes**:
- Create new pure function `estimate_effort_per_item(category: &DebtCategory, average_severity: f64) -> u32`
- Move lines 228-264 into this function
- Replace nested conditionals with function call
- Add unit tests for all category and severity combinations

**Testing**:
- Test all 4 categories (Architecture, Testing, Performance, CodeQuality)
- Test high severity (≥70.0) and low severity scenarios for each
- Test edge cases (severity = 70.0, 90.0)
- Run `cargo test --lib` to verify existing tests pass

**Success Criteria**:
- [ ] New `estimate_effort_per_item` function is pure
- [ ] Function has comprehensive unit tests (16+ test cases)
- [ ] Cyclomatic complexity of main function reduced by ~8 more
- [ ] All tests pass
- [ ] Ready to commit

### Phase 3: Extract Category Summary Builder

**Goal**: Extract the logic that builds a CategorySummary from a collection of items.

**Changes**:
- Create new pure function `build_category_summary(category: DebtCategory, items: Vec<DebtItem>) -> CategorySummary`
- Move lines 224-279 into this function
- Replace summary building logic with function call
- Add unit tests for summary computation

**Testing**:
- Test with empty items (edge case)
- Test with single item
- Test with multiple items (score aggregation, averaging)
- Test top items limiting (verify only top 5 are taken)
- Run `cargo test --lib` to verify existing tests pass

**Success Criteria**:
- [ ] New `build_category_summary` function is pure
- [ ] Function has comprehensive unit tests
- [ ] Cyclomatic complexity of main function reduced by ~4 more
- [ ] Main function now has cyclomatic complexity ≤10
- [ ] All tests pass
- [ ] Ready to commit

### Phase 4: Extract Item Collection and Categorization

**Goal**: Extract the loop that collects and categorizes items into a pure function.

**Changes**:
- Create new pure function `collect_categorized_items(items: Vector<DebtItem>) -> BTreeMap<DebtCategory, Vec<DebtItem>>`
- Move lines 198-215 into this function (using Phase 1's `categorize_debt_item`)
- Replace collection loop with function call
- Add unit tests for item collection

**Testing**:
- Test with empty items
- Test with items from single category
- Test with items spanning multiple categories
- Test that items are correctly bucketed by category
- Run `cargo test --lib` to verify existing tests pass

**Success Criteria**:
- [ ] New `collect_categorized_items` function is pure
- [ ] Function has comprehensive unit tests
- [ ] Main function reduced to ~25-30 lines
- [ ] Nesting depth in main function reduced to ≤2
- [ ] All tests pass
- [ ] Ready to commit

### Phase 5: Final Verification and Documentation

**Goal**: Verify all improvements, update documentation, and ensure code quality.

**Changes**:
- Add module-level documentation for new helper functions
- Add inline comments explaining the high-level flow in `get_categorized_debt`
- Run full test suite with coverage
- Verify debtmap shows improved metrics

**Testing**:
- `cargo test --all-features` - All tests pass
- `cargo clippy --all-targets --all-features -- -D warnings` - No warnings
- `cargo fmt --all -- --check` - Properly formatted
- `cargo doc --no-deps` - Documentation builds
- Re-run debtmap analysis to verify complexity reduction

**Success Criteria**:
- [ ] All new functions have doc comments with examples
- [ ] Main function has clear inline comments
- [ ] Cyclomatic complexity ≤10 (verified by debtmap)
- [ ] Cognitive complexity ≤20 (verified by debtmap)
- [ ] All CI checks pass (`just ci` if available)
- [ ] Code follows project's functional programming principles
- [ ] Ready for final commit

## Testing Strategy

**For each phase**:
1. Write tests for the extracted function FIRST (TDD approach)
2. Extract the function and verify tests pass
3. Run `cargo test --lib` to ensure no regressions
4. Run `cargo clippy` to check for warnings
5. Run `cargo fmt` to ensure proper formatting

**Final verification**:
1. `just ci` - Full CI checks (if available)
2. `cargo test --all-features` - All tests in all configurations
3. Re-run debtmap: `debtmap analyze --format json > after.json`
4. Compare before/after metrics to verify improvements

## Rollback Plan

If a phase fails:
1. Revert the phase with `git reset --hard HEAD~1`
2. Review the failure (test failures, compilation errors, etc.)
3. Adjust the implementation approach:
   - For test failures: Fix the logic in the extracted function
   - For compilation errors: Check type signatures and imports
   - For clippy warnings: Address the specific warning
4. Retry the phase with adjustments

If multiple retries fail (>3 attempts):
1. Document what failed and why
2. Consider alternative extraction strategy
3. May need to split the phase into smaller sub-phases

## Notes

### Functional Programming Alignment

This refactoring aligns with the project's functional programming principles:
- **Pure functions**: All extracted functions will be pure with no side effects
- **Immutable data flow**: Functions transform data without mutation
- **Single responsibility**: Each function does one thing well
- **Composability**: Main function becomes a composition of pure functions
- **Testability**: Pure functions are trivial to unit test

### Key Complexity Drivers

The main complexity drivers being addressed:
1. **Deep nesting** (4 levels): Eliminated through function extraction
2. **Multiple conditionals**: Isolated into separate functions
3. **Mixed responsibilities**: Separated into focused functions
4. **Long function**: Reduced to high-level orchestration

### Expected Transformation

Before (97 lines, complexity 22):
```rust
fn get_categorized_debt(&self, limit: usize) -> CategorizedDebt {
    // All logic inline: categorization + summary + effort estimation
    // Deep nesting, many branches
}
```

After (~25 lines, complexity ≤10):
```rust
fn get_categorized_debt(&self, limit: usize) -> CategorizedDebt {
    let all_items = self.get_top_mixed_priorities(limit);
    let categories = collect_categorized_items(all_items);
    let category_summaries = categories.into_iter()
        .map(|(cat, items)| (cat, build_category_summary(cat, items)))
        .collect();
    let cross_dependencies = identify_cross_category_dependencies(&category_summaries);
    CategorizedDebt { categories: category_summaries, cross_dependencies }
}
```

### Performance Considerations

- No performance impact expected; extracted functions have same algorithmic complexity
- Iterator chains may enable better compiler optimizations
- Pure functions are easier to parallelize in future if needed

### Dependencies

This function depends on:
- `get_top_mixed_priorities()` - already exists, no changes needed
- `identify_cross_category_dependencies()` - already exists, no changes needed
- Helper functions (`categorize_debt_item`, etc.) - will be created in this refactoring
