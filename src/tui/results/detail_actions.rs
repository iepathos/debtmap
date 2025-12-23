//! Pure action determination for detail view keyboard handling.
//!
//! This module separates the pure logic of "which action does this key trigger?"
//! from the effectful "execute this action" code. Following Stillwater philosophy:
//! - Pure core: `classify_detail_key` maps key + context → action
//! - Imperative shell: `execute_detail_action` in navigation.rs performs mutations
//!
//! This separation enables:
//! - Unit testing the action determination without mocking app state
//! - Property testing key-action mappings
//! - Clear documentation of what each key does
//! - Reduced cyclomatic complexity in the event handler

use super::detail_page::DetailPage;
use crossterm::event::{KeyCode, KeyEvent};

/// Actions that can be triggered from detail view.
///
/// This enum represents all possible user intents in the detail view,
/// independent of how they're executed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetailAction {
    /// Navigate back to the previous view.
    NavigateBack,

    /// Navigate to the next available page.
    NextPage,

    /// Navigate to the previous available page.
    PrevPage,

    /// Jump to a specific page (1-indexed digit maps to page).
    JumpToPage(DetailPage),

    /// Move selection up or down in the list.
    /// Positive values move down, negative values move up.
    MoveSelection(i32),

    /// Cycle to next item at same location (spec 267).
    /// Only active when multiple items exist at the current location.
    NextLocationItem,

    /// Cycle to previous item at same location (spec 267).
    /// Only active when multiple items exist at the current location.
    PrevLocationItem,

    /// Copy all context ranges (only valid on Context page).
    CopyContext,

    /// Copy primary range only (only valid on Context page).
    CopyPrimary,

    /// Copy current page content (valid on non-Context pages).
    CopyPage,

    /// Open the current item in an external editor.
    OpenInEditor,

    /// Show help overlay.
    ShowHelp,
}

/// Context needed to determine detail view actions.
///
/// This captures the minimal state needed to evaluate context-sensitive
/// key bindings, allowing the classification function to remain pure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DetailActionContext {
    /// The currently displayed detail page.
    pub current_page: DetailPage,
}

impl DetailActionContext {
    /// Create context for a specific page.
    #[must_use]
    pub fn new(current_page: DetailPage) -> Self {
        Self { current_page }
    }
}

/// Pure function: Map a digit character to a DetailPage.
///
/// Returns the page corresponding to the digit (1-indexed).
/// Invalid digits return None.
#[must_use]
pub fn page_from_digit(c: char) -> Option<DetailPage> {
    match c {
        '1' => Some(DetailPage::Overview),
        '2' => Some(DetailPage::ScoreBreakdown),
        '3' => Some(DetailPage::Context),
        '4' => Some(DetailPage::Dependencies),
        '5' => Some(DetailPage::GitContext),
        '6' => Some(DetailPage::Patterns),
        '7' => Some(DetailPage::DataFlow),
        '8' => Some(DetailPage::Responsibilities),
        _ => None,
    }
}

/// Pure function: Determine which action a key triggers in detail view.
///
/// This function is the pure core of key handling. It takes immutable
/// inputs (key event and context) and returns an optional action.
/// No side effects, no mutations, fully testable.
///
/// # Arguments
/// * `key` - The key event to process
/// * `ctx` - Context about current state (for context-sensitive bindings)
///
/// # Returns
/// * `Some(action)` - The action to execute
/// * `None` - Key has no action in detail view
pub fn classify_detail_key(key: KeyEvent, ctx: DetailActionContext) -> Option<DetailAction> {
    match key.code {
        // Back navigation - escape or 'q' returns to previous view
        KeyCode::Esc | KeyCode::Char('q') => Some(DetailAction::NavigateBack),

        // Page navigation - Tab/arrows cycle through available pages
        KeyCode::Tab | KeyCode::Right | KeyCode::Char('l') => Some(DetailAction::NextPage),
        KeyCode::BackTab | KeyCode::Left | KeyCode::Char('h') => Some(DetailAction::PrevPage),

        // Direct page jump - number keys (1-8) jump to specific pages
        KeyCode::Char(c @ '1'..='8') => page_from_digit(c).map(DetailAction::JumpToPage),

        // Item navigation - up/down moves through the list
        KeyCode::Down | KeyCode::Char('j') => Some(DetailAction::MoveSelection(1)),
        KeyCode::Up | KeyCode::Char('k') => Some(DetailAction::MoveSelection(-1)),

        // Location item cycling - [ and ] cycle through items at same location (spec 267)
        KeyCode::Char(']') => Some(DetailAction::NextLocationItem),
        KeyCode::Char('[') => Some(DetailAction::PrevLocationItem),

        // Copy actions - context-sensitive based on current page
        KeyCode::Char('c') => {
            if ctx.current_page == DetailPage::Context {
                Some(DetailAction::CopyContext)
            } else {
                Some(DetailAction::CopyPage)
            }
        }

        // Primary range copy - only meaningful on Context page
        KeyCode::Char('p') => {
            if ctx.current_page == DetailPage::Context {
                Some(DetailAction::CopyPrimary)
            } else {
                None
            }
        }

        // Open in editor
        KeyCode::Char('e') | KeyCode::Char('o') => Some(DetailAction::OpenInEditor),

        // Help overlay
        KeyCode::Char('?') => Some(DetailAction::ShowHelp),

        // Unknown key - no action
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
    // Navigation Action Tests
    // ============================================================================

    #[test]
    fn escape_navigates_back() {
        let ctx = DetailActionContext::new(DetailPage::Overview);
        assert_eq!(
            classify_detail_key(key(KeyCode::Esc), ctx),
            Some(DetailAction::NavigateBack)
        );
    }

    #[test]
    fn q_navigates_back() {
        let ctx = DetailActionContext::new(DetailPage::Overview);
        assert_eq!(
            classify_detail_key(key(KeyCode::Char('q')), ctx),
            Some(DetailAction::NavigateBack)
        );
    }

    #[test]
    fn tab_goes_to_next_page() {
        let ctx = DetailActionContext::new(DetailPage::Overview);
        assert_eq!(
            classify_detail_key(key(KeyCode::Tab), ctx),
            Some(DetailAction::NextPage)
        );
    }

    #[test]
    fn right_arrow_goes_to_next_page() {
        let ctx = DetailActionContext::new(DetailPage::Overview);
        assert_eq!(
            classify_detail_key(key(KeyCode::Right), ctx),
            Some(DetailAction::NextPage)
        );
    }

    #[test]
    fn l_goes_to_next_page() {
        let ctx = DetailActionContext::new(DetailPage::Overview);
        assert_eq!(
            classify_detail_key(key(KeyCode::Char('l')), ctx),
            Some(DetailAction::NextPage)
        );
    }

    #[test]
    fn backtab_goes_to_prev_page() {
        let ctx = DetailActionContext::new(DetailPage::Context);
        assert_eq!(
            classify_detail_key(key(KeyCode::BackTab), ctx),
            Some(DetailAction::PrevPage)
        );
    }

    #[test]
    fn left_arrow_goes_to_prev_page() {
        let ctx = DetailActionContext::new(DetailPage::Context);
        assert_eq!(
            classify_detail_key(key(KeyCode::Left), ctx),
            Some(DetailAction::PrevPage)
        );
    }

    #[test]
    fn h_goes_to_prev_page() {
        let ctx = DetailActionContext::new(DetailPage::Context);
        assert_eq!(
            classify_detail_key(key(KeyCode::Char('h')), ctx),
            Some(DetailAction::PrevPage)
        );
    }

    // ============================================================================
    // Page Jump Tests
    // ============================================================================

    #[test]
    fn number_keys_jump_to_pages() {
        let ctx = DetailActionContext::new(DetailPage::Overview);

        let expected_pages = [
            ('1', DetailPage::Overview),
            ('2', DetailPage::ScoreBreakdown),
            ('3', DetailPage::Context),
            ('4', DetailPage::Dependencies),
            ('5', DetailPage::GitContext),
            ('6', DetailPage::Patterns),
            ('7', DetailPage::DataFlow),
            ('8', DetailPage::Responsibilities),
        ];

        for (digit, expected_page) in expected_pages {
            assert_eq!(
                classify_detail_key(key(KeyCode::Char(digit)), ctx),
                Some(DetailAction::JumpToPage(expected_page)),
                "digit '{}' should jump to {:?}",
                digit,
                expected_page
            );
        }
    }

    #[test]
    fn page_from_digit_returns_none_for_invalid() {
        assert_eq!(page_from_digit('0'), None);
        assert_eq!(page_from_digit('9'), None);
        assert_eq!(page_from_digit('a'), None);
    }

    // ============================================================================
    // Selection Movement Tests
    // ============================================================================

    #[test]
    fn down_moves_selection_positive() {
        let ctx = DetailActionContext::new(DetailPage::Overview);
        assert_eq!(
            classify_detail_key(key(KeyCode::Down), ctx),
            Some(DetailAction::MoveSelection(1))
        );
    }

    #[test]
    fn j_moves_selection_positive() {
        let ctx = DetailActionContext::new(DetailPage::Overview);
        assert_eq!(
            classify_detail_key(key(KeyCode::Char('j')), ctx),
            Some(DetailAction::MoveSelection(1))
        );
    }

    #[test]
    fn up_moves_selection_negative() {
        let ctx = DetailActionContext::new(DetailPage::Overview);
        assert_eq!(
            classify_detail_key(key(KeyCode::Up), ctx),
            Some(DetailAction::MoveSelection(-1))
        );
    }

    #[test]
    fn k_moves_selection_negative() {
        let ctx = DetailActionContext::new(DetailPage::Overview);
        assert_eq!(
            classify_detail_key(key(KeyCode::Char('k')), ctx),
            Some(DetailAction::MoveSelection(-1))
        );
    }

    // ============================================================================
    // Context-Sensitive Copy Tests
    // ============================================================================

    #[test]
    fn c_on_context_page_copies_context() {
        let ctx = DetailActionContext::new(DetailPage::Context);
        assert_eq!(
            classify_detail_key(key(KeyCode::Char('c')), ctx),
            Some(DetailAction::CopyContext)
        );
    }

    #[test]
    fn c_on_other_pages_copies_page() {
        for page in [
            DetailPage::Overview,
            DetailPage::ScoreBreakdown,
            DetailPage::Dependencies,
            DetailPage::GitContext,
            DetailPage::Patterns,
            DetailPage::DataFlow,
            DetailPage::Responsibilities,
        ] {
            let ctx = DetailActionContext::new(page);
            assert_eq!(
                classify_detail_key(key(KeyCode::Char('c')), ctx),
                Some(DetailAction::CopyPage),
                "'c' on {:?} should copy page",
                page
            );
        }
    }

    #[test]
    fn p_on_context_page_copies_primary() {
        let ctx = DetailActionContext::new(DetailPage::Context);
        assert_eq!(
            classify_detail_key(key(KeyCode::Char('p')), ctx),
            Some(DetailAction::CopyPrimary)
        );
    }

    #[test]
    fn p_on_other_pages_does_nothing() {
        for page in [
            DetailPage::Overview,
            DetailPage::ScoreBreakdown,
            DetailPage::Dependencies,
            DetailPage::GitContext,
            DetailPage::Patterns,
            DetailPage::DataFlow,
            DetailPage::Responsibilities,
        ] {
            let ctx = DetailActionContext::new(page);
            assert_eq!(
                classify_detail_key(key(KeyCode::Char('p')), ctx),
                None,
                "'p' on {:?} should do nothing",
                page
            );
        }
    }

    // ============================================================================
    // Editor and Help Tests
    // ============================================================================

    #[test]
    fn e_opens_in_editor() {
        let ctx = DetailActionContext::new(DetailPage::Overview);
        assert_eq!(
            classify_detail_key(key(KeyCode::Char('e')), ctx),
            Some(DetailAction::OpenInEditor)
        );
    }

    #[test]
    fn o_opens_in_editor() {
        let ctx = DetailActionContext::new(DetailPage::Overview);
        assert_eq!(
            classify_detail_key(key(KeyCode::Char('o')), ctx),
            Some(DetailAction::OpenInEditor)
        );
    }

    #[test]
    fn question_mark_shows_help() {
        let ctx = DetailActionContext::new(DetailPage::Overview);
        assert_eq!(
            classify_detail_key(key(KeyCode::Char('?')), ctx),
            Some(DetailAction::ShowHelp)
        );
    }

    // ============================================================================
    // Unknown Key Tests
    // ============================================================================

    #[test]
    fn unknown_keys_return_none() {
        let ctx = DetailActionContext::new(DetailPage::Overview);

        assert_eq!(classify_detail_key(key(KeyCode::Char('x')), ctx), None);
        assert_eq!(classify_detail_key(key(KeyCode::Char('z')), ctx), None);
        assert_eq!(classify_detail_key(key(KeyCode::F(1)), ctx), None);
        assert_eq!(classify_detail_key(key(KeyCode::Char('0')), ctx), None);
        assert_eq!(classify_detail_key(key(KeyCode::Char('9')), ctx), None);
    }

    // ============================================================================
    // Pure Function Property Tests
    // ============================================================================

    #[test]
    fn classification_is_deterministic() {
        let ctx = DetailActionContext::new(DetailPage::Context);
        let k = key(KeyCode::Char('c'));

        let r1 = classify_detail_key(k, ctx);
        let r2 = classify_detail_key(k, ctx);
        assert_eq!(r1, r2);
    }

    #[test]
    fn context_affects_copy_action() {
        let k = key(KeyCode::Char('c'));

        let context_page = DetailActionContext::new(DetailPage::Context);
        let overview_page = DetailActionContext::new(DetailPage::Overview);

        assert_ne!(
            classify_detail_key(k, context_page),
            classify_detail_key(k, overview_page)
        );
    }

    #[test]
    fn navigation_keys_work_on_all_pages() {
        for page in [
            DetailPage::Overview,
            DetailPage::ScoreBreakdown,
            DetailPage::Context,
            DetailPage::Dependencies,
            DetailPage::GitContext,
            DetailPage::Patterns,
            DetailPage::DataFlow,
            DetailPage::Responsibilities,
        ] {
            let ctx = DetailActionContext::new(page);

            assert_eq!(
                classify_detail_key(key(KeyCode::Esc), ctx),
                Some(DetailAction::NavigateBack),
                "Esc should work on {:?}",
                page
            );
            assert_eq!(
                classify_detail_key(key(KeyCode::Tab), ctx),
                Some(DetailAction::NextPage),
                "Tab should work on {:?}",
                page
            );
            assert_eq!(
                classify_detail_key(key(KeyCode::BackTab), ctx),
                Some(DetailAction::PrevPage),
                "BackTab should work on {:?}",
                page
            );
        }
    }
}

#[cfg(test)]
mod property_tests {
    use super::*;
    use crossterm::event::KeyModifiers;
    use proptest::prelude::*;

    fn detail_page_strategy() -> impl Strategy<Value = DetailPage> {
        prop_oneof![
            Just(DetailPage::Overview),
            Just(DetailPage::ScoreBreakdown),
            Just(DetailPage::Context),
            Just(DetailPage::Dependencies),
            Just(DetailPage::GitContext),
            Just(DetailPage::Patterns),
            Just(DetailPage::DataFlow),
            Just(DetailPage::Responsibilities),
        ]
    }

    fn key_code_strategy() -> impl Strategy<Value = KeyCode> {
        prop_oneof![
            Just(KeyCode::Esc),
            Just(KeyCode::Char('q')),
            Just(KeyCode::Tab),
            Just(KeyCode::BackTab),
            Just(KeyCode::Right),
            Just(KeyCode::Left),
            Just(KeyCode::Char('l')),
            Just(KeyCode::Char('h')),
            Just(KeyCode::Char('j')),
            Just(KeyCode::Char('k')),
            Just(KeyCode::Up),
            Just(KeyCode::Down),
            Just(KeyCode::Char('c')),
            Just(KeyCode::Char('p')),
            Just(KeyCode::Char('e')),
            Just(KeyCode::Char('o')),
            Just(KeyCode::Char('?')),
            Just(KeyCode::Char('1')),
            Just(KeyCode::Char('2')),
            Just(KeyCode::Char('3')),
            Just(KeyCode::Char('4')),
            Just(KeyCode::Char('5')),
            Just(KeyCode::Char('6')),
            Just(KeyCode::Char('7')),
            Just(KeyCode::Char('8')),
        ]
    }

    proptest! {
        /// Property: Navigation keys are always available regardless of page.
        #[test]
        fn navigation_always_available(page in detail_page_strategy()) {
            let ctx = DetailActionContext::new(page);

            let esc = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
            let tab = KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE);
            let backtab = KeyEvent::new(KeyCode::BackTab, KeyModifiers::NONE);

            prop_assert_eq!(classify_detail_key(esc, ctx), Some(DetailAction::NavigateBack));
            prop_assert_eq!(classify_detail_key(tab, ctx), Some(DetailAction::NextPage));
            prop_assert_eq!(classify_detail_key(backtab, ctx), Some(DetailAction::PrevPage));
        }

        /// Property: Movement keys are always available.
        #[test]
        fn movement_always_available(page in detail_page_strategy()) {
            let ctx = DetailActionContext::new(page);

            let up = KeyEvent::new(KeyCode::Up, KeyModifiers::NONE);
            let down = KeyEvent::new(KeyCode::Down, KeyModifiers::NONE);

            prop_assert_eq!(classify_detail_key(up, ctx), Some(DetailAction::MoveSelection(-1)));
            prop_assert_eq!(classify_detail_key(down, ctx), Some(DetailAction::MoveSelection(1)));
        }

        /// Property: 'c' key always produces some copy action.
        #[test]
        fn c_always_copies_something(page in detail_page_strategy()) {
            let ctx = DetailActionContext::new(page);
            let c = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE);

            let action = classify_detail_key(c, ctx);
            prop_assert!(
                action == Some(DetailAction::CopyContext) || action == Some(DetailAction::CopyPage),
                "c should always produce a copy action"
            );
        }

        /// Property: Pure function - same input always produces same output.
        #[test]
        fn deterministic(
            code in key_code_strategy(),
            page in detail_page_strategy()
        ) {
            let ctx = DetailActionContext::new(page);
            let key = KeyEvent::new(code, KeyModifiers::NONE);

            let r1 = classify_detail_key(key, ctx);
            let r2 = classify_detail_key(key, ctx);
            prop_assert_eq!(r1, r2);
        }

        /// Property: Number keys 1-8 always produce JumpToPage action.
        #[test]
        fn number_keys_jump_to_page(
            digit in prop_oneof![
                Just('1'), Just('2'), Just('3'), Just('4'),
                Just('5'), Just('6'), Just('7'), Just('8')
            ],
            page in detail_page_strategy()
        ) {
            let ctx = DetailActionContext::new(page);
            let key = KeyEvent::new(KeyCode::Char(digit), KeyModifiers::NONE);

            let action = classify_detail_key(key, ctx);
            prop_assert!(
                matches!(action, Some(DetailAction::JumpToPage(_))),
                "digit {} should produce JumpToPage action",
                digit
            );
        }
    }
}
