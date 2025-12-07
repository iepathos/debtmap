//! Dependencies page (Page 2) - Call graph and blast radius.

use super::components::{add_label_value, add_section_header};
use crate::priority::UnifiedDebtItem;
use crate::tui::results::app::ResultsApp;
use crate::tui::theme::Theme;
use ratatui::{
    layout::Rect,
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

/// Render dependencies page showing dependency metrics and blast radius
pub fn render(
    frame: &mut Frame,
    _app: &ResultsApp,
    item: &UnifiedDebtItem,
    area: Rect,
    theme: &Theme,
) {
    let mut lines = Vec::new();

    // Dependency Metrics section
    add_section_header(&mut lines, "dependency metrics", theme);
    add_label_value(
        &mut lines,
        "upstream",
        item.upstream_dependencies.to_string(),
        theme,
        area.width,
    );
    add_label_value(
        &mut lines,
        "downstream",
        item.downstream_dependencies.to_string(),
        theme,
        area.width,
    );

    let blast_radius = item.upstream_dependencies + item.downstream_dependencies;
    add_label_value(
        &mut lines,
        "blast radius",
        blast_radius.to_string(),
        theme,
        area.width,
    );

    // Critical path indicator (simplified - based on high dependency count)
    let is_critical = item.upstream_dependencies > 5 || item.downstream_dependencies > 10;
    add_label_value(
        &mut lines,
        "critical",
        if is_critical { "Yes" } else { "No" }.to_string(),
        theme,
        area.width,
    );

    // Add note for god objects about what matters
    if let Some(indicators) = &item.god_object_indicators {
        if indicators.is_god_object {
            lines.push(ratatui::text::Line::from(""));
            lines.push(ratatui::text::Line::from(vec![
                ratatui::text::Span::styled(
                    "Note: ",
                    ratatui::style::Style::default().fg(theme.primary),
                ),
                ratatui::text::Span::styled(
                    "God objects are structural issues (too many",
                    ratatui::style::Style::default().fg(theme.muted),
                ),
            ]));
            lines.push(ratatui::text::Line::from(vec![
                ratatui::text::Span::styled(
                    "responsibilities). Focus on functions/methods count.",
                    ratatui::style::Style::default().fg(theme.muted),
                ),
            ]));
            if blast_radius == 0 {
                lines.push(ratatui::text::Line::from(vec![
                    ratatui::text::Span::styled(
                        "Zero deps = all functions are simple (good!).",
                        ratatui::style::Style::default().fg(theme.muted),
                    ),
                ]));
            }
        }
    }

    let paragraph = Paragraph::new(lines)
        .block(Block::default().borders(Borders::NONE))
        .wrap(Wrap { trim: false });

    frame.render_widget(paragraph, area);
}
