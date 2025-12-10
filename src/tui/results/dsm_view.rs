//! Dependency Structure Matrix (DSM) View (Spec 205)
//!
//! Displays a matrix visualization of module dependencies:
//! - Rows and columns represent modules
//! - Cells show dependencies (row depends on column)
//! - Cycles are highlighted (cells above diagonal)
//! - Metrics summary displayed in header

use super::app::ResultsApp;
use crate::analysis::dsm::DependencyMatrix;
use crate::output::unified::{convert_to_unified_format, UnifiedDebtItemOutput};
use crate::tui::theme::Theme;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

/// Render the DSM view
pub fn render(frame: &mut Frame, app: &ResultsApp) {
    let theme = Theme::default();
    let area = frame.size();

    // Build the DSM from file items
    let matrix = build_dsm(app);

    // Split into header and matrix areas
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(8), // Header with metrics
            Constraint::Min(1),    // Matrix
            Constraint::Length(3), // Footer with legend
        ])
        .split(area);

    render_header(frame, &matrix, chunks[0], &theme);
    render_matrix(frame, app, &matrix, chunks[1], &theme);
    render_footer(frame, chunks[2], &theme);
}

/// Build DSM from the app's analysis data
fn build_dsm(app: &ResultsApp) -> DependencyMatrix {
    let unified = convert_to_unified_format(app.analysis(), false);

    let file_items: Vec<_> = unified
        .items
        .into_iter()
        .filter_map(|item| match item {
            UnifiedDebtItemOutput::File(file_item) => Some(*file_item),
            UnifiedDebtItemOutput::Function(_) => None,
        })
        .collect();

    let mut matrix = DependencyMatrix::from_file_dependencies(&file_items);
    matrix.optimize_ordering();
    matrix
}

/// Render the header with DSM metrics
fn render_header(frame: &mut Frame, matrix: &DependencyMatrix, area: Rect, theme: &Theme) {
    let metrics = &matrix.metrics;

    let mut lines = vec![
        Line::from(vec![
            Span::styled(
                "DEPENDENCY STRUCTURE MATRIX",
                Style::default()
                    .fg(theme.primary_color())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            Span::styled(
                "[m] to exit".to_string(),
                Style::default().fg(theme.muted_color()),
            ),
        ]),
        Line::from(""),
    ];

    // Metrics line 1
    lines.push(Line::from(vec![
        Span::styled("Modules: ", Style::default().fg(theme.muted_color())),
        Span::styled(
            metrics.module_count.to_string(),
            Style::default().fg(Color::White),
        ),
        Span::raw("  "),
        Span::styled("Dependencies: ", Style::default().fg(theme.muted_color())),
        Span::styled(
            metrics.dependency_count.to_string(),
            Style::default().fg(Color::White),
        ),
        Span::raw("  "),
        Span::styled("Cycles: ", Style::default().fg(theme.muted_color())),
        Span::styled(
            metrics.cycle_count.to_string(),
            if metrics.cycle_count > 0 {
                Style::default().fg(Color::Red)
            } else {
                Style::default().fg(Color::Green)
            },
        ),
    ]));

    // Metrics line 2
    let layering_color = if metrics.layering_score >= 0.9 {
        Color::Green
    } else if metrics.layering_score >= 0.7 {
        Color::Yellow
    } else {
        Color::Red
    };

    lines.push(Line::from(vec![
        Span::styled("Layering: ", Style::default().fg(theme.muted_color())),
        Span::styled(
            format!("{:.0}%", metrics.layering_score * 100.0),
            Style::default().fg(layering_color),
        ),
        Span::raw("  "),
        Span::styled("Density: ", Style::default().fg(theme.muted_color())),
        Span::styled(
            format!("{:.1}%", metrics.density * 100.0),
            Style::default().fg(Color::White),
        ),
        Span::raw("  "),
        Span::styled("Propagation: ", Style::default().fg(theme.muted_color())),
        Span::styled(
            format!("{:.1}", metrics.propagation_cost),
            Style::default().fg(Color::White),
        ),
    ]));

    // Cycle details if any
    if !matrix.cycles.is_empty() {
        lines.push(Line::from(""));
        let cycle_text = matrix
            .cycles
            .iter()
            .take(3) // Show max 3 cycles
            .map(|c| c.modules.join("→"))
            .collect::<Vec<_>>()
            .join(", ");

        let more = if matrix.cycles.len() > 3 {
            format!(" (+{} more)", matrix.cycles.len() - 3)
        } else {
            String::new()
        };

        lines.push(Line::from(vec![
            Span::styled("Cycles: ", Style::default().fg(Color::Red)),
            Span::styled(
                format!("{}{}", cycle_text, more),
                Style::default().fg(Color::Red),
            ),
        ]));
    }

    let paragraph = Paragraph::new(lines)
        .block(Block::default().borders(Borders::BOTTOM))
        .wrap(Wrap { trim: false });

    frame.render_widget(paragraph, area);
}

/// Render the DSM matrix
fn render_matrix(
    frame: &mut Frame,
    app: &ResultsApp,
    matrix: &DependencyMatrix,
    area: Rect,
    theme: &Theme,
) {
    if matrix.modules.is_empty() {
        let paragraph =
            Paragraph::new("No modules to display. Run analysis with file dependencies.")
                .style(Style::default().fg(theme.muted_color()));
        frame.render_widget(paragraph, area);
        return;
    }

    let scroll_x = app.dsm_scroll_x();
    let scroll_y = app.dsm_scroll_y();

    // Calculate visible area
    let name_width = 20usize; // Module name column width
    let cell_width = 3usize; // Width per cell
    let visible_cols = ((area.width as usize).saturating_sub(name_width + 2)) / cell_width;
    let visible_rows = area.height as usize;

    let mut lines = Vec::new();

    // Column header row (module indices)
    let mut header_spans = vec![Span::styled(
        format!("{:>width$} ", "", width = name_width),
        Style::default().fg(theme.muted_color()),
    )];

    for col_idx in scroll_x..matrix.modules.len().min(scroll_x + visible_cols) {
        let col_str = format!("{:>2} ", col_idx);
        header_spans.push(Span::styled(
            col_str,
            Style::default()
                .fg(theme.muted_color())
                .add_modifier(Modifier::DIM),
        ));
    }
    lines.push(Line::from(header_spans));

    // Matrix rows
    for row_idx in scroll_y
        ..matrix
            .modules
            .len()
            .min(scroll_y + visible_rows.saturating_sub(1))
    {
        let module_name = &matrix.modules[row_idx];
        let short_name: String = if module_name.len() > name_width {
            format!("..{}", &module_name[module_name.len() - (name_width - 2)..])
        } else {
            format!("{:>width$}", module_name, width = name_width)
        };

        let mut row_spans = vec![Span::styled(
            format!("{} ", short_name),
            Style::default().fg(Color::White),
        )];

        for col_idx in scroll_x..matrix.modules.len().min(scroll_x + visible_cols) {
            let cell = &matrix.matrix[row_idx][col_idx];
            let (symbol, style) = cell_style(cell, row_idx, col_idx, theme);
            row_spans.push(Span::styled(format!("{:>2} ", symbol), style));
        }

        lines.push(Line::from(row_spans));
    }

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, area);
}

/// Get the display style for a cell
fn cell_style(
    cell: &crate::analysis::dsm::DsmCell,
    row: usize,
    col: usize,
    _theme: &Theme,
) -> (&'static str, Style) {
    if row == col {
        // Diagonal
        ("■", Style::default().fg(Color::DarkGray))
    } else if cell.is_cycle && cell.has_dependency {
        // Cycle (above diagonal with dependency)
        (
            "●",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        )
    } else if cell.has_dependency {
        // Normal dependency (below diagonal)
        ("×", Style::default().fg(Color::Green))
    } else {
        // No dependency
        ("·", Style::default().fg(Color::DarkGray))
    }
}

/// Render the footer with legend
fn render_footer(frame: &mut Frame, area: Rect, theme: &Theme) {
    let lines = vec![
        Line::from(vec![
            Span::styled("× ", Style::default().fg(Color::Green)),
            Span::styled("dependency  ", Style::default().fg(theme.muted_color())),
            Span::styled("● ", Style::default().fg(Color::Red)),
            Span::styled("cycle  ", Style::default().fg(theme.muted_color())),
            Span::styled("■ ", Style::default().fg(Color::DarkGray)),
            Span::styled("self  ", Style::default().fg(theme.muted_color())),
            Span::styled("· ", Style::default().fg(Color::DarkGray)),
            Span::styled("none", Style::default().fg(theme.muted_color())),
        ]),
        Line::from(vec![Span::styled(
            "↑↓←→/hjkl: scroll  g: reset  m/Esc/q: exit",
            Style::default().fg(theme.muted_color()),
        )]),
    ];

    let paragraph = Paragraph::new(lines).block(Block::default().borders(Borders::TOP));

    frame.render_widget(paragraph, area);
}
