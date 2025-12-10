//! Dependencies page (Page 2) - Call graph, blast radius, and coupling visualization.
//!
//! This page displays:
//! - Function-level dependency metrics (upstream/downstream counts)
//! - File-level coupling metrics with visual indicators (spec 203)
//! - Coupling classification badges with semantic coloring
//! - Instability progress bars with color gradients
//! - Lists of dependents and dependencies

use super::components::{add_label_value, add_section_header};
use crate::priority::UnifiedDebtItem;
use crate::tui::results::app::ResultsApp;
use crate::tui::theme::Theme;
use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
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

    let paragraph = Paragraph::new(lines)
        .block(Block::default().borders(Borders::NONE))
        .wrap(Wrap { trim: false });

    frame.render_widget(paragraph, area);
}

/// Render file-level coupling metrics section with enhanced visualization (spec 201, 203).
///
/// Looks up file-level metrics from the analysis and displays:
/// - Coupling classification badge with color coding
/// - Afferent coupling (Ca) - files that depend on this file
/// - Efferent coupling (Ce) - files this file depends on
/// - Instability progress bar with color gradient
/// - Lists of top dependents and dependencies
fn render_file_coupling_section(
    lines: &mut Vec<Line<'static>>,
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

    lines.push(Line::from(""));
    add_section_header(lines, "coupling profile", theme);

    // Classification badge with color (spec 203)
    let classification = derive_coupling_classification(
        metrics.afferent_coupling,
        metrics.efferent_coupling,
        metrics.instability,
    );
    render_classification_badge(lines, &classification, theme, width);

    // Afferent coupling (Ca)
    add_label_value(
        lines,
        "afferent (ca)",
        metrics.afferent_coupling.to_string(),
        theme,
        width,
    );

    // Efferent coupling (Ce)
    add_label_value(
        lines,
        "efferent (ce)",
        metrics.efferent_coupling.to_string(),
        theme,
        width,
    );

    // Instability with progress bar (spec 203)
    render_instability_bar(lines, metrics.instability, theme, width);

    // Add context note for extreme values
    if total_coupling > 15 {
        lines.push(Line::from(vec![
            Span::styled("Warning: ", Style::default().fg(ratatui::style::Color::Red)),
            Span::styled(
                "High coupling may indicate architectural issues.",
                Style::default().fg(theme.muted),
            ),
        ]));
    } else if metrics.instability < 0.1 && metrics.afferent_coupling > 0 {
        lines.push(Line::from(vec![
            Span::styled("Note: ", Style::default().fg(theme.primary)),
            Span::styled(
                "Stable core - changes need careful review.",
                Style::default().fg(theme.muted),
            ),
        ]));
    } else if metrics.instability > 0.9 {
        lines.push(Line::from(vec![
            Span::styled("Note: ", Style::default().fg(theme.success)),
            Span::styled(
                "Unstable leaf - safe to refactor.",
                Style::default().fg(theme.muted),
            ),
        ]));
    }

    // Dependents list (who uses this) - spec 203
    render_dependency_list(
        lines,
        &metrics.dependents,
        "dependents (who uses this)",
        theme,
        width,
    );

    // Dependencies list (what this uses) - spec 203
    render_dependency_list(
        lines,
        &metrics.dependencies_list,
        "dependencies (what this uses)",
        theme,
        width,
    );
}

/// Render classification badge with semantic coloring (spec 203).
///
/// Displays classification as a colored badge like `[STABLE CORE]`
/// Uses the standard label-value column layout for consistency.
fn render_classification_badge(
    lines: &mut Vec<Line<'static>>,
    classification: &str,
    theme: &Theme,
    _width: u16,
) {
    const INDENT: usize = 2;
    const LABEL_WIDTH: usize = 24;
    const GAP: usize = 4;

    let badge_text = format!("[{}]", classification.to_uppercase());
    let badge_style = theme.coupling_badge_style(classification);

    let label_with_indent = format!("{}{}", " ".repeat(INDENT), "classification");
    let padded_label = format!("{:width$}", label_with_indent, width = LABEL_WIDTH);
    let gap = " ".repeat(GAP);

    lines.push(Line::from(vec![
        Span::raw(padded_label),
        Span::raw(gap),
        Span::styled(badge_text, badge_style),
    ]));
}

/// Render instability as a progress bar with color gradient (spec 203).
///
/// Format: `  instability              0.40 ████████░░░░░░░░░░░░`
/// Uses standard label-value column layout with progress bar appended.
/// Color: Green (0.0) -> Yellow (0.5) -> Red (1.0)
fn render_instability_bar(
    lines: &mut Vec<Line<'static>>,
    instability: f64,
    theme: &Theme,
    _width: u16,
) {
    const INDENT: usize = 2;
    const LABEL_WIDTH: usize = 24;
    const GAP: usize = 4;

    let label_with_indent = format!("{}{}", " ".repeat(INDENT), "instability");
    let padded_label = format!("{:width$}", label_with_indent, width = LABEL_WIDTH);
    let gap = " ".repeat(GAP);

    // Progress bar configuration
    let bar_width = 20;
    let filled = ((instability * bar_width as f64).round() as usize).min(bar_width);
    let empty = bar_width - filled;

    let bar_color = theme.instability_color(instability);
    let filled_bar: String = "█".repeat(filled);
    let empty_bar: String = "░".repeat(empty);

    lines.push(Line::from(vec![
        Span::raw(padded_label),
        Span::raw(gap),
        Span::styled(
            format!("{:.2} ", instability),
            Style::default().fg(theme.primary),
        ),
        Span::styled(filled_bar, Style::default().fg(bar_color)),
        Span::styled(empty_bar, Style::default().fg(theme.muted)),
    ]));
}

/// Render a dependency list section (spec 203).
///
/// Displays up to 5 items with a truncation indicator if more exist.
fn render_dependency_list(
    lines: &mut Vec<Line<'static>>,
    items: &[String],
    title: &str,
    theme: &Theme,
    _width: u16,
) {
    // Skip if empty
    if items.is_empty() {
        return;
    }

    lines.push(Line::from(""));
    add_section_header(lines, title, theme);

    let max_display = 5;
    for item in items.iter().take(max_display) {
        // Shorten path for display (show just filename or last component)
        let display_name = shorten_path(item);
        lines.push(Line::from(vec![Span::styled(
            format!("  {} {}", "\u{2022}", display_name), // bullet point
            Style::default().fg(theme.text),
        )]));
    }

    // Show truncation indicator
    if items.len() > max_display {
        lines.push(Line::from(vec![Span::styled(
            format!("    (+{} more)", items.len() - max_display),
            Style::default().fg(theme.muted),
        )]));
    }
}

/// Shorten a file path for display.
///
/// If the path contains a directory separator, show only the last component.
/// Otherwise, return the path as-is.
fn shorten_path(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
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
