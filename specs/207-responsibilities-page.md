---
number: 207
title: Move Responsibilities to Dedicated TUI Page
category: optimization
priority: medium
status: draft
dependencies: []
created: 2025-12-08
---

# Specification 207: Move Responsibilities to Dedicated TUI Page

**Category**: optimization
**Priority**: medium
**Status**: draft
**Dependencies**: None

## Context

The TUI detail view currently has 5 pages:
1. Overview (score, metrics, recommendation)
2. Dependencies (callers, callees, blast radius, coupling profile, responsibilities)
3. Git Context (history, risk, dampening)
4. Patterns (purity, frameworks, language features)
5. Data Flow (mutations, I/O operations, escape analysis)

The Dependencies page (page 2) currently includes the responsibilities section (lines 71-146 in `src/tui/results/detail_pages/dependencies.rs`). This section displays:
- For god objects: All responsibilities with method counts
- For regular functions: Single responsibility category

The dependencies functionality is being expanded (spec 205 DSM, spec 203 coupling visualization), and space is needed. Moving responsibilities to its own page provides:
- More room for enhanced dependency visualizations
- Dedicated space for responsibility analysis
- Better information architecture (single responsibility per page)
- Room for future responsibility enhancements (e.g., responsibility cohesion metrics)

## Objective

Create a new "Responsibilities" page (page 6) in the TUI detail view and remove the responsibilities section from the Dependencies page.

## Requirements

### Functional Requirements

1. **Create new DetailPage variant**
   - Add `Responsibilities` variant to `DetailPage` enum in `src/tui/results/app.rs`
   - Update `next()`, `prev()`, `from_index()`, `index()`, and `name()` methods
   - Update page count from 5 to 6

2. **Create responsibilities page module**
   - Create `src/tui/results/detail_pages/responsibilities.rs`
   - Move responsibility rendering logic from dependencies.rs
   - Display god object responsibilities with method counts
   - Display single responsibility category for regular functions
   - Include explanatory notes for god objects

3. **Remove responsibilities from dependencies page**
   - Remove lines 71-146 from `src/tui/results/detail_pages/dependencies.rs`
   - Keep the god object note about zero deps (if still relevant)

4. **Wire up the new page**
   - Add `pub mod responsibilities;` to `detail_pages/mod.rs`
   - Update `detail_view.rs` to route to new page
   - Update page documentation comment in `mod.rs`

### Non-Functional Requirements

1. **Consistency**
   - Follow existing page module patterns (overview.rs, patterns.rs)
   - Use existing component helpers (add_section_header, add_label_value)
   - Maintain theme consistency

2. **Navigation**
   - Page accessible via Tab/←→ navigation
   - Page accessible via number key (6)
   - Proper wrapping at boundaries

## Acceptance Criteria

- [ ] New `DetailPage::Responsibilities` variant exists
- [ ] New `responsibilities.rs` module renders responsibility information
- [ ] Dependencies page no longer shows responsibilities section
- [ ] Navigation works correctly (Tab, arrows, number keys 1-6)
- [ ] Page indicator shows "6/6" on responsibilities page
- [ ] All existing TUI tests pass
- [ ] No clippy warnings

## Technical Details

### Implementation Approach

**Phase 1: Add DetailPage variant**

In `src/tui/results/app.rs`:

```rust
pub enum DetailPage {
    Overview,
    Dependencies,
    GitContext,
    Patterns,
    DataFlow,
    Responsibilities,  // New
}

impl DetailPage {
    pub fn next(self) -> Self {
        match self {
            DetailPage::Overview => DetailPage::Dependencies,
            DetailPage::Dependencies => DetailPage::GitContext,
            DetailPage::GitContext => DetailPage::Patterns,
            DetailPage::Patterns => DetailPage::DataFlow,
            DetailPage::DataFlow => DetailPage::Responsibilities,
            DetailPage::Responsibilities => DetailPage::Overview,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            DetailPage::Overview => DetailPage::Responsibilities,
            DetailPage::Dependencies => DetailPage::Overview,
            DetailPage::GitContext => DetailPage::Dependencies,
            DetailPage::Patterns => DetailPage::GitContext,
            DetailPage::DataFlow => DetailPage::Patterns,
            DetailPage::Responsibilities => DetailPage::DataFlow,
        }
    }

    pub fn from_index(idx: usize) -> Option<Self> {
        match idx {
            0 => Some(DetailPage::Overview),
            1 => Some(DetailPage::Dependencies),
            2 => Some(DetailPage::GitContext),
            3 => Some(DetailPage::Patterns),
            4 => Some(DetailPage::DataFlow),
            5 => Some(DetailPage::Responsibilities),
            _ => None,
        }
    }

    pub fn index(self) -> usize {
        match self {
            DetailPage::Overview => 0,
            DetailPage::Dependencies => 1,
            DetailPage::GitContext => 2,
            DetailPage::Patterns => 3,
            DetailPage::DataFlow => 4,
            DetailPage::Responsibilities => 5,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            DetailPage::Overview => "Overview",
            DetailPage::Dependencies => "Dependencies",
            DetailPage::GitContext => "Git Context",
            DetailPage::Patterns => "Patterns",
            DetailPage::DataFlow => "Data Flow",
            DetailPage::Responsibilities => "Responsibilities",
        }
    }
}
```

**Phase 2: Create responsibilities.rs**

```rust
//! Responsibilities page (Page 6) - Role and responsibility analysis.
//!
//! This page displays:
//! - God object responsibilities with method counts
//! - Single responsibility category for regular functions
//! - Responsibility-related notes and guidance

use super::components::{add_label_value, add_section_header};
use crate::priority::UnifiedDebtItem;
use crate::tui::theme::Theme;
use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

/// Render responsibilities page
pub fn render(
    frame: &mut Frame,
    item: &UnifiedDebtItem,
    area: Rect,
    theme: &Theme,
) {
    let mut lines = Vec::new();

    // Check for god object responsibilities first
    let god_object_shown = render_god_object_responsibilities(&mut lines, item, theme, area.width);

    // Fall back to single responsibility category
    if !god_object_shown {
        render_single_responsibility(&mut lines, item, theme, area.width);
    }

    // Add explanatory note for god objects
    if let Some(indicators) = &item.god_object_indicators {
        if indicators.is_god_object {
            render_god_object_note(&mut lines, theme);
        }
    }

    let paragraph = Paragraph::new(lines)
        .block(Block::default().borders(Borders::NONE))
        .wrap(Wrap { trim: false });

    frame.render_widget(paragraph, area);
}

fn render_god_object_responsibilities(
    lines: &mut Vec<Line<'static>>,
    item: &UnifiedDebtItem,
    theme: &Theme,
    width: u16,
) -> bool {
    let Some(indicators) = &item.god_object_indicators else {
        return false;
    };

    if !indicators.is_god_object || indicators.responsibilities.is_empty() {
        return false;
    }

    add_section_header(lines, "responsibilities", theme);

    for resp in indicators.responsibilities.iter() {
        let method_count = indicators
            .responsibility_method_counts
            .get(resp)
            .copied()
            .unwrap_or(0);

        let resp_text = resp.to_lowercase();
        let count_text = if method_count > 0 {
            format!("{} methods", method_count)
        } else {
            String::new()
        };

        add_label_value(lines, &resp_text, count_text, theme, width);
    }

    true
}

fn render_single_responsibility(
    lines: &mut Vec<Line<'static>>,
    item: &UnifiedDebtItem,
    theme: &Theme,
    width: u16,
) {
    if let Some(ref category) = item.responsibility_category {
        add_section_header(lines, "responsibility", theme);
        add_label_value(
            lines,
            "category",
            category.to_lowercase(),
            theme,
            width,
        );
    }
}

fn render_god_object_note(lines: &mut Vec<Line<'static>>, theme: &Theme) {
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("Note: ", Style::default().fg(theme.primary)),
        Span::styled(
            "God objects are structural issues (too many",
            Style::default().fg(theme.muted),
        ),
    ]));
    lines.push(Line::from(vec![Span::styled(
        "responsibilities). Focus on splitting by responsibility.",
        Style::default().fg(theme.muted),
    )]));
}
```

**Phase 3: Update detail_view.rs routing**

Add match arm for `Responsibilities`:

```rust
DetailPage::Responsibilities => {
    detail_pages::responsibilities::render(frame, item, content_area, &theme)
}
```

**Phase 4: Update page_count()**

Find and update `page_count()` method to return 6.

**Phase 5: Clean up dependencies.rs**

Remove responsibility section (lines 71-146), keeping only the god object zero-deps note at lines 139-145 if still relevant for context.

### Architecture Changes

- New file: `src/tui/results/detail_pages/responsibilities.rs`
- Modified: `src/tui/results/app.rs` (DetailPage enum)
- Modified: `src/tui/results/detail_pages/mod.rs` (add module)
- Modified: `src/tui/results/detail_view.rs` (add routing)
- Modified: `src/tui/results/detail_pages/dependencies.rs` (remove responsibilities)

## Dependencies

- **Prerequisites**: None
- **Affected Components**:
  - `src/tui/results/app.rs` - DetailPage enum extension
  - `src/tui/results/detail_pages/` - New module and cleanup
  - `src/tui/results/detail_view.rs` - Page routing
- **External Dependencies**: None

## Testing Strategy

### Unit Tests

- Verify `DetailPage::from_index(5)` returns `Responsibilities`
- Verify `DetailPage::Responsibilities.index()` returns 5
- Verify `DetailPage::Responsibilities.name()` returns "Responsibilities"
- Verify navigation wrapping at new boundaries

### Integration Tests

- Existing TUI integration tests should continue to pass
- Manual verification of page navigation (Tab, arrows, number keys)
- Visual verification of responsibilities display

## Documentation Requirements

### Code Documentation

- Module-level doc comments in responsibilities.rs
- Function documentation for render helpers

### User Documentation

- Update page navigation comments in mod.rs (5 pages → 6 pages)

## Implementation Notes

### Function Signature

Note that the responsibilities page render function takes fewer parameters than other pages (no app reference needed) since all data comes from the item itself. This follows the existing pattern where simpler pages have simpler signatures.

### Refactoring Opportunity

The god object note about "zero deps" currently in dependencies.rs can be removed entirely since the responsibilities page will have its own focused note. The dependencies page should focus purely on dependency/coupling concerns.

## Migration and Compatibility

### Breaking Changes

**None** - This is an internal UI reorganization. No API changes.

### User-Facing Changes

- Users will see 6 pages instead of 5
- Responsibilities moved from page 2 to page 6
- Page number key 6 now available for direct access
