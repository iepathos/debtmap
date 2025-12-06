//! Overview page (Page 1) - Core metrics and recommendation.

use super::components::{add_blank_line, add_label_value, add_section_header};
use crate::priority::UnifiedDebtItem;
use crate::tui::results::app::ResultsApp;
use crate::tui::theme::Theme;
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

/// Render overview page showing core information
pub fn render(
    frame: &mut Frame,
    _app: &ResultsApp,
    item: &UnifiedDebtItem,
    area: Rect,
    theme: &Theme,
) {
    let mut lines = Vec::new();

    // Location section
    add_section_header(&mut lines, "LOCATION", theme);
    add_label_value(
        &mut lines,
        "File",
        item.location.file.display().to_string(),
        theme,
    );
    add_label_value(
        &mut lines,
        "Function",
        item.location.function.clone(),
        theme,
    );
    add_label_value(&mut lines, "Line", item.location.line.to_string(), theme);
    add_blank_line(&mut lines);

    // Score section
    add_section_header(&mut lines, "SCORE", theme);
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
    add_blank_line(&mut lines);

    // Metrics section
    add_section_header(&mut lines, "METRICS", theme);
    add_label_value(
        &mut lines,
        "Cyclomatic Complexity",
        item.cyclomatic_complexity.to_string(),
        theme,
    );
    add_label_value(
        &mut lines,
        "Cognitive Complexity",
        item.cognitive_complexity.to_string(),
        theme,
    );
    add_label_value(
        &mut lines,
        "Nesting Depth",
        item.nesting_depth.to_string(),
        theme,
    );
    add_label_value(
        &mut lines,
        "Function Length",
        item.function_length.to_string(),
        theme,
    );
    add_blank_line(&mut lines);

    // Coverage section
    add_section_header(&mut lines, "COVERAGE", theme);
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
    add_blank_line(&mut lines);

    // Recommendation section
    add_section_header(&mut lines, "RECOMMENDATION", theme);
    add_label_value(
        &mut lines,
        "Action",
        item.recommendation.primary_action.clone(),
        theme,
    );
    add_blank_line(&mut lines);

    lines.push(Line::from(vec![
        Span::raw("  Rationale: "),
        Span::styled(
            item.recommendation.rationale.clone(),
            Style::default().fg(theme.secondary()),
        ),
    ]));
    add_blank_line(&mut lines);

    // Debt type section
    add_section_header(&mut lines, "DEBT TYPE", theme);
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
