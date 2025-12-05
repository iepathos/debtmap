---
number: 201
title: Legacy Code Cleanup for 1.0 Release
category: optimization
priority: high
status: draft
dependencies: []
created: 2025-12-04
---

# Specification 201: Legacy Code Cleanup for 1.0 Release

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: None

## Context

Debtmap is approaching its 1.0 release (currently at v0.9.0). The codebase contains several deprecated modules, functions, and compatibility shims that were marked for removal in v0.9.0 or earlier. These legacy components were intentionally kept during active development for backward compatibility, but now create maintenance burden and code confusion.

**Current state**:
- 3 legacy functions in `legacy_compat.rs` that panic when called (marked for removal in v0.9.0)
- 2 deprecated formatter functions using old I/O patterns (deprecated since v0.1.0)
- Legacy cognitive complexity calculation still used as default
- Multiple documentation references to removed/deprecated code
- ~132 lines of deprecated code identified

**Removal rationale**:
1. **Version milestones passed**: Code marked "remove in v0.9.0" is overdue
2. **No active usage**: Deprecated functions have no callers in the codebase
3. **Better replacements exist**: All deprecated code has well-documented modern alternatives
4. **Pre-1.0 cleanup**: Major version is the appropriate time for breaking changes
5. **Reduced confusion**: New contributors encounter deprecated code and don't know what to use

## Objective

Remove all deprecated legacy code and compatibility shims before the 1.0 release, ensuring clean, maintainable codebase with clear modern APIs. This includes removing unused functions, updating documentation references, and making informed decisions about cognitive complexity calculation defaults.

## Requirements

### Functional Requirements

1. **Remove legacy compatibility module**
   - Delete `src/organization/god_object/legacy_compat.rs` (59 lines)
   - Remove re-exports from `src/organization/mod.rs`
   - Remove module declaration from `src/organization/god_object/mod.rs`

2. **Remove deprecated formatter functions**
   - Delete `format_priority_item_legacy()` from `src/priority/formatter/mod.rs` (26 lines)
   - Refactor `format_priority_item()` to use pure + writer pattern directly
   - Delete `apply_formatted_sections()` from `src/priority/formatter/sections.rs` (47 lines)
   - Delete `generate_formatted_sections()` if no longer needed

3. **Evaluate cognitive complexity calculation**
   - Analyze differences between legacy and normalized cognitive complexity
   - Run comprehensive comparison tests on real codebases
   - Decide whether to switch default to normalized or keep legacy
   - Document decision and rationale

4. **Update documentation**
   - Remove references to deleted functions from ARCHITECTURE.md
   - Remove references to deleted functions from REFACTORING_PLAN.md
   - Update any book chapters referencing deprecated code
   - Ensure migration guide exists for external users if needed

### Non-Functional Requirements

1. **No regressions**: All existing tests must pass after removals
2. **Clear migration**: Breaking changes must be documented
3. **Code quality**: Maintain or improve clippy/fmt compliance
4. **Performance**: No performance regressions from refactoring

## Acceptance Criteria

### Phase 1: Safe Immediate Removals (30 minutes)

- [ ] `src/organization/god_object/legacy_compat.rs` file deleted
- [ ] Legacy re-exports removed from `src/organization/mod.rs:92-97`
- [ ] `pub mod legacy_compat;` removed from `src/organization/god_object/mod.rs:38`
- [ ] `format_priority_item_legacy()` deleted from `src/priority/formatter/mod.rs:105-130`
- [ ] ARCHITECTURE.md updated to remove references to deleted functions
- [ ] REFACTORING_PLAN.md updated to remove references to deleted functions
- [ ] `cargo test --all-features` passes
- [ ] `cargo clippy --all-targets --all-features` passes with no warnings

### Phase 2: Formatter Refactoring (30 minutes)

- [ ] `format_priority_item()` refactored to use `pure::format_priority_item()` + `writer::write_priority_item()` directly
- [ ] `apply_formatted_sections()` deleted from `src/priority/formatter/sections.rs:364-410`
- [ ] `generate_formatted_sections()` evaluated and deleted if unused
- [ ] All formatter tests pass
- [ ] No deprecated attribute warnings in formatter module
- [ ] `cargo test --all-features` passes

### Phase 3: Cognitive Complexity Decision (2-4 hours)

- [ ] Comprehensive test suite comparing legacy vs normalized cognitive complexity created
- [ ] Tests run on debtmap itself and at least 2 other Rust projects
- [ ] Score differences documented and analyzed
- [ ] Decision made: switch to normalized OR keep legacy with clear documentation
- [ ] If switching: default changed to `calculate_cognitive_normalized()`
- [ ] If keeping: legacy function renamed to clarify it's intentional (e.g., `calculate_cognitive_visitor_based()`)
- [ ] Breaking changes documented in CHANGELOG.md
- [ ] All tests updated and passing

### Phase 4: Final Validation

- [ ] All phases complete with no test failures
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` passes
- [ ] `cargo fmt --all -- --check` passes
- [ ] `cargo test --all-features` passes
- [ ] `cargo doc --no-deps` builds without warnings
- [ ] Line count reduction verified (target: ~132 lines removed)
- [ ] Git commit created with clear summary of removals

## Technical Details

### Implementation Approach

**Phase 1**: Low-risk deletions of code with zero active usage.

**Phase 2**: Small refactoring to eliminate deprecated formatter functions. The refactoring is straightforward:

```rust
// Current (uses deprecated apply_formatted_sections)
pub fn format_priority_item(
    output: &mut String,
    rank: usize,
    item: &UnifiedDebtItem,
    has_coverage_data: bool,
) {
    let format_context = create_format_context(rank, item, has_coverage_data);
    let formatted_sections = generate_formatted_sections(&format_context);
    #[allow(deprecated)]
    apply_formatted_sections(output, formatted_sections);
}

// After (direct pure + writer pattern)
pub fn format_priority_item(
    output: &mut String,
    rank: usize,
    item: &UnifiedDebtItem,
    has_coverage_data: bool,
) {
    let formatted = pure::format_priority_item(
        rank,
        item,
        0, // default verbosity
        FormattingConfig::default(),
        has_coverage_data,
    );

    let mut buffer = Vec::new();
    let _ = writer::write_priority_item(&mut buffer, &formatted);
    if let Ok(result) = String::from_utf8(buffer) {
        output.push_str(&result);
    }
}
```

**Phase 3**: Data-driven decision making for cognitive complexity. Create comparison tests:

```rust
#[test]
fn compare_cognitive_complexity_on_real_code() {
    let test_cases = vec![
        ("debtmap/src/analyzers/rust_analyzer.rs", 500),
        ("debtmap/src/priority/unified_scorer.rs", 800),
        // ... more test cases
    ];

    for (file_path, max_acceptable_diff) in test_cases {
        let content = std::fs::read_to_string(file_path).unwrap();
        let file = syn::parse_file(&content).unwrap();

        let legacy_scores: Vec<u32> = extract_functions(&file)
            .map(|f| calculate_cognitive_legacy(&f.block))
            .collect();

        let normalized_scores: Vec<u32> = extract_functions(&file)
            .map(|f| calculate_cognitive_normalized(&f.block))
            .collect();

        // Analyze differences
        let diffs: Vec<i32> = legacy_scores.iter()
            .zip(&normalized_scores)
            .map(|(l, n)| *n as i32 - *l as i32)
            .collect();

        let max_diff = diffs.iter().map(|d| d.abs()).max().unwrap();
        assert!(
            max_diff <= max_acceptable_diff,
            "File {} has max diff {} (threshold {})",
            file_path, max_diff, max_acceptable_diff
        );
    }
}
```

### Architecture Changes

**Module Structure**:
- `src/organization/god_object/legacy_compat.rs` removed entirely
- `src/priority/formatter/sections.rs` simplified (remove deprecated functions)
- `src/priority/formatter/mod.rs` updated (remove legacy function, update wrapper)

**Function Call Graph**:
- Before: `format_priority_item()` → `generate_formatted_sections()` → `apply_formatted_sections()`
- After: `format_priority_item()` → `pure::format_priority_item()` → `writer::write_priority_item()`

### Data Structures

No new data structures required. Existing structures remain:
- `FormattedPriorityItem` (from pure module)
- `FormattingConfig`
- `UnifiedDebtItem`

### APIs and Interfaces

**Breaking Changes** (public API):

1. **Removed functions** (public but deprecated):
   - `organization::legacy_compat::group_methods_by_responsibility_with_domain_patterns()`
   - `organization::legacy_compat::calculate_domain_diversity_from_structs()`
   - `organization::legacy_compat::suggest_splits_by_struct_grouping()`
   - `priority::formatter::format_priority_item_legacy()`

2. **Changed default behavior** (if switching cognitive complexity):
   - `complexity::cognitive::calculate_cognitive()` may return different values
   - This affects all complexity scoring and prioritization

**Non-breaking Changes**:
- Internal formatter refactoring (pure + writer pattern already public)
- Legacy compatibility module removal (functions already panic)

## Dependencies

**Prerequisites**: None

**Affected Components**:
- `src/organization/god_object/` - Module exports updated
- `src/priority/formatter/` - Function implementations simplified
- `src/complexity/cognitive.rs` - Default implementation potentially changed
- Documentation files (ARCHITECTURE.md, REFACTORING_PLAN.md, book chapters)

**External Dependencies**: None (no new crates required)

## Testing Strategy

### Unit Tests

- [ ] Verify all existing unit tests pass after Phase 1 removals
- [ ] Verify all existing unit tests pass after Phase 2 refactoring
- [ ] Create new comparison tests for Phase 3 (cognitive complexity)
- [ ] Ensure no tests reference deleted functions

### Integration Tests

- [ ] Run full integration test suite after each phase
- [ ] Verify output formatting remains consistent (Phase 2)
- [ ] Verify scoring remains stable (Phase 3, unless intentionally changed)
- [ ] Test with example projects to catch unexpected issues

### Performance Tests

- [ ] Benchmark cognitive complexity calculation if switching to normalized
- [ ] Ensure formatter refactoring doesn't introduce performance regression
- [ ] Profile memory usage to verify no leaks from refactoring

### Manual Testing

- [ ] Run `debtmap analyze .` on debtmap itself
- [ ] Verify output quality and formatting
- [ ] Check for any error messages or warnings
- [ ] Test with various CLI flags and configurations

## Documentation Requirements

### Code Documentation

- [ ] Remove doc comments for deleted functions
- [ ] Update module-level docs in `src/organization/god_object/mod.rs`
- [ ] Update module-level docs in `src/priority/formatter/mod.rs`
- [ ] Add migration notes to CHANGELOG.md

### User Documentation

- [ ] Update ARCHITECTURE.md to remove references to:
  - `legacy_compat.rs` module
  - `format_priority_item_legacy()`
  - `apply_formatted_sections()`
- [ ] Update REFACTORING_PLAN.md to mark relevant sections as complete/archived
- [ ] Check book chapters for references to deleted code (search for function names)
- [ ] Add "Breaking Changes in 1.0" section to appropriate documentation

### Migration Guide

Create migration guide for external users (if any exist):

```markdown
## Breaking Changes in 1.0

### Removed Legacy God Object Analysis Functions

The following deprecated functions in `organization::legacy_compat` have been removed:
- `group_methods_by_responsibility_with_domain_patterns()`
- `calculate_domain_diversity_from_structs()`
- `suggest_splits_by_struct_grouping()`

**Migration**: Use the modular god_object API directly:
- For method grouping: `god_object::classifier` module
- For diversity calculation: `organization::domain_diversity` module
- For split suggestions: `god_object::recommender` module

### Removed Legacy Formatter Functions

The following deprecated formatter functions have been removed:
- `priority::formatter::format_priority_item_legacy()`
- Internal: `sections::apply_formatted_sections()`

**Migration**: Use the pure functional API:
```rust
use debtmap::priority::formatter::{pure, writer};
use debtmap::formatting::FormattingConfig;

let formatted = pure::format_priority_item(
    rank, item, 0, FormattingConfig::default(), has_coverage_data
);
let mut output = Vec::new();
writer::write_priority_item(&mut output, &formatted)?;
```

### Cognitive Complexity Calculation (if changed)

The default cognitive complexity calculation has been updated to use semantic normalization.
This may result in different complexity scores for some functions.

**Impact**: Prioritization and scoring may change slightly.
**Rollback**: Set `--use-legacy-cognitive` flag (if we add this option).
```

## Implementation Notes

### Removal Order

Execute phases in strict order to minimize risk:
1. **Phase 1**: Remove code with zero usage (safest)
2. **Phase 2**: Refactor formatter (low risk, single caller)
3. **Phase 3**: Cognitive complexity decision (highest risk, affects scoring)

### Gotchas and Considerations

1. **Legacy compat functions panic**: Even though they panic, they're marked `#[deprecated]` and may still be used by external code. Check download stats/usage if concerned.

2. **Formatter refactoring scope**: Only refactor `format_priority_item()`, don't touch other formatter code to minimize risk.

3. **Cognitive complexity implications**: Changing the default affects:
   - All complexity calculations
   - Scoring and prioritization
   - Test expectations
   - User expectations from previous versions

4. **Documentation sprawl**: Search broadly for references:
   ```bash
   rg "legacy_compat|format_priority_item_legacy|apply_formatted_sections" docs/ book/ README.md ARCHITECTURE.md
   ```

5. **Git commit granularity**: Consider separate commits per phase for easier rollback if issues arise.

### Best Practices

- Run full test suite after each phase
- Keep deprecated code in git history (don't squash commits excessively)
- Update CHANGELOG.md with each breaking change
- Consider adding deprecation warnings in 0.9.x if time allows (though marked version passed)

## Migration and Compatibility

### Breaking Changes

**For external users** (if debtmap is used as library):
1. Removed public deprecated functions (see APIs section)
2. Changed default cognitive complexity calculation (if Phase 3 switches)
3. Removed legacy formatter API

**For internal users** (debtmap CLI):
- No breaking changes expected (CLI interface unchanged)
- Output format remains stable
- Scoring may change if cognitive complexity default changes

### Compatibility Strategy

**Versioning**:
- This is appropriate for 1.0 release (major version bump)
- Mark as `v1.0.0-rc.1` initially for release candidate testing
- Full `v1.0.0` after validation period

**Deprecation Timeline**:
- Legacy code already marked deprecated in v0.8.0-v0.1.0
- Removal timeline already communicated (v0.9.0 target)
- No further deprecation period needed

**Rollback Plan**:
- If critical issues found, phases can be reverted independently
- Git history preserves all deleted code
- Can cherry-pick fixes without reverting entire cleanup

### Migration Validation

- [ ] Search GitHub/crates.io for external usage of debtmap as library
- [ ] Check if any projects import deprecated functions
- [ ] Create example migration code for documented cases
- [ ] Test migration examples compile and work correctly

## Success Metrics

**Code Metrics**:
- Lines of code removed: ~132 (target)
- Deprecated attributes removed: ~9
- Documentation files updated: ~4-6

**Quality Metrics**:
- Test pass rate: 100%
- Clippy warnings: 0
- Documentation build: Success

**Validation Metrics**:
- No performance regression (cognitive complexity ±5%)
- No output format regression (formatter byte-identical for same inputs)
- Git commit message quality: Clear, descriptive, follows conventions

## Timeline Estimate

- **Phase 1**: 30 minutes (delete, update, test)
- **Phase 2**: 30 minutes (refactor, test)
- **Phase 3**: 2-4 hours (analyze, decide, implement, test)
- **Phase 4**: 30 minutes (final validation, documentation)
- **Total**: 4-6 hours

## Risk Assessment

**Low Risk**:
- Phase 1 removals (no active usage)
- Phase 2 refactoring (single caller, well-defined replacement)

**Medium Risk**:
- Documentation updates (may miss references)
- Migration guide completeness

**High Risk**:
- Phase 3 cognitive complexity decision (affects scoring)

**Mitigation**:
- Comprehensive testing for Phase 3
- Release candidate period before 1.0
- Clear rollback plan
- Git history preservation

## Open Questions

1. **Cognitive complexity**: Should we switch to normalized as default?
   - Need data from comparison tests
   - Consider user impact
   - Evaluate performance implications

2. **External usage**: Are there any external users depending on deprecated functions?
   - Search crates.io reverse dependencies
   - Check GitHub for imports

3. **Deprecation warnings**: Should we add runtime warnings in 0.9.x before 1.0?
   - May help external users migrate
   - Adds complexity for short-term benefit

4. **Feature flags**: Should we keep legacy code behind feature flag?
   - `--features legacy-api` for compatibility
   - Increases maintenance burden
   - Defeats purpose of cleanup

## References

- ARCHITECTURE.md: Current architecture documentation
- REFACTORING_PLAN.md: God object refactoring plan (Phase 1 source)
- Spec 139: Pure Core, Imperative Shell architecture (Phase 2 rationale)
- Spec 181: God object modular refactoring (Phase 1 context)
- CLAUDE.md: Development guidelines and commit message standards
