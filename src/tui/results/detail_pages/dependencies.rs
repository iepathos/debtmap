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
    app: &ResultsApp,
    item: &UnifiedDebtItem,
    area: Rect,
    theme: &Theme,
) {
    let mut lines = Vec::new();

    // Function-level Dependency Metrics section
    add_section_header(&mut lines, "function dependencies", theme);
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

    // File-level Coupling Metrics section (spec 201)
    render_file_coupling_section(&mut lines, app, item, theme, area.width);

    // Responsibility section - show for all items
    // God objects: show all responsibilities with method counts
    // Regular functions: show single responsibility category
    let god_object_responsibilities_shown = if let Some(indicators) = &item.god_object_indicators {
        if indicators.is_god_object && !indicators.responsibilities.is_empty() {
            lines.push(ratatui::text::Line::from(""));

            // Section header
            add_section_header(&mut lines, "responsibilities", theme);

            // List all responsibilities (sorted by count, no truncation)
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
            true
        } else {
            false
        }
    } else {
        false
    };

    // Fall back to single responsibility category if god object responsibilities weren't shown
    if !god_object_responsibilities_shown {
        if let Some(ref category) = item.responsibility_category {
            lines.push(ratatui::text::Line::from(""));
            add_section_header(&mut lines, "responsibility", theme);
            add_label_value(
                &mut lines,
                "category",
                category.to_lowercase(),
                theme,
                area.width,
            );
        }
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

/// Render file-level coupling metrics section (spec 201).
///
/// Looks up file-level metrics from the analysis and displays:
/// - Afferent coupling (Ca) - files that depend on this file
/// - Efferent coupling (Ce) - files this file depends on
/// - Instability (I = Ce / (Ca + Ce))
/// - Coupling classification (StableCore, HighlyCoupled, etc.)
fn render_file_coupling_section(
    lines: &mut Vec<ratatui::text::Line<'static>>,
    app: &ResultsApp,
    item: &UnifiedDebtItem,
    theme: &Theme,
    width: u16,
) {
    // Find the file-level metrics for this item's file
    let file_metrics = app
        .analysis()
        .file_items
        .iter()
        .find(|f| f.metrics.path == item.location.file);

    // Only show section if we have file-level coupling data
    let Some(file_item) = file_metrics else {
        return;
    };

    let metrics = &file_item.metrics;

    // Only show if we have meaningful coupling data
    let total_coupling = metrics.afferent_coupling + metrics.efferent_coupling;
    if total_coupling == 0 && metrics.instability == 0.0 {
        return;
    }

    lines.push(ratatui::text::Line::from(""));
    add_section_header(lines, "file coupling (spec 201)", theme);

    add_label_value(
        lines,
        "afferent (ca)",
        format!("{} files depend on this", metrics.afferent_coupling),
        theme,
        width,
    );

    add_label_value(
        lines,
        "efferent (ce)",
        format!("{} files depended on", metrics.efferent_coupling),
        theme,
        width,
    );

    add_label_value(
        lines,
        "instability",
        format!("{:.2} (0=stable, 1=unstable)", metrics.instability),
        theme,
        width,
    );

    // Derive coupling classification
    let classification = derive_coupling_classification(
        metrics.afferent_coupling,
        metrics.efferent_coupling,
        metrics.instability,
    );
    add_label_value(lines, "classification", classification, theme, width);

    // Add context note for extreme values
    if total_coupling > 15 {
        lines.push(ratatui::text::Line::from(vec![
            ratatui::text::Span::styled(
                "Warning: ",
                ratatui::style::Style::default().fg(ratatui::style::Color::Red),
            ),
            ratatui::text::Span::styled(
                "High coupling may indicate architectural issues.",
                ratatui::style::Style::default().fg(theme.muted),
            ),
        ]));
    } else if metrics.instability < 0.1 && metrics.afferent_coupling > 0 {
        lines.push(ratatui::text::Line::from(vec![
            ratatui::text::Span::styled(
                "Note: ",
                ratatui::style::Style::default().fg(theme.primary),
            ),
            ratatui::text::Span::styled(
                "Stable core - changes need careful review.",
                ratatui::style::Style::default().fg(theme.muted),
            ),
        ]));
    } else if metrics.instability > 0.9 {
        lines.push(ratatui::text::Line::from(vec![
            ratatui::text::Span::styled(
                "Note: ",
                ratatui::style::Style::default().fg(theme.success),
            ),
            ratatui::text::Span::styled(
                "Unstable leaf - safe to refactor.",
                ratatui::style::Style::default().fg(theme.muted),
            ),
        ]));
    }
}

/// Derive coupling classification from metrics (same logic as CouplingClassification).
fn derive_coupling_classification(afferent: usize, efferent: usize, instability: f64) -> String {
    let total = afferent + efferent;

    if total > 15 {
        "Highly Coupled".to_string()
    } else if total <= 2 {
        "Isolated".to_string()
    } else if instability < 0.3 && afferent > efferent {
        "Stable Core".to_string()
    } else if instability > 0.7 && efferent > afferent {
        "Leaf Module".to_string()
    } else {
        "Utility Module".to_string()
    }
}
