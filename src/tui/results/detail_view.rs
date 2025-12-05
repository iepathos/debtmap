//! Detail view rendering for selected debt item.

use super::app::ResultsApp;
use crate::tui::theme::Theme;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

/// Render detail view for selected item
pub fn render(frame: &mut Frame, app: &ResultsApp) {
    let theme = Theme::default();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(0),    // Content
            Constraint::Length(2), // Footer
        ])
        .split(frame.size());

    // Render header
    render_header(frame, app, chunks[0], &theme);

    // Render content
    if let Some(item) = app.selected_item() {
        render_content(frame, item, chunks[1], &theme);
    } else {
        let empty = Paragraph::new("No item selected").style(Style::default().fg(theme.muted));
        frame.render_widget(empty, chunks[1]);
    }

    // Render footer
    render_footer(frame, app, chunks[2], &theme);
}

/// Render header
fn render_header(frame: &mut Frame, app: &ResultsApp, area: Rect, theme: &Theme) {
    let position = format!(
        "Detail View ({}/{})",
        app.selected_index() + 1,
        app.item_count()
    );

    let header = Paragraph::new(vec![Line::from(vec![
        Span::styled(position, Style::default().fg(theme.accent())),
        Span::raw("  "),
        Span::styled("Esc", Style::default().fg(theme.muted)),
        Span::raw(": Back  "),
        Span::styled("n/p", Style::default().fg(theme.muted)),
        Span::raw(": Next/Prev"),
    ])])
    .block(Block::default().borders(Borders::BOTTOM));

    frame.render_widget(header, area);
}

/// Render item details
#[allow(clippy::vec_init_then_push)]
fn render_content(
    frame: &mut Frame,
    item: &crate::priority::UnifiedDebtItem,
    area: Rect,
    theme: &Theme,
) {
    let mut lines = Vec::new();

    // Location section
    lines.push(Line::from(vec![Span::styled(
        "LOCATION",
        Style::default()
            .fg(theme.accent())
            .add_modifier(Modifier::BOLD),
    )]));

    lines.push(Line::from(vec![
        Span::raw("  File: "),
        Span::styled(
            item.location.file.display().to_string(),
            Style::default().fg(theme.primary),
        ),
    ]));

    lines.push(Line::from(vec![
        Span::raw("  Function: "),
        Span::styled(&item.location.function, Style::default().fg(theme.primary)),
    ]));

    lines.push(Line::from(vec![
        Span::raw("  Line: "),
        Span::styled(
            item.location.line.to_string(),
            Style::default().fg(theme.primary),
        ),
    ]));

    lines.push(Line::from(""));

    // Score section
    lines.push(Line::from(vec![Span::styled(
        "SCORE",
        Style::default()
            .fg(theme.accent())
            .add_modifier(Modifier::BOLD),
    )]));

    let severity = calculate_severity(item.unified_score.final_score);
    let severity_color = severity_color(severity);

    lines.push(Line::from(vec![
        Span::raw("  Total: "),
        Span::styled(
            format!("{:.1}", item.unified_score.final_score),
            Style::default().fg(theme.primary),
        ),
        Span::raw("  ["),
        Span::styled(severity, Style::default().fg(severity_color)),
        Span::raw("]"),
    ]));

    lines.push(Line::from(""));

    // Metrics section
    lines.push(Line::from(vec![Span::styled(
        "METRICS",
        Style::default()
            .fg(theme.accent())
            .add_modifier(Modifier::BOLD),
    )]));

    lines.push(Line::from(vec![
        Span::raw("  Cyclomatic Complexity: "),
        Span::styled(
            item.cyclomatic_complexity.to_string(),
            Style::default().fg(theme.primary),
        ),
    ]));

    lines.push(Line::from(vec![
        Span::raw("  Cognitive Complexity: "),
        Span::styled(
            item.cognitive_complexity.to_string(),
            Style::default().fg(theme.primary),
        ),
    ]));

    lines.push(Line::from(vec![
        Span::raw("  Nesting Depth: "),
        Span::styled(
            item.nesting_depth.to_string(),
            Style::default().fg(theme.primary),
        ),
    ]));

    lines.push(Line::from(vec![
        Span::raw("  Function Length: "),
        Span::styled(
            item.function_length.to_string(),
            Style::default().fg(theme.primary),
        ),
    ]));

    lines.push(Line::from(""));

    // Coverage section
    lines.push(Line::from(vec![Span::styled(
        "COVERAGE",
        Style::default()
            .fg(theme.accent())
            .add_modifier(Modifier::BOLD),
    )]));

    if let Some(coverage) = item.transitive_coverage.as_ref().map(|c| c.direct) {
        lines.push(Line::from(vec![
            Span::raw("  Coverage: "),
            Span::styled(
                format!("{:.1}%", coverage),
                Style::default().fg(coverage_color(coverage)),
            ),
        ]));
    } else {
        lines.push(Line::from(vec![
            Span::raw("  Coverage: "),
            Span::styled("No data", Style::default().fg(theme.muted)),
        ]));
    }

    lines.push(Line::from(""));

    // Recommendation section
    lines.push(Line::from(vec![Span::styled(
        "RECOMMENDATION",
        Style::default()
            .fg(theme.accent())
            .add_modifier(Modifier::BOLD),
    )]));

    lines.push(Line::from(vec![
        Span::raw("  Action: "),
        Span::styled(
            &item.recommendation.primary_action,
            Style::default().fg(theme.primary),
        ),
    ]));

    lines.push(Line::from(""));

    lines.push(Line::from(vec![
        Span::raw("  Rationale: "),
        Span::styled(
            &item.recommendation.rationale,
            Style::default().fg(theme.secondary()),
        ),
    ]));

    lines.push(Line::from(""));

    // Category section
    lines.push(Line::from(vec![Span::styled(
        "DEBT TYPE",
        Style::default()
            .fg(theme.accent())
            .add_modifier(Modifier::BOLD),
    )]));

    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled(
            format!("{:?}", item.debt_type),
            Style::default().fg(theme.primary),
        ),
    ]));

    let paragraph = Paragraph::new(lines)
        .block(Block::default().borders(Borders::NONE))
        .wrap(Wrap { trim: false });

    frame.render_widget(paragraph, area);
}

/// Render footer with action hints
fn render_footer(frame: &mut Frame, _app: &ResultsApp, area: Rect, theme: &Theme) {
    let footer_text = Line::from(vec![
        Span::styled("c", Style::default().fg(theme.accent())),
        Span::raw(":Copy  "),
        Span::styled("e", Style::default().fg(theme.accent())),
        Span::raw(":Editor  "),
        Span::styled("n/p", Style::default().fg(theme.accent())),
        Span::raw(":Next/Prev  "),
        Span::styled("?", Style::default().fg(theme.accent())),
        Span::raw(":Help  "),
        Span::styled("Esc", Style::default().fg(theme.accent())),
        Span::raw(":Back"),
    ]);

    let footer = Paragraph::new(footer_text).block(Block::default().borders(Borders::TOP));

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

/// Get color for coverage percentage
fn coverage_color(coverage: f64) -> Color {
    if coverage >= 70.0 {
        Color::Green
    } else if coverage >= 30.0 {
        Color::Yellow
    } else {
        Color::Red
    }
}
