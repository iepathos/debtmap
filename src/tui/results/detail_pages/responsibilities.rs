//! Responsibilities page (Page 6) - Role and responsibility analysis.
//!
//! This page displays:
//! - God object responsibilities with method counts
//! - Single responsibility category for regular functions
//! - Responsibility-related notes and guidance

use super::components::{add_label_value, add_section_header};
use crate::priority::UnifiedDebtItem;
use crate::tui::theme::Theme;
use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

/// Build all lines for the responsibilities page (pure function).
///
/// This is public so text_extraction can reuse it for clipboard copy.
pub fn build_page_lines(item: &UnifiedDebtItem, theme: &Theme, width: u16) -> Vec<Line<'static>> {
    let mut lines = Vec::new();

    // Check for god object responsibilities first
    let god_object_shown = build_god_object_responsibilities(&mut lines, item, theme, width);

    // Fall back to single responsibility category
    if !god_object_shown {
        build_single_responsibility(&mut lines, item, theme, width);
    }

    // Add explanatory note for god objects
    if let Some(indicators) = &item.god_object_indicators {
        if indicators.is_god_object {
            build_god_object_note(&mut lines, theme);
        }
    }

    lines
}

/// Render responsibilities page showing responsibility information
pub fn render(frame: &mut Frame, item: &UnifiedDebtItem, area: Rect, theme: &Theme) {
    let lines = build_page_lines(item, theme, area.width);

    let paragraph = Paragraph::new(lines)
        .block(Block::default().borders(Borders::NONE))
        .wrap(Wrap { trim: false });

    frame.render_widget(paragraph, area);
}

/// Build god object responsibilities with method counts.
///
/// Returns true if responsibilities were rendered.
fn build_god_object_responsibilities(
    lines: &mut Vec<Line<'static>>,
    item: &UnifiedDebtItem,
    theme: &Theme,
    width: u16,
) -> bool {
    let Some(indicators) = &item.god_object_indicators else {
        return false;
    };

    // Show responsibilities even if score is below threshold (is_god_object = false)
    // The data is still useful for understanding file structure
    if indicators.responsibilities.is_empty() {
        return false;
    }

    add_section_header(lines, "responsibilities", theme);

    for resp in indicators.responsibilities.iter() {
        let method_count = indicators
            .responsibility_method_counts
            .get(resp)
            .copied()
            .unwrap_or(0);

        let resp_text = resp.to_lowercase();
        let count_text = if method_count > 0 {
            format!("{} methods", method_count)
        } else {
            String::new()
        };

        add_label_value(lines, &resp_text, count_text, theme, width);
    }

    true
}

/// Build single responsibility category for non-god-object functions.
/// Always shows something - falls back to "unclassified" if no category detected.
fn build_single_responsibility(
    lines: &mut Vec<Line<'static>>,
    item: &UnifiedDebtItem,
    theme: &Theme,
    width: u16,
) {
    add_section_header(lines, "responsibility", theme);
    let category = item
        .responsibility_category
        .as_deref()
        .unwrap_or("unclassified");
    add_label_value(lines, "category", category.to_lowercase(), theme, width);
}

/// Build explanatory note for god objects.
fn build_god_object_note(lines: &mut Vec<Line<'static>>, theme: &Theme) {
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("Note: ", Style::default().fg(theme.primary)),
        Span::styled(
            "God objects are structural issues (too many",
            Style::default().fg(theme.muted),
        ),
    ]));
    lines.push(Line::from(vec![Span::styled(
        "responsibilities). Focus on splitting by responsibility.",
        Style::default().fg(theme.muted),
    )]));
}
