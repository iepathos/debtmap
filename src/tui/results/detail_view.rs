//! Detail view rendering for selected debt item.

use super::app::{DetailPage, ResultsApp};
use super::detail_pages;
use crate::tui::theme::Theme;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

/// Render detail view for selected item
pub fn render(frame: &mut Frame, app: &ResultsApp) {
    let theme = Theme::default();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header with page indicator
            Constraint::Min(0),    // Content
            Constraint::Length(2), // Footer
        ])
        .split(frame.size());

    // Render header with page indicator
    render_header(frame, app, chunks[0], &theme);

    // Route to appropriate page renderer
    if let Some(item) = app.selected_item() {
        match app.detail_page() {
            DetailPage::Overview => {
                detail_pages::overview::render(frame, app, item, chunks[1], &theme)
            }
            DetailPage::Dependencies => {
                detail_pages::dependencies::render(frame, app, item, chunks[1], &theme)
            }
            DetailPage::GitContext => {
                detail_pages::git_context::render(frame, app, item, chunks[1], &theme)
            }
            DetailPage::Patterns => {
                detail_pages::patterns::render(frame, app, item, chunks[1], &theme)
            }
        }
    } else {
        let empty = Paragraph::new("No item selected").style(Style::default().fg(theme.muted));
        frame.render_widget(empty, chunks[1]);
    }

    // Render footer
    render_footer(frame, app, chunks[2], &theme);
}

/// Render header with page indicator
fn render_header(frame: &mut Frame, app: &ResultsApp, area: Rect, theme: &Theme) {
    let current_page = app.detail_page().index() + 1; // 1-based for display
    let total_pages = app.page_count();
    let page_name = app.detail_page().name();

    let position = format!(
        "Detail View ({}/{})  [Page {}/{}] {}",
        app.selected_index() + 1,
        app.item_count(),
        current_page,
        total_pages,
        page_name
    );

    let header = Paragraph::new(vec![Line::from(vec![Span::styled(
        position,
        Style::default().fg(theme.accent()),
    )])])
    .block(Block::default().borders(Borders::BOTTOM));

    frame.render_widget(header, area);
}

/// Render footer with action hints
fn render_footer(frame: &mut Frame, _app: &ResultsApp, area: Rect, theme: &Theme) {
    let footer_text = Line::from(vec![
        Span::styled("Tab/←→", Style::default().fg(theme.accent())),
        Span::raw(": Pages  "),
        Span::styled("1-4", Style::default().fg(theme.accent())),
        Span::raw(": Jump  "),
        Span::styled("n/p", Style::default().fg(theme.accent())),
        Span::raw(": Items  "),
        Span::styled("c", Style::default().fg(theme.accent())),
        Span::raw(": Copy  "),
        Span::styled("e", Style::default().fg(theme.accent())),
        Span::raw(": Edit  "),
        Span::styled("?", Style::default().fg(theme.accent())),
        Span::raw(": Help  "),
        Span::styled("Esc", Style::default().fg(theme.accent())),
        Span::raw(": Back"),
    ]);

    let footer = Paragraph::new(footer_text).block(Block::default().borders(Borders::TOP));

    frame.render_widget(footer, area);
}
