---
number: 207
title: Unify God Object Type Representations
category: foundation
priority: high
status: draft
dependencies: [133]
created: 2025-12-06
---

# Specification 207: Unify God Object Type Representations

**Category**: foundation
**Priority**: high
**Status**: draft
**Dependencies**: Spec 133 (God Object Detection Refinement)

## Context

The codebase currently has two distinct types representing god object analysis results:

1. **`GodObjectIndicators`** (in `src/priority/file_metrics.rs`) - Used by file-level analysis and parallel execution path
2. **`GodObjectAnalysis`** (in `src/organization/god_object/core_types.rs`) - Used by sequential analysis path and god object detection

This duplication causes several issues:
- **Type confusion**: Similar data represented twice with different field names
- **Conversion complexity**: Parallel path requires lossy conversion from `GodObjectIndicators` → `GodObjectAnalysis`
- **Bug susceptibility**: God objects don't appear in TUI when using parallel analysis (default mode) because conversion is missing
- **Maintenance burden**: Changes must be synchronized across two types
- **Violation of DRY principle**: Single domain concept split across two representations

### Current Bug

God objects detected in parallel analysis are stored in `FileDebtMetrics.god_object_indicators` (type `GodObjectIndicators`) but are never converted to `UnifiedDebtItem` entries with `GodObjectAnalysis`. This means:
- God objects appear in `--no-tui` output (which reads `file_items`)
- God objects do NOT appear in TUI (which reads `analysis.items`)
- User-facing regression from spec 133 implementation

### Field Name Mismatches

| GodObjectIndicators | GodObjectAnalysis | Issue |
|---------------------|-------------------|-------|
| `methods_count` | `method_count` | Inconsistent naming |
| `fields_count` | `field_count` | Inconsistent naming |
| `ModuleSplit` (file_metrics) | `ModuleSplit` (split_types) | Different types, same name |

### Architecture Impact

**Sequential Path** (`src/builders/unified_analysis.rs`):
```rust
struct ProcessedFileData {
    god_analysis: Option<GodObjectAnalysis>,  // ✓ Direct usage
}

let god_item = create_god_object_debt_item(..., &god_analysis);
unified.add_item(god_item);  // ✓ Works correctly
```

**Parallel Path** (`src/builders/parallel_unified_analysis.rs`):
```rust
struct FileDebtMetrics {
    god_object_indicators: GodObjectIndicators,  // ✗ Different type
}

// Missing: conversion to GodObjectAnalysis and UnifiedDebtItem creation
// Result: God objects invisible in TUI
```

## Objective

Eliminate the `GodObjectIndicators` type and use `GodObjectAnalysis` as the single source of truth for god object detection results throughout the codebase. This ensures:

1. **Type safety**: Impossible to use wrong type
2. **Consistency**: Same data representation in sequential and parallel paths
3. **Correctness**: God objects visible in TUI for all execution modes
4. **Maintainability**: Single type to update and maintain
5. **Stillwater alignment**: Pure core without conversion logic

## Requirements

### Functional Requirements

- **FR1**: Replace `FileDebtMetrics.god_object_indicators: GodObjectIndicators` with `god_object_analysis: Option<GodObjectAnalysis>`
- **FR2**: Update all god object creation sites to produce `GodObjectAnalysis` directly
- **FR3**: Remove all conversion logic between `GodObjectIndicators` and `GodObjectAnalysis`
- **FR4**: Ensure god objects appear in TUI for both sequential and parallel execution paths
- **FR5**: Maintain backward compatibility for serialized analysis outputs (JSON)

### Non-Functional Requirements

- **NFR1**: Zero performance regression in parallel analysis path
- **NFR2**: All existing tests pass with updated types
- **NFR3**: No breaking changes to public API or CLI flags
- **NFR4**: Code reduction by eliminating conversion functions

## Acceptance Criteria

- [ ] `GodObjectIndicators` struct is deleted from `src/priority/file_metrics.rs`
- [ ] `FileDebtMetrics.god_object_indicators` replaced with `god_object_analysis: Option<GodObjectAnalysis>`
- [ ] All god object creation in `UnifiedFileAnalyzer` returns `GodObjectAnalysis`
- [ ] Parallel analysis path creates god object `UnifiedDebtItem` entries without conversion
- [ ] Sequential analysis path continues to work correctly (no regressions)
- [ ] God objects appear in TUI for both execution modes
- [ ] Unit test `test_god_objects_created_in_parallel_analysis` passes
- [ ] Unit test `test_god_objects_visible_in_tui` passes
- [ ] All existing god object tests pass without modification
- [ ] No compilation warnings related to god object types
- [ ] JSON serialization format preserves field names (or migration guide provided)

## Technical Details

### Implementation Approach

**Phase 1: Update FileDebtMetrics**

1. Change `src/priority/file_metrics.rs`:
```rust
// Before:
pub struct FileDebtMetrics {
    pub god_object_indicators: GodObjectIndicators,
    // ...
}

// After:
pub struct FileDebtMetrics {
    pub god_object_analysis: Option<crate::organization::GodObjectAnalysis>,
    // ...
}
```

2. Update `Default` implementation to use `None` instead of default `GodObjectIndicators`

**Phase 2: Update God Object Creation**

Update `src/analyzers/file_analyzer.rs` (UnifiedFileAnalyzer):
```rust
// Change method signatures and return types
impl UnifiedFileAnalyzer {
    pub fn analyze_file(&self, path: &Path, content: &str)
        -> Result<AnalyzedFile, String> {
        // ...
        let god_analysis = if self.should_analyze_god_objects() {
            Some(self.analyze_god_object(content, path)?)
        } else {
            None
        };

        Ok(AnalyzedFile {
            god_object_analysis: god_analysis,
            // ...
        })
    }
}
```

**Phase 3: Update Parallel Analysis**

Modify `src/builders/parallel_unified_analysis.rs`:
```rust
// Remove convert_indicators_to_analysis() function entirely

// In build() method:
for file_item in file_items {
    if let Some(god_analysis) = &file_item.metrics.god_object_analysis {
        let god_item = crate::builders::unified_analysis::create_god_object_debt_item(
            &file_item.metrics.path,
            &file_item.metrics,
            god_analysis,  // Direct usage, no conversion!
        );
        unified.add_item(god_item);
    }
    unified.add_file_item(file_item);
}
```

**Phase 4: Update All Usage Sites**

Find and update all code accessing `god_object_indicators`:
```bash
rg "god_object_indicators" --type rust
```

Update each location to use `god_object_analysis` and handle `Option<>` wrapper.

**Phase 5: Delete Old Type**

Remove `GodObjectIndicators` struct and related code from `src/priority/file_metrics.rs`.

### Architecture Changes

**Before** (Dual Type System):
```
God Object Detection
        ↓
  GodObjectAnalysis
        ↓
   ┌────┴────┐
   │         │
Sequential  Parallel
   │         │
   │    Convert to
   │    GodObjectIndicators
   │         │
   │    Convert back to
   │    GodObjectAnalysis
   │         │
   └────┬────┘
        ↓
  UnifiedDebtItem
```

**After** (Single Type):
```
God Object Detection
        ↓
  GodObjectAnalysis
        ↓
   ┌────┴────┐
   │         │
Sequential  Parallel
   │         │
   └────┬────┘
        ↓
  UnifiedDebtItem
```

### Data Structure Changes

**Field Mapping**:
```rust
// GodObjectIndicators fields → GodObjectAnalysis equivalents
methods_count      → method_count
fields_count       → field_count
responsibilities   → responsibility_count
is_god_object      → is_god_object
god_object_score   → god_object_score
responsibility_names → responsibilities
recommended_splits → recommended_splits (but different ModuleSplit type!)
module_structure   → module_structure
domain_count       → domain_count
domain_diversity   → domain_diversity
struct_ratio       → struct_ratio
analysis_method    → analysis_method
cross_domain_severity → cross_domain_severity
domain_diversity_metrics → domain_diversity_metrics
detection_type     → detection_type
```

**Type Compatibility Issues**:

The `recommended_splits` field references different `ModuleSplit` types:
- `file_metrics::ModuleSplit` (used by GodObjectIndicators)
- `split_types::ModuleSplit` (used by GodObjectAnalysis)

Resolution: Both types are already compatible (same fields, different paths). Update imports to use `organization::god_object::ModuleSplit` consistently.

### Migration Strategy

**Serialization Compatibility**:

If `FileDebtMetrics` is serialized to JSON and read by external tools:

1. Add serde alias to maintain compatibility:
```rust
pub struct FileDebtMetrics {
    #[serde(alias = "god_object_indicators")]
    pub god_object_analysis: Option<GodObjectAnalysis>,
}
```

2. Or provide migration guide for external consumers

## Dependencies

### Prerequisites
- **Spec 133**: God Object Detection Refinement - defines `GodObjectAnalysis` structure

### Affected Components
- `src/priority/file_metrics.rs` - FileDebtMetrics structure
- `src/builders/parallel_unified_analysis.rs` - Parallel analysis build
- `src/builders/unified_analysis.rs` - Sequential analysis (minimal changes)
- `src/analyzers/file_analyzer.rs` - God object creation
- `src/tui/results/app.rs` - TUI display (indirect, via analysis.items)

### External Dependencies
None - internal refactoring only

## Testing Strategy

### Unit Tests

**New Tests** (already written in `tests/parallel_unified_analysis_test.rs`):
```rust
#[test]
fn test_god_objects_created_in_parallel_analysis()
// Verifies god objects created as UnifiedDebtItems

#[test]
fn test_god_objects_visible_in_tui()
// Verifies god objects appear in TUI ResultsApp

#[test]
fn test_god_objects_not_created_when_disabled()
// Verifies no_god_object flag still works
```

**Existing Tests to Verify**:
- All god object detection tests in `tests/god_object_*.rs`
- Parallel vs sequential consistency tests
- File metrics serialization tests

### Integration Tests

1. **End-to-end TUI test**:
   - Run debtmap on codebase with god objects
   - Verify god objects appear in TUI list
   - Verify god object detail view shows correct data

2. **Parallel analysis regression test**:
   - Run with `DEBTMAP_PARALLEL=true`
   - Verify god objects in `analysis.items`
   - Compare with sequential results

3. **JSON output compatibility**:
   - Generate JSON output before/after change
   - Verify structure compatibility (or document breaking changes)

### Performance Tests

Run benchmarks to ensure no regression:
```bash
cargo bench --bench parallel_performance
```

Expected: <1% difference in parallel analysis time

## Documentation Requirements

### Code Documentation

1. Update `FileDebtMetrics` rustdoc to explain `god_object_analysis` field
2. Add migration comment in `parallel_unified_analysis.rs` explaining removal of conversion
3. Document why `GodObjectAnalysis` is now canonical type

### User Documentation

No user-facing changes - internal refactoring only.

### Architecture Updates

Update `ARCHITECTURE.md`:
- Remove mention of `GodObjectIndicators` type
- Document `GodObjectAnalysis` as single source of truth
- Explain god object creation flow in parallel vs sequential paths

## Implementation Notes

### Critical Path

1. **Must update atomically**: Cannot have half the codebase using old type, half using new
2. **Test coverage is critical**: Must catch any missed conversion sites
3. **Serde compatibility**: Check if JSON field renaming breaks anything

### Gotchas

- **Field name changes**: `methods_count` → `method_count` (plural vs singular)
- **Option wrapper**: New field is `Option<GodObjectAnalysis>`, old was non-optional `GodObjectIndicators`
- **Import paths**: `use crate::organization::GodObjectAnalysis` needed in file_metrics.rs

### Best Practices

- Search for ALL usages before deleting old type
- Run full test suite after each phase
- Use compiler errors to guide refactoring
- Add `#[deprecated]` to old type first, then remove in separate commit

### Stillwater Alignment

This refactoring follows Stillwater philosophy:

✅ **Pure Core**: Eliminates conversion logic (impure transformation)
✅ **Composition**: Removes unnecessary abstraction layer
✅ **Types Guide**: One type prevents misuse
✅ **Pragmatic**: Fixes real bug, improves maintainability

From PHILOSOPHY.md:
> *"Types Guide, Don't Restrict: Use types to make wrong code hard to write"*

Having one canonical type makes it impossible to use the wrong representation.

## Migration and Compatibility

### Breaking Changes

**Internal API**:
- `FileDebtMetrics.god_object_indicators` → `FileDebtMetrics.god_object_analysis`
- Type changes from `GodObjectIndicators` to `Option<GodObjectAnalysis>`
- Field names change (pluralization)

**Public API**: None - CLI and TUI behavior unchanged

### Serialization

**JSON Output**:

If `debtmap --output json` is used by external tools:

**Option A** (Backward compatible):
```rust
#[serde(alias = "god_object_indicators", rename = "god_object_analysis")]
pub god_object_analysis: Option<GodObjectAnalysis>,
```
Reads both old and new field names, writes new name.

**Option B** (Breaking):
Document field rename in changelog, increment major version.

**Recommendation**: Use Option A for smooth transition.

### Rollback Plan

If issues arise:
1. Revert single commit (atomic change)
2. Known test failures indicate missed conversion sites
3. No data loss (change is structural, not data)

### Timeline

- Phase 1-2: 2 hours (update structures and creation)
- Phase 3-4: 2 hours (update all usage sites)
- Phase 5: 30 minutes (delete old type)
- Testing: 1 hour (run full test suite, manual verification)

**Total**: ~6 hours for complete implementation

## Success Metrics

### Correctness
- [ ] God objects visible in TUI for all execution modes
- [ ] All unit tests pass
- [ ] No type conversion errors

### Code Quality
- [ ] Lines of code reduced by removing conversion logic
- [ ] No duplicate type definitions
- [ ] Consistent naming throughout codebase

### Performance
- [ ] Parallel analysis performance unchanged (<1% variance)
- [ ] No additional allocations introduced

### Maintainability
- [ ] Single source of truth for god object data
- [ ] Clear type boundaries
- [ ] Simplified parallel analysis code

## Related Specifications

- **Spec 133**: God Object Detection Refinement - defines `GodObjectAnalysis`
- **Spec 182**: (referenced in conversation) - unrelated to this issue
- **Spec 207**: Current specification - fixes god object TUI visibility bug
