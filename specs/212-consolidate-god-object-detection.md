---
number: 212
title: Consolidate God Object Detection to Extraction Adapter
category: architecture
priority: high
status: draft
dependencies: [206]
created: 2025-12-15
---

# Specification 212: Consolidate God Object Detection to Extraction Adapter

**Category**: architecture
**Priority**: high (P0)
**Status**: draft
**Dependencies**: Spec 206 (Cohesion Gate)

## Context

The god object detection system has accumulated multiple code paths doing similar work:

### Current Code Paths (Problem)

1. **OLD PATH** (`src/organization/god_object/detector.rs`)
   - AST-based using `syn::File`
   - `GodObjectDetector::analyze_enhanced()` → `analyze_comprehensive()` → `analyze_single_struct()`
   - 1075 lines, complex orchestration

2. **NEW PATH** (`src/extraction/adapters/god_object.rs`)
   - Pure functions on `ExtractedFileData`
   - `analyze_god_objects()` → `is_god_object_candidate()` → `build_god_object_analysis()`
   - 984 lines, O(n) performance, no I/O

3. **INTEGRATION LAYER** (`src/analyzers/file_analyzer.rs`)
   - Tries extraction adapter first, falls back to AST-based
   - Duplicate heuristic logic

4. **PARALLEL BUILDER** (`src/builders/parallel_unified_analysis.rs`)
   - Uses extraction adapter when data available
   - Has its own heuristic fallback (~100 lines duplicated)

### Issues Caused by Duplication

1. **Spec 206 Bug**: Cohesion gate was added to `detector.rs` but not `adapters/god_object.rs`, causing CrossModuleTracker to still be flagged via the adapter path
2. **Scoring Inconsistency**: `detector.rs` uses weighted complexity scoring; adapter uses simpler per-component scoring
3. **Maintenance Burden**: Changes must be applied to multiple locations
4. **Test Coverage Gaps**: Easy to miss a code path

## Objective

Consolidate god object detection to a **single source of truth** using the extraction adapter pattern:

1. Make `extraction::adapters::god_object` the canonical implementation
2. Deprecate and eventually remove `GodObjectDetector` AST-based analysis
3. Unify scoring algorithms
4. Extract shared heuristic logic

## Requirements

### Functional Requirements

1. **Single Detection Path**: All god object detection flows through `extraction::adapters::god_object`
2. **Unified Scoring**: One scoring algorithm used everywhere
3. **Shared Heuristics**: Extract `fallback_god_object_heuristics()` function
4. **Deprecation Path**: Mark old APIs as deprecated with migration guide

### Non-Functional Requirements

- No behavior changes for existing users (same results)
- Performance improvement (no redundant parsing)
- Reduced code surface (~500 lines removable)

## Acceptance Criteria

- [ ] `GodObjectDetector::analyze_enhanced()` marked `#[deprecated]`
- [ ] All callers migrated to extraction adapter
- [ ] Scoring algorithm unified (detector's weighted approach in adapter)
- [ ] Heuristic fallback extracted to shared function
- [ ] Test coverage for single path (no parallel paths to test)
- [ ] Cohesion gate applied in exactly ONE place

## Technical Details

### Phase 1: Align Scoring Algorithm

Port the weighted scoring from `detector.rs:451-469` to `adapters/god_object.rs`:

```rust
// Current adapter scoring (simple)
fn calculate_god_object_score(...) -> f64 {
    let method_score = (method_count / max_methods * 40.0).min(40.0);
    let field_score = (field_count / max_fields * 20.0).min(20.0);
    let resp_score = (resp_count / max_traits * 40.0).min(40.0);
    method_score + field_score + resp_score
}

// Detector scoring (weighted - more accurate)
// Port this to adapter
let method_factor = (method_count / max_methods).min(3.0);
let field_factor = (field_count / max_fields).min(3.0);
let responsibility_factor = (responsibility_count / 3.0).min(3.0);
let size_factor = (loc / max_lines).min(3.0);
let complexity_factor = (avg_complexity / 10.0).min(3.0);

let base_score = method_factor * field_factor * responsibility_factor
               * size_factor * complexity_factor;
```

### Phase 2: Extract Shared Heuristics

Create `src/organization/god_object/heuristics.rs`:

```rust
/// Pure heuristic god object detection for simple/fallback cases.
pub fn fallback_god_object_heuristics(
    function_count: usize,
    line_count: usize,
    field_count: usize,
) -> Option<GodObjectAnalysis> {
    const MAX_FUNCTIONS: usize = 50;
    const MAX_LINES: usize = 2000;
    const MAX_FIELDS: usize = 30;

    let is_god_object = function_count > MAX_FUNCTIONS
        || line_count > MAX_LINES
        || field_count > MAX_FIELDS;

    if !is_god_object {
        return None;
    }

    // Build minimal analysis...
}
```

Remove duplicate implementations from:
- `file_analyzer.rs:79-99` (analyze_god_object_simple)
- `parallel_unified_analysis.rs:1020-1120` (heuristic fallback)

### Phase 3: Remove Old Path

Delete the duplicate analysis methods from `detector.rs`:
- `analyze_enhanced()` - Remove, use adapter instead
- `analyze_comprehensive()` - Remove, use adapter instead
- `analyze_single_struct()` - Remove, use adapter instead
- `analyze_file_level()` - Remove, use adapter instead

Keep only:
- `GodObjectDetector` struct for threshold configuration
- Helper functions that are reused (if any)
- Types and data structures

### Phase 4: Update Callers

**file_analyzer.rs**:
```rust
// Before: Complex fallback chain
if path.extension() == Some("rs") {
    if let Ok(extracted) = UnifiedFileExtractor::extract(path, content) {
        return self.analyze_god_object_from_extracted(path, content, &extracted);
    }
}
self.analyze_god_object_simple(content)

// After: Single path
let extracted = UnifiedFileExtractor::extract(path, content)?;
adapters::god_object::analyze_god_object(path, &extracted)
    .or_else(|| heuristics::fallback_god_object_heuristics(...))
```

### Migration Table

| Old API | New API | Status |
|---------|---------|--------|
| `GodObjectDetector::analyze_enhanced()` | `adapters::god_object::analyze_god_objects()` | Deprecate |
| `GodObjectDetector::analyze_comprehensive()` | `adapters::god_object::analyze_god_objects()` | Deprecate |
| `GodObjectDetector::analyze_single_struct()` | Internal to adapter | Remove |
| `UnifiedFileAnalyzer::analyze_god_object_simple()` | `heuristics::fallback_god_object_heuristics()` | Extract |

## Dependencies

- **Prerequisites**: Spec 206 (Cohesion Gate) - already implemented
- **Affected Components**:
  - `src/organization/god_object/detector.rs` - Deprecate
  - `src/extraction/adapters/god_object.rs` - Enhance
  - `src/analyzers/file_analyzer.rs` - Simplify
  - `src/builders/parallel_unified_analysis.rs` - Remove duplicates

## Testing Strategy

1. **Integration Tests**: Ensure same results before/after consolidation
2. **Golden File Tests**: Capture current output, verify unchanged
3. **Performance Tests**: Confirm no redundant parsing

## Implementation Notes

1. **Gradual Migration**: Keep old path working during transition
2. **Feature Flag**: Add `--legacy-god-object` for fallback
3. **Logging**: Add debug logs when deprecated path is used

## Estimated Effort

- Phase 1 (Align Scoring): 2 hours
- Phase 2 (Extract Heuristics): 2 hours
- Phase 3 (Deprecate Old Path): 1 hour
- Phase 4 (Update Callers): 3 hours
- Testing: 2 hours
- **Total**: ~10 hours

## Benefits After Consolidation

1. **Single Source of Truth**: One place for cohesion gate, scoring, etc.
2. **Easier Maintenance**: Changes apply once
3. **Better Performance**: No redundant parsing
4. **Clearer Architecture**: Extraction → Adapters → Analysis
5. **Reduced Code**: ~500 lines removable
