---
number: 267
title: Multi-Item Score Display Clarity
category: foundation
priority: high
status: draft
dependencies: []
created: 2024-12-22
---

# Specification 267: Multi-Item Score Display Clarity

**Category**: foundation
**Priority**: high
**Status**: draft
**Dependencies**: None

## Context

When multiple debt items exist at the same code location (e.g., both "Testing Gap" and "High Complexity" for the same function), the TUI displays a "combined" score that is the sum of all item scores. However, the score breakdown page shows the detailed calculation for only ONE of these items, without any indication that:

1. Multiple items exist at this location
2. Which item's breakdown is being displayed
3. How the "combined" score relates to individual scores

**Current confusing output:**

```
location
  file                      ./examples/data_pipeline.rs
  function                  run_pipeline
  line                      262

score
  combined                  163.7 [critical]    <-- What is this?

...later in score breakdown...

final score
  total                     81.8 [critical]     <-- Why different?
```

The user sees 163.7 and 81.8 with no explanation of how they relate. This violates Stillwater's "Errors Should Tell Stories" principle and DESIGN.md's "Clarity Through Restraint" guideline.

**Root cause analysis:**
- `overview.rs:75-101`: When `location_items.len() > 1`, displays "combined" as sum
- `score_breakdown.rs:32-69`: Shows "total" for a single item with no multi-item context
- No navigation exists to switch between items at the same location
- The connection between debt types listed and their scores is invisible

## Objective

Update the TUI to clearly communicate when multiple debt items exist at a location, which item's details are being shown, and how individual scores relate to the combined total. Users should never wonder why two different score numbers appear.

## Requirements

### Functional Requirements

#### 1. Overview Page Updates (`src/tui/results/detail_pages/overview.rs`)

**Current behavior (line 75-101):**
```rust
if location_items.len() > 1 {
    // Shows "combined" with sum of scores
} else {
    // Shows "total" with single score
}
```

**Required changes:**

1. When multiple items exist, add item count context to the section header:
   ```
   score (2 items at this location)
     combined                  163.7 [critical]
   ```

2. List each item's individual score below the combined:
   ```
   score (2 items at this location)
     combined                  163.7 [critical]

     item scores
       Testing Gap             81.8
       High Complexity         81.9
   ```

3. Indicate which item is currently selected for detail viewing (if applicable).

#### 2. Score Breakdown Page Updates (`src/tui/results/detail_pages/score_breakdown.rs`)

**Current behavior (line 32-69):**
- Shows "final score" -> "total" with single item's score
- No indication of which item or that other items exist

**Required changes:**

1. Add item indicator section at the top of the page when multiple items exist:
   ```
   viewing item 1 of 2: Testing Gap

   final score
     total                     81.8 [critical]
   ```

2. The item indicator should:
   - Use lowercase header style per DESIGN.md
   - Show position (1 of N) and debt type name
   - Use muted color for the indicator line
   - Be omitted when only 1 item exists at location

3. Add combined score reference at bottom of calculation steps:
   ```
   calculation steps
     ...existing steps...
     final                     81.8

   location combined           163.7 (this + 1 other item)
   ```

#### 3. Navigation Between Items

**New keyboard bindings:**

1. Add `[` and `]` keys to cycle through items at the same location:
   - `[` = previous item
   - `]` = next item
   - Wrap around at boundaries
   - Only active when multiple items exist at location

2. Update help overlay to show new bindings when applicable.

3. Display navigation hint in footer when multiple items exist:
   ```
   [/] prev/next item at location
   ```

#### 4. List View Updates (`src/tui/results/list_view.rs`)

When displaying location groups with multiple items:

1. Show item count indicator:
   ```
   #1    CRITICAL   163.7   data_pipeline.rs::run_pipeline   (2 items)
   ```

2. The count indicator should be in muted color.

### Non-Functional Requirements

1. **Performance**: No additional computation - use existing `location_items` vector
2. **Consistency**: Follow DESIGN.md column alignment (20-char labels, 4-char gap)
3. **Accessibility**: Information not conveyed by color alone (count is text)
4. **Minimalism**: Add only essential context, no visual clutter

## Acceptance Criteria

- [ ] Overview page shows "(N items at this location)" when multiple items exist
- [ ] Overview page lists individual item scores below combined score
- [ ] Score breakdown page shows "viewing item X of N: {DebtType}" header
- [ ] Score breakdown page shows "location combined" reference when applicable
- [ ] `[` and `]` keys cycle through items at same location
- [ ] Help overlay shows item navigation when multiple items exist
- [ ] List view shows "(N items)" indicator for multi-item locations
- [ ] Single-item locations display unchanged (no regression)
- [ ] All text follows DESIGN.md lowercase header convention
- [ ] Column alignment follows 20-char label + 4-char gap pattern

## Technical Details

### Implementation Approach

1. **Data Flow**: The `location_items: Vec<&UnifiedDebtItem>` already exists in detail view context. No new data structures needed.

2. **State Management**: Add `current_item_index: usize` to detail view state to track which item is being shown.

3. **Pure Functions**: All new display logic should be pure functions in the respective `build_*_section` modules.

### Architecture Changes

**Modified files:**

1. `src/tui/results/detail_pages/overview.rs`
   - Modify `build_score_section()` to accept item count and show individual scores
   - Add `build_item_scores_subsection()` pure function

2. `src/tui/results/detail_pages/score_breakdown.rs`
   - Add `build_item_indicator_section()` pure function
   - Modify `build_calculation_summary_section()` to include combined reference

3. `src/tui/results/navigation.rs`
   - Add `[` and `]` key handlers
   - Add `cycle_location_item(direction: i32)` function

4. `src/tui/results/app.rs`
   - Add `current_item_index: usize` to `ResultsApp` state
   - Add `location_item_count()` helper method

5. `src/tui/results/list_view.rs`
   - Modify item rendering to show "(N items)" indicator

6. `src/tui/results/help.rs`
   - Add conditional item navigation hint

### Data Structures

No new data structures required. Existing `LocationGroup` already contains:
```rust
pub struct LocationGroup {
    pub items: Vec<ViewItem>,
    pub combined_score: f64,
    pub item_count: usize,
}
```

### APIs and Interfaces

New pure functions:

```rust
// overview.rs
pub fn build_item_scores_subsection(
    items: &[&UnifiedDebtItem],
    theme: &Theme,
    width: u16,
) -> Vec<Line<'static>>;

// score_breakdown.rs
pub fn build_item_indicator_section(
    current_index: usize,
    total_items: usize,
    debt_type: &DebtType,
    theme: &Theme,
    width: u16,
) -> Vec<Line<'static>>;

pub fn build_combined_reference_line(
    item_score: f64,
    combined_score: f64,
    other_item_count: usize,
    theme: &Theme,
    width: u16,
) -> Line<'static>;
```

## Dependencies

- **Prerequisites**: None
- **Affected Components**: TUI detail pages, navigation, list view
- **External Dependencies**: None (uses existing ratatui primitives)

## Testing Strategy

### Unit Tests

1. `build_item_indicator_section()` - verify output for various item counts
2. `build_item_scores_subsection()` - verify formatting and alignment
3. `build_combined_reference_line()` - verify math and display

### Integration Tests

1. Navigate to multi-item location, verify overview shows count
2. Press `]` to cycle items, verify breakdown updates
3. Verify single-item locations remain unchanged

### Manual Testing

1. Run debtmap on prodigy codebase
2. Find a location with multiple debt items
3. Verify all acceptance criteria visually
4. Test keyboard navigation

## Documentation Requirements

- **Code Documentation**: Doc comments on all new public functions
- **User Documentation**: None (self-explanatory UI)
- **DESIGN.md Updates**: None (follows existing patterns)

## Implementation Notes

### Display Format Examples

**Overview page with 2 items:**
```
score (2 items at this location)
  combined                  163.7 [critical]

  item scores
    Testing Gap             81.8
    High Complexity         81.9
```

**Score breakdown page header:**
```
viewing item 1 of 2: Testing Gap

final score
  total                     81.8 [critical]
```

**Score breakdown page footer:**
```
calculation steps
  1. weighted base          34.25 = ...
  ...
  final                     81.8

location combined           163.7 (this + 1 other item)
```

**List view:**
```
▸ #1    CRITICAL   163.7   data_pipeline.rs::run_pipeline   (2 items)
  #2    HIGH        45.2   parser.rs::parse_expression
```

### Edge Cases

1. **Single item**: No indicator shown, display unchanged
2. **Many items (5+)**: Consider truncating item list with "and N more..."
3. **Item cycling wrap**: After last item, wrap to first
4. **Different pages**: Item indicator only on relevant pages (overview, breakdown)

## Migration and Compatibility

No breaking changes. All changes are additive UI enhancements.

Existing behavior for single-item locations is preserved exactly.
