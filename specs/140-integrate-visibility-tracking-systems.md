---
number: 140
title: Integrate Visibility Tracking Systems
category: optimization
priority: high
status: draft
dependencies: [134]
created: 2025-10-27
---

# Specification 140: Integrate Visibility Tracking Systems

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: Spec 134 (God Object Metric Contradictions - completed)

## Context

Spec 134 successfully implemented accurate visibility tracking for god object detection through `GodObjectAnalysis.visibility_breakdown`. However, the terminal output still shows contradictions like "37 total (0 public, 0 private)" because the output formatter uses a separate, older visibility tracking system (`ModuleStructure.function_counts`).

### Current State

**Two Parallel Systems**:
1. **New System** (Spec 134): `GodObjectAnalysis.visibility_breakdown`
   - Accurate tracking via AST traversal
   - Handles pub, pub(crate), pub(super), private
   - Tracks both standalone functions and impl methods
   - Includes validation layer

2. **Old System**: `ModuleStructure.function_counts`
   - Used by output formatter (src/priority/formatter.rs)
   - Separate data pipeline
   - No connection to GodObjectAnalysis
   - May have inaccurate counts

### Problem

Terminal output shows:
```
- FUNCTIONS: 37 total (0 public, 0 private)
```

While internally:
```rust
GodObjectAnalysis {
    visibility_breakdown: Some(FunctionVisibilityBreakdown {
        public: 15,
        pub_crate: 5,
        pub_super: 2,
        private: 15,  // total = 37
    })
}
```

The formatter displays wrong data because it reads from `ModuleStructure.function_counts` instead of `GodObjectAnalysis.visibility_breakdown`.

## Objective

Integrate the two visibility tracking systems so that:
1. Terminal output displays accurate visibility counts
2. Single source of truth for visibility metrics
3. No contradictions between internal data and displayed output
4. Backwards compatibility maintained

## Requirements

### Functional Requirements

1. **Data Flow Integration**
   - `ModuleStructure` should use `GodObjectAnalysis.visibility_breakdown` as source
   - Populate `ModuleStructure.function_counts` from visibility breakdown
   - Ensure data flows from AST analysis → GodObjectAnalysis → ModuleStructure → Formatter

2. **Backwards Compatibility**
   - Maintain `ModuleStructure.function_counts` API
   - Support non-Rust languages that don't have visibility_breakdown
   - Graceful fallback when visibility_breakdown is None

3. **Output Accuracy**
   - Terminal output shows correct public/private counts
   - Handles all visibility levels (pub, pub(crate), pub(super), private)
   - Counts match validation expectations

### Non-Functional Requirements

- **Performance**: No significant overhead from integration
- **Maintainability**: Clear data flow, single source of truth
- **Testability**: End-to-end tests verify output accuracy

## Acceptance Criteria

- [ ] `ModuleStructure` populates `function_counts` from `GodObjectAnalysis.visibility_breakdown`
- [ ] Terminal output shows accurate visibility counts for god objects
- [ ] Output matches `visibility_breakdown` data (no contradictions)
- [ ] Non-Rust files or files without visibility_breakdown still work
- [ ] All existing tests pass
- [ ] New integration test verifies output accuracy
- [ ] No performance regression

## Technical Details

### Implementation Approach

**Phase 1: Data Bridge**

Update where `ModuleStructure` is populated with god object data:

```rust
// In src/organization/god_object_detector.rs or relevant location
fn create_module_structure(analysis: &GodObjectAnalysis) -> ModuleStructure {
    let function_counts = if let Some(ref breakdown) = analysis.visibility_breakdown {
        // Use new accurate data
        FunctionCounts {
            public_functions: breakdown.public,
            private_functions: breakdown.private + breakdown.pub_crate + breakdown.pub_super,
            total_functions: breakdown.total(),
        }
    } else {
        // Fallback for non-Rust or legacy paths
        FunctionCounts {
            public_functions: 0,
            private_functions: 0,
            total_functions: analysis.method_count,
        }
    };

    ModuleStructure {
        function_counts,
        // ... other fields
    }
}
```

**Phase 2: Enhanced Output** (Optional Enhancement)

Consider adding pub(crate) and pub(super) to output:

```rust
// Current: "37 total (15 public, 22 private)"
// Enhanced: "37 total (15 pub, 5 pub(crate), 2 pub(super), 15 private)"
```

### Architecture Changes

**Before (Current)**:
```
AST Traversal
    ↓
GodObjectAnalysis.visibility_breakdown (accurate)
    ↓ [NOT CONNECTED]

Separate Pipeline
    ↓
ModuleStructure.function_counts (inaccurate)
    ↓
Formatter Output (shows wrong data)
```

**After (Integrated)**:
```
AST Traversal
    ↓
GodObjectAnalysis.visibility_breakdown (accurate)
    ↓ [CONNECTED]
ModuleStructure.function_counts (sourced from breakdown)
    ↓
Formatter Output (shows correct data)
```

### Data Structures

**Existing `FunctionCounts`** (src/analysis/module_structure.rs):
```rust
pub struct FunctionCounts {
    pub public_functions: usize,
    pub private_functions: usize,
    pub total_functions: usize,
}
```

**Option 1: Keep as-is**, map visibility_breakdown:
- `public_functions` = `breakdown.public`
- `private_functions` = `breakdown.private + pub_crate + pub_super`
- `total_functions` = `breakdown.total()`

**Option 2: Enhance**, add granular fields:
```rust
pub struct FunctionCounts {
    pub public_functions: usize,
    pub pub_crate_functions: usize,
    pub pub_super_functions: usize,
    pub private_functions: usize,
    pub total_functions: usize,
}
```

**Recommendation**: Start with Option 1 for minimal changes, consider Option 2 for future enhancement.

### Integration Points

1. **God Object Detector** → **Module Structure**
   - Location: Where `GodObjectAnalysis` is converted to `ModuleStructure`
   - Files: Likely `src/organization/god_object_detector.rs` or `src/analysis/module_structure.rs`

2. **Module Structure** → **Formatter**
   - Location: `src/priority/formatter.rs`
   - Already reads `module_structure.function_counts`
   - No changes needed if we populate correctly upstream

### Files to Modify

1. **src/organization/god_object_detector.rs**
   - Add function to populate `FunctionCounts` from `visibility_breakdown`
   - Update where `module_structure` field is set in `GodObjectAnalysis`

2. **src/analysis/module_structure.rs** (if needed)
   - Update `ModuleStructure` creation logic
   - Add helper function to convert visibility_breakdown

3. **Tests**
   - Add integration test: AST → Analysis → Structure → Output
   - Verify output matches internal data

## Dependencies

- **Prerequisites**: Spec 134 (completed - visibility tracking implemented)
- **Affected Components**:
  - `GodObjectAnalysis` (read visibility_breakdown)
  - `ModuleStructure` (populate function_counts)
  - Output formatter (no changes, just reads correct data)

## Testing Strategy

### Unit Tests

```rust
#[test]
fn test_function_counts_from_visibility_breakdown() {
    let breakdown = FunctionVisibilityBreakdown {
        public: 10,
        pub_crate: 3,
        pub_super: 2,
        private: 12,
    };

    let counts = FunctionCounts::from_visibility_breakdown(&breakdown);

    assert_eq!(counts.public_functions, 10);
    assert_eq!(counts.private_functions, 17); // 3 + 2 + 12
    assert_eq!(counts.total_functions, 27);
}
```

### Integration Tests

```rust
#[test]
fn test_output_matches_internal_metrics() {
    let code = r#"
        pub struct GodClass {}
        impl GodClass {
            pub fn m1(&self) {}
            pub fn m2(&self) {}
            fn m3(&self) {}
            pub(crate) fn m4(&self) {}
        }
    "#;

    let analysis = analyze_code(code);
    let module_structure = extract_module_structure(&analysis);

    // Verify visibility_breakdown
    let breakdown = analysis.visibility_breakdown.unwrap();
    assert_eq!(breakdown.public, 2);
    assert_eq!(breakdown.pub_crate, 1);
    assert_eq!(breakdown.private, 1);

    // Verify function_counts matches
    assert_eq!(module_structure.function_counts.public_functions, 2);
    assert_eq!(module_structure.function_counts.private_functions, 2); // crate + private
    assert_eq!(module_structure.function_counts.total_functions, 4);
}
```

### Output Validation

Run debtmap on actual codebase and verify:
- No "0 public, 0 private" contradictions
- Totals match between description and breakdown
- Output is consistent with internal metrics

## Documentation Requirements

### Code Documentation

- Document the data flow from visibility_breakdown to function_counts
- Add inline comments explaining the integration
- Update module-level documentation

### Architecture Updates

Update ARCHITECTURE.md:
- Document unified visibility tracking system
- Explain data flow: AST → Analysis → Structure → Output
- Note that Spec 140 completed integration

### User-Facing Documentation

No user-facing documentation changes needed (internal improvement).

## Implementation Notes

### Phased Approach

**Phase 1: Minimal Integration** (This spec)
- Connect existing systems
- Use visibility_breakdown as source
- Map to existing FunctionCounts structure
- Verify output accuracy

**Phase 2: Enhanced Granularity** (Future)
- Add pub(crate) and pub(super) to output
- Enhance FunctionCounts with more fields
- Provide detailed visibility breakdown in output

### Gotchas

1. **Non-Rust Files**: visibility_breakdown is None for non-Rust files
   - Need fallback logic
   - Use method_count as total, leave pub/private as 0 or estimate

2. **Legacy Code Paths**: Some analyses may not use GodObjectAnalysis
   - Ensure backwards compatibility
   - Graceful degradation if visibility_breakdown unavailable

3. **Test Filtering**: GodClass counts production methods only (tests excluded)
   - Ensure function_counts matches this filtered count
   - Don't accidentally include test methods

### Performance Considerations

- Data conversion is O(1) (just field mapping)
- No additional AST traversal needed
- Negligible performance impact

## Migration and Compatibility

### Breaking Changes

None. This is an internal improvement.

### Backwards Compatibility

- Old code paths without visibility_breakdown still work
- FunctionCounts API unchanged
- Output format unchanged (just more accurate data)

### Rollback Plan

If issues arise:
1. Keep visibility_breakdown population (Phase 1-3 of Spec 134)
2. Revert function_counts population to old logic
3. Output will show old (possibly inaccurate) data but won't break

## Success Metrics

- Zero contradiction reports in output (0 public, 0 private when functions exist)
- Output validation passes for all test files
- User-reported accuracy issues decrease
- Internal metrics match displayed metrics

## Related Work

- **Spec 134**: Implemented accurate visibility tracking (Phase 1-3 completed)
- **Future**: Consider enhancing output to show all visibility levels separately
