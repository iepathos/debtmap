//! Overview page (Page 1) - Core metrics and recommendation.

use super::components::{add_blank_line, add_label_value, add_section_header};
use crate::priority::classification::Severity;
use crate::priority::{DebtType, UnifiedDebtItem};
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
        let severity = Severity::from_score_100(combined_score)
            .as_str()
            .to_lowercase();
        let severity_color = severity_color(&severity);

        lines.push(Line::from(vec![
            Span::raw("  combined              "),
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
        let severity = Severity::from_score_100(item.unified_score.final_score.value())
            .as_str()
            .to_lowercase();
        let severity_color = severity_color(&severity);

        lines.push(Line::from(vec![
            Span::raw("  total                 "),
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

    // For god objects, show structural metrics first (spec 253)
    // Use detection_type to customize labels and display
    if let DebtType::GodObject {
        methods,
        fields,
        responsibilities,
        lines: debt_lines,
        ..
    } = &item.debt_type
    {
        // Determine detection type from indicators
        let detection_type = item
            .god_object_indicators
            .as_ref()
            .map(|i| &i.detection_type);

        // Customize header and labels based on detection type
        let header = match detection_type {
            Some(crate::organization::DetectionType::GodClass) => "god object structure",
            Some(crate::organization::DetectionType::GodFile) => "god file structure",
            Some(crate::organization::DetectionType::GodModule) => "god module structure",
            None => "god object structure",
        };
        add_section_header(&mut lines, header, theme);

        // Use appropriate label for methods/functions
        let method_label = match detection_type {
            Some(crate::organization::DetectionType::GodClass) => "methods",
            _ => "functions",
        };
        add_label_value(
            &mut lines,
            method_label,
            methods.to_string(),
            theme,
            area.width,
        );

        // Show fields only if present (Some for GodClass, None for GodFile/GodModule)
        if let Some(field_count) = fields {
            add_label_value(
                &mut lines,
                "fields",
                field_count.to_string(),
                theme,
                area.width,
            );
        }

        add_label_value(
            &mut lines,
            "responsibilities",
            responsibilities.to_string(),
            theme,
            area.width,
        );

        // Use debt_lines for LOC (spec 253 adds this field)
        add_label_value(&mut lines, "loc", debt_lines.to_string(), theme, area.width);
        add_blank_line(&mut lines);
    }

    // Complexity metrics section
    // For god objects, use "accumulated complexity" to clarify that metrics
    // are aggregated across all functions (cyclomatic/cognitive are summed,
    // nesting is max). Regular functions show "complexity" for single-function metrics.
    let complexity_header = if matches!(item.debt_type, DebtType::GodObject { .. }) {
        "accumulated complexity"
    } else {
        "complexity"
    };
    add_section_header(&mut lines, complexity_header, theme);
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

    // For non-god objects, show function LOC
    if !matches!(item.debt_type, DebtType::GodObject { .. }) {
        add_label_value(
            &mut lines,
            "loc",
            item.function_length.to_string(),
            theme,
            area.width,
        );
    }
    add_blank_line(&mut lines);

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
        for debt_item in location_items.iter() {
            let debt_name = format_debt_type_name(&debt_item.debt_type);
            lines.push(Line::from(vec![
                Span::raw("  "),
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
                Style::default().fg(theme.secondary()),
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
