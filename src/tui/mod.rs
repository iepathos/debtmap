//! Terminal User Interface (TUI) for debtmap analysis progress.
//!
//! This module provides a beautiful zen minimalist TUI using `ratatui` that visualizes
//! the entire analysis pipeline with hierarchical progress, smooth animations, and rich context.
//!
//! # Features
//!
//! - **Full pipeline visibility**: All 7 analysis stages displayed at once
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
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use parking_lot::Mutex;
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use app::App;
use layout::render_adaptive;

/// TUI manager for rendering the analysis progress interface
pub struct TuiManager {
    terminal: Arc<Mutex<Terminal<CrosstermBackend<io::Stdout>>>>,
    app: Arc<Mutex<App>>,
    should_exit: Arc<AtomicBool>,
    render_thread: Option<std::thread::JoinHandle<()>>,
}

impl TuiManager {
    /// Create a new TUI manager and initialize the terminal
    ///
    /// Following Stillwater's composition principle, initialization is split into:
    /// 1. Terminal setup (I/O shell)
    /// 2. Signal handler thread (extracted)
    /// 3. Render thread (extracted)
    pub fn new() -> io::Result<Self> {
        // Phase 1: Terminal initialization (I/O shell)
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;

        let backend = CrosstermBackend::new(stdout);
        let terminal = Arc::new(Mutex::new(Terminal::new(backend)?));
        let should_exit = Arc::new(AtomicBool::new(false));
        let app = Arc::new(Mutex::new(App::new()));

        // Phase 2: Start signal handler thread
        Self::spawn_signal_handler(should_exit.clone());

        // Phase 3: Start render thread
        let render_thread =
            Self::spawn_render_thread(terminal.clone(), app.clone(), should_exit.clone());

        Ok(Self {
            terminal,
            app,
            should_exit,
            render_thread: Some(render_thread),
        })
    }

    /// Spawn the signal handler thread for Ctrl+C and Ctrl+Z
    fn spawn_signal_handler(exit_flag: Arc<AtomicBool>) {
        std::thread::spawn(move || {
            while !exit_flag.load(Ordering::Relaxed) {
                if let Some(key) = Self::poll_key_event() {
                    Self::handle_control_key(key);
                }
            }
        });
    }

    /// Poll for a key event with timeout
    fn poll_key_event() -> Option<KeyEvent> {
        event::poll(std::time::Duration::from_millis(100))
            .ok()
            .filter(|&ready| ready)
            .and_then(|_| event::read().ok())
            .and_then(|evt| match evt {
                Event::Key(key) => Some(key),
                _ => None,
            })
    }

    /// Handle Ctrl+C and Ctrl+Z key combinations
    fn handle_control_key(key: KeyEvent) {
        let is_ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        match (key.code, is_ctrl) {
            (KeyCode::Char('c'), true) => Self::exit_with_cleanup("Interrupted by user", 130),
            (KeyCode::Char('z'), true) => Self::exit_with_cleanup("Suspended by user", 148),
            _ => {}
        }
    }

    /// Clean up terminal and exit with message
    fn exit_with_cleanup(message: &str, code: i32) {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        eprintln!("\n{}", message);
        std::process::exit(code);
    }

    /// Spawn the background render thread (60 FPS)
    fn spawn_render_thread(
        terminal: Arc<Mutex<Terminal<CrosstermBackend<io::Stdout>>>>,
        app: Arc<Mutex<App>>,
        exit_flag: Arc<AtomicBool>,
    ) -> std::thread::JoinHandle<()> {
        std::thread::spawn(move || {
            const FRAME_DURATION: std::time::Duration = std::time::Duration::from_millis(16);

            while !exit_flag.load(Ordering::Relaxed) {
                // Render frame - parking_lot::Mutex::lock() never fails (no poisoning)
                {
                    let mut terminal = terminal.lock();
                    let mut app = app.lock();
                    app.tick();
                    let _ = terminal.draw(|f| render_adaptive(f, &app));
                }
                std::thread::sleep(FRAME_DURATION);
            }
        })
    }

    /// Render the current frame (now handled by background thread, kept for compatibility)
    pub fn render(&mut self) -> io::Result<()> {
        // Background render thread handles continuous rendering at 60 FPS
        // This method is now a no-op but kept for API compatibility
        Ok(())
    }

    /// Get mutable reference to the application state
    /// parking_lot::Mutex::lock() never fails (no poisoning)
    pub fn app_mut(&mut self) -> parking_lot::MutexGuard<'_, App> {
        self.app.lock()
    }

    /// Get immutable reference to the application state (clone the Arc for read access)
    pub fn app(&self) -> Arc<Mutex<App>> {
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
        // parking_lot::Mutex::lock() never fails (no poisoning)
        let mut terminal = self.terminal.lock();
        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
        terminal.show_cursor()?;
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
        assert_eq!(app.stages.len(), 6); // 6 stages
        assert_eq!(app.overall_progress, 0.0);
    }
}
