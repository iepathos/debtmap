//! Interactive TUI for exploring analysis results.
//!
//! This module provides a keyboard-driven interface for navigating,
//! searching, filtering, and acting on technical debt items.
//!
//! # Examples
//!
//! ```rust,no_run
//! use debtmap::tui::results::ResultsExplorer;
//!
//! let analysis = perform_analysis()?;
//! let mut explorer = ResultsExplorer::new(analysis);
//! explorer.run()?;
//! # Ok::<(), anyhow::Error>(())
//! ```

pub mod actions;
pub mod app;
pub mod detail_view;
pub mod filter;
pub mod layout;
pub mod list_view;
pub mod navigation;
pub mod search;
pub mod sort;

use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;

use crate::priority::UnifiedAnalysis;
use app::ResultsApp;

/// Results explorer TUI manager
pub struct ResultsExplorer {
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
    app: ResultsApp,
}

impl ResultsExplorer {
    /// Create a new results explorer from analysis results
    pub fn new(analysis: UnifiedAnalysis) -> Result<Self> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;

        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;

        let app = ResultsApp::new(analysis);

        Ok(Self { terminal, app })
    }

    /// Run the interactive TUI event loop
    pub fn run(&mut self) -> Result<()> {
        loop {
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
