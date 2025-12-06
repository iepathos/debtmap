//! Data flow page (Page 5) - Data flow analysis details.

use super::components::{add_blank_line, add_label_value, add_section_header};
use crate::data_flow::DataFlowGraph;
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

    // Mutation Analysis Section
    if let Some(mutation_info) = data_flow.get_mutation_info(&func_id) {
        add_section_header(&mut lines, "MUTATION ANALYSIS", theme);

        add_label_value(
            &mut lines,
            "Total Mutations",
            mutation_info.total_mutations.to_string(),
            theme,
        );

        add_label_value(
            &mut lines,
            "Live Mutations",
            mutation_info.live_mutations.len().to_string(),
            theme,
        );

        add_label_value(
            &mut lines,
            "Dead Stores",
            mutation_info.dead_stores.len().to_string(),
            theme,
        );

        if !mutation_info.live_mutations.is_empty() {
            add_blank_line(&mut lines);
            lines.push(Line::from(vec![Span::styled(
                "  Live Mutations:",
                Style::default().fg(theme.secondary()),
            )]));
            for mutation in &mutation_info.live_mutations {
                lines.push(Line::from(vec![
                    Span::raw("    • "),
                    Span::styled(mutation.clone(), Style::default().fg(Color::Yellow)),
                ]));
            }
        }

        if !mutation_info.dead_stores.is_empty() {
            add_blank_line(&mut lines);
            lines.push(Line::from(vec![Span::styled(
                "  Dead Stores:",
                Style::default().fg(theme.secondary()),
            )]));
            for dead in &mutation_info.dead_stores {
                lines.push(Line::from(vec![
                    Span::raw("    • "),
                    Span::styled(
                        format!("{} (never read)", dead),
                        Style::default().fg(theme.muted),
                    ),
                ]));
            }
        }

        add_blank_line(&mut lines);
    }

    // I/O Operations Section
    if let Some(io_ops) = data_flow.get_io_operations(&func_id) {
        if !io_ops.is_empty() {
            add_section_header(&mut lines, "I/O OPERATIONS", theme);

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

    // Escape Analysis Section
    if let Some(cfg_analysis) = data_flow.get_cfg_analysis(&func_id) {
        add_section_header(&mut lines, "ESCAPE ANALYSIS", theme);

        let escaping_count = cfg_analysis.escape_info.escaping_vars.len();
        add_label_value(
            &mut lines,
            "Escaping Variables",
            escaping_count.to_string(),
            theme,
        );

        if escaping_count > 0 {
            add_blank_line(&mut lines);
            lines.push(Line::from(vec![Span::styled(
                "  Variables affecting return value:",
                Style::default().fg(theme.secondary()),
            )]));
            for var in &cfg_analysis.escape_info.return_dependencies {
                lines.push(Line::from(vec![
                    Span::raw("    • "),
                    Span::styled(format!("{:?}", var), Style::default().fg(theme.primary)),
                ]));
            }
        }

        add_blank_line(&mut lines);
    }

    // Purity Analysis Section
    if let Some(purity_info) = data_flow.get_purity_info(&func_id) {
        add_section_header(&mut lines, "PURITY ANALYSIS", theme);

        add_label_value(
            &mut lines,
            "Is Pure",
            if purity_info.is_pure { "Yes" } else { "No" }.to_string(),
            theme,
        );

        add_label_value(
            &mut lines,
            "Confidence",
            format!("{:.1}%", purity_info.confidence * 100.0),
            theme,
        );

        if !purity_info.impurity_reasons.is_empty() {
            add_blank_line(&mut lines);
            lines.push(Line::from(vec![Span::styled(
                "  Impurity Reasons:",
                Style::default().fg(theme.secondary()),
            )]));
            for reason in &purity_info.impurity_reasons {
                lines.push(Line::from(vec![
                    Span::raw("    • "),
                    Span::styled(reason.clone(), Style::default().fg(theme.muted)),
                ]));
            }
        }
    }

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
