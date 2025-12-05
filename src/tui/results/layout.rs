//! Layout utilities and help overlay.

use super::app::ResultsApp;
use crate::tui::theme::Theme;
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

/// Render help overlay
pub fn render_help_overlay(frame: &mut Frame, app: &ResultsApp) {
    let theme = Theme::default();

    // First render the list view underneath
    super::list_view::render(frame, app);

    // Calculate centered area for help overlay
    let area = frame.size();
    let help_area = centered_rect(60, 80, area);

    // Clear the area first
    frame.render_widget(Clear, help_area);

    // Create help content
    let help_text = vec![
        Line::from(vec![Span::styled(
            "KEYBOARD SHORTCUTS",
            Style::default()
                .fg(theme.accent())
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Navigation",
            Style::default()
                .fg(theme.accent())
                .add_modifier(Modifier::UNDERLINED),
        )]),
        Line::from("  ↑/k         Move up"),
        Line::from("  ↓/j         Move down"),
        Line::from("  g           Go to top"),
        Line::from("  G           Go to bottom"),
        Line::from("  PgUp/PgDn   Page up/down"),
        Line::from("  Enter       View details"),
        Line::from("  Esc         Back/Cancel"),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Search & Filter",
            Style::default()
                .fg(theme.accent())
                .add_modifier(Modifier::UNDERLINED),
        )]),
        Line::from("  /           Search"),
        Line::from("  s           Sort menu"),
        Line::from("  f           Filter menu"),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Actions",
            Style::default()
                .fg(theme.accent())
                .add_modifier(Modifier::UNDERLINED),
        )]),
        Line::from("  c           Copy file path to clipboard"),
        Line::from("  e           Open in $EDITOR"),
        Line::from("  o           Open at line number"),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Detail View",
            Style::default()
                .fg(theme.accent())
                .add_modifier(Modifier::UNDERLINED),
        )]),
        Line::from("  n           Next item"),
        Line::from("  p           Previous item"),
        Line::from("  Esc         Back to list"),
        Line::from(""),
        Line::from(vec![Span::styled(
            "General",
            Style::default()
                .fg(theme.accent())
                .add_modifier(Modifier::UNDERLINED),
        )]),
        Line::from("  ?           This help"),
        Line::from("  q           Quit"),
        Line::from("  Ctrl+C      Force quit"),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Press any key to close",
            Style::default().fg(theme.muted),
        )]),
    ];

    let help = Paragraph::new(help_text).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Help")
            .border_style(Style::default().fg(theme.accent())),
    );

    frame.render_widget(help, help_area);
}

/// Create a centered rectangle
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([
            ratatui::layout::Constraint::Percentage((100 - percent_y) / 2),
            ratatui::layout::Constraint::Percentage(percent_y),
            ratatui::layout::Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Horizontal)
        .constraints([
            ratatui::layout::Constraint::Percentage((100 - percent_x) / 2),
            ratatui::layout::Constraint::Percentage(percent_x),
            ratatui::layout::Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
