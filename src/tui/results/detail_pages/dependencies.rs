//! Dependencies page (Page 2) - Call graph and blast radius.

use super::components::{add_blank_line, add_label_value, add_list_section, add_section_header};
use crate::priority::UnifiedDebtItem;
use crate::tui::results::app::ResultsApp;
use crate::tui::theme::Theme;
use ratatui::{
    layout::Rect,
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

/// Render dependencies page showing callers, callees, and metrics
pub fn render(
    frame: &mut Frame,
    _app: &ResultsApp,
    item: &UnifiedDebtItem,
    area: Rect,
    theme: &Theme,
) {
    let mut lines = Vec::new();

    // Upstream Callers section
    add_section_header(&mut lines, "UPSTREAM CALLERS", theme);
    if !item.upstream_callers.is_empty() {
        add_list_section(&mut lines, "", &item.upstream_callers, 10, theme);
    } else {
        add_label_value(&mut lines, "Count", "0".to_string(), theme);
        add_blank_line(&mut lines);
    }

    // Downstream Callees section
    add_section_header(&mut lines, "DOWNSTREAM CALLEES", theme);
    if !item.downstream_callees.is_empty() {
        add_list_section(&mut lines, "", &item.downstream_callees, 10, theme);
    } else {
        add_label_value(&mut lines, "Count", "0".to_string(), theme);
        add_blank_line(&mut lines);
    }

    // Dependency Metrics section
    add_section_header(&mut lines, "DEPENDENCY METRICS", theme);
    add_label_value(
        &mut lines,
        "Upstream Dependencies",
        item.upstream_dependencies.to_string(),
        theme,
    );
    add_label_value(
        &mut lines,
        "Downstream Dependencies",
        item.downstream_dependencies.to_string(),
        theme,
    );

    let blast_radius = item.upstream_dependencies + item.downstream_dependencies;
    add_label_value(&mut lines, "Blast Radius", blast_radius.to_string(), theme);

    // Critical path indicator (simplified - based on high dependency count)
    let is_critical = item.upstream_dependencies > 5 || item.downstream_dependencies > 10;
    add_label_value(
        &mut lines,
        "Critical Path",
        if is_critical { "Yes" } else { "No" }.to_string(),
        theme,
    );

    let paragraph = Paragraph::new(lines)
        .block(Block::default().borders(Borders::NONE))
        .wrap(Wrap { trim: false });

    frame.render_widget(paragraph, area);
}
