# Implementation Plan: Split formatter.rs into Focused Sub-Modules

## Problem Summary

**Location**: ./src/priority/formatter.rs:file:0
**Priority Score**: 105.37
**Debt Type**: File-level God Object (God Module)
**Current Metrics**:
- Lines of Code: 2,985
- Functions: 117
- Total Cyclomatic Complexity: 243
- Max Function Complexity: 13
- Coverage: 0.0%

**Issue**: The `formatter.rs` module has grown too large with 117 functions across 2,985 lines, making it difficult to navigate, test, and maintain. The module handles multiple distinct responsibilities including dependency filtering, output formatting (default/tail/detailed), terminal display, section formatting, and utility functions for metrics, labels, and extraction. This violates the single responsibility principle and creates a maintenance burden.

**Recommendation**: Split into: 1) Core formatter 2) Section writers (one per major section) 3) Style/theme handling. Max 20 functions per writer module.

## Target State

**Expected Impact** (from debtmap):
- Complexity Reduction: 48.6 points
- Maintainability Improvement: 10.54 points
- Test Effort Reduction: 298.5 points (easier to test smaller modules)

**Success Criteria**:
- [ ] Module broken into 5-7 focused sub-modules with clear responsibilities
- [ ] Each sub-module has < 400 lines and < 20 functions
- [ ] All existing tests continue to pass
- [ ] No clippy warnings
- [ ] Proper formatting with `cargo fmt`
- [ ] Public API remains unchanged (backward compatibility)

## Implementation Phases

### Phase 1: Extract Dependency Filtering Module

**Goal**: Extract dependency filtering and caller/callee logic into a dedicated module

**Changes**:
- Create new module `src/priority/formatter/dependencies.rs`
- Move functions:
  - `should_include_in_output`
  - `is_standard_library_call`
  - `is_external_crate_call`
  - `filter_dependencies`
  - `format_function_reference`
- Export public functions as needed
- Update imports in `formatter.rs`

**Testing**:
- Run `cargo test --lib` to verify existing tests pass
- Run `cargo clippy` to check for warnings
- Verify compilation with `cargo check`

**Success Criteria**:
- [ ] New `dependencies.rs` module created with 5 functions
- [ ] All tests pass
- [ ] No clippy warnings
- [ ] Ready to commit

### Phase 2: Extract Section Formatting Module

**Goal**: Extract detailed item section formatting into a dedicated module

**Changes**:
- Create new module `src/priority/formatter/sections.rs`
- Move structs and functions related to section formatting:
  - `FormatContext` struct
  - `SeverityInfo`, `LocationInfo`, `ComplexityInfo`, `DependencyInfo` structs
  - `DebtSpecificInfo` enum
  - `FormattedSections` struct
  - All `format_*_section` functions (header, location, action, impact, complexity, evidence, dependencies, debt_specific, rationale)
  - `generate_formatted_sections`
  - `apply_formatted_sections`
  - `create_format_context`
- Export public types and functions
- Update imports in `formatter.rs`

**Testing**:
- Run `cargo test --lib` to verify existing tests pass
- Run `cargo clippy` to check for warnings
- Verify compilation with `cargo check`

**Success Criteria**:
- [ ] New `sections.rs` module created with ~20 functions and types
- [ ] All tests pass
- [ ] No clippy warnings
- [ ] Ready to commit

### Phase 3: Extract Utility Functions Module

**Goal**: Extract utility and helper functions into a dedicated module

**Changes**:
- Create new module `src/priority/formatter/utils.rs`
- Move utility functions:
  - `format_role`
  - `format_visibility`
  - `get_severity_label`
  - `get_severity_color`
  - `get_file_extension`
  - `get_language_name`
  - `classify_file_size`
  - `classify_function_count`
  - `determine_file_type_label`
  - `extract_complexity_info`
  - `extract_dependency_info`
  - `format_truncated_list`
- Export public functions
- Update imports in `formatter.rs`

**Testing**:
- Run `cargo test --lib` to verify existing tests pass
- Run `cargo clippy` to check for warnings
- Verify compilation with `cargo check`

**Success Criteria**:
- [ ] New `utils.rs` module created with ~12 utility functions
- [ ] All tests pass
- [ ] No clippy warnings
- [ ] Ready to commit

### Phase 4: Extract Terminal Display Module

**Goal**: Extract terminal-specific display formatting into a dedicated module

**Changes**:
- Create new module `src/priority/formatter/terminal.rs`
- Move terminal display functions:
  - `format_summary_terminal`
  - `format_tiered_terminal`
  - `format_tier_terminal`
  - `format_display_group_terminal`
  - `format_compact_item`
  - `format_item_location`
- Export public functions
- Update imports in `formatter.rs`

**Testing**:
- Run `cargo test --lib` to verify existing tests pass
- Run `cargo clippy` to check for warnings
- Verify compilation with `cargo check`

**Success Criteria**:
- [ ] New `terminal.rs` module created with ~6 display functions
- [ ] All tests pass
- [ ] No clippy warnings
- [ ] Ready to commit

### Phase 5: Extract Detailed Item Formatting Module

**Goal**: Extract detailed item formatting logic into a dedicated module

**Changes**:
- Create new module `src/priority/formatter/detailed.rs`
- Move detailed formatting functions:
  - `format_detailed`
  - `format_detailed_item`
  - `format_priority_item`
  - `format_mixed_priority_item`
  - `format_file_priority_item`
  - `format_detailed_metrics`
  - `format_scoring_and_dependencies`
  - `generate_why_message`
  - `calculate_impact_message`
  - `format_god_object_steps`
  - `format_module_structure_analysis`
  - `format_language_specific_advice`
  - `format_generic_god_object_steps`
- Export public functions (especially `format_priority_item` and `format_detailed`)
- Update imports in `formatter.rs`

**Testing**:
- Run `cargo test --lib` to verify existing tests pass
- Run `cargo clippy` to check for warnings
- Verify compilation with `cargo check`

**Success Criteria**:
- [ ] New `detailed.rs` module created with ~13 formatting functions
- [ ] All tests pass
- [ ] No clippy warnings
- [ ] Ready to commit

### Phase 6: Reorganize Core Formatter and Create Module Structure

**Goal**: Finalize the module structure and create a clean public API

**Changes**:
- Convert `formatter.rs` to `formatter/mod.rs`
- Keep only core orchestration functions in `mod.rs`:
  - `OutputFormat` enum
  - `format_priorities` and variants
  - `format_default` and variants
  - `format_tail` and variants
  - `format_impact`
  - `format_debt_type`
- Re-export public APIs from sub-modules
- Ensure all internal functions are properly scoped
- Verify module structure:
  ```
  src/priority/formatter/
  ├── mod.rs              (~200 lines, core API)
  ├── dependencies.rs     (~80 lines, 5 functions)
  ├── sections.rs         (~400 lines, 20 functions)
  ├── utils.rs            (~200 lines, 12 functions)
  ├── terminal.rs         (~300 lines, 6 functions)
  ├── detailed.rs         (~800 lines, 13 functions)
  └── formatter_verbosity.rs (existing, already modularized)
  ```

**Testing**:
- Run `cargo test --lib` to verify existing tests pass
- Run `cargo clippy --all-targets --all-features -- -D warnings` for strict linting
- Run `cargo fmt --all -- --check` to verify formatting
- Run full CI with `just ci` if available

**Success Criteria**:
- [ ] Module structure reorganized with clear separation
- [ ] Public API unchanged (backward compatible)
- [ ] All tests pass
- [ ] No clippy warnings
- [ ] Code properly formatted
- [ ] Ready to commit

## Testing Strategy

**For each phase**:
1. Run `cargo test --lib` to verify existing tests pass
2. Run `cargo clippy` to check for warnings
3. Run `cargo check` to verify compilation
4. Manually verify that moved functions are properly imported

**Between phases**:
- Commit after each successful phase
- Use descriptive commit messages explaining what was extracted

**Final verification**:
1. `just ci` - Full CI checks (or `cargo test --all-features`)
2. Verify public API is unchanged by checking exports
3. Check that all 6 sub-modules are properly integrated
4. Ensure no duplicate code exists

## Rollback Plan

If a phase fails:
1. Revert the phase with `git reset --hard HEAD~1`
2. Review the failure:
   - Check for missing imports
   - Verify function visibility (pub vs private)
   - Check for circular dependencies
3. Adjust the approach:
   - May need to extract in different order
   - May need to adjust function visibility
   - May need to add intermediate re-exports
4. Retry the phase with corrections

## Notes

- **Backward Compatibility**: The public API must remain unchanged. All public functions currently exported from `formatter.rs` must still be accessible after the refactoring.
- **Import Strategy**: Use `pub use` to re-export public functions from sub-modules in `mod.rs` to maintain the existing API.
- **Verbosity Module**: The existing `formatter_verbosity.rs` module is already separated and should not be moved.
- **Function Visibility**: Pay careful attention to which functions are currently public vs private. Only public functions need to be re-exported.
- **Testing**: Since there's 0% coverage currently, we won't add new tests in this refactoring. The focus is purely on structural improvement.
- **Incremental Progress**: Each phase should compile and pass tests independently. Don't try to do multiple extractions in one commit.
- **Module Size Guidelines**: Target 200-400 lines per module, maximum 20 functions. The `detailed.rs` module may be slightly larger (~800 lines) due to the complexity of formatting functions, but this is still better than 2,985 lines.

## Expected Final Structure

```
src/priority/formatter/
├── mod.rs                     # Core API orchestration (~200 lines)
│   ├── OutputFormat enum
│   ├── format_priorities family
│   ├── format_default family
│   ├── format_tail family
│   ├── format_impact
│   └── format_debt_type
│
├── dependencies.rs            # Dependency filtering (~80 lines)
│   ├── should_include_in_output
│   ├── is_standard_library_call
│   ├── is_external_crate_call
│   ├── filter_dependencies
│   └── format_function_reference
│
├── sections.rs                # Section formatting (~400 lines)
│   ├── FormatContext and related structs
│   ├── generate_formatted_sections
│   ├── All format_*_section functions
│   └── apply_formatted_sections
│
├── utils.rs                   # Utility functions (~200 lines)
│   ├── Label and classification functions
│   ├── Extraction functions
│   └── Formatting helpers
│
├── terminal.rs                # Terminal display (~300 lines)
│   ├── format_summary_terminal
│   ├── format_tiered_terminal
│   ├── Terminal rendering functions
│   └── Compact item formatting
│
├── detailed.rs                # Detailed item formatting (~800 lines)
│   ├── format_detailed
│   ├── format_detailed_item
│   ├── format_priority_item
│   ├── God object formatting
│   └── Language-specific advice
│
└── formatter_verbosity.rs     # Existing verbosity config
```

This structure provides clear separation of concerns, easier testing boundaries, and better maintainability.
