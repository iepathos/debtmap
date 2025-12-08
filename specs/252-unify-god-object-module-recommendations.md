---
number: 252
title: Unify God Object and God Module Action Recommendations
category: optimization
priority: medium
status: draft
dependencies: [133]
created: 2025-12-07
---

# Specification 252: Unify God Object and God Module Action Recommendations

**Category**: optimization
**Priority**: medium
**Status**: draft
**Dependencies**: [133 - God Object Detection Refinement]

## Context

The current implementation maintains an artificial distinction in action recommendations between god objects (GodClass) and god modules (GodFile/GodModule):

```rust
match detection_type {
    GodClass => "Split god object into X modules by responsibility"
    GodFile | GodModule => "Split god module into X focused modules"
}
```

**Problems with this distinction:**

1. **Vague language**: "focused modules" is less actionable than "by responsibility"
2. **Artificial complexity**: Both detection types suffer from the same problem (too many responsibilities) and require the same solution (split by responsibility)
3. **Inconsistent guidance**: Users get different advice for fundamentally the same architectural issue
4. **Violates composition principles**: Creates branching complexity without adding semantic value

**The fundamental truth:**
- **Same problem**: Too many responsibilities in one place
- **Same solution**: Split by responsibility boundaries
- **Same metric**: `responsibility_count` drives the recommendation
- **Different only in diagnosis**: The detection type tells us *what structure* has the problem (class vs module), not *how to fix it*

This follows the Stillwater philosophy principle: "Composition Over Complexity - Build complex behavior from simple, composable pieces." The detection type distinction is valuable for **understanding** the problem but adds unnecessary complexity to the **recommendation**.

## Objective

Simplify god object/module recommendations by removing the artificial distinction in action text while preserving diagnostic information where it adds value.

## Requirements

### Functional Requirements

1. **Unified Action Recommendation**
   - Use single, clear action format: `"Split into X modules by responsibility"`
   - Remove the `match` statement on `detection_type` in action generation
   - Apply same action text to all detection types (GodClass, GodFile, GodModule)

2. **Preserve Diagnostic Context**
   - Keep `detection_type` in the data model (still valuable for analysis)
   - Include detection type in rationale/context if needed for clarity
   - Maintain different DebtType variants (GodObject vs GodModule) for categorization

3. **Consistent Split Count Logic**
   - Keep existing split count calculation (already unified):
     ```rust
     split_count = if recommended_splits.len() >= 2 {
         recommended_splits.len()
     } else {
         responsibility_count.clamp(2, 5)
     }
     ```
   - Apply uniformly to all detection types

4. **Clean Up Related Code**
   - Remove unnecessary detection type branching in recommendation formatting
   - Simplify code paths that differ only in string formatting
   - Update tests to reflect unified recommendation format

### Non-Functional Requirements

1. **Clarity**: Action text should be immediately actionable without jargon
2. **Consistency**: All god object/module issues show identical action format
3. **Simplicity**: Reduce code complexity by removing unnecessary branching
4. **Maintainability**: Easier to update recommendation text in future (single location)

## Acceptance Criteria

- [ ] Action recommendation uses unified format: `"Split into X modules by responsibility"`
- [ ] GodClass, GodFile, and GodModule all produce identical action text
- [ ] Split count calculation remains unchanged (uses responsibility_count.clamp(2, 5) fallback)
- [ ] Detection type is preserved in data structures (not removed, just not used in action text)
- [ ] All existing tests pass with updated action text
- [ ] No performance regression (simplified code should be faster)
- [ ] Documentation updated to reflect unified recommendation approach

## Technical Details

### Implementation Approach

**File to modify**: `src/builders/unified_analysis.rs`

**Current code** (lines ~1687-1698):
```rust
let primary_action = match god_analysis.detection_type {
    crate::organization::DetectionType::GodClass => {
        format!(
            "Split god object into {} modules by responsibility",
            split_count
        )
    }
    crate::organization::DetectionType::GodFile
    | crate::organization::DetectionType::GodModule => {
        format!("Split god module into {} focused modules", split_count)
    }
};
```

**Simplified code**:
```rust
let primary_action = format!(
    "Split into {} modules by responsibility",
    split_count
);
```

### Architecture Changes

**Before** (artificial branching):
```
detection_type ──┬──→ GodClass ──→ "Split god object into X modules by responsibility"
                 │
                 └──→ GodFile/Module ──→ "Split god module into X focused modules"
```

**After** (simple composition):
```
detection_type ──→ (preserved for diagnostics)

split_count ──→ "Split into X modules by responsibility"
```

### Code Cleanup Opportunities

1. **Primary simplification**: `src/builders/unified_analysis.rs:1687-1698`
   - Remove match statement
   - Use single format! call

2. **Potential related areas** (search for similar patterns):
   - Check if any output formatters have similar branching
   - Verify recommendation display code doesn't duplicate logic
   - Update any documentation that explains the distinction

3. **Test updates**:
   - Update test expectations for action text
   - Verify all god object/module tests use new format
   - Add regression test ensuring unified format is maintained

## Dependencies

- **Prerequisites**: [133 - God Object Detection Refinement]
  - Detection type classification logic is already correct
  - This spec only changes the recommendation text, not the detection

- **Affected Components**:
  - `src/builders/unified_analysis.rs` - Main implementation
  - `tests/god_object_*.rs` - Test expectations need updating
  - Any formatters that display action recommendations

- **External Dependencies**: None

## Testing Strategy

### Unit Tests

1. **Recommendation generation**:
   ```rust
   #[test]
   fn test_unified_action_recommendation() {
       let god_class = create_god_analysis(DetectionType::GodClass, 8, 34);
       let god_module = create_god_analysis(DetectionType::GodModule, 5, 58);

       let class_rec = create_god_object_recommendation(&god_class);
       let module_rec = create_god_object_recommendation(&god_module);

       // Both should have identical action format
       assert!(class_rec.primary_action.starts_with("Split into"));
       assert_eq!(
           class_rec.primary_action.contains("by responsibility"),
           module_rec.primary_action.contains("by responsibility")
       );
   }
   ```

2. **Split count consistency**:
   - Verify split count uses same calculation for all types
   - Test responsibility_count.clamp(2, 5) fallback
   - Test recommended_splits.len() when >= 2

### Integration Tests

1. **End-to-end analysis**:
   - Run debtmap on files with god objects
   - Run debtmap on files with god modules
   - Verify both show unified action format

2. **Regression tests**:
   - Test existing god object detection files
   - Verify output format changes as expected
   - Confirm detection types still correctly identified

### Manual Validation

1. Run on known god objects:
   ```bash
   cargo run -- analyze src/analysis/call_graph/mod.rs
   # Should show: "Split into 5 modules by responsibility"
   ```

2. Run on known god modules:
   ```bash
   cargo run -- analyze src/complexity/pure.rs
   # Should show: "Split into 5 modules by responsibility"
   ```

3. Compare outputs - actions should be identical format

## Documentation Requirements

### Code Documentation

- Update inline comments in `create_god_object_recommendation()` to explain unified approach
- Document the philosophical reasoning (detection type for diagnosis, unified action for prescription)
- Add docstring examples showing the unified format

### User Documentation

- Update any user-facing docs that explain god object vs module recommendations
- Clarify that detection types are diagnostic, actions are prescriptive
- Add examples showing unified recommendation format

### Architecture Updates

- Document the simplification in ARCHITECTURE.md if relevant
- Note adherence to "Composition Over Complexity" principle
- Explain the diagnostic vs prescriptive distinction

## Implementation Notes

### Why This Improves the Codebase

1. **Follows Stillwater philosophy**:
   - "Composition Over Complexity" - simpler composable piece
   - "Pragmatism Over Purity" - detection types useful for detection, not prescription
   - "Types Guide, Don't Restrict" - preserve type for diagnosis, don't restrict recommendation

2. **Better user experience**:
   - Clearer action: "by responsibility" is more specific than "focused modules"
   - Consistent guidance: same problem always gets same advice
   - Less cognitive load: users don't need to understand detection type distinction

3. **Code maintainability**:
   - Single source of truth for action text
   - Easier to update recommendation wording
   - Less branching complexity
   - Faster code (no match overhead)

### Design Rationale

The detection type (GodClass, GodFile, GodModule) serves a valuable diagnostic purpose:
- Helps understand *what structure* has too many responsibilities
- Guides initial investigation (look at impl blocks vs top-level functions)
- Informs complexity calculation (exclude tests for GodClass, include for GodFile)

However, for the **recommendation**, the detection type is irrelevant:
- All three suffer from too many responsibilities
- All three need the same solution: split by responsibility boundaries
- The metric driving the recommendation is `responsibility_count`, not detection type

Separating diagnosis from prescription simplifies the code while preserving important information.

### Alternative Approaches Considered

1. **Keep distinction but improve "focused modules" text**:
   - Still creates artificial complexity
   - Doesn't address fundamental issue (unnecessary branching)
   - Rejected in favor of unified approach

2. **Include detection type in action text**:
   - Example: "Split this god object into X modules by responsibility"
   - Adds noise without value
   - Users care about *what to do*, not *what to call it*
   - Rejected in favor of simple, clear action

3. **Move detection type to rationale instead**:
   - Could add context: "This god object (class-based) has 8 responsibilities..."
   - Adds complexity without clear benefit
   - Current rationale is already clear
   - Considered but not required for initial implementation

## Migration and Compatibility

### Breaking Changes

**User-facing**: Action text changes from:
- `"Split god object into X modules by responsibility"` → `"Split into X modules by responsibility"`
- `"Split god module into X focused modules"` → `"Split into X modules by responsibility"`

Impact: **Low**
- Text change only, no API or data format changes
- Improved clarity and consistency
- No tool integrations affected (JSON structure unchanged)

### Compatibility Preservation

1. **Data structures**: No changes to `DetectionType`, `GodObjectAnalysis`, or `ActionableRecommendation`
2. **JSON output**: Structure remains identical, only text field value changes
3. **Detection logic**: No changes to how god objects/modules are detected
4. **Test infrastructure**: Update test expectations, but test logic unchanged

### Migration Steps

None required - this is a purely presentational change with immediate effect after deployment.

## Success Metrics

1. **Code simplicity**: Reduction in lines of code in recommendation generation
2. **Consistency**: 100% of god object/module recommendations use unified format
3. **User clarity**: Recommendation text is immediately actionable
4. **Maintainability**: Single location to update recommendation wording

## Implementation Checklist

- [ ] Update `create_god_object_recommendation()` to use unified action format
- [ ] Remove detection type match statement
- [ ] Update all test expectations for action text
- [ ] Run full test suite and verify all pass
- [ ] Manual validation on known god objects and modules
- [ ] Update inline documentation
- [ ] Update user-facing documentation if needed
- [ ] Commit with descriptive message explaining simplification
- [ ] Delete this spec file after successful implementation
