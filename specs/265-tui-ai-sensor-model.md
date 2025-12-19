---
number: 265
title: TUI Updates for AI Sensor Model
category: foundation
priority: high
status: draft
dependencies: [262, 263]
created: 2024-12-19
---

# Specification 265: TUI Updates for AI Sensor Model

**Category**: foundation
**Priority**: high
**Status**: draft
**Dependencies**: [262 - Remove Recommendation Engine, 263 - Context Window Suggestions]

## Context

The TUI currently displays recommendation sections that will be removed per spec 262. Additionally, the TUI should surface the new context suggestions (spec 263) and present data in a way that supports the "AI sensor" model - showing raw signals and metrics rather than interpreted recommendations.

The TUI serves two purposes in the new model:
1. **Human exploration** - Developers browsing debt items to understand their codebase
2. **AI agent debugging** - Understanding what signals debtmap is providing

**Design Reference**: All TUI changes must follow `DESIGN.md` - the "Futuristic Zen Minimalist" aesthetic with:
- 5-color palette (cyan/green/darkgray/white/bg)
- Lowercase section headers in muted color
- Label-value pairs with 20-char fixed columns
- Generous whitespace, no decorative borders
- Progressive disclosure

## Objective

Update the TUI to:
1. Remove recommendation display sections
2. Add context suggestion display
3. Emphasize raw metrics and signals over interpretations
4. Support the "AI sensor" mental model

## Requirements

### Functional Requirements

#### Remove Recommendation Display

1. **Overview Page** (`src/tui/results/detail_pages/overview.rs`):
   - Remove `build_recommendation_section()` function
   - Remove call to it in `render()` function
   - Remove related tests

2. **Text Extraction** (`src/tui/results/actions/text_extraction.rs`):
   - Remove recommendation section from clipboard copy

3. **Data Structures**:
   - Remove or make optional `recommendation` field from `UnifiedDebtItem` display

#### Add Context Suggestions Display

1. **New Context Page** (Page 8 or replace Recommendations on Page 1):
   - Display primary scope with file:line range
   - List related context with relationships
   - Show total lines and completeness confidence
   - Provide copy-to-clipboard for file ranges

2. **Context Section Format** (following DESIGN.md - no borders, lowercase headers):
   ```
   context to read                    168 lines · 85% confidence

   primary
   src/analyzers/purity_detector.rs:10-85
       PurityDetector::analyze

   related
   Caller    src/extraction/extractor.rs:234-267
             extract_purity
   Test      src/analyzers/purity_detector.rs:1500-1600
             test_purity_detection
   Module    src/analyzers/purity_detector.rs:1-9
             imports and constants
   ```

   **Design Notes** (per DESIGN.md):
   - Section headers ("context to read", "primary", "related") in DarkGray, lowercase
   - Values in Cyan (primary color)
   - 4-space indentation for hierarchy
   - No box-drawing characters or decorative borders

3. **Keyboard Shortcuts**:
   - `c` - Copy all context file ranges to clipboard
   - `p` - Copy primary scope range
   - `Enter` on context item - Copy that specific range

#### Enhance Metrics Display

1. **Overview Page Updates**:
   - More prominent display of complexity breakdown
   - Show all scoring factors (not just final score)
   - Display entropy and dampening clearly
   - Show purity analysis prominently

2. **Score Breakdown Page** (Page 2):
   - Already shows detailed breakdown - ensure it's complete
   - Add any missing factors from new scoring model

3. **New Signals Section** (following DESIGN.md label-value pattern):
   ```
   complexity
   cyclomatic              233 (dampened: 183, factor: 0.5)
   cognitive               366
   nesting                 5 levels
   entropy                 0.44 (low variety)

   coverage
   direct                  78%
   transitive              65%
   uncovered lines         506

   coupling
   upstream                33 callers
   downstream              20 callees
   instability             0.38

   patterns
   type                    god_object (92% confidence)
   responsibilities        10 detected
   cohesion                0.28 (low)

   purity
   classification          Impure (95% confidence)
   side effects            mutable ref, HashMap mutation
   ```

   **Design Notes** (per DESIGN.md):
   - Section headers lowercase in DarkGray
   - Labels left-aligned in 20-char column (white text)
   - 4-char gap between label and value
   - Values in primary color (Cyan) starting at column 24
   - Blank line between sections for visual separation

#### Update Page Navigation

Current pages:
1. Overview (metrics + recommendations)
2. Score Breakdown
3. Dependencies
4. Git Context
5. Patterns
6. Data Flow
7. Responsibilities

New pages:
1. Overview (metrics only - no recommendations)
2. Score Breakdown (unchanged)
3. Context (NEW - context suggestions)
4. Dependencies (unchanged)
5. Git Context (unchanged)
6. Patterns (unchanged)
7. Data Flow (unchanged)
8. Responsibilities (unchanged)

Or alternatively, add Context as a section within Overview page.

### Non-Functional Requirements

- TUI performance unchanged (60 FPS target per DESIGN.md)
- Keyboard navigation unchanged (Vi-style + arrows)
- Color scheme: Use existing 5-color palette only (primary/success/muted/text/bg)
- Responsive to terminal size (graceful degradation per DESIGN.md breakpoints)

### DESIGN.md Compliance

All new TUI elements must follow DESIGN.md "Futuristic Zen Minimalist" aesthetic:
- **No decorative borders** - Use whitespace for separation
- **Lowercase section headers** - In muted (DarkGray) color
- **Fixed-column label-value pairs** - 20-char label, 4-char gap, value at column 24
- **Restrained color use** - Primary (Cyan) for important values, muted for secondary
- **Generous whitespace** - Blank lines between sections
- **Pure rendering functions** - Follow Stillwater pattern (build_* returns Lines, render() displays them)

## Acceptance Criteria

### Functional
- [ ] Recommendation section removed from Overview page
- [ ] Recommendation section removed from text extraction
- [ ] Context suggestions displayed on new Context page (or section)
- [ ] All raw signals prominently displayed
- [ ] Scoring breakdown complete with all factors
- [ ] Keyboard shortcuts for copying context ranges
- [ ] Page navigation updated for new structure

### DESIGN.md Compliance
- [ ] Section headers are lowercase with muted (DarkGray) color
- [ ] Label-value pairs use 20-char fixed column format
- [ ] No box-drawing borders on new sections
- [ ] Whitespace used for visual separation
- [ ] Only 5-color palette used (primary/success/muted/text/bg)
- [ ] Pure rendering functions (build_* pattern)

### Quality
- [ ] All existing TUI tests updated
- [ ] New tests for context display
- [ ] `cargo test` passes
- [ ] `cargo clippy` passes
- [ ] Manual visual inspection passes DESIGN.md checklist

## Technical Details

### Implementation Approach

**Phase 1: Remove Recommendations**
1. Delete `build_recommendation_section()` from overview.rs
2. Remove call from `render()` function
3. Update text extraction to skip recommendations
4. Update tests

**Phase 2: Add Context Display**
1. Create new `context.rs` in `src/tui/results/detail_pages/`
2. Implement context rendering with file ranges
3. Add to page navigation
4. Implement clipboard copy for ranges

**Phase 3: Enhance Metrics Display**
1. Update overview.rs to show all signals
2. Ensure score breakdown is complete
3. Add missing metrics if any

### Files to Modify

```
src/tui/results/detail_pages/
├── mod.rs              # Add context page, update page enum
├── overview.rs         # Remove recommendations, enhance metrics
├── context.rs          # NEW: Context suggestions display
└── score_breakdown.rs  # Ensure complete

src/tui/results/
├── list_view.rs        # May need updates for new columns
└── actions/
    └── text_extraction.rs  # Remove recommendation extraction
```

### Context Page Implementation

Following DESIGN.md component patterns (see `components.rs` for shared helpers):

```rust
// src/tui/results/detail_pages/context.rs
use crate::tui::results::detail_pages::components::{
    INDENT, LABEL_WIDTH, GAP, build_section_header, build_label_value_line
};

/// Pure function: builds context lines from data (Stillwater pattern)
pub fn build_context_lines(
    item: &UnifiedDebtItem,
    theme: &Theme,
    width: u16,
) -> Vec<Line<'static>> {
    let mut lines = Vec::new();

    // Section header - lowercase, muted color (per DESIGN.md)
    lines.push(build_section_header(
        "context to read",
        Some(&format!(
            "{} lines · {}% confidence",
            item.context.total_lines,
            (item.context.completeness_confidence * 100.0) as u32
        )),
        theme,
    ));
    lines.push(Line::default()); // Spacing

    // Primary subsection
    lines.push(build_section_header("primary", None, theme));
    let primary = &item.context.primary;
    lines.push(Line::from(Span::styled(
        format!("{}:{}-{}", primary.file.display(), primary.start_line, primary.end_line),
        Style::default().fg(theme.primary),
    )));
    if let Some(symbol) = &primary.symbol {
        lines.push(Line::from(Span::styled(
            format!("    {}", symbol),
            Style::default().fg(theme.muted),
        )));
    }
    lines.push(Line::default()); // Spacing

    // Related subsection
    if !item.context.related.is_empty() {
        lines.push(build_section_header("related", None, theme));
        for related in &item.context.related {
            // Label-value format per DESIGN.md (20-char label column)
            lines.push(build_label_value_line(
                &format_relationship(&related.relationship),
                &format!(
                    "{}:{}-{}",
                    related.range.file.display(),
                    related.range.start_line,
                    related.range.end_line
                ),
                theme,
            ));
            // Indented reason
            lines.push(Line::from(Span::styled(
                format!("{:width$}{}", "", related.reason, width = LABEL_WIDTH + GAP),
                Style::default().fg(theme.muted),
            )));
        }
    }

    lines
}

fn format_relationship(rel: &ContextRelationship) -> &'static str {
    match rel {
        ContextRelationship::Caller => "Caller",
        ContextRelationship::Callee => "Callee",
        ContextRelationship::TypeDefinition => "Type",
        ContextRelationship::TestCode => "Test",
        ContextRelationship::SiblingMethod => "Sibling",
        ContextRelationship::TraitDefinition => "Trait",
        ContextRelationship::ModuleHeader => "Module",
    }
}
```

**Key DESIGN.md compliance**:
- Pure rendering function (no side effects)
- Uses shared `components.rs` helpers for consistent label-value formatting
- Section headers lowercase with muted color
- Values in primary color (Cyan)
- Proper spacing and indentation

### Keyboard Shortcuts

| Key | Action | Page |
|-----|--------|------|
| `c` | Copy all context ranges | Context |
| `p` | Copy primary range | Context |
| `Enter` | Copy selected range | Context |
| `1-8` | Navigate to page | All |
| `Tab` | Next page | All |

### Data Flow

```
UnifiedDebtItem (with context from spec 263)
    ↓
TUI State
    ↓
Page Router (based on current page)
    ↓
├── OverviewPage::render() - metrics, no recommendations
├── ScoreBreakdownPage::render() - unchanged
├── ContextPage::render() - NEW
├── DependenciesPage::render() - unchanged
└── ... other pages
```

## Dependencies

- **Prerequisites**:
  - [262 - Remove Recommendation Engine] (recommendation data gone)
  - [263 - Context Window Suggestions] (context data available)
- **Affected Components**: TUI only
- **External Dependencies**: None

## Testing Strategy

- **Unit Tests**: Each page renders correctly
- **Integration Tests**: Page navigation works
- **Manual Tests**: Visual inspection of all pages
- **User Acceptance**: Information is clear and useful

## Documentation Requirements

- **Code Documentation**: Page documentation updated
- **User Documentation**: TUI navigation guide updated
- **Architecture Updates**: TUI page structure documented

## Implementation Notes

### Graceful Degradation

If `context` field is missing (old data), show placeholder:
```
Context information not available.
Re-run analysis to generate context suggestions.
```

### Terminal Size Handling

Context display should wrap gracefully on narrow terminals:
- Truncate file paths with `...` prefix
- Collapse related items if too many
- Show "more items..." indicator

### Copy to Clipboard

Use existing clipboard infrastructure from text_extraction.rs:
```rust
// Format for AI consumption
fn format_context_for_clipboard(context: &ContextSuggestion) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "Primary: {}:{}-{}\n",
        context.primary.file.display(),
        context.primary.start_line,
        context.primary.end_line
    ));
    for rel in &context.related {
        out.push_str(&format!(
            "Related ({}): {}:{}-{}\n",
            format_relationship(&rel.relationship),
            rel.range.file.display(),
            rel.range.start_line,
            rel.range.end_line
        ));
    }
    out
}
```

## Migration and Compatibility

- No breaking changes to TUI interface
- Page numbers may shift (if Context becomes Page 3)
- Existing keyboard shortcuts preserved
- New shortcuts added for context operations
