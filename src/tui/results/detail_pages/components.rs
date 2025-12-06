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
/// # Example
/// ```text
/// cyclomatic    15
/// cognitive     22
/// nesting       4
/// ```
pub fn add_label_value(
    lines: &mut Vec<Line<'static>>,
    label: &str,
    value: String,
    theme: &Theme,
    _width: u16,
) {
    const INDENT: usize = 2;
    const LABEL_WIDTH: usize = 20; // Fixed column width for alignment
    const GAP: usize = 4; // Breathing room between label and value

    let label_with_indent = format!("{}{}", " ".repeat(INDENT), label);
    let padded_label = format!("{:width$}", label_with_indent, width = LABEL_WIDTH);
    let gap = " ".repeat(GAP);

    lines.push(Line::from(vec![
        Span::raw(padded_label),
        Span::raw(gap),
        Span::styled(value, Style::default().fg(theme.primary)),
    ]));
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
