# Implementation Plan: Refactor God Object formatter.rs

## Problem Summary

**Location**: ./src/priority/formatter.rs:file:0
**Priority Score**: 82.30
**Debt Type**: God Object / High Complexity File
**Current Metrics**:
- Lines of Code: 2,822
- Functions: 106 (56 module-level functions, 5 impl methods)
- Total Cyclomatic Complexity: 210
- Average Complexity: 1.98
- Max Complexity: 12
- Coverage: 0.0%
- Responsibilities: 15 distinct domains
- God Object Score: 1.0

**Issue**: The `formatter.rs` file is a god object with 106 functions and 2,822 lines of code handling multiple distinct responsibilities: format orchestration, section formatting, terminal output, detailed item formatting, god object analysis, metrics classification, utility functions, and more. This violates the single responsibility principle and makes the code difficult to maintain, test, and understand.

**Recommendation**: Split into: 1) Core formatter orchestrator, 2) Section writers (one per major section), 3) Style/theme handling. Max 20 functions per writer module.

## Target State

**Expected Impact**:
- Complexity Reduction: 42.0
- Maintainability Improvement: 8.23
- Test Effort: 282.2

**Success Criteria**:
- [ ] File split into focused modules with < 20 functions each
- [ ] Each module has a single, clear responsibility
- [ ] Public API remains unchanged (backward compatibility)
- [ ] All existing tests continue to pass
- [ ] No clippy warnings
- [ ] Code coverage improves from 0% to at least 50%
- [ ] Each extracted module is independently testable
- [ ] Total complexity reduced by ~42 points

## Implementation Phases

This refactoring will be broken into 5 incremental phases, each resulting in working, testable code.

### Phase 1: Extract Section Writers

**Goal**: Extract all section formatting functions into a dedicated module that handles the generation of formatted output sections.

**Rationale**: The section formatting functions (format_header_section, format_location_section, etc.) are cohesive and can be extracted as a unit. They depend on FormatContext and generate FormattedSections, making them a natural module boundary.

**Changes**:
1. Create `src/priority/formatter/sections.rs`
2. Move the following structures and functions:
   - `struct FormattedSections`
   - `fn generate_formatted_sections`
   - `fn format_header_section`
   - `fn format_location_section`
   - `fn format_action_section`
   - `fn format_impact_section`
   - `fn format_complexity_section`
   - `fn format_evidence_section`
   - `fn format_dependencies_section_with_config`
   - `fn format_dependencies_section`
   - `fn format_debt_specific_section`
   - `fn format_rationale_section`
   - `fn apply_formatted_sections`
3. Make necessary types public (FormatContext, FormattedSections)
4. Update `formatter.rs` to import from `sections` module

**Testing**:
- Run `cargo test --lib` to verify existing tests pass
- Run `cargo clippy` to check for warnings
- Verify all section formatting still works correctly

**Success Criteria**:
- [ ] `src/priority/formatter/sections.rs` created with ~13 functions
- [ ] All section formatting logic moved
- [ ] Public API unchanged
- [ ] All tests pass
- [ ] No clippy warnings
- [ ] Complexity reduced by ~10 points

### Phase 2: Extract Context and Info Types

**Goal**: Extract the context and info structures into a dedicated types module to support better organization and separation of concerns.

**Rationale**: The various Info structures (SeverityInfo, LocationInfo, ComplexityInfo, etc.) and FormatContext are pure data types that can be extracted to improve code organization and make the formatter more modular.

**Changes**:
1. Create `src/priority/formatter/context.rs`
2. Move the following structures and their impl blocks:
   - `struct FormatContext` and `fn create_format_context`
   - `struct SeverityInfo` + impl
   - `struct LocationInfo` + impl
   - `struct ComplexityInfo` + impl
   - `struct DependencyInfo` + impl
   - `enum DebtSpecificInfo` + impl
3. Make all types and constructors public
4. Update imports in `formatter.rs` and `sections.rs`

**Testing**:
- Run `cargo test --lib` to verify existing tests pass
- Run `cargo clippy` to check for warnings
- Verify context creation and info extraction still works

**Success Criteria**:
- [ ] `src/priority/formatter/context.rs` created with types and builders
- [ ] All context/info types moved
- [ ] Clean module boundaries established
- [ ] All tests pass
- [ ] No clippy warnings

### Phase 3: Extract Utility and Classification Functions

**Goal**: Extract utility and classification functions into focused utility modules.

**Rationale**: Functions like `get_file_extension`, `get_language_name`, `classify_file_size`, etc. are pure utility functions that don't belong in the main formatter. They can be grouped by responsibility into utility modules.

**Changes**:
1. Create `src/priority/formatter/utils.rs`
2. Move utility functions:
   - `fn get_file_extension`
   - `fn get_language_name`
   - `fn determine_file_type_label`
   - `fn format_visibility`
   - `fn format_role`
   - `fn format_debt_type`
   - `fn get_severity_label`
   - `fn get_severity_color`
3. Create `src/priority/formatter/classifiers.rs`
4. Move classification functions:
   - `fn classify_file_size`
   - `fn classify_function_count`
   - `fn calculate_impact_message`
5. Create `src/priority/formatter/extractors.rs`
6. Move extraction functions:
   - `fn extract_complexity_info`
   - `fn extract_dependency_info`
7. Update imports throughout

**Testing**:
- Run `cargo test --lib` to verify existing tests pass
- Run `cargo clippy` to check for warnings
- Verify all utility, classification, and extraction functions work

**Success Criteria**:
- [ ] Three new utility modules created (utils, classifiers, extractors)
- [ ] ~15 utility functions moved
- [ ] Each module has < 10 functions
- [ ] All tests pass
- [ ] No clippy warnings
- [ ] Complexity reduced by ~12 points

### Phase 4: Extract Item Formatters

**Goal**: Extract the various item formatting functions into dedicated item formatter modules organized by formatting style.

**Rationale**: The code has different formatting strategies (compact, detailed, mixed, file-based) that can be separated into focused modules, each handling a specific output format.

**Changes**:
1. Create `src/priority/formatter/item_formatters.rs`
2. Move item formatting functions:
   - `fn format_priority_item`
   - `fn format_detailed_item`
   - `fn format_mixed_priority_item`
   - `fn format_file_priority_item`
   - `fn format_compact_item`
   - `fn format_item_location`
3. Create `src/priority/formatter/detailed.rs`
4. Move detailed formatting functions:
   - `fn format_detailed_metrics`
   - `fn format_scoring_and_dependencies`
   - `fn format_god_object_steps`
   - `fn format_module_structure_analysis`
   - `fn format_language_specific_advice`
   - `fn format_generic_god_object_steps`
   - `fn generate_why_message`
5. Create `src/priority/formatter/lists.rs`
6. Move list formatting:
   - `fn format_truncated_list`
7. Update imports and make functions public as needed

**Testing**:
- Run `cargo test --lib` to verify existing tests pass
- Run `cargo clippy` to check for warnings
- Verify all item formatting functions work correctly

**Success Criteria**:
- [ ] Three new modules created (item_formatters, detailed, lists)
- [ ] ~15 formatting functions moved
- [ ] Each module focused on specific formatting style
- [ ] All tests pass
- [ ] No clippy warnings
- [ ] Complexity reduced by ~15 points

### Phase 5: Reorganize Core Formatter and Create Module Index

**Goal**: Clean up the main `formatter.rs` to be a thin orchestration layer and create a proper module structure with re-exports.

**Rationale**: The main formatter.rs should only contain the public API and orchestration logic, delegating to specialized modules. This creates a clean separation between the public interface and implementation details.

**Changes**:
1. Create `src/priority/formatter/mod.rs` as the module index
2. Re-export public API from `mod.rs`:
   - `pub use sections::*;`
   - `pub use context::*;`
   - `pub use item_formatters::*;`
   - `pub use utils::{format_debt_type, get_severity_label, get_severity_color};`
   - `pub use extractors::{extract_complexity_info, extract_dependency_info};`
3. Keep in `formatter.rs` (should be ~200-300 lines):
   - `enum OutputFormat`
   - `fn format_priorities` and variants
   - `fn format_default_with_config` (orchestration)
   - `fn format_tail_with_config` (orchestration)
   - `fn format_summary_terminal` (orchestration)
   - `fn format_tiered_terminal` (orchestration)
   - `fn format_tier_terminal`
   - `fn format_display_group_terminal`
   - Top-level orchestration functions
4. Update all imports to use the new module structure
5. Ensure backward compatibility by maintaining existing public API

**Testing**:
- Run `cargo test --all-features` to verify all tests pass
- Run `cargo clippy` to check for warnings
- Run `cargo build --release` to verify compilation
- Manually test key formatting outputs

**Success Criteria**:
- [ ] `mod.rs` created with proper re-exports
- [ ] `formatter.rs` reduced to < 400 lines
- [ ] Clear module hierarchy established
- [ ] Public API unchanged (backward compatible)
- [ ] All tests pass
- [ ] No clippy warnings
- [ ] Total complexity reduced by ~42 points (as predicted)

## Module Organization (Target State)

```
src/priority/formatter/
├── mod.rs                 # Module index and re-exports
├── context.rs             # FormatContext and Info types (~150 lines, 6 types)
├── sections.rs            # Section formatters (~300 lines, 13 functions)
├── item_formatters.rs     # Item formatting functions (~250 lines, 6 functions)
├── detailed.rs            # Detailed formatting logic (~400 lines, 9 functions)
├── utils.rs               # Utility functions (~150 lines, 8 functions)
├── classifiers.rs         # Classification logic (~100 lines, 3 functions)
├── extractors.rs          # Data extraction (~80 lines, 2 functions)
├── lists.rs               # List formatting (~50 lines, 1 function)
├── dependencies.rs        # Existing dependency formatting (keep as-is)
└── verbosity.rs           # Existing verbosity handling (keep as-is)
```

Main `formatter.rs` becomes orchestration only (~400 lines, 12 functions).

## Testing Strategy

**For each phase**:
1. Run `cargo test --lib` to verify existing tests pass
2. Run `cargo clippy --all-targets` to check for warnings
3. Run `cargo fmt` to ensure consistent formatting
4. Verify specific functionality related to the phase

**Phase-specific testing**:
- **Phase 1**: Test section generation with sample debt items
- **Phase 2**: Test context creation and info extraction
- **Phase 3**: Test utility functions, classifiers, and extractors independently
- **Phase 4**: Test item formatting with various debt types
- **Phase 5**: Test full integration and backward compatibility

**Final verification**:
1. `cargo test --all-features` - All tests pass
2. `cargo clippy --all-targets --all-features -- -D warnings` - No warnings
3. `cargo fmt --all -- --check` - Code is formatted
4. `just ci` - Full CI checks
5. `cargo tarpaulin` - Generate coverage report (target: 50%+)
6. `debtmap analyze` - Verify improvement in metrics

## Rollback Plan

If a phase fails:
1. Identify the failing test or compilation error
2. Use `git diff` to review changes in the phase
3. If the issue is fixable quickly (< 10 minutes), fix it
4. Otherwise, revert the phase with `git reset --hard HEAD~1`
5. Review the failure and adjust the plan
6. Retry the phase with fixes

## Notes

### Important Considerations:

1. **Backward Compatibility**: The public API must remain unchanged. All existing code that uses the formatter should continue to work without modification.

2. **Module Visibility**: Be careful about what is made public. Only expose types and functions that are needed by other modules or external code.

3. **Import Organization**: Keep imports organized and minimal. Use re-exports in `mod.rs` to provide a clean public interface.

4. **Testing During Refactoring**: Since the file currently has 0% coverage, we're primarily relying on integration tests. Each phase should maintain the existing behavior.

5. **God Object Analysis Functions**: The functions related to god object analysis and recommendations (format_god_object_steps, etc.) belong in the `detailed.rs` module as they're part of detailed output formatting.

6. **Incremental Commits**: After each phase passes all tests and clippy checks, commit the changes with a clear message describing what was extracted.

7. **Function Count Target**: Each module should have < 20 functions (ideally < 15) to avoid recreating the god object problem.

### Risk Mitigation:

- **Circular Dependencies**: Be careful when extracting modules that depend on each other. Use the hierarchy: context → utils/classifiers/extractors → sections → item_formatters → formatter
- **Type Visibility**: Some types may need to be public for module communication but shouldn't be part of the public API. Consider using `pub(crate)` or `pub(super)`.
- **Performance**: This is primarily a structural refactoring. Performance should not be affected, but verify that compilation times don't increase significantly.

### Post-Refactoring Opportunities:

After this refactoring is complete, consider:
1. Adding unit tests for extracted modules (especially pure utility functions)
2. Adding integration tests for different formatting scenarios
3. Further breaking down `detailed.rs` if it remains too large
4. Extracting common patterns into shared utilities
