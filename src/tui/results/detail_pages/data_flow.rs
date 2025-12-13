//! Data flow page (Page 5) - Data flow analysis details.

use super::components::{add_blank_line, add_label_value, add_section_header};
use crate::data_flow::{DataFlowGraph, PurityInfo};
use crate::priority::call_graph::FunctionId;
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

/// Format a reason with inline suggestion context
fn format_reason_with_suggestion(reason: &str) -> String {
    let reason_lower = reason.to_lowercase();
    if reason_lower.contains("i/o") || reason_lower.contains("print") {
        format!("{} (extract to caller)", reason)
    } else if reason_lower.contains("mutable") || reason_lower.contains("&mut") {
        format!("{} (consider &self instead)", reason)
    } else if reason_lower.contains("unsafe") {
        reason.to_string() // Can't easily fix unsafe
    } else if reason_lower.contains("external") {
        format!("{} (pass as parameter)", reason)
    } else {
        reason.to_string()
    }
}

/// Get actionable fix suggestion for almost-pure functions (1-2 issues)
fn get_fix_suggestion(reasons: &[String]) -> Option<&'static str> {
    if reasons.len() > 2 {
        return None; // Too many issues for a simple fix
    }

    let first_reason = reasons.first()?.to_lowercase();

    if first_reason.contains("i/o")
        || first_reason.contains("print")
        || first_reason.contains("log")
    {
        Some("Move logging to caller - function becomes pure")
    } else if first_reason.contains("time") || first_reason.contains("now") {
        Some("Pass time as parameter instead of calling now()")
    } else if first_reason.contains("random") || first_reason.contains("rand") {
        Some("Inject RNG as parameter for deterministic behavior")
    } else if first_reason.contains("mutable param") || first_reason.contains("&mut") {
        Some("Consider taking &self instead of &mut self")
    } else {
        None
    }
}

/// Render purity analysis section with actionable details. Returns true if anything was rendered.
fn render_purity_section(
    lines: &mut Vec<Line<'static>>,
    purity_info: &PurityInfo,
    theme: &Theme,
    width: u16,
) -> bool {
    add_section_header(lines, "purity analysis", theme);

    // Show pure status with issue count for impure functions
    let pure_display = if purity_info.is_pure {
        "Yes".to_string()
    } else {
        let issue_count = purity_info.impurity_reasons.len();
        if issue_count == 0 {
            "No".to_string()
        } else if issue_count == 1 {
            "No (1 issue)".to_string()
        } else {
            format!("No ({} issues)", issue_count)
        }
    };

    add_label_value(lines, "pure", pure_display, theme, width);

    add_label_value(
        lines,
        "confidence",
        format!("{:.1}%", purity_info.confidence * 100.0),
        theme,
        width,
    );

    // Show impurity reasons with inline suggestions
    if !purity_info.impurity_reasons.is_empty() {
        let formatted_reasons = purity_info
            .impurity_reasons
            .iter()
            .map(|r| format_reason_with_suggestion(r))
            .collect::<Vec<_>>()
            .join("; ");

        add_label_value(lines, "reasons", formatted_reasons, theme, width);

        // Show actionable fix suggestion for almost-pure functions (1-2 issues)
        if purity_info.impurity_reasons.len() <= 2 {
            if let Some(suggestion) = get_fix_suggestion(&purity_info.impurity_reasons) {
                add_label_value(lines, "fix", suggestion.to_string(), theme, width);
            }
        }
    }

    add_blank_line(lines);
    true
}

/// Render data flow page showing data flow analysis
pub fn render(
    frame: &mut Frame,
    _app: &ResultsApp,
    item: &UnifiedDebtItem,
    data_flow: &DataFlowGraph,
    area: Rect,
    theme: &Theme,
) {
    let func_id = FunctionId::new(
        item.location.file.clone(),
        item.location.function.clone(),
        item.location.line,
    );

    let mut lines = Vec::new();

    // Purity Analysis Section (moved from patterns page - conceptually belongs here)
    if let Some(purity_info) = data_flow.get_purity_info(&func_id) {
        render_purity_section(&mut lines, purity_info, theme, area.width);
    }

    // Mutation Analysis Section (spec 257: binary signals)
    if let Some(mutation_info) = data_flow.get_mutation_info(&func_id) {
        add_section_header(&mut lines, "mutation analysis", theme);

        add_label_value(
            &mut lines,
            "has mutations",
            if mutation_info.has_mutations {
                "yes"
            } else {
                "no"
            }
            .to_string(),
            theme,
            area.width,
        );

        if !mutation_info.detected_mutations.is_empty() {
            add_blank_line(&mut lines);
            add_section_header(&mut lines, "detected mutations (best-effort)", theme);
            for mutation in &mutation_info.detected_mutations {
                lines.push(Line::from(vec![
                    Span::raw("                        "), // Align to value column (24 chars)
                    Span::styled(mutation.clone(), Style::default().fg(Color::Yellow)),
                ]));
            }
        }

        add_blank_line(&mut lines);
    }

    // I/O Operations Section
    if let Some(io_ops) = data_flow.get_io_operations(&func_id) {
        if !io_ops.is_empty() {
            add_section_header(&mut lines, "i/o operations", theme);

            for op in io_ops {
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled(
                        format!(
                            "{} at line {} (variables: {})",
                            op.operation_type,
                            op.line,
                            op.variables.join(", ")
                        ),
                        Style::default().fg(Color::Yellow),
                    ),
                ]));
            }

            add_blank_line(&mut lines);
        }
    }

    // Escape/taint analysis removed - not providing actionable debt signals

    // If no data available
    if lines.is_empty() {
        lines.push(Line::from(vec![Span::styled(
            "No data flow analysis data available for this function.",
            Style::default().fg(theme.muted),
        )]));
    }

    let paragraph = Paragraph::new(lines)
        .block(Block::default().borders(Borders::NONE))
        .wrap(Wrap { trim: false });

    frame.render_widget(paragraph, area);
}
