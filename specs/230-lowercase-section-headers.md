---
number: 230
title: Lowercase Section Headers with Muted Styling
category: optimization
priority: medium
status: draft
dependencies: [229]
created: 2025-12-06
---

# Specification 230: Lowercase Section Headers with Muted Styling

**Category**: optimization
**Priority**: medium
**Status**: draft
**Dependencies**: Spec 229 (Lowercase Severity Labels)

## Context

Detail view section headers currently use ALL CAPS with bold accent color styling:

```
LOCATION
  File: src/analysis/complexity.rs
  Function: calculate_cognitive_complexity
  Line: 142

SCORE
  Total: 142.3  [CRITICAL]

METRICS
  Cyclomatic Complexity: 15
  Cognitive Complexity: 22
```

This is implemented in `src/tui/results/detail_pages/components.rs:10-17`:

```rust
pub fn add_section_header(lines: &mut Vec<Line<'static>>, title: &str, theme: &Theme) {
    lines.push(Line::from(vec![Span::styled(
        title.to_uppercase(),  // <-- Forces uppercase
        Style::default()
            .fg(theme.accent())      // <-- Cyan accent color
            .add_modifier(Modifier::BOLD),  // <-- Bold weight
    )]));
}
```

**Design Issues**:
1. **ALL CAPS**: Feels aggressive, conflicts with zen minimalism
2. **Bold + Accent Color**: Too much visual weight for supporting information
3. **Inconsistent with progression view**: Which uses muted lowercase labels

From DESIGN.md Section Header pattern:

> ```rust
> // ALL CAPS with muted color
> Span::styled("SECTION NAME", Style::default().fg(theme.muted))
> ```
>
> **Rationale**: Uppercase provides visual anchor without bold weight. Muted color prevents overwhelming.

The design document suggests muted color but still uppercase - however, comparing to the progression view reveals that lowercase with muted color is the true zen aesthetic:

**Progression view example**:
```
debtmap  12.3s
stage 3/9
functions 10,234  │  debt 45  │  coverage 72.4%
```

No section headers use uppercase. Labels are minimal and muted.

## Objective

Convert section headers from uppercase-bold-accent to lowercase-regular-muted to align with zen minimalist philosophy and progression view aesthetic.

**Visual Transformation**:

**Before** (bold cyan uppercase):
```
LOCATION
  File: src/analysis/complexity.rs

METRICS
  Cyclomatic Complexity: 15
```

**After** (regular muted lowercase):
```
location
  File: src/analysis/complexity.rs

metrics
  Cyclomatic Complexity: 15
```

**Success Metrics**:
- Reduced visual weight on section headers
- Improved consistency with progression view
- Maintained clear section boundaries through spacing

## Requirements

### Functional Requirements
- Section headers remain clearly identifiable
- Spacing and hierarchy preserved
- Information content unchanged

### Visual Requirements
- Remove `.to_uppercase()` transformation
- Change color from `theme.accent()` (cyan) to `theme.muted` (dark gray)
- Remove `.add_modifier(Modifier::BOLD)`
- Maintain blank line spacing after headers

### Scope
All detail page section headers:
- **Overview page** (spec file: `overview.rs`): location, score, metrics, entropy, coverage, recommendation, debt type/types
- **Dependencies page**: calls this function, called by this, transitive dependencies
- **Git Context page**: commit history, authors, churn analysis
- **Patterns page**: framework patterns, purity analysis, language traits

## Acceptance Criteria

- [ ] `add_section_header()` no longer uppercases titles
- [ ] Section headers use muted color instead of accent
- [ ] Bold modifier removed from section headers
- [ ] All detail pages render lowercase muted headers
- [ ] Visual hierarchy maintained through spacing
- [ ] Headers clearly distinguish sections despite lower weight
- [ ] Manual testing confirms improved zen aesthetic

## Technical Details

### Implementation Approach

**Single function modification**: `src/tui/results/detail_pages/components.rs:10-17`

```rust
// Current (uppercase, bold, accent)
pub fn add_section_header(lines: &mut Vec<Line<'static>>, title: &str, theme: &Theme) {
    lines.push(Line::from(vec![Span::styled(
        title.to_uppercase(),  // REMOVE THIS
        Style::default()
            .fg(theme.accent())           // CHANGE TO theme.muted
            .add_modifier(Modifier::BOLD),  // REMOVE THIS
    )]));
}

// Proposed (lowercase, regular, muted)
pub fn add_section_header(lines: &mut Vec<Line<'static>>, title: &str, theme: &Theme) {
    lines.push(Line::from(vec![Span::styled(
        title,  // Use title as-is, don't transform
        Style::default()
            .fg(theme.muted),  // Muted color recedes
    )]));
}
```

**Update all call sites** to pass lowercase strings:

Example from `overview.rs:26-40`:
```rust
// Current
add_section_header(&mut lines, "LOCATION", theme);
add_section_header(&mut lines, "SCORE", theme);
add_section_header(&mut lines, "METRICS", theme);

// Proposed
add_section_header(&mut lines, "location", theme);
add_section_header(&mut lines, "score", theme);
add_section_header(&mut lines, "metrics", theme);
```

### Architecture Changes

**Files Modified**:
1. `src/tui/results/detail_pages/components.rs`
   - `add_section_header()` function signature unchanged
   - Implementation simplified (no uppercase, no bold)

2. `src/tui/results/detail_pages/overview.rs`
   - All `add_section_header()` call sites (7 locations)

3. `src/tui/results/detail_pages/dependencies.rs` (if exists)
   - All `add_section_header()` call sites

4. `src/tui/results/detail_pages/git_context.rs` (if exists)
   - All `add_section_header()` call sites

5. `src/tui/results/detail_pages/patterns.rs` (if exists)
   - All `add_section_header()` call sites

**Verification**: Grep for all call sites:
```bash
rg "add_section_header" src/tui/results/detail_pages/
```

### Visual Impact Analysis

**Before** (Current - Bold Cyan Uppercase):
```
┌─────────────────────────────────────────────────────────────┐
│ page 1/4  overview                            item 1/45     │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│ LOCATION                                                     │
│   File: src/analysis/complexity.rs                          │
│   Function: calculate_cognitive_complexity                  │
│                                                              │
│ SCORE                                                        │
│   Total: 142.3  [CRITICAL]                                  │
│                                                              │
│ METRICS                                                      │
│   Cyclomatic Complexity: 15                                 │
│   Cognitive Complexity: 22                                  │
```

**After** (Proposed - Regular Muted Lowercase):
```
┌─────────────────────────────────────────────────────────────┐
│ page 1/4  overview                            item 1/45     │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│ location                                                     │
│   File: src/analysis/complexity.rs                          │
│   Function: calculate_cognitive_complexity                  │
│                                                              │
│ score                                                        │
│   Total: 142.3  [critical]                                  │
│                                                              │
│ metrics                                                      │
│   Cyclomatic Complexity: 15                                 │
│   Cognitive Complexity: 22                                  │
```

**Aesthetic Improvements**:
- Softer, calmer visual hierarchy
- Headers recede into background (muted gray)
- Data values pop more (primary/accent colors)
- Consistent with progression view lowercase style

### Design Philosophy Alignment

From DESIGN.md:

> **Section Headers**
> ```rust
> Span::styled("SECTION NAME", Style::default().fg(theme.muted))
> ```
> **Rationale**: Uppercase provides visual anchor without bold weight. Muted color prevents overwhelming.

After spec 229 (lowercase severity) and this spec, we're evolving the design to:

```rust
Span::styled("section name", Style::default().fg(theme.muted))
```

**Updated Rationale**: Lowercase with muted color creates gentle visual anchor. Whitespace provides structure. Bold and uppercase are unnecessary visual weight.

This aligns with:
- "Clarity Through Restraint" - Remove unnecessary bold/caps
- "Information Hierarchy" - Important data (values) stands out, labels recede
- "Muted palette" - Calm, not aggressive

## Dependencies

**Prerequisites**:
- Spec 229 (Lowercase Severity Labels) - establishes lowercase pattern

**Affected Components**:
- All detail page modules
- Shared components module

**External Dependencies**: None

## Testing Strategy

### Unit Tests
- No unit tests needed (purely cosmetic rendering change)

### Manual Testing
- [ ] Launch TUI and navigate to detail view
- [ ] Verify all section headers are lowercase
- [ ] Confirm muted gray color (not cyan)
- [ ] Verify no bold styling
- [ ] Test all 4 detail pages (overview, dependencies, git, patterns)
- [ ] Confirm section boundaries still clear with whitespace
- [ ] Test in 80-col and 120-col terminals

### Visual Regression Testing
- [ ] Compare before/after screenshots of each detail page
- [ ] Verify spacing unchanged
- [ ] Confirm data values still prominent
- [ ] Check readability and scannability

### Accessibility Testing
- [ ] Section headers still distinguishable without color
- [ ] Whitespace creates clear section boundaries
- [ ] No reliance on bold for structure

## Documentation Requirements

### Code Documentation
- Update DESIGN.md section on "Component Design Patterns"
- Revise "Section Headers" example to show lowercase + muted

### User Documentation
- No user-facing documentation needed

### Architecture Documentation
- Document the evolution from uppercase-bold to lowercase-muted
- Explain zen minimalist rationale

## Implementation Notes

### Call Site Updates

**Search pattern**: `add_section_header(&mut lines, "`

**Replacement strategy**: Convert each uppercase string to lowercase

Example transformations:
- `"LOCATION"` → `"location"`
- `"SCORE"` → `"score"`
- `"METRICS"` → `"metrics"`
- `"ENTROPY"` → `"entropy"`
- `"COVERAGE"` → `"coverage"`
- `"RECOMMENDATION"` → `"recommendation"`
- `"DEBT TYPE"` → `"debt type"`
- `"DEBT TYPES"` → `"debt types"`

### Multi-word Headers

Some headers have multiple words:
- `"DEBT TYPE"` → `"debt type"` (preserve space, lowercase both)
- `"GIT CONTEXT"` → `"git context"`
- `"CALLS THIS FUNCTION"` → `"calls this function"`

Maintain spaces, lowercase all words.

### Verification Commands

```bash
# Find all section header call sites
rg "add_section_header" src/tui/results/detail_pages/

# Verify no uppercase strings passed
rg 'add_section_header.*"[A-Z]' src/tui/results/detail_pages/

# Should return no results after implementation
```

## Migration and Compatibility

**Breaking Changes**: None (cosmetic output only)

**Backward Compatibility**: Full compatibility

**Migration Path**: None needed

## Related Specifications

- **Spec 229**: Lowercase Severity Labels (prerequisite)
- **Spec 231**: Simplified Label Format (planned - reduces verbosity)
- **Spec 232**: Dotted Leaders in Detail View (planned - replaces colons)

Together these specs transform the detail view from "enterprise dashboard" to "zen minimal" aesthetic.

## Visual Design Philosophy

This change embodies the core zen principle: **Less is More**

**Remove**:
- Uppercase transformation (unnecessary shouting)
- Bold modifier (unnecessary weight)
- Accent color (unnecessary prominence)

**Keep**:
- Clear text labels (information preserved)
- Whitespace separation (structure maintained)
- Muted color (gentle hierarchy)

**Result**: Calmer, more focused interface where data stands out and labels recede into the background.

From DESIGN.md conclusion:

> This design serves the tool's purpose: helping developers quickly identify and understand technical debt **without cognitive overload**. Every pixel, every color, every animation exists to clarify, not decorate.

Lowercase muted section headers clarify structure without adding cognitive load through visual weight.
