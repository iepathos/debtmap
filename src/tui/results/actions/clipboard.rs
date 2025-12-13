//! Clipboard operations for TUI actions.
//!
//! This module handles all clipboard I/O. Functions return Result
//! to handle cases where clipboard is unavailable (SSH sessions,
//! headless environments, etc.).
//!
//! # Design
//!
//! This module is part of the "Imperative Shell" in the Pure Core,
//! Imperative Shell pattern. It handles I/O operations while the
//! text_extraction module provides pure formatting functions.

use anyhow::Result;

/// Copy text to system clipboard and return status message.
///
/// Returns a success or failure message - never errors out, so the
/// TUI can continue operating even if clipboard is unavailable.
///
/// # Arguments
///
/// * `text` - The text to copy to clipboard
/// * `description` - A description for the status message (e.g., "path", "page content")
///
/// # Returns
///
/// A status message indicating success or failure.
pub fn copy_to_clipboard(text: &str, description: &str) -> Result<String> {
    use arboard::Clipboard;

    match Clipboard::new() {
        Ok(mut clipboard) => match clipboard.set_text(text) {
            Ok(_) => Ok(format!("Copied {} to clipboard", description)),
            Err(e) => {
                // Show error so user knows what happened
                Ok(format!("Clipboard error: {}", e))
            }
        },
        Err(e) => {
            // Clipboard not available (SSH, headless, etc.)
            Ok(format!("Clipboard not available: {}", e))
        }
    }
}

/// Check if clipboard is available.
///
/// Can be used to conditionally show clipboard-related UI hints.
pub fn clipboard_available() -> bool {
    use arboard::Clipboard;
    Clipboard::new().is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_copy_to_clipboard_returns_ok() {
        // This test verifies the function doesn't panic and returns Ok
        // The actual clipboard may or may not be available in CI
        let result = copy_to_clipboard("test", "test text");
        assert!(result.is_ok());
        let message = result.unwrap();
        // Should contain either success or error message
        assert!(message.contains("Copied") || message.contains("Clipboard"));
    }

    #[test]
    fn test_clipboard_available_returns_bool() {
        // Just ensure it doesn't panic - result depends on environment
        let _ = clipboard_available();
    }
}
