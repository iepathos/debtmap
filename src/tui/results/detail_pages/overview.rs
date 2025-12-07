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
    app: &ResultsApp,
    item: &UnifiedDebtItem,
    area: Rect,
    theme: &Theme,
) {
    let mut lines = Vec::new();

    // Location section
    add_section_header(&mut lines, "location", theme);
    add_label_value(
        &mut lines,
        "file",
        item.location.file.display().to_string(),
        theme,
        area.width,
    );
    add_label_value(
        &mut lines,
        "function",
        item.location.function.clone(),
        theme,
        area.width,
    );
    add_label_value(
        &mut lines,
        "line",
        item.location.line.to_string(),
        theme,
        area.width,
    );
    add_blank_line(&mut lines);

    // Get all items at this location
    let location_items = get_items_at_location(app, item);

    // Score section
    add_section_header(&mut lines, "score", theme);

    if location_items.len() > 1 {
        // Multiple debt types - show combined score
        let combined_score: f64 = location_items
            .iter()
            .map(|i| i.unified_score.final_score.value())
            .sum();
        let severity = calculate_severity(combined_score);
        let severity_color = severity_color(severity);

        lines.push(Line::from(vec![
            Span::raw("  combined  "),
            Span::styled(
                format!("{:.1}", combined_score),
                Style::default().fg(theme.primary),
            ),
            Span::raw("  ["),
            Span::styled(severity, Style::default().fg(severity_color)),
            Span::raw("]"),
        ]));
    } else {
        // Single debt type - show single score
        let severity = calculate_severity(item.unified_score.final_score.value());
        let severity_color = severity_color(severity);

        lines.push(Line::from(vec![
            Span::raw("  total  "),
            Span::styled(
                format!("{:.1}", item.unified_score.final_score.value()),
                Style::default().fg(theme.primary),
            ),
            Span::raw("  ["),
            Span::styled(severity, Style::default().fg(severity_color)),
            Span::raw("]"),
        ]));
    }
    add_blank_line(&mut lines);

    // Metrics section
    add_section_header(&mut lines, "metrics", theme);
    add_label_value(
        &mut lines,
        "cyclomatic",
        item.cyclomatic_complexity.to_string(),
        theme,
        area.width,
    );
    add_label_value(
        &mut lines,
        "cognitive",
        item.cognitive_complexity.to_string(),
        theme,
        area.width,
    );
    add_label_value(
        &mut lines,
        "nesting",
        item.nesting_depth.to_string(),
        theme,
        area.width,
    );
    add_label_value(
        &mut lines,
        "length",
        item.function_length.to_string(),
        theme,
        area.width,
    );
    add_blank_line(&mut lines);

    // Entropy section
    if item.entropy_details.is_some() || item.entropy_dampening_factor.is_some() {
        add_section_header(&mut lines, "entropy", theme);

        if let Some(ref entropy) = item.entropy_details {
            add_label_value(
                &mut lines,
                "entropy",
                format!("{:.3}", entropy.entropy_score),
                theme,
                area.width,
            );
            add_label_value(
                &mut lines,
                "repetition",
                format!("{:.3}", entropy.pattern_repetition),
                theme,
                area.width,
            );
            add_label_value(
                &mut lines,
                "dampening",
                format!("{:.3}x", entropy.dampening_factor),
                theme,
                area.width,
            );

            // Show original vs adjusted cognitive complexity
            // Note: Only cognitive is dampened, not cyclomatic (structural metric)
            if entropy.dampening_factor < 1.0 {
                lines.push(Line::from(vec![
                    Span::raw("  cognitive reduction  "),
                    Span::styled(
                        format!(
                            "{} → {}",
                            entropy.original_complexity, entropy.adjusted_cognitive
                        ),
                        Style::default().fg(theme.primary),
                    ),
                ]));
            }
        } else if let Some(dampening) = item.entropy_dampening_factor {
            add_label_value(
                &mut lines,
                "dampening",
                format!("{:.3}x", dampening),
                theme,
                area.width,
            );
        }

        add_blank_line(&mut lines);
    }

    // Coverage section
    add_section_header(&mut lines, "coverage", theme);
    if let Some(coverage) = item.transitive_coverage.as_ref().map(|c| c.direct) {
        lines.push(Line::from(vec![
            Span::raw("  coverage              "),
            Span::styled(
                format!("{:.1}%", coverage),
                Style::default().fg(coverage_color(coverage)),
            ),
        ]));
    } else {
        lines.push(Line::from(vec![
            Span::raw("  coverage              "),
            Span::styled("No data", Style::default().fg(theme.muted)),
        ]));
    }
    add_blank_line(&mut lines);

    // Recommendation section
    add_section_header(&mut lines, "recommendation", theme);
    add_label_value(
        &mut lines,
        "action",
        item.recommendation.primary_action.clone(),
        theme,
        area.width,
    );
    add_blank_line(&mut lines);

    add_label_value(
        &mut lines,
        "rationale",
        item.recommendation.rationale.clone(),
        theme,
        area.width,
    );
    add_blank_line(&mut lines);

    // Debt type section
    if location_items.len() > 1 {
        // Multiple debt types - show all (simplified, no detailed metrics)
        add_section_header(&mut lines, "debt types", theme);
        for (idx, debt_item) in location_items.iter().enumerate() {
            let debt_name = format_debt_type_name(&debt_item.debt_type);
            lines.push(Line::from(vec![
                Span::raw(format!("  {}. ", idx + 1)),
                Span::styled(debt_name, Style::default().fg(theme.secondary())),
            ]));
        }
    } else {
        // Single debt type - show as before
        add_section_header(&mut lines, "debt type", theme);
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(
                format_debt_type_name(&item.debt_type),
                Style::default().fg(theme.primary),
            ),
        ]));
    }

    let paragraph = Paragraph::new(lines)
        .block(Block::default().borders(Borders::NONE))
        .wrap(Wrap { trim: false });

    frame.render_widget(paragraph, area);
}

/// Get all debt items at the same location as the selected item
fn get_items_at_location<'a>(
    app: &'a ResultsApp,
    selected: &UnifiedDebtItem,
) -> Vec<&'a UnifiedDebtItem> {
    app.analysis()
        .items
        .iter()
        .filter(|item| {
            item.location.file == selected.location.file
                && item.location.function == selected.location.function
                && item.location.line == selected.location.line
        })
        .collect()
}

/// Format debt type as human-readable name
fn format_debt_type_name(debt_type: &crate::priority::DebtType) -> String {
    #[allow(unused_imports)]
    use crate::priority::score_types::Score0To100;
    use crate::priority::DebtType;
    match debt_type {
        DebtType::ComplexityHotspot { .. } => "High Complexity".to_string(),
        DebtType::TestingGap { .. } => "Testing Gap".to_string(),
        DebtType::DeadCode { .. } => "Dead Code".to_string(),
        DebtType::Duplication { .. } => "Duplication".to_string(),
        DebtType::Risk { .. } => "Risk".to_string(),
        DebtType::TestComplexityHotspot { .. } => "Test Complexity".to_string(),
        DebtType::TestTodo { .. } => "Test TODO".to_string(),
        DebtType::TestDuplication { .. } => "Test Duplication".to_string(),
        DebtType::ErrorSwallowing { .. } => "Error Swallowing".to_string(),
        DebtType::AllocationInefficiency { .. } => "Allocation Inefficiency".to_string(),
        DebtType::StringConcatenation { .. } => "String Concatenation".to_string(),
        DebtType::NestedLoops { .. } => "Nested Loops".to_string(),
        DebtType::BlockingIO { .. } => "Blocking I/O".to_string(),
        DebtType::SuboptimalDataStructure { .. } => "Suboptimal Data Structure".to_string(),
        DebtType::GodObject { .. } => "God Object".to_string(),
        DebtType::GodModule { .. } => "God Module".to_string(),
        DebtType::FeatureEnvy { .. } => "Feature Envy".to_string(),
        DebtType::PrimitiveObsession { .. } => "Primitive Obsession".to_string(),
        DebtType::MagicValues { .. } => "Magic Values".to_string(),
        DebtType::AssertionComplexity { .. } => "Assertion Complexity".to_string(),
        DebtType::FlakyTestPattern { .. } => "Flaky Test Pattern".to_string(),
        DebtType::AsyncMisuse { .. } => "Async Misuse".to_string(),
        DebtType::ResourceLeak { .. } => "Resource Leak".to_string(),
        DebtType::CollectionInefficiency { .. } => "Collection Inefficiency".to_string(),
        DebtType::ScatteredType { .. } => "Scattered Type".to_string(),
        DebtType::OrphanedFunctions { .. } => "Orphaned Functions".to_string(),
        DebtType::UtilitiesSprawl { .. } => "Utilities Sprawl".to_string(),
        // Default for legacy variants
        _ => "Other".to_string(),
    }
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
