//! Navigation state machine with explicit transitions and pure guards.
//!
//! This module implements mindset-inspired patterns for TUI navigation:
//! - Explicit state transitions via a transition table
//! - Pure guard functions for conditional navigation
//! - Navigation history for proper back navigation
//!
//! Navigation action functions are in the `nav_actions` module.
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

// Re-export navigation actions for backwards compatibility
pub use super::nav_actions::{
    available_actions, navigate_back, navigate_detail_page, navigate_to_detail,
    navigate_to_dsm, navigate_to_filter_menu, navigate_to_help, navigate_to_search,
    navigate_to_sort_menu,
};

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

    /// DSM horizontal scroll offset.
    pub dsm_scroll_x: usize,

    /// DSM vertical scroll offset.
    pub dsm_scroll_y: usize,
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
            dsm_scroll_x: 0,
            dsm_scroll_y: 0,
        }
    }

    /// Push current view mode to history before transitioning.
    pub fn push_and_set_view(&mut self, new_mode: ViewMode) {
        self.history.push(self.view_mode);
        self.view_mode = new_mode;
    }

    /// Go back to previous view mode.
    pub fn go_back(&mut self) -> Option<ViewMode> {
        self.history.pop().inspect(|&mode| {
            self.view_mode = mode;
        })
    }

    /// Clear navigation history.
    pub fn clear_history(&mut self) {
        self.history.clear();
    }

    /// Reset DSM scroll position.
    pub fn reset_dsm_scroll(&mut self) {
        self.dsm_scroll_x = 0;
        self.dsm_scroll_y = 0;
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
        // Same input -> same output (deterministic)
        let r1 = can_enter_detail(ViewMode::List, true, true);
        let r2 = can_enter_detail(ViewMode::List, true, true);
        assert_eq!(r1, r2);

        let r3 = can_enter_dsm(ViewMode::List, true);
        let r4 = can_enter_dsm(ViewMode::List, true);
        assert_eq!(r3, r4);
    }

    // ============================================================================
    // NavigationState Method Tests
    // ============================================================================

    #[test]
    fn test_push_and_set_view() {
        let mut state = NavigationState::new(true);
        assert_eq!(state.view_mode, ViewMode::List);
        assert!(state.history.is_empty());

        state.push_and_set_view(ViewMode::Detail);
        assert_eq!(state.view_mode, ViewMode::Detail);
        assert_eq!(state.history.len(), 1);
        assert_eq!(state.history[0], ViewMode::List);

        state.push_and_set_view(ViewMode::Help);
        assert_eq!(state.view_mode, ViewMode::Help);
        assert_eq!(state.history.len(), 2);
    }

    #[test]
    fn test_go_back_with_history() {
        let mut state = NavigationState::new(true);
        state.push_and_set_view(ViewMode::Detail);
        state.push_and_set_view(ViewMode::Help);

        let result = state.go_back();
        assert_eq!(result, Some(ViewMode::Detail));
        assert_eq!(state.view_mode, ViewMode::Detail);

        let result = state.go_back();
        assert_eq!(result, Some(ViewMode::List));
        assert_eq!(state.view_mode, ViewMode::List);

        let result = state.go_back();
        assert_eq!(result, None);
    }

    #[test]
    fn test_clear_history() {
        let mut state = NavigationState::new(true);
        state.push_and_set_view(ViewMode::Detail);
        state.push_and_set_view(ViewMode::Help);
        assert_eq!(state.history.len(), 2);

        state.clear_history();
        assert!(state.history.is_empty());
    }

    #[test]
    fn test_dsm_scroll_default() {
        let state = NavigationState::new(true);
        assert_eq!(state.dsm_scroll_x, 0);
        assert_eq!(state.dsm_scroll_y, 0);
    }

    #[test]
    fn test_reset_dsm_scroll() {
        let mut state = NavigationState::new(true);
        state.dsm_scroll_x = 10;
        state.dsm_scroll_y = 20;

        state.reset_dsm_scroll();
        assert_eq!(state.dsm_scroll_x, 0);
        assert_eq!(state.dsm_scroll_y, 0);
    }
}

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    /// Strategy for generating ViewMode values.
    fn view_mode_strategy() -> impl Strategy<Value = ViewMode> {
        prop_oneof![
            Just(ViewMode::List),
            Just(ViewMode::Detail),
            Just(ViewMode::Search),
            Just(ViewMode::SortMenu),
            Just(ViewMode::FilterMenu),
            Just(ViewMode::Help),
            Just(ViewMode::Dsm),
        ]
    }

    proptest! {
        /// Property: valid_destinations is consistent with is_valid_transition.
        ///
        /// If a mode is in valid_destinations, then is_valid_transition should return true.
        #[test]
        fn valid_destinations_consistent(from in view_mode_strategy()) {
            let destinations = valid_destinations(from);
            for to in destinations {
                prop_assert!(
                    is_valid_transition(from, to),
                    "valid_destinations({:?}) contains {:?} but is_valid_transition returns false",
                    from, to
                );
            }
        }

        /// Property: navigation history is LIFO.
        ///
        /// Push/pop sequence should be last-in-first-out.
        #[test]
        fn history_is_lifo(modes in proptest::collection::vec(view_mode_strategy(), 0..10)) {
            let mut state = NavigationState::new(false);

            // Push all modes
            for &mode in &modes {
                state.push_and_set_view(mode);
            }

            // Pop should return in reverse order (but we get the mode we navigated TO,
            // not FROM, because go_back returns what we set view_mode to)
            for &expected in modes.iter().rev() {
                let current = state.view_mode;
                prop_assert_eq!(current, expected);
                state.go_back();
            }
        }

        /// Property: clear_history empties history.
        #[test]
        fn clear_history_empties(
            push_count in 0usize..20
        ) {
            let mut state = NavigationState::new(true);

            for _ in 0..push_count {
                state.push_and_set_view(ViewMode::Detail);
            }

            state.clear_history();
            prop_assert!(state.history.is_empty());
        }

        /// Property: can_enter_help is false only when already in Help.
        #[test]
        fn help_blocked_only_from_help(mode in view_mode_strategy()) {
            let can_enter = can_enter_help(mode);
            prop_assert_eq!(can_enter, mode != ViewMode::Help);
        }

        /// Property: DSM scroll reset zeros both dimensions.
        #[test]
        fn dsm_scroll_reset_zeros(x in 0usize..1000, y in 0usize..1000) {
            let mut state = NavigationState::new(true);
            state.dsm_scroll_x = x;
            state.dsm_scroll_y = y;

            state.reset_dsm_scroll();

            prop_assert_eq!(state.dsm_scroll_x, 0);
            prop_assert_eq!(state.dsm_scroll_y, 0);
        }
    }
}
