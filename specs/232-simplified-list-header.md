---
number: 232
title: Simplified List View Header Labels
category: optimization
priority: low
status: draft
dependencies: [229, 230]
created: 2025-12-06
---

# Specification 232: Simplified List View Header Labels

**Category**: optimization
**Priority**: low
**Status**: draft
**Dependencies**: Spec 229 (Lowercase Severity), Spec 230 (Lowercase Section Headers)

## Context

The list view header currently uses verbose label-value format:

```
──────────────────────────────────────────────────────────────────
Debtmap Results  Total: 45  Debt Score: 1234  Density: 12.34/1K LOC
Sort: Unified Score  Filters: 0  Grouping: OFF
──────────────────────────────────────────────────────────────────
```

This conflicts with the progression view's minimal aesthetic:

**Progression view header** (from renderer.rs:74-99):
```
debtmap  12.3s

▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓░░░ 75%

stage 3/9
```

**Progression view footer** (from renderer.rs:250-258):
```
functions 10,234  │  debt 45  │  coverage 72.4%
```

Key differences:
- **Lowercase titles**: "debtmap" not "Debtmap Results"
- **No label prefixes**: "10,234" not "Total: 10,234"
- **Pipe separators**: Clean `│` instead of multiple spaces
- **Minimal text**: "stage 3/9" not "Stage: 3/9"

The list view header is verbose and "enterprise-y" rather than zen minimal.

**Current implementation** (`src/tui/results/list_view.rs:169-214`):
```rust
Line::from(vec![
    Span::styled("Debtmap Results", Style::default().fg(theme.accent())),
    Span::raw("  "),
    Span::styled(format!("Total: {}", count_display), ...),
    Span::raw("  "),
    Span::styled(format!("Debt Score: {:.0}", analysis.total_debt_score), ...),
    Span::raw("  "),
    Span::styled(format!("Density: {:.2}/1K LOC", analysis.debt_density), ...),
])
```

## Objective

Simplify list view header to match progression view's minimal style with lowercase labels, no verbose prefixes, and pipe separators.

**Visual Transformation**:

**Before** (verbose):
```
Debtmap Results  Total: 45  Debt Score: 1234  Density: 12.34/1K LOC
Sort: Unified Score  Filters: 0  Grouping: OFF
```

**After** (minimal):
```
debtmap results  45  │  score 1234  │  12.34/1K loc
sort unified  │  filters 0  │  grouping off
```

**Success Metrics**:
- 50% reduction in header text length
- Consistent with progression view aesthetic
- Maintained information clarity

## Requirements

### Functional Requirements
- All information preserved (no data loss)
- Metrics remain readable
- Status indicators clear

### Visual Requirements
- Lowercase all text ("debtmap results" not "Debtmap Results")
- Remove label prefixes ("45" not "Total: 45")
- Use pipe separators `│` between metrics
- Lowercase boolean states ("on"/"off" not "ON"/"OFF")
- Abbreviate where contextually clear ("loc" for "LOC")

### Scope
- First header line (title, counts, metrics)
- Second header line (sort, filters, grouping status)

## Acceptance Criteria

- [ ] Title changed to "debtmap results" (lowercase)
- [ ] Count display has no "Total:" prefix
- [ ] Score shows as "score N" not "Debt Score: N"
- [ ] Density shows as "N/1K loc" (lowercase loc)
- [ ] Pipe separators `│` between all metrics
- [ ] Sort shows as "sort {criteria}" not "Sort: {criteria}"
- [ ] Filters shows as "filters N" not "Filters: N"
- [ ] Grouping shows as "grouping on/off" not "Grouping: ON/OFF"
- [ ] Manual testing confirms improved minimalism

## Technical Details

### Implementation Approach

**File**: `src/tui/results/list_view.rs:169-214`

**Current header rendering**:
```rust
fn render_header(frame: &mut Frame, app: &ResultsApp, area: Rect, theme: &Theme) {
    let analysis = app.analysis();
    let count_display = app.count_display();

    let header_text = vec![
        Line::from(vec![
            Span::styled("Debtmap Results", Style::default().fg(theme.accent())),
            Span::raw("  "),
            Span::styled(format!("Total: {}", count_display), ...),
            Span::raw("  "),
            Span::styled(format!("Debt Score: {:.0}", analysis.total_debt_score), ...),
            Span::raw("  "),
            Span::styled(format!("Density: {:.2}/1K LOC", analysis.debt_density), ...),
        ]),
        Line::from(vec![
            Span::styled(format!("Sort: {}", app.sort_by().display_name()), ...),
            Span::raw("  "),
            Span::styled(format!("Filters: {}", app.filters().len()), ...),
            Span::raw("  "),
            Span::styled(format!("Grouping: {}", if app.is_grouped() { "ON" } else { "OFF" }), ...),
        ]),
    ];
    // ...
}
```

**Proposed simplified version**:
```rust
fn render_header(frame: &mut Frame, app: &ResultsApp, area: Rect, theme: &Theme) {
    let analysis = app.analysis();
    let count_display = app.count_display();

    let header_text = vec![
        // Line 1: Title and metrics
        Line::from(vec![
            Span::raw("debtmap results"),  // Lowercase, no styling
            Span::raw("  "),
            Span::styled(count_display.to_string(), Style::default().fg(theme.primary)),
            Span::raw("  │  "),  // Pipe separator
            Span::raw("score "),
            Span::styled(format!("{:.0}", analysis.total_debt_score), Style::default().fg(theme.secondary())),
            Span::raw("  │  "),
            Span::styled(format!("{:.2}/1K loc", analysis.debt_density), Style::default().fg(theme.muted)),
        ]),

        // Line 2: Status indicators
        Line::from(vec![
            Span::styled(format!("sort {}", app.sort_by().display_name().to_lowercase()), Style::default().fg(theme.muted)),
            Span::raw("  │  "),
            Span::styled(format!("filters {}", app.filters().len()), Style::default().fg(theme.muted)),
            Span::raw("  │  "),
            Span::styled(
                format!("grouping {}", if app.is_grouped() { "on" } else { "off" }),
                Style::default().fg(theme.muted)
            ),
        ]),
    ];

    let header = Paragraph::new(header_text)
        .block(Block::default().borders(Borders::BOTTOM))
        .style(Style::default());

    frame.render_widget(header, area);
}
```

### Key Changes

**Line 1 transformations**:
- `"Debtmap Results"` → `"debtmap results"` (lowercase, plain text)
- `format!("Total: {}", count)` → `count.to_string()` (no label)
- `format!("Debt Score: {:.0}", score)` → `format!("score {:.0}", score)` (minimal label)
- `format!("Density: {:.2}/1K LOC", ...)` → `format!("{:.2}/1K loc", ...)` (lowercase unit)
- `Span::raw("  ")` → `Span::raw("  │  ")` (pipe separators)

**Line 2 transformations**:
- `format!("Sort: {}", name)` → `format!("sort {}", name.to_lowercase())` (lowercase)
- `format!("Filters: {}", n)` → `format!("filters {}", n)` (lowercase)
- `format!("Grouping: {}", "ON"/"OFF")` → `format!("grouping {}", "on"/"off")` (lowercase)
- Consistent pipe separators

**Sort criteria lowercase**:
The `app.sort_by().display_name()` may return title-case strings like "Unified Score". Apply `.to_lowercase()`:
- `"Unified Score"` → `"unified score"`
- `"Coverage"` → `"coverage"`
- `"Complexity"` → `"complexity"`

### Color Adjustments

**Current**: Heavy use of accent and primary colors

**Proposed**:
- Title: Plain text (no color)
- Count: Primary color (important metric)
- Score: Secondary color (important but less than count)
- Density: Muted (contextual metric)
- All status indicators (line 2): Muted (supporting information)

This creates hierarchy: data pops, labels recede.

### Visual Impact Analysis

**Before** (Current - Verbose):
```
──────────────────────────────────────────────────────────────────
Debtmap Results  Total: 45  Debt Score: 1234  Density: 12.34/1K LOC
Sort: Unified Score  Filters: 0  Grouping: OFF
──────────────────────────────────────────────────────────────────
```

**After** (Proposed - Minimal):
```
──────────────────────────────────────────────────────────────────
debtmap results  45  │  score 1234  │  12.34/1K loc
sort unified score  │  filters 0  │  grouping off
──────────────────────────────────────────────────────────────────
```

**Character count reduction**:
- Line 1: 75 chars → 52 chars (31% reduction)
- Line 2: 42 chars → 45 chars (slight increase due to pipes, but more scannable)

**Aesthetic improvements**:
- Calmer lowercase text
- Clear visual separation with pipes
- Numbers stand out (no label clutter)
- Consistent with progression footer pattern

### Alignment with Progression View

**Progression footer** for comparison:
```rust
format!(
    "functions {}  │  debt {}  │  coverage {:.1}%",
    format_number(app.functions_count),
    app.debt_count,
    app.coverage_percent
)
```

Pattern: `{label} {value}  │  {label} {value}  │  ...`

**List header** (after this spec):
```rust
format!("debtmap results  {}  │  score {}  │  {:.2}/1K loc", count, score, density)
```

Same pattern, same aesthetic.

## Dependencies

**Prerequisites**:
- Spec 229 (Lowercase Severity Labels)
- Spec 230 (Lowercase Section Headers)

**Affected Components**:
- List view header rendering

**External Dependencies**: None

## Testing Strategy

### Manual Testing
- [ ] Launch TUI, verify header displays in lowercase
- [ ] Check pipe separators render correctly (│)
- [ ] Verify all metrics still visible and readable
- [ ] Test with various item counts (0, 1, 100, 1000+)
- [ ] Test with different sort criteria selected
- [ ] Test with filters active (0, 1, multiple)
- [ ] Test with grouping on and off
- [ ] Verify in 80-col and 120-col terminals

### Visual Regression Testing
- [ ] Compare before/after screenshots
- [ ] Verify spacing maintained
- [ ] Check color hierarchy (values prominent, labels muted)
- [ ] Confirm border alignment unchanged

### Edge Cases
- [ ] Very high debt scores (>10000)
- [ ] Very high density values
- [ ] Long sort criteria names
- [ ] Zero items scenario

## Documentation Requirements

### Code Documentation
- Add comment explaining minimal header philosophy

### Design Documentation
- Update DESIGN.md if it includes list header examples

### User Documentation
- No user documentation needed (cosmetic change)

## Implementation Notes

### Sort Criteria Display Names

The `display_name()` method on sort criteria may need review to ensure lowercase friendly:

```rust
// If display_name() returns "Unified Score"
app.sort_by().display_name().to_lowercase()  // "unified score"

// Or update display_name() to return lowercase by default
impl SortCriteria {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::UnifiedScore => "unified score",  // lowercase
            Self::Coverage => "coverage",
            // ...
        }
    }
}
```

Recommend updating `display_name()` to return lowercase by default for consistency.

### Grouping State

Currently uses "ON"/"OFF". Change to "on"/"off":

```rust
// Current
if app.is_grouped() { "ON" } else { "OFF" }

// Proposed
if app.is_grouped() { "on" } else { "off" }
```

Simple string literal change.

### Color Theme Consistency

Ensure color usage matches progression view:
- **Muted** for labels and secondary info
- **Primary** for key metrics
- **Secondary** for contextual metrics
- **Plain text** for titles

```rust
// Title - no color
Span::raw("debtmap results")

// Count - primary
Span::styled(count, theme.primary)

// Score - secondary
Span::styled(score, theme.secondary())

// Density - muted
Span::styled(density, theme.muted)

// Status line - all muted
Span::styled("sort unified score", theme.muted)
```

## Migration and Compatibility

**Breaking Changes**: None (cosmetic output only)

**Backward Compatibility**: Full compatibility

**Migration Path**: None needed

## Related Specifications

- **Spec 229**: Lowercase Severity Labels (foundation)
- **Spec 230**: Lowercase Section Headers (prerequisite)
- **Spec 231**: Dotted Leaders in Detail View (detail view counterpart)
- **Spec 233**: Simplified Footer Shortcuts (planned - footer consistency)

Together these create consistent minimal aesthetic across all TUI views.

## Visual Design Philosophy

From DESIGN.md:

> **Minimize text** - Show only what's necessary
> **Lowercase dominates** - Calm, not aggressive
> **Space as design element** - Breathing room

This spec applies these principles to the list header:

**Removed**:
- Verbose label prefixes ("Total:", "Debt Score:", "Sort:")
- Uppercase styling ("ON", "OFF", "Debtmap Results")
- Excessive spacing (double spaces)

**Added**:
- Pipe separators for clear visual division
- Lowercase calm aesthetic
- Tighter, more scannable layout

**Preserved**:
- All information content
- Clear hierarchy through color
- Readability

Result: Header that respects the user's attention and matches the zen progression view.
