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
pub mod detail_page;
pub mod detail_pages;
pub mod detail_view;
pub mod dsm_view;
pub mod filter;
pub mod grouping;
pub mod layout;
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
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;

use crate::priority::view::PreparedDebtView;
use crate::priority::UnifiedAnalysis;
use app::ResultsApp;

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
            // Check if we need to force a full redraw (e.g., after external editor)
            if self.app.take_needs_redraw() {
                self.terminal.clear()?;
            }

            // Render current state
            self.terminal.draw(|f| self.app.render(f))?;

            // Handle input events
            if event::poll(std::time::Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    // Handle Ctrl+C to quit
                    if key.code == KeyCode::Char('c')
                        && key.modifiers.contains(KeyModifiers::CONTROL)
                    {
                        break;
                    }

                    // Handle other keys
                    if self.app.handle_key(key)? {
                        break; // Exit requested
                    }
                }
            }
        }

        // Cleanup
        self.cleanup()?;
        Ok(())
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
