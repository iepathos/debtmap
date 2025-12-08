---
number: 255
title: God Object Accumulated Complexity Display
category: foundation
priority: low
status: draft
dependencies: [253]
created: 2025-12-07
---

# Specification 255: God Object Accumulated Complexity Display

**Category**: foundation
**Priority**: low
**Status**: draft
**Dependencies**: Spec 253 (unified god object types)

## Context

The TUI detail view overview page displays complexity metrics for all debt items. For regular functions, these metrics (cyclomatic, cognitive, nesting) represent the complexity of that single function.

For god objects (structs/classes/modules with many functions), the current display shows the **same fields** but the values have a **different semantic meaning** - they represent accumulated/aggregated complexity across all functions in the god object.

### Current Behavior

**File**: `src/tui/results/detail_pages/overview.rs:160-183`

```rust
// Complexity metrics section
add_section_header(&mut lines, "complexity", theme);
add_label_value(&mut lines, "cyclomatic", item.cyclomatic_complexity.to_string(), ...);
add_label_value(&mut lines, "cognitive", item.cognitive_complexity.to_string(), ...);
add_label_value(&mut lines, "nesting", item.nesting_depth.to_string(), ...);
```

For a **regular function**:
- `cyclomatic_complexity = 15` → "This function has cyclomatic complexity of 15"
- `cognitive_complexity = 20` → "This function has cognitive complexity of 20"
- `nesting_depth = 4` → "This function has max nesting depth of 4"

For a **god object**:
- `cyclomatic_complexity = 150` → "Sum of cyclomatic complexity across all 30 methods"
- `cognitive_complexity = 200` → "Sum of cognitive complexity across all 30 methods"
- `nesting_depth = 6` → "Maximum nesting depth among all 30 methods"

The label "complexity" is **ambiguous** - it doesn't indicate whether we're looking at a single function or accumulated metrics.

### Data Source

God object complexity values come from `GodObjectAnalysis`:

**File**: `src/organization/god_object/core_types.rs:69`
```rust
pub struct GodObjectAnalysis {
    // ...
    pub complexity_sum: u32,  // ← Aggregated cyclomatic complexity
    // ...
}
```

**File**: `src/organization/god_object/detector.rs:306-311`
```rust
let complexity_sum: u32 = visitor
    .function_complexity
    .iter()
    .filter(|fc| method_names.contains(&fc.name))
    .map(|fc| fc.cyclomatic_complexity)
    .sum();  // ← Summing across all methods
```

The complexity is **intentionally aggregated** - this is correct behavior. The issue is purely a **display/labeling problem**.

### Problem Statement

Users viewing a god object in the TUI see:
```
complexity
  cyclomatic            150
  cognitive             200
  nesting               6
```

Without context, this looks like:
- ❌ "This struct/module has impossibly high complexity!"
- ❌ "How can cognitive complexity be 200?"

When it should communicate:
- ✅ "All 30 functions combined have cyclomatic complexity of 150"
- ✅ "Total cognitive complexity across all functions is 200"
- ✅ "Maximum nesting depth found in any function is 6"

## Objective

Update the TUI overview page to use **"accumulated complexity"** as the section header for god objects, making it immediately clear that the metrics represent **aggregated values across all functions** rather than a single function's complexity.

For regular functions, keep the existing "complexity" header to maintain clarity that metrics apply to a single function.

## Requirements

### Functional Requirements

1. **Conditional Section Header**
   - God objects: Display section header as **"accumulated complexity"**
   - Regular functions: Display section header as **"complexity"** (existing)
   - Use `matches!(item.debt_type, DebtType::GodObject { .. })` to determine type

2. **Metric Labels Remain Unchanged**
   - "cyclomatic", "cognitive", "nesting" labels stay the same
   - Values stay the same (already correct)
   - Only the section header changes

3. **Semantic Clarity**
   - "accumulated complexity" conveys: "sum/aggregate of complexity metrics"
   - "complexity" conveys: "this single function's complexity metrics"
   - Users should immediately understand the scope of the metrics

### Non-Functional Requirements

1. **Backwards Compatibility**
   - No data structure changes
   - No changes to complexity calculation logic
   - Pure display/labeling change

2. **Consistency**
   - Follow existing TUI patterns for section headers
   - Use `add_section_header()` helper (already in use)
   - Match theme and styling of other sections

3. **User Experience**
   - Reduce confusion about god object complexity values
   - Immediate clarity without needing to read documentation
   - Consistent with "god object structure" header pattern (spec 253)

## Acceptance Criteria

- [ ] **Conditional Logic**: TUI checks `DebtType::GodObject` vs other types
- [ ] **God Object Header**: Shows "accumulated complexity" for god objects
- [ ] **Regular Function Header**: Shows "complexity" for non-god objects
- [ ] **Metric Labels**: "cyclomatic", "cognitive", "nesting" unchanged
- [ ] **Values**: Complexity values unchanged (still aggregated for god objects)
- [ ] **Visual Consistency**: Follows existing theme and section header styling
- [ ] **Manual Testing**: Verified with actual god object and regular function items
- [ ] **Screenshot Comparison**: Before/after screenshots show clear improvement
- [ ] **No Regressions**: Other pages (dependencies, action) unaffected

## Technical Details

### Implementation Approach

**File**: `src/tui/results/detail_pages/overview.rs:160-183`

#### Before (Current)
```rust
// Complexity metrics section
add_section_header(&mut lines, "complexity", theme);
add_label_value(&mut lines, "cyclomatic", item.cyclomatic_complexity.to_string(), ...);
add_label_value(&mut lines, "cognitive", item.cognitive_complexity.to_string(), ...);
add_label_value(&mut lines, "nesting", item.nesting_depth.to_string(), ...);
```

#### After (This Spec)
```rust
// Complexity metrics section
// For god objects, use "accumulated complexity" to indicate aggregated metrics
let complexity_header = if matches!(item.debt_type, DebtType::GodObject { .. }) {
    "accumulated complexity"
} else {
    "complexity"
};
add_section_header(&mut lines, complexity_header, theme);

add_label_value(&mut lines, "cyclomatic", item.cyclomatic_complexity.to_string(), ...);
add_label_value(&mut lines, "cognitive", item.cognitive_complexity.to_string(), ...);
add_label_value(&mut lines, "nesting", item.nesting_depth.to_string(), ...);
```

### Semantic Clarification

#### God Object Metrics Meaning

For a god class with 30 methods:

| Metric | Current Display | Meaning | After Change |
|--------|----------------|---------|--------------|
| cyclomatic | 150 | Sum of all 30 functions' cyclomatic complexity | Same value, clearer context |
| cognitive | 200 | Sum of all 30 functions' cognitive complexity | Same value, clearer context |
| nesting | 6 | **Maximum** nesting depth across all 30 functions | Same value, clearer context |

**Note**: Nesting is max (not sum) because it represents the deepest nesting found in any function.

#### Regular Function Metrics Meaning

For a single complex function:

| Metric | Current Display | Meaning | After Change |
|--------|----------------|---------|--------------|
| cyclomatic | 15 | This function's cyclomatic complexity | Same (no change) |
| cognitive | 20 | This function's cognitive complexity | Same (no change) |
| nesting | 4 | This function's maximum nesting depth | Same (no change) |

### Architecture Changes

No architecture changes - this is a pure display enhancement.

**Data Flow** (unchanged):
```
God Object Analysis
    ↓
GodObjectAnalysis.complexity_sum (aggregated)
    ↓
UnifiedDebtItem.cyclomatic_complexity
    ↓
TUI displays value (NEW: with clarifying header)
```

### Alternative Designs Considered

#### Option 1: Change Metric Labels ❌
```rust
add_label_value(&mut lines, "total cyclomatic", ...);
add_label_value(&mut lines, "total cognitive", ...);
add_label_value(&mut lines, "max nesting", ...);
```

**Rejected**: Breaks consistency with regular functions, harder to scan

#### Option 2: Add Explanation Text ❌
```rust
lines.push(Line::from("  (aggregated across all functions)"));
```

**Rejected**: Too verbose, clutters display

#### Option 3: Different Section Name ❌
```rust
add_section_header(&mut lines, "total complexity", theme);
```

**Rejected**: "accumulated" is more accurate (nesting is max, not total)

#### Option 4: This Spec's Approach ✅
```rust
let header = if matches!(item.debt_type, DebtType::GodObject { .. }) {
    "accumulated complexity"
} else {
    "complexity"
};
```

**Accepted**: Minimal change, maximum clarity, consistent with existing patterns

## Dependencies

### Prerequisites
- **Spec 253**: Unified god object types (makes `DebtType::GodObject` check simpler)

### Affected Components
- `src/tui/results/detail_pages/overview.rs` - Update section header logic

### External Dependencies
None

## Testing Strategy

### Manual Testing

1. **Regular Function Display**
   ```
   Test: Open TUI for a complex regular function
   Expected:
     complexity
       cyclomatic            15
       cognitive             20
       nesting               4
   ```

2. **God Object Display**
   ```
   Test: Open TUI for a god class/module
   Expected:
     accumulated complexity
       cyclomatic            150
       cognitive             200
       nesting               6
   ```

3. **Edge Cases**
   - God object with 1 method (should still show "accumulated")
   - Regular function with very high complexity (should show "complexity")

### Visual Regression Testing

**Before Screenshot**:
```
┌─ overview ──────────────────┐
│ location                     │
│   file        foo.rs         │
│   function    UserManager    │
│   line        10             │
│                              │
│ god object structure         │
│   methods                30  │
│   fields                 8   │
│   responsibilities       5   │
│   lines                  450 │
│                              │
│ complexity                   │  ← Ambiguous!
│   cyclomatic            150  │
│   cognitive             200  │
│   nesting               6    │
└──────────────────────────────┘
```

**After Screenshot**:
```
┌─ overview ──────────────────┐
│ location                     │
│   file        foo.rs         │
│   function    UserManager    │
│   line        10             │
│                              │
│ god object structure         │
│   methods                30  │
│   fields                 8   │
│   responsibilities       5   │
│   lines                  450 │
│                              │
│ accumulated complexity       │  ← Clear!
│   cyclomatic            150  │
│   cognitive             200  │
│   nesting               6    │
└──────────────────────────────┘
```

### Integration Tests

**File**: `tests/tui_integration_test.rs` or inline test

```rust
#[test]
fn test_complexity_header_for_god_objects() {
    let god_object_item = create_test_god_object(/* ... */);
    let regular_item = create_test_regular_function(/* ... */);

    // Render overview page
    let god_lines = render_overview(&god_object_item);
    let regular_lines = render_overview(&regular_item);

    // Verify headers
    assert!(contains_section_header(&god_lines, "accumulated complexity"));
    assert!(contains_section_header(&regular_lines, "complexity"));
}
```

### User Acceptance Testing

1. **User Study** (informal)
   - Show before/after screenshots to 3-5 developers
   - Ask: "What do the complexity numbers represent?"
   - Expected: Higher comprehension with "accumulated complexity"

2. **Documentation Check**
   - Verify existing docs don't assume "complexity" label
   - Update any references to god object complexity display

## Documentation Requirements

### Code Documentation

**File**: `src/tui/results/detail_pages/overview.rs`

Add comment before conditional logic:
```rust
// For god objects, use "accumulated complexity" to clarify that metrics
// are aggregated across all functions (cyclomatic/cognitive are summed,
// nesting is max). Regular functions show "complexity" for single-function metrics.
let complexity_header = if matches!(item.debt_type, DebtType::GodObject { .. }) {
    "accumulated complexity"
} else {
    "complexity"
};
```

### User Documentation

**File**: `book/src/tui-interface.md` (or similar)

Add section:
```markdown
### Complexity Metrics

#### Regular Functions
The "complexity" section shows metrics for the individual function:
- **cyclomatic**: Cyclomatic complexity of this function
- **cognitive**: Cognitive complexity of this function
- **nesting**: Maximum nesting depth in this function

#### God Objects
The "accumulated complexity" section shows aggregated metrics across all functions:
- **cyclomatic**: Sum of cyclomatic complexity across all methods
- **cognitive**: Sum of cognitive complexity across all methods
- **nesting**: Maximum nesting depth found in any method

The "accumulated" label indicates these are combined metrics, not a single function's complexity.
```

### Architecture Updates

No architecture documentation changes needed (display-only change).

## Implementation Notes

### Best Practices

1. **Consistency with Spec 253**
   - Spec 253 uses detection type to customize "god object structure" header
   - This spec follows same pattern: use debt type to customize complexity header
   - Maintains consistent approach across overview page

2. **Keep It Simple**
   - Single `if` expression, no complex branching
   - Reuse existing `add_section_header()` helper
   - No new functions or modules needed

3. **Readability**
   - Store header string in variable with clear name
   - Add comment explaining the distinction
   - Use pattern matching for clarity

### Common Pitfalls

1. **Don't change metric labels**
   - ❌ "total cyclomatic" vs "cyclomatic"
   - ✅ Keep labels consistent, change section header only

2. **Don't add explanatory text**
   - ❌ "accumulated complexity (sum across all methods)"
   - ✅ "accumulated complexity" is self-explanatory

3. **Don't forget god modules**
   - After spec 253, `DebtType::GodObject` covers classes, files, and modules
   - Single check works for all god object types

### Future Enhancements (Out of Scope)

- Add tooltip/help text explaining "accumulated" (spec 256?)
- Show per-function breakdown on separate page (spec 257?)
- Display average complexity alongside accumulated (spec 258?)
- Add visual indicator (icon) for accumulated vs single metrics (spec 259?)

## Migration and Compatibility

### Breaking Changes
None - pure display change, no data model or API changes.

### Migration Path
Not applicable - this is a label change in the TUI.

### Compatibility Considerations

1. **JSON Output**
   - No changes to JSON format
   - Complexity fields remain unchanged
   - Only TUI display affected

2. **Markdown Output**
   - May want to update markdown formatter similarly
   - Out of scope for this spec (TUI only)
   - Consider in future spec if users request consistency

3. **Existing Tests**
   - Tests checking for "complexity" header may need updates
   - Tests checking complexity **values** unaffected

### Rollback Strategy

If users find "accumulated complexity" confusing:
1. Revert to simple "complexity" header
2. Add explanatory note instead
3. Consider alternative wordings

Rollback is trivial (2-line change reversal).

## Success Metrics

### Quantitative Metrics
- Code change: <10 lines modified
- No performance impact (string literal change)
- Zero regressions (display-only change)

### Qualitative Metrics
- **User Comprehension**: Developers immediately understand aggregated metrics
- **Reduced Confusion**: Fewer questions about "impossibly high complexity"
- **Consistency**: Aligns with "god object structure" header pattern

### Validation Criteria
- Manual testing confirms header changes correctly
- Screenshots show clear improvement in semantic clarity
- No regressions in other TUI pages
- Documentation is updated and clear

## Rationale

### Why "Accumulated" vs Alternatives?

1. **"accumulated complexity"** ✅
   - Accurate: Complexity is accumulated/aggregated across functions
   - Works for both sum (cyclomatic/cognitive) and max (nesting)
   - Natural language, easy to understand
   - Matches existing terminology in codebase

2. **"total complexity"** ❌
   - Inaccurate: Nesting is max, not total
   - Implies simple addition (misleading for max)

3. **"aggregated complexity"** ⚠️
   - More technical/precise but less familiar to users
   - "accumulated" is more conversational

4. **"combined complexity"** ⚠️
   - Less specific about how values are combined
   - "accumulated" better conveys summation/aggregation

### Why Section Header vs Metric Labels?

Changing the **section header** is better than changing **metric labels** because:

1. **Consistency**: Metric labels match across god objects and regular functions
2. **Scannability**: Users can quickly compare "cyclomatic" values across items
3. **Minimal Change**: One label change vs three (cyclomatic, cognitive, nesting)
4. **Semantic Grouping**: Header provides context for all metrics in section

## Appendix: God Object Complexity Calculation

### Cyclomatic Complexity (Sum)

**Implementation**: `src/organization/god_object/detector.rs:306-311`

```rust
let complexity_sum: u32 = visitor
    .function_complexity
    .iter()
    .filter(|fc| method_names.contains(&fc.name))
    .map(|fc| fc.cyclomatic_complexity)
    .sum();
```

For a god class with methods: `[5, 3, 8, 12, 6]`
- Accumulated cyclomatic = 5 + 3 + 8 + 12 + 6 = **34**

### Cognitive Complexity (Sum)

Similar calculation - sum of all functions' cognitive complexity.

For methods with cognitive complexity: `[7, 4, 10, 15, 8]`
- Accumulated cognitive = 7 + 4 + 10 + 15 + 8 = **44**

### Nesting Depth (Max)

**Implementation**: Takes maximum nesting across all functions

For methods with nesting: `[2, 3, 1, 4, 2]`
- Accumulated nesting = max(2, 3, 1, 4, 2) = **4**

Note: "Accumulated" still applies even though it's max - we're accumulating observations and reporting the maximum.

## References

- **Spec 253**: Unified god object types
- **Current Code**: `src/tui/results/detail_pages/overview.rs:160-183`
- **God Object Analysis**: `src/organization/god_object/detector.rs`
- **Complexity Calculation**: `src/organization/god_object/classifier.rs`
