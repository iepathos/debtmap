//! List view rendering.

use super::app::ResultsApp;
use crate::priority::UnifiedDebtItem;
use crate::tui::theme::Theme;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

/// Render main list view
pub fn render(frame: &mut Frame, app: &ResultsApp) {
    let theme = Theme::default();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(0),    // List
            Constraint::Length(2), // Footer
        ])
        .split(frame.size());

    // Render header
    render_header(frame, app, chunks[0], &theme);

    // Render list
    render_list(frame, app, chunks[1], &theme);

    // Render footer
    render_footer(frame, app, chunks[2], &theme);
}

/// Render list view with search overlay
pub fn render_with_search(frame: &mut Frame, app: &ResultsApp) {
    // First render the normal list
    render(frame, app);

    // Then overlay search input
    let theme = Theme::default();
    let area = frame.size();

    // Create search box in center
    let search_area = Rect {
        x: area.width / 4,
        y: 2,
        width: area.width / 2,
        height: 3,
    };

    let search_text = format!("Search: {}", app.search().query());
    let search_widget = Paragraph::new(search_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Search (Esc to cancel, Enter to apply)")
                .border_style(Style::default().fg(theme.accent())),
        )
        .style(Style::default().fg(theme.primary));

    frame.render_widget(search_widget, search_area);
}

/// Render list view with sort menu overlay
pub fn render_with_sort_menu(frame: &mut Frame, app: &ResultsApp) {
    render(frame, app);

    let theme = Theme::default();
    let area = frame.size();

    // Create sort menu in center
    let menu_area = Rect {
        x: area.width / 3,
        y: area.height / 4,
        width: area.width / 3,
        height: 11,
    };

    let sort_options = super::sort::SortCriteria::all();
    let current_sort = app.sort_by();

    let items: Vec<ListItem> = sort_options
        .iter()
        .enumerate()
        .map(|(i, criteria)| {
            let prefix = if *criteria == current_sort {
                "▸ "
            } else {
                "  "
            };
            let text = format!("{}. {}{}", i + 1, prefix, criteria.display_name());
            ListItem::new(text).style(if *criteria == current_sort {
                Style::default()
                    .fg(theme.accent())
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.primary)
            })
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Sort By (press number, Esc to cancel)")
            .border_style(Style::default().fg(theme.accent())),
    );

    frame.render_widget(list, menu_area);
}

/// Render list view with filter menu overlay
pub fn render_with_filter_menu(frame: &mut Frame, app: &ResultsApp) {
    render(frame, app);

    let theme = Theme::default();
    let area = frame.size();

    // Create filter menu in center
    let menu_area = Rect {
        x: area.width / 4,
        y: area.height / 6,
        width: area.width / 2,
        height: 16,
    };

    let mut lines = vec![
        Line::from("Severity Filters:"),
        Line::from("  1. Critical"),
        Line::from("  2. High"),
        Line::from("  3. Medium"),
        Line::from("  4. Low"),
        Line::from(""),
        Line::from("Coverage Filters:"),
        Line::from("  n. No Coverage"),
        Line::from("  l. Low (0-30%)"),
        Line::from("  m. Medium (30-70%)"),
        Line::from("  h. High (70-100%)"),
        Line::from(""),
        Line::from("  c. Clear all filters"),
    ];

    // Show active filters
    if !app.filters().is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Active filters:",
            Style::default().fg(theme.accent()),
        )));
        for filter in app.filters() {
            lines.push(Line::from(format!("  • {}", filter.display_name())));
        }
    }

    let paragraph = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Filters (press key, Esc to cancel)")
            .border_style(Style::default().fg(theme.accent())),
    );

    frame.render_widget(paragraph, menu_area);
}

/// Render header with summary metrics
fn render_header(frame: &mut Frame, app: &ResultsApp, area: Rect, theme: &Theme) {
    let analysis = app.analysis();

    let header_text = vec![
        Line::from(vec![
            Span::styled("Debtmap Results", Style::default().fg(theme.accent())),
            Span::raw("  "),
            Span::styled(
                format!("Total: {} items", app.item_count()),
                Style::default().fg(theme.primary),
            ),
            Span::raw("  "),
            Span::styled(
                format!("Debt Score: {:.0}", analysis.total_debt_score),
                Style::default().fg(theme.secondary()),
            ),
            Span::raw("  "),
            Span::styled(
                format!("Density: {:.2}/1K LOC", analysis.debt_density),
                Style::default().fg(theme.muted),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                format!("Sort: {}", app.sort_by().display_name()),
                Style::default().fg(theme.muted),
            ),
            Span::raw("  "),
            Span::styled(
                format!("Filters: {}", app.filters().len()),
                Style::default().fg(theme.muted),
            ),
        ]),
    ];

    let header = Paragraph::new(header_text)
        .block(Block::default().borders(Borders::BOTTOM))
        .style(Style::default());

    frame.render_widget(header, area);
}

/// Render list of items
fn render_list(frame: &mut Frame, app: &ResultsApp, area: Rect, theme: &Theme) {
    let items: Vec<ListItem> = app
        .filtered_items()
        .enumerate()
        .skip(app.scroll_offset())
        .take(area.height as usize)
        .map(|(idx, item)| {
            let is_selected = idx == app.selected_index();
            format_list_item(item, idx, is_selected, theme)
        })
        .collect();

    if items.is_empty() {
        let empty_text = if app.filters().is_empty() && app.search().query().is_empty() {
            "No debt items found"
        } else {
            "No items match current filters/search"
        };

        let empty = Paragraph::new(empty_text)
            .style(Style::default().fg(theme.muted))
            .block(Block::default().borders(Borders::NONE));

        frame.render_widget(empty, area);
    } else {
        let list = List::new(items).block(Block::default().borders(Borders::NONE));
        frame.render_widget(list, area);
    }
}

/// Format a single list item
fn format_list_item(
    item: &UnifiedDebtItem,
    index: usize,
    is_selected: bool,
    theme: &Theme,
) -> ListItem<'static> {
    let severity = calculate_severity(item.unified_score.final_score);
    let severity_color = severity_color(severity);

    let indicator = if is_selected { "▸ " } else { "  " };

    let file_name = item
        .location
        .file
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown");

    let coverage_str = item
        .transitive_coverage
        .as_ref()
        .map(|c| format!("{:.0}%", c.direct))
        .unwrap_or_else(|| "N/A".to_string());

    let complexity = item.cyclomatic_complexity;

    let line = Line::from(vec![
        Span::styled(indicator, Style::default().fg(theme.accent())),
        Span::styled(
            format!("#{:<4}", index + 1),
            Style::default().fg(theme.muted),
        ),
        Span::styled(
            format!("{:<10}", severity),
            Style::default().fg(severity_color),
        ),
        Span::styled(
            format!("Score:{:<7.1}", item.unified_score.final_score),
            Style::default().fg(theme.primary),
        ),
        Span::raw("  "),
        Span::styled(
            format!("{}::{}", file_name, item.location.function),
            Style::default().fg(theme.secondary()),
        ),
        Span::raw("  "),
        Span::styled(
            format!("(Cov:{} Comp:{})", coverage_str, complexity),
            Style::default().fg(theme.muted),
        ),
    ]);

    let style = if is_selected {
        Style::default().bg(Color::DarkGray)
    } else {
        Style::default()
    };

    ListItem::new(line).style(style)
}

/// Render footer with navigation hints
fn render_footer(frame: &mut Frame, app: &ResultsApp, area: Rect, theme: &Theme) {
    let position_text = if app.item_count() > 0 {
        format!("{}/{} items", app.selected_index() + 1, app.item_count())
    } else {
        "0 items".to_string()
    };

    let footer_text = Line::from(vec![
        Span::styled(position_text, Style::default().fg(theme.muted)),
        Span::raw("  |  "),
        Span::styled("↑↓/jk", Style::default().fg(theme.accent())),
        Span::raw(":Nav  "),
        Span::styled("/", Style::default().fg(theme.accent())),
        Span::raw(":Search  "),
        Span::styled("s", Style::default().fg(theme.accent())),
        Span::raw(":Sort  "),
        Span::styled("f", Style::default().fg(theme.accent())),
        Span::raw(":Filter  "),
        Span::styled("?", Style::default().fg(theme.accent())),
        Span::raw(":Help  "),
        Span::styled("q", Style::default().fg(theme.accent())),
        Span::raw(":Quit"),
    ]);

    let footer = Paragraph::new(footer_text)
        .block(Block::default().borders(Borders::TOP))
        .style(Style::default());

    frame.render_widget(footer, area);
}

/// Calculate severity level from score
fn calculate_severity(score: f64) -> &'static str {
    if score >= 100.0 {
        "CRITICAL"
    } else if score >= 50.0 {
        "HIGH"
    } else if score >= 10.0 {
        "MEDIUM"
    } else {
        "LOW"
    }
}

/// Get color for severity level
fn severity_color(severity: &str) -> Color {
    match severity {
        "CRITICAL" => Color::Red,
        "HIGH" => Color::LightRed,
        "MEDIUM" => Color::Yellow,
        "LOW" => Color::Green,
        _ => Color::White,
    }
}
