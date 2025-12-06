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

/// Add label-value pair with dotted leader connection
///
/// Creates a visual connection from label to value using dotted leaders (middle dots).
/// The dots are muted to recede visually while values use primary color for prominence.
///
/// # Example
/// ```text
/// cyclomatic ················ 15
/// cognitive ················· 22
/// nesting ··················· 4
/// ```
pub fn add_label_value(
    lines: &mut Vec<Line<'static>>,
    label: &str,
    value: String,
    theme: &Theme,
    width: u16,
) {
    const INDENT: usize = 2;
    const MIN_DOTS: usize = 3;

    let label_with_indent = format!("{}{}", " ".repeat(INDENT), label);
    let value_len = value.len();
    let label_len = label_with_indent.len();

    // Calculate dots: total_width - (label + space + value + space)
    let total_content_len = label_len + 1 + value_len + 1;
    let dots_needed = (width as usize)
        .saturating_sub(total_content_len)
        .max(MIN_DOTS); // Ensure minimum dots for visual consistency

    lines.push(Line::from(vec![
        Span::raw(label_with_indent),
        Span::raw(" "),
        Span::styled("·".repeat(dots_needed), Style::default().fg(theme.muted)),
        Span::raw(" "),
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
