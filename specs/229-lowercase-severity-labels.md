---
number: 229
title: Lowercase Severity Labels for Zen Minimalism
category: optimization
priority: medium
status: draft
dependencies: []
created: 2025-12-06
---

# Specification 229: Lowercase Severity Labels for Zen Minimalism

**Category**: optimization
**Priority**: medium
**Status**: draft
**Dependencies**: None

## Context

The TUI currently displays severity levels in ALL CAPS across list and detail views:

```
▸ #1    CRITICAL   Score:142.3   file.rs::process_data
  #2    HIGH       Score:87.5    api.rs::handle_request
  #3    MEDIUM     Score:34.2    util.rs::format_output
  #4    LOW        Score:8.1     test.rs::verify_result
```

This conflicts with the "futuristic zen minimalism" design philosophy established in DESIGN.md and implemented in the progression view, which uses lowercase throughout:

```
debtmap  12.3s
stage 3/9
functions 10,234  │  debt 45  │  coverage 72.4%
```

**Design Philosophy from DESIGN.md**:
- "Clarity Through Restraint" - Every visual element serves a purpose
- "No decoration" - Every element serves information, not aesthetics
- "Muted palette" - Calm, not aggressive or flashy

ALL CAPS severity labels feel "aggressive" and "shouting" rather than zen and minimal. The severity is already communicated through color (red for critical, yellow for medium, green for low), making the capitalization redundant visual noise.

## Objective

Convert all severity labels from uppercase to lowercase throughout the TUI to achieve visual consistency with the progression view and align with the zen minimalist design philosophy.

**Success Metrics**:
- All severity labels render in lowercase
- Visual weight reduced while maintaining information clarity
- Consistent aesthetic across all TUI views

## Requirements

### Functional Requirements
- **Severity calculation**: No change to `calculate_severity()` logic
- **Color mapping**: Preserve existing severity color scheme (red/yellow/green)
- **Information content**: No loss of information - severity still clearly visible

### Visual Requirements
- Convert "CRITICAL" → "critical"
- Convert "HIGH" → "high"
- Convert "MEDIUM" → "medium"
- Convert "LOW" → "low"
- Maintain severity color coding unchanged
- Update column width calculations if needed (8 chars vs 10 chars)

### Scope
- **List view**: Severity column in both grouped and ungrouped modes
- **Detail view - Overview page**: Severity label in score section
- **Consistency**: All locations where severity is displayed

## Acceptance Criteria

- [ ] `calculate_severity()` returns lowercase severity strings
- [ ] List view displays lowercase severity labels
- [ ] Grouped list view displays lowercase severity labels
- [ ] Detail view overview page displays lowercase severity in score section
- [ ] Severity colors remain unchanged (CRITICAL still red, etc.)
- [ ] Column alignment maintained in list view
- [ ] No visual regression - spacing and layout preserved
- [ ] Manual testing confirms improved zen aesthetic

## Technical Details

### Implementation Approach

**Phase 1: Update severity calculation function**

Location: `src/tui/results/list_view.rs:531-541`

```rust
// Current
fn calculate_severity(score: f64) -> &'static str {
    if score >= 100.0 {
        "CRITICAL"
    } else if score >= 50.0 {
        "HIGH"
    } else if score >= 10.0 {
        "MEDIUM"
    } else {
        "LOW"
    }
}

// Proposed
fn calculate_severity(score: f64) -> &'static str {
    if score >= 100.0 {
        "critical"
    } else if score >= 50.0 {
        "high"
    } else if score >= 10.0 {
        "medium"
    } else {
        "low"
    }
}
```

**Phase 2: Update column formatting**

Location: `src/tui/results/list_view.rs:474`

```rust
// Current - 10 character width for uppercase
format!("{:<10}", severity)  // "CRITICAL  " or "HIGH      "

// Proposed - 8 character width for lowercase
format!("{:<8}", severity)   // "critical" or "high    "
```

**Phase 3: Update detail view**

Location: `src/tui/results/detail_pages/overview.rs:340-350`

The `calculate_severity()` function is duplicated here - apply same lowercase change.

### Architecture Changes

**Files Modified**:
1. `src/tui/results/list_view.rs`
   - `calculate_severity()` function (line 531-541)
   - Format string in `format_list_item()` (line 474)
   - Format string in `format_grouped_item()` (line 331)

2. `src/tui/results/detail_pages/overview.rs`
   - `calculate_severity()` function (line 340-350)

**No structural changes** - purely cosmetic string value changes.

### Visual Impact Analysis

**Before**:
```
▸ #1    CRITICAL   142.3   file.rs::process_data
  #2    HIGH        87.5   api.rs::handle_request
  #3    MEDIUM      34.2   util.rs::format_output
  #4    LOW          8.1   test.rs::verify_result
```

**After**:
```
▸ #1    critical  142.3   file.rs::process_data
  #2    high       87.5   api.rs::handle_request
  #3    medium     34.2   util.rs::format_output
  #4    low         8.1   test.rs::verify_result
```

**Improved aesthetics**:
- Softer visual weight - less "shouting"
- Consistent with progression view lowercase style
- Color still provides severity signal
- More zen and minimal

## Dependencies

**Prerequisites**: None - standalone cosmetic change

**Affected Components**:
- List view rendering
- Grouped list view rendering
- Detail view overview page

**External Dependencies**: None

## Testing Strategy

### Manual Testing
- [ ] Launch TUI with sample analysis results
- [ ] Verify list view shows lowercase severity
- [ ] Test grouped mode shows lowercase severity
- [ ] Navigate to detail view, confirm lowercase in score section
- [ ] Test all severity levels (critical, high, medium, low)
- [ ] Confirm colors unchanged (red for critical, etc.)
- [ ] Verify column alignment maintained

### Visual Regression Testing
- [ ] Compare before/after screenshots
- [ ] Verify spacing preserved
- [ ] Confirm no layout shifts
- [ ] Check alignment in both 80-col and 120-col terminals

### Edge Cases
- [ ] Empty results list
- [ ] Single item with each severity level
- [ ] Mixed severity levels in grouped view
- [ ] Very narrow terminal (compact mode)

## Documentation Requirements

### Code Documentation
- No changes needed - function signatures unchanged

### User Documentation
- No user-facing documentation needed (cosmetic change)

### Design Documentation
- Update DESIGN.md example screenshots if present
- Document lowercase convention in "Typography & Glyphs" section

## Implementation Notes

### Simplicity
This is a **high-impact, low-effort** change:
- 2 functions modified (same logic duplicated in 2 files)
- String literals changed from uppercase to lowercase
- Column width adjusted (-2 characters)
- No algorithmic changes
- No new dependencies

### Consistency Opportunity
This change creates foundation for further lowercase conversions in spec 230 (section headers) and spec 231 (label simplification).

### Color Reliance
After this change, severity information is conveyed through:
1. **Color** (primary signal - red/yellow/green)
2. **Text label** (secondary - "critical"/"medium"/etc.)

Users relying on color-blind-friendly terminals still have the text label. The lowercase doesn't reduce accessibility.

## Migration and Compatibility

**Breaking Changes**: None

**Backward Compatibility**: Full compatibility - output format changes but no API/data changes

**Migration Path**: None needed - cosmetic change only

## Related Specifications

- **Spec 230**: Lowercase Section Headers (planned)
- **Spec 231**: Simplified Label Format (planned)
- **Spec 232**: Dotted Leaders in Detail View (planned)

These specifications together will bring the entire TUI into alignment with the zen minimalist aesthetic established in DESIGN.md and demonstrated in the progression view.

## Visual Design Philosophy

From DESIGN.md:

> **Futuristic Zen Minimalism** - The design philosophy rests on three pillars:
> 1. Clarity Through Restraint - Every visual element serves a purpose
> 2. Subtle Motion - Animations guide attention without distraction
> 3. Information Hierarchy - Important data stands out naturally through color and spacing

Lowercase severity labels support all three pillars:
- **Clarity**: Color and position already establish severity - caps add no clarity
- **Subtlety**: Lowercase is calmer and less aggressive than CAPS
- **Hierarchy**: Color creates the hierarchy - text reinforces without dominating

This change is small but represents a philosophical shift toward restraint and minimalism.
