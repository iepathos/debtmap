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

    // Responsibilities section (for god objects)
    if let Some(indicators) = &item.god_object_indicators {
        if indicators.is_god_object && !indicators.responsibilities.is_empty() {
            lines.push(ratatui::text::Line::from(""));

            // Section header
            add_section_header(&mut lines, "responsibilities", theme);

            // Primary responsibility (for all items - spec 254)
            if let Some(category) = &item.responsibility_category {
                add_label_value(
                    &mut lines,
                    "primary responsibility",
                    category.clone(),
                    theme,
                    area.width,
                );
            }

            // List all responsibilities (no truncation)
            for resp in indicators.responsibilities.iter() {
                // Get method count from responsibility_method_counts
                let method_count = indicators
                    .responsibility_method_counts
                    .get(resp)
                    .copied()
                    .unwrap_or(0);

                // Lowercase responsibility name for consistency
                let resp_text = resp.to_lowercase();
                let count_text = if method_count > 0 {
                    format!("{} methods", method_count)
                } else {
                    String::new()
                };

                // Use the same column system as dependency metrics
                add_label_value(&mut lines, &resp_text, count_text, theme, area.width);
            }
        }
    } else if let Some(category) = &item.responsibility_category {
        // Show primary responsibility for non-god-object items
        lines.push(ratatui::text::Line::from(""));
        add_section_header(&mut lines, "responsibilities", theme);
        add_label_value(
            &mut lines,
            "primary responsibility",
            category.clone(),
            theme,
            area.width,
        );
    }

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
