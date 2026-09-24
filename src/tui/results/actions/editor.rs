//! External editor launching for TUI actions.
//!
//! Handles launching editors at specific file:line locations. This module
//! properly suspends and resumes the TUI during editor operations.
//!
//! # Design
//!
//! This module is part of the "Imperative Shell" in the Pure Core,
//! Imperative Shell pattern. It handles I/O operations for launching
//! external processes. The command module provides pure argument construction.
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

mod command;

use anyhow::{Context, Result};
use crossterm::{
    cursor::MoveTo,
    event::{self, DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{
        Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode,
        enable_raw_mode,
    },
};
use std::io;
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
    let editor = selected_editor();
    let args = command::arguments(&editor, path, line);
    with_suspended_tui(suspend_tui, || launch_editor(&editor, &args), resume_tui)
}

fn selected_editor() -> String {
    std::env::var("EDITOR")
        .or_else(|_| std::env::var("VISUAL"))
        .unwrap_or_else(|_| "vim".to_string())
}

fn launch_editor(editor: &str, args: &[std::ffi::OsString]) -> Result<()> {
    let status = Command::new(editor)
        .args(args)
        .status()
        .with_context(|| format!("Failed to launch editor: {editor}"))?;
    anyhow::ensure!(status.success(), "Editor exited with status: {status}");
    Ok(())
}

/// Restore the TUI even when the editor fails; restoration errors take priority.
fn with_suspended_tui(
    suspend: impl FnOnce() -> Result<()>,
    action: impl FnOnce() -> Result<()>,
    resume: impl FnOnce() -> Result<()>,
) -> Result<()> {
    suspend()?;
    let result = action();
    resume()?;
    result
}

fn suspend_tui() -> Result<()> {
    disable_raw_mode().context("Failed to disable raw mode")?;
    execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture)
        .context("Failed to leave alternate screen")?;
    execute!(io::stdout(), Clear(ClearType::All), MoveTo(0, 0)).context("Failed to clear screen")
}

fn resume_tui() -> Result<()> {
    execute!(io::stdout(), EnterAlternateScreen, EnableMouseCapture)
        .context("Failed to re-enter alternate screen")?;
    enable_raw_mode().context("Failed to re-enable raw mode")?;
    drain_pending_events()
}

fn drain_pending_events() -> Result<()> {
    while event::poll(std::time::Duration::ZERO)? {
        let _ = event::read()?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    fn record_step(events: &RefCell<Vec<&str>>, name: &'static str, fail: bool) -> Result<()> {
        events.borrow_mut().push(name);
        anyhow::ensure!(!fail, "{name} failed");
        Ok(())
    }

    #[test]
    fn editor_session_restores_after_success_or_failure() {
        for fail in [false, true] {
            let events = RefCell::new(Vec::new());
            let result = with_suspended_tui(
                || record_step(&events, "suspend", false),
                || record_step(&events, "launch", fail),
                || record_step(&events, "resume", false),
            );
            assert_eq!(*events.borrow(), ["suspend", "launch", "resume"]);
            assert_eq!(result.is_err(), fail);
            if fail {
                assert_eq!(result.unwrap_err().to_string(), "launch failed");
            }
        }
    }

    #[test]
    fn failed_suspension_prevents_launch() {
        let events = RefCell::new(Vec::new());
        let result = with_suspended_tui(
            || record_step(&events, "suspend", true),
            || record_step(&events, "launch", false),
            || record_step(&events, "resume", false),
        );
        assert_eq!(*events.borrow(), ["suspend"]);
        assert_eq!(result.unwrap_err().to_string(), "suspend failed");
    }

    #[test]
    fn restoration_errors_take_priority() {
        for fail in [false, true] {
            let events = RefCell::new(Vec::new());
            let result = with_suspended_tui(
                || record_step(&events, "suspend", false),
                || record_step(&events, "launch", fail),
                || record_step(&events, "resume", true),
            );
            assert_eq!(*events.borrow(), ["suspend", "launch", "resume"]);
            assert_eq!(result.unwrap_err().to_string(), "resume failed");
        }
    }

    #[test]
    fn failed_launch_reports_editor_and_restores_tui() {
        let directory = tempfile::tempdir().unwrap();
        let missing = directory.path().join("missing-editor");
        let editor = missing.to_str().unwrap();
        let events = RefCell::new(Vec::new());
        let result = with_suspended_tui(
            || record_step(&events, "suspend", false),
            || launch_editor(editor, &[]),
            || record_step(&events, "resume", false),
        );
        assert_eq!(*events.borrow(), ["suspend", "resume"]);
        assert_eq!(
            result.unwrap_err().to_string(),
            format!("Failed to launch editor: {editor}")
        );
    }

    #[cfg(unix)]
    #[test]
    fn editor_exit_status_is_reported() {
        assert!(launch_editor("/bin/sh", &["-c".into(), "exit 0".into()]).is_ok());
        let error = launch_editor("/bin/sh", &["-c".into(), "exit 7".into()]).unwrap_err();
        assert_eq!(
            error.to_string(),
            "Editor exited with status: exit status: 7"
        );
    }
}
