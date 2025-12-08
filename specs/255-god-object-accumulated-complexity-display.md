---
number: 255
title: God Object Accumulated Complexity Metric Labels
category: foundation
priority: low
status: draft
dependencies: [253]
created: 2025-12-07
---

# Specification 255: God Object Accumulated Complexity Metric Labels

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

Update the TUI overview page to use **descriptive metric labels** for god object complexity, making it immediately clear that the metrics represent **aggregated values across all functions** rather than a single function's complexity.

For god objects:
- Use **"accumulated cyclomatic"** and **"accumulated cognitive"** (summed across all functions)
- Use **"max nesting"** (maximum nesting depth found in any function)

For regular functions, keep existing labels ("cyclomatic", "cognitive", "nesting") as they represent single-function metrics.

## Requirements

### Functional Requirements

1. **Consistent Section Header**
   - All items: Display section header as **"complexity"**
   - No conditional logic for section header (simpler)

2. **Conditional Metric Labels**
   - God objects:
     - **"accumulated cyclomatic"** (sum across all functions)
     - **"accumulated cognitive"** (sum across all functions)
     - **"max nesting"** (maximum across all functions)
   - Regular functions:
     - **"cyclomatic"** (this function's value)
     - **"cognitive"** (this function's value)
     - **"nesting"** (this function's value)
   - Use `matches!(item.debt_type, DebtType::GodObject { .. })` to determine labels

3. **Semantic Precision**
   - "accumulated" clearly indicates summation
   - "max" clearly indicates maximum value
   - Labels directly describe the aggregation strategy
   - Users immediately understand what each metric represents

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
- [ ] **Section Header**: All items show "complexity" (unchanged)
- [ ] **God Object Labels**: Shows "accumulated cyclomatic", "accumulated cognitive", "max nesting"
- [ ] **Regular Function Labels**: Shows "cyclomatic", "cognitive", "nesting" (unchanged)
- [ ] **Values**: Complexity values unchanged (still aggregated for god objects)
- [ ] **Semantic Accuracy**: "accumulated" for sums, "max" for maximum
- [ ] **Visual Consistency**: Follows existing theme and label styling
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
add_section_header(&mut lines, "complexity", theme);

// For god objects, use descriptive labels indicating aggregation strategy
let is_god_object = matches!(item.debt_type, DebtType::GodObject { .. });

let cyclomatic_label = if is_god_object { "accumulated cyclomatic" } else { "cyclomatic" };
let cognitive_label = if is_god_object { "accumulated cognitive" } else { "cognitive" };
let nesting_label = if is_god_object { "max nesting" } else { "nesting" };

add_label_value(&mut lines, cyclomatic_label, item.cyclomatic_complexity.to_string(), ...);
add_label_value(&mut lines, cognitive_label, item.cognitive_complexity.to_string(), ...);
add_label_value(&mut lines, nesting_label, item.nesting_depth.to_string(), ...);
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

#### Option 1: Change Section Header ❌
```rust
let header = if is_god_object { "accumulated complexity" } else { "complexity" };
add_section_header(&mut lines, header, theme);
```

**Rejected**: Less precise - doesn't distinguish sum vs max

#### Option 2: Add Explanation Text ❌
```rust
lines.push(Line::from("  (aggregated across all functions)"));
```

**Rejected**: Too verbose, clutters display

#### Option 3: Use "total" instead of "accumulated" ❌
```rust
add_label_value(&mut lines, "total cyclomatic", ...);
add_label_value(&mut lines, "total cognitive", ...);
add_label_value(&mut lines, "total nesting", ...);
```

**Rejected**: "total nesting" is misleading (it's max, not sum)

#### Option 4: This Spec's Approach ✅
```rust
let cyclomatic_label = if is_god_object { "accumulated cyclomatic" } else { "cyclomatic" };
let cognitive_label = if is_god_object { "accumulated cognitive" } else { "cognitive" };
let nesting_label = if is_god_object { "max nesting" } else { "nesting" };
```

**Accepted**:
- Semantically precise ("accumulated" for sums, "max" for maximum)
- Consistent section header across all items
- Self-documenting labels
- Easy to scan and understand

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
┌─ overview ──────────────────────────┐
│ location                             │
│   file        foo.rs                 │
│   function    UserManager            │
│   line        10                     │
│                                      │
│ god object structure                 │
│   methods                30          │
│   fields                 8           │
│   responsibilities       5           │
│   lines                  450         │
│                                      │
│ complexity                           │
│   cyclomatic            150          │  ← Ambiguous!
│   cognitive             200          │  ← Is this sum or single value?
│   nesting               6            │  ← Is this max or sum?
└──────────────────────────────────────┘
```

**After Screenshot**:
```
┌─ overview ──────────────────────────┐
│ location                             │
│   file        foo.rs                 │
│   function    UserManager            │
│   line        10                     │
│                                      │
│ god object structure                 │
│   methods                30          │
│   fields                 8           │
│   responsibilities       5           │
│   lines                  450         │
│                                      │
│ complexity                           │
│   accumulated cyclomatic    150      │  ← Clear: sum!
│   accumulated cognitive     200      │  ← Clear: sum!
│   max nesting              6         │  ← Clear: maximum!
└──────────────────────────────────────┘
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
   - Expected: Higher comprehension with descriptive labels ("accumulated", "max")

2. **Documentation Check**
   - Verify existing docs don't assume "complexity" label
   - Update any references to god object complexity display

## Documentation Requirements

### Code Documentation

**File**: `src/tui/results/detail_pages/overview.rs`

Add comment before conditional logic:
```rust
// For god objects, use descriptive labels to clarify aggregation strategy:
// - "accumulated cyclomatic/cognitive" = sum across all functions
// - "max nesting" = maximum nesting depth found in any function
// Regular functions use simple labels as they represent single-function metrics.
let is_god_object = matches!(item.debt_type, DebtType::GodObject { .. });
let cyclomatic_label = if is_god_object { "accumulated cyclomatic" } else { "cyclomatic" };
let cognitive_label = if is_god_object { "accumulated cognitive" } else { "cognitive" };
let nesting_label = if is_god_object { "max nesting" } else { "nesting" };
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
The "complexity" section shows aggregated metrics across all functions with descriptive labels:
- **accumulated cyclomatic**: Sum of cyclomatic complexity across all methods
- **accumulated cognitive**: Sum of cognitive complexity across all methods
- **max nesting**: Maximum nesting depth found in any method

The labels clearly indicate the aggregation strategy used for each metric.
```

### Architecture Updates

No architecture documentation changes needed (display-only change).

## Implementation Notes

### Best Practices

1. **Semantic Precision**
   - "accumulated" specifically means sum/aggregation
   - "max" specifically means maximum value
   - Different metrics use different aggregation strategies
   - Labels must reflect the actual calculation

2. **Keep It Simple**
   - Single `if` expression, no complex branching
   - Reuse existing `add_section_header()` helper
   - No new functions or modules needed

3. **Readability**
   - Store header string in variable with clear name
   - Add comment explaining the distinction
   - Use pattern matching for clarity

### Common Pitfalls

1. **Don't use "total" for all metrics**
   - ❌ "total nesting" (misleading - it's max, not sum)
   - ✅ "max nesting" (accurate)

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

### Why Metric Labels vs Section Header?

Changing the **metric labels** is better than changing the **section header** because:

1. **Precision**: Can use "accumulated" for sums and "max" for maximum
2. **Accuracy**: Labels directly describe the aggregation strategy
3. **Semantic Clarity**: Each metric self-documents its meaning
4. **Simplicity**: Section header stays consistent across all items

### Why "Accumulated" vs "Total"?

1. **"accumulated cyclomatic/cognitive"** ✅
   - Accurate: Values are accumulated/summed across functions
   - Natural language, easy to understand
   - Matches terminology in analysis phase

2. **"total cyclomatic/cognitive"** ⚠️
   - Less precise about accumulation process
   - Could work, but "accumulated" is more descriptive

3. **"sum cyclomatic/cognitive"** ❌
   - Too technical/mathematical
   - Less natural in UI context

### Why "Max" for Nesting?

1. **"max nesting"** ✅
   - Accurate: Value is maximum across all functions
   - Clear and unambiguous
   - Immediately understandable

2. **"accumulated nesting"** ❌
   - Misleading: Implies sum, but it's actually max
   - Semantically incorrect

3. **"nesting depth"** ❌
   - Doesn't clarify aggregation strategy
   - Ambiguous like current implementation

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
