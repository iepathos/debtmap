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

    // Apply margins to content area per DESIGN.md:
    // - 1-character margin on layout edges (horizontal)
    // - 1 line margin above/below content (vertical)
    let content_area = apply_content_margins(chunks[1]);

    // Route to appropriate page renderer
    if let Some(item) = app.selected_item() {
        match app.detail_page() {
            DetailPage::Overview => {
                detail_pages::overview::render(frame, app, item, content_area, &theme)
            }
            DetailPage::Dependencies => {
                detail_pages::dependencies::render(frame, app, item, content_area, &theme)
            }
            DetailPage::GitContext => {
                detail_pages::git_context::render(frame, app, item, content_area, &theme)
            }
            DetailPage::Patterns => detail_pages::patterns::render(
                frame,
                app,
                item,
                &app.analysis().data_flow_graph,
                content_area,
                &theme,
            ),
            DetailPage::DataFlow => detail_pages::data_flow::render(
                frame,
                app,
                item,
                &app.analysis().data_flow_graph,
                content_area,
                &theme,
            ),
            DetailPage::Responsibilities => {
                detail_pages::responsibilities::render(frame, item, content_area, &theme)
            }
        }
    } else {
        let empty = Paragraph::new("No item selected").style(Style::default().fg(theme.muted));
        frame.render_widget(empty, content_area);
    }

    // Render footer
    render_footer(frame, app, chunks[2], &theme);
}

/// Horizontal margin constant per DESIGN.md spacing rules.
const HORIZONTAL_MARGIN: u16 = 1;

/// Apply content margins per DESIGN.md spacing rules.
///
/// Creates breathing room around content with:
/// - 1 character horizontal margin on each side
/// - 1 line vertical margin top and bottom
fn apply_content_margins(area: Rect) -> Rect {
    const VERTICAL_MARGIN: u16 = 1;

    Rect {
        x: area.x.saturating_add(HORIZONTAL_MARGIN),
        y: area.y.saturating_add(VERTICAL_MARGIN),
        width: area.width.saturating_sub(HORIZONTAL_MARGIN * 2),
        height: area.height.saturating_sub(VERTICAL_MARGIN * 2),
    }
}

/// Apply horizontal margin only (for header/footer areas).
///
/// Creates consistent horizontal padding without vertical margin.
fn apply_horizontal_margin(area: Rect) -> Rect {
    Rect {
        x: area.x.saturating_add(HORIZONTAL_MARGIN),
        y: area.y,
        width: area.width.saturating_sub(HORIZONTAL_MARGIN * 2),
        height: area.height,
    }
}

/// Render header with page indicator
fn render_header(frame: &mut Frame, app: &ResultsApp, area: Rect, theme: &Theme) {
    let current_page = app.current_page_index() + 1; // 1-based for display
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

    // Apply horizontal margin per DESIGN.md
    let header_area = apply_horizontal_margin(area);

    let header = Paragraph::new(vec![Line::from(vec![Span::styled(
        position,
        Style::default().fg(theme.accent()),
    )])])
    .block(Block::default().borders(Borders::BOTTOM));

    frame.render_widget(header, header_area);
}

/// Render footer with action hints and status message
fn render_footer(frame: &mut Frame, app: &ResultsApp, area: Rect, theme: &Theme) {
    // If there's a status message, show it on first line, shortcuts on second line
    let lines = if let Some(status) = app.status_message() {
        let status_color = if status.starts_with('✓') {
            theme.success()
        } else {
            theme.warning()
        };

        vec![
            Line::from(vec![Span::styled(
                status,
                Style::default().fg(status_color),
            )]),
            Line::from(vec![
                Span::styled("Tab/←→", Style::default().fg(theme.accent())),
                Span::raw(": Pages  "),
                Span::styled("1-6", Style::default().fg(theme.accent())),
                Span::raw(": Jump  "),
                Span::styled("n/p", Style::default().fg(theme.accent())),
                Span::raw(": Items  "),
                Span::styled("c", Style::default().fg(theme.accent())),
                Span::raw(": Copy Page  "),
                Span::styled("e", Style::default().fg(theme.accent())),
                Span::raw(": Edit  "),
                Span::styled("?", Style::default().fg(theme.accent())),
                Span::raw(": Help  "),
                Span::styled("Esc", Style::default().fg(theme.accent())),
                Span::raw(": Back"),
            ]),
        ]
    } else {
        vec![Line::from(vec![
            Span::styled("Tab/←→", Style::default().fg(theme.accent())),
            Span::raw(": Pages  "),
            Span::styled("1-6", Style::default().fg(theme.accent())),
            Span::raw(": Jump  "),
            Span::styled("n/p", Style::default().fg(theme.accent())),
            Span::raw(": Items  "),
            Span::styled("c", Style::default().fg(theme.accent())),
            Span::raw(": Copy Page  "),
            Span::styled("e", Style::default().fg(theme.accent())),
            Span::raw(": Edit  "),
            Span::styled("?", Style::default().fg(theme.accent())),
            Span::raw(": Help  "),
            Span::styled("Esc", Style::default().fg(theme.accent())),
            Span::raw(": Back"),
        ])]
    };

    // Apply horizontal margin per DESIGN.md
    let footer_area = apply_horizontal_margin(area);

    let footer = Paragraph::new(lines).block(Block::default().borders(Borders::TOP));

    frame.render_widget(footer, footer_area);
}
