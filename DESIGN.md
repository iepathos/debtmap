# Debtmap TUI Design Document

## Design Philosophy

### Futuristic Zen Minimalism

Debtmap's TUI embodies a "futuristic zen minimalist" aesthetic that balances advanced technical visualization with calm, focused simplicity. The design philosophy rests on three pillars:

1. **Clarity Through Restraint** - Every visual element serves a purpose. No decorative clutter.
2. **Subtle Motion** - Animations guide attention without distraction (60 FPS smooth rendering)
3. **Information Hierarchy** - Important data stands out naturally through color and spacing

### Core Design Principles

- **Space as a Design Element** - Generous whitespace creates breathing room and visual hierarchy
- **Monochromatic with Accent** - Primarily grayscale with cyan and green accents for active/completed states
- **Progressive Disclosure** - Show overview first, reveal details on demand
- **Responsive Adaptation** - Gracefully degrade from rich to minimal as terminal size decreases
- **Smooth Transitions** - 60 FPS frame rate for fluid animations and real-time updates

---

## Color Palette

### Primary Colors

```rust
Theme {
    primary: Cyan,      // Active elements, highlights, interactive components
    success: Green,     // Completed states, positive metrics
    muted: DarkGray,    // Inactive, secondary information, dotted leaders
    text: White,        // Primary content text
    background: Reset,  // Terminal default (transparent/black)
}
```

### Semantic Color Usage

| Element | Color | Purpose |
|---------|-------|---------|
| Active stage markers (▸) | Cyan + Bold | Draw eye to current activity |
| Completed markers (✓) | Green | Positive reinforcement |
| Pending markers (·) | DarkGray | Recede into background |
| Progress bars (▓) | Cyan | Show forward momentum |
| Dotted leaders (···) | DarkGray | Connect without dominating |
| Metrics/stats | DarkGray | Supporting data, not primary |
| Elapsed time | DarkGray | Context without distraction |

### Severity Color Scale

For debt severity visualization:

- **CRITICAL** - `Red` - Immediate attention required (score ≥ 100)
- **HIGH** - `LightRed` - High priority (score ≥ 50)
- **MEDIUM** - `Yellow` - Moderate concern (score ≥ 10)
- **LOW** - `Green` - Minor issues (score < 10)

### Coverage Color Scale

For test coverage visualization:

- **High** (70-100%) - `Green` - Well tested
- **Medium** (30-70%) - `Yellow` - Partial coverage
- **Low** (0-30%) - `Red` - Needs attention
- **None** - `DarkGray` - No data available

---

## Typography & Glyphs

### Status Indicators

Carefully chosen Unicode characters convey state at a glance:

- `✓` - Completed (U+2713) - Universal checkmark
- `▸` - Active (U+25B8) - Forward momentum
- `·` - Pending (U+00B7) - Minimalist placeholder
- `▹` - Animated variant (U+25B9) - Subtle animation frame

### Progress Visualization

- `▓` (U+2593) - Filled progress block - Solid, weighty
- `░` (U+2591) - Empty progress block - Light, potential
- `·` (U+00B7) - Dotted leader - Connector without weight

### Braille Spinner

For subtle loading states (currently in animation controller):

- `⠋ ⠙ ⠹ ⠸` - Braille pattern spinner - Minimal, sophisticated

### Navigation & UI Elements

- `│` - Vertical separator - Clean divisions
- `←→` - Horizontal navigation - Page movement
- `↑↓` - Vertical navigation - List scrolling

---

## Layout Architecture

### Responsive Breakpoints

The TUI adapts to terminal width with four distinct layouts:

| Mode | Width | Features | Use Case |
|------|-------|----------|----------|
| **Full** | >120 cols | All metrics, full hierarchy, sub-tasks | Desktop terminals |
| **Standard** | 80-119 cols | Standard view, sub-tasks visible | Default terminal |
| **Compact** | 40-79 cols | No sub-tasks, compressed metrics | Small windows |
| **Minimal** | <40 cols | Progress bar + stage counter only | Very constrained |

### Standard Layout Structure

```
┌─────────────────────────────────────────────────────┐
│ Header (5 lines)                                    │
│   - Title + elapsed time                            │
│   - Overall progress bar                            │
│   - Stage counter (stage N/9)                       │
├─────────────────────────────────────────────────────┤
│ Main Content (flexible)                             │
│   - Pipeline stages with hierarchical sub-tasks     │
│   - Active stage expands to show progress           │
│   - Generous vertical spacing                       │
├─────────────────────────────────────────────────────┤
│ Footer (3 lines)                                    │
│   - Summary statistics                              │
│   - Thread count, coverage %, debt count            │
└─────────────────────────────────────────────────────┘
```

### Results List View Layout

```
┌─────────────────────────────────────────────────────┐
│ Header (3 lines)                                    │
│   - Title, total items, debt score, density         │
│   - Current sort, active filter count               │
├─────────────────────────────────────────────────────┤
│ Scrollable List (flexible)                          │
│   - Debt items with severity, score, location       │
│   - Selected item highlighted                       │
│   - Coverage and complexity preview                 │
├─────────────────────────────────────────────────────┤
│ Footer (2 lines)                                    │
│   - Position indicator (item N/total)               │
│   - Keyboard shortcuts                              │
└─────────────────────────────────────────────────────┘
```

### Detail View Layout (Multi-Page)

```
┌─────────────────────────────────────────────────────┐
│ Header (3 lines)                                    │
│   - Page indicator [Page N/M]                       │
│   - Item position (N/total)                         │
│   - Current page name                               │
├─────────────────────────────────────────────────────┤
│ Content (flexible, varies by page)                  │
│                                                      │
│   Page 1: Overview - Core metrics, location         │
│   Page 2: Dependencies - Call graph, dependencies   │
│   Page 3: Git Context - Churn, authors, recency     │
│   Page 4: Patterns - Framework patterns, purity     │
│                                                      │
├─────────────────────────────────────────────────────┤
│ Footer (2 lines)                                    │
│   - Page navigation: Tab/←→, 1-4 jump               │
│   - Actions: copy, edit, help, back                 │
└─────────────────────────────────────────────────────┘
```

---

## Animation System

### Frame Rate & Timing

- **Target FPS**: 60 frames per second
- **Frame Duration**: ~16ms per frame
- **Background Thread**: Dedicated render thread for smooth updates
- **Animation Controller**: Frame-based state machine cycling every 10 seconds

### Animation Types

#### 1. Arrow Animation (Sub-task Progress)

Cycles through variants every 3 frames for subtle motion:

```rust
match (frame / 3) % 3 {
    0 => "▸",  // Solid
    1 => "▹",  // Outline
    _ => "▸",  // Back to solid
}
```

**Design Intent**: Subtle pulsing indicates active work without being distracting.

#### 2. Spinner Animation (Loading States)

Braille patterns cycle every 8 frames:

```rust
match (frame / 8) % 4 {
    0 => "⠋",
    1 => "⠙",
    2 => "⠹",
    _ => "⠸",
}
```

**Design Intent**: Sophisticated loading indicator, minimal screen real estate.

#### 3. Pulse Alpha (Future Use)

Sinusoidal pulse for emphasis (0.7-1.0 range):

```rust
(phase * PI * 2.0).sin() * 0.3 + 0.7
```

**Design Intent**: Breathing effect for critical alerts or focus states.

### Animation Guidelines

- **Never obstruct information** - Animations should enhance, not distract
- **Respect terminal capabilities** - Unicode support required for full experience
- **Performance awareness** - 60 FPS is target, degrade gracefully if needed
- **Purposeful motion** - Every animation communicates state or progress

---

## Spacing & Rhythm

### Vertical Spacing Rules

- **Section separation**: 1 blank line between major sections
- **Sub-task indentation**: 4 spaces for hierarchy clarity
- **Header/footer padding**: 1 line margin above/below content
- **Detail page sections**: 1-2 blank lines between logical groups

### Horizontal Spacing

- **Dotted leaders**: Fill remaining width to create visual connection
- **Label-value pairs**: 2-space separation for readability
- **List columns**: Aligned with `format!` width specifiers
- **Margins**: 1-character margin on layout edges

### Visual Weight Distribution

```
Header:    5 lines  (fixed)  - 10-15% of screen
Content:   N lines  (flex)   - 70-80% of screen
Footer:    3 lines  (fixed)  - 5-10% of screen
```

This creates natural eye flow: context → content → actions.

---

## Interactive States

### View Modes

1. **List** - Default browsable list of debt items
2. **Detail** - Multi-page deep dive into selected item
3. **Search** - Overlay search input box
4. **SortMenu** - Modal sort criteria selector
5. **FilterMenu** - Modal filter configuration
6. **Help** - Keyboard shortcut reference

### Selection & Focus

- **Selected list item**: `DarkGray` background, cyan `▸` indicator
- **Active menu option**: Cyan + Bold with `▸` prefix
- **Input focus**: Cyan border on modal overlays
- **Page indicator**: Bold cyan for current page

### Keyboard Navigation Philosophy

- **Vi-style**: `j/k` for vertical navigation (alongside arrows)
- **Single-key actions**: `/, s, f, e, c` for common operations
- **Number shortcuts**: `1-4` for page jumping, `1-9` for sort/filter
- **Escape hierarchy**: `Esc` always exits current mode/overlay
- **Question mark**: `?` universally opens help

---

## Component Design Patterns

### Section Headers

```rust
// ALL CAPS with muted color
Span::styled("SECTION NAME", Style::default().fg(theme.muted))
```

**Rationale**: Uppercase provides visual anchor without bold weight. Muted color prevents overwhelming.

### Label-Value Pairs

```rust
// Label in normal text, value in primary color
Line::from(vec![
    Span::raw("  Label: "),  // 2-space indent
    Span::styled(value, Style::default().fg(theme.primary))
])
```

**Rationale**: Clear hierarchy, values pop as important data.

### Progress Bars

```rust
// Filled blocks in cyan, empty in muted
format!("{}{}", "▓".repeat(filled), "░".repeat(empty))
```

**Rationale**: High contrast between filled/empty, but not jarring. Block characters feel substantial.

### Dotted Leaders

```rust
// Connect label to value with muted dots
format!("{} {} {}", label, "·".repeat(width), value)
```

**Rationale**: Classic typographic technique creates visual connection without lines/borders.

### Modal Overlays

- **Centered positioning**: 25-33% from edges
- **Border style**: All borders with cyan accent
- **Title instructions**: Embedded in border title
- **Semi-transparent effect**: (Not implemented, but background shows through)

---

## Information Architecture

### List View Information Density

Each list item shows in ~80 characters:

```
▸ #1    CRITICAL   Score:142.3   file.rs::function_name   (Cov:45% Comp:12)
```

**Elements (left to right)**:
1. Selection indicator (▸ or space)
2. Item number (#N)
3. Severity level (CRITICAL/HIGH/MEDIUM/LOW)
4. Unified score (primary metric)
5. Location (file::function)
6. Quick metrics (coverage, complexity)

### Detail Page Organization

#### Page 1: Overview
- **Purpose**: Core identity and metrics
- **Content**: Location, unified score, complexity metrics, coverage, recommendation
- **Design**: Clean label-value pairs with clear sections

#### Page 2: Dependencies
- **Purpose**: Relationship and impact
- **Content**: Calls this function, Called by this, transitive dependencies
- **Design**: Hierarchical lists, dependency depth visualization

#### Page 3: Git Context
- **Purpose**: Historical risk factors
- **Content**: Commit frequency, authors, recency, churn risk
- **Design**: Timeline visualization, contributor patterns

#### Page 4: Patterns
- **Purpose**: Code quality signals
- **Content**: Framework patterns, purity analysis, language-specific traits
- **Design**: Tag-based presentation, boolean indicators

### Progressive Detail Strategy

```
List View:    High-level overview, scannable, quick sorting/filtering
  ↓ Enter
Detail View:  Comprehensive data, multi-page, supports deep analysis
  ↓ 'e'
Editor:       Direct manipulation, external tool
```

---

## Accessibility Considerations

### Color Independence

While color enhances the experience, information is never conveyed by color alone:

- Severity includes text label ("CRITICAL") not just color
- Status uses distinct glyphs (✓ ▸ ·) not just color
- Progress shows percentage number alongside bar

### Terminal Compatibility

- **Minimal mode**: Degrades to ASCII-safe characters if needed
- **Unicode fallbacks**: Core functionality works without fancy glyphs
- **No true color**: Uses 16-color ANSI palette for maximum compatibility
- **No cursor hiding**: Respects terminal cursor preferences

### Keyboard Accessibility

- All features accessible via keyboard
- No mouse dependency
- Clear visual focus indicators
- Consistent navigation patterns (Vim-style + arrows)

---

## Performance Characteristics

### Rendering Optimization

- **Background render thread**: Decouples rendering from data updates
- **60 FPS target**: Smooth animations without tearing
- **Lazy evaluation**: Only render visible items in scrollable lists
- **Efficient layout**: Pre-calculated constraints, minimal recomputation

### Memory Footprint

- **Immutable data**: Results loaded once, filtered/sorted via indices
- **String allocation**: Minimized through string slices and borrowing
- **Frame buffer**: Single terminal buffer, double-buffered by ratatui

### Responsiveness

- **Non-blocking input**: Dedicated event polling thread
- **Signal handling**: Ctrl+C/Ctrl+Z handled gracefully
- **Terminal restore**: Guaranteed cleanup via Drop implementation
- **Alternate screen**: Preserves user's terminal history

---

## Future Design Directions

### Potential Enhancements

1. **Trend Visualization**
   - Sparklines for score history
   - Inline mini-graphs for metrics over time
   - Maintain minimalist aesthetic with subtle visualization

2. **Enhanced Search**
   - Fuzzy matching with score-based ranking
   - Highlight matched terms in results
   - Search history with arrow key navigation

3. **Themes**
   - Light mode variant (inverted palette)
   - Customizable accent colors
   - User-defined theme files
   - Maintain core minimalist principles

4. **Smart Truncation**
   - Ellipsize long file paths intelligently
   - Show most relevant path components
   - Hover/expand for full paths (if mouse support added)

5. **Syntax Highlighting**
   - Code snippets in detail view
   - Language-aware highlighting
   - Muted color scheme to fit aesthetic

### Design Constraints to Maintain

- **Never exceed 5 colors** in standard view (primary, success, muted, text, bg)
- **Keep animations under 100ms cycle** time for smoothness
- **Maintain 60 FPS** even on slower hardware
- **Preserve keyboard-only workflow** - mouse is enhancement only
- **No modal dialogs** that block workflow - overlays should be skippable
- **Information density** must not overwhelm - add pages, not density

---

## Implementation Notes

### Technology Stack

- **Framework**: [ratatui](https://github.com/ratatui-org/ratatui) (Rust TUI library)
- **Backend**: CrossTerm for cross-platform terminal control
- **Threading**: Dedicated render thread + event polling thread
- **State Management**: Arc + Mutex for thread-safe shared state

### File Organization

```
src/tui/
├── mod.rs              # TuiManager, lifecycle
├── theme.rs            # Color palette, style definitions
├── layout.rs           # Responsive breakpoints, layout calculation
├── renderer.rs         # Core rendering logic (full/compact/minimal)
├── animation.rs        # Animation controller, frame-based state
├── app.rs              # Progress view application state
└── results/
    ├── mod.rs          # Results TUI exports
    ├── app.rs          # Results application state
    ├── list_view.rs    # List rendering
    ├── detail_view.rs  # Detail routing
    ├── layout.rs       # Results-specific layouts
    ├── navigation.rs   # Keyboard input handling
    ├── search.rs       # Search functionality
    ├── sort.rs         # Sort criteria
    ├── filter.rs       # Filter logic
    └── detail_pages/
        ├── overview.rs      # Page 1: Core metrics
        ├── dependencies.rs  # Page 2: Relationships
        ├── git_context.rs   # Page 3: History
        ├── patterns.rs      # Page 4: Quality signals
        └── components.rs    # Shared rendering helpers
```

### Key Abstractions

- **Theme**: Centralized color and style definitions
- **LayoutMode**: Enum for responsive breakpoints
- **ViewMode**: State machine for different TUI screens
- **DetailPage**: Pagination abstraction for detail view
- **AnimationController**: Frame-based animation state

---

## Design Rationale: Why This Aesthetic?

### Futuristic

- **Clean lines and geometry**: Rectangular blocks, aligned grids
- **Unicode glyphs**: Modern terminal capabilities
- **60 FPS animations**: Smooth, responsive, high-fidelity
- **Cyan accent**: Technical, digital, forward-looking

### Zen

- **Generous whitespace**: Breathing room prevents overwhelm
- **Muted palette**: Calm, not aggressive or flashy
- **Single point of focus**: Active stage/selected item stands out clearly
- **No decoration**: Every element serves information, not aesthetics

### Minimalist

- **5-color palette**: Restrained, intentional color use
- **Simple glyphs**: ✓ ▸ · convey meaning instantly
- **No borders** on list items: Space creates separation
- **Progressive disclosure**: Show summary, reveal detail on demand

### Result

A TUI that feels:
- **Professional**: Serious tool for serious analysis
- **Calm**: Long analysis sessions don't cause fatigue
- **Clear**: Information hierarchy is immediately obvious
- **Modern**: Leverages contemporary terminal capabilities
- **Respectful**: Doesn't fight for attention, serves the user

---

## Design Review Checklist

When adding new TUI features, ensure:

- [ ] Color usage follows 5-color palette (primary, success, muted, text, bg)
- [ ] Spacing creates clear visual hierarchy (sections separated by blank lines)
- [ ] Glyphs are meaningful and consistent with existing set
- [ ] Animations serve a purpose (indicate state/progress)
- [ ] Layout adapts to terminal width (test all 4 breakpoints)
- [ ] Keyboard shortcuts follow existing patterns (single-key, Vi-style)
- [ ] Information is accessible without color (severity has label + color)
- [ ] Text alignment creates visual order (aligned columns, consistent indentation)
- [ ] Performance maintains 60 FPS (avoid expensive per-frame computation)
- [ ] States are visually distinct (active vs pending vs completed)

---

## Conclusion

Debtmap's TUI design achieves **futuristic zen minimalism** through:

1. **Restrained color palette** - Cyan and green accents on grayscale foundation
2. **Purposeful animation** - 60 FPS smooth motion that guides without distracting
3. **Generous spacing** - Whitespace as a first-class design element
4. **Clear hierarchy** - Information density balanced with readability
5. **Responsive degradation** - Graceful adaptation to any terminal size
6. **Keyboard-first interaction** - Efficient, focused workflow

This design serves the tool's purpose: helping developers quickly identify and understand technical debt without cognitive overload. Every pixel, every color, every animation exists to clarify, not decorate.

The aesthetic is not an end in itself, but a means to better analysis.
