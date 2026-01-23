//! List view rendering.

use super::app::ResultsApp;
use super::grouping;
use crate::priority::classification::Severity;
use crate::tui::theme::Theme;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
    Frame,
};

/// Horizontal margin constant per DESIGN.md spacing rules.
const HORIZONTAL_MARGIN: u16 = 1;

/// Apply horizontal margin for consistent padding.
fn apply_horizontal_margin(area: Rect) -> Rect {
    Rect {
        x: area.x.saturating_add(HORIZONTAL_MARGIN),
        y: area.y,
        width: area.width.saturating_sub(HORIZONTAL_MARGIN * 2),
        height: area.height,
    }
}

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
        .split(frame.area());

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
    let area = frame.area();

    // Create search box in center
    let search_area = Rect {
        x: area.width / 4,
        y: 2,
        width: area.width / 2,
        height: 3,
    };

    // Clear the area first to prevent background bleed-through
    frame.render_widget(Clear, search_area);

    let search_text = format!("Search: {}", app.query().search().query());
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
    let area = frame.area();

    // Create sort menu in center
    let menu_area = Rect {
        x: area.width / 3,
        y: area.height / 4,
        width: area.width / 3,
        height: 11,
    };

    // Clear the area first to prevent background bleed-through
    frame.render_widget(Clear, menu_area);

    let sort_options = super::sort::SortCriteria::all();
    let current_sort = app.query().sort_by();

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
    let area = frame.area();

    // Create filter menu in center
    let menu_area = Rect {
        x: area.width / 4,
        y: area.height / 6,
        width: area.width / 2,
        height: 16,
    };

    // Clear the area first to prevent background bleed-through
    frame.render_widget(Clear, menu_area);

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
    if !app.query().filters().is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Active filters:",
            Style::default().fg(theme.accent()),
        )));
        for filter in app.query().filters() {
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

/// Render header with summary metrics (minimal lowercase style)
fn render_header(frame: &mut Frame, app: &ResultsApp, area: Rect, theme: &Theme) {
    let analysis = app.analysis();
    let count_display = app.count_display();

    let header_text = vec![
        Line::from(vec![
            Span::raw("debtmap results"),
            Span::raw("  "),
            Span::styled(
                count_display.to_string(),
                Style::default().fg(theme.primary),
            ),
            Span::raw("  │  "),
            Span::raw("score "),
            Span::styled(
                format!("{:.0}", analysis.total_debt_score),
                Style::default().fg(theme.secondary()),
            ),
            Span::raw("  │  "),
            Span::styled(
                format!("{:.2}/1K loc", analysis.debt_density),
                Style::default().fg(theme.muted),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                format!("sort {}", app.query().sort_by().display_name()),
                Style::default().fg(theme.muted),
            ),
            Span::raw("  │  "),
            Span::styled(
                format!("filters {}", app.query().filters().len()),
                Style::default().fg(theme.muted),
            ),
        ]),
    ];

    // Apply horizontal margin per DESIGN.md
    let header_area = apply_horizontal_margin(area);

    let header = Paragraph::new(header_text)
        .block(Block::default().borders(Borders::BOTTOM))
        .style(Style::default());

    frame.render_widget(header, header_area);
}

/// Render list of items (always grouped by location)
fn render_list(frame: &mut Frame, app: &ResultsApp, area: Rect, theme: &Theme) {
    // Apply horizontal margin per DESIGN.md
    let list_area = apply_horizontal_margin(area);

    let items: Vec<ListItem> = render_grouped_list(app, list_area, theme);

    if items.is_empty() {
        let empty_text =
            if app.query().filters().is_empty() && app.query().search().query().is_empty() {
                "No debt items found"
            } else {
                "No items match current filters/search"
            };

        let empty = Paragraph::new(empty_text)
            .style(Style::default().fg(theme.muted))
            .block(Block::default().borders(Borders::NONE));

        frame.render_widget(empty, list_area);
    } else {
        let list = List::new(items).block(Block::default().borders(Borders::NONE));
        frame.render_widget(list, list_area);
    }
}

/// Render grouped list by location
fn render_grouped_list(app: &ResultsApp, area: Rect, theme: &Theme) -> Vec<ListItem<'static>> {
    let groups = grouping::group_by_location(app.filtered_items(), app.query().sort_by());

    let mut list_items = Vec::new();

    for (display_index, group) in groups.iter().skip(app.list().scroll_offset()).enumerate() {
        if list_items.len() >= area.height as usize {
            break;
        }

        let is_selected =
            (display_index + app.list().scroll_offset()) == app.list().selected_index();
        list_items.push(format_grouped_item(
            group,
            display_index + app.list().scroll_offset(),
            is_selected,
            theme,
        ));
    }

    list_items
}

/// Format a grouped item with badge and aggregated metrics
fn format_grouped_item(
    group: &grouping::LocationGroup,
    index: usize,
    is_selected: bool,
    theme: &Theme,
) -> ListItem<'static> {
    // Color based on combined score, not max individual severity
    let severity = Severity::from_score_100(group.combined_score);
    let severity_color = severity_to_color(severity);
    let indicator = if is_selected { "▸ " } else { "  " };

    let file_name = group
        .location
        .file
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown");

    // Badge for multiple issues (spec 267: show item count indicator)
    let badge = if group.items.len() > 1 {
        format!(" ({} items)", group.items.len())
    } else {
        String::new()
    };

    // Aggregated metrics
    let metrics = grouping::aggregate_metrics(group);

    let mut metric_parts = Vec::new();

    // Only show coverage if available (skip N/A)
    if let Some(cov) = metrics.coverage {
        metric_parts.push(format!("Cov:{:.0}%", cov.direct * 100.0));
    }

    if metrics.cognitive_complexity > 0 {
        metric_parts.push(format!("Cog:{}", metrics.cognitive_complexity));
    }
    if metrics.nesting_depth > 0 {
        metric_parts.push(format!("Nest:{}", metrics.nesting_depth));
    }
    if metrics.function_length > 0 {
        // Changed from "Len:" to "LOC:" for consistency (spec 207)
        metric_parts.push(format!("LOC:{}", metrics.function_length));
    }

    // For file-scope items (god files), show just the filename without "::[file-scope]"
    let location_display = if group.location.function == "[file-scope]" {
        file_name.to_string()
    } else {
        format!("{}::{}", file_name, group.location.function)
    };

    // Single line: indicator, rank, score (colored by severity), location, badge, metrics
    let line = Line::from(vec![
        Span::styled(indicator, Style::default().fg(theme.accent())),
        Span::styled(
            format!("#{:<4}", index + 1),
            Style::default().fg(theme.muted),
        ),
        Span::styled(
            format!("{:<7.1}", group.combined_score),
            Style::default().fg(severity_color),
        ),
        Span::raw("  "),
        Span::styled(location_display, Style::default().fg(theme.secondary())),
        Span::styled(badge, Style::default().fg(theme.muted)),
        Span::raw("  "),
        Span::styled(
            format!("({})", metric_parts.join(" ")),
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
        format!(
            "{}/{} items",
            app.list().selected_index() + 1,
            app.item_count()
        )
    } else {
        "0 items".to_string()
    };

    let footer_text = Line::from(vec![
        Span::styled(position_text, Style::default().fg(theme.muted)),
        Span::raw("  |  "),
        Span::styled("↑↓/jk", Style::default().fg(theme.accent())),
        Span::raw(":Nav  "),
        Span::styled("→/l", Style::default().fg(theme.accent())),
        Span::raw(":Details  "),
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

    // Apply horizontal margin per DESIGN.md
    let footer_area = apply_horizontal_margin(area);

    let footer = Paragraph::new(footer_text)
        .block(Block::default().borders(Borders::TOP))
        .style(Style::default());

    frame.render_widget(footer, footer_area);
}

/// Get color for severity level from Severity enum
fn severity_to_color(severity: Severity) -> Color {
    match severity {
        Severity::Critical => Color::Red,
        Severity::High => Color::LightRed,
        Severity::Medium => Color::Yellow,
        Severity::Low => Color::Green,
    }
}
