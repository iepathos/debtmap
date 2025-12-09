---
number: 203
title: TUI Coupling Visualization
category: optimization
priority: medium
status: draft
dependencies: [201, 202]
created: 2025-12-09
---

# Specification 203: TUI Coupling Visualization

**Category**: optimization
**Priority**: medium
**Status**: draft
**Dependencies**: [201 - File-Level Dependency Metrics, 202 - Coupling Text Output]

## Context

The TUI currently shows a basic dependencies page (`src/tui/results/detail_pages/dependencies.rs`) with upstream/downstream counts and blast radius. This spec enhances it with:

1. **Coupling classification badges** (visual indicators)
2. **Color-coded instability** (green=stable, red=unstable)
3. **Expanded dependent/dependency lists** (top 5 each)
4. **Visual coupling indicators**

**Current TUI Dependencies Page**:
```
┌─ dependency metrics ─────────────────┐
│ upstream        9                    │
│ downstream      0                    │
│ blast radius    9                    │
│ critical        Yes                  │
└──────────────────────────────────────┘
```

**Desired Enhancement**:
```
┌─ coupling profile ───────────────────┐
│ classification  [STABLE CORE]        │  ← Green badge
│ afferent (Ca)   12                   │
│ efferent (Ce)   8                    │
│ instability     0.40 ████████░░      │  ← Color bar
└──────────────────────────────────────┘
┌─ dependents (who uses this) ─────────┐
│ • main.rs                            │
│ • lib.rs                             │
│ • commands/analyze.rs                │
│ • builders/mod.rs                    │
│ • priority/mod.rs                    │
│   (+7 more)                          │
└──────────────────────────────────────┘
┌─ dependencies (what this uses) ──────┐
│ • std::collections                   │
│ • serde                              │
│ • crate::core                        │
│ • crate::priority                    │
│   (+4 more)                          │
└──────────────────────────────────────┘
```

## Objective

Enhance the TUI dependencies detail page to provide visual coupling analysis with:
1. Classification badge with semantic coloring
2. Instability progress bar with color gradient
3. Scrollable lists of dependents and dependencies
4. Clear visual separation between incoming and outgoing

## Requirements

### Functional Requirements

1. **Coupling Classification Badge**
   - Display classification as a colored badge: `[STABLE CORE]`, `[UTILITY]`, `[LEAF]`, `[ISOLATED]`, `[HIGHLY COUPLED]`
   - Color mapping:
     - Stable Core: Green (healthy)
     - Utility: Blue (normal)
     - Leaf: Yellow (attention)
     - Isolated: Gray (neutral)
     - Highly Coupled: Red (warning)

2. **Instability Visualization**
   - Show instability as a progress bar (0.0 to 1.0)
   - Color gradient: Green (0.0) → Yellow (0.5) → Red (1.0)
   - Format: `0.40 ████████░░░░░░░░░░░░`
   - Width: 20 characters

3. **Dependents List (Incoming)**
   - Section header: "dependents (who uses this)"
   - Show top 5 files that depend on this file
   - Bullet point format with `•` prefix
   - Show "(+N more)" if truncated
   - Scrollable if list is long

4. **Dependencies List (Outgoing)**
   - Section header: "dependencies (what this uses)"
   - Show top 5 modules/files this file imports
   - Same format as dependents
   - Distinguish internal vs external (different styling)

5. **Coupling Metrics Summary**
   - Afferent coupling (Ca) with label
   - Efferent coupling (Ce) with label
   - Total coupling (Ca + Ce)
   - Blast radius (existing, keep for compatibility)

6. **File vs Function Context**
   - For file items: Show file-level coupling (new)
   - For function items: Keep existing upstream/downstream (unchanged)

### Non-Functional Requirements

1. **Visual Clarity**: Information hierarchy clear at a glance
2. **Performance**: No noticeable lag when rendering
3. **Accessibility**: Work in terminals without color support (fallback to text)
4. **Responsiveness**: Adapt to different terminal widths

## Acceptance Criteria

- [ ] Classification badge displays with appropriate color
- [ ] Instability bar renders with color gradient
- [ ] Dependents list shows top 5 with truncation indicator
- [ ] Dependencies list shows top 5 with truncation indicator
- [ ] File items show full coupling profile
- [ ] Function items retain existing dependency display
- [ ] Colors degrade gracefully in no-color terminals
- [ ] Keyboard navigation works for scrollable lists
- [ ] Page renders without layout overflow

## Technical Details

### Implementation Approach

**Update dependencies.rs**:

```rust
use ratatui::{
    style::{Color, Style},
    widgets::{Block, Borders, Gauge, List, ListItem, Paragraph},
};

/// Render coupling classification badge
fn render_classification_badge(classification: &str) -> Span {
    let (text, color) = match classification {
        "stable core" => ("[STABLE CORE]", Color::Green),
        "utility" => ("[UTILITY]", Color::Blue),
        "leaf" => ("[LEAF]", Color::Yellow),
        "isolated" => ("[ISOLATED]", Color::Gray),
        "highly coupled" => ("[HIGHLY COUPLED]", Color::Red),
        _ => ("[UNKNOWN]", Color::White),
    };
    Span::styled(text, Style::default().fg(color).bold())
}

/// Render instability as colored progress bar
fn render_instability_bar(instability: f64, width: u16) -> Gauge {
    let color = instability_color(instability);
    Gauge::default()
        .ratio(instability)
        .gauge_style(Style::default().fg(color))
        .label(format!("{:.2}", instability))
}

fn instability_color(i: f64) -> Color {
    if i < 0.3 {
        Color::Green
    } else if i < 0.7 {
        Color::Yellow
    } else {
        Color::Red
    }
}

/// Render dependents/dependencies list
fn render_dependency_list(
    items: &[String],
    title: &str,
    max_display: usize,
) -> List {
    let mut list_items: Vec<ListItem> = items
        .iter()
        .take(max_display)
        .map(|s| ListItem::new(format!("• {}", s)))
        .collect();

    if items.len() > max_display {
        list_items.push(ListItem::new(format!(
            "  (+{} more)",
            items.len() - max_display
        )));
    }

    List::new(list_items)
        .block(Block::default().title(title).borders(Borders::ALL))
}
```

**Layout Structure**:

```
┌─────────────────────────────────────────┐
│ Coupling Profile (30% height)           │
│   - Classification badge                │
│   - Ca, Ce, Instability bar            │
├─────────────────────────────────────────┤
│ Dependents List (35% height)            │
│   - Scrollable list                     │
├─────────────────────────────────────────┤
│ Dependencies List (35% height)          │
│   - Scrollable list                     │
└─────────────────────────────────────────┘
```

### Data Flow

1. `UnifiedDebtItem` → check if file or function
2. For files: extract `FileDependencies` from item
3. Call classification/color helpers
4. Render three sections with appropriate widgets

### Affected Components

- `src/tui/results/detail_pages/dependencies.rs` - Main changes
- `src/tui/theme.rs` - Add coupling colors
- `src/tui/results/app.rs` - May need state for scrolling

### Color Palette

```rust
// Add to theme.rs
pub fn coupling_classification_color(&self, classification: &str) -> Color {
    match classification {
        "stable core" => Color::Rgb(0, 200, 0),    // Green
        "utility" => Color::Rgb(100, 149, 237),    // Cornflower Blue
        "leaf" => Color::Rgb(255, 200, 0),         // Gold
        "isolated" => Color::Rgb(128, 128, 128),   // Gray
        "highly coupled" => Color::Rgb(255, 69, 0), // Red-Orange
        _ => Color::White,
    }
}

pub fn instability_gradient(&self, instability: f64) -> Color {
    // Interpolate from green (0.0) to red (1.0)
    let r = (255.0 * instability) as u8;
    let g = (255.0 * (1.0 - instability)) as u8;
    Color::Rgb(r, g, 0)
}
```

## Dependencies

- **Prerequisites**:
  - Spec 201 (data structures)
  - Spec 202 (classification logic can be shared)
- **Affected Components**: TUI detail pages, theme
- **External Dependencies**: ratatui (already used)

## Testing Strategy

- **Unit Tests**: Test color selection, badge rendering
- **Visual Tests**: Manual inspection of TUI rendering
- **Edge Cases**: Empty lists, zero coupling, max instability

## Documentation Requirements

- **User Documentation**: Document new TUI page features
- **Screenshots**: Update any TUI documentation with new visuals

## Implementation Notes

1. Keep backward compatibility with function-level items
2. Consider adding a "toggle" to switch between simple/detailed view
3. Test with both light and dark terminal themes
4. Handle terminals that don't support RGB colors (fallback to basic 16 colors)

## Migration and Compatibility

- Non-breaking change to TUI
- Existing keyboard shortcuts preserved
- Falls back gracefully if coupling data unavailable
