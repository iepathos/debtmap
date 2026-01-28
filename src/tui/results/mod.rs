//! Interactive TUI for exploring analysis results.
//!
//! This module provides a keyboard-driven interface for navigating,
//! searching, filtering, and acting on technical debt items.
//!
//! # Examples
//!
//! ```rust,ignore
//! use debtmap::tui::results::ResultsExplorer;
//!
//! // Option 1: Create from UnifiedAnalysis (existing API)
//! let analysis = perform_analysis()?;
//! let mut explorer = ResultsExplorer::new(analysis)?;
//! explorer.run()?;
//!
//! // Option 2: Create from PreparedDebtView (spec 252 - unified pipeline)
//! let view = prepare_view_for_tui(&analysis);
//! let mut explorer = ResultsExplorer::from_prepared_view(view, &analysis)?;
//! explorer.run()?;
//! ```

pub mod actions;
pub mod app;
pub mod detail_actions;
pub mod detail_page;
pub mod detail_pages;
pub mod detail_view;
pub mod filter;
pub mod grouping;
pub mod layout;
pub mod list_actions;
pub mod list_state;
pub mod list_view;
pub mod nav_actions;
pub mod nav_state;
pub mod navigation;
pub mod page_availability;
pub mod query_state;
pub mod search;
pub mod sort;
pub mod view_mode;

use anyhow::Result;
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyModifiers,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;

use crate::priority::view::PreparedDebtView;
use crate::priority::UnifiedAnalysis;
use app::ResultsApp;

// ============================================================================
// PURE HELPER FUNCTIONS
// ============================================================================

/// Check if a key event is the quit key (Ctrl+C).
///
/// This is a pure function that can be easily tested.
#[inline]
fn is_quit_key(key: &KeyEvent) -> bool {
    key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL)
}

/// Poll for a key event with the given timeout.
///
/// Returns `Ok(Some(key))` if a key event occurred, `Ok(None)` if timeout
/// or non-key event, and propagates errors.
fn poll_key_event(timeout_ms: u64) -> Result<Option<KeyEvent>> {
    if !event::poll(std::time::Duration::from_millis(timeout_ms))? {
        return Ok(None);
    }

    match event::read()? {
        Event::Key(key) => Ok(Some(key)),
        _ => Ok(None),
    }
}

/// Results explorer TUI manager
pub struct ResultsExplorer {
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
    app: ResultsApp,
}

impl ResultsExplorer {
    /// Create a new results explorer from analysis results.
    ///
    /// This is the original API that internally creates a view.
    pub fn new(analysis: UnifiedAnalysis) -> Result<Self> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;

        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;

        let app = ResultsApp::new(analysis);

        Ok(Self { terminal, app })
    }

    /// Create a new results explorer from a prepared view (Spec 252).
    ///
    /// This constructor accepts a `PreparedDebtView` that was prepared
    /// by the view pipeline, ensuring consistent data across all outputs.
    ///
    /// The `analysis` parameter is still needed for:
    /// - Call graph access (for dependency traversal in detail views)
    /// - Data flow graph access (for purity analysis in detail views)
    ///
    /// # Arguments
    ///
    /// * `view` - The prepared view from `prepare_view_for_tui()`
    /// * `analysis` - The original analysis (for call graph and data flow access)
    pub fn from_prepared_view(view: PreparedDebtView, analysis: UnifiedAnalysis) -> Result<Self> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;

        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;

        let app = ResultsApp::from_prepared_view(view, analysis);

        Ok(Self { terminal, app })
    }

    /// Run the interactive TUI event loop
    pub fn run(&mut self) -> Result<()> {
        loop {
            self.app.expire_status_message();
            self.render_frame()?;

            if self.process_next_event()? {
                break;
            }
        }

        self.cleanup()
    }

    /// Render a single frame, clearing if redraw requested.
    fn render_frame(&mut self) -> Result<()> {
        if self.app.take_needs_redraw() {
            self.terminal.clear()?;
        }
        self.terminal.draw(|f| self.app.render(f))?;
        Ok(())
    }

    /// Process the next event from the terminal.
    ///
    /// Returns `Ok(true)` if the application should exit,
    /// `Ok(false)` to continue the event loop.
    fn process_next_event(&mut self) -> Result<bool> {
        let Some(key) = poll_key_event(100)? else {
            return Ok(false);
        };

        if is_quit_key(&key) {
            return Ok(true);
        }

        self.app.handle_key(key)
    }

    /// Clean up and restore terminal
    fn cleanup(&mut self) -> Result<()> {
        disable_raw_mode()?;
        execute!(
            self.terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        self.terminal.show_cursor()?;
        Ok(())
    }
}

impl Drop for ResultsExplorer {
    fn drop(&mut self) {
        let _ = self.cleanup();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyEventKind;

    fn make_key_event(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent {
            code,
            modifiers,
            kind: KeyEventKind::Press,
            state: crossterm::event::KeyEventState::NONE,
        }
    }

    #[test]
    fn is_quit_key_detects_ctrl_c() {
        let key = make_key_event(KeyCode::Char('c'), KeyModifiers::CONTROL);
        assert!(is_quit_key(&key));
    }

    #[test]
    fn is_quit_key_rejects_plain_c() {
        let key = make_key_event(KeyCode::Char('c'), KeyModifiers::NONE);
        assert!(!is_quit_key(&key));
    }

    #[test]
    fn is_quit_key_rejects_other_keys_with_ctrl() {
        let key = make_key_event(KeyCode::Char('q'), KeyModifiers::CONTROL);
        assert!(!is_quit_key(&key));
    }

    #[test]
    fn is_quit_key_rejects_escape() {
        let key = make_key_event(KeyCode::Esc, KeyModifiers::NONE);
        assert!(!is_quit_key(&key));
    }

    #[test]
    fn is_quit_key_handles_ctrl_c_with_shift() {
        // Ctrl+Shift+C should still be detected as quit (CONTROL is present)
        let key = make_key_event(
            KeyCode::Char('c'),
            KeyModifiers::CONTROL | KeyModifiers::SHIFT,
        );
        assert!(is_quit_key(&key));
    }
}
