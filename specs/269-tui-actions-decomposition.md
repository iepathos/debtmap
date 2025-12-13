---
number: 269
title: TUI Actions Decomposition
category: maintainability
priority: low
status: draft
dependencies: []
created: 2025-12-13
---

# Specification 269: TUI Actions Decomposition

**Category**: maintainability
**Priority**: low
**Status**: draft
**Dependencies**: None (can proceed independently)

## Context

`src/tui/results/actions.rs` has grown to 1,134 lines, mixing:

1. **Text extraction** - Pure functions to format analysis data as text
2. **Clipboard operations** - I/O to copy text to system clipboard
3. **Editor launching** - I/O to open files in external editors
4. **Action dispatching** - Routing user actions to handlers

**Current Problems:**

```rust
// actions.rs mixes pure and I/O concerns:

// Pure text extraction (should be separate)
fn extract_debt_item_text(item: &DebtItem) -> String {
    format!("{}: {} (score: {})", item.file, item.description, item.score)
}

// I/O operation (different module)
fn copy_to_clipboard(text: &str) -> Result<()> {
    let mut clipboard = Clipboard::new()?;
    clipboard.set_text(text)?;
    Ok(())
}

// Action handler mixing both
fn handle_copy_action(state: &AppState) -> Result<()> {
    let text = extract_debt_item_text(&state.selected_item);  // Pure
    copy_to_clipboard(&text)?;  // I/O
    Ok(())
}
```

**Stillwater Philosophy:**

> "Pure Core, Imperative Shell" - Text extraction is pure computation; clipboard and editor are I/O.

## Objective

Decompose `actions.rs` into focused modules:

1. **Text extraction** - Pure functions for formatting
2. **Clipboard operations** - I/O for system clipboard
3. **Editor operations** - I/O for external editor launching
4. **Action dispatcher** - Thin routing layer

Result: Clear separation between pure text formatting and I/O operations.

## Requirements

### Functional Requirements

1. **Text Extraction Module**
   - Pure functions for formatting debt items, metrics, etc.
   - No I/O operations
   - Easily unit testable

2. **Clipboard Module**
   - Single responsibility: clipboard operations
   - Error handling for clipboard unavailability
   - Platform-agnostic interface

3. **Editor Module**
   - Launch external editors at specific file:line
   - Handle editor not found gracefully
   - Support configurable editors

4. **Action Dispatcher**
   - Route actions to appropriate handlers
   - Compose pure + I/O operations
   - Maintain action history (if needed)

### Non-Functional Requirements

1. **File Size Limits**
   - Each file under 400 lines
   - Clear single responsibility

2. **Testability**
   - Text extraction 100% unit testable
   - I/O operations mockable

3. **Error Handling**
   - Clipboard failures don't crash TUI
   - Editor launch failures reported gracefully

## Acceptance Criteria

- [ ] Text extraction functions in separate module with no I/O
- [ ] Clipboard operations isolated in own module
- [ ] Editor operations isolated in own module
- [ ] Action dispatcher composes modules cleanly
- [ ] Each file under 400 lines
- [ ] All text extraction functions have unit tests
- [ ] No clippy warnings

## Technical Details

### Target Module Structure

```
src/tui/results/
├── actions/
│   ├── mod.rs              (~100 lines) - Action dispatcher, re-exports
│   ├── text_extraction.rs  (~400 lines) - Pure text formatting
│   ├── clipboard.rs        (~150 lines) - Clipboard I/O
│   └── editor.rs           (~150 lines) - Editor launching
```

### Implementation Approach

**Phase 1: Extract Text Extraction**

```rust
// src/tui/results/actions/text_extraction.rs

//! Pure text extraction functions for TUI actions.
//!
//! All functions in this module are pure - they take data and return
//! formatted strings without any I/O operations.

use crate::core::DebtItem;
use crate::analysis::FileMetrics;

/// Format a debt item for display/copying
pub fn format_debt_item(item: &DebtItem) -> String {
    format!(
        "{}:{}\n  {}\n  Score: {:.1} | Priority: {}",
        item.file.display(),
        item.line,
        item.description,
        item.score,
        item.priority
    )
}

/// Format multiple debt items as a list
pub fn format_debt_items(items: &[DebtItem]) -> String {
    items
        .iter()
        .map(format_debt_item)
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// Format file metrics for display
pub fn format_file_metrics(metrics: &FileMetrics) -> String {
    format!(
        "File: {}\n\
         Complexity: {} (cyclomatic) / {} (cognitive)\n\
         Functions: {}\n\
         Lines: {}",
        metrics.path.display(),
        metrics.cyclomatic,
        metrics.cognitive,
        metrics.function_count,
        metrics.line_count
    )
}

/// Format a summary of analysis results
pub fn format_analysis_summary(
    total_files: usize,
    total_debt_items: usize,
    high_priority: usize,
) -> String {
    format!(
        "Analysis Summary\n\
         ================\n\
         Files analyzed: {}\n\
         Debt items found: {}\n\
         High priority: {}",
        total_files,
        total_debt_items,
        high_priority
    )
}

/// Format debt item for markdown export
pub fn format_debt_item_markdown(item: &DebtItem) -> String {
    format!(
        "### {}\n\n\
         - **Location**: `{}:{}`\n\
         - **Score**: {:.1}\n\
         - **Priority**: {}\n\n\
         {}",
        item.title(),
        item.file.display(),
        item.line,
        item.score,
        item.priority,
        item.description
    )
}

/// Format debt items as JSON
pub fn format_debt_items_json(items: &[DebtItem]) -> String {
    serde_json::to_string_pretty(items).unwrap_or_else(|_| "[]".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_debt_item_includes_all_fields() {
        let item = DebtItem {
            file: PathBuf::from("src/main.rs"),
            line: 42,
            description: "Complex function".to_string(),
            score: 85.5,
            priority: Priority::High,
        };

        let text = format_debt_item(&item);

        assert!(text.contains("src/main.rs"));
        assert!(text.contains("42"));
        assert!(text.contains("Complex function"));
        assert!(text.contains("85.5"));
        assert!(text.contains("High"));
    }

    #[test]
    fn test_format_empty_items_list() {
        let text = format_debt_items(&[]);
        assert!(text.is_empty());
    }

    #[test]
    fn test_format_markdown_is_valid() {
        let item = DebtItem::default();
        let md = format_debt_item_markdown(&item);
        assert!(md.starts_with("###"));
        assert!(md.contains("**Location**"));
    }
}
```

**Phase 2: Extract Clipboard Operations**

```rust
// src/tui/results/actions/clipboard.rs

//! Clipboard operations for TUI actions.
//!
//! This module handles all clipboard I/O. Functions return Result
//! to handle cases where clipboard is unavailable.

use anyhow::{Context, Result};
use arboard::Clipboard;

/// Copy text to system clipboard
pub fn copy_to_clipboard(text: &str) -> Result<()> {
    let mut clipboard = Clipboard::new()
        .context("Failed to access system clipboard")?;

    clipboard
        .set_text(text)
        .context("Failed to copy text to clipboard")?;

    Ok(())
}

/// Get text from system clipboard
pub fn get_from_clipboard() -> Result<String> {
    let mut clipboard = Clipboard::new()
        .context("Failed to access system clipboard")?;

    clipboard
        .get_text()
        .context("Failed to read from clipboard")
}

/// Check if clipboard is available
pub fn clipboard_available() -> bool {
    Clipboard::new().is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clipboard_available_returns_bool() {
        // Just ensure it doesn't panic
        let _ = clipboard_available();
    }

    // Note: Clipboard tests are flaky in CI, so we test the interface
    // rather than actual clipboard operations
}
```

**Phase 3: Extract Editor Operations**

```rust
// src/tui/results/actions/editor.rs

//! External editor launching for TUI actions.
//!
//! Handles launching editors at specific file:line locations.

use anyhow::{Context, Result};
use std::path::Path;
use std::process::Command;

/// Configuration for editor launching
#[derive(Clone, Debug)]
pub struct EditorConfig {
    /// Editor command (e.g., "code", "nvim", "emacs")
    pub command: String,
    /// Arguments for line number (e.g., ["--goto", "{file}:{line}"])
    pub line_args: Vec<String>,
}

impl Default for EditorConfig {
    fn default() -> Self {
        // Detect editor from environment
        if let Ok(editor) = std::env::var("VISUAL").or_else(|_| std::env::var("EDITOR")) {
            Self::for_editor(&editor)
        } else {
            Self {
                command: "code".to_string(),
                line_args: vec!["--goto".to_string(), "{file}:{line}".to_string()],
            }
        }
    }
}

impl EditorConfig {
    /// Create config for a known editor
    pub fn for_editor(editor: &str) -> Self {
        match editor {
            "code" | "code-insiders" => Self {
                command: editor.to_string(),
                line_args: vec!["--goto".to_string(), "{file}:{line}".to_string()],
            },
            "nvim" | "vim" => Self {
                command: editor.to_string(),
                line_args: vec!["+{line}".to_string(), "{file}".to_string()],
            },
            "emacs" | "emacsclient" => Self {
                command: editor.to_string(),
                line_args: vec!["+{line}".to_string(), "{file}".to_string()],
            },
            "subl" | "sublime" => Self {
                command: "subl".to_string(),
                line_args: vec!["{file}:{line}".to_string()],
            },
            _ => Self {
                command: editor.to_string(),
                line_args: vec!["{file}".to_string()],
            },
        }
    }

    /// Build command arguments for opening a file at a line
    fn build_args(&self, file: &Path, line: usize) -> Vec<String> {
        self.line_args
            .iter()
            .map(|arg| {
                arg.replace("{file}", &file.to_string_lossy())
                    .replace("{line}", &line.to_string())
            })
            .collect()
    }
}

/// Open file at specific line in external editor
pub fn open_in_editor(file: &Path, line: usize, config: &EditorConfig) -> Result<()> {
    let args = config.build_args(file, line);

    Command::new(&config.command)
        .args(&args)
        .spawn()
        .context(format!(
            "Failed to launch editor '{}' for {}:{}",
            config.command,
            file.display(),
            line
        ))?;

    Ok(())
}

/// Open file at specific line using default editor
pub fn open_in_default_editor(file: &Path, line: usize) -> Result<()> {
    let config = EditorConfig::default();
    open_in_editor(file, line, &config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_editor_config_for_vscode() {
        let config = EditorConfig::for_editor("code");
        assert_eq!(config.command, "code");
        assert!(config.line_args.contains(&"--goto".to_string()));
    }

    #[test]
    fn test_editor_config_for_nvim() {
        let config = EditorConfig::for_editor("nvim");
        assert_eq!(config.command, "nvim");
    }

    #[test]
    fn test_build_args_substitutes_placeholders() {
        let config = EditorConfig {
            command: "test".to_string(),
            line_args: vec!["{file}:{line}".to_string()],
        };

        let args = config.build_args(Path::new("src/main.rs"), 42);
        assert_eq!(args, vec!["src/main.rs:42"]);
    }
}
```

**Phase 4: Action Dispatcher**

```rust
// src/tui/results/actions/mod.rs

//! TUI action handling - dispatches user actions to appropriate handlers.
//!
//! This module composes pure text extraction with I/O operations
//! following the "Pure Core, Imperative Shell" pattern.

mod clipboard;
mod editor;
mod text_extraction;

pub use clipboard::{copy_to_clipboard, clipboard_available};
pub use editor::{open_in_editor, open_in_default_editor, EditorConfig};
pub use text_extraction::*;

use crate::tui::state::AppState;
use anyhow::Result;

/// Actions that can be performed from the TUI
#[derive(Clone, Debug)]
pub enum Action {
    /// Copy selected debt item to clipboard
    CopyDebtItem,
    /// Copy all visible debt items to clipboard
    CopyAllDebtItems,
    /// Copy as markdown
    CopyAsMarkdown,
    /// Copy as JSON
    CopyAsJson,
    /// Open selected item in editor
    OpenInEditor,
    /// Export analysis results
    ExportResults { format: ExportFormat },
}

#[derive(Clone, Debug)]
pub enum ExportFormat {
    Markdown,
    Json,
    Csv,
}

/// Result of an action execution
pub struct ActionResult {
    /// Message to display to user
    pub message: String,
    /// Whether the action succeeded
    pub success: bool,
}

impl ActionResult {
    pub fn success(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            success: true,
        }
    }

    pub fn failure(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            success: false,
        }
    }
}

/// Execute an action and return result
pub fn execute_action(action: Action, state: &AppState) -> ActionResult {
    match action {
        Action::CopyDebtItem => execute_copy_debt_item(state),
        Action::CopyAllDebtItems => execute_copy_all_debt_items(state),
        Action::CopyAsMarkdown => execute_copy_as_markdown(state),
        Action::CopyAsJson => execute_copy_as_json(state),
        Action::OpenInEditor => execute_open_in_editor(state),
        Action::ExportResults { format } => execute_export(state, format),
    }
}

fn execute_copy_debt_item(state: &AppState) -> ActionResult {
    let Some(item) = state.selected_debt_item() else {
        return ActionResult::failure("No item selected");
    };

    // Pure: extract text
    let text = format_debt_item(item);

    // I/O: copy to clipboard
    match copy_to_clipboard(&text) {
        Ok(()) => ActionResult::success("Copied debt item to clipboard"),
        Err(e) => ActionResult::failure(format!("Failed to copy: {}", e)),
    }
}

fn execute_copy_all_debt_items(state: &AppState) -> ActionResult {
    let items = state.visible_debt_items();
    if items.is_empty() {
        return ActionResult::failure("No debt items to copy");
    }

    // Pure: extract text
    let text = format_debt_items(items);

    // I/O: copy to clipboard
    match copy_to_clipboard(&text) {
        Ok(()) => ActionResult::success(format!("Copied {} items to clipboard", items.len())),
        Err(e) => ActionResult::failure(format!("Failed to copy: {}", e)),
    }
}

fn execute_copy_as_markdown(state: &AppState) -> ActionResult {
    let Some(item) = state.selected_debt_item() else {
        return ActionResult::failure("No item selected");
    };

    let text = format_debt_item_markdown(item);

    match copy_to_clipboard(&text) {
        Ok(()) => ActionResult::success("Copied as Markdown"),
        Err(e) => ActionResult::failure(format!("Failed to copy: {}", e)),
    }
}

fn execute_copy_as_json(state: &AppState) -> ActionResult {
    let items = state.visible_debt_items();
    let text = format_debt_items_json(items);

    match copy_to_clipboard(&text) {
        Ok(()) => ActionResult::success("Copied as JSON"),
        Err(e) => ActionResult::failure(format!("Failed to copy: {}", e)),
    }
}

fn execute_open_in_editor(state: &AppState) -> ActionResult {
    let Some(item) = state.selected_debt_item() else {
        return ActionResult::failure("No item selected");
    };

    match open_in_default_editor(&item.file, item.line) {
        Ok(()) => ActionResult::success(format!("Opening {}:{}", item.file.display(), item.line)),
        Err(e) => ActionResult::failure(format!("Failed to open editor: {}", e)),
    }
}

fn execute_export(state: &AppState, format: ExportFormat) -> ActionResult {
    let items = state.all_debt_items();

    let text = match format {
        ExportFormat::Markdown => format_debt_items_markdown(items),
        ExportFormat::Json => format_debt_items_json(items),
        ExportFormat::Csv => format_debt_items_csv(items),
    };

    // For now, copy to clipboard. Could also write to file.
    match copy_to_clipboard(&text) {
        Ok(()) => ActionResult::success(format!("Exported {} items as {:?}", items.len(), format)),
        Err(e) => ActionResult::failure(format!("Export failed: {}", e)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test action dispatch (without actual I/O)
    #[test]
    fn test_copy_with_no_selection_fails() {
        let state = AppState::empty();
        let result = execute_action(Action::CopyDebtItem, &state);
        assert!(!result.success);
        assert!(result.message.contains("No item selected"));
    }
}
```

### Migration Strategy

1. **Create module structure** - Empty files
2. **Extract text_extraction.rs** - Pure functions first
3. **Add tests for text extraction** - Easy to test, high value
4. **Extract clipboard.rs** - I/O isolation
5. **Extract editor.rs** - I/O isolation
6. **Create mod.rs dispatcher** - Compose modules
7. **Update callers** - Switch to new module
8. **Remove old actions.rs** - After migration complete

### Files to Create/Modify

1. **Create** `src/tui/results/actions/mod.rs`
2. **Create** `src/tui/results/actions/text_extraction.rs`
3. **Create** `src/tui/results/actions/clipboard.rs`
4. **Create** `src/tui/results/actions/editor.rs`
5. **Modify** `src/tui/results/mod.rs` - Update imports
6. **Delete** `src/tui/results/actions.rs` - After migration

## Dependencies

- **Prerequisites**: None
- **Affected Components**:
  - `src/tui/results/actions.rs`
  - TUI key handlers that call actions
- **External Dependencies**: `arboard` crate (already used)

## Testing Strategy

### Unit Tests

Text extraction functions are 100% unit testable:

```rust
#[test]
fn test_format_debt_item() {
    let item = DebtItem { /* ... */ };
    let text = format_debt_item(&item);
    // Assert all fields present
}

#[test]
fn test_format_json_is_valid() {
    let items = vec![DebtItem::default()];
    let json = format_debt_items_json(&items);
    assert!(serde_json::from_str::<Vec<DebtItem>>(&json).is_ok());
}
```

### Integration Tests

```rust
#[test]
fn test_action_dispatch_routing() {
    // Test that each action routes to correct handler
    // (mock clipboard/editor for CI)
}
```

## Documentation Requirements

### Code Documentation

- Module-level docs explaining pure vs I/O split
- Function docs with examples
- Editor configuration documented

### User Documentation

Document supported editors in README/help.

## Implementation Notes

### Error Handling

- Clipboard unavailable → show message, don't crash
- Editor not found → show message with help

### Platform Considerations

- Clipboard may not work in SSH sessions
- Editor detection varies by platform

## Migration and Compatibility

### Breaking Changes

None - internal restructuring only.

### Backward Compatibility

All action behavior preserved.

## Success Metrics

- Text extraction functions under 400 lines total
- Each I/O module under 200 lines
- 100% test coverage for text extraction
- Action dispatch clear and maintainable
