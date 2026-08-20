//! Canonical display metadata for Results Explorer detail shortcuts.

use super::detail_actions::DetailAction;
use super::detail_page::DetailPage;
use crossterm::event::{KeyCode, KeyModifiers};

/// Help/footer section for a displayed detail shortcut.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetailShortcutSection {
    Navigation,
    Scrolling,
    Actions,
}

/// Display metadata tied to a supported detail action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DetailShortcut {
    pub keys: &'static str,
    pub description: &'static str,
    pub section: DetailShortcutSection,
    pub bindings: &'static [DetailShortcutBinding],
    pub footer: Option<(&'static str, &'static str)>,
}

/// Concrete key/action pair represented by displayed shortcut metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DetailShortcutBinding {
    pub code: KeyCode,
    pub modifiers: KeyModifiers,
    pub action: DetailAction,
}

macro_rules! binding {
    ($code:expr, $action:expr) => {
        DetailShortcutBinding {
            code: $code,
            modifiers: KeyModifiers::NONE,
            action: $action,
        }
    };
    ($code:expr, $modifiers:expr, $action:expr) => {
        DetailShortcutBinding {
            code: $code,
            modifiers: $modifiers,
            action: $action,
        }
    };
}

/// Detail shortcuts displayed by both the help overlay and condensed footer.
pub const DETAIL_SHORTCUTS: &[DetailShortcut] = &[
    DetailShortcut {
        keys: "Esc/q/h/⌫",
        description: "Back to list",
        section: DetailShortcutSection::Navigation,
        bindings: &[
            binding!(KeyCode::Esc, DetailAction::NavigateBack),
            binding!(KeyCode::Char('q'), DetailAction::NavigateBack),
            binding!(KeyCode::Char('h'), DetailAction::NavigateBack),
            binding!(KeyCode::Backspace, DetailAction::NavigateBack),
        ],
        footer: Some(("q/Esc", "Back")),
    },
    DetailShortcut {
        keys: "←/→/Tab/⇧Tab/l",
        description: "Previous/next page",
        section: DetailShortcutSection::Navigation,
        bindings: &[
            binding!(KeyCode::Left, DetailAction::PrevPage),
            binding!(KeyCode::Right, DetailAction::NextPage),
            binding!(KeyCode::Tab, DetailAction::NextPage),
            binding!(KeyCode::BackTab, DetailAction::PrevPage),
            binding!(KeyCode::Char('l'), DetailAction::NextPage),
        ],
        footer: Some(("←/→", "Pages")),
    },
    DetailShortcut {
        keys: "1-8",
        description: "Jump to page",
        section: DetailShortcutSection::Navigation,
        bindings: &[
            binding!(
                KeyCode::Char('1'),
                DetailAction::JumpToPage(DetailPage::Overview)
            ),
            binding!(
                KeyCode::Char('2'),
                DetailAction::JumpToPage(DetailPage::ScoreBreakdown)
            ),
            binding!(
                KeyCode::Char('3'),
                DetailAction::JumpToPage(DetailPage::Context)
            ),
            binding!(
                KeyCode::Char('4'),
                DetailAction::JumpToPage(DetailPage::Dependencies)
            ),
            binding!(
                KeyCode::Char('5'),
                DetailAction::JumpToPage(DetailPage::GitContext)
            ),
            binding!(
                KeyCode::Char('6'),
                DetailAction::JumpToPage(DetailPage::Patterns)
            ),
            binding!(
                KeyCode::Char('7'),
                DetailAction::JumpToPage(DetailPage::DataFlow)
            ),
            binding!(
                KeyCode::Char('8'),
                DetailAction::JumpToPage(DetailPage::Responsibilities)
            ),
        ],
        footer: None,
    },
    DetailShortcut {
        keys: "↑↓/j/k",
        description: "Previous/next location",
        section: DetailShortcutSection::Navigation,
        bindings: &[
            binding!(KeyCode::Up, DetailAction::MoveSelection(-1)),
            binding!(KeyCode::Down, DetailAction::MoveSelection(1)),
            binding!(KeyCode::Char('j'), DetailAction::MoveSelection(1)),
            binding!(KeyCode::Char('k'), DetailAction::MoveSelection(-1)),
        ],
        footer: Some(("j/k", "Locations")),
    },
    DetailShortcut {
        keys: "[ / ]",
        description: "Previous/next finding",
        section: DetailShortcutSection::Navigation,
        bindings: &[
            binding!(KeyCode::Char('['), DetailAction::PrevGroupItem),
            binding!(KeyCode::Char(']'), DetailAction::NextGroupItem),
        ],
        footer: Some(("[ / ]", "Findings")),
    },
    DetailShortcut {
        keys: "Ctrl+D/U",
        description: "Scroll half page down/up",
        section: DetailShortcutSection::Scrolling,
        bindings: &[
            binding!(
                KeyCode::Char('d'),
                KeyModifiers::CONTROL,
                DetailAction::ScrollHalfPageDown
            ),
            binding!(
                KeyCode::Char('u'),
                KeyModifiers::CONTROL,
                DetailAction::ScrollHalfPageUp
            ),
        ],
        footer: None,
    },
    DetailShortcut {
        keys: "Ctrl+F/B PgDn/PgUp",
        description: "Scroll full page down/up",
        section: DetailShortcutSection::Scrolling,
        bindings: &[
            binding!(
                KeyCode::Char('f'),
                KeyModifiers::CONTROL,
                DetailAction::ScrollPageDown
            ),
            binding!(
                KeyCode::Char('b'),
                KeyModifiers::CONTROL,
                DetailAction::ScrollPageUp
            ),
            binding!(KeyCode::PageDown, DetailAction::ScrollPageDown),
            binding!(KeyCode::PageUp, DetailAction::ScrollPageUp),
        ],
        footer: None,
    },
    DetailShortcut {
        keys: "g/G",
        description: "Jump to content top/bottom",
        section: DetailShortcutSection::Scrolling,
        bindings: &[
            binding!(KeyCode::Char('g'), DetailAction::ScrollToTop),
            binding!(KeyCode::Char('G'), DetailAction::ScrollToBottom),
        ],
        footer: None,
    },
    DetailShortcut {
        keys: "c",
        description: "Copy current detail page",
        section: DetailShortcutSection::Actions,
        bindings: &[binding!(KeyCode::Char('c'), DetailAction::CopyPage)],
        footer: None,
    },
    DetailShortcut {
        keys: "C",
        description: "Copy full finding as LLM markdown",
        section: DetailShortcutSection::Actions,
        bindings: &[binding!(KeyCode::Char('C'), DetailAction::CopyItemAsLlm)],
        footer: None,
    },
    DetailShortcut {
        keys: "e/o",
        description: "Open finding in editor",
        section: DetailShortcutSection::Actions,
        bindings: &[
            binding!(KeyCode::Char('e'), DetailAction::OpenInEditor),
            binding!(KeyCode::Char('o'), DetailAction::OpenInEditor),
        ],
        footer: None,
    },
    DetailShortcut {
        keys: "?",
        description: "Show help",
        section: DetailShortcutSection::Actions,
        bindings: &[binding!(KeyCode::Char('?'), DetailAction::ShowHelp)],
        footer: Some(("?", "Help")),
    },
];
