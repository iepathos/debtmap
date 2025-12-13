//! Navigation action functions for the TUI.
//!
//! This module contains the navigation action functions that perform
//! state transitions. These are separated from `nav_state.rs` to keep
//! the state module focused on the NavigationState struct and pure guards.
//!
//! # Design
//!
//! Each navigation action:
//! 1. Validates the transition against the transition table
//! 2. Checks guards for preconditions
//! 3. Updates state on success
//! 4. Returns a NavigationResult indicating success/failure

use super::app::{DetailPage, ViewMode};
use super::nav_state::{
    can_enter_detail, can_enter_dsm, can_enter_filter_menu, can_enter_help, can_enter_search,
    can_enter_sort_menu, can_go_back, can_navigate_detail_pages, is_valid_transition,
    NavigationResult, NavigationState,
};

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

        // Navigate List -> Detail
        navigate_to_detail(&mut state, true, true);
        assert_eq!(state.view_mode, ViewMode::Detail);

        // Navigate Detail -> Help
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

    #[test]
    fn test_navigate_to_sort_menu_success() {
        let mut state = NavigationState::new(false);

        let result = navigate_to_sort_menu(&mut state);
        assert!(result.is_success());
        assert_eq!(state.view_mode, ViewMode::SortMenu);
        assert_eq!(state.history.len(), 1);
    }

    #[test]
    fn test_navigate_to_filter_menu_success() {
        let mut state = NavigationState::new(false);

        let result = navigate_to_filter_menu(&mut state);
        assert!(result.is_success());
        assert_eq!(state.view_mode, ViewMode::FilterMenu);
        assert_eq!(state.history.len(), 1);
    }

    #[test]
    fn test_navigate_to_help_success() {
        let mut state = NavigationState::new(false);

        let result = navigate_to_help(&mut state);
        assert!(result.is_success());
        assert_eq!(state.view_mode, ViewMode::Help);
    }

    #[test]
    fn test_navigate_to_help_blocked_from_help() {
        let mut state = NavigationState::new(false);
        state.view_mode = ViewMode::Help;

        let result = navigate_to_help(&mut state);
        assert!(matches!(result, NavigationResult::Blocked { .. }));
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
        /// Property: navigate_back from non-root without history goes to List.
        #[test]
        fn back_without_history_goes_to_list(mode in view_mode_strategy()) {
            if mode == ViewMode::List {
                return Ok(());
            }

            let mut state = NavigationState::new(false);
            state.view_mode = mode;
            // Intentionally don't push to history

            let result = navigate_back(&mut state);
            prop_assert!(result.is_success());
            prop_assert_eq!(state.view_mode, ViewMode::List);
        }

        /// Property: detail page navigation is cyclic.
        ///
        /// Navigating forward through all pages and back should preserve state.
        #[test]
        fn detail_page_navigation_cyclic(
            forward_count in 0usize..20
        ) {
            let mut state = NavigationState::new(false);
            state.view_mode = ViewMode::Detail;
            let initial_page = state.detail_page;

            // Navigate forward multiple times
            for _ in 0..forward_count {
                let _ = navigate_detail_page(&mut state, true);
            }

            // Navigate backward the same number of times
            for _ in 0..forward_count {
                let _ = navigate_detail_page(&mut state, false);
            }

            prop_assert_eq!(state.detail_page, initial_page);
        }
    }
}
