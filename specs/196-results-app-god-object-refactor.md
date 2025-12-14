---
number: 196
title: Refactor ResultsApp God Object to Expose Inner State Modules
category: optimization
priority: high
status: draft
dependencies: []
created: 2025-12-13
---

# Specification 196: Refactor ResultsApp God Object to Expose Inner State Modules

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: none

## Context

The `src/tui/results/app.rs` file has been flagged by debtmap as a God Object with critical technical debt:

```
evaluate location
  file                      ./src/tui/results/app.rs
  function                  app.rs
  line                      1

score
  total                     100.0  [critical]

god file structure
  functions                 56
  responsibilities          9
  loc                       473

complexity
  accumulated cyclomatic    76
  accumulated cognitive     42 → 21 (dampened)
  max nesting               1

coverage
  coverage                  9.6%

recommendation
  action                    Split into 5 modules by responsibility
```

### Current Problem

The `ResultsApp` struct has 56 methods but only ~6 are genuine coordination. The rest are **delegation boilerplate** that just forward calls to inner state modules which already exist:

| Responsibility | Methods | Already Handled By |
|----------------|---------|-------------------|
| List state delegation | 8 methods | `ListState` (list_state.rs) |
| Query state delegation | 10 methods | `QueryState` (query_state.rs) |
| Navigation delegation | 14 methods | `NavigationState` (nav_state.rs) |
| Page availability | 6 methods | `page_availability` module |
| UI state | 6 methods | Local fields |
| Construction | 2 methods | Essential |
| Coordination | 3 methods | Essential (`handle_key`, `render`, `analysis`) |
| Data access | 3 methods | Essential |
| Navigation helpers | 4 methods | Computed from state |

### Root Cause

The pattern of hiding internal state behind delegation creates 50+ passthrough methods. Each inner module already has its own well-designed API with proper methods.

**Example of redundant delegation:**

```rust
// Current: 50+ methods like this in app.rs
pub fn selected_index(&self) -> usize { self.list.selected_index() }
pub fn scroll_offset(&self) -> usize { self.list.scroll_offset() }
pub fn view_mode(&self) -> ViewMode { self.nav.view_mode }
pub fn search(&self) -> &SearchState { self.query.search() }
// ... 46 more passthrough methods
```

The inner modules (`ListState`, `QueryState`, `NavigationState`) are already well-designed with:
- Pure state containers
- Proper encapsulation
- Comprehensive tests
- Clear APIs

### Stillwater Philosophy Alignment

Following the Stillwater philosophy:

1. **Pure Core, Imperative Shell** - The inner state modules are already pure state containers
2. **Composition Over Complexity** - Expose components directly instead of hiding behind delegation
3. **Pragmatism Over Purity** - Don't create 50 wrapper methods when direct access is cleaner
4. **Types Guide, Don't Restrict** - Let callers access what they need without boilerplate

## Objective

Refactor `ResultsApp` from a God Object with 56 methods to a slim coordinator with ~15 essential methods by:

1. **Exposing inner state modules** via accessor methods (`list()`, `query()`, `nav()`)
2. **Removing 40+ delegation methods** that just forward to inner modules
3. **Keeping only essential coordination** that requires multiple modules
4. **Updating all call sites** to use direct state access

**Target State:**

```rust
// After: ~15 essential methods
impl ResultsApp {
    // Construction
    pub fn new(analysis: UnifiedAnalysis) -> Self { ... }
    pub fn from_prepared_view(view: PreparedDebtView, analysis: UnifiedAnalysis) -> Self { ... }

    // State accessors
    pub fn list(&self) -> &ListState { &self.list }
    pub fn list_mut(&mut self) -> &mut ListState { &mut self.list }
    pub fn query(&self) -> &QueryState { &self.query }
    pub fn query_mut(&mut self) -> &mut QueryState { &mut self.query }
    pub fn nav(&self) -> &NavigationState { &self.nav }
    pub fn nav_mut(&mut self) -> &mut NavigationState { &mut self.nav }

    // Core data
    pub fn analysis(&self) -> &UnifiedAnalysis { &self.analysis }

    // Coordination (requires multiple modules)
    pub fn handle_key(&mut self, key: KeyEvent) -> Result<bool> { ... }
    pub fn render(&mut self, frame: &mut Frame) { ... }
    pub fn selected_item(&self) -> Option<&UnifiedDebtItem> { ... }
    pub fn filtered_items(&self) -> impl Iterator<Item = &UnifiedDebtItem> { ... }
    pub fn item_count(&self) -> usize { ... }

    // UI state (minimal, stays here)
    pub fn terminal_size(&self) -> (u16, u16) { ... }
    pub fn request_redraw(&mut self) { ... }
    pub fn set_status_message(&mut self, message: String) { ... }
}
```

## Requirements

### Functional Requirements

1. **Add State Accessor Methods**
   - `pub fn list(&self) -> &ListState`
   - `pub fn list_mut(&mut self) -> &mut ListState`
   - `pub fn query(&self) -> &QueryState`
   - `pub fn query_mut(&mut self) -> &mut QueryState`
   - `pub fn nav(&self) -> &NavigationState`
   - `pub fn nav_mut(&mut self) -> &mut NavigationState`

2. **Remove Delegation Methods**
   Remove these passthrough methods (they just forward to accessors):

   **List delegation (remove 8 methods):**
   - `selected_index()` → Use `app.list().selected_index()`
   - `set_selected_index()` → Use `app.list_mut().set_selected_index()`
   - `scroll_offset()` → Use `app.list().scroll_offset()`
   - `set_scroll_offset()` → Use `app.list_mut().set_scroll_offset()`
   - `toggle_grouping()` → Use `app.list_mut().toggle_grouping()`
   - `is_grouped()` → Use `app.list().is_grouped()`

   **Query delegation (remove 10 methods):**
   - `search()` → Use `app.query().search()`
   - `search_mut()` → Use `app.query_mut().search_mut()`
   - `filters()` → Use `app.query().filters()`
   - `sort_by()` → Use `app.query().sort_by()`
   - `set_sort_by()` → Use `app.query_mut().set_sort_by()`
   - `apply_search()` → Use `app.query_mut().apply_search()`
   - `add_filter()` → Use `app.query_mut().add_filter()`
   - `remove_filter()` → Use `app.query_mut().remove_filter()`
   - `clear_filters()` → Use `app.query_mut().clear_filters()`

   **Navigation delegation (remove 14 methods):**
   - `view_mode()` → Use `app.nav().view_mode`
   - `set_view_mode()` → Use `app.nav_mut().view_mode = mode`
   - `detail_page()` → Use `app.nav().detail_page`
   - `set_detail_page()` → Use `app.nav_mut().detail_page = page`
   - `nav_history()` → Use `app.nav().history`
   - `push_nav_history()` → Use `app.nav_mut().history.push()`
   - `pop_nav_history()` → Use `app.nav_mut().history.pop()`
   - `clear_nav_history()` → Use `app.nav_mut().clear_history()`
   - `dsm_enabled()` → Use `app.nav().dsm_enabled`
   - `dsm_scroll_x()` → Use `app.nav().dsm_scroll_x`
   - `set_dsm_scroll_x()` → Use `app.nav_mut().dsm_scroll_x = offset`
   - `dsm_scroll_y()` → Use `app.nav().dsm_scroll_y`
   - `set_dsm_scroll_y()` → Use `app.nav_mut().dsm_scroll_y = offset`

   **Page availability delegation (remove 6 methods):**
   - `available_pages()` → Call `page_availability::available_pages()` directly
   - `page_count()` → Call `page_availability::available_pages().len()`
   - `current_page_index()` → Inline or move to page_availability module
   - `next_available_page()` → Call `page_availability::next_available_page()` directly
   - `prev_available_page()` → Call `page_availability::prev_available_page()` directly
   - `is_page_available()` → Call `page_availability::is_page_available()` directly
   - `ensure_valid_page()` → Call `page_availability::ensure_valid_page()` directly

3. **Keep Essential Methods**
   These methods genuinely coordinate multiple modules and should stay:
   - `new()`, `from_prepared_view()` - Construction
   - `handle_key()` - Coordination via navigation module
   - `render()` - Dispatches to view renderers
   - `analysis()` - Core data access
   - `selected_item()` - Requires list + query + analysis
   - `filtered_items()` - Requires query + analysis
   - `item_count()` - Requires list + query state
   - `terminal_size()`, `request_redraw()`, `take_needs_redraw()` - UI state
   - `set_status_message()`, `status_message()`, `clear_status_message()` - UI state
   - `has_selection()`, `has_items()` - Navigation guards (or move to pure functions)
   - `count_display()` - Display helper

4. **Update All Call Sites**
   All files that call the removed methods must be updated:
   - `src/tui/results/navigation.rs` (~60 call sites)
   - `src/tui/results/list_view.rs`
   - `src/tui/results/detail_view.rs`
   - `src/tui/results/dsm_view.rs`
   - `src/tui/results/layout.rs`
   - `src/tui/results/actions/*.rs`

### Non-Functional Requirements

1. **Reduced Complexity**: Method count from 56 to ~15
2. **Improved Clarity**: Dependencies explicitly visible at call sites
3. **Better Testability**: State modules can be tested independently
4. **No Behavior Change**: All functionality preserved
5. **Backwards Compatibility for Public API**: `ResultsApp::new()` and `ResultsApp::from_prepared_view()` unchanged

## Acceptance Criteria

- [ ] `ResultsApp` has accessor methods: `list()`, `list_mut()`, `query()`, `query_mut()`, `nav()`, `nav_mut()`
- [ ] ~40 delegation methods removed from `ResultsApp`
- [ ] `ResultsApp` has ≤20 methods total
- [ ] All call sites updated to use direct state access
- [ ] `src/tui/results/navigation.rs` updated with new access patterns
- [ ] All existing tests pass
- [ ] `cargo clippy` passes with no new warnings
- [ ] `cargo fmt` applied
- [ ] debtmap self-analysis shows improved score for `app.rs`

## Technical Details

### Implementation Approach

#### Phase 1: Add State Accessors

Add the new accessor methods without removing anything:

```rust
impl ResultsApp {
    // NEW: State accessors
    pub fn list(&self) -> &ListState { &self.list }
    pub fn list_mut(&mut self) -> &mut ListState { &mut self.list }
    pub fn query(&self) -> &QueryState { &self.query }
    pub fn query_mut(&mut self) -> &mut QueryState { &mut self.query }
    pub fn nav(&self) -> &NavigationState { &self.nav }
    pub fn nav_mut(&mut self) -> &mut NavigationState { &mut self.nav }

    // Existing methods unchanged for now
}
```

#### Phase 2: Update Call Sites

Update each file to use new patterns. Example transformations:

```rust
// navigation.rs - Before
app.set_selected_index(0);
app.toggle_grouping();
app.set_view_mode(ViewMode::Detail);
app.search_mut().clear();

// navigation.rs - After
app.list_mut().set_selected_index(0, app.item_count());
app.list_mut().toggle_grouping();
app.nav_mut().view_mode = ViewMode::Detail;
app.query_mut().search_mut().clear();
```

#### Phase 3: Remove Delegation Methods

After all call sites are updated, remove the delegation methods from `ResultsApp`.

#### Phase 4: Extract Pure Navigation Helpers

Move remaining navigation helpers to pure functions if they don't need `self`:

```rust
// Move from ResultsApp methods to standalone functions in nav_state.rs
pub fn has_selection(list: &ListState, item_count: usize) -> bool {
    list.selected_index() < item_count
}

pub fn has_items(item_count: usize) -> bool {
    item_count > 0
}
```

### Files to Modify

| File | Changes |
|------|---------|
| `src/tui/results/app.rs` | Add accessors, remove delegation methods |
| `src/tui/results/navigation.rs` | Update ~60 call sites |
| `src/tui/results/list_view.rs` | Update rendering calls |
| `src/tui/results/detail_view.rs` | Update rendering calls |
| `src/tui/results/dsm_view.rs` | Update DSM scroll access |
| `src/tui/results/layout.rs` | Update layout rendering |
| `src/tui/results/actions/text_extraction.rs` | Update action calls |
| `src/tui/results/nav_state.rs` | Add pure helper functions |

### Call Site Migration Guide

**Pattern 1: Simple Getter**
```rust
// Before                    // After
app.selected_index()         app.list().selected_index()
app.view_mode()              app.nav().view_mode
app.sort_by()                app.query().sort_by()
```

**Pattern 2: Simple Setter**
```rust
// Before                           // After
app.set_view_mode(mode)             app.nav_mut().view_mode = mode
app.set_detail_page(page)           app.nav_mut().detail_page = page
app.set_scroll_offset(offset)       app.list_mut().set_scroll_offset(offset)
```

**Pattern 3: Method That Needs Analysis**
```rust
// Before
app.set_sort_by(criteria);

// After - QueryState.set_sort_by needs analysis reference
app.query_mut().set_sort_by(criteria, app.analysis());
```

**Pattern 4: Chained Access**
```rust
// Before
app.search_mut().clear();

// After
app.query_mut().search_mut().clear();
```

**Pattern 5: Complex Method (Keep in ResultsApp)**
```rust
// These stay as ResultsApp methods because they need multiple internal states
app.selected_item()    // Needs list + query + analysis
app.filtered_items()   // Needs query + analysis
app.item_count()       // Needs list + query
```

## Dependencies

- **Prerequisites**: None
- **Affected Components**:
  - `src/tui/results/app.rs` - Main refactoring target
  - `src/tui/results/navigation.rs` - Largest consumer
  - `src/tui/results/*.rs` - All view modules
  - `src/tui/results/actions/*.rs` - Action handlers
- **External Dependencies**: None

## Testing Strategy

### Unit Tests

Existing tests in `list_state.rs`, `query_state.rs`, and `nav_state.rs` should continue to pass unchanged as we're not modifying the inner modules.

### Integration Tests

The TUI integration tests should pass as behavior is preserved.

### Manual Testing

- Launch TUI with `cargo run -- analyze . --tui`
- Verify all navigation works (list scrolling, detail view, search, sort, filter)
- Verify DSM view if enabled
- Verify help overlay
- Verify keyboard shortcuts

### Regression Testing

```bash
# Run all tests
cargo test

# Run TUI-specific tests
cargo test --lib tui::

# Run clippy
cargo clippy -- -D warnings

# Format check
cargo fmt -- --check
```

## Documentation Requirements

- **Code Documentation**: Update doc comments on `ResultsApp` to reflect new accessor pattern
- **User Documentation**: None (internal refactoring)
- **Architecture Updates**: Note the composition pattern in ARCHITECTURE.md

## Implementation Notes

### Why Not Make Fields Public?

We use accessor methods instead of public fields because:
1. Maintains encapsulation for future changes
2. Allows adding logging/tracing if needed
3. Consistent with Rust API design guidelines
4. Allows `&mut` vs `&` distinction

### Special Cases

**`set_selected_index` needs item_count:**
```rust
// ListState.set_selected_index requires item_count for bounds checking
// Either pass it explicitly or compute it in the call site:
app.list_mut().set_selected_index(index, app.item_count());
```

**Query operations need analysis reference:**
```rust
// QueryState methods like apply_search need the analysis:
app.query_mut().apply_search(app.analysis());
```

### Estimated Impact

| Metric | Before | After |
|--------|--------|-------|
| Methods in ResultsApp | 56 | ~15 |
| Lines in app.rs | 473 | ~200 |
| Responsibilities | 9 | 3-4 |
| Debtmap score | 100 (critical) | <30 (low) |

## Migration and Compatibility

- **Breaking Changes**: None for public API
- **Internal Changes**: Call sites within TUI module updated
- **Serialization**: Not affected (ResultsApp not serialized)
