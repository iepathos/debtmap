//! TUI action handling - dispatches user actions to appropriate handlers.
//!
//! This module composes pure text extraction with I/O operations
//! following the "Pure Core, Imperative Shell" pattern:
//!
//! - **text_extraction**: Pure functions for formatting data as text
//! - **clipboard**: I/O operations for system clipboard
//! - **editor**: I/O operations for launching external editors
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────┐
//! │                   Action Dispatcher                  │
//! │  (this module - composes pure + I/O)                │
//! └─────────────┬───────────────────┬───────────────────┘
//!               │                   │
//!       ┌───────▼───────┐   ┌───────▼───────┐
//!       │ Pure Core     │   │ Imperative    │
//!       │               │   │ Shell         │
//!       │ text_         │   │               │
//!       │ extraction.rs │   │ clipboard.rs  │
//!       │               │   │ editor.rs     │
//!       └───────────────┘   └───────────────┘
//! ```

pub mod clipboard;
pub mod editor;
pub mod text_extraction;

// Re-export commonly used items
pub use clipboard::copy_to_clipboard;
pub use editor::open_in_editor;
pub use text_extraction::{extract_page_text, format_debt_type_name, format_path_text};

use super::{app::ResultsApp, detail_page::DetailPage};
use crate::priority::UnifiedDebtItem;
use anyhow::Result;
use std::path::Path;

/// Copy file path to system clipboard and return status message.
pub fn copy_path_to_clipboard(path: &Path) -> Result<String> {
    let path_str = format_path_text(path);
    copy_to_clipboard(&path_str, "path")
}

/// Copy detail page content to clipboard and return status message.
pub fn copy_page_to_clipboard(
    item: &UnifiedDebtItem,
    page: DetailPage,
    app: &ResultsApp,
) -> Result<String> {
    let content = extract_page_text(item, page, app);
    copy_to_clipboard(&content, "page content")
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
        assert!(result.is_ok()); // Should always return Ok with status message
        let message = result.unwrap();
        assert!(message.contains("Copied") || message.contains("Clipboard"));
    }
}
