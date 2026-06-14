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
    Frame,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};

/// Build all lines for the dependencies page (pure function).
///
/// This is public so text_extraction can reuse it for clipboard copy.
pub fn build_page_lines(item: &UnifiedDebtItem, theme: &Theme, width: u16) -> Vec<Line<'static>> {
    let mut lines = Vec::new();

    // Function-level Dependency Metrics section
    add_section_header(&mut lines, "function dependencies", theme);
    add_label_value(
        &mut lines,
        "upstream",
        item.upstream_dependencies.to_string(),
        theme,
        width,
    );
    add_label_value(
        &mut lines,
        "downstream",
        item.downstream_dependencies.to_string(),
        theme,
        width,
    );

    let blast_radius = item.upstream_dependencies + item.downstream_dependencies;
    add_label_value(
        &mut lines,
        "blast radius",
        blast_radius.to_string(),
        theme,
        width,
    );

    // Critical path indicator (simplified - based on high dependency count)
    let is_critical = item.upstream_dependencies > 5 || item.downstream_dependencies > 10;
    add_label_value(
        &mut lines,
        "critical",
        if is_critical { "Yes" } else { "No" }.to_string(),
        theme,
        width,
    );

    // File-level Coupling Metrics section (spec 201)
    build_file_coupling_section(&mut lines, item, theme, width);

    lines
}

/// Render dependencies page showing dependency metrics and blast radius
pub fn render(
    frame: &mut Frame,
    app: &ResultsApp,
    item: &UnifiedDebtItem,
    area: Rect,
    theme: &Theme,
) {
    let lines = build_page_lines(item, theme, area.width);

    let paragraph = Paragraph::new(lines)
        .block(Block::default().borders(Borders::NONE))
        .wrap(Wrap { trim: false })
        .scroll(app.detail_scroll_offset());

    frame.render_widget(paragraph, area);
}

/// Build function-level coupling metrics section with enhanced visualization.
///
/// For regular functions, displays function-level dependency data:
/// - Coupling classification badge with color coding
/// - Afferent coupling (Ca) - functions that call this function
/// - Efferent coupling (Ce) - functions this function calls
/// - Instability progress bar with color gradient
/// - Lists of callers (dependents) and callees (dependencies)
///
/// For god objects, aggregates file-level metrics from member functions.
fn build_file_coupling_section(
    lines: &mut Vec<Line<'static>>,
    item: &UnifiedDebtItem,
    theme: &Theme,
    width: u16,
) {
    // Use function-level dependency data from the item itself
    let afferent_coupling = item.upstream_dependencies;
    let efferent_coupling = item.downstream_dependencies;

    // Only show if we have meaningful coupling data
    let total_coupling = afferent_coupling + efferent_coupling;
    if total_coupling == 0 {
        return;
    }

    // Calculate instability: Ce / (Ca + Ce)
    let instability = if total_coupling > 0 {
        efferent_coupling as f64 / total_coupling as f64
    } else {
        0.0
    };

    lines.push(Line::from(""));
    add_section_header(lines, "coupling profile", theme);

    // Classification badge with color (spec 203)
    let classification =
        derive_coupling_classification(afferent_coupling, efferent_coupling, instability);
    render_classification_badge(lines, &classification, theme, width);

    // Afferent coupling (Ca) - functions that call this function
    add_label_value(
        lines,
        "afferent (ca)",
        afferent_coupling.to_string(),
        theme,
        width,
    );

    // Efferent coupling (Ce) - functions this function calls
    add_label_value(
        lines,
        "efferent (ce)",
        efferent_coupling.to_string(),
        theme,
        width,
    );

    // Instability with progress bar (spec 203)
    render_instability_bar(lines, instability, theme, width);

    // Add context note for extreme values
    if total_coupling > 15 {
        lines.push(Line::from(vec![
            Span::styled("Warning: ", Style::default().fg(ratatui::style::Color::Red)),
            Span::styled(
                "High coupling may indicate architectural issues.",
                Style::default().fg(theme.muted),
            ),
        ]));
    } else if instability < 0.1 && afferent_coupling > 0 {
        lines.push(Line::from(vec![
            Span::styled("Note: ", Style::default().fg(theme.primary)),
            Span::styled(
                "Stable core - changes need careful review.",
                Style::default().fg(theme.muted),
            ),
        ]));
    } else if instability > 0.9 {
        lines.push(Line::from(vec![
            Span::styled("Note: ", Style::default().fg(theme.success)),
            Span::styled(
                "Unstable leaf - safe to refactor.",
                Style::default().fg(theme.muted),
            ),
        ]));
    }

    // Dependents list (who calls this function)
    render_dependency_list(
        lines,
        &item.upstream_callers,
        "dependents (who calls this)",
        theme,
        width,
    );

    // Dependencies list (what this function calls)
    render_dependency_list(
        lines,
        &item.downstream_callees,
        "dependencies (what this calls)",
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
/// Displays every item. The detail view is scrollable, and callers/callees are
/// analysis context that should not be hidden behind a summary count.
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

    for item in items {
        // Shorten path for display (show just filename or last component)
        let display_name = shorten_path(item);
        lines.push(Line::from(vec![Span::styled(
            format!("  {} {}", "\u{2022}", display_name), // bullet point
            Style::default().fg(theme.text),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::priority::{
        ActionableRecommendation, DebtType, FunctionRole, ImpactMetrics, Location, UnifiedDebtItem,
        UnifiedScore,
    };
    use std::path::PathBuf;

    fn line_text(line: &Line<'_>) -> String {
        line.spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect::<Vec<_>>()
            .join("")
    }

    fn create_item_with_dependencies() -> UnifiedDebtItem {
        UnifiedDebtItem {
            location: Location {
                file: PathBuf::from("src/main.rs"),
                line: 10,
                function: "target".to_string(),
            },
            debt_type: DebtType::ComplexityHotspot {
                cyclomatic: 20,
                cognitive: 25,
            },
            unified_score: UnifiedScore {
                complexity_factor: 50.0,
                coverage_factor: 20.0,
                dependency_factor: 30.0,
                role_multiplier: 1.0,
                final_score: 50.0,
                base_score: None,
                exponential_factor: None,
                risk_boost: None,
                pre_adjustment_score: None,
                adjustment_applied: None,
                purity_factor: None,
                refactorability_factor: None,
                pattern_factor: None,
                debt_adjustment: None,
                pre_normalization_score: None,
                structural_multiplier: None,
                has_coverage_data: false,
                contextual_risk_multiplier: None,
                pre_contextual_score: None,
                debt_type_multiplier: None,
            },
            function_role: FunctionRole::Unknown,
            recommendation: ActionableRecommendation::default(),
            expected_impact: ImpactMetrics {
                coverage_improvement: 0.0,
                lines_reduction: 0,
                complexity_reduction: 0.0,
                risk_reduction: 0.0,
            },
            transitive_coverage: None,
            file_context: None,
            upstream_dependencies: 6,
            downstream_dependencies: 6,
            upstream_callers: (1..=6)
                .map(|n| format!("src/callers/caller{n}.rs:caller{n}"))
                .collect(),
            downstream_callees: (1..=6)
                .map(|n| format!("src/callees/callee{n}.rs:callee{n}"))
                .collect(),
            upstream_production_callers: vec![],
            upstream_test_callers: vec![],
            production_blast_radius: 0,
            nesting_depth: 1,
            function_length: 20,
            cyclomatic_complexity: 20,
            cognitive_complexity: 25,
            entropy_analysis: None,
            is_pure: None,
            purity_confidence: None,
            purity_level: None,
            god_object_indicators: None,
            tier: None,
            function_context: None,
            context_confidence: None,
            contextual_recommendation: None,
            pattern_analysis: None,
            context_multiplier: None,
            context_type: None,
            language_specific: None,
            detected_pattern: None,
            contextual_risk: None,
            file_line_count: None,
            responsibility_category: None,
            error_swallowing_count: None,
            error_swallowing_patterns: None,
            context_suggestion: None,
        }
    }

    #[test]
    fn dependencies_page_lists_all_callers_and_callees() {
        let item = create_item_with_dependencies();
        let theme = Theme::default();
        let text = build_page_lines(&item, &theme, 100)
            .iter()
            .map(line_text)
            .collect::<Vec<_>>()
            .join("\n");

        assert!(text.contains("caller6.rs:caller6"), "{text}");
        assert!(text.contains("callee6.rs:callee6"), "{text}");
        assert!(!text.contains("(+"), "{text}");
    }
}
