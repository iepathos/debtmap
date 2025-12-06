---
number: 213
title: Multi-Page TUI Detail View with Contextual Data
category: optimization
priority: high
status: draft
dependencies: []
created: 2025-12-05
---

# Specification 213: Multi-Page TUI Detail View with Contextual Data

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: None

## Context

The current TUI detail view (`src/tui/results/detail_view.rs`) displays only basic information about debt items:
- Location (file, function, line)
- Score and severity
- Basic metrics (cyclomatic, cognitive, nesting, length)
- Coverage percentage
- Recommendation (action + rationale)
- Debt type

However, debtmap's analysis engine collects extensive contextual data that is currently only visible in CLI verbose output (`-v` flag). This missing information is crucial for making informed refactoring decisions:

**Missing from TUI:**
1. **Git Context** - Change frequency, bug density, code age, author count, risk multipliers
2. **Dependencies** - Upstream callers, downstream callees, blast radius, critical path analysis
3. **Pattern Detection** - Purity analysis, framework patterns, Rust-specific patterns, state machine detection
4. **File Context** - Test vs production classification, context dampening multipliers

Users must exit the TUI and re-run with `-v` flag to access this data, breaking the interactive workflow.

**Design Challenge**: Adding all this information to a single page would violate the zen minimalist aesthetic that makes the TUI clean and scannable. Tree characters (`├─`, `└─`) used in CLI verbose output clash with the futuristic, clean design.

## Objective

Implement a multi-page detail view that:

1. **Preserves zen aesthetic** - Each page remains clean, uncluttered, and scannable
2. **Provides complete context** - All analyzed data accessible without leaving TUI
3. **Maintains consistency** - Uses existing design patterns (ALL CAPS headers, 2-space indentation, simple label-value format)
4. **Enables efficient navigation** - Quick keyboard shortcuts to switch pages and navigate between items
5. **Shows only relevant data** - Conditionally displays sections based on available data

**Success Metric**: Users can access all contextual data for any debt item within the TUI in < 5 seconds using intuitive navigation.

## Requirements

### Functional Requirements

#### 1. Page Structure

Implement 4 distinct pages in detail view:

**Page 1: Overview** (current view, minimal changes)
- Location (file, function, line)
- Score and severity classification
- Core metrics (cyclomatic, cognitive, nesting, length)
- Coverage percentage (if available)
- Primary recommendation and rationale
- Debt type classification

**Page 2: Dependencies**
- **Upstream Callers** section
  - List of functions that call this function
  - Show first 10, indicate total count if more
  - Format: `function_name` (truncate if list empty)
- **Downstream Callees** section
  - List of functions this function calls
  - Show first 10, indicate total count if more
  - Format: `function_name` (truncate if list empty)
- **Dependency Metrics** section
  - Upstream dependencies count
  - Downstream dependencies count
  - Blast radius (total affected functions)
  - Critical path indicator (Yes/No)

**Page 3: Git Context**
- **Change Patterns** section
  - Change frequency (changes/month)
  - Stability classification (stable/moderately unstable/highly unstable)
  - Bug density percentage
  - Code age in days
  - Author/contributor count
- **Risk Impact** section
  - Base risk score
  - Contextual risk score (with git history applied)
  - Risk multiplier (contextual/base)
- **Context Dampening** section (if applicable)
  - File type (Test/Example/Benchmark/etc.)
  - Score reduction percentage applied

**Page 4: Patterns**
- **Purity Analysis** section (if available)
  - Pure/impure classification
  - Confidence level
  - Purity level detail
  - Detected effects (I/O, mutation, etc.)
- **Detected Patterns** section (if available)
  - Pattern type (State Machine, Coordinator, etc.)
  - Pattern confidence level
  - Pattern-specific recommendation
- **Framework Patterns** section (if available)
  - Detected framework patterns (Actix, Rocket, etc.)
  - Framework-specific context
- **Language-Specific** section (if available)
  - Rust-specific patterns
  - Language features detected

#### 2. Navigation System

**Page Navigation:**
- `Tab` or `→` (right arrow): Next page (wrap to page 1 from page 4)
- `Shift+Tab` or `←` (left arrow): Previous page (wrap to page 4 from page 1)
- `1`, `2`, `3`, `4`: Jump directly to specific page

**Item Navigation (works on all pages):**
- `n` or `j` or `Down`: Next debt item (preserve current page)
- `p` or `k` or `Up`: Previous debt item (preserve current page)
- `Esc` or `q`: Return to list view

**Actions (available on all pages):**
- `c`: Copy file path to clipboard
- `e` or `o`: Open in editor
- `?`: Show help overlay

#### 3. Visual Design (Zen Aesthetic)

**Page Indicator:**
```
[2/4] Dependencies
```
- Displayed at top of view, right-aligned or centered
- Format: `[current/total] PageName`
- Uses accent color from theme

**Section Headers:**
- ALL CAPS format: `SECTION NAME`
- Accent color (bright blue by default)
- No decorative characters (no trees, boxes, pipes)
- Blank line after header before content

**Content Format:**
- 2-space indentation from section header
- Simple `Label: Value` format for single items
- Subsection headers (not all caps): `Subsection Name (count)`
- List items with 2-space indent, no bullets or decorators

**Truncation Format:**
```
Upstream Callers (12)
  process_request
  handle_workflow
  execute_pipeline
  ... 9 more
```

**Footer:**
```
Tab/←→: Pages | 1-4: Jump | n/p: Items | c: Copy | e: Edit | ?: Help | Esc: Back
```

#### 4. Conditional Display

**Show sections only when data is available:**
- Skip "Upstream Callers" section if list is empty
- Skip "Git Context" page entirely if no contextual risk data
- Skip "Purity Analysis" section if purity data not available
- Skip "Pattern Detection" if no patterns detected
- Display "No data available" message for empty pages

**Adaptive page numbering:**
- If Git Context has no data, show `[2/3]` instead of `[2/4]`
- Update total page count based on available data

#### 5. State Persistence

**Remember last viewed page:**
- When user navigates to detail view, show last viewed page for previous item
- When returning from help overlay, restore current page
- When switching items (n/p), maintain current page number

**Scroll state:**
- If page content exceeds screen height, allow scrolling within page
- Preserve scroll position when switching items on same page

### Non-Functional Requirements

#### 1. Performance
- Page switching should feel instant (< 16ms render time)
- No perceptible lag when navigating between items
- Conditional rendering should not impact performance

#### 2. Consistency
- All pages use identical visual styling
- Navigation keys work identically on all pages
- Footer format consistent across all pages
- Color usage matches existing theme

#### 3. Maintainability
- Each page implemented as separate pure rendering function
- Shared components for common patterns (section headers, lists)
- Clear separation of data access and presentation
- Easy to add new pages in future

#### 4. Accessibility
- All information accessible via keyboard only
- Clear visual indicators of current page
- Obvious navigation hints in footer
- Help overlay documents all shortcuts

## Acceptance Criteria

- [ ] Four pages implemented: Overview, Dependencies, Git Context, Patterns
- [ ] Page navigation works with Tab/Shift+Tab and arrow keys
- [ ] Direct page jumping works with number keys 1-4
- [ ] Item navigation (n/p) preserves current page
- [ ] Page indicator shows current page and total
- [ ] All sections use zen aesthetic (no tree characters)
- [ ] Section headers are ALL CAPS with accent color
- [ ] Content uses 2-space indentation and simple label-value format
- [ ] Lists show first 10 items with "... N more" truncation
- [ ] Empty sections are skipped (not displayed)
- [ ] Pages with no data are omitted from page count
- [ ] Git Context displays change frequency, bug density, age, authors
- [ ] Git Context shows risk impact comparison (base vs contextual)
- [ ] Dependencies page shows upstream callers and downstream callees
- [ ] Dependencies page shows blast radius and critical path indicator
- [ ] Patterns page shows purity analysis with confidence
- [ ] Patterns page shows detected patterns (state machine, etc.)
- [ ] Context dampening shown when applicable (test files, etc.)
- [ ] Footer shows all navigation options concisely
- [ ] Copy to clipboard works from all pages
- [ ] Editor integration works from all pages
- [ ] Help overlay accessible from all pages
- [ ] Esc/q returns to list view from any page
- [ ] Page switching feels instant (< 16ms)
- [ ] No visual artifacts when switching pages
- [ ] Tests verify conditional rendering logic
- [ ] Tests verify navigation state transitions
- [ ] Tests verify data formatting for each section

## Technical Details

### Implementation Approach

#### Phase 1: Core Infrastructure

1. **Add DetailPage enum** (`src/tui/results/app.rs`):
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetailPage {
    Overview,      // Page 1
    Dependencies,  // Page 2
    GitContext,    // Page 3
    Patterns,      // Page 4
}

impl DetailPage {
    fn next(self) -> Self { /* wrap around */ }
    fn prev(self) -> Self { /* wrap around */ }
    fn from_index(idx: usize) -> Option<Self> { /* 0-based */ }
    fn index(self) -> usize { /* 0-based */ }
}
```

2. **Update ResultsApp** (`src/tui/results/app.rs`):
```rust
pub struct ResultsApp {
    // ... existing fields ...
    detail_page: DetailPage,  // Track current page in detail view
}
```

3. **Add helper for available pages**:
```rust
impl ResultsApp {
    /// Get list of available pages based on data
    fn available_pages(&self) -> Vec<DetailPage> {
        let item = self.selected_item();
        // Conditionally include pages based on data availability
    }

    /// Get total page count for current item
    fn page_count(&self) -> usize {
        self.available_pages().len()
    }
}
```

#### Phase 2: Navigation Logic

Update `src/tui/results/navigation.rs`:

```rust
fn handle_detail_key(app: &mut ResultsApp, key: KeyEvent) -> Result<bool> {
    match key.code {
        // Page navigation
        KeyCode::Tab => {
            app.detail_page = app.detail_page.next();
        }
        KeyCode::BackTab => {  // Shift+Tab
            app.detail_page = app.detail_page.prev();
        }
        KeyCode::Right => {
            app.detail_page = app.detail_page.next();
        }
        KeyCode::Left => {
            app.detail_page = app.detail_page.prev();
        }

        // Jump to page
        KeyCode::Char('1') => app.detail_page = DetailPage::Overview,
        KeyCode::Char('2') => app.detail_page = DetailPage::Dependencies,
        KeyCode::Char('3') => app.detail_page = DetailPage::GitContext,
        KeyCode::Char('4') => app.detail_page = DetailPage::Patterns,

        // Item navigation (preserve page)
        KeyCode::Char('n') | KeyCode::Char('j') => {
            move_selection(app, 1);
            // detail_page unchanged
        }

        // ... existing code ...
    }
}
```

#### Phase 3: Page Renderers

Create module structure:
```
src/tui/results/detail_pages/
├── mod.rs           // Module exports
├── overview.rs      // Page 1 (refactored from detail_view.rs)
├── dependencies.rs  // Page 2
├── git_context.rs   // Page 3
└── patterns.rs      // Page 4
```

Each renderer follows pattern:
```rust
pub fn render(
    frame: &mut Frame,
    app: &ResultsApp,
    item: &UnifiedDebtItem,
    area: Rect,
    theme: &Theme,
) {
    let mut lines = Vec::new();

    // Build content lines
    add_section_header(&mut lines, "SECTION NAME", theme);
    add_label_value(&mut lines, "Label", "Value", theme);
    // ... more content ...

    // Render
    let paragraph = Paragraph::new(lines)
        .block(Block::default().borders(Borders::NONE))
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, area);
}
```

#### Phase 4: Shared Components

Create `src/tui/results/detail_pages/components.rs`:

```rust
/// Add ALL CAPS section header
pub fn add_section_header(lines: &mut Vec<Line>, title: &str, theme: &Theme) {
    lines.push(Line::from(vec![Span::styled(
        title.to_uppercase(),
        Style::default()
            .fg(theme.accent())
            .add_modifier(Modifier::BOLD),
    )]));
}

/// Add simple label: value line
pub fn add_label_value(lines: &mut Vec<Line>, label: &str, value: String, theme: &Theme) {
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::raw(format!("{}: ", label)),
        Span::styled(value, Style::default().fg(theme.primary)),
    ]));
}

/// Add list with truncation
pub fn add_list_section(
    lines: &mut Vec<Line>,
    title: &str,
    items: &[String],
    max_display: usize,
    theme: &Theme,
) {
    if items.is_empty() {
        return; // Skip empty lists
    }

    lines.push(Line::from(vec![
        Span::raw(format!("{} ({})", title, items.len())),
    ]));

    let display_count = items.len().min(max_display);
    for item in &items[..display_count] {
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(item, Style::default().fg(theme.secondary())),
        ]));
    }

    if items.len() > max_display {
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(
                format!("... {} more", items.len() - max_display),
                Style::default().fg(theme.muted),
            ),
        ]));
    }

    lines.push(Line::from("")); // Blank line after section
}
```

#### Phase 5: Main Detail View Router

Update `src/tui/results/detail_view.rs`:

```rust
pub fn render(frame: &mut Frame, app: &ResultsApp) {
    let theme = Theme::default();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header with page indicator
            Constraint::Min(0),    // Content
            Constraint::Length(2), // Footer
        ])
        .split(frame.size());

    // Render header with page indicator
    render_header(frame, app, chunks[0], &theme);

    // Route to appropriate page renderer
    if let Some(item) = app.selected_item() {
        match app.detail_page {
            DetailPage::Overview => {
                detail_pages::overview::render(frame, app, item, chunks[1], &theme)
            }
            DetailPage::Dependencies => {
                detail_pages::dependencies::render(frame, app, item, chunks[1], &theme)
            }
            DetailPage::GitContext => {
                detail_pages::git_context::render(frame, app, item, chunks[1], &theme)
            }
            DetailPage::Patterns => {
                detail_pages::patterns::render(frame, app, item, chunks[1], &theme)
            }
        }
    }

    // Render footer
    render_footer(frame, app, chunks[2], &theme);
}

fn render_header(frame: &mut Frame, app: &ResultsApp, area: Rect, theme: &Theme) {
    let current_page = app.detail_page.index() + 1; // 1-based for display
    let total_pages = app.page_count();
    let page_name = match app.detail_page {
        DetailPage::Overview => "Overview",
        DetailPage::Dependencies => "Dependencies",
        DetailPage::GitContext => "Git Context",
        DetailPage::Patterns => "Patterns",
    };

    let header = Paragraph::new(vec![Line::from(vec![
        Span::styled(
            format!("[{}/{}] {}", current_page, total_pages, page_name),
            Style::default().fg(theme.accent()),
        ),
    ])])
    .block(Block::default().borders(Borders::BOTTOM));

    frame.render_widget(header, area);
}

fn render_footer(frame: &mut Frame, app: &ResultsApp, area: Rect, theme: &Theme) {
    let footer_text = Line::from(vec![
        Span::styled("Tab/←→", Style::default().fg(theme.accent())),
        Span::raw(": Pages  "),
        Span::styled("1-4", Style::default().fg(theme.accent())),
        Span::raw(": Jump  "),
        Span::styled("n/p", Style::default().fg(theme.accent())),
        Span::raw(": Items  "),
        Span::styled("c", Style::default().fg(theme.accent())),
        Span::raw(": Copy  "),
        Span::styled("e", Style::default().fg(theme.accent())),
        Span::raw(": Edit  "),
        Span::styled("?", Style::default().fg(theme.accent())),
        Span::raw(": Help  "),
        Span::styled("Esc", Style::default().fg(theme.accent())),
        Span::raw(": Back"),
    ]);

    let footer = Paragraph::new(footer_text)
        .block(Block::default().borders(Borders::TOP));

    frame.render_widget(footer, area);
}
```

### Architecture Changes

**New modules:**
- `src/tui/results/detail_pages/mod.rs` - Module declarations
- `src/tui/results/detail_pages/components.rs` - Shared rendering utilities
- `src/tui/results/detail_pages/overview.rs` - Page 1 renderer
- `src/tui/results/detail_pages/dependencies.rs` - Page 2 renderer
- `src/tui/results/detail_pages/git_context.rs` - Page 3 renderer
- `src/tui/results/detail_pages/patterns.rs` - Page 4 renderer

**Modified modules:**
- `src/tui/results/app.rs` - Add DetailPage enum and state
- `src/tui/results/detail_view.rs` - Refactor to router pattern
- `src/tui/results/navigation.rs` - Add page navigation logic

**Deleted content:**
- Current detail rendering logic from `detail_view.rs` moves to `overview.rs`

### Data Structures

All required data already exists in `UnifiedDebtItem`:

```rust
pub struct UnifiedDebtItem {
    // Page 1: Overview (existing)
    pub location: Location,
    pub unified_score: UnifiedScore,
    pub debt_type: DebtType,
    pub recommendation: ActionableRecommendation,
    pub cyclomatic_complexity: u32,
    pub cognitive_complexity: u32,
    pub nesting_depth: u32,
    pub function_length: usize,
    pub transitive_coverage: Option<TransitiveCoverage>,

    // Page 2: Dependencies
    pub upstream_dependencies: usize,
    pub downstream_dependencies: usize,
    pub upstream_callers: Vec<String>,
    pub downstream_callees: Vec<String>,

    // Page 3: Git Context
    pub contextual_risk: Option<ContextualRisk>,  // Contains git history
    pub context_multiplier: Option<f64>,
    pub context_type: Option<FileType>,
    pub file_context: Option<FileContext>,

    // Page 4: Patterns
    pub pattern_analysis: Option<PatternAnalysis>,
    pub detected_pattern: Option<DetectedPattern>,
    pub is_pure: Option<bool>,
    pub purity_confidence: Option<f32>,
    pub purity_level: Option<PurityLevel>,
    pub language_specific: Option<LanguageSpecificData>,
}
```

Accessing git context details:
```rust
if let Some(ref contextual_risk) = item.contextual_risk {
    for context in &contextual_risk.contexts {
        if context.provider == "git_history" {
            if let ContextDetails::Historical {
                change_frequency,
                bug_density,
                age_days,
                author_count,
            } = &context.details {
                // Display git metrics
            }
        }
    }
}
```

### APIs and Interfaces

No new external APIs. All changes are internal to TUI module.

**Internal interfaces:**

```rust
// Page renderer signature
pub fn render(
    frame: &mut Frame,
    app: &ResultsApp,
    item: &UnifiedDebtItem,
    area: Rect,
    theme: &Theme,
);

// Shared component signatures
pub fn add_section_header(lines: &mut Vec<Line>, title: &str, theme: &Theme);
pub fn add_label_value(lines: &mut Vec<Line>, label: &str, value: String, theme: &Theme);
pub fn add_list_section(
    lines: &mut Vec<Line>,
    title: &str,
    items: &[String],
    max_display: usize,
    theme: &Theme,
);
```

## Dependencies

**Prerequisites:**
- None (builds on existing TUI infrastructure)

**Affected Components:**
- `src/tui/results/app.rs` - Add page state
- `src/tui/results/detail_view.rs` - Refactor to router
- `src/tui/results/navigation.rs` - Add page navigation

**External Dependencies:**
- No new crates required
- Uses existing `ratatui`, `crossterm` dependencies

## Testing Strategy

### Unit Tests

**Navigation logic** (`src/tui/results/navigation.rs`):
```rust
#[test]
fn test_page_navigation_wraps_forward() {
    assert_eq!(DetailPage::Patterns.next(), DetailPage::Overview);
}

#[test]
fn test_page_navigation_wraps_backward() {
    assert_eq!(DetailPage::Overview.prev(), DetailPage::Patterns);
}

#[test]
fn test_jump_to_page() {
    assert_eq!(DetailPage::from_index(2), Some(DetailPage::GitContext));
}
```

**Conditional page availability** (`src/tui/results/app.rs`):
```rust
#[test]
fn test_available_pages_skips_empty_git_context() {
    let item = create_test_item_without_git_context();
    let app = ResultsApp::new_with_item(item);
    let pages = app.available_pages();
    assert!(!pages.contains(&DetailPage::GitContext));
    assert_eq!(app.page_count(), 3);
}

#[test]
fn test_available_pages_includes_all_with_full_data() {
    let item = create_test_item_with_all_data();
    let app = ResultsApp::new_with_item(item);
    assert_eq!(app.page_count(), 4);
}
```

**Component rendering** (`src/tui/results/detail_pages/components.rs`):
```rust
#[test]
fn test_list_truncation() {
    let items = vec!["a", "b", "c", "d", "e"].into_iter()
        .map(|s| s.to_string()).collect::<Vec<_>>();
    let mut lines = Vec::new();
    add_list_section(&mut lines, "Test", &items, 3, &Theme::default());

    // Should show 3 items + "... 2 more" line
    assert_eq!(lines.len(), 5); // Title + 3 items + "... more" + blank
}

#[test]
fn test_empty_list_skipped() {
    let items = vec![];
    let mut lines = Vec::new();
    add_list_section(&mut lines, "Test", &items, 10, &Theme::default());
    assert_eq!(lines.len(), 0); // Nothing added
}
```

### Integration Tests

**Page rendering** (`tests/tui_detail_pages_test.rs`):
```rust
#[test]
fn test_dependencies_page_renders_callers() {
    let item = create_test_item_with_callers();
    let app = create_app_with_item(item);

    // Render dependencies page
    let mut terminal = create_test_terminal();
    terminal.draw(|f| {
        detail_pages::dependencies::render(f, &app, app.selected_item().unwrap(),
            f.size(), &Theme::default());
    }).unwrap();

    // Verify callers section appears
    let buffer = terminal.backend().buffer();
    assert!(buffer_contains_text(buffer, "Upstream Callers"));
}

#[test]
fn test_git_context_page_shows_change_frequency() {
    let item = create_test_item_with_git_history();
    let app = create_app_with_item(item);

    let mut terminal = create_test_terminal();
    terminal.draw(|f| {
        detail_pages::git_context::render(f, &app, app.selected_item().unwrap(),
            f.size(), &Theme::default());
    }).unwrap();

    let buffer = terminal.backend().buffer();
    assert!(buffer_contains_text(buffer, "Change Patterns"));
    assert!(buffer_contains_text(buffer, "changes/month"));
}
```

### Manual Testing Checklist

- [ ] Navigate through all 4 pages with Tab key
- [ ] Navigate backward through pages with Shift+Tab
- [ ] Jump to specific pages with number keys 1-4
- [ ] Navigate to next/prev items while preserving page
- [ ] Verify page indicator updates correctly
- [ ] Verify footer shows correct shortcuts
- [ ] Test with items that have no git context (page skipped)
- [ ] Test with items that have no pattern data (page skipped)
- [ ] Test with items that have empty caller/callee lists
- [ ] Verify truncation works for long lists (> 10 items)
- [ ] Copy to clipboard from each page
- [ ] Open editor from each page
- [ ] Help overlay from each page
- [ ] Esc returns to list from any page
- [ ] No visual artifacts when switching pages
- [ ] Page switching feels instant
- [ ] All sections use zen aesthetic (no tree chars)
- [ ] Section headers are ALL CAPS
- [ ] Content uses 2-space indentation

## Documentation Requirements

### Code Documentation

**Module-level docs:**
```rust
//! Multi-page detail view for debt items.
//!
//! Provides four pages of contextual information:
//! - Page 1: Overview (score, metrics, recommendation)
//! - Page 2: Dependencies (callers, callees, blast radius)
//! - Page 3: Git Context (history, risk, dampening)
//! - Page 4: Patterns (purity, frameworks, language features)
//!
//! Navigation:
//! - Tab/←→: Switch pages
//! - 1-4: Jump to page
//! - n/p: Navigate items (preserves page)
```

**Function docs:**
```rust
/// Render the dependencies page showing callers and callees.
///
/// Displays:
/// - Upstream callers (first 10, truncated)
/// - Downstream callees (first 10, truncated)
/// - Dependency metrics (counts, blast radius, critical path)
///
/// Sections are skipped if no data available.
pub fn render(frame: &mut Frame, ...) { ... }
```

### User Documentation

Update `book/src/tui-guide.md`:

```markdown
## Detail View Navigation

The detail view provides four pages of information for each debt item:

### Page 1: Overview
Core information including score, metrics, coverage, and recommendation.

### Page 2: Dependencies
Call graph information showing who calls this function and what it calls.
Includes blast radius and critical path analysis.

### Page 3: Git Context
Historical information from git including change frequency, bug density,
code age, and risk multipliers.

### Page 4: Patterns
Pattern detection including purity analysis, framework patterns, and
language-specific features.

### Navigation

- **Tab** or **→**: Next page
- **Shift+Tab** or **←**: Previous page
- **1-4**: Jump to specific page
- **n/p**: Navigate between items (preserves current page)
- **c**: Copy file path
- **e**: Open in editor
- **?**: Show help
- **Esc**: Return to list view

### Smart Page Display

Pages without data are automatically skipped. For example, if a function
has no git history, the Git Context page won't appear and the page
indicator will show [1/3] instead of [1/4].
```

### Architecture Updates

Update `ARCHITECTURE.md`:

```markdown
### TUI Multi-Page Detail View

The detail view uses a page-based architecture to present contextual
information while maintaining zen minimalism:

- **Page Router** (`detail_view.rs`): Dispatches to appropriate page renderer
- **Page Renderers** (`detail_pages/*.rs`): Each page is a pure function
- **Shared Components** (`detail_pages/components.rs`): Reusable rendering utilities
- **Navigation** (`navigation.rs`): Page and item navigation state machine

Pages are conditionally rendered based on data availability, ensuring
users only see relevant information.
```

## Implementation Notes

### Design Principles

**Zen Aesthetic:**
- No decorative characters (no `├─`, `└─`, `│`, boxes, etc.)
- Clean spacing and typography for visual hierarchy
- ALL CAPS for major section headers
- Simple label: value format for data
- 2-space indentation only
- Blank lines between major sections

**Progressive Disclosure:**
- Default view (Overview) shows essential decision data
- Additional pages provide depth without clutter
- Empty sections silently omitted
- Truncation with clear indicators ("... N more")

**Functional Patterns:**
- Page renderers are pure functions
- Shared components extracted for reuse
- State changes localized to navigation module
- Data access separated from presentation

### Gotchas and Edge Cases

**Empty Data:**
- Always check for `None`/empty before rendering sections
- Skip entire pages if all sections would be empty
- Update page count dynamically based on available data

**Long Lists:**
- Truncate at 10 items consistently
- Show total count in section header
- Use "... N more" format for truncation indicator

**Navigation Wrapping:**
- Tab from page 4 wraps to page 1
- Shift+Tab from page 1 wraps to page 4
- Handle case where page is skipped (e.g., no git context)

**State Preservation:**
- Preserve current page when navigating items (n/p)
- Reset to page 1 when entering detail view from list
- Maintain scroll position within pages if scrollable

**Terminal Resize:**
- Recalculate visible area on resize
- Maintain current page selection
- Gracefully handle very small terminals

### Performance Considerations

**Lazy Rendering:**
- Only render current page, not all pages
- Keep page switching logic minimal
- Avoid expensive computations in render path

**Data Access:**
- Access data from `UnifiedDebtItem` fields directly
- No additional analysis required during rendering
- All data pre-computed during analysis phase

**Memory:**
- Page renderers don't allocate persistent state
- Use stack-allocated vectors for line building
- Clean up after each render

## Migration and Compatibility

### Breaking Changes

None. This is a purely additive feature.

### Migration Path

1. Extract current detail view rendering to `overview.rs`
2. Add new page infrastructure (enum, state)
3. Implement navigation logic
4. Build out additional page renderers
5. Test thoroughly with various data scenarios

### Backwards Compatibility

- Existing list view unchanged
- Existing navigation (Esc, c, e) still works
- No changes to data structures or analysis
- No changes to CLI output

### Future Extensions

**Potential Future Pages:**
- **Page 5: Tests** - Test coverage details, test complexity
- **Page 6: History** - Detailed git commit history
- **Page 7: Related** - Related debt items, similar patterns

**Enhancements:**
- Scrollable content within pages
- Search within detail view
- Export current page to file
- Compare two debt items side-by-side

## Success Metrics

**Qualitative:**
- Users report improved decision-making with full context
- No complaints about cluttered or confusing interface
- Positive feedback on clean, scannable design
- Users discover value in git context and pattern data

**Quantitative:**
- < 5 seconds to access any contextual data
- < 16ms page switch render time
- 100% test coverage for navigation logic
- Zero visual artifacts during page transitions
- All manual test checklist items pass

**Adoption:**
- Users spend more time in TUI vs CLI with `-v`
- Reduction in "how do I see git history?" support questions
- Increased usage of pattern detection insights
