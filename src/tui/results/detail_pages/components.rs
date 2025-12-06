//! Shared rendering components for detail pages.

use crate::tui::theme::Theme;
use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
};

/// Add ALL CAPS section header with accent color
pub fn add_section_header(lines: &mut Vec<Line<'static>>, title: &str, theme: &Theme) {
    lines.push(Line::from(vec![Span::styled(
        title.to_uppercase(),
        Style::default()
            .fg(theme.accent())
            .add_modifier(Modifier::BOLD),
    )]));
}

/// Add simple label: value line with 2-space indentation
pub fn add_label_value(lines: &mut Vec<Line<'static>>, label: &str, value: String, theme: &Theme) {
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::raw(format!("{}: ", label)),
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
