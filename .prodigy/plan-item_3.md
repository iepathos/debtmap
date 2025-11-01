# Implementation Plan: Refactor src/config.rs God Object

## Problem Summary

**Location**: ./src/config.rs:file:0
**Priority Score**: 69.84
**Debt Type**: God Object (File-level)
**Current Metrics**:
- Lines of Code: 2424
- Functions: 181
- Average Complexity: 1.09
- Coverage: 0%
- God Object Score: 1.0 (Critical)
- Domains: 4 (thresholds, misc, detection, core_config)
- Domain Diversity: 0.16 (very low - indicates poor separation)

**Issue**: URGENT: 2424 lines, 181 functions! This config file has grown into a massive god object with 4 distinct domains poorly organized. The file contains 27-field struct (DebtmapConfig), 96 module-level functions (mostly default value providers), and mixes configuration types, validation, loading, and accessor functions. Cross-domain analysis indicates 4 clear split opportunities for better organization.

## Target State

**Expected Impact** (from debtmap):
- Complexity Reduction: 39.6 points
- Maintainability Improvement: 6.98
- Test Effort Reduction: 242.4 lines

**Success Criteria**:
- [ ] File split into 5 focused modules (<500 lines each)
- [ ] Each module has single clear responsibility
- [ ] All existing tests continue to pass
- [ ] No clippy warnings
- [ ] Proper formatting with rustfmt
- [ ] Public API remains backward compatible
- [ ] God object score reduced to < 0.3

## Implementation Phases

### Phase 1: Create Module Structure and Extract Thresholds

**Goal**: Create the config module directory structure and extract the first domain (thresholds)

**Changes**:
- Create `src/config/` directory
- Create `src/config/thresholds.rs` for all threshold-related types
- Move `GodObjectThresholds`, `ValidationThresholds`, `FileSizeThresholds`, `ThresholdsConfig` structs
- Move ~20 default value functions for thresholds
- Create `src/config/mod.rs` with re-exports

**Structs to move**:
- `GodObjectThresholds` (lines 464-535)
- `ValidationThresholds` (lines 785-881)
- `FileSizeThresholds` (lines 900-953)
- `ThresholdsConfig` (lines 755-783)

**Functions to move**:
- All `default_max_*`, `default_min_*`, `default_*_threshold` functions related to these structs

**Testing**:
- Run `cargo test --lib -- config::tests` to verify config tests pass
- Run `cargo build` to ensure no compilation errors
- Check that imports resolve correctly

**Success Criteria**:
- [ ] `src/config/thresholds.rs` created (~200 lines)
- [ ] All threshold types moved and re-exported
- [ ] All tests pass
- [ ] No clippy warnings
- [ ] Ready to commit

### Phase 2: Extract Detection Configuration

**Goal**: Move all detection-related configuration to a dedicated module

**Changes**:
- Create `src/config/detection.rs`
- Move `OrchestratorDetectionConfig`, `ConstructorDetectionConfig`, `AccessorDetectionConfig`
- Move `DataFlowClassificationConfig`, `ErrorHandlingConfig`, `ErrorPatternConfig`, `SeverityOverride`
- Move all related default value functions (~25 functions)
- Update re-exports in `mod.rs`

**Structs to move**:
- `OrchestratorDetectionConfig` (lines 75-140)
- `ConstructorDetectionConfig` (lines 99-205)
- `AccessorDetectionConfig` (lines 208-298)
- `DataFlowClassificationConfig` (lines 301-345)
- `ErrorHandlingConfig` (lines 1397-1440)
- `ErrorPatternConfig` (lines 1463-1482)
- `SeverityOverride` (lines 1484-1495)

**Testing**:
- Run `cargo test --lib` to verify all tests pass
- Verify detection config accessors work correctly
- Check that detection modules can access new types

**Success Criteria**:
- [ ] `src/config/detection.rs` created (~350 lines)
- [ ] All detection types moved and re-exported
- [ ] All tests pass
- [ ] No clippy warnings
- [ ] Ready to commit

### Phase 3: Extract Context and Classification Configuration

**Goal**: Move context-aware and classification configuration to dedicated modules

**Changes**:
- Create `src/config/classification.rs`
- Move `ClassificationConfig`, `CallerCalleeConfig`, `ContextConfig`, `ContextRuleConfig`, `ContextMatcherConfig`, `FunctionPatternConfig`
- Move all related default value functions (~10 functions)
- Update re-exports in `mod.rs`

**Structs to move**:
- `ClassificationConfig` (lines 717-731)
- `CallerCalleeConfig` (lines 28-73)
- `ContextConfig` (lines 350-378)
- `ContextRuleConfig` (lines 380-405)
- `ContextMatcherConfig` (lines 407-424)
- `FunctionPatternConfig` (lines 426-444)

**Testing**:
- Run `cargo test --lib` to verify all tests pass
- Verify classification and context detection works
- Check backward compatibility of public API

**Success Criteria**:
- [ ] `src/config/classification.rs` created (~200 lines)
- [ ] All classification types moved and re-exported
- [ ] All tests pass
- [ ] No clippy warnings
- [ ] Ready to commit

### Phase 4: Extract Language and Display Configuration

**Goal**: Move language-specific and display configuration to dedicated modules

**Changes**:
- Create `src/config/languages.rs`
- Move `LanguagesConfig`, `LanguageFeatures`, `EntropyConfig`
- Create `src/config/display.rs`
- Move `DisplayConfig`, `VerbosityLevel`, `GodObjectConfig`
- Move all related default value functions (~20 functions)
- Update re-exports in `mod.rs`

**Structs to move to languages.rs**:
- `LanguagesConfig` (lines 984-1003)
- `LanguageFeatures` (lines 1005-1029)
- `EntropyConfig` (lines 1044-1108)

**Structs to move to display.rs**:
- `DisplayConfig` (lines 569-593)
- `VerbosityLevel` (lines 556-567)
- `GodObjectConfig` (lines 446-554)

**Testing**:
- Run `cargo test --lib` to verify all tests pass
- Verify language detection and display formatting works
- Test entropy calculations with new module

**Success Criteria**:
- [ ] `src/config/languages.rs` created (~200 lines)
- [ ] `src/config/display.rs` created (~200 lines)
- [ ] All types moved and re-exported
- [ ] All tests pass
- [ ] No clippy warnings
- [ ] Ready to commit

### Phase 5: Finalize Core Config Module and Config Loading

**Goal**: Clean up remaining code in main config file, focusing on core types and loading logic

**Changes**:
- Keep `DebtmapConfig` in `src/config/core.rs` (the root aggregating struct)
- Keep `IgnoreConfig`, `OutputConfig` in `src/config/core.rs` (small utility types)
- Keep all loading/parsing functions in `src/config/loader.rs`
- Move all accessor functions (`get_*` functions) to `src/config/accessors.rs`
- Update `src/config/mod.rs` to re-export everything for backward compatibility
- Move tests to appropriate module files

**Remaining in core.rs** (~300 lines):
- `DebtmapConfig` struct (lines 603-715)
- `IgnoreConfig`, `OutputConfig` (lines 1150-1161)
- Re-export from scoring submodule

**Move to loader.rs** (~200 lines):
- All file reading and parsing functions
- `load_config()`, `read_config_file()`, `parse_and_validate_config_impl()`
- `try_load_config_from_path()`, `handle_read_error()`, `directory_ancestors_impl()`
- `CONFIG` and `SCORING_WEIGHTS` statics

**Move to accessors.rs** (~200 lines):
- All `get_*` accessor functions (15+ functions)
- `get_config()`, `get_config_safe()`, `get_scoring_weights()`
- All specialized getters for each config section

**Testing**:
- Run full test suite: `cargo test`
- Run clippy: `cargo clippy --all-targets`
- Run formatter: `cargo fmt --check`
- Verify all public API usage in other modules still works

**Success Criteria**:
- [ ] Core config types in `core.rs` (<300 lines)
- [ ] Config loading logic in `loader.rs` (~200 lines)
- [ ] Accessor functions in `accessors.rs` (~200 lines)
- [ ] All tests moved to appropriate modules and passing
- [ ] All imports updated throughout codebase
- [ ] Public API backward compatible (re-exports work)
- [ ] No clippy warnings
- [ ] Properly formatted
- [ ] Ready to commit

## Implementation Strategy

**Key Principles**:
1. **Maintain backward compatibility**: Use re-exports in `src/config/mod.rs` so existing code doesn't break
2. **Move tests with code**: When moving a struct, move its tests to the new module
3. **Incremental commits**: Commit after each phase so work is preserved
4. **Functional organization**: Group by domain/responsibility, not by type
5. **Keep public API surface unchanged**: All existing `pub use` statements preserved

**Module Structure** (final state):
```
src/config/
├── mod.rs              # Re-exports for backward compatibility (~100 lines)
├── core.rs             # DebtmapConfig and core types (~300 lines)
├── loader.rs           # Config file loading and parsing (~200 lines)
├── accessors.rs        # All get_* accessor functions (~200 lines)
├── thresholds.rs       # All threshold configuration (~200 lines)
├── detection.rs        # Detection configuration (~350 lines)
├── classification.rs   # Classification configuration (~200 lines)
├── languages.rs        # Language-specific configuration (~200 lines)
├── display.rs          # Display and output configuration (~200 lines)
└── scoring/            # Already exists - scoring configuration
    └── mod.rs
```

**Total**: ~2000 lines across 9 focused files (down from 2424 in one file)

## Testing Strategy

**For each phase**:
1. Run `cargo test --lib -- config` to verify config tests pass
2. Run `cargo test` to verify all tests pass (may catch import issues)
3. Run `cargo clippy` to check for warnings
4. Run `cargo build` to ensure compilation succeeds
5. Visually inspect that imports resolve correctly

**Final verification**:
1. `cargo test --all-features` - Full test suite
2. `cargo clippy --all-targets --all-features -- -D warnings` - No warnings allowed
3. `cargo fmt --check` - Ensure formatting
4. `cargo doc --no-deps` - Documentation builds
5. Run debtmap on itself to verify improvement: `cargo run -- analyze`

**Expected improvements**:
- God object score: 1.0 → < 0.3
- File size: 2424 lines → largest file ~350 lines
- Function count per file: 181 → max ~40 per file
- Domain separation: 0.16 → > 0.8 (clear boundaries)

## Rollback Plan

If a phase fails:
1. Review the error carefully
2. Use `git status` and `git diff` to see what changed
3. If needed, revert with `git reset --hard HEAD~1`
4. Review the failure reason
5. Adjust the plan if needed
6. Retry the phase with corrections

Each phase is designed to be independently valuable and reversible.

## Notes

**Important considerations**:

1. **Re-exports are critical**: The public API must remain unchanged. All moved types must be re-exported from `src/config/mod.rs` using `pub use` statements.

2. **Test organization**: Tests should move with their code. If we move `ValidationThresholds` to `thresholds.rs`, its tests should move too.

3. **Circular dependencies**: Watch out for circular dependencies. The `loader.rs` module will need to import from all other config modules to construct `DebtmapConfig`.

4. **OnceLock statics**: The `CONFIG` and `SCORING_WEIGHTS` statics use `OnceLock` for thread-safe lazy initialization. These should stay with the loader logic.

5. **Scoring module**: The existing `src/config/scoring.rs` submodule is already well-organized. We'll keep it as-is and continue re-exporting its types.

6. **Default implementations**: Many structs have `Default` impls that call default value functions. Keep the default functions in the same module as the struct.

7. **Documentation**: When moving code, preserve all documentation comments. This is part of the public API.

8. **Feature flags**: Some config types are conditional on feature flags. Ensure these are preserved correctly.

**Risks**:
- Breaking imports in other modules (mitigated by re-exports)
- Circular dependency issues (mitigated by careful module design)
- Test failures due to visibility changes (mitigated by incremental testing)

**Not included in this plan**:
- Refactoring the `DebtmapConfig` struct itself (out of scope for this debt item)
- Changing the config file format (out of scope)
- Optimizing config loading performance (not a priority)
