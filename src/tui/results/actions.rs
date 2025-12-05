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

/// Open file in editor
pub fn open_in_editor(path: &Path, line: Option<usize>) -> Result<()> {
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

    cmd.spawn()
        .with_context(|| format!("Failed to launch editor: {}", editor))?;

    eprintln!("✓ Opened in {}: {}", editor, path.display());
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
    fn test_editor_command_construction() {
        // Can't easily test actual spawning, but we can verify the function exists
        let path = PathBuf::from("/tmp/test.rs");
        std::env::set_var("EDITOR", "echo"); // Safe command for testing

        // Should not panic
        let result = open_in_editor(&path, Some(42));
        assert!(result.is_ok());
    }
}
