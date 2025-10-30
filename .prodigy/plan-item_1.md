# Implementation Plan: Refactor God Object src/config.rs

## Problem Summary

**Location**: src/config.rs:file:0
**Priority Score**: 112.07
**Debt Type**: God Object (File-level)

**Current Metrics**:
- Lines of Code: 2,888
- Functions: 226
- Cyclomatic Complexity: 256
- Coverage: 0%
- Domain Count: 5 distinct domains
- God Object Score: 1.0 (Critical)
- Struct Count: 32 structs in single file
- DebtmapConfig struct: 27 fields (God Class)

**Issue**: The config.rs file is a massive God Object containing 2,888 lines with 226 functions across 5 distinct domains (thresholds, scoring, detection, core_config, misc). This violates single responsibility principle and makes the codebase difficult to maintain, test, and navigate. The analysis identifies this as requiring urgent refactoring into 5-8 focused modules with <30 functions each.

## Target State

**Expected Impact**:
- Complexity Reduction: 51.2 points
- Maintainability Improvement: 11.2 points
- Test Effort Reduction: 288.8 points

**Success Criteria**:
- [ ] src/config.rs reduced to <500 lines (orchestration/re-exports only)
- [ ] 5 new focused sub-modules created under src/config/
- [ ] Each sub-module has <50 functions and single clear responsibility
- [ ] All existing tests continue to pass
- [ ] No clippy warnings
- [ ] Public API remains unchanged (backward compatible)
- [ ] Proper formatting with cargo fmt

## Implementation Phases

### Phase 1: Create Module Structure and Extract Scoring Domain

**Goal**: Create the src/config/ directory structure and extract the scoring-related types as the first domain module. This is the safest starting point as scoring types have clear boundaries.

**Changes**:
1. Create `src/config/` directory
2. Create `src/config/scoring.rs` with:
   - `ScoringWeights` struct and impl
   - `RoleMultipliers` struct and impl
   - `RoleCoverageWeights` struct
   - `ComplexityWeightsConfig` struct and impl
   - `RebalancedScoringConfig` struct and impl
   - `RoleMultiplierConfig` struct and impl
   - All associated default functions (~30 functions)
3. Update `src/config.rs` to re-export these types
4. Update internal uses to import from new location

**Testing**:
- Run `cargo test --lib` to ensure all tests pass
- Run `cargo clippy` to check for warnings
- Verify imports work correctly throughout codebase

**Success Criteria**:
- [ ] src/config/scoring.rs created (~145 lines)
- [ ] All scoring types moved and functional
- [ ] All tests pass
- [ ] No clippy warnings
- [ ] Ready to commit

### Phase 2: Extract Thresholds Domain

**Goal**: Extract all threshold-related configuration into a focused module.

**Changes**:
1. Create `src/config/thresholds.rs` with:
   - `ValidationThresholds` struct and impl
   - `GodObjectThresholds` struct and impl
   - `FileSizeThresholds` struct and impl
   - `ThresholdsConfig` struct
   - All associated default functions (~15 functions)
2. Update `src/config.rs` to re-export these types
3. Update any internal cross-references

**Testing**:
- Run `cargo test --lib` to ensure all tests pass
- Run `cargo clippy` to check for warnings
- Verify threshold configurations load correctly

**Success Criteria**:
- [ ] src/config/thresholds.rs created (~158 lines)
- [ ] All threshold types moved and functional
- [ ] All tests pass
- [ ] No clippy warnings
- [ ] Ready to commit

### Phase 3: Extract Detection Domain

**Goal**: Extract detection configuration (orchestrator, constructor, accessor detection) into focused module.

**Changes**:
1. Create `src/config/detection.rs` with:
   - `OrchestratorDetectionConfig` struct and impl
   - `ConstructorDetectionConfig` struct and impl
   - `AccessorDetectionConfig` struct and impl
   - `DataFlowClassificationConfig` struct and impl
   - All associated default functions (~20 functions)
2. Update `src/config.rs` to re-export these types
3. Update detection-related code to use new imports

**Testing**:
- Run `cargo test --lib` to ensure all tests pass
- Run `cargo clippy` to check for warnings
- Verify detection configurations work correctly

**Success Criteria**:
- [ ] src/config/detection.rs created (~100 lines)
- [ ] All detection types moved and functional
- [ ] All tests pass
- [ ] No clippy warnings
- [ ] Ready to commit

### Phase 4: Extract Miscellaneous Types and Core Config

**Goal**: Extract remaining smaller configuration types into appropriate modules.

**Changes**:
1. Create `src/config/display.rs` with:
   - `DisplayConfig` struct and impl
   - `VerbosityLevel` enum
   - Associated default functions
2. Create `src/config/language.rs` with:
   - `LanguagesConfig` struct and impl
   - `LanguageFeatures` struct and impl
   - `ErrorHandlingConfig` struct and impl
   - `ErrorPatternConfig` struct
   - `SeverityOverride` struct
   - Associated default functions
3. Create `src/config/analysis.rs` with:
   - `EntropyConfig` struct and impl
   - `CallerCalleeConfig` struct and impl
   - `ContextConfig` struct and impl
   - `ContextRuleConfig` struct
   - `ContextMatcherConfig` struct
   - `FunctionPatternConfig` struct
   - Associated default functions
4. Update `src/config.rs` to re-export all types

**Testing**:
- Run `cargo test --lib` after each new file
- Run `cargo clippy` to check for warnings
- Verify all configuration types accessible

**Success Criteria**:
- [ ] src/config/display.rs created (~80 lines)
- [ ] src/config/language.rs created (~150 lines)
- [ ] src/config/analysis.rs created (~200 lines)
- [ ] All types moved and functional
- [ ] All tests pass
- [ ] No clippy warnings
- [ ] Ready to commit

### Phase 5: Extract Main Config and Refactor DebtmapConfig

**Goal**: Create a focused module for the main DebtmapConfig struct and refactor it to use the extracted types.

**Changes**:
1. Create `src/config/main.rs` with:
   - `DebtmapConfig` struct (refactored to reference sub-configs)
   - `ClassificationConfig` struct
   - `NormalizationConfig` struct
   - `GodObjectConfig` struct
   - `OutputConfig` struct
   - `IgnoreConfig` struct
   - Associated impl blocks
2. Refactor `DebtmapConfig` to group related fields into sub-structs
3. Update loading/parsing logic
4. Update `src/config.rs` to re-export main config

**Testing**:
- Run `cargo test --lib` to ensure all tests pass
- Run `cargo clippy` to check for warnings
- Test config file loading with existing .toml files
- Verify backward compatibility

**Success Criteria**:
- [ ] src/config/main.rs created (~250 lines)
- [ ] DebtmapConfig refactored with clear structure
- [ ] All tests pass
- [ ] Config loading works with existing files
- [ ] No clippy warnings
- [ ] Ready to commit

### Phase 6: Create Module Coordinator and Refactor config.rs

**Goal**: Transform src/config.rs into a thin module coordinator that re-exports types and provides the loading interface.

**Changes**:
1. Create `src/config/mod.rs` with:
   - Module declarations for all sub-modules
   - Public re-exports of all types
   - Config loading functions (load_config, get_config, etc.)
   - Helper functions for accessing specific configs
2. Replace `src/config.rs` with `src/config/mod.rs`
3. Ensure all getter functions (get_scoring_weights, get_entropy_config, etc.) work correctly
4. Verify public API is identical to before refactoring

**Testing**:
- Run `cargo test --lib --all-features` - full test suite
- Run `cargo clippy --all-targets --all-features` - full linting
- Run `cargo doc --no-deps` - ensure docs build
- Test with actual debtmap runs on test projects
- Verify no breaking changes to public API

**Success Criteria**:
- [ ] src/config/mod.rs created (<400 lines, down from 2,888)
- [ ] All 5+ sub-modules properly declared and re-exported
- [ ] Public API unchanged and backward compatible
- [ ] All tests pass (including integration tests)
- [ ] No clippy warnings
- [ ] Documentation builds successfully
- [ ] Ready to commit

## Testing Strategy

**For each phase**:
1. Run `cargo test --lib` to verify existing tests pass
2. Run `cargo clippy` to check for warnings
3. Verify imports compile correctly throughout codebase
4. Check that moved types are accessible at expected paths

**After Phase 3** (halfway point):
1. Run full test suite: `cargo test --all-features`
2. Check compilation: `cargo check --all-targets`
3. Verify no regressions in actual usage

**Final verification** (after Phase 6):
1. `cargo test --all-features` - All tests pass
2. `cargo clippy --all-targets --all-features -- -D warnings` - No warnings
3. `cargo fmt --all -- --check` - Code is formatted
4. `cargo doc --no-deps` - Documentation builds
5. Run debtmap on test projects to verify no behavioral changes
6. Check git diff to verify public API unchanged

## Rollback Plan

If a phase fails:

1. **Immediate Rollback**: `git reset --hard HEAD~1` to revert the phase
2. **Review the Failure**:
   - Examine compilation errors or test failures
   - Check for missed dependencies between types
   - Verify re-exports are correct
3. **Adjust the Plan**:
   - May need to move types together if they have tight coupling
   - Check for circular dependencies between modules
   - Ensure all necessary helper functions moved with their types
4. **Retry with Adjustments**:
   - Fix the identified issues
   - Re-attempt the phase
   - Commit when successful

**Common Issues to Watch For**:
- Circular dependencies between config types
- Missing re-exports causing compilation failures
- Default functions not moved with their structs
- Getter functions referencing types before they're imported
- Tests importing old paths instead of new module structure

## Notes

**Key Considerations**:

1. **Backward Compatibility**: The public API must remain unchanged. All existing imports should continue to work through re-exports.

2. **Module Organization**: Following the debtmap recommendations, we're creating 5 focused domains:
   - `scoring` - All scoring weight configurations
   - `thresholds` - Validation and limit thresholds
   - `detection` - Function role detection configs
   - `display` + `language` + `analysis` - Remaining configs by purpose
   - `main` - DebtmapConfig orchestration

3. **Incremental Approach**: Each phase moves ~20-40 functions and ~100-400 lines, keeping changes manageable and testable.

4. **Pure Function Preservation**: The existing code already uses pure functions with good functional patterns. We're preserving these while improving organization.

5. **Testing Focus**: Since current coverage is 0%, we focus on ensuring existing tests pass rather than adding new coverage in this refactoring.

6. **Config Loading**: All config loading and parsing logic remains in the main module to maintain the interface. Only type definitions and their impls move to sub-modules.

**Gotchas**:
- Watch for `use` statements that need updating
- Ensure `Default` impls call the right default functions after moves
- Test with actual config files to ensure serde serialization still works
- Some structs may reference types from multiple domains - handle carefully
