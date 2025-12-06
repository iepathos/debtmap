//! Git Context page (Page 3) - Git history and risk analysis.

use super::components::{add_blank_line, add_label_value, add_section_header};
use crate::priority::UnifiedDebtItem;
use crate::risk::context::ContextDetails;
use crate::tui::results::app::ResultsApp;
use crate::tui::theme::Theme;
use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

/// Render git context page showing change patterns and risk impact
pub fn render(
    frame: &mut Frame,
    _app: &ResultsApp,
    item: &UnifiedDebtItem,
    area: Rect,
    theme: &Theme,
) {
    let mut lines = Vec::new();

    if let Some(ref contextual_risk) = item.contextual_risk {
        // Look for git history context
        let git_context = contextual_risk
            .contexts
            .iter()
            .find(|ctx| ctx.provider == "git_history");

        if let Some(ctx) = git_context {
            if let ContextDetails::Historical {
                change_frequency,
                bug_density,
                age_days,
                author_count,
            } = &ctx.details
            {
                // Change Patterns section
                add_section_header(&mut lines, "change patterns", theme);
                add_label_value(
                    &mut lines,
                    "frequency",
                    format!("{:.2} changes/month", change_frequency),
                    theme,
                    area.width,
                );

                let stability = classify_stability(*change_frequency);
                add_label_value(
                    &mut lines,
                    "stability",
                    stability.to_string(),
                    theme,
                    area.width,
                );

                add_label_value(
                    &mut lines,
                    "bugs",
                    format!("{:.1}%", bug_density * 100.0),
                    theme,
                    area.width,
                );
                add_label_value(
                    &mut lines,
                    "age",
                    format!("{} days", age_days),
                    theme,
                    area.width,
                );
                add_label_value(
                    &mut lines,
                    "contributors",
                    author_count.to_string(),
                    theme,
                    area.width,
                );
                add_blank_line(&mut lines);
            }
        }

        // Risk Impact section
        add_section_header(&mut lines, "risk impact", theme);
        add_label_value(
            &mut lines,
            "base",
            format!("{:.1}", contextual_risk.base_risk),
            theme,
            area.width,
        );
        add_label_value(
            &mut lines,
            "contextual",
            format!("{:.1}", contextual_risk.contextual_risk),
            theme,
            area.width,
        );

        let multiplier = if contextual_risk.base_risk > 0.0 {
            contextual_risk.contextual_risk / contextual_risk.base_risk
        } else {
            1.0
        };
        add_label_value(
            &mut lines,
            "multiplier",
            format!("{:.2}x", multiplier),
            theme,
            area.width,
        );
        add_blank_line(&mut lines);
    }

    // Context Dampening section (if applicable)
    if let Some(ref file_type) = item.context_type {
        add_section_header(&mut lines, "context dampening", theme);
        add_label_value(
            &mut lines,
            "file type",
            format!("{:?}", file_type),
            theme,
            area.width,
        );

        if let Some(multiplier) = item.context_multiplier {
            let reduction = (1.0 - multiplier) * 100.0;
            add_label_value(
                &mut lines,
                "reduction",
                format!("{:.1}%", reduction),
                theme,
                area.width,
            );
        }
        add_blank_line(&mut lines);
    }

    // If no data available
    if lines.is_empty() {
        lines.push(Line::from(vec![Span::styled(
            "No git context data available",
            Style::default().fg(theme.muted),
        )]));
    }

    let paragraph = Paragraph::new(lines)
        .block(Block::default().borders(Borders::NONE))
        .wrap(Wrap { trim: false });

    frame.render_widget(paragraph, area);
}

/// Classify stability based on change frequency
fn classify_stability(change_frequency: f64) -> &'static str {
    if change_frequency < 1.0 {
        "Stable"
    } else if change_frequency < 5.0 {
        "Moderately Unstable"
    } else {
        "Highly Unstable"
    }
}
