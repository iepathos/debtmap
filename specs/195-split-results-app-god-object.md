---
number: 195
title: Split ResultsApp God Object into Focused Modules
category: optimization
priority: high
status: draft
dependencies: []
created: 2025-12-13
---

# Specification 195: Split ResultsApp God Object into Focused Modules

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: None

## Context

The `ResultsApp` struct in `src/tui/results/app.rs` (line 111) has been identified as a God Object with:

| Metric | Current Value | Target |
|--------|--------------|--------|
| Fields | 17 | 5-6 per struct |
| Methods | 58 | 10-15 per impl |
| Responsibilities | 9 | 1 (single responsibility) |
| Total LOC | 37 (struct) + 600+ (impl) | <100 per module |
| Accumulated Cyclomatic | 169 | <50 per module |
| Accumulated Cognitive | 129 | <30 per module |
| Test Coverage | 20.9% | >80% |
| Debt Score | 100.0 (critical) | <20 |

### Current Responsibilities (9 detected)

1. **Core Data** - `analysis`, `filtered_indices`
2. **List Selection** - `selected_index`, `scroll_offset`
3. **Grouping** - `show_grouped`
4. **Filtering** - `filters`
5. **Sorting** - `sort_by`
6. **Searching** - `search`
7. **View Mode / Rendering** - `view_mode`, `terminal_size`, `needs_redraw`, `status_message`
8. **Detail Page Navigation** - `detail_page` and 12 related methods
9. **DSM & Navigation History** - `dsm_scroll_x`, `dsm_scroll_y`, `nav_history`, `dsm_enabled`

### Philosophy Violations (Stillwater)

- **Pure Core, Imperative Shell**: State mutations scattered throughout methods
- **Composition Over Complexity**: Single monolithic struct instead of composable pieces
- **Small, Focused Functions**: Some methods like `has_pattern_data()` are 31 lines with 10+ conditions

## Objective

Split `ResultsApp` into 3 focused modules following the single responsibility principle:

1. **ListState** - List selection, scroll position, and grouping
2. **QueryState** - Filtering, sorting, searching, and filtered indices
3. **NavigationState** - View modes, detail pages, DSM, and navigation history

The main `ResultsApp` becomes a slim coordinator that composes these modules.

**Target Architecture:**

```
src/tui/results/
├── app.rs           # Slim coordinator (composes modules)
├── list_state.rs    # Selection, scroll, grouping (~100 lines)
├── query_state.rs   # Filter, sort, search (~150 lines)
├── nav_state.rs     # View mode, pages, history (~150 lines)
└── ... (existing files unchanged)
```

## Requirements

### Functional Requirements

1. **Extract ListState Module**
   - Move fields: `selected_index`, `scroll_offset`, `show_grouped`
   - Move methods: selection getters/setters, scroll getters/setters, grouping toggle
   - Create pure functions for selection bounds checking
   - Implement `Default` trait for initial state

2. **Extract QueryState Module**
   - Move fields: `filtered_indices`, `search`, `filters`, `sort_by`
   - Move methods: `apply_search()`, `apply_filters()`, `apply_sort()`, filter CRUD
   - Require `&UnifiedAnalysis` reference for operations (not ownership)
   - Create pure filter/sort functions that return new indices

3. **Extract NavigationState Module**
   - Move fields: `view_mode`, `detail_page`, `nav_history`, `dsm_enabled`, `dsm_scroll_x`, `dsm_scroll_y`
   - Move methods: view mode getters/setters, page navigation, history management
   - Move `ViewMode` and `DetailPage` enums to this module
   - Create pure state transition functions

4. **Refactor ResultsApp as Coordinator**
   - Compose the three state modules
   - Keep only: `analysis`, `terminal_size`, `needs_redraw`, `status_message`
   - Delegate to modules via method forwarding or direct field access
   - Keep `handle_key()` and `render()` as coordination points

5. **Preserve All Existing Behavior**
   - All TUI interactions work identically
   - Same rendering output
   - Same keyboard handling
   - Backward compatible API for external callers

### Non-Functional Requirements

1. **Testability**
   - Each module testable in isolation without TUI setup
   - Pure functions for state transitions
   - 80%+ test coverage for each extracted module
   - Fast unit tests (no I/O)

2. **Maintainability**
   - Clear module boundaries
   - Single responsibility per module
   - Easy to understand state flow
   - Consistent patterns across modules

3. **Performance**
   - No performance regression
   - Same or better memory usage
   - Efficient state updates

## Acceptance Criteria

- [ ] `ListState` extracted to `src/tui/results/list_state.rs` with <100 LOC
- [ ] `QueryState` extracted to `src/tui/results/query_state.rs` with <150 LOC
- [ ] `NavigationState` extracted to `src/tui/results/nav_state.rs` with <150 LOC
- [ ] `ResultsApp` reduced to <100 LOC (coordinator only)
- [ ] Each module has single responsibility
- [ ] All functions under 20 lines
- [ ] Cyclomatic complexity <5 for all functions
- [ ] Unit tests added for each extracted module (80%+ coverage)
- [ ] All existing TUI integration tests pass
- [ ] `cargo clippy` passes with no warnings
- [ ] `cargo test` passes
- [ ] Debt score reduced from 100.0 to <30

## Technical Details

### Implementation Approach

**Phase 1: Extract ListState**

```rust
// src/tui/results/list_state.rs

/// Manages list selection, scroll position, and grouping state.
///
/// Pure state container with no I/O operations.
#[derive(Debug, Clone)]
pub struct ListState {
    selected_index: usize,
    scroll_offset: usize,
    show_grouped: bool,
}

impl Default for ListState {
    fn default() -> Self {
        Self {
            selected_index: 0,
            scroll_offset: 0,
            show_grouped: true, // Default: grouping enabled
        }
    }
}

impl ListState {
    /// Get selected index.
    pub fn selected_index(&self) -> usize {
        self.selected_index
    }

    /// Set selected index with bounds checking.
    ///
    /// Pure function - validates against provided item count.
    pub fn set_selected_index(&mut self, index: usize, item_count: usize) {
        self.selected_index = clamp_selection(index, item_count);
    }

    /// Get scroll offset.
    pub fn scroll_offset(&self) -> usize {
        self.scroll_offset
    }

    /// Set scroll offset.
    pub fn set_scroll_offset(&mut self, offset: usize) {
        self.scroll_offset = offset;
    }

    /// Check if grouping is enabled.
    pub fn is_grouped(&self) -> bool {
        self.show_grouped
    }

    /// Toggle grouping on/off.
    pub fn toggle_grouping(&mut self) {
        self.show_grouped = !self.show_grouped;
    }

    /// Reset selection and scroll to top.
    pub fn reset(&mut self) {
        self.selected_index = 0;
        self.scroll_offset = 0;
    }
}

// ============================================================================
// PURE FUNCTIONS
// ============================================================================

/// Clamps selection index to valid range (pure).
fn clamp_selection(index: usize, item_count: usize) -> usize {
    if item_count == 0 {
        0
    } else {
        index.min(item_count - 1)
    }
}

/// Calculates visible range for scrolling (pure).
pub fn calculate_visible_range(
    scroll_offset: usize,
    viewport_height: usize,
    total_items: usize,
) -> std::ops::Range<usize> {
    let start = scroll_offset;
    let end = (scroll_offset + viewport_height).min(total_items);
    start..end
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clamp_selection_empty() {
        assert_eq!(clamp_selection(5, 0), 0);
    }

    #[test]
    fn test_clamp_selection_within_bounds() {
        assert_eq!(clamp_selection(3, 10), 3);
    }

    #[test]
    fn test_clamp_selection_exceeds_bounds() {
        assert_eq!(clamp_selection(15, 10), 9);
    }

    #[test]
    fn test_toggle_grouping() {
        let mut state = ListState::default();
        assert!(state.is_grouped());
        state.toggle_grouping();
        assert!(!state.is_grouped());
    }
}
```

**Phase 2: Extract QueryState**

```rust
// src/tui/results/query_state.rs

use crate::priority::UnifiedAnalysis;
use super::{filter::Filter, search::SearchState, sort::SortCriteria};

/// Manages filtering, sorting, and search state.
///
/// Operates on analysis data without owning it.
#[derive(Debug)]
pub struct QueryState {
    filtered_indices: Vec<usize>,
    search: SearchState,
    filters: Vec<Filter>,
    sort_by: SortCriteria,
}

impl QueryState {
    /// Create new query state for given item count.
    pub fn new(item_count: usize) -> Self {
        Self {
            filtered_indices: (0..item_count).collect(),
            search: SearchState::new(),
            filters: Vec::new(),
            sort_by: SortCriteria::Score,
        }
    }

    /// Get filtered indices.
    pub fn filtered_indices(&self) -> &[usize] {
        &self.filtered_indices
    }

    /// Get search state reference.
    pub fn search(&self) -> &SearchState {
        &self.search
    }

    /// Get mutable search state reference.
    pub fn search_mut(&mut self) -> &mut SearchState {
        &mut self.search
    }

    /// Get active filters.
    pub fn filters(&self) -> &[Filter] {
        &self.filters
    }

    /// Get current sort criteria.
    pub fn sort_by(&self) -> SortCriteria {
        self.sort_by
    }

    /// Set sort criteria and re-sort.
    pub fn set_sort_by(&mut self, criteria: SortCriteria, analysis: &UnifiedAnalysis) {
        self.sort_by = criteria;
        self.apply_sort(analysis);
    }

    /// Apply current search filter.
    pub fn apply_search(&mut self, analysis: &UnifiedAnalysis) {
        let query = self.search.query();
        self.filtered_indices = if query.is_empty() {
            (0..analysis.items.len()).collect()
        } else {
            super::search::filter_items(analysis, query)
        };
        self.apply_filters(analysis);
        self.apply_sort(analysis);
    }

    /// Add a filter.
    pub fn add_filter(&mut self, filter: Filter, analysis: &UnifiedAnalysis) {
        self.filters.push(filter);
        self.reapply_all(analysis);
    }

    /// Remove a filter by index.
    pub fn remove_filter(&mut self, index: usize, analysis: &UnifiedAnalysis) {
        if index < self.filters.len() {
            self.filters.remove(index);
            self.reapply_all(analysis);
        }
    }

    /// Clear all filters.
    pub fn clear_filters(&mut self, analysis: &UnifiedAnalysis) {
        self.filters.clear();
        self.reapply_all(analysis);
    }

    // ========================================================================
    // PRIVATE HELPERS
    // ========================================================================

    fn apply_filters(&mut self, analysis: &UnifiedAnalysis) {
        if self.filters.is_empty() {
            return;
        }
        self.filtered_indices.retain(|&idx| {
            analysis.items.get(idx)
                .map(|item| self.filters.iter().all(|f| f.matches(item)))
                .unwrap_or(false)
        });
    }

    fn apply_sort(&mut self, analysis: &UnifiedAnalysis) {
        super::sort::sort_indices(&mut self.filtered_indices, analysis, self.sort_by);
    }

    fn reapply_all(&mut self, analysis: &UnifiedAnalysis) {
        self.apply_filters(analysis);
        self.apply_sort(analysis);
    }
}
```

**Phase 3: Enhance NavigationState**

The existing `nav_state.rs` module already exists. Extend it with:

```rust
// src/tui/results/nav_state.rs (enhanced)

use super::app::{ViewMode, DetailPage};

/// Manages view mode, detail page navigation, and history.
#[derive(Debug, Clone)]
pub struct NavigationState {
    pub view_mode: ViewMode,
    pub detail_page: DetailPage,
    pub nav_history: Vec<ViewMode>,
    pub dsm_enabled: bool,
    pub dsm_scroll_x: usize,
    pub dsm_scroll_y: usize,
}

impl Default for NavigationState {
    fn default() -> Self {
        Self {
            view_mode: ViewMode::List,
            detail_page: DetailPage::Overview,
            nav_history: Vec::new(),
            dsm_enabled: true,
            dsm_scroll_x: 0,
            dsm_scroll_y: 0,
        }
    }
}

impl NavigationState {
    /// Push current view mode to history before transitioning.
    pub fn push_and_set_view(&mut self, new_mode: ViewMode) {
        self.nav_history.push(self.view_mode);
        self.view_mode = new_mode;
    }

    /// Go back to previous view mode.
    pub fn go_back(&mut self) -> Option<ViewMode> {
        self.nav_history.pop().map(|mode| {
            self.view_mode = mode;
            mode
        })
    }

    /// Clear navigation history.
    pub fn clear_history(&mut self) {
        self.nav_history.clear();
    }

    /// Reset DSM scroll position.
    pub fn reset_dsm_scroll(&mut self) {
        self.dsm_scroll_x = 0;
        self.dsm_scroll_y = 0;
    }
}
```

**Phase 4: Refactor ResultsApp as Coordinator**

```rust
// src/tui/results/app.rs (refactored)

use crate::priority::{UnifiedAnalysis, UnifiedDebtItem};
use anyhow::Result;
use crossterm::event::KeyEvent;
use ratatui::Frame;

use super::{
    list_state::ListState,
    query_state::QueryState,
    nav_state::NavigationState,
    // ... other imports
};

// ViewMode and DetailPage enums stay here or move to nav_state.rs
// (keeping here for backward compatibility initially)

/// Main application state - slim coordinator.
///
/// Composes ListState, QueryState, and NavigationState modules.
pub struct ResultsApp {
    // Core data (owned)
    analysis: UnifiedAnalysis,

    // Composed state modules
    list: ListState,
    query: QueryState,
    nav: NavigationState,

    // UI state (stays here - minimal)
    terminal_size: (u16, u16),
    needs_redraw: bool,
    status_message: Option<String>,
}

impl ResultsApp {
    /// Create new application state.
    pub fn new(analysis: UnifiedAnalysis) -> Self {
        let item_count = analysis.items.len();
        Self {
            analysis,
            list: ListState::default(),
            query: QueryState::new(item_count),
            nav: NavigationState::default(),
            terminal_size: (80, 24),
            needs_redraw: false,
            status_message: None,
        }
    }

    // ========================================================================
    // DELEGATION TO LIST STATE
    // ========================================================================

    pub fn selected_index(&self) -> usize {
        self.list.selected_index()
    }

    pub fn set_selected_index(&mut self, index: usize) {
        self.list.set_selected_index(index, self.item_count());
    }

    pub fn scroll_offset(&self) -> usize {
        self.list.scroll_offset()
    }

    pub fn set_scroll_offset(&mut self, offset: usize) {
        self.list.set_scroll_offset(offset);
    }

    pub fn is_grouped(&self) -> bool {
        self.list.is_grouped()
    }

    pub fn toggle_grouping(&mut self) {
        self.list.toggle_grouping();
    }

    // ========================================================================
    // DELEGATION TO QUERY STATE
    // ========================================================================

    pub fn filters(&self) -> &[super::filter::Filter] {
        self.query.filters()
    }

    pub fn sort_by(&self) -> super::sort::SortCriteria {
        self.query.sort_by()
    }

    pub fn set_sort_by(&mut self, criteria: super::sort::SortCriteria) {
        self.query.set_sort_by(criteria, &self.analysis);
    }

    pub fn search(&self) -> &super::search::SearchState {
        self.query.search()
    }

    pub fn search_mut(&mut self) -> &mut super::search::SearchState {
        self.query.search_mut()
    }

    pub fn apply_search(&mut self) {
        self.query.apply_search(&self.analysis);
        self.list.reset();
    }

    pub fn add_filter(&mut self, filter: super::filter::Filter) {
        self.query.add_filter(filter, &self.analysis);
        self.list.reset();
    }

    // ========================================================================
    // DELEGATION TO NAVIGATION STATE
    // ========================================================================

    pub fn view_mode(&self) -> ViewMode {
        self.nav.view_mode
    }

    pub fn set_view_mode(&mut self, mode: ViewMode) {
        self.nav.view_mode = mode;
    }

    pub fn detail_page(&self) -> DetailPage {
        self.nav.detail_page
    }

    pub fn set_detail_page(&mut self, page: DetailPage) {
        self.nav.detail_page = page;
    }

    // ... remaining delegation methods

    // ========================================================================
    // COORDINATOR METHODS
    // ========================================================================

    pub fn handle_key(&mut self, key: KeyEvent) -> Result<bool> {
        super::navigation::handle_key(self, key)
    }

    pub fn render(&mut self, frame: &mut Frame) {
        self.terminal_size = (frame.area().width, frame.area().height);
        // ... render logic
    }

    // ... remaining coordinator methods
}
```

### Architecture Changes

**Before:**
```
ResultsApp (God Object)
├── 17 fields
├── 58 methods
└── 9 responsibilities
```

**After:**
```
ResultsApp (Coordinator)
├── analysis (owned data)
├── ListState (composed)
│   ├── 3 fields
│   └── ~8 methods
├── QueryState (composed)
│   ├── 4 fields
│   └── ~12 methods
├── NavigationState (composed)
│   ├── 6 fields
│   └── ~15 methods
└── 3 UI fields
```

### Data Structures

No new external data structures. Internal restructuring only.

### APIs and Interfaces

**Public API Preserved:**
- All existing `ResultsApp` methods remain available
- Same signatures, same behavior
- Backward compatible for all callers

**New Internal APIs:**
- `ListState::new()`, `ListState::default()`
- `QueryState::new(item_count)`
- `NavigationState::default()`

## Dependencies

- **Prerequisites**: None
- **Affected Components**:
  - `src/tui/results/app.rs` (major refactor)
  - `src/tui/results/list_state.rs` (new file)
  - `src/tui/results/query_state.rs` (new file)
  - `src/tui/results/nav_state.rs` (enhanced)
  - `src/tui/results/mod.rs` (exports)
- **External Dependencies**: None

## Testing Strategy

### Unit Tests (Per Module)

**ListState Tests:**
```rust
#[test]
fn test_selection_bounds() { ... }

#[test]
fn test_grouping_toggle() { ... }

#[test]
fn test_reset() { ... }
```

**QueryState Tests:**
```rust
#[test]
fn test_filter_application() { ... }

#[test]
fn test_sort_criteria_change() { ... }

#[test]
fn test_search_filtering() { ... }
```

**NavigationState Tests:**
```rust
#[test]
fn test_history_push_pop() { ... }

#[test]
fn test_view_mode_transitions() { ... }

#[test]
fn test_dsm_scroll_reset() { ... }
```

### Integration Tests

```rust
#[test]
fn test_results_app_preserves_behavior() {
    // Ensure refactored app behaves identically to original
}

#[test]
fn test_filter_then_sort_then_select() {
    // Cross-module interaction test
}
```

### Property-Based Tests

```rust
proptest! {
    #[test]
    fn test_selection_always_valid(
        index in 0usize..1000,
        count in 0usize..100
    ) {
        let clamped = clamp_selection(index, count);
        prop_assert!(clamped < count || (count == 0 && clamped == 0));
    }
}
```

## Documentation Requirements

### Code Documentation

- Each module has module-level documentation
- Each public function has doc comments
- Pure functions marked with `/// Pure function - ...`
- Examples in doc comments for key functions

### Architecture Updates

Add section to `ARCHITECTURE.md`:

```markdown
## TUI State Management

The TUI uses a composed state architecture:

- **ListState**: Selection, scroll, grouping
- **QueryState**: Filter, sort, search
- **NavigationState**: View modes, pages, history
- **ResultsApp**: Slim coordinator

This follows the single responsibility principle and enables
isolated testing of each state module.
```

## Implementation Notes

### Refactoring Order

1. Create `list_state.rs` with tests
2. Create `query_state.rs` with tests
3. Enhance `nav_state.rs` with tests
4. Refactor `ResultsApp` to compose modules
5. Update `mod.rs` exports
6. Verify all tests pass
7. Run clippy and fix warnings

### Common Pitfalls

1. **Borrow checker issues** - Be careful with `&self.analysis` in query methods
2. **Circular dependencies** - Ensure clean module boundaries
3. **State synchronization** - Reset list state when query changes
4. **API compatibility** - Keep all public methods working

### Migration Notes

Internal refactoring only. No external migration needed.

## Migration and Compatibility

### Breaking Changes

**None** - Internal refactoring preserves all public APIs.

### Migration Steps

No user or developer migration needed. Internal improvement only.

## Success Metrics

| Metric | Before | Target | Weight |
|--------|--------|--------|--------|
| Debt Score | 100.0 | <30 | High |
| Test Coverage | 20.9% | >80% | High |
| Max LOC/Module | 600+ | <150 | Medium |
| Fields/Struct | 17 | 5-6 | Medium |
| Methods/Impl | 58 | 10-15 | Medium |
| Responsibilities | 9 | 1 | High |

## Follow-up Work

After this implementation:
- Apply same patterns to other TUI components if needed
- Consider extracting pure rendering functions
- Improve TUI test coverage further

## References

- **Stillwater Philosophy** - Pure core, composition over complexity
- **CLAUDE.md** - Function design guidelines (max 20 lines)
- **Spec 187** - Extract pure functions pattern (similar approach)
- **Debtmap self-analysis** - God Object detection at line 111
