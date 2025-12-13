//! External editor launching for TUI actions.
//!
//! Handles launching editors at specific file:line locations. This module
//! properly suspends and resumes the TUI during editor operations.
//!
//! # Design
//!
//! This module is part of the "Imperative Shell" in the Pure Core,
//! Imperative Shell pattern. It handles I/O operations for launching
//! external processes.
//!
//! # Supported Editors
//!
//! The module automatically detects and handles line number syntax for:
//! - vim/nvim/vi: `+N file`
//! - VS Code: `--goto file:N`
//! - emacs: `+N file`
//! - Sublime Text: `file:N`
//! - Helix: `file:N`
//! - nano: `+N file`

use anyhow::{Context, Result};
use std::path::Path;
use std::process::Command;

/// Open file in editor (suspends TUI during editing).
///
/// This function:
/// 1. Suspends the TUI (disables raw mode, leaves alternate screen)
/// 2. Clears the screen to prevent visual artifacts
/// 3. Launches the editor and waits for it to exit
/// 4. Resumes the TUI (re-enters alternate screen, enables raw mode)
/// 5. Drains any pending input events
///
/// # Arguments
///
/// * `path` - Path to the file to open
/// * `line` - Optional line number to jump to
///
/// # Errors
///
/// Returns an error if:
/// - Terminal operations fail
/// - Editor cannot be launched
/// - Editor exits with non-zero status
pub fn open_in_editor(path: &Path, line: Option<usize>) -> Result<()> {
    use crossterm::{
        cursor::MoveTo,
        event::{DisableMouseCapture, EnableMouseCapture},
        execute,
        terminal::{
            disable_raw_mode, enable_raw_mode, Clear, ClearType, EnterAlternateScreen,
            LeaveAlternateScreen,
        },
    };
    use std::io;

    let editor = std::env::var("EDITOR")
        .or_else(|_| std::env::var("VISUAL"))
        .unwrap_or_else(|_| "vim".to_string());

    let mut cmd = Command::new(&editor);

    // Support common editor line number syntax
    match (editor.as_str(), line) {
        ("vim" | "nvim" | "vi", Some(n)) => {
            cmd.arg(format!("+{}", n));
            cmd.arg(path);
        }
        ("code" | "code-insiders", Some(n)) => {
            cmd.arg("--goto");
            cmd.arg(format!("{}:{}", path.display(), n));
        }
        ("emacs", Some(n)) => {
            cmd.arg(format!("+{}", n));
            cmd.arg(path);
        }
        ("subl" | "sublime" | "sublime_text", Some(n)) => {
            cmd.arg(format!("{}:{}", path.display(), n));
        }
        ("hx" | "helix", Some(n)) => {
            cmd.arg(format!("{}:{}", path.display(), n));
        }
        ("nano", Some(n)) => {
            cmd.arg(format!("+{}", n));
            cmd.arg(path);
        }
        _ => {
            // Default: just open the file
            cmd.arg(path);
        }
    }

    // Suspend TUI: disable raw mode, leave alternate screen, disable mouse
    disable_raw_mode().context("Failed to disable raw mode")?;
    execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture)
        .context("Failed to leave alternate screen")?;

    // Clear the main screen to prevent flash of old terminal content
    execute!(io::stdout(), Clear(ClearType::All), MoveTo(0, 0))
        .context("Failed to clear screen")?;

    // Launch editor and wait for it to complete
    let status = cmd
        .status()
        .with_context(|| format!("Failed to launch editor: {}", editor))?;

    // Resume TUI: re-enter alternate screen, enable mouse, re-enable raw mode
    execute!(io::stdout(), EnterAlternateScreen, EnableMouseCapture)
        .context("Failed to re-enter alternate screen")?;
    enable_raw_mode().context("Failed to re-enable raw mode")?;

    // Drain any pending events from the queue to avoid stale input
    use crossterm::event;
    while event::poll(std::time::Duration::from_millis(0))? {
        let _ = event::read()?;
    }

    if !status.success() {
        anyhow::bail!("Editor exited with status: {}", status);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    #[ignore] // Requires terminal context (TUI must be active)
    fn test_editor_command_construction() {
        // This test requires a terminal in raw mode, which isn't available
        // during normal test runs.
        // Manual testing: run with --ignored --nocapture
        let path = PathBuf::from("/tmp/test.rs");
        std::env::set_var("EDITOR", "true"); // Use `true` command

        let result = open_in_editor(&path, Some(42));
        assert!(result.is_ok());
    }
}
