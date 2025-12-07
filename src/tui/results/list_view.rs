//! List view rendering.

use super::app::ResultsApp;
use super::grouping;
use crate::priority::{DebtType, UnifiedDebtItem};
use crate::tui::theme::Theme;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

/// Render main list view
pub fn render(frame: &mut Frame, app: &ResultsApp) {
    let theme = Theme::default();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(0),    // List
            Constraint::Length(2), // Footer
        ])
        .split(frame.size());

    // Render header
    render_header(frame, app, chunks[0], &theme);

    // Render list
    render_list(frame, app, chunks[1], &theme);

    // Render footer
    render_footer(frame, app, chunks[2], &theme);
}

/// Render list view with search overlay
pub fn render_with_search(frame: &mut Frame, app: &ResultsApp) {
    // First render the normal list
    render(frame, app);

    // Then overlay search input
    let theme = Theme::default();
    let area = frame.size();

    // Create search box in center
    let search_area = Rect {
        x: area.width / 4,
        y: 2,
        width: area.width / 2,
        height: 3,
    };

    let search_text = format!("Search: {}", app.search().query());
    let search_widget = Paragraph::new(search_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Search (Esc to cancel, Enter to apply)")
                .border_style(Style::default().fg(theme.accent())),
        )
        .style(Style::default().fg(theme.primary));

    frame.render_widget(search_widget, search_area);
}

/// Render list view with sort menu overlay
pub fn render_with_sort_menu(frame: &mut Frame, app: &ResultsApp) {
    render(frame, app);

    let theme = Theme::default();
    let area = frame.size();

    // Create sort menu in center
    let menu_area = Rect {
        x: area.width / 3,
        y: area.height / 4,
        width: area.width / 3,
        height: 11,
    };

    let sort_options = super::sort::SortCriteria::all();
    let current_sort = app.sort_by();

    let items: Vec<ListItem> = sort_options
        .iter()
        .enumerate()
        .map(|(i, criteria)| {
            let prefix = if *criteria == current_sort {
                "▸ "
            } else {
                "  "
            };
            let text = format!("{}. {}{}", i + 1, prefix, criteria.display_name());
            ListItem::new(text).style(if *criteria == current_sort {
                Style::default()
                    .fg(theme.accent())
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.primary)
            })
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Sort By (press number, Esc to cancel)")
            .border_style(Style::default().fg(theme.accent())),
    );

    frame.render_widget(list, menu_area);
}

/// Render list view with filter menu overlay
pub fn render_with_filter_menu(frame: &mut Frame, app: &ResultsApp) {
    render(frame, app);

    let theme = Theme::default();
    let area = frame.size();

    // Create filter menu in center
    let menu_area = Rect {
        x: area.width / 4,
        y: area.height / 6,
        width: area.width / 2,
        height: 16,
    };

    let mut lines = vec![
        Line::from("Severity Filters:"),
        Line::from("  1. Critical"),
        Line::from("  2. High"),
        Line::from("  3. Medium"),
        Line::from("  4. Low"),
        Line::from(""),
        Line::from("Coverage Filters:"),
        Line::from("  n. No Coverage"),
        Line::from("  l. Low (0-30%)"),
        Line::from("  m. Medium (30-70%)"),
        Line::from("  h. High (70-100%)"),
        Line::from(""),
        Line::from("  c. Clear all filters"),
    ];

    // Show active filters
    if !app.filters().is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Active filters:",
            Style::default().fg(theme.accent()),
        )));
        for filter in app.filters() {
            lines.push(Line::from(format!("  • {}", filter.display_name())));
        }
    }

    let paragraph = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Filters (press key, Esc to cancel)")
            .border_style(Style::default().fg(theme.accent())),
    );

    frame.render_widget(paragraph, menu_area);
}

/// Render header with summary metrics (minimal lowercase style)
fn render_header(frame: &mut Frame, app: &ResultsApp, area: Rect, theme: &Theme) {
    let analysis = app.analysis();
    let count_display = app.count_display();

    let header_text = vec![
        Line::from(vec![
            Span::raw("debtmap results"),
            Span::raw("  "),
            Span::styled(
                count_display.to_string(),
                Style::default().fg(theme.primary),
            ),
            Span::raw("  │  "),
            Span::raw("score "),
            Span::styled(
                format!("{:.0}", analysis.total_debt_score),
                Style::default().fg(theme.secondary()),
            ),
            Span::raw("  │  "),
            Span::styled(
                format!("{:.2}/1K loc", analysis.debt_density),
                Style::default().fg(theme.muted),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                format!("sort {}", app.sort_by().display_name()),
                Style::default().fg(theme.muted),
            ),
            Span::raw("  │  "),
            Span::styled(
                format!("filters {}", app.filters().len()),
                Style::default().fg(theme.muted),
            ),
            Span::raw("  │  "),
            Span::styled(
                format!("grouping {}", if app.is_grouped() { "on" } else { "off" }),
                Style::default().fg(theme.muted),
            ),
        ]),
    ];

    let header = Paragraph::new(header_text)
        .block(Block::default().borders(Borders::BOTTOM))
        .style(Style::default());

    frame.render_widget(header, area);
}

/// Render list of items
fn render_list(frame: &mut Frame, app: &ResultsApp, area: Rect, theme: &Theme) {
    let items: Vec<ListItem> = if app.is_grouped() {
        render_grouped_list(app, area, theme)
    } else {
        render_ungrouped_list(app, area, theme)
    };

    if items.is_empty() {
        let empty_text = if app.filters().is_empty() && app.search().query().is_empty() {
            "No debt items found"
        } else {
            "No items match current filters/search"
        };

        let empty = Paragraph::new(empty_text)
            .style(Style::default().fg(theme.muted))
            .block(Block::default().borders(Borders::NONE));

        frame.render_widget(empty, area);
    } else {
        let list = List::new(items).block(Block::default().borders(Borders::NONE));
        frame.render_widget(list, area);
    }
}

/// Render ungrouped list (original behavior)
fn render_ungrouped_list(app: &ResultsApp, area: Rect, theme: &Theme) -> Vec<ListItem<'static>> {
    app.filtered_items()
        .enumerate()
        .skip(app.scroll_offset())
        .take(area.height as usize)
        .map(|(idx, item)| {
            let is_selected = idx == app.selected_index();
            format_list_item(item, idx, is_selected, theme)
        })
        .collect()
}

/// Render grouped list by location
fn render_grouped_list(app: &ResultsApp, area: Rect, theme: &Theme) -> Vec<ListItem<'static>> {
    let groups = grouping::group_by_location(app.filtered_items(), app.sort_by());

    let mut list_items = Vec::new();

    for (display_index, group) in groups.iter().skip(app.scroll_offset()).enumerate() {
        if list_items.len() >= area.height as usize {
            break;
        }

        let is_selected = (display_index + app.scroll_offset()) == app.selected_index();
        list_items.push(format_grouped_item(
            group,
            display_index + app.scroll_offset(),
            is_selected,
            theme,
        ));
    }

    list_items
}

/// Format a grouped item with badge and aggregated metrics
fn format_grouped_item(
    group: &grouping::LocationGroup,
    index: usize,
    is_selected: bool,
    theme: &Theme,
) -> ListItem<'static> {
    let severity_color = severity_color(group.max_severity);
    let indicator = if is_selected { "▸ " } else { "  " };

    let file_name = group
        .location
        .file
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown");

    // Badge for multiple issues
    let badge = if group.items.len() > 1 {
        format!(" [{}]", group.items.len())
    } else {
        String::new()
    };

    // Aggregated metrics
    let metrics = grouping::aggregate_metrics(group);
    let coverage_str = metrics
        .coverage
        .map(|c| format!("{:.0}%", c.direct))
        .unwrap_or_else(|| "N/A".to_string());

    let mut metric_parts = vec![format!("Cov:{}", coverage_str)];

    if metrics.cognitive_complexity > 0 {
        metric_parts.push(format!("Cog:{}", metrics.cognitive_complexity));
    }
    if metrics.nesting_depth > 0 {
        metric_parts.push(format!("Nest:{}", metrics.nesting_depth));
    }
    if metrics.function_length > 0 {
        // Changed from "Len:" to "LOC:" for consistency (spec 207)
        metric_parts.push(format!("LOC:{}", metrics.function_length));
    }

    // Single line: indicator, rank, severity, score, location, badge, metrics
    let line = Line::from(vec![
        Span::styled(indicator, Style::default().fg(theme.accent())),
        Span::styled(
            format!("#{:<4}", index + 1),
            Style::default().fg(theme.muted),
        ),
        Span::styled(
            format!("{:<8}", group.max_severity),
            Style::default().fg(severity_color),
        ),
        Span::styled(
            format!("{:<7.1}", group.combined_score),
            Style::default().fg(theme.primary),
        ),
        Span::raw("  "),
        Span::styled(
            format!("{}::{}", file_name, group.location.function),
            Style::default().fg(theme.secondary()),
        ),
        Span::styled(badge, Style::default().fg(theme.muted)),
        Span::raw("  "),
        Span::styled(
            format!("({})", metric_parts.join(" ")),
            Style::default().fg(theme.muted),
        ),
    ]);

    let style = if is_selected {
        Style::default().bg(Color::DarkGray)
    } else {
        Style::default()
    };

    ListItem::new(line).style(style)
}

/// Format complexity metric for list view display.
///
/// Uses cognitive complexity as the primary metric since it better
/// represents mental burden than cyclomatic complexity.
///
/// When entropy-adjusted complexity is available (from spec 214):
/// - Shows adjustment if reduction > 5%: "Cog:22→14"
/// - Otherwise shows raw value: "Cog:22"
///
/// # Examples
/// ```
/// // With entropy adjustment
/// let item = create_item_with_entropy(22, 14);
/// assert_eq!(format_complexity_metric(&item), "Cog:22→14");
///
/// // Without entropy or negligible adjustment
/// let item = create_item_without_entropy(22);
/// assert_eq!(format_complexity_metric(&item), "Cog:22");
/// ```
fn format_complexity_metric(item: &UnifiedDebtItem) -> String {
    // Check if entropy-adjusted cognitive complexity is available (spec 214)
    if let Some(adjusted_cog) = item.entropy_adjusted_cognitive {
        let raw_cog = item.cognitive_complexity;

        // Show adjustment if there's a meaningful difference (>5%)
        let diff_pct = if raw_cog > 0 {
            ((raw_cog as f64 - adjusted_cog as f64) / raw_cog as f64).abs()
        } else {
            0.0
        };

        if diff_pct > 0.05 {
            format!("Cog:{}→{}", raw_cog, adjusted_cog)
        } else {
            // Negligible adjustment, just show raw
            format!("Cog:{}", raw_cog)
        }
    } else {
        // No entropy data, show raw cognitive complexity
        format!("Cog:{}", item.cognitive_complexity)
    }
}

/// Format a single list item
fn format_list_item(
    item: &UnifiedDebtItem,
    index: usize,
    is_selected: bool,
    theme: &Theme,
) -> ListItem<'static> {
    let severity = calculate_severity(item.unified_score.final_score.value());
    let severity_color = severity_color(severity);

    let indicator = if is_selected { "▸ " } else { "  " };

    let file_name = item
        .location
        .file
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown");

    // Check if this is a god object - format differently (spec 207)
    let (location_str, metrics_str) = match &item.debt_type {
        DebtType::GodObject {
            methods,
            responsibilities,
            ..
        } => {
            let loc = item.file_line_count.unwrap_or(item.function_length);
            (
                format!("{} (God Object)", file_name),
                format!("(LOC:{} Resp:{} Fns:{})", loc, responsibilities, methods),
            )
        }
        DebtType::GodModule { functions, .. } => {
            let loc = item.file_line_count.unwrap_or(item.function_length);
            (
                format!("{} (God Module)", file_name),
                format!("(LOC:{} Fns:{})", loc, functions),
            )
        }
        _ => {
            // Regular function item
            let coverage_str = item
                .transitive_coverage
                .as_ref()
                .map(|c| format!("{:.0}%", c.direct))
                .unwrap_or_else(|| "N/A".to_string());

            // Format complexity with entropy adjustment if available
            let complexity_str = format_complexity_metric(item);

            let mut metric_parts = vec![format!("Cov:{}", coverage_str), complexity_str];

            // Add LOC for function length (spec 207: changed from Len: to LOC:)
            if item.function_length > 0 {
                metric_parts.push(format!("LOC:{}", item.function_length));
            }

            (
                format!("{}::{}", file_name, item.location.function),
                format!("({})", metric_parts.join(" ")),
            )
        }
    };

    let line = Line::from(vec![
        Span::styled(indicator, Style::default().fg(theme.accent())),
        Span::styled(
            format!("#{:<4}", index + 1),
            Style::default().fg(theme.muted),
        ),
        Span::styled(
            format!("{:<8}", severity),
            Style::default().fg(severity_color),
        ),
        Span::styled(
            format!("{:<7.1}", item.unified_score.final_score.value()),
            Style::default().fg(theme.primary),
        ),
        Span::raw("  "),
        Span::styled(location_str, Style::default().fg(theme.secondary())),
        Span::raw("  "),
        Span::styled(metrics_str, Style::default().fg(theme.muted)),
    ]);

    let style = if is_selected {
        Style::default().bg(Color::DarkGray)
    } else {
        Style::default()
    };

    ListItem::new(line).style(style)
}

/// Render footer with navigation hints
fn render_footer(frame: &mut Frame, app: &ResultsApp, area: Rect, theme: &Theme) {
    let position_text = if app.item_count() > 0 {
        format!("{}/{} items", app.selected_index() + 1, app.item_count())
    } else {
        "0 items".to_string()
    };

    let footer_text = Line::from(vec![
        Span::styled(position_text, Style::default().fg(theme.muted)),
        Span::raw("  |  "),
        Span::styled("↑↓/jk", Style::default().fg(theme.accent())),
        Span::raw(":Nav  "),
        Span::styled("G", Style::default().fg(theme.accent())),
        Span::raw(":Group  "),
        Span::styled("/", Style::default().fg(theme.accent())),
        Span::raw(":Search  "),
        Span::styled("s", Style::default().fg(theme.accent())),
        Span::raw(":Sort  "),
        Span::styled("f", Style::default().fg(theme.accent())),
        Span::raw(":Filter  "),
        Span::styled("?", Style::default().fg(theme.accent())),
        Span::raw(":Help  "),
        Span::styled("q", Style::default().fg(theme.accent())),
        Span::raw(":Quit"),
    ]);

    let footer = Paragraph::new(footer_text)
        .block(Block::default().borders(Borders::TOP))
        .style(Style::default());

    frame.render_widget(footer, area);
}

/// Calculate severity level from score
fn calculate_severity(score: f64) -> &'static str {
    if score >= 100.0 {
        "critical"
    } else if score >= 50.0 {
        "high"
    } else if score >= 10.0 {
        "medium"
    } else {
        "low"
    }
}

/// Get color for severity level
fn severity_color(severity: &str) -> Color {
    match severity {
        "critical" => Color::Red,
        "high" => Color::LightRed,
        "medium" => Color::Yellow,
        "low" => Color::Green,
        _ => Color::White,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::priority::score_types::Score0To100;
    use crate::priority::{
        ActionableRecommendation, DebtType, ImpactMetrics, Location, UnifiedScore,
    };
    use std::path::PathBuf;

    fn create_test_item(
        cognitive_complexity: u32,
        entropy_adjusted: Option<u32>,
    ) -> UnifiedDebtItem {
        UnifiedDebtItem {
            location: Location {
                file: PathBuf::from("test.rs"),
                function: "test_function".to_string(),
                line: 1,
            },
            debt_type: DebtType::ComplexityHotspot {
                cyclomatic: 5,
                cognitive: cognitive_complexity,
                adjusted_cyclomatic: None,
            },
            unified_score: UnifiedScore {
                complexity_factor: 5.0,
                coverage_factor: 5.0,
                dependency_factor: 5.0,
                role_multiplier: 1.0,
                final_score: Score0To100::new(50.0),
                base_score: None,
                exponential_factor: None,
                risk_boost: None,
                pre_adjustment_score: None,
                adjustment_applied: None,
                purity_factor: None,
                refactorability_factor: None,
                pattern_factor: None,
            },
            function_role: crate::priority::semantic_classifier::FunctionRole::Unknown,
            recommendation: ActionableRecommendation {
                primary_action: "Test action".to_string(),
                rationale: "Test rationale".to_string(),
                implementation_steps: vec![],
                related_items: vec![],
                steps: None,
                estimated_effort_hours: None,
            },
            expected_impact: ImpactMetrics {
                coverage_improvement: 0.2,
                lines_reduction: 10,
                complexity_reduction: 5.0,
                risk_reduction: 0.1,
            },
            transitive_coverage: None,
            upstream_dependencies: 0,
            downstream_dependencies: 0,
            upstream_callers: vec![],
            downstream_callees: vec![],
            nesting_depth: 1,
            function_length: 10,
            cyclomatic_complexity: 5,
            cognitive_complexity,
            entropy_details: None,
            entropy_adjusted_cyclomatic: None,
            entropy_adjusted_cognitive: entropy_adjusted,
            entropy_dampening_factor: None,
            is_pure: None,
            purity_confidence: None,
            purity_level: None,
            god_object_indicators: None,
            tier: None,
            function_context: None,
            context_confidence: None,
            contextual_recommendation: None,
            pattern_analysis: None,
            file_context: None,
            context_multiplier: None,
            context_type: None,
            language_specific: None,
            detected_pattern: None,
            contextual_risk: None,
            file_line_count: None,
        }
    }

    #[test]
    fn test_format_complexity_no_entropy() {
        let item = create_test_item(15, None);
        assert_eq!(format_complexity_metric(&item), "Cog:15");
    }

    #[test]
    fn test_format_complexity_with_meaningful_adjustment() {
        let item = create_test_item(22, Some(14));
        assert_eq!(format_complexity_metric(&item), "Cog:22→14");
    }

    #[test]
    fn test_format_complexity_negligible_adjustment() {
        // 5% difference = 1 point at complexity 20
        let item = create_test_item(20, Some(19));
        // Less than 5% difference, should show raw only
        assert_eq!(format_complexity_metric(&item), "Cog:20");
    }

    #[test]
    fn test_format_complexity_zero_raw() {
        // Edge case: zero cognitive complexity
        let item = create_test_item(0, Some(0));
        assert_eq!(format_complexity_metric(&item), "Cog:0");
    }

    #[test]
    fn test_format_complexity_exactly_5_percent() {
        // Exactly 5% should not show arrow (using > 0.05)
        let item = create_test_item(100, Some(95));
        assert_eq!(format_complexity_metric(&item), "Cog:100");
    }

    #[test]
    fn test_format_complexity_just_over_5_percent() {
        // Just over 5% should show arrow
        let item = create_test_item(100, Some(94));
        assert_eq!(format_complexity_metric(&item), "Cog:100→94");
    }
}
