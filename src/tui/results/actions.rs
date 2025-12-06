//! User actions (clipboard, editor).

use anyhow::{Context, Result};
use std::path::Path;
use std::process::Command;

/// Copy file path to system clipboard
pub fn copy_path_to_clipboard(path: &Path) -> Result<()> {
    use arboard::Clipboard;

    let path_str = path.to_string_lossy().to_string();

    match Clipboard::new() {
        Ok(mut clipboard) => match clipboard.set_text(&path_str) {
            Ok(_) => {
                eprintln!("✓ Copied to clipboard: {}", path_str);
                Ok(())
            }
            Err(e) => {
                // Show error with path so user can manually copy
                eprintln!("✗ Clipboard error: {}", e);
                eprintln!("  Path: {}", path_str);
                Ok(()) // Don't fail, just inform user
            }
        },
        Err(e) => {
            // Clipboard not available (SSH, headless, etc.)
            eprintln!("✗ Clipboard not available: {}", e);
            eprintln!("  Path: {}", path_str);
            Ok(()) // Don't fail, just inform user
        }
    }
}

/// Open file in editor (suspends TUI during editing)
pub fn open_in_editor(path: &Path, line: Option<usize>) -> Result<()> {
    use crossterm::{
        event::{DisableMouseCapture, EnableMouseCapture},
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
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
    fn test_copy_path_succeeds_or_fails_gracefully() {
        let path = PathBuf::from("/tmp/test.rs");
        // This might fail in CI/headless, but should not panic
        let result = copy_path_to_clipboard(&path);
        assert!(result.is_ok()); // Should always return Ok (graceful failure)
    }

    #[test]
    #[ignore] // Requires terminal context (TUI must be active)
    fn test_editor_command_construction() {
        // This test requires a terminal in raw mode, which isn't available during normal test runs
        // Manual testing: run `cargo test test_editor_command_construction -- --ignored --nocapture`
        let path = PathBuf::from("/tmp/test.rs");
        std::env::set_var("EDITOR", "true"); // Use `true` command (always succeeds, does nothing)

        let result = open_in_editor(&path, Some(42));
        assert!(result.is_ok());
    }
}
