//! User actions (clipboard, editor).

use anyhow::{Context, Result};
use std::path::Path;
use std::process::Command;

use super::app::DetailPage;
use crate::priority::UnifiedDebtItem;

/// Copy text to system clipboard and return status message
fn copy_to_clipboard(text: &str, description: &str) -> Result<String> {
    use arboard::Clipboard;

    match Clipboard::new() {
        Ok(mut clipboard) => match clipboard.set_text(text) {
            Ok(_) => Ok(format!("✓ Copied {} to clipboard", description)),
            Err(e) => {
                // Show error so user knows what happened
                Ok(format!("✗ Clipboard error: {}", e))
            }
        },
        Err(e) => {
            // Clipboard not available (SSH, headless, etc.)
            Ok(format!("✗ Clipboard not available: {}", e))
        }
    }
}

/// Copy file path to system clipboard and return status message
pub fn copy_path_to_clipboard(path: &Path) -> Result<String> {
    let path_str = path.to_string_lossy().to_string();
    copy_to_clipboard(&path_str, "path")
}

/// Copy detail page content to clipboard and return status message
pub fn copy_page_to_clipboard(item: &UnifiedDebtItem, page: DetailPage) -> Result<String> {
    let content = extract_page_text(item, page);
    copy_to_clipboard(&content, "page content")
}

/// Extract plain text content from a detail page
fn extract_page_text(item: &UnifiedDebtItem, page: DetailPage) -> String {
    match page {
        DetailPage::Overview => extract_overview_text(item),
        DetailPage::Dependencies => extract_dependencies_text(item),
        DetailPage::GitContext => extract_git_context_text(item),
        DetailPage::Patterns => extract_patterns_text(item),
        DetailPage::DataFlow => extract_data_flow_text(item),
    }
}

/// Extract overview page content as plain text
fn extract_overview_text(item: &UnifiedDebtItem) -> String {
    use crate::priority::classification::Severity;
    use crate::priority::DebtType;

    let mut output = String::new();

    // Location
    output.push_str(&format!("FILE: {}\n", item.location.file.display()));
    output.push_str(&format!("FUNCTION: {}\n", item.location.function));
    output.push_str(&format!("LINE: {}\n\n", item.location.line));

    // Score
    let score = item.unified_score.final_score.value();
    let severity = Severity::from_score_100(score).as_str().to_lowercase();
    output.push_str(&format!("SCORE: {:.1} [{}]\n\n", score, severity));

    // God Object structure (if applicable)
    if let DebtType::GodObject {
        methods,
        fields,
        responsibilities,
        lines: debt_lines,
        ..
    } = &item.debt_type
    {
        let detection_type = item
            .god_object_indicators
            .as_ref()
            .map(|i| &i.detection_type);

        let header = match detection_type {
            Some(crate::organization::DetectionType::GodClass) => "GOD OBJECT STRUCTURE",
            Some(crate::organization::DetectionType::GodFile) => "GOD FILE STRUCTURE",
            Some(crate::organization::DetectionType::GodModule) => "GOD MODULE STRUCTURE",
            None => "GOD OBJECT STRUCTURE",
        };

        output.push_str(&format!("{}:\n", header));

        let method_label = match detection_type {
            Some(crate::organization::DetectionType::GodClass) => "Methods",
            _ => "Functions",
        };
        output.push_str(&format!("  {}: {}\n", method_label, methods));

        if let Some(field_count) = fields {
            output.push_str(&format!("  Fields: {}\n", field_count));
        }

        output.push_str(&format!("  Responsibilities: {}\n", responsibilities));
        output.push_str(&format!("  LOC: {}\n\n", debt_lines));
    }

    // Complexity
    let is_god_object = matches!(item.debt_type, DebtType::GodObject { .. });
    let cyclomatic_label = if is_god_object {
        "Accumulated Cyclomatic"
    } else {
        "Cyclomatic"
    };
    let cognitive_label = if is_god_object {
        "Accumulated Cognitive"
    } else {
        "Cognitive"
    };
    let nesting_label = if is_god_object {
        "Max Nesting"
    } else {
        "Nesting"
    };

    output.push_str("COMPLEXITY:\n");
    output.push_str(&format!(
        "  {}: {}\n",
        cyclomatic_label, item.cyclomatic_complexity
    ));
    output.push_str(&format!(
        "  {}: {}\n",
        cognitive_label, item.cognitive_complexity
    ));
    output.push_str(&format!("  {}: {}\n", nesting_label, item.nesting_depth));

    if !is_god_object {
        output.push_str(&format!("  LOC: {}\n", item.function_length));
    }
    output.push('\n');

    // Coverage
    output.push_str("COVERAGE:\n");
    if let Some(coverage) = item.transitive_coverage.as_ref().map(|c| c.direct) {
        output.push_str(&format!("  {:.1}%\n\n", coverage * 100.0));
    } else {
        output.push_str("  No data\n\n");
    }

    // Recommendation
    output.push_str("RECOMMENDATION:\n");
    output.push_str(&format!(
        "  Action: {}\n\n",
        item.recommendation.primary_action
    ));
    output.push_str(&format!(
        "  Rationale: {}\n\n",
        item.recommendation.rationale
    ));

    // Debt type
    output.push_str("DEBT TYPE:\n");
    output.push_str(&format!("  {}\n", format_debt_type_name(&item.debt_type)));

    output
}

/// Extract dependencies page content as plain text
fn extract_dependencies_text(item: &UnifiedDebtItem) -> String {
    let mut output = String::new();
    output.push_str(&format!("DEPENDENCIES - {}\n\n", item.location.function));
    output.push_str("(Full dependency details available in TUI)\n");
    output.push_str(&format!("File: {}\n", item.location.file.display()));
    output
}

/// Extract git context page content as plain text
fn extract_git_context_text(item: &UnifiedDebtItem) -> String {
    let mut output = String::new();
    output.push_str(&format!("GIT CONTEXT - {}\n\n", item.location.function));

    if let Some(risk) = &item.contextual_risk {
        output.push_str(&format!("Base Risk: {:.2}\n", risk.base_risk));
        output.push_str(&format!("Contextual Risk: {:.2}\n", risk.contextual_risk));
        output.push_str(&format!("\nExplanation:\n{}\n", risk.explanation));

        if !risk.contexts.is_empty() {
            output.push_str("\nContexts:\n");
            for context in &risk.contexts {
                output.push_str(&format!("  - {:?}\n", context));
            }
        }
    } else {
        output.push_str("No git context available\n");
    }

    output.push_str(&format!("\nFile: {}\n", item.location.file.display()));
    output
}

/// Extract patterns page content as plain text
fn extract_patterns_text(item: &UnifiedDebtItem) -> String {
    let mut output = String::new();
    output.push_str(&format!("PATTERNS - {}\n\n", item.location.function));

    if let Some(is_pure) = item.is_pure {
        output.push_str(&format!("Pure Function: {}\n", is_pure));
    }

    if let Some(pattern) = &item.detected_pattern {
        output.push_str(&format!("Detected Pattern: {:?}\n", pattern));
    }

    output.push_str(&format!("\nFile: {}\n", item.location.file.display()));
    output
}

/// Extract data flow page content as plain text
fn extract_data_flow_text(item: &UnifiedDebtItem) -> String {
    let mut output = String::new();
    output.push_str(&format!("DATA FLOW - {}\n\n", item.location.function));
    output.push_str("(Full data flow details available in TUI)\n");
    output.push_str(&format!("File: {}\n", item.location.file.display()));
    output
}

/// Format debt type as human-readable name
fn format_debt_type_name(debt_type: &crate::priority::DebtType) -> String {
    use crate::priority::DebtType;
    match debt_type {
        DebtType::ComplexityHotspot { .. } => "High Complexity".to_string(),
        DebtType::TestingGap { .. } => "Testing Gap".to_string(),
        DebtType::DeadCode { .. } => "Dead Code".to_string(),
        DebtType::Duplication { .. } => "Duplication".to_string(),
        DebtType::Risk { .. } => "Risk".to_string(),
        DebtType::TestComplexityHotspot { .. } => "Test Complexity".to_string(),
        DebtType::TestTodo { .. } => "Test TODO".to_string(),
        DebtType::TestDuplication { .. } => "Test Duplication".to_string(),
        DebtType::ErrorSwallowing { .. } => "Error Swallowing".to_string(),
        DebtType::AllocationInefficiency { .. } => "Allocation Inefficiency".to_string(),
        DebtType::StringConcatenation { .. } => "String Concatenation".to_string(),
        DebtType::NestedLoops { .. } => "Nested Loops".to_string(),
        DebtType::BlockingIO { .. } => "Blocking I/O".to_string(),
        DebtType::SuboptimalDataStructure { .. } => "Suboptimal Data Structure".to_string(),
        DebtType::GodObject { .. } => "God Object".to_string(),
        DebtType::FeatureEnvy { .. } => "Feature Envy".to_string(),
        DebtType::PrimitiveObsession { .. } => "Primitive Obsession".to_string(),
        DebtType::MagicValues { .. } => "Magic Values".to_string(),
        DebtType::AssertionComplexity { .. } => "Assertion Complexity".to_string(),
        DebtType::FlakyTestPattern { .. } => "Flaky Test Pattern".to_string(),
        DebtType::AsyncMisuse { .. } => "Async Misuse".to_string(),
        DebtType::ResourceLeak { .. } => "Resource Leak".to_string(),
        DebtType::CollectionInefficiency { .. } => "Collection Inefficiency".to_string(),
        DebtType::ScatteredType { .. } => "Scattered Type".to_string(),
        DebtType::OrphanedFunctions { .. } => "Orphaned Functions".to_string(),
        DebtType::UtilitiesSprawl { .. } => "Utilities Sprawl".to_string(),
        _ => "Other".to_string(),
    }
}

/// Open file in editor (suspends TUI during editing)
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
    fn test_copy_path_succeeds_or_fails_gracefully() {
        let path = PathBuf::from("/tmp/test.rs");
        // This might fail in CI/headless, but should not panic
        let result = copy_path_to_clipboard(&path);
        assert!(result.is_ok()); // Should always return Ok with status message
        let message = result.unwrap();
        assert!(message.contains("Copied") || message.contains("Clipboard"));
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
