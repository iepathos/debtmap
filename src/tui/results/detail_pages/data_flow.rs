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
        add_section_header(&mut lines, "mutation analysis", theme);

        add_label_value(
            &mut lines,
            "total",
            mutation_info.total_mutations.to_string(),
            theme,
            area.width,
        );

        add_label_value(
            &mut lines,
            "mutations",
            mutation_info.live_mutations.len().to_string(),
            theme,
            area.width,
        );

        if !mutation_info.live_mutations.is_empty() {
            add_blank_line(&mut lines);
            add_section_header(&mut lines, "mutations", theme);
            for mutation in &mutation_info.live_mutations {
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

    // Escape Analysis Section
    if let Some(cfg_ctx) = data_flow.get_cfg_analysis_with_context(&func_id) {
        add_section_header(&mut lines, "escape analysis", theme);

        let escaping_count = cfg_ctx.analysis.escape_info.escaping_vars.len();
        add_label_value(
            &mut lines,
            "escaping",
            escaping_count.to_string(),
            theme,
            area.width,
        );

        // Display return dependencies with translated variable names
        let return_dep_names = data_flow.get_return_dependency_names(&func_id);
        if !return_dep_names.is_empty() {
            add_blank_line(&mut lines);
            add_section_header(&mut lines, "variables affecting return value", theme);
            for var_name in &return_dep_names {
                lines.push(Line::from(vec![
                    Span::raw("                        "), // Align to value column (24 chars)
                    Span::styled(var_name.clone(), Style::default().fg(theme.primary)),
                ]));
            }
        }

        // Display tainted variables (optional, for debugging)
        let tainted_names = data_flow.get_tainted_var_names(&func_id);
        if !tainted_names.is_empty() && tainted_names.len() < 10 {
            // Only show if reasonable number
            add_blank_line(&mut lines);
            add_section_header(&mut lines, "tainted variables", theme);
            for var_name in &tainted_names {
                lines.push(Line::from(vec![
                    Span::raw("                        "),
                    Span::styled(
                        format!("{} (affected by mutations)", var_name),
                        Style::default().fg(Color::DarkGray),
                    ),
                ]));
            }
        }

        add_blank_line(&mut lines);
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
