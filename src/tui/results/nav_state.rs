//! Navigation state machine with explicit transitions and pure guards.
//!
//! This module implements mindset-inspired patterns for TUI navigation:
//! - Explicit state transitions via a transition table
//! - Pure guard functions for conditional navigation
//! - Navigation history for proper back navigation
//!
//! # Navigation Graph
//!
//! ```text
//!     ┌──────────────────────────────────────┐
//!     │                                      │
//!     v                                      │
//!     List ──────► Search ───────────────────┤
//!      │ │ │                                 │
//!      │ │ └──────► SortMenu ────────────────┤
//!      │ │                                   │
//!      │ └────────► FilterMenu ──────────────┤
//!      │                                     │
//!      v                                     │
//!     Detail ────────────────────────────────┤
//!      │                                     │
//!      v                                     │
//!     Dsm ───────────────────────────────────┤
//!                                            │
//!     Help ◄─────────────────────────────────┘
//!      │
//!      │ (Esc returns to previous view)
//!      v
//! ```

use super::app::{DetailPage, ViewMode};

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
    (ViewMode::Search, ViewMode::Detail), // Search result selected
    // From SortMenu
    (ViewMode::SortMenu, ViewMode::List),
    // From FilterMenu
    (ViewMode::FilterMenu, ViewMode::List),
    // From Help (returns to previous)
    (ViewMode::Help, ViewMode::List),
    (ViewMode::Help, ViewMode::Detail),
    (ViewMode::Help, ViewMode::Search),
    (ViewMode::Help, ViewMode::Dsm),
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

/// Complete navigation state including history.
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

impl Default for NavigationState {
    fn default() -> Self {
        Self::new(false)
    }
}

impl NavigationState {
    /// Create new navigation state.
    pub fn new(dsm_enabled: bool) -> Self {
        Self {
            view_mode: ViewMode::List,
            detail_page: DetailPage::Overview,
            history: vec![],
            dsm_enabled,
        }
    }
}

// ============================================================================
// Pure Guard Functions
// ============================================================================

/// Guard: Can enter Detail view?
///
/// Pure function - requires items and a selection.
pub fn can_enter_detail(current_mode: ViewMode, has_items: bool, has_selection: bool) -> bool {
    matches!(current_mode, ViewMode::List | ViewMode::Search) && has_items && has_selection
}

/// Guard: Can enter DSM view?
///
/// Requires DSM feature enabled and being in a valid source mode.
pub fn can_enter_dsm(current_mode: ViewMode, dsm_enabled: bool) -> bool {
    matches!(current_mode, ViewMode::List) && dsm_enabled
}

/// Guard: Can enter Search?
///
/// Only from List view.
pub fn can_enter_search(current_mode: ViewMode) -> bool {
    matches!(current_mode, ViewMode::List)
}

/// Guard: Can enter SortMenu?
pub fn can_enter_sort_menu(current_mode: ViewMode) -> bool {
    matches!(current_mode, ViewMode::List)
}

/// Guard: Can enter FilterMenu?
pub fn can_enter_filter_menu(current_mode: ViewMode) -> bool {
    matches!(current_mode, ViewMode::List)
}

/// Guard: Can enter Help?
///
/// Help is accessible from most views (but not from Help itself).
pub fn can_enter_help(current_mode: ViewMode) -> bool {
    !matches!(current_mode, ViewMode::Help)
}

/// Guard: Can go back?
///
/// True if there's history or not in List view.
pub fn can_go_back(current_mode: ViewMode, history_len: usize) -> bool {
    history_len > 0 || !matches!(current_mode, ViewMode::List)
}

/// Guard: Can navigate detail pages?
pub fn can_navigate_detail_pages(current_mode: ViewMode) -> bool {
    matches!(current_mode, ViewMode::Detail)
}

// ============================================================================
// Navigation Result
// ============================================================================

/// Result of attempting a navigation action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NavigationResult {
    /// Navigation succeeded.
    Success,

    /// Navigation failed - guard rejected.
    Blocked { reason: &'static str },

    /// Navigation invalid - not in transition table.
    Invalid { from: ViewMode, to: ViewMode },
}

impl NavigationResult {
    /// Returns true if navigation succeeded.
    pub fn is_success(&self) -> bool {
        matches!(self, NavigationResult::Success)
    }
}

// ============================================================================
// Navigation Actions
// ============================================================================

/// Navigate to Detail view.
pub fn navigate_to_detail(
    state: &mut NavigationState,
    has_items: bool,
    has_selection: bool,
) -> NavigationResult {
    if !is_valid_transition(state.view_mode, ViewMode::Detail) {
        return NavigationResult::Invalid {
            from: state.view_mode,
            to: ViewMode::Detail,
        };
    }

    if !can_enter_detail(state.view_mode, has_items, has_selection) {
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

    if !can_enter_search(state.view_mode) {
        return NavigationResult::Blocked {
            reason: "Search only available from List view",
        };
    }

    state.history.push(state.view_mode);
    state.view_mode = ViewMode::Search;

    NavigationResult::Success
}

/// Navigate to SortMenu view.
pub fn navigate_to_sort_menu(state: &mut NavigationState) -> NavigationResult {
    if !is_valid_transition(state.view_mode, ViewMode::SortMenu) {
        return NavigationResult::Invalid {
            from: state.view_mode,
            to: ViewMode::SortMenu,
        };
    }

    if !can_enter_sort_menu(state.view_mode) {
        return NavigationResult::Blocked {
            reason: "Sort menu only available from List view",
        };
    }

    state.history.push(state.view_mode);
    state.view_mode = ViewMode::SortMenu;

    NavigationResult::Success
}

/// Navigate to FilterMenu view.
pub fn navigate_to_filter_menu(state: &mut NavigationState) -> NavigationResult {
    if !is_valid_transition(state.view_mode, ViewMode::FilterMenu) {
        return NavigationResult::Invalid {
            from: state.view_mode,
            to: ViewMode::FilterMenu,
        };
    }

    if !can_enter_filter_menu(state.view_mode) {
        return NavigationResult::Blocked {
            reason: "Filter menu only available from List view",
        };
    }

    state.history.push(state.view_mode);
    state.view_mode = ViewMode::FilterMenu;

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

    if !can_enter_dsm(state.view_mode, state.dsm_enabled) {
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
    if !can_enter_help(state.view_mode) {
        return NavigationResult::Blocked {
            reason: "Already in Help view",
        };
    }

    // Help can be accessed from most views, so we don't check transition table strictly
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
    if !can_navigate_detail_pages(state.view_mode) {
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

/// Get available actions for current state (for status bar display).
pub fn available_actions(
    state: &NavigationState,
    has_items: bool,
    has_selection: bool,
) -> Vec<(&'static str, &'static str)> {
    let mut actions = vec![];

    if can_enter_detail(state.view_mode, has_items, has_selection) {
        actions.push(("Enter", "View details"));
    }

    if can_enter_search(state.view_mode) {
        actions.push(("/", "Search"));
    }

    if can_enter_sort_menu(state.view_mode) {
        actions.push(("s", "Sort"));
    }

    if can_enter_filter_menu(state.view_mode) {
        actions.push(("f", "Filter"));
    }

    if can_enter_dsm(state.view_mode, state.dsm_enabled) {
        actions.push(("m", "DSM view"));
    }

    if can_enter_help(state.view_mode) {
        actions.push(("?", "Help"));
    }

    if can_go_back(state.view_mode, state.history.len()) {
        actions.push(("Esc", "Back"));
    }

    actions.push(("q", "Quit"));

    actions
}

#[cfg(test)]
mod tests {
    use super::*;

    // ============================================================================
    // Transition Table Tests
    // ============================================================================

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
        assert!(destinations.contains(&ViewMode::SortMenu));
        assert!(destinations.contains(&ViewMode::FilterMenu));
        assert!(destinations.contains(&ViewMode::Dsm));
    }

    #[test]
    fn test_valid_destinations_from_detail() {
        let destinations = valid_destinations(ViewMode::Detail);
        assert!(destinations.contains(&ViewMode::List));
        assert!(destinations.contains(&ViewMode::Help));
        // Detail cannot go to Search, SortMenu, FilterMenu, or Dsm
        assert!(!destinations.contains(&ViewMode::Search));
        assert!(!destinations.contains(&ViewMode::SortMenu));
        assert!(!destinations.contains(&ViewMode::FilterMenu));
        assert!(!destinations.contains(&ViewMode::Dsm));
    }

    // ============================================================================
    // Guard Function Tests
    // ============================================================================

    #[test]
    fn test_can_enter_detail_requires_selection() {
        // No items - can't enter
        assert!(!can_enter_detail(ViewMode::List, false, false));

        // Items but no selection - can't enter
        assert!(!can_enter_detail(ViewMode::List, true, false));

        // Items and selection - can enter
        assert!(can_enter_detail(ViewMode::List, true, true));

        // From Search with selection - can enter
        assert!(can_enter_detail(ViewMode::Search, true, true));

        // From Detail - cannot re-enter
        assert!(!can_enter_detail(ViewMode::Detail, true, true));
    }

    #[test]
    fn test_can_enter_dsm_requires_feature() {
        // DSM disabled
        assert!(!can_enter_dsm(ViewMode::List, false));

        // DSM enabled
        assert!(can_enter_dsm(ViewMode::List, true));

        // DSM enabled but wrong mode
        assert!(!can_enter_dsm(ViewMode::Detail, true));
    }

    #[test]
    fn test_can_enter_search_only_from_list() {
        assert!(can_enter_search(ViewMode::List));
        assert!(!can_enter_search(ViewMode::Detail));
        assert!(!can_enter_search(ViewMode::Search));
    }

    #[test]
    fn test_can_enter_help_not_from_help() {
        assert!(can_enter_help(ViewMode::List));
        assert!(can_enter_help(ViewMode::Detail));
        assert!(can_enter_help(ViewMode::Search));
        assert!(can_enter_help(ViewMode::Dsm));
        assert!(!can_enter_help(ViewMode::Help));
    }

    #[test]
    fn test_can_go_back_with_history() {
        // No history, at root
        assert!(!can_go_back(ViewMode::List, 0));

        // No history, not at root
        assert!(can_go_back(ViewMode::Detail, 0));

        // With history
        assert!(can_go_back(ViewMode::List, 1));
        assert!(can_go_back(ViewMode::Detail, 1));
    }

    #[test]
    fn test_guards_are_pure() {
        // Same input → same output (deterministic)
        let r1 = can_enter_detail(ViewMode::List, true, true);
        let r2 = can_enter_detail(ViewMode::List, true, true);
        assert_eq!(r1, r2);

        let r3 = can_enter_dsm(ViewMode::List, true);
        let r4 = can_enter_dsm(ViewMode::List, true);
        assert_eq!(r3, r4);
    }

    // ============================================================================
    // Navigation Action Tests
    // ============================================================================

    #[test]
    fn test_navigate_to_detail_success() {
        let mut state = NavigationState::new(false);

        let result = navigate_to_detail(&mut state, true, true);
        assert!(result.is_success());
        assert_eq!(state.view_mode, ViewMode::Detail);
        assert_eq!(state.history.len(), 1);
        assert_eq!(state.history[0], ViewMode::List);
    }

    #[test]
    fn test_navigate_to_detail_blocked_no_selection() {
        let mut state = NavigationState::new(false);

        let result = navigate_to_detail(&mut state, true, false);
        assert!(matches!(result, NavigationResult::Blocked { .. }));
        assert_eq!(state.view_mode, ViewMode::List);
        assert!(state.history.is_empty());
    }

    #[test]
    fn test_navigate_to_detail_invalid_from_detail() {
        let mut state = NavigationState::new(false);
        state.view_mode = ViewMode::Detail;

        let result = navigate_to_detail(&mut state, true, true);
        assert!(matches!(result, NavigationResult::Invalid { .. }));
    }

    #[test]
    fn test_navigate_to_search_success() {
        let mut state = NavigationState::new(false);

        let result = navigate_to_search(&mut state);
        assert!(result.is_success());
        assert_eq!(state.view_mode, ViewMode::Search);
    }

    #[test]
    fn test_navigate_to_search_invalid_from_detail() {
        let mut state = NavigationState::new(false);
        state.view_mode = ViewMode::Detail;

        let result = navigate_to_search(&mut state);
        assert!(matches!(result, NavigationResult::Invalid { .. }));
    }

    #[test]
    fn test_navigate_to_dsm_blocked_when_disabled() {
        let mut state = NavigationState::new(false);

        let result = navigate_to_dsm(&mut state);
        assert!(matches!(result, NavigationResult::Blocked { .. }));
    }

    #[test]
    fn test_navigate_to_dsm_success_when_enabled() {
        let mut state = NavigationState::new(true);

        let result = navigate_to_dsm(&mut state);
        assert!(result.is_success());
        assert_eq!(state.view_mode, ViewMode::Dsm);
    }

    #[test]
    fn test_navigate_back_uses_history() {
        let mut state = NavigationState::new(false);

        // Navigate List → Detail
        navigate_to_detail(&mut state, true, true);
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
    fn test_navigate_back_blocked_at_root() {
        let mut state = NavigationState::new(false);

        let result = navigate_back(&mut state);
        assert!(matches!(result, NavigationResult::Blocked { .. }));
    }

    #[test]
    fn test_navigate_back_without_history() {
        let mut state = NavigationState::new(false);
        state.view_mode = ViewMode::Detail;

        // No history but not at root - should go to List
        let result = navigate_back(&mut state);
        assert!(result.is_success());
        assert_eq!(state.view_mode, ViewMode::List);
    }

    #[test]
    fn test_navigate_detail_page() {
        let mut state = NavigationState::new(false);
        state.view_mode = ViewMode::Detail;
        state.detail_page = DetailPage::Overview;

        // Forward
        let result = navigate_detail_page(&mut state, true);
        assert!(result.is_success());
        assert_eq!(state.detail_page, DetailPage::Dependencies);

        // Backward
        let result = navigate_detail_page(&mut state, false);
        assert!(result.is_success());
        assert_eq!(state.detail_page, DetailPage::Overview);
    }

    #[test]
    fn test_navigate_detail_page_blocked_outside_detail() {
        let mut state = NavigationState::new(false);

        let result = navigate_detail_page(&mut state, true);
        assert!(matches!(result, NavigationResult::Blocked { .. }));
    }

    // ============================================================================
    // Integration Tests
    // ============================================================================

    #[test]
    fn test_typical_user_flow() {
        let mut state = NavigationState::new(true);

        // User views list
        assert_eq!(state.view_mode, ViewMode::List);

        // User selects item and views detail
        let result = navigate_to_detail(&mut state, true, true);
        assert!(result.is_success());
        assert_eq!(state.view_mode, ViewMode::Detail);

        // User navigates detail pages
        let result = navigate_detail_page(&mut state, true);
        assert!(result.is_success());
        assert_eq!(state.detail_page, DetailPage::Dependencies);

        // User opens help
        let result = navigate_to_help(&mut state);
        assert!(result.is_success());
        assert_eq!(state.view_mode, ViewMode::Help);

        // User closes help (returns to detail)
        let result = navigate_back(&mut state);
        assert!(result.is_success());
        assert_eq!(state.view_mode, ViewMode::Detail);

        // User goes back to list
        let result = navigate_back(&mut state);
        assert!(result.is_success());
        assert_eq!(state.view_mode, ViewMode::List);
    }

    #[test]
    fn test_search_to_detail_flow() {
        let mut state = NavigationState::new(false);

        // Start search
        let result = navigate_to_search(&mut state);
        assert!(result.is_success());
        assert_eq!(state.view_mode, ViewMode::Search);

        // Select search result (goes to detail)
        let result = navigate_to_detail(&mut state, true, true);
        assert!(result.is_success());
        assert_eq!(state.view_mode, ViewMode::Detail);

        // Go back (should return to Search due to history)
        let result = navigate_back(&mut state);
        assert!(result.is_success());
        assert_eq!(state.view_mode, ViewMode::Search);
    }

    #[test]
    fn test_available_actions() {
        let state = NavigationState::new(true);

        let actions = available_actions(&state, true, true);

        // From List with items and selection, should have most actions
        let action_keys: Vec<_> = actions.iter().map(|(k, _)| *k).collect();
        assert!(action_keys.contains(&"Enter"));
        assert!(action_keys.contains(&"/"));
        assert!(action_keys.contains(&"s"));
        assert!(action_keys.contains(&"f"));
        assert!(action_keys.contains(&"m")); // DSM enabled
        assert!(action_keys.contains(&"?"));
        assert!(action_keys.contains(&"q"));
    }

    #[test]
    fn test_available_actions_detail_view() {
        let mut state = NavigationState::new(false);
        state.view_mode = ViewMode::Detail;
        state.history.push(ViewMode::List);

        let actions = available_actions(&state, true, true);

        // From Detail, should have fewer actions
        let action_keys: Vec<_> = actions.iter().map(|(k, _)| *k).collect();
        assert!(!action_keys.contains(&"/")); // No search from detail
        assert!(!action_keys.contains(&"s")); // No sort from detail
        assert!(action_keys.contains(&"Esc")); // Can go back
        assert!(action_keys.contains(&"?")); // Help available
    }
}
