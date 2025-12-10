---
number: 203
title: TUI Navigation State Machine with Mindset
category: optimization
priority: medium
status: draft
dependencies: []
created: 2025-12-10
---

# Specification 203: TUI Navigation State Machine with Mindset

**Category**: optimization
**Priority**: medium
**Status**: draft
**Dependencies**: None

## Context

The TUI results viewer (`src/tui/results/app.rs`) already has good enum-based state tracking for `ViewMode` and `DetailPage`. However, transitions between states are implicit and not validated:

**Current State:**
```rust
// src/tui/results/app.rs
pub enum ViewMode {
    List,
    Detail,
    Search,
    SortMenu,
    FilterMenu,
    Help,
    Dsm,
}

pub enum DetailPage {
    Overview,
    Dependencies,
    GitContext,
    Patterns,
    DataFlow,
    Responsibilities,
}

impl ResultsApp {
    // Direct state mutation - no validation
    fn handle_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Enter => {
                if !self.items.is_empty() {
                    self.view_mode = ViewMode::Detail;  // No guard!
                    self.detail_page = DetailPage::Overview;
                }
            }
            KeyCode::Esc => {
                self.view_mode = ViewMode::List;  // Always allowed?
            }
            // ... more transitions
        }
    }
}
```

**Problems:**
1. **No transition validation** - Any state can transition to any other
2. **Implicit transitions** - Hard to see valid navigation paths
3. **No documentation** - Valid transitions not explicit
4. **Testing difficulty** - Can't test transition logic separately
5. **Potential invalid states** - Could accidentally allow invalid transitions

**With mindset:**
```rust
// Valid transitions explicitly defined
const TRANSITIONS: &[(ViewMode, ViewMode)] = &[
    (ViewMode::List, ViewMode::Detail),
    (ViewMode::List, ViewMode::Search),
    (ViewMode::List, ViewMode::SortMenu),
    // ...
];

// Pure guard
fn can_enter_detail(state: &TuiState) -> bool {
    matches!(state.view_mode, ViewMode::List)
        && !state.items.is_empty()
        && state.selected_index.is_some()
}
```

## Objective

Use **mindset** patterns to make TUI navigation explicit and testable:

1. **Explicit transitions** - Define valid state transitions
2. **Pure guards** - Validate transitions with pure functions
3. **Documented flow** - Clear navigation graph
4. **Testable** - Unit test navigation logic without TUI

**Success Metric**: All valid navigation paths documented and tested; invalid transitions prevented.

## Requirements

### Functional Requirements

1. **Define Navigation States**
   - `ViewMode` - Main view states (existing)
   - `DetailPage` - Detail sub-pages (existing)
   - `NavigationState` - Combined state with history

2. **Define Valid Transitions**
   - List → Detail (when item selected)
   - List → Search (always)
   - List → SortMenu (always)
   - List → FilterMenu (always)
   - List → Help (always)
   - List → Dsm (when enabled)
   - Detail → List (back)
   - Detail → DetailPage transitions (left/right)
   - Search → List (escape or enter)
   - Menu → List (escape or select)
   - Help → Previous (escape)

3. **Implement Pure Guards**
   - `can_enter_detail(state)` - Item must be selected
   - `can_enter_dsm(state)` - DSM feature enabled
   - `can_navigate_detail(state, direction)` - Valid page bounds
   - Each guard is pure, testable

4. **Track Navigation History**
   - Remember previous state for "back" navigation
   - Support multi-level back (Help from Detail → Detail → List)

5. **Visual Feedback**
   - Show available actions based on valid transitions
   - Disable/hide unavailable navigation options

### Non-Functional Requirements

1. **Responsiveness** - Transition validation < 1ms
2. **Clarity** - Navigation paths clear to users
3. **Testability** - All guards unit testable
4. **Extensibility** - Easy to add new views

## Acceptance Criteria

- [ ] `NavigationState` struct combining view mode, detail page, and history
- [ ] Transition table defining valid `(from, to)` pairs
- [ ] Pure guard functions for each conditional transition
- [ ] Navigation history for back navigation
- [ ] Unit tests for all guards
- [ ] Integration tests for navigation flows
- [ ] Documentation of navigation graph
- [ ] Existing TUI behavior unchanged

## Technical Details

### Implementation Approach

**Phase 1: Define Navigation Types**

```rust
// src/tui/results/navigation.rs
use mindset::{State, state_enum};

state_enum! {
    /// Main view modes for TUI results viewer.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum ViewMode {
        /// List of debt recommendations.
        List,

        /// Detailed view of single recommendation.
        Detail,

        /// Search/filter interface.
        Search,

        /// Sort options menu.
        SortMenu,

        /// Filter options menu.
        FilterMenu,

        /// Help overlay.
        Help,

        /// Dependency Structure Matrix view.
        Dsm,
    }

    // No final states - TUI runs until quit
    final: []
}

state_enum! {
    /// Detail view sub-pages.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum DetailPage {
        Overview,
        Dependencies,
        GitContext,
        Patterns,
        DataFlow,
        Responsibilities,
    }
    final: []
}

impl DetailPage {
    /// All pages in navigation order.
    pub const ALL: &'static [DetailPage] = &[
        DetailPage::Overview,
        DetailPage::Dependencies,
        DetailPage::GitContext,
        DetailPage::Patterns,
        DetailPage::DataFlow,
        DetailPage::Responsibilities,
    ];

    /// Navigate to next page (wrapping).
    pub fn next(self) -> Self {
        let idx = Self::ALL.iter().position(|&p| p == self).unwrap();
        Self::ALL[(idx + 1) % Self::ALL.len()]
    }

    /// Navigate to previous page (wrapping).
    pub fn prev(self) -> Self {
        let idx = Self::ALL.iter().position(|&p| p == self).unwrap();
        Self::ALL[(idx + Self::ALL.len() - 1) % Self::ALL.len()]
    }

    /// Page index (0-based).
    pub fn index(self) -> usize {
        Self::ALL.iter().position(|&p| p == self).unwrap()
    }
}

/// Complete navigation state.
#[derive(Debug, Clone)]
pub struct NavigationState {
    /// Current view mode.
    pub view_mode: ViewMode,

    /// Current detail page (when in Detail mode).
    pub detail_page: DetailPage,

    /// Navigation history for back navigation.
    pub history: Vec<ViewMode>,

    /// Whether DSM feature is enabled.
    pub dsm_enabled: bool,
}

impl NavigationState {
    pub fn new(dsm_enabled: bool) -> Self {
        Self {
            view_mode: ViewMode::List,
            detail_page: DetailPage::Overview,
            history: vec![],
            dsm_enabled,
        }
    }
}
```

**Phase 2: Define Transition Table**

```rust
// src/tui/results/transitions.rs

use super::navigation::{ViewMode, NavigationState};

/// Valid navigation transitions.
///
/// This table defines ALL allowed transitions between view modes.
/// Any transition not in this table is invalid.
pub const TRANSITIONS: &[(ViewMode, ViewMode)] = &[
    // From List
    (ViewMode::List, ViewMode::Detail),
    (ViewMode::List, ViewMode::Search),
    (ViewMode::List, ViewMode::SortMenu),
    (ViewMode::List, ViewMode::FilterMenu),
    (ViewMode::List, ViewMode::Help),
    (ViewMode::List, ViewMode::Dsm),

    // From Detail
    (ViewMode::Detail, ViewMode::List),
    (ViewMode::Detail, ViewMode::Help),

    // From Search
    (ViewMode::Search, ViewMode::List),
    (ViewMode::Search, ViewMode::Detail),  // Search result selected

    // From SortMenu
    (ViewMode::SortMenu, ViewMode::List),

    // From FilterMenu
    (ViewMode::FilterMenu, ViewMode::List),

    // From Help (returns to previous)
    (ViewMode::Help, ViewMode::List),
    (ViewMode::Help, ViewMode::Detail),
    (ViewMode::Help, ViewMode::Search),

    // From DSM
    (ViewMode::Dsm, ViewMode::List),
    (ViewMode::Dsm, ViewMode::Help),
];

/// Check if a transition is valid based on the table.
pub fn is_valid_transition(from: ViewMode, to: ViewMode) -> bool {
    TRANSITIONS.contains(&(from, to))
}

/// Get all valid destinations from current mode.
pub fn valid_destinations(from: ViewMode) -> Vec<ViewMode> {
    TRANSITIONS
        .iter()
        .filter(|(f, _)| *f == from)
        .map(|(_, t)| *t)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_to_detail_valid() {
        assert!(is_valid_transition(ViewMode::List, ViewMode::Detail));
    }

    #[test]
    fn test_detail_to_search_invalid() {
        // Can't go directly from Detail to Search
        assert!(!is_valid_transition(ViewMode::Detail, ViewMode::Search));
    }

    #[test]
    fn test_valid_destinations_from_list() {
        let destinations = valid_destinations(ViewMode::List);
        assert!(destinations.contains(&ViewMode::Detail));
        assert!(destinations.contains(&ViewMode::Search));
        assert!(destinations.contains(&ViewMode::Help));
    }
}
```

**Phase 3: Pure Guard Functions**

```rust
// src/tui/results/guards.rs

use super::navigation::{ViewMode, NavigationState};

/// Guard: Can enter Detail view?
///
/// Pure function - requires item to be selected.
pub fn can_enter_detail(state: &NavigationState, has_items: bool, selected: Option<usize>) -> bool {
    matches!(state.view_mode, ViewMode::List)
        && has_items
        && selected.is_some()
}

/// Guard: Can enter DSM view?
///
/// Requires DSM feature enabled.
pub fn can_enter_dsm(state: &NavigationState) -> bool {
    matches!(state.view_mode, ViewMode::List | ViewMode::Detail)
        && state.dsm_enabled
}

/// Guard: Can enter Search?
///
/// Only from List view.
pub fn can_enter_search(state: &NavigationState) -> bool {
    matches!(state.view_mode, ViewMode::List)
}

/// Guard: Can enter SortMenu?
pub fn can_enter_sort_menu(state: &NavigationState) -> bool {
    matches!(state.view_mode, ViewMode::List)
}

/// Guard: Can enter FilterMenu?
pub fn can_enter_filter_menu(state: &NavigationState) -> bool {
    matches!(state.view_mode, ViewMode::List)
}

/// Guard: Can enter Help?
///
/// Help is accessible from most views.
pub fn can_enter_help(state: &NavigationState) -> bool {
    !matches!(state.view_mode, ViewMode::Help)
}

/// Guard: Can go back?
///
/// True if there's history to go back to.
pub fn can_go_back(state: &NavigationState) -> bool {
    !state.history.is_empty() || !matches!(state.view_mode, ViewMode::List)
}

/// Guard: Can navigate detail pages?
pub fn can_navigate_detail_pages(state: &NavigationState) -> bool {
    matches!(state.view_mode, ViewMode::Detail)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_can_enter_detail_requires_selection() {
        let state = NavigationState::new(false);

        // No items - can't enter
        assert!(!can_enter_detail(&state, false, None));

        // Items but no selection - can't enter
        assert!(!can_enter_detail(&state, true, None));

        // Items and selection - can enter
        assert!(can_enter_detail(&state, true, Some(0)));
    }

    #[test]
    fn test_can_enter_dsm_requires_feature() {
        let disabled = NavigationState::new(false);
        let enabled = NavigationState::new(true);

        assert!(!can_enter_dsm(&disabled));
        assert!(can_enter_dsm(&enabled));
    }

    #[test]
    fn test_help_not_from_help() {
        let mut state = NavigationState::new(false);
        assert!(can_enter_help(&state));

        state.view_mode = ViewMode::Help;
        assert!(!can_enter_help(&state));
    }

    #[test]
    fn test_guards_are_pure() {
        let state = NavigationState::new(false);

        // Same input → same output
        let r1 = can_enter_detail(&state, true, Some(0));
        let r2 = can_enter_detail(&state, true, Some(0));
        assert_eq!(r1, r2);
    }
}
```

**Phase 4: Navigation Actions**

```rust
// src/tui/results/actions.rs

use super::{guards::*, navigation::*, transitions::*};

/// Result of attempting a navigation action.
#[derive(Debug, Clone)]
pub enum NavigationResult {
    /// Navigation succeeded.
    Success,

    /// Navigation failed - guard rejected.
    Blocked { reason: &'static str },

    /// Navigation invalid - not in transition table.
    Invalid { from: ViewMode, to: ViewMode },
}

/// Navigate to Detail view.
pub fn navigate_to_detail(
    state: &mut NavigationState,
    has_items: bool,
    selected: Option<usize>,
) -> NavigationResult {
    if !is_valid_transition(state.view_mode, ViewMode::Detail) {
        return NavigationResult::Invalid {
            from: state.view_mode,
            to: ViewMode::Detail,
        };
    }

    if !can_enter_detail(state, has_items, selected) {
        return NavigationResult::Blocked {
            reason: "No item selected",
        };
    }

    state.history.push(state.view_mode);
    state.view_mode = ViewMode::Detail;
    state.detail_page = DetailPage::Overview;

    NavigationResult::Success
}

/// Navigate to Search view.
pub fn navigate_to_search(state: &mut NavigationState) -> NavigationResult {
    if !is_valid_transition(state.view_mode, ViewMode::Search) {
        return NavigationResult::Invalid {
            from: state.view_mode,
            to: ViewMode::Search,
        };
    }

    if !can_enter_search(state) {
        return NavigationResult::Blocked {
            reason: "Search only available from List view",
        };
    }

    state.history.push(state.view_mode);
    state.view_mode = ViewMode::Search;

    NavigationResult::Success
}

/// Navigate to DSM view.
pub fn navigate_to_dsm(state: &mut NavigationState) -> NavigationResult {
    if !is_valid_transition(state.view_mode, ViewMode::Dsm) {
        return NavigationResult::Invalid {
            from: state.view_mode,
            to: ViewMode::Dsm,
        };
    }

    if !can_enter_dsm(state) {
        return NavigationResult::Blocked {
            reason: "DSM feature not enabled",
        };
    }

    state.history.push(state.view_mode);
    state.view_mode = ViewMode::Dsm;

    NavigationResult::Success
}

/// Navigate to Help view.
pub fn navigate_to_help(state: &mut NavigationState) -> NavigationResult {
    if !can_enter_help(state) {
        return NavigationResult::Blocked {
            reason: "Already in Help view",
        };
    }

    state.history.push(state.view_mode);
    state.view_mode = ViewMode::Help;

    NavigationResult::Success
}

/// Navigate back (escape).
pub fn navigate_back(state: &mut NavigationState) -> NavigationResult {
    if let Some(previous) = state.history.pop() {
        state.view_mode = previous;
        NavigationResult::Success
    } else if state.view_mode != ViewMode::List {
        state.view_mode = ViewMode::List;
        NavigationResult::Success
    } else {
        NavigationResult::Blocked {
            reason: "Already at root view",
        }
    }
}

/// Navigate detail pages (left/right).
pub fn navigate_detail_page(state: &mut NavigationState, forward: bool) -> NavigationResult {
    if !can_navigate_detail_pages(state) {
        return NavigationResult::Blocked {
            reason: "Not in Detail view",
        };
    }

    state.detail_page = if forward {
        state.detail_page.next()
    } else {
        state.detail_page.prev()
    };

    NavigationResult::Success
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_navigate_to_detail() {
        let mut state = NavigationState::new(false);

        // Without selection - blocked
        let result = navigate_to_detail(&mut state, true, None);
        assert!(matches!(result, NavigationResult::Blocked { .. }));

        // With selection - success
        let result = navigate_to_detail(&mut state, true, Some(0));
        assert!(matches!(result, NavigationResult::Success));
        assert_eq!(state.view_mode, ViewMode::Detail);
    }

    #[test]
    fn test_navigate_back_uses_history() {
        let mut state = NavigationState::new(false);

        // Navigate List → Detail
        navigate_to_detail(&mut state, true, Some(0));
        assert_eq!(state.view_mode, ViewMode::Detail);

        // Navigate Detail → Help
        navigate_to_help(&mut state);
        assert_eq!(state.view_mode, ViewMode::Help);

        // Back goes to Detail (not List)
        navigate_back(&mut state);
        assert_eq!(state.view_mode, ViewMode::Detail);

        // Back goes to List
        navigate_back(&mut state);
        assert_eq!(state.view_mode, ViewMode::List);
    }

    #[test]
    fn test_invalid_transition_rejected() {
        let mut state = NavigationState::new(false);
        state.view_mode = ViewMode::Detail;

        // Detail → Search is not in transition table
        let result = navigate_to_search(&mut state);
        assert!(matches!(result, NavigationResult::Invalid { .. }));
    }
}
```

**Phase 5: Integration with TUI**

```rust
// src/tui/results/app.rs (updated)

use super::navigation::*;
use super::actions::*;

impl ResultsApp {
    /// Handle keyboard input using navigation state machine.
    pub fn handle_key(&mut self, key: KeyCode) -> Option<Action> {
        match key {
            KeyCode::Enter => {
                let result = navigate_to_detail(
                    &mut self.nav,
                    !self.items.is_empty(),
                    self.selected_index,
                );
                match result {
                    NavigationResult::Success => None,
                    NavigationResult::Blocked { reason } => {
                        self.show_message(reason);
                        None
                    }
                    NavigationResult::Invalid { .. } => None,
                }
            }

            KeyCode::Esc => {
                navigate_back(&mut self.nav);
                None
            }

            KeyCode::Char('/') => {
                navigate_to_search(&mut self.nav);
                None
            }

            KeyCode::Char('s') => {
                navigate_to_sort_menu(&mut self.nav);
                None
            }

            KeyCode::Char('f') => {
                navigate_to_filter_menu(&mut self.nav);
                None
            }

            KeyCode::Char('?') => {
                navigate_to_help(&mut self.nav);
                None
            }

            KeyCode::Char('d') => {
                navigate_to_dsm(&mut self.nav);
                None
            }

            KeyCode::Left if self.nav.view_mode == ViewMode::Detail => {
                navigate_detail_page(&mut self.nav, false);
                None
            }

            KeyCode::Right if self.nav.view_mode == ViewMode::Detail => {
                navigate_detail_page(&mut self.nav, true);
                None
            }

            KeyCode::Char('q') => Some(Action::Quit),

            _ => None,
        }
    }

    /// Get available actions for current state (for status bar).
    pub fn available_actions(&self) -> Vec<(&'static str, &'static str)> {
        let mut actions = vec![];

        if can_enter_detail(&self.nav, !self.items.is_empty(), self.selected_index) {
            actions.push(("Enter", "View details"));
        }

        if can_enter_search(&self.nav) {
            actions.push(("/", "Search"));
        }

        if can_enter_dsm(&self.nav) {
            actions.push(("d", "DSM view"));
        }

        if can_enter_help(&self.nav) {
            actions.push(("?", "Help"));
        }

        if can_go_back(&self.nav) {
            actions.push(("Esc", "Back"));
        }

        actions.push(("q", "Quit"));

        actions
    }
}
```

### File Structure

```
src/tui/results/
├── mod.rs           # Re-exports
├── app.rs           # Main TUI app (uses navigation)
├── navigation.rs    # ViewMode, DetailPage, NavigationState
├── transitions.rs   # Valid transition table
├── guards.rs        # Pure guard functions
├── actions.rs       # Navigation actions
├── render.rs        # Rendering logic
└── tests.rs         # Integration tests
```

### Navigation Graph

```
                    ┌──────────────────────────────────────┐
                    │                                      │
                    v                                      │
    ┌──────────────────┐      ┌────────────────┐          │
    │       List       │─────>│     Search     │──────────┤
    └──────────────────┘      └────────────────┘          │
           │ │ │ │                                        │
           │ │ │ └────────────────────────────────────────┤
           │ │ │                                          │
           │ │ └──────>┌────────────────┐                 │
           │ │         │    SortMenu    │─────────────────┤
           │ │         └────────────────┘                 │
           │ │                                            │
           │ └────────>┌────────────────┐                 │
           │           │   FilterMenu   │─────────────────┤
           │           └────────────────┘                 │
           │                                              │
           v                                              │
    ┌──────────────────┐                                  │
    │      Detail      │──────────────────────────────────┤
    │  ┌────────────┐  │                                  │
    │  │ Overview   │◄─┼─┐                                │
    │  │Dependencies│  │ │                                │
    │  │GitContext  │  │ │ Left/Right                     │
    │  │Patterns    │  │ │                                │
    │  │DataFlow    │  │ │                                │
    │  │Responsib.  │──┼─┘                                │
    │  └────────────┘  │                                  │
    └──────────────────┘                                  │
           │                                              │
           v                                              │
    ┌──────────────────┐                                  │
    │       DSM        │──────────────────────────────────┤
    └──────────────────┘                                  │
                                                          │
    ┌──────────────────┐                                  │
    │       Help       │<─────────────────────────────────┘
    └──────────────────┘
           │
           │ (Esc returns to previous view)
           v
```

## Dependencies

- **Prerequisites**: None
- **Affected Components**:
  - `src/tui/results/app.rs` - Use navigation state machine
  - `src/tui/results/mod.rs` - Add new modules
- **External Dependencies**:
  - `mindset` patterns (no direct crate dependency needed - patterns only)

## Testing Strategy

### Unit Tests (Guards)

```rust
#[test]
fn test_all_guards_are_pure() {
    // Guards should be deterministic
    let state = NavigationState::new(false);

    // Multiple calls with same input → same output
    for _ in 0..100 {
        assert_eq!(
            can_enter_detail(&state, true, Some(0)),
            can_enter_detail(&state, true, Some(0))
        );
    }
}
```

### Integration Tests (Navigation Flows)

```rust
#[test]
fn test_typical_user_flow() {
    let mut state = NavigationState::new(true);

    // User views list
    assert_eq!(state.view_mode, ViewMode::List);

    // User selects item and views detail
    navigate_to_detail(&mut state, true, Some(0));
    assert_eq!(state.view_mode, ViewMode::Detail);

    // User navigates detail pages
    navigate_detail_page(&mut state, true);
    assert_eq!(state.detail_page, DetailPage::Dependencies);

    // User opens help
    navigate_to_help(&mut state);
    assert_eq!(state.view_mode, ViewMode::Help);

    // User closes help (returns to detail)
    navigate_back(&mut state);
    assert_eq!(state.view_mode, ViewMode::Detail);

    // User goes back to list
    navigate_back(&mut state);
    assert_eq!(state.view_mode, ViewMode::List);
}
```

## Documentation Requirements

- **Code Documentation**: Each state and transition documented
- **User Documentation**: Update TUI help with navigation keys
- **Architecture Updates**: Add navigation diagram to ARCHITECTURE.md

## Migration and Compatibility

### Breaking Changes

None - internal refactoring.

### Migration Steps

1. Add new navigation modules alongside existing code
2. Update `ResultsApp` to use `NavigationState`
3. Remove direct state mutations
4. Verify all existing TUI tests pass

## Implementation Notes

### Benefits of mindset Patterns

1. **Explicit Transitions** - Can't accidentally add invalid paths
2. **Testable Guards** - Pure functions, easy to test
3. **Documentation** - Transition table IS the documentation
4. **Dynamic UI** - Available actions derived from guards
5. **History Tracking** - Proper back navigation

### Future Enhancements

1. **Breadcrumb Navigation** - Show path: List > Item #3 > Dependencies
2. **Keyboard Shortcuts** - Jump directly to specific pages
3. **State Persistence** - Remember view mode between runs

## References

- **mindset documentation**: State machine patterns
- **Current TUI implementation**: `src/tui/results/app.rs`
- **Ratatui examples**: TUI navigation patterns
