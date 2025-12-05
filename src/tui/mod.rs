//! Terminal User Interface (TUI) for debtmap analysis progress.
//!
//! This module provides a beautiful zen minimalist TUI using `ratatui` that visualizes
//! the entire analysis pipeline with hierarchical progress, smooth animations, and rich context.
//!
//! # Features
//!
//! - **Full pipeline visibility**: All 9 analysis stages displayed at once
//! - **Hierarchical progress**: Active stages expand to show sub-tasks
//! - **Rich context**: Counts, percentages, and real-time statistics
//! - **Smooth animations**: 60 FPS rendering with progress bars and sliding arrows
//! - **Responsive**: Adapts to terminal size gracefully
//! - **Zen minimalist design**: Clean, spacious, with subtle visual hierarchy
//!
//! # Usage
//!
//! ```rust,no_run
//! use debtmap::tui::TuiManager;
//!
//! // Create and initialize TUI
//! let mut tui = TuiManager::new()?;
//!
//! // Render a frame
//! tui.render()?;
//!
//! // TUI cleanup happens automatically on drop
//! # Ok::<(), std::io::Error>(())
//! ```

pub mod animation;
pub mod app;
pub mod layout;
pub mod renderer;
pub mod theme;

use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use app::App;
use layout::render_adaptive;

/// TUI manager for rendering the analysis progress interface
pub struct TuiManager {
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
    app: App,
    should_exit: Arc<AtomicBool>,
}

impl TuiManager {
    /// Create a new TUI manager and initialize the terminal
    pub fn new() -> io::Result<Self> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;

        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;

        let should_exit = Arc::new(AtomicBool::new(false));

        // Setup signal handlers for Ctrl+C and Ctrl+Z
        let exit_flag = should_exit.clone();
        std::thread::spawn(move || {
            loop {
                if exit_flag.load(Ordering::Relaxed) {
                    break;
                }

                // Poll for events with a timeout
                if event::poll(std::time::Duration::from_millis(100)).unwrap_or(false) {
                    if let Ok(Event::Key(key)) = event::read() {
                        // Handle Ctrl+C or Ctrl+Z
                        if key.code == KeyCode::Char('c')
                            && key.modifiers.contains(KeyModifiers::CONTROL)
                        {
                            // Attempt cleanup before exiting
                            let _ = disable_raw_mode();
                            let _ = execute!(io::stdout(), LeaveAlternateScreen);
                            eprintln!("\nInterrupted by user");
                            std::process::exit(130); // Standard exit code for Ctrl+C
                        }
                        if key.code == KeyCode::Char('z')
                            && key.modifiers.contains(KeyModifiers::CONTROL)
                        {
                            // Attempt cleanup before exiting
                            let _ = disable_raw_mode();
                            let _ = execute!(io::stdout(), LeaveAlternateScreen);
                            eprintln!("\nSuspended by user");
                            std::process::exit(148); // Standard exit code for Ctrl+Z
                        }
                    }
                }
            }
        });

        Ok(Self {
            terminal,
            app: App::new(),
            should_exit,
        })
    }

    /// Render the current frame
    pub fn render(&mut self) -> io::Result<()> {
        self.app.tick();
        self.terminal.draw(|f| render_adaptive(f, &self.app))?;
        Ok(())
    }

    /// Get mutable reference to the application state
    pub fn app_mut(&mut self) -> &mut App {
        &mut self.app
    }

    /// Get immutable reference to the application state
    pub fn app(&self) -> &App {
        &self.app
    }

    /// Clean up and restore terminal
    pub fn cleanup(&mut self) -> io::Result<()> {
        // Signal event thread to stop
        self.should_exit.store(true, Ordering::Relaxed);

        // Give the thread a moment to exit
        std::thread::sleep(std::time::Duration::from_millis(50));

        disable_raw_mode()?;
        execute!(self.terminal.backend_mut(), LeaveAlternateScreen)?;
        self.terminal.show_cursor()?;
        Ok(())
    }
}

impl Drop for TuiManager {
    fn drop(&mut self) {
        let _ = self.cleanup();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_initialization() {
        let app = App::new();
        assert_eq!(app.stages.len(), 9);
        assert_eq!(app.overall_progress, 0.0);
    }
}
