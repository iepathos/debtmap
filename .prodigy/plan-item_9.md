# Implementation Plan: Refactor src/priority/mod.rs God Object

## Problem Summary

**Location**: ./src/priority/mod.rs:file:0
**Priority Score**: 50.48
**Debt Type**: God Object (File-Level)
**Current Metrics**:
- Lines of Code: 1658
- Functions: 58 (22 methods on UnifiedAnalysis impl, 36 others)
- Cyclomatic Complexity: 181 total (avg 3.12)
- Max Complexity: 22
- Coverage: 0.0%
- God Object Score: 1.0 (maximum)
- Responsibilities: 6 (Construction, Utilities, Computation, Validation, Data Access, Filtering & Selection)

**Issue**: URGENT: 1658 lines, 58 functions! This file violates the single responsibility principle with the UnifiedAnalysis struct having 22 methods across 6 distinct responsibilities. The debtmap analysis identifies two high-priority splits: 1) Utilities (11 methods, ~220 lines) and 2) Data Access (7 methods, ~140 lines). The file mixes data structures, business logic, formatting, and utilities.

## Target State

**Expected Impact**:
- Complexity Reduction: 36.2
- Maintainability Improvement: 5.05
- Test Effort Reduction: 165.8

**Success Criteria**:
- [ ] File reduced to < 600 lines (core types and constructor only)
- [ ] UnifiedAnalysis impl has ≤ 10 methods (down from 22)
- [ ] New modules created for extracted responsibilities
- [ ] All existing tests continue to pass
- [ ] No clippy warnings
- [ ] Proper formatting (cargo fmt)
- [ ] Public API remains unchanged (backward compatible)

## Implementation Phases

### Phase 1: Extract Data Access Module

**Goal**: Extract the 7 data access methods from UnifiedAnalysis into a new `unified_analysis_queries` module

**Changes**:
- Create `src/priority/unified_analysis_queries.rs`
- Extract these methods from UnifiedAnalysis impl:
  - `get_top_priorities`
  - `get_top_mixed_priorities`
  - `get_top_mixed_priorities_tiered`
  - `get_bottom_priorities`
  - `get_tiered_display`
  - `get_debt_type_key` (private helper)
  - `get_categorized_debt`
- Implement as pure functions or extension trait
- Update UnifiedAnalysis impl to delegate to new module
- Add module to `mod.rs` exports

**Testing**:
- Run `cargo test --lib` to ensure all tests pass
- Run `cargo clippy` to check for warnings
- Verify no breaking changes to public API

**Success Criteria**:
- [ ] New module created with 7 functions/methods
- [ ] UnifiedAnalysis impl reduced by 7 methods (down to 15)
- [ ] All tests pass
- [ ] No clippy warnings
- [ ] Ready to commit

### Phase 2: Extract Utilities Module

**Goal**: Extract the 11 utility methods from UnifiedAnalysis into a new `unified_analysis_utils` module

**Changes**:
- Create `src/priority/unified_analysis_utils.rs`
- Extract these methods from UnifiedAnalysis impl:
  - `timings`
  - `add_file_item`
  - `add_item`
  - `sort_by_priority`
  - `generate_batch_action` (private helper)
  - `data_flow_graph`
  - `data_flow_graph_mut`
  - `populate_purity_analysis`
  - `add_io_operation`
  - `add_variable_dependencies`
  - `identify_cross_category_dependencies` (private helper)
- Consider splitting into sub-responsibilities:
  - Item management: `add_file_item`, `add_item`, `sort_by_priority`
  - Data flow operations: `data_flow_graph`, `data_flow_graph_mut`, `populate_purity_analysis`, `add_io_operation`, `add_variable_dependencies`
  - Helpers: `generate_batch_action`, `identify_cross_category_dependencies`, `timings`
- Update UnifiedAnalysis impl to use new utilities
- Add module to `mod.rs` exports (if public API needed)

**Testing**:
- Run `cargo test --lib` to ensure all tests pass
- Run `cargo clippy` to check for warnings
- Verify backward compatibility

**Success Criteria**:
- [ ] New module(s) created with 11 functions
- [ ] UnifiedAnalysis impl reduced by 11 methods (down to 4)
- [ ] All tests pass
- [ ] No clippy warnings
- [ ] Ready to commit

### Phase 3: Extract Computation Module

**Goal**: Extract `calculate_total_impact` and `filter_by_categories` computation logic into a focused module

**Changes**:
- Create `src/priority/unified_analysis_computation.rs`
- Extract computation methods:
  - `calculate_total_impact` - complex aggregation logic
  - `filter_by_categories` - filtering and recalculation
  - `is_critical_item` (private helper)
- Refactor `calculate_total_impact` to be more functional:
  - Extract pure calculation functions
  - Separate I/O (file reading) from computation
  - Use iterator chains where possible
- Update UnifiedAnalysis impl to use computation module
- Add module to `mod.rs`

**Testing**:
- Run `cargo test --lib` to ensure all tests pass
- Run specific tests for debt density calculation (lines 1303-1366)
- Run `cargo clippy`
- Verify metrics calculations remain accurate

**Success Criteria**:
- [ ] New module created with extracted computation logic
- [ ] UnifiedAnalysis impl reduced to ~2 core methods
- [ ] Computation logic more functional and testable
- [ ] All tests pass (especially debt density tests)
- [ ] No clippy warnings
- [ ] Ready to commit

### Phase 4: Organize Type Definitions

**Goal**: Move supporting type definitions to focused submodules to reduce main file size

**Changes**:
- Create `src/priority/types.rs` for shared types:
  - `FunctionAnalysis`
  - `ImpactMetrics`
  - `ActionableRecommendation`
  - `ActionStep`
  - `Difficulty`
  - `FunctionVisibility`
- Create `src/priority/debt_types.rs`:
  - `DebtType` enum (24 variants)
  - `DebtCategory` enum
  - Related implementations
- Create `src/priority/display_types.rs`:
  - `Tier` enum
  - `DisplayGroup`
  - `TieredDisplay`
  - `DebtItem` enum
  - `CategorySummary`
  - `CategorizedDebt`
  - `CrossCategoryDependency`
  - `ImpactLevel`
- Update `mod.rs` to re-export types from new modules
- Keep `UnifiedAnalysis` struct in main `mod.rs` (core type)

**Testing**:
- Run `cargo test --all` to ensure all tests pass
- Run `cargo clippy --all-targets`
- Verify no breaking changes to imports

**Success Criteria**:
- [ ] Three new type modules created
- [ ] Main `mod.rs` reduced to < 200 lines
- [ ] All re-exports properly configured
- [ ] All tests pass
- [ ] No clippy warnings
- [ ] Ready to commit

### Phase 5: Final Cleanup and Documentation

**Goal**: Finalize the refactoring with documentation and verification

**Changes**:
- Add module-level documentation to each new module
- Update any inline documentation referencing old structure
- Verify all new modules follow project conventions
- Run full test suite with coverage
- Final `cargo fmt` pass
- Update any references in other modules if needed

**Testing**:
- Run `just ci` - Full CI checks
- Run `cargo test --all-features`
- Run `cargo doc --no-deps` to ensure docs build
- Visual inspection of module structure

**Success Criteria**:
- [ ] All new modules have proper documentation
- [ ] Full test suite passes
- [ ] No clippy warnings
- [ ] Documentation builds successfully
- [ ] Module structure is clean and logical
- [ ] Ready for final commit

## Testing Strategy

**For each phase**:
1. Run `cargo test --lib` to verify existing tests pass
2. Run `cargo clippy` to check for warnings
3. Run `cargo fmt --all -- --check` to verify formatting
4. Manual review of public API compatibility

**Final verification**:
1. `just ci` - Full CI checks
2. `cargo test --all-features` - All tests with all feature combinations
3. `cargo doc --no-deps` - Documentation builds
4. Manual verification of file sizes:
   - `wc -l src/priority/mod.rs` (should be < 200)
   - `wc -l src/priority/*.rs` (verify distribution)

## Rollback Plan

If a phase fails:
1. Revert the phase with `git reset --hard HEAD~1`
2. Review the failure cause:
   - Compilation errors → check for missing imports or visibility
   - Test failures → check for behavioral changes in extraction
   - Clippy warnings → address before proceeding
3. Adjust the approach:
   - For compilation: ensure all types are properly re-exported
   - For tests: verify extracted logic maintains same behavior
   - For complexity: consider smaller extraction steps
4. Retry the phase with adjustments

## Notes

### Key Considerations

1. **Backward Compatibility**: The public API of UnifiedAnalysis must remain unchanged. All extracted functionality should be accessed through the original methods (which will delegate to new modules) or through clearly documented new paths.

2. **Module Organization**: Following Rust conventions:
   - New modules should be added to `src/priority/`
   - Re-export types at the appropriate level
   - Use `pub(crate)` for internal helpers

3. **Functional Refactoring**: Where possible during extraction:
   - Convert mutable patterns to immutable
   - Extract pure calculation functions
   - Separate I/O from computation

4. **Test Coverage**: The existing test suite (lines 1180-1659) should continue to pass without modification. These tests are comprehensive and cover:
   - Serialization (File vs Function variants)
   - Debt density calculations
   - Threshold filtering
   - Priority sorting

5. **Incremental Progress**: Each phase should:
   - Compile successfully
   - Pass all tests
   - Be committable independently
   - Reduce file size measurably

### Potential Challenges

1. **Circular Dependencies**: The UnifiedAnalysis struct is used throughout. May need to use traits or accept `&UnifiedAnalysis` parameters.

2. **Privacy Boundaries**: Some helper methods (like `is_critical_item`, `generate_batch_action`) are private but may need to become pub(crate) when extracted.

3. **Complex Interactions**: Methods like `get_tiered_display` call multiple helpers. May need to extract helpers first or keep them in same module.

4. **Test Module Location**: The test modules at the end (lines 1180-1659) may need imports updated as types move.

### Success Metrics

After completion, verify:
- Main file: ~150-200 lines (types + minimal impl)
- Queries module: ~150-200 lines (7 methods)
- Utils module: ~250-300 lines (11 methods, may split further)
- Computation module: ~200-250 lines (complex calculations)
- Type modules: ~200-300 lines each (type definitions)
- **Total reduction**: 1658 → ~200 lines in main file
- **Responsibilities**: 6 → 1 per file/module
