---
number: 211
title: Interactive TUI Results Explorer
category: foundation
priority: high
status: draft
dependencies: [210]
created: 2025-12-05
---

# Specification 211: Interactive TUI Results Explorer

**Category**: foundation
**Priority**: high
**Status**: draft
**Dependencies**: Existing TUI infrastructure (`src/tui/`)

**Note**: This spec depends on the TUI foundation implemented in `src/tui/` (Theme, TuiManager, animation). While this was originally documented as "Spec 210", the implementation exists and is production-ready. See CHANGELOG.md for details on the zen minimalist TUI system.

## Context

Debtmap currently outputs analysis results as **flat text to stdout**. For large codebases with hundreds of debt items, this creates several UX problems:

**Current Text Output** (with `--no-tui` or in CI):
```
============================================
    Debtmap v0.8.0
============================================

TOP 10 RECOMMENDATIONS

#1 SCORE: 370 [CRITICAL] IMPACT: -16-31% complexity
  └─ god_object_detector.rs · functions: 55, responsibilities: 8
  └─ ACTION: Split by analysis phase (data → detect → score → report)

#2 SCORE: 185 [HIGH] IMPACT: -12-18% complexity
  └─ priority/formatter.rs · functions: 42, responsibilities: 6
  └─ ACTION: Extract formatters by output type (JSON, Markdown, HTML)

... (items #3-#10 continue scrolling off screen) ...
```

**Current Issues**:
- **Information overload**: 386 recommendations dumped as scrolling text
- **No exploration**: Can't drill down into specific items for details
- **Lost context**: Full details shown for every item (7-10 metrics each)
- **No filtering**: Must read through all items to find relevant ones
- **Poor ergonomics**: Requires scrolling terminal buffers, copying paths manually

**User Pain Points**:
- "I have 386 debt items, where do I start?"
- "How do I find all items related to testing?"
- "I want to see full git history, but not for every item"
- "Can I jump directly to this file in my editor?"

This flat output model doesn't scale with the zen minimalist philosophy of "show only what's needed."

**Comparison: Text vs Interactive TUI**

| Feature | Text Output (`--no-tui`) | Interactive TUI (new) |
|---------|--------------------------|----------------------|
| **Navigation** | Scroll terminal buffer | ↑↓ / jk keys, g/G shortcuts |
| **Filtering** | Pipe to `grep` | Real-time `/` search + filters |
| **Sorting** | Fixed (by score) | Sort by score, coverage, complexity |
| **Details** | All shown or use -v flags | Press Enter to expand/collapse |
| **Actions** | Copy path manually | `c` to clipboard, `e` to editor |
| **Large results** | Scroll 386 items | Virtual scrolling, instant jump |
| **Context** | Re-run with different flags | Switch views interactively |

## Objective

Create an interactive TUI for exploring analysis results that:

1. **Defaults to interactive mode** - Makes TUI the primary interface
2. **Progressive disclosure** - Show summary, reveal details on demand
3. **Keyboard-driven navigation** - Efficient, accessible interaction
4. **Actionable integration** - Copy paths, open editor, filter/search
5. **Preserves automation** - Keep --json, --markdown, --html for CI/CD

**Success Metrics**:
- Users can navigate 386 items comfortably without terminal scrolling
- 80% of users prefer TUI over flat text output (user testing)
- Time to find specific debt item reduced by 50%
- Zero automation workflow breakage (CI/CD continues to work)

## Requirements

### Functional Requirements

**FR1: Main List View**
- Display debt items as table with columns: Rank, Score, Location, Action
- Show top 20 visible items with scrolling for more
- Highlight current selection with visual indicator (▸)
- Display summary metrics in header (total debt, density, coverage)
- Show navigation hints in footer status bar
- Support unlimited scrolling through all debt items

**FR2: Detail View**
- Show full details for selected item in organized sections
- Include all available information: location, recommendation, impact, complexity, coverage, git history, patterns
- Support scrolling for long content
- Provide contextual actions (copy path, open editor)
- Show position context (e.g., "1/386")

**FR3: Search Functionality**
- Fuzzy search by filename, function name, or path
- Filter results in real-time as user types
- Show match count
- Navigate between search results
- Clear search to return to full list

**FR4: Sort Capabilities**
- Sort by: Score (default), Coverage, Complexity, Changes, Age
- Toggle ascending/descending order
- Persist sort selection during session
- Visual indicator of current sort

**FR5: Filter Capabilities**
- Filter by severity: All, Critical, High, Medium, Low
- Filter by coverage: No coverage, Partial, Full
- Filter by complexity thresholds
- Filter by recency (changed in last N days)
- Combine multiple filters
- Show active filter count

**FR6: Keyboard Navigation**
- Arrow keys (↑↓) or vim keys (jk) - Navigate items
- Enter - View details
- Esc - Back/cancel/quit
- / - Search
- s - Sort menu
- f - Filter menu
- g/G - Go to top/bottom
- n/p - Next/previous in detail view
- q - Quit application
- ? - Help overlay

**FR7: Actions**
- c - Copy file path to clipboard (system clipboard, with graceful fallback on failure)
- e - Open in $EDITOR (respects environment variable, falls back to VISUAL then vim)
- o - Open file at line number in editor
- Export selected items to file

**Note on Clipboard**: If clipboard access fails (SSH, headless, permissions), show error message with the path so user can manually copy it.

**FR8: Fallback Mode Detection**
- Auto-detect non-interactive environments (CI/CD, pipes)
- Fall back to text output when stdout is not a TTY
- Support --no-tui flag to force text output
- Maintain backward compatibility with existing scripts

**Text Output Format** (when TUI is disabled):

The fallback text output is a formatted, colored terminal report showing a ranked list of debt items:

```
============================================
    Debtmap v0.8.0
============================================

TOP 10 RECOMMENDATIONS

#1 SCORE: 370 [CRITICAL] IMPACT: -16-31% complexity
  └─ god_object_detector.rs · functions: 55, responsibilities: 8
  └─ ACTION: Split by analysis phase (data → detect → score → report)

#2 SCORE: 185 [HIGH] IMPACT: -12-18% complexity
  └─ priority/formatter.rs · functions: 42, responsibilities: 6
  └─ ACTION: Extract formatters by output type (JSON, Markdown, HTML)

...

TOTAL DEBT SCORE: 1842
DEBT DENSITY: 12.3 per 1K LOC (149,832 total LOC)
OVERALL COVERAGE: 72.45%
```

This text format is:
- **Scannable**: 3-4 lines per item by default
- **Colored**: Uses terminal colors (respects NO_COLOR)
- **Scrollable**: Users scroll terminal buffer to view all items
- **Compact**: Summary mode for quick overview
- **Detailed**: Verbose flags (-v, -vv) show more metrics

The TUI improvement addresses pain points of this text format:
- **No filtering/sorting** without piping to external tools
- **No interactive exploration** of individual items
- **Terminal buffer scrolling** required for 100+ items
- **No drill-down** for detailed metrics without re-running

### Non-Functional Requirements

**NFR1: Performance**
- List view renders in <16ms (60 FPS) target
- Handle 10,000+ items without lag (requires virtual scrolling)
- Instant search filtering (<100ms)
- Smooth scrolling with no frame drops

**Performance Note**: Targets validated on modern hardware (M1 Mac, iTerm2). Actual performance may vary by terminal emulator and system capabilities. Virtual scrolling (rendering only visible items) is essential for large result sets.

**NFR2: Accessibility**
- Screen reader compatible (text-based interface)
- Configurable key bindings
- High contrast mode support
- Respect NO_COLOR and TERM environment variables:
  ```rust
  fn should_use_color() -> bool {
      std::env::var("NO_COLOR").is_err() &&
      std::env::var("TERM").map(|t| t != "dumb").unwrap_or(true)
  }
  ```
- When color is disabled, use text-only indicators (*, >, -, etc.)

**NFR3: Terminal Compatibility**
- Support terminals: iTerm2, Terminal.app, Alacritty, Kitty, Windows Terminal
- Minimum terminal size: 80x24
- Graceful degradation for smaller terminals
- UTF-8 box drawing character support with ASCII fallback

**NFR4: Consistency**
- Match zen minimalist aesthetic from analysis progress TUI
- Consistent color palette across TUI components
- Unified keybinding conventions
- Coherent visual language (same spacing, borders, indicators)

**NFR5: Maintainability**
- Modular architecture separating concerns (view/model/controller)
- Reusable TUI components shared with progress visualization
- Clear separation from output formatting logic
- Comprehensive unit tests for navigation logic

## Acceptance Criteria

### Core Functionality
- [ ] Interactive TUI launches by default when running `debtmap analyze .`
- [ ] List view displays all debt items in table format with scrolling
- [ ] Detail view shows full information for selected item
- [ ] Search filters items in real-time by filename/function
- [ ] Sort menu changes item ordering
- [ ] Filter menu reduces visible items by criteria
- [ ] Keyboard navigation works for all primary actions

### User Actions
- [ ] Copy path (c) copies file path to system clipboard
- [ ] Clipboard copy shows error message with path if clipboard unavailable
- [ ] Open editor (e) launches $EDITOR with correct file
- [ ] Editor action falls back to VISUAL, then vim if EDITOR unset
- [ ] Open at line (o) launches editor at specific line number
- [ ] Editor line number syntax correct for vim, VS Code, emacs, Sublime, Helix
- [ ] Navigation (j/k/↑/↓) moves between items smoothly
- [ ] Detail navigation (n/p) moves to next/previous item

### Automation Compatibility
- [ ] `debtmap analyze . --json` outputs JSON without TUI
- [ ] `debtmap analyze . --markdown` outputs Markdown without TUI
- [ ] `debtmap analyze . --html` outputs HTML without TUI
- [ ] `debtmap analyze . --no-tui` outputs text without TUI
- [ ] `--no-tui` flag properly parsed and passed to AnalyzeConfig
- [ ] CI/CD environments (CI=true) auto-detect and use text output
- [ ] Piped output (e.g., `| grep`) auto-detects and uses text
- [ ] Redirected output (e.g., `> file.txt`) auto-detects and uses text
- [ ] SSH sessions without TTY use text output

### Performance
- [ ] List view renders 60 FPS with 1000+ items
- [ ] Search filters 10,000 items in <100ms
- [ ] Detail view switches instantly (<16ms)
- [ ] No memory leaks during extended sessions

### Visual Quality
- [ ] Matches zen minimalist progress TUI aesthetic
- [ ] Clean, readable typography with proper spacing
- [ ] Smooth animations and transitions
- [ ] Consistent color usage (muted, intentional palette)
- [ ] Help overlay (?) shows all keybindings clearly

### Edge Cases
- [ ] Handles empty results gracefully (no items) - shows helpful message
- [ ] Handles single result without errors
- [ ] Handles very long file paths (truncation/wrapping)
- [ ] Handles very long function names
- [ ] Handles terminal resize events - recomputes layout
- [ ] Handles invalid $EDITOR gracefully - shows error, doesn't crash
- [ ] Handles clipboard access failure - shows path in error message
- [ ] Handles SSH/headless environments - falls back to text mode
- [ ] Handles narrow terminals (<80 cols) - degrades gracefully or shows warning

## Technical Details

### Implementation Approach

**Architecture: Model-View-Controller**

```
src/tui/results/
├── mod.rs              # Public API, TUI manager
├── app.rs              # Application state (Model)
├── list_view.rs        # Main list rendering (View)
├── detail_view.rs      # Detail panel rendering (View)
├── search.rs           # Search functionality
├── sort.rs             # Sort logic
├── filter.rs           # Filter logic
├── actions.rs          # User actions (copy, editor)
├── navigation.rs       # Keyboard navigation (Controller)
├── layout.rs           # Layout calculations
└── theme.rs            # Shared theme/colors
```

**State Management**

```rust
pub struct ResultsApp {
    // Data
    items: Vec<DebtItem>,
    filtered_items: Vec<usize>, // Indices into items

    // View state
    current_view: ViewMode,
    selected_index: usize,
    scroll_offset: usize,

    // Filters/search
    search_query: String,
    active_filters: Vec<Filter>,
    sort_by: SortCriteria,
    sort_order: SortOrder,

    // UI state
    terminal_size: (u16, u16),
    animation_frame: usize,
}

pub enum ViewMode {
    List,
    Detail,
    Search,
    SortMenu,
    FilterMenu,
    Help,
}
```

**Navigation Logic**

```rust
impl ResultsApp {
    pub fn handle_key(&mut self, key: KeyEvent) -> Action {
        match (self.current_view, key) {
            (ViewMode::List, Key::Up) => self.move_selection(-1),
            (ViewMode::List, Key::Down) => self.move_selection(1),
            (ViewMode::List, Key::Enter) => self.enter_detail_view(),
            (ViewMode::List, Key::Char('/')) => self.enter_search(),
            (ViewMode::Detail, Key::Esc) => self.back_to_list(),
            // ... comprehensive key handling
        }
    }
}
```

**Rendering Pipeline**

```rust
pub fn render_ui(frame: &mut Frame, app: &ResultsApp) {
    match app.current_view {
        ViewMode::List => render_list_view(frame, app),
        ViewMode::Detail => render_detail_view(frame, app),
        ViewMode::Search => render_search_view(frame, app),
        // ...
    }
}
```

### Data Structures

**Table Layout**

```rust
pub struct TableRow {
    rank: usize,
    score: f64,
    severity: Severity,
    location: String,      // Truncated path
    action: String,        // First line of recommendation
    item_index: usize,     // Index into full items vector (avoid cloning)
}

pub struct TableLayout {
    visible_rows: usize,
    column_widths: [u16; 4],  // Rank, Score, Location, Action
    header_height: u16,
    footer_height: u16,
}

// Note: Using item_index instead of cloning full_item reduces memory usage
// significantly for large result sets (e.g., 10k items). The full DebtItem
// can be accessed via app.items[row.item_index] when needed.
```

**Filter System**

```rust
pub enum Filter {
    Severity(Vec<Severity>),
    Coverage(CoverageRange),
    Complexity(ComplexityRange),
    RecentChanges(Duration),
}

impl Filter {
    pub fn matches(&self, item: &DebtItem) -> bool {
        // Filter matching logic
    }
}
```

### Integration Points

**Entry Point**

```rust
// src/commands/analyze.rs
pub fn handle_analyze(config: AnalyzeConfig) -> Result<()> {
    // ... perform analysis ...

    // Determine output mode
    if should_use_tui(&config) {
        // Launch interactive TUI
        tui::results::ResultsExplorer::new(filtered_analysis).run()?;
    } else {
        // Use traditional text/JSON/markdown output
        output::output_unified_priorities_with_config(
            filtered_analysis,
            output_config,
            &results,
            config.coverage_file.as_ref(),
        )?;
    }

    Ok(())
}

fn should_use_tui(config: &AnalyzeConfig) -> bool {
    // Auto-detect: Use TUI if interactive terminal and no explicit format
    use std::io::IsTerminal;

    !config.no_tui                                      // Not explicitly disabled
        && config.format == OutputFormat::Terminal      // Terminal format (not JSON/Markdown/Html)
        && config.output.is_none()                      // No output file specified
        && std::io::stdout().is_terminal()              // Interactive terminal
}
```

**Shared Components from Progress TUI**

Reuse theme, layout helpers, and terminal management:

```rust
use crate::tui::{Theme, TuiManager};

pub struct ResultsExplorer {
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
    app: ResultsApp,
    theme: Theme,  // Shared with progress TUI
}
```

### Clipboard Integration

```rust
use arboard::Clipboard;

pub fn copy_to_clipboard(text: &str) -> Result<()> {
    let mut clipboard = Clipboard::new()
        .context("Failed to access clipboard")?;

    clipboard.set_text(text)
        .context("Failed to copy to clipboard")?;

    Ok(())
}

// Note: arboard provides better cross-platform support than copypasta:
// - Works on macOS, Linux (X11/Wayland), Windows
// - Better maintained and more reliable
// - Gracefully handles headless/SSH environments
```

### Editor Integration

```rust
use std::process::Command;

pub fn open_in_editor(path: &Path, line: Option<usize>) -> Result<()> {
    let editor = std::env::var("EDITOR")
        .or_else(|_| std::env::var("VISUAL"))
        .unwrap_or_else(|_| "vim".to_string());

    let mut cmd = Command::new(&editor);

    // Support common editor line number syntax
    match (editor.as_str(), line) {
        ("vim" | "nvim", Some(n)) => cmd.arg(format!("+{}", n)),
        ("code" | "code-insiders", Some(n)) => {
            cmd.arg("--goto").arg(format!("{}:{}", path.display(), n))
        }
        ("emacs", Some(n)) => cmd.arg(format!("+{}", n)),
        ("subl" | "sublime", Some(n)) => cmd.arg(format!("{}:{}", path.display(), n)),
        ("hx" | "helix", Some(n)) => cmd.arg(format!("{}:{}", path.display(), n)),
        _ => &mut cmd,
    };

    cmd.arg(path)
       .spawn()
       .context("Failed to launch editor")?;

    Ok(())
}

// Supported editors:
// - vim/nvim: +{line} {file}
// - VS Code: --goto {file}:{line}
// - emacs: +{line} {file}
// - Sublime Text: {file}:{line}
// - Helix: {file}:{line}
// - Others: {file} (no line number support, fallback gracefully)
//
// Note: IntelliJ IDEA and JetBrains IDEs use different command-line tools per product
// (idea, pycharm, rubymine, etc.) and are not auto-detected. Users can set EDITOR
// explicitly if needed.
```

## Dependencies

### Prerequisites
- **Existing TUI Infrastructure** (`src/tui/`)
  - Provides foundation: `TuiManager`, `Theme`, terminal initialization
  - Establishes visual language and interaction patterns
  - Defines shared TUI components
  - Already implemented and production-ready

### Affected Components
- `src/commands/analyze.rs` - Add TUI vs text output decision logic, add `no_tui: bool` field to `AnalyzeConfig`
- `src/cli.rs` - Add `--no-tui` flag to CLI argument parsing
- `src/output/mod.rs` - Refactor to support both modes
- `src/tui/mod.rs` - Extend with results explorer submodule

### External Dependencies

Add to `Cargo.toml`:
```toml
[dependencies]
# Existing TUI dependencies (already in Cargo.toml)
# ratatui = "0.26"
# crossterm = "0.27"

# New dependencies for results explorer
arboard = "3.3"  # Clipboard access (cross-platform, well-maintained)
fuzzy-matcher = "0.3"  # Fuzzy search
```

## Testing Strategy

### Unit Tests

**Navigation Tests**
```rust
#[test]
fn test_list_navigation() {
    let mut app = ResultsApp::new(create_test_items(100));

    app.handle_key(Key::Down);
    assert_eq!(app.selected_index, 1);

    app.handle_key(Key::Up);
    assert_eq!(app.selected_index, 0);
}

#[test]
fn test_bounds_checking() {
    let mut app = ResultsApp::new(create_test_items(5));

    // Try to go above top
    app.selected_index = 0;
    app.handle_key(Key::Up);
    assert_eq!(app.selected_index, 0); // Should stay at 0

    // Try to go below bottom
    app.selected_index = 4;
    app.handle_key(Key::Down);
    assert_eq!(app.selected_index, 4); // Should stay at 4
}
```

**Search Tests**
```rust
#[test]
fn test_search_filtering() {
    let app = ResultsApp::new(vec![
        debt_item("formatter/writer.rs", "write_section"),
        debt_item("parser/lexer.rs", "tokenize"),
        debt_item("formatter/reader.rs", "read_section"),
    ]);

    app.set_search_query("format");
    assert_eq!(app.filtered_items.len(), 2); // writer.rs and reader.rs
}

#[test]
fn test_fuzzy_search() {
    let app = ResultsApp::new(vec![
        debt_item("src/priority/formatter.rs", "format_output"),
    ]);

    app.set_search_query("prifmt"); // Fuzzy match
    assert_eq!(app.filtered_items.len(), 1);
}
```

**Filter Tests**
```rust
#[test]
fn test_severity_filter() {
    let app = ResultsApp::new(vec![
        critical_item(),
        high_item(),
        medium_item(),
    ]);

    app.add_filter(Filter::Severity(vec![Severity::Critical]));
    assert_eq!(app.filtered_items.len(), 1);
}

#[test]
fn test_combined_filters() {
    let app = ResultsApp::new(create_mixed_items());

    app.add_filter(Filter::Severity(vec![Severity::Critical]));
    app.add_filter(Filter::Coverage(CoverageRange::None));

    // Should only show critical items with no coverage
    assert!(app.filtered_items.iter().all(|&i| {
        app.items[i].is_critical() && app.items[i].has_no_coverage()
    }));
}
```

**Sort Tests**
```rust
#[test]
fn test_sort_by_score() {
    let mut app = ResultsApp::new(vec![
        item_with_score(10.0),
        item_with_score(50.0),
        item_with_score(25.0),
    ]);

    app.sort_by(SortCriteria::Score, SortOrder::Descending);

    assert_eq!(app.filtered_items, vec![1, 2, 0]); // 50, 25, 10
}
```

### Integration Tests

**TUI Lifecycle**
```rust
#[test]
fn test_tui_launch_and_quit() {
    let analysis = create_test_analysis();
    let mut explorer = ResultsExplorer::new(analysis);

    // Simulate quit key
    explorer.handle_key(Key::Char('q'));

    // Should exit cleanly
    assert!(explorer.should_quit());
}
```

**Auto-detection**
```rust
#[test]
fn test_auto_detect_ci_environment() {
    std::env::set_var("CI", "true");

    let config = AnalyzeConfig::default();
    assert!(!should_use_tui(&config)); // Should use text output in CI

    std::env::remove_var("CI");
}

#[test]
fn test_auto_detect_piped_output() {
    // When stdout is piped, should not use TUI
    // (This would need to be tested in subprocess)
}
```

**Editor Integration**
```rust
#[test]
fn test_editor_command_generation() {
    std::env::set_var("EDITOR", "vim");

    let cmd = build_editor_command(Path::new("test.rs"), Some(42));
    assert_eq!(cmd.get_args().collect::<Vec<_>>(), vec!["+42", "test.rs"]);
}
```

### Manual Testing Checklist

**User Experience**
- [ ] Navigate through 500+ items feels smooth
- [ ] Search feels instant and intuitive
- [ ] Detail view provides useful information
- [ ] Keyboard shortcuts are discoverable
- [ ] Help overlay (?) is comprehensive
- [ ] Terminal resize doesn't break layout

**Edge Cases**
- [ ] Empty results (0 items) displays helpful message
- [ ] Single item doesn't crash
- [ ] Very long paths wrap/truncate properly
- [ ] Very long function names handle gracefully
- [ ] Small terminals (80x24) work acceptably
- [ ] Large terminals (200x60) use space well

**Compatibility**
- [ ] Works in iTerm2 on macOS
- [ ] Works in Terminal.app on macOS
- [ ] Works in Alacritty on Linux
- [ ] Works in Windows Terminal
- [ ] Handles NO_COLOR environment variable
- [ ] Clipboard works on macOS/Linux/Windows

## Documentation Requirements

### Code Documentation

**Module Documentation**
```rust
//! Interactive TUI for exploring analysis results.
//!
//! This module provides a keyboard-driven interface for navigating,
//! searching, filtering, and acting on technical debt items.
//!
//! # Examples
//!
//! ```rust,no_run
//! use debtmap::tui::results::ResultsExplorer;
//!
//! let analysis = perform_analysis()?;
//! let mut explorer = ResultsExplorer::new(analysis);
//! explorer.run()?;
//! ```
```

**Public API Documentation**
- Document all public functions with examples
- Explain keybinding system and customization
- Document filter/sort capabilities
- Provide architecture overview

### User Documentation

**README.md Updates**
```markdown
## Interactive Results Explorer

By default, debtmap launches an interactive TUI to explore analysis results:

```bash
debtmap analyze .
```

### Navigation
- `↑/↓` or `j/k` - Navigate items
- `Enter` - View details
- `/` - Search
- `s` - Sort
- `f` - Filter
- `q` - Quit

### Actions
- `c` - Copy file path
- `e` - Open in editor
- `o` - Open at line number

### Output Formats

For automation and CI/CD:

```bash
debtmap analyze . --json > results.json
debtmap analyze . --markdown > report.md
debtmap analyze . --no-tui  # Force text output
```
```

**Help Screen Documentation**

Create comprehensive in-app help (? key):
```
Keyboard Shortcuts
──────────────────

Navigation
  ↑/k         Move up
  ↓/j         Move down
  Enter       View details
  Esc         Back/Cancel
  g           Go to top
  G           Go to bottom

Views
  /           Search
  s           Sort menu
  f           Filter menu
  ?           This help

Actions
  c           Copy path
  e           Open editor
  o           Open at line

  q           Quit
```

### Architecture Documentation

Update `ARCHITECTURE.md`:
```markdown
## Interactive TUI Results Explorer

The results explorer provides an interactive interface for navigating
analysis results using a Model-View-Controller architecture.

### Components

- **Model** (`app.rs`): Application state and data
- **Views** (`*_view.rs`): Rendering logic
- **Controller** (`navigation.rs`): Input handling
- **Actions** (`actions.rs`): User interactions

### State Flow

User Input → Navigation → State Update → View Render → Terminal Output

### Shared Components

Reuses TUI infrastructure from analysis progress visualization:
- Terminal management
- Theme and color palette
- Layout utilities
```

## Implementation Notes

### Phased Rollout

Implementation should be broken into logical phases to ensure incremental progress and testability:

**Phase 1: Core Foundation**
- Basic list view with scrolling
- Simple detail view
- Quit functionality
- Auto-detection logic

**Phase 2: Search & Filter**
- Search implementation
- Filter system
- Sort capabilities
- Status bar updates

**Phase 3: Actions**
- Clipboard integration
- Editor launching
- Help overlay
- Keybinding polish

**Phase 4: Polish**
- Animation refinements
- Theme consistency
- Performance optimization
- Comprehensive testing

Each phase should be completed with full testing before moving to the next. Phases are designed to deliver incremental value - even Phase 1 alone provides a usable interactive interface.

### Performance Considerations

**Large Lists**
- Use virtual scrolling (render only visible items):
  ```rust
  // Only render items within viewport
  let visible_start = scroll_offset;
  let visible_end = (scroll_offset + visible_rows).min(filtered_items.len());
  let visible_items = &filtered_items[visible_start..visible_end];
  ```
- Lazy calculation of filtered indices (compute on-demand, not eagerly)
- Incremental search with debouncing (wait 50-100ms after last keystroke)
- Cache layout calculations (terminal size, column widths)
- Use indices instead of cloning items (as specified in TableRow)

**Memory Management**
- Share UnifiedAnalysis data (don't clone)
- Use indices instead of cloning DebtItems
- Reuse string allocations where possible

**Rendering Optimization**
- Only redraw changed regions
- Batch terminal updates
- Throttle render to 60 FPS max

### Accessibility

**Screen Readers**
- Text-based interface is inherently accessible
- Provide audio cues for state changes (optional)
- Support standard screen reader navigation

**Color Blindness**
- Don't rely solely on color for information
- Use symbols/text alongside colors
- Support high contrast mode

**Keyboard Navigation**
- All features accessible via keyboard
- No mouse-only functionality
- Customizable key bindings (future)

### Gotchas

**Terminal State Management**
- Always restore terminal on panic (use Drop impl)
- Handle SIGTERM gracefully
- Clear alternate screen before exit

**Cross-Platform**
- Test clipboard on all platforms
- Verify editor launching on Windows
- Handle path separators correctly

**Backward Compatibility**
- Ensure CI/CD workflows don't break
- Test piped output extensively
- Validate --json/--markdown unchanged

## Migration and Compatibility

### Breaking Changes

**None** - This is purely additive:
- Existing `--json`, `--markdown`, `--html` flags unchanged
- Text output still available via `--no-tui` or auto-detection
- CI/CD environments auto-detect and use text mode

### Migration Path

**For Users**
1. Update to version with TUI
2. Run `debtmap analyze .` - automatically gets TUI
3. Use `--no-tui` if old behavior preferred
4. Set alias if always want text: `alias debtmap='debtmap --no-tui'`

**For Scripts/Automation**
- No changes needed - auto-detection handles it
- Optionally add `--json` explicitly for clarity
- Set `CI=true` environment variable for guaranteed text mode

### Rollback Plan

If TUI causes issues:
1. Add `DEBTMAP_DISABLE_TUI=1` environment variable check
2. Document workaround in release notes
3. Fix issues and re-enable in patch release

### Future Enhancements

**Post-MVP Features** (explicitly out of scope for initial release):
- Export filtered results to file
- Mark items as reviewed/ignored (persistent state)
- Side-by-side comparison of items
- Customizable keybindings via config file
- Theme customization
- Mouse support (MVP is keyboard-only by design for accessibility and performance)
- Integration with git (show blame, commit info inline)
- Watch mode (re-run analysis on file changes)
- Multi-select for batch operations
- Bookmarking/favoriting specific items
