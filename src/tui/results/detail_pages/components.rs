//! Shared rendering components for detail pages.

use crate::tui::theme::Theme;
use ratatui::{
    style::Style,
    text::{Line, Span},
};

/// Add lowercase section header with muted color
pub fn add_section_header(lines: &mut Vec<Line<'static>>, title: &str, theme: &Theme) {
    lines.push(Line::from(vec![Span::styled(
        title.to_string(),
        Style::default().fg(theme.muted),
    )]));
}

/// Add label-value pair with aligned columns
///
/// Creates clean label-value pairs using whitespace for visual separation.
/// Labels are left-aligned in a fixed-width column, with generous spacing
/// before the value. Embodies zen minimalism through restraint and space.
///
/// Multi-line values are automatically wrapped with continuation lines
/// indented to align with the first line's value column.
///
/// # Example
/// ```text
/// cyclomatic    15
/// cognitive     22
/// rationale     This is a long explanation that wraps
///               onto the next line with proper alignment
/// ```
pub fn add_label_value(
    lines: &mut Vec<Line<'static>>,
    label: &str,
    value: String,
    theme: &Theme,
    width: u16,
) {
    const INDENT: usize = 2;
    const LABEL_WIDTH: usize = 20; // Fixed column width for alignment
    const GAP: usize = 4; // Breathing room between label and value
    const VALUE_COLUMN: usize = LABEL_WIDTH + GAP; // Column where values start (24)

    let label_with_indent = format!("{}{}", " ".repeat(INDENT), label);
    let padded_label = format!("{:width$}", label_with_indent, width = LABEL_WIDTH);
    let gap = " ".repeat(GAP);

    // Calculate available width for value
    let value_width = width.saturating_sub(VALUE_COLUMN as u16) as usize;

    // Split value into lines that fit within value_width
    let value_lines = wrap_text(&value, value_width);

    // First line: label + gap + value
    if let Some(first_line) = value_lines.first() {
        lines.push(Line::from(vec![
            Span::raw(padded_label),
            Span::raw(gap),
            Span::styled(first_line.clone(), Style::default().fg(theme.primary)),
        ]));
    }

    // Continuation lines: indent to value column
    let continuation_indent = " ".repeat(VALUE_COLUMN);
    for continuation in value_lines.iter().skip(1) {
        lines.push(Line::from(vec![
            Span::raw(continuation_indent.clone()),
            Span::styled(continuation.clone(), Style::default().fg(theme.primary)),
        ]));
    }
}

/// Wrap text to fit within specified width, preserving words
fn wrap_text(text: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return vec![text.to_string()];
    }

    let mut result = Vec::new();
    let mut current_line = String::new();

    for word in text.split_whitespace() {
        // Check if adding this word would exceed width
        let potential_len = if current_line.is_empty() {
            word.len()
        } else {
            current_line.len() + 1 + word.len() // +1 for space
        };

        if potential_len <= width {
            // Word fits on current line
            if !current_line.is_empty() {
                current_line.push(' ');
            }
            current_line.push_str(word);
        } else {
            // Word doesn't fit, start new line
            if !current_line.is_empty() {
                result.push(current_line.clone());
                current_line.clear();
            }

            // If single word is longer than width, add it anyway
            current_line.push_str(word);
        }
    }

    // Add final line if not empty
    if !current_line.is_empty() {
        result.push(current_line);
    }

    // Return at least one line (even if empty)
    if result.is_empty() {
        result.push(String::new());
    }

    result
}

/// Add list section with truncation support
pub fn add_list_section(
    lines: &mut Vec<Line<'static>>,
    title: &str,
    items: &[String],
    max_display: usize,
    theme: &Theme,
) {
    if items.is_empty() {
        return; // Skip empty lists
    }

    // Subsection header with count
    lines.push(Line::from(vec![Span::raw(format!(
        "{} ({})",
        title,
        items.len()
    ))]));

    let display_count = items.len().min(max_display);
    for item in &items[..display_count] {
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(item.clone(), Style::default().fg(theme.secondary())),
        ]));
    }

    if items.len() > max_display {
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(
                format!("... {} more", items.len() - max_display),
                Style::default().fg(theme.muted),
            ),
        ]));
    }

    lines.push(Line::from("")); // Blank line after section
}

/// Add blank line separator
pub fn add_blank_line(lines: &mut Vec<Line<'static>>) {
    lines.push(Line::from(""));
}
