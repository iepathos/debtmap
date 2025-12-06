---
number: 231
title: Dotted Leader Connections for Detail View Labels
category: optimization
priority: medium
status: draft
dependencies: [230]
created: 2025-12-06
---

# Specification 231: Dotted Leader Connections for Detail View Labels

**Category**: optimization
**Priority**: medium
**Status**: draft
**Dependencies**: Spec 230 (Lowercase Section Headers)

## Context

Detail view currently uses simple colon-separated label-value pairs:

```
metrics
  Cyclomatic Complexity: 15
  Cognitive Complexity: 22
  Nesting Depth: 4
  Function Length: 87
```

This format works but lacks the visual elegance of the progression view, which uses dotted leaders to connect related information:

**Progression view pattern** (from `src/tui/renderer.rs`):
```
✓  parsing ·················································· 1,234 files
✓  semantic analysis ··········································· done
    analyzing functions ▸▸▸▸▸▸▸▸▸▸▸▸····················· 4,231/10,000
```

Dotted leaders create visual connection without adding clutter. They guide the eye from label to value while maintaining breathing room.

**Current implementation** (`src/tui/results/detail_pages/components.rs:20-26`):
```rust
pub fn add_label_value(lines: &mut Vec<Line<'static>>, label: &str, value: String, theme: &Theme) {
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::raw(format!("{}: ", label)),  // <-- Simple colon
        Span::styled(value, Style::default().fg(theme.primary)),
    ]));
}
```

**Design gap**: Colon is abrupt. Dotted leaders are more zen - they connect without dominating.

## Objective

Replace colon-separated labels with dotted leader connections in detail view label-value pairs, matching the elegant aesthetic of the progression view.

**Visual Transformation** (Option A from mockups):

**Before** (colon-separated):
```
metrics
  Cyclomatic Complexity: 15
  Cognitive Complexity: 22
  Nesting Depth: 4
  Function Length: 87
```

**After** (dotted leaders):
```
metrics
  cyclomatic ················ 15
  cognitive ················· 22
  nesting ··················· 4
  length ···················· 87
```

**Success Metrics**:
- Improved visual elegance matching progression view
- Clearer visual connection between labels and values
- Maintained readability and scannability

## Requirements

### Functional Requirements
- All label-value information preserved
- Dynamic width calculation for variable-length labels
- Proper alignment in different terminal widths

### Visual Requirements
- Use `·` (U+00B7 middle dot) for leaders
- Muted color for dotted leaders (matching progression view)
- Labels in muted color, values in primary color
- Maintain 2-space indentation
- Calculate dots dynamically based on available width

### Scope
All `add_label_value()` call sites in detail pages:
- **Overview page**: file, function, line, all metrics, coverage
- **Dependencies page**: dependency counts, caller/callee names
- **Git Context page**: commit frequency, authors, churn metrics
- **Patterns page**: framework patterns, purity indicators

## Acceptance Criteria

- [ ] `add_label_value()` uses dotted leaders instead of colons
- [ ] Dots calculated dynamically based on terminal width
- [ ] Labels shortened to single word (e.g., "Cyclomatic Complexity" → "cyclomatic")
- [ ] Dotted leaders use muted color
- [ ] Values maintain primary color for prominence
- [ ] Alignment preserved in 80-col and 120-col terminals
- [ ] All detail pages render with new format
- [ ] Manual testing confirms improved visual elegance

## Technical Details

### Implementation Approach

**Phase 1: Update `add_label_value()` function**

Location: `src/tui/results/detail_pages/components.rs:20-26`

```rust
// Current (colon-separated)
pub fn add_label_value(lines: &mut Vec<Line<'static>>, label: &str, value: String, theme: &Theme) {
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::raw(format!("{}: ", label)),
        Span::styled(value, Style::default().fg(theme.primary)),
    ]));
}

// Proposed (dotted leaders)
pub fn add_label_value(
    lines: &mut Vec<Line<'static>>,
    label: &str,
    value: String,
    theme: &Theme,
    width: u16,  // New parameter: terminal width for dynamic calculation
) {
    const INDENT: usize = 2;
    const VALUE_BUFFER: usize = 3; // Space before value

    let label_with_indent = format!("{}{}", " ".repeat(INDENT), label);
    let total_content_len = label_with_indent.len() + value.len() + VALUE_BUFFER;
    let dots_needed = (width as usize).saturating_sub(total_content_len);

    lines.push(Line::from(vec![
        Span::raw(label_with_indent),
        Span::raw(" "),
        Span::styled(
            "·".repeat(dots_needed),
            Style::default().fg(theme.muted)
        ),
        Span::raw(" "),
        Span::styled(value, Style::default().fg(theme.primary)),
    ]));
}
```

**Phase 2: Simplify label text at call sites**

Example from `overview.rs:27-39`:

```rust
// Current
add_label_value(&mut lines, "File", item.location.file.display().to_string(), theme);
add_label_value(&mut lines, "Function", item.location.function.clone(), theme);
add_label_value(&mut lines, "Line", item.location.line.to_string(), theme);

// Metrics section
add_label_value(&mut lines, "Cyclomatic Complexity", item.cyclomatic_complexity.to_string(), theme);
add_label_value(&mut lines, "Cognitive Complexity", item.cognitive_complexity.to_string(), theme);
add_label_value(&mut lines, "Nesting Depth", item.nesting_depth.to_string(), theme);
add_label_value(&mut lines, "Function Length", item.function_length.to_string(), theme);

// Proposed (simplified labels + width parameter)
add_label_value(&mut lines, "file", item.location.file.display().to_string(), theme, area.width);
add_label_value(&mut lines, "function", item.location.function.clone(), theme, area.width);
add_label_value(&mut lines, "line", item.location.line.to_string(), theme, area.width);

// Metrics section (abbreviated labels)
add_label_value(&mut lines, "cyclomatic", item.cyclomatic_complexity.to_string(), theme, area.width);
add_label_value(&mut lines, "cognitive", item.cognitive_complexity.to_string(), theme, area.width);
add_label_value(&mut lines, "nesting", item.nesting_depth.to_string(), theme, area.width);
add_label_value(&mut lines, "length", item.function_length.to_string(), theme, area.width);
```

**Label simplification mapping**:
- `"File"` → `"file"`
- `"Function"` → `"function"`
- `"Line"` → `"line"`
- `"Cyclomatic Complexity"` → `"cyclomatic"`
- `"Cognitive Complexity"` → `"cognitive"`
- `"Nesting Depth"` → `"nesting"`
- `"Function Length"` → `"length"`
- `"Coverage"` → `"direct"` (or `"coverage"`)

### Architecture Changes

**Files Modified**:
1. `src/tui/results/detail_pages/components.rs`
   - `add_label_value()` - add `width` parameter, implement dotted leaders

2. `src/tui/results/detail_pages/overview.rs`
   - All `add_label_value()` call sites (~10-15 locations)
   - Simplify label strings
   - Pass `area.width` parameter

3. `src/tui/results/detail_pages/dependencies.rs`
   - Update call sites if present

4. `src/tui/results/detail_pages/git_context.rs`
   - Update call sites if present

5. `src/tui/results/detail_pages/patterns.rs`
   - Update call sites if present

**Function signature change** (breaking for internal API):
```rust
// Old signature
pub fn add_label_value(lines: &mut Vec<Line<'static>>, label: &str, value: String, theme: &Theme)

// New signature
pub fn add_label_value(lines: &mut Vec<Line<'static>>, label: &str, value: String, theme: &Theme, width: u16)
```

All call sites must be updated to pass `area.width`.

### Visual Impact Analysis

**Before** (Current - Colon-separated):
```
┌─────────────────────────────────────────────────────────────┐
│ location                                                     │
│   File: src/analysis/complexity.rs                          │
│   Function: calculate_cognitive_complexity                  │
│   Line: 142                                                  │
│                                                              │
│ metrics                                                      │
│   Cyclomatic Complexity: 15                                 │
│   Cognitive Complexity: 22                                  │
│   Nesting Depth: 4                                          │
│   Function Length: 87                                       │
└─────────────────────────────────────────────────────────────┘
```

**After** (Proposed - Dotted Leaders):
```
┌─────────────────────────────────────────────────────────────┐
│ location                                                     │
│   file ············· src/analysis/complexity.rs             │
│   function ········· calculate_cognitive_complexity         │
│   line ············· 142                                    │
│                                                              │
│ metrics                                                      │
│   cyclomatic ······· 15                                     │
│   cognitive ········ 22                                     │
│   nesting ·········· 4                                      │
│   length ··········· 87                                     │
└─────────────────────────────────────────────────────────────┘
```

**Aesthetic Improvements**:
- Visual connection created by dotted path from label to value
- Shorter labels reduce visual clutter
- Dots recede (muted color) while values pop (primary color)
- Matches progression view's elegant aesthetic
- More breathing room between label and value

### Dynamic Width Calculation

**Algorithm**:
```rust
// Terminal width: 80 columns
// Label: "  cyclomatic" (13 chars including indent)
// Value: "15" (2 chars)
// Buffer: 2 spaces (before and after dots)

let available = 80;
let used = 13 (label) + 2 (value) + 2 (buffer) = 17;
let dots = available - used = 63 dots

Result: "  cyclomatic ···························································· 15"
```

**Edge cases**:
- Very narrow terminals (<40 cols): Reduce dots, may show minimal or no dots
- Very long values: Dots compress, minimum 3 dots preserved
- Maximum dots: Capped at reasonable limit to avoid overwhelming

### Pattern Consistency

This change makes detail view consistent with progression view dotted leader usage:

**Progression subtask** (renderer.rs:204-210):
```rust
let dots_needed = width.saturating_sub((name_with_indent.len() + 10) as u16) as usize;
Line::from(vec![
    Span::raw(name_with_indent),
    Span::raw(" "),
    Span::styled("·".repeat(dots_needed), theme.dotted_leader_style()),
    Span::styled(" done", theme.completed_style()),
])
```

**Detail view label-value** (proposed):
```rust
let dots_needed = (width as usize).saturating_sub(total_content_len);
Line::from(vec![
    Span::raw(label_with_indent),
    Span::raw(" "),
    Span::styled("·".repeat(dots_needed), theme.muted),
    Span::raw(" "),
    Span::styled(value, theme.primary),
])
```

Same pattern, same aesthetic.

## Dependencies

**Prerequisites**:
- Spec 230 (Lowercase Section Headers) - establishes lowercase pattern

**Affected Components**:
- All detail page modules
- Shared components module

**External Dependencies**: None

## Testing Strategy

### Unit Tests
```rust
#[test]
fn test_dotted_leader_calculation() {
    let label = "cyclomatic";
    let value = "15";
    let width = 80;

    let expected_dots = width - (2 + label.len() + 1 + value.len() + 1);
    assert_eq!(expected_dots, 63);
}

#[test]
fn test_minimum_dots_narrow_terminal() {
    let label = "very_long_label_name";
    let value = "12345";
    let width = 40;

    // Should still show at least 3 dots or gracefully degrade
    let dots = calculate_dots(label, value, width);
    assert!(dots >= 3 || width < 30);
}
```

### Manual Testing
- [ ] Launch TUI, navigate to detail view overview page
- [ ] Verify dotted leaders connect labels to values
- [ ] Test in 80-column terminal - dots fill space appropriately
- [ ] Test in 120-column terminal - more dots, still aligned
- [ ] Test in narrow terminal (40 cols) - graceful degradation
- [ ] Verify all metrics sections use dotted leaders
- [ ] Test all 4 detail pages
- [ ] Confirm dots are muted color, values are primary color

### Visual Regression Testing
- [ ] Compare before/after screenshots
- [ ] Verify alignment across different value lengths
- [ ] Check spacing consistency
- [ ] Confirm readability maintained

### Edge Cases
- [ ] Very long file paths
- [ ] Single-digit vs multi-digit values
- [ ] Terminal resize during display
- [ ] Maximum label length scenarios

## Documentation Requirements

### Code Documentation
- Document `add_label_value()` width parameter
- Add example usage to function doc comment
- Explain dotted leader calculation

### Design Documentation
- Update DESIGN.md "Label-Value Pairs" section
- Add dotted leader pattern to design patterns
- Include terminal width calculation guidelines

### User Documentation
- No user documentation needed (cosmetic change)

## Implementation Notes

### Width Parameter Propagation

All detail page render functions receive `area: Rect` parameter:
```rust
pub fn render(
    frame: &mut Frame,
    app: &ResultsApp,
    item: &UnifiedDebtItem,
    area: Rect,  // <-- Contains width
    theme: &Theme,
)
```

Pass `area.width` to `add_label_value()`:
```rust
add_label_value(&mut lines, "cyclomatic", value, theme, area.width);
```

### Label Abbreviation Strategy

**Principle**: Use shortest unambiguous word

- Multi-word labels → single representative word
- Technical terms → common abbreviation
- Context-specific → maintain clarity

**Examples**:
- `"Cyclomatic Complexity"` → `"cyclomatic"` (cognitive is also present, context clear)
- `"Function Length"` → `"length"` (context is function, obvious)
- `"Nesting Depth"` → `"nesting"` (depth implied)
- `"Coverage"` → `"direct"` (when showing direct coverage specifically)

### Minimum Dots Threshold

If calculated dots < 3, consider fallback:
```rust
let dots_needed = (width as usize).saturating_sub(total_content_len);
if dots_needed < 3 {
    // Option 1: Minimal format without dots
    // Option 2: Force minimum 3 dots, allow overflow
    // Option 3: Truncate value
}
```

Recommend **Option 2**: Force minimum 3 dots for visual consistency, accept minor overflow in extremely narrow terminals.

### Grep Commands for Call Site Updates

```bash
# Find all add_label_value call sites
rg "add_label_value" src/tui/results/detail_pages/

# Count call sites to track progress
rg "add_label_value" src/tui/results/detail_pages/ -c

# Verify all updated with width parameter
rg "add_label_value.*area\.width" src/tui/results/detail_pages/ -c
```

## Migration and Compatibility

**Breaking Changes**:
- Internal API only - `add_label_value()` signature changed
- No external API impact

**Backward Compatibility**: Full compatibility for users

**Migration Path**:
- Update all call sites in single commit
- No gradual migration needed (internal API)

## Related Specifications

- **Spec 229**: Lowercase Severity Labels (foundation)
- **Spec 230**: Lowercase Section Headers (prerequisite)
- **Spec 232**: Simplified Header Labels (planned - list view)
- **Spec 233**: Simplified Footer Shortcuts (planned)

These specs together transform the entire TUI to match the progression view's zen aesthetic.

## Visual Design Philosophy

From DESIGN.md "Component Design Patterns":

> **Dotted Leaders**
> ```rust
> format!("{} {} {}", label, "·".repeat(width), value)
> ```
> **Rationale**: Classic typographic technique creates visual connection without lines/borders.

This spec applies that pattern consistently to detail view label-value pairs.

**Why dotted leaders work**:
1. **Visual connection** - Eye follows dots from label to value
2. **Breathing room** - Dots create space without emptiness
3. **Typographic tradition** - Used in tables of contents for centuries
4. **Minimal visual weight** - Dots recede, content stands out
5. **Dynamic flexibility** - Adapts to any terminal width

The result is a detail view that feels as polished and zen as the progression view.
