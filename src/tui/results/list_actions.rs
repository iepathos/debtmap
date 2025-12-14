//! Pure action determination for list view keyboard handling.
//!
//! This module separates the pure logic of "which action does this key trigger?"
//! from the effectful "execute this action" code. Following Stillwater philosophy:
//! - Pure core: `determine_list_action` maps key + context → action
//! - Imperative shell: `execute_list_action` performs the actual mutations
//!
//! This separation enables:
//! - Unit testing the action determination without mocking app state
//! - Property testing key-action mappings
//! - Clear documentation of what each key does

use crossterm::event::{KeyCode, KeyEvent};

/// Actions that can be triggered from list view.
///
/// This enum represents all possible user intents in the list view,
/// independent of how they're executed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListAction {
    /// Quit the application.
    Quit,

    /// Move selection up by one item.
    MoveUp,

    /// Move selection down by one item.
    MoveDown,

    /// Jump to first item.
    JumpToTop,

    /// Jump to last item.
    JumpToBottom,

    /// Move selection up by a page.
    PageUp,

    /// Move selection down by a page.
    PageDown,

    /// Toggle file grouping.
    ToggleGrouping,

    /// Enter detail view for selected item.
    EnterDetail,

    /// Enter search mode.
    EnterSearch,

    /// Open sort menu.
    OpenSortMenu,

    /// Open filter menu.
    OpenFilterMenu,

    /// Show help overlay.
    ShowHelp,

    /// Copy file path to clipboard.
    CopyPath,

    /// Open selected file in editor.
    OpenInEditor,
}

/// Context needed to determine if an action is valid.
///
/// This captures the minimal state needed to evaluate guards,
/// allowing the determination function to remain pure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ListActionContext {
    /// Whether there are any items in the list.
    pub has_items: bool,

    /// Whether an item is currently selected.
    pub has_selection: bool,
}

impl ListActionContext {
    /// Create context for an empty list.
    #[cfg(test)]
    pub fn empty() -> Self {
        Self {
            has_items: false,
            has_selection: false,
        }
    }

    /// Create context for a list with selection.
    #[cfg(test)]
    pub fn with_selection() -> Self {
        Self {
            has_items: true,
            has_selection: true,
        }
    }
}

/// Pure function: Determine which action a key triggers in list view.
///
/// This function is the pure core of key handling. It takes immutable
/// inputs (key event and context) and returns an optional action.
/// No side effects, no mutations, fully testable.
///
/// # Arguments
/// * `key` - The key event to process
/// * `ctx` - Context about current state (for guard evaluation)
///
/// # Returns
/// * `Some(action)` - The action to execute
/// * `None` - Key has no action or guard prevented it
pub fn determine_list_action(key: KeyEvent, ctx: ListActionContext) -> Option<ListAction> {
    match key.code {
        // Quit - always available
        KeyCode::Char('q') => Some(ListAction::Quit),

        // Navigation - always available
        KeyCode::Up | KeyCode::Char('k') => Some(ListAction::MoveUp),
        KeyCode::Down | KeyCode::Char('j') => Some(ListAction::MoveDown),
        KeyCode::Char('g') | KeyCode::Home => Some(ListAction::JumpToTop),
        KeyCode::End => Some(ListAction::JumpToBottom),
        KeyCode::PageUp => Some(ListAction::PageUp),
        KeyCode::PageDown => Some(ListAction::PageDown),

        // Grouping toggle
        KeyCode::Char('G') => Some(ListAction::ToggleGrouping),

        // Detail view - guarded: requires items and selection
        KeyCode::Enter => {
            if ctx.has_items && ctx.has_selection {
                Some(ListAction::EnterDetail)
            } else {
                None
            }
        }

        // Search - always available from list
        KeyCode::Char('/') => Some(ListAction::EnterSearch),

        // Sort menu - always available from list
        KeyCode::Char('s') => Some(ListAction::OpenSortMenu),

        // Filter menu - always available from list
        KeyCode::Char('f') => Some(ListAction::OpenFilterMenu),

        // Help - always available
        KeyCode::Char('?') => Some(ListAction::ShowHelp),

        // Clipboard - requires selection
        KeyCode::Char('c') => {
            if ctx.has_selection {
                Some(ListAction::CopyPath)
            } else {
                None
            }
        }

        // Editor - requires selection
        KeyCode::Char('e') | KeyCode::Char('o') => {
            if ctx.has_selection {
                Some(ListAction::OpenInEditor)
            } else {
                None
            }
        }

        // No action for this key
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyModifiers;

    /// Create a KeyEvent from a KeyCode.
    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    // ============================================================================
    // Quit Action Tests
    // ============================================================================

    #[test]
    fn quit_with_q() {
        let ctx = ListActionContext::empty();
        assert_eq!(
            determine_list_action(key(KeyCode::Char('q')), ctx),
            Some(ListAction::Quit)
        );
    }

    #[test]
    fn quit_works_with_any_context() {
        // Quit should work regardless of state
        for ctx in [
            ListActionContext::empty(),
            ListActionContext::with_selection(),
        ] {
            assert_eq!(
                determine_list_action(key(KeyCode::Char('q')), ctx),
                Some(ListAction::Quit)
            );
        }
    }

    // ============================================================================
    // Navigation Action Tests
    // ============================================================================

    #[test]
    fn move_up_with_up_arrow() {
        let ctx = ListActionContext::with_selection();
        assert_eq!(
            determine_list_action(key(KeyCode::Up), ctx),
            Some(ListAction::MoveUp)
        );
    }

    #[test]
    fn move_up_with_k() {
        let ctx = ListActionContext::with_selection();
        assert_eq!(
            determine_list_action(key(KeyCode::Char('k')), ctx),
            Some(ListAction::MoveUp)
        );
    }

    #[test]
    fn move_down_with_down_arrow() {
        let ctx = ListActionContext::with_selection();
        assert_eq!(
            determine_list_action(key(KeyCode::Down), ctx),
            Some(ListAction::MoveDown)
        );
    }

    #[test]
    fn move_down_with_j() {
        let ctx = ListActionContext::with_selection();
        assert_eq!(
            determine_list_action(key(KeyCode::Char('j')), ctx),
            Some(ListAction::MoveDown)
        );
    }

    #[test]
    fn jump_to_top_with_g() {
        let ctx = ListActionContext::with_selection();
        assert_eq!(
            determine_list_action(key(KeyCode::Char('g')), ctx),
            Some(ListAction::JumpToTop)
        );
    }

    #[test]
    fn jump_to_top_with_home() {
        let ctx = ListActionContext::with_selection();
        assert_eq!(
            determine_list_action(key(KeyCode::Home), ctx),
            Some(ListAction::JumpToTop)
        );
    }

    #[test]
    fn jump_to_bottom_with_end() {
        let ctx = ListActionContext::with_selection();
        assert_eq!(
            determine_list_action(key(KeyCode::End), ctx),
            Some(ListAction::JumpToBottom)
        );
    }

    #[test]
    fn page_up_key() {
        let ctx = ListActionContext::with_selection();
        assert_eq!(
            determine_list_action(key(KeyCode::PageUp), ctx),
            Some(ListAction::PageUp)
        );
    }

    #[test]
    fn page_down_key() {
        let ctx = ListActionContext::with_selection();
        assert_eq!(
            determine_list_action(key(KeyCode::PageDown), ctx),
            Some(ListAction::PageDown)
        );
    }

    // ============================================================================
    // Grouping Action Tests
    // ============================================================================

    #[test]
    fn toggle_grouping_with_shift_g() {
        let ctx = ListActionContext::with_selection();
        assert_eq!(
            determine_list_action(key(KeyCode::Char('G')), ctx),
            Some(ListAction::ToggleGrouping)
        );
    }

    // ============================================================================
    // View Transition Tests
    // ============================================================================

    #[test]
    fn enter_detail_requires_selection() {
        // Without selection - no action
        let empty = ListActionContext::empty();
        assert_eq!(determine_list_action(key(KeyCode::Enter), empty), None);

        // With items but no selection - no action
        let items_only = ListActionContext {
            has_items: true,
            has_selection: false,
        };
        assert_eq!(determine_list_action(key(KeyCode::Enter), items_only), None);

        // With selection - action
        let with_sel = ListActionContext::with_selection();
        assert_eq!(
            determine_list_action(key(KeyCode::Enter), with_sel),
            Some(ListAction::EnterDetail)
        );
    }

    #[test]
    fn enter_search_with_slash() {
        let ctx = ListActionContext::with_selection();
        assert_eq!(
            determine_list_action(key(KeyCode::Char('/')), ctx),
            Some(ListAction::EnterSearch)
        );
    }

    #[test]
    fn open_sort_menu_with_s() {
        let ctx = ListActionContext::with_selection();
        assert_eq!(
            determine_list_action(key(KeyCode::Char('s')), ctx),
            Some(ListAction::OpenSortMenu)
        );
    }

    #[test]
    fn open_filter_menu_with_f() {
        let ctx = ListActionContext::with_selection();
        assert_eq!(
            determine_list_action(key(KeyCode::Char('f')), ctx),
            Some(ListAction::OpenFilterMenu)
        );
    }

    #[test]
    fn show_help_with_question_mark() {
        let ctx = ListActionContext::with_selection();
        assert_eq!(
            determine_list_action(key(KeyCode::Char('?')), ctx),
            Some(ListAction::ShowHelp)
        );
    }

    // ============================================================================
    // Action Requiring Selection Tests
    // ============================================================================

    #[test]
    fn copy_path_requires_selection() {
        // Without selection
        let empty = ListActionContext::empty();
        assert_eq!(determine_list_action(key(KeyCode::Char('c')), empty), None);

        // With selection
        let with_sel = ListActionContext::with_selection();
        assert_eq!(
            determine_list_action(key(KeyCode::Char('c')), with_sel),
            Some(ListAction::CopyPath)
        );
    }

    #[test]
    fn open_in_editor_with_e_requires_selection() {
        // Without selection
        let empty = ListActionContext::empty();
        assert_eq!(determine_list_action(key(KeyCode::Char('e')), empty), None);

        // With selection
        let with_sel = ListActionContext::with_selection();
        assert_eq!(
            determine_list_action(key(KeyCode::Char('e')), with_sel),
            Some(ListAction::OpenInEditor)
        );
    }

    #[test]
    fn open_in_editor_with_o_requires_selection() {
        // Without selection
        let empty = ListActionContext::empty();
        assert_eq!(determine_list_action(key(KeyCode::Char('o')), empty), None);

        // With selection
        let with_sel = ListActionContext::with_selection();
        assert_eq!(
            determine_list_action(key(KeyCode::Char('o')), with_sel),
            Some(ListAction::OpenInEditor)
        );
    }

    // ============================================================================
    // Unknown Key Tests
    // ============================================================================

    #[test]
    fn unknown_key_returns_none() {
        let ctx = ListActionContext::with_selection();
        assert_eq!(determine_list_action(key(KeyCode::Char('x')), ctx), None);
        assert_eq!(determine_list_action(key(KeyCode::Char('z')), ctx), None);
        assert_eq!(determine_list_action(key(KeyCode::F(1)), ctx), None);
    }

    // ============================================================================
    // Pure Function Property Tests
    // ============================================================================

    #[test]
    fn determine_action_is_deterministic() {
        let ctx = ListActionContext::with_selection();
        let k = key(KeyCode::Enter);

        // Same input always produces same output
        let r1 = determine_list_action(k, ctx);
        let r2 = determine_list_action(k, ctx);
        assert_eq!(r1, r2);
    }

    #[test]
    fn context_affects_guarded_actions() {
        let k = key(KeyCode::Enter);

        // Different contexts produce different results for guarded actions
        let empty = ListActionContext::empty();
        let with_sel = ListActionContext::with_selection();

        assert_ne!(
            determine_list_action(k, empty),
            determine_list_action(k, with_sel)
        );
    }

    #[test]
    fn navigation_keys_work_on_empty_list() {
        // Navigation keys should still return actions on empty list
        // (the execution layer handles the empty case)
        let empty = ListActionContext::empty();

        assert_eq!(
            determine_list_action(key(KeyCode::Up), empty),
            Some(ListAction::MoveUp)
        );
        assert_eq!(
            determine_list_action(key(KeyCode::Down), empty),
            Some(ListAction::MoveDown)
        );
    }
}

#[cfg(test)]
mod property_tests {
    use super::*;
    use crossterm::event::KeyModifiers;
    use proptest::prelude::*;

    fn key_code_strategy() -> impl Strategy<Value = KeyCode> {
        prop_oneof![
            Just(KeyCode::Char('q')),
            Just(KeyCode::Char('k')),
            Just(KeyCode::Char('j')),
            Just(KeyCode::Char('g')),
            Just(KeyCode::Char('G')),
            Just(KeyCode::Char('/')),
            Just(KeyCode::Char('s')),
            Just(KeyCode::Char('f')),
            Just(KeyCode::Char('?')),
            Just(KeyCode::Char('c')),
            Just(KeyCode::Char('e')),
            Just(KeyCode::Char('o')),
            Just(KeyCode::Up),
            Just(KeyCode::Down),
            Just(KeyCode::Home),
            Just(KeyCode::End),
            Just(KeyCode::PageUp),
            Just(KeyCode::PageDown),
            Just(KeyCode::Enter),
            Just(KeyCode::Esc),
            Just(KeyCode::Tab),
        ]
    }

    fn context_strategy() -> impl Strategy<Value = ListActionContext> {
        (any::<bool>(), any::<bool>()).prop_map(|(has_items, has_selection)| ListActionContext {
            has_items,
            has_selection: has_items && has_selection, // Can't have selection without items
        })
    }

    proptest! {
        /// Property: Quit action is always available regardless of context.
        #[test]
        fn quit_always_available(ctx in context_strategy()) {
            let key = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);
            prop_assert_eq!(determine_list_action(key, ctx), Some(ListAction::Quit));
        }

        /// Property: Navigation actions are always available.
        #[test]
        fn navigation_always_available(ctx in context_strategy()) {
            let up_key = KeyEvent::new(KeyCode::Up, KeyModifiers::NONE);
            let down_key = KeyEvent::new(KeyCode::Down, KeyModifiers::NONE);

            prop_assert_eq!(determine_list_action(up_key, ctx), Some(ListAction::MoveUp));
            prop_assert_eq!(determine_list_action(down_key, ctx), Some(ListAction::MoveDown));
        }

        /// Property: Enter requires both items and selection.
        #[test]
        fn enter_requires_items_and_selection(ctx in context_strategy()) {
            let key = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
            let result = determine_list_action(key, ctx);

            if ctx.has_items && ctx.has_selection {
                prop_assert_eq!(result, Some(ListAction::EnterDetail));
            } else {
                prop_assert_eq!(result, None);
            }
        }

        /// Property: Pure function - same input always produces same output.
        #[test]
        fn deterministic(
            code in key_code_strategy(),
            ctx in context_strategy()
        ) {
            let key = KeyEvent::new(code, KeyModifiers::NONE);
            let r1 = determine_list_action(key, ctx);
            let r2 = determine_list_action(key, ctx);
            prop_assert_eq!(r1, r2);
        }
    }
}
