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
pub mod results;
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
    terminal: Arc<std::sync::Mutex<Terminal<CrosstermBackend<io::Stdout>>>>,
    app: Arc<std::sync::Mutex<App>>,
    should_exit: Arc<AtomicBool>,
    render_thread: Option<std::thread::JoinHandle<()>>,
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
        let terminal = Arc::new(std::sync::Mutex::new(terminal));
        let app = Arc::new(std::sync::Mutex::new(App::new()));

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

        // Start background render thread for smooth 60 FPS updates
        let render_terminal = terminal.clone();
        let render_app = app.clone();
        let render_exit_flag = should_exit.clone();

        let render_thread = std::thread::spawn(move || {
            let frame_duration = std::time::Duration::from_millis(16); // ~60 FPS

            loop {
                if render_exit_flag.load(Ordering::Relaxed) {
                    break;
                }

                // Render frame
                if let (Ok(mut terminal), Ok(mut app)) = (render_terminal.lock(), render_app.lock())
                {
                    app.tick();
                    let _ = terminal.draw(|f| render_adaptive(f, &app));
                }

                std::thread::sleep(frame_duration);
            }
        });

        Ok(Self {
            terminal,
            app,
            should_exit,
            render_thread: Some(render_thread),
        })
    }

    /// Render the current frame (now handled by background thread, kept for compatibility)
    pub fn render(&mut self) -> io::Result<()> {
        // Background render thread handles continuous rendering at 60 FPS
        // This method is now a no-op but kept for API compatibility
        Ok(())
    }

    /// Get mutable reference to the application state
    pub fn app_mut(&mut self) -> std::sync::MutexGuard<'_, App> {
        self.app.lock().unwrap()
    }

    /// Get immutable reference to the application state (clone the Arc for read access)
    pub fn app(&self) -> Arc<std::sync::Mutex<App>> {
        self.app.clone()
    }

    /// Clean up and restore terminal
    pub fn cleanup(&mut self) -> io::Result<()> {
        // Signal all threads to stop
        self.should_exit.store(true, Ordering::Relaxed);

        // Wait for render thread to finish
        if let Some(handle) = self.render_thread.take() {
            let _ = handle.join();
        }

        // Give event thread a moment to exit
        std::thread::sleep(std::time::Duration::from_millis(50));

        disable_raw_mode()?;
        if let Ok(mut terminal) = self.terminal.lock() {
            execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
            terminal.show_cursor()?;
        }
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
        assert_eq!(app.stages.len(), 8);
        assert_eq!(app.overall_progress, 0.0);
    }
}
