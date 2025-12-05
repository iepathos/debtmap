//! Core rendering logic for TUI components.

use ratatui::{
    layout::{Constraint, Direction, Layout},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use super::app::{App, StageStatus, SubTask};
use super::layout::calculate_layout;
use super::theme::Theme;

/// Render the full TUI interface
pub fn render_ui(frame: &mut Frame, app: &App) {
    let theme = Theme::default_theme();
    let chunks = calculate_layout(frame.size());

    render_header(frame, app, &theme, chunks[0]);
    render_pipeline(frame, app, &theme, chunks[1]);
    render_footer(frame, app, &theme, chunks[2]);
}

/// Render compact view (no sub-tasks)
pub fn render_compact(frame: &mut Frame, app: &App) {
    let theme = Theme::default_theme();
    let chunks = calculate_layout(frame.size());

    render_header(frame, app, &theme, chunks[0]);
    render_pipeline_compact(frame, app, &theme, chunks[1]);
    render_footer(frame, app, &theme, chunks[2]);
}

/// Render minimal view (just progress bar)
pub fn render_minimal(frame: &mut Frame, app: &App) {
    let theme = Theme::default_theme();
    let area = frame.size();

    // Simple centered progress bar
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([Constraint::Percentage(50), Constraint::Length(3)])
        .split(area);

    let progress_text = format!(
        "{} stage {}/9 - {:.1}s",
        render_progress_bar(app.overall_progress, 30),
        app.current_stage + 1,
        app.elapsed_time.as_secs_f64()
    );

    frame.render_widget(
        Paragraph::new(progress_text).style(theme.progress_bar_style()),
        chunks[1],
    );
}

/// Render header section (title + progress bar)
fn render_header(frame: &mut Frame, app: &App, theme: &Theme, area: ratatui::layout::Rect) {
    let header_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Title line
            Constraint::Length(1), // Empty
            Constraint::Length(1), // Progress bar
            Constraint::Length(1), // Stage counter
        ])
        .split(area);

    // Title with elapsed time
    let title_line = Line::from(vec![
        Span::raw("debtmap"),
        Span::raw("  "),
        Span::styled(
            format!("{:.1}s", app.elapsed_time.as_secs_f64()),
            theme.time_style(),
        ),
    ]);
    frame.render_widget(Paragraph::new(title_line), header_chunks[0]);

    // Progress bar
    let progress_text = format!(
        "{} {}%",
        render_progress_bar(app.overall_progress, area.width.saturating_sub(6) as usize),
        (app.overall_progress * 100.0) as u32
    );
    frame.render_widget(
        Paragraph::new(progress_text).style(theme.progress_bar_style()),
        header_chunks[2],
    );

    // Stage counter
    let stage_info = format!("stage {}/9", app.current_stage + 1);
    frame.render_widget(
        Paragraph::new(stage_info).style(theme.metric_style()),
        header_chunks[3],
    );
}

/// Render pipeline stages (full view with sub-tasks)
fn render_pipeline(frame: &mut Frame, app: &App, theme: &Theme, area: ratatui::layout::Rect) {
    let mut lines = Vec::new();

    for stage in &app.stages {
        // Add spacing
        lines.push(Line::from(""));

        // Stage line
        let (icon, style) = match stage.status {
            StageStatus::Completed => ("✓", theme.completed_style()),
            StageStatus::Active => ("▸", theme.active_style()),
            StageStatus::Pending => ("·", theme.pending_style()),
        };

        let mut spans = vec![
            Span::styled(icon, style),
            Span::raw("  "),
            Span::styled(
                &stage.name,
                theme.stage_name_style(stage.status == StageStatus::Active),
            ),
        ];

        // Add metric if present
        if let Some(metric) = &stage.metric {
            let remaining = area
                .width
                .saturating_sub((icon.len() + 2 + stage.name.len() + metric.len() + 2) as u16);
            spans.push(Span::raw(" ".repeat(remaining as usize)));
            spans.push(Span::styled(metric, theme.metric_style()));
        }

        lines.push(Line::from(spans));

        // Sub-tasks (only for active stage)
        if stage.status == StageStatus::Active && !stage.sub_tasks.is_empty() {
            for subtask in &stage.sub_tasks {
                lines.push(render_subtask_line(
                    subtask,
                    app.animation_frame,
                    theme,
                    area.width,
                ));
            }
        }
    }

    frame.render_widget(Paragraph::new(lines), area);
}

/// Render pipeline stages (compact view without sub-tasks)
fn render_pipeline_compact(
    frame: &mut Frame,
    app: &App,
    theme: &Theme,
    area: ratatui::layout::Rect,
) {
    let mut lines = Vec::new();

    for stage in &app.stages {
        let (icon, style) = match stage.status {
            StageStatus::Completed => ("✓", theme.completed_style()),
            StageStatus::Active => ("▸", theme.active_style()),
            StageStatus::Pending => ("·", theme.pending_style()),
        };

        let mut spans = vec![
            Span::styled(icon, style),
            Span::raw("  "),
            Span::styled(
                &stage.name,
                theme.stage_name_style(stage.status == StageStatus::Active),
            ),
        ];

        if let Some(metric) = &stage.metric {
            spans.push(Span::raw("  "));
            spans.push(Span::styled(metric, theme.metric_style()));
        }

        lines.push(Line::from(spans));
    }

    frame.render_widget(Paragraph::new(lines), area);
}

/// Render a sub-task line with progress
fn render_subtask_line(
    subtask: &SubTask,
    frame: usize,
    theme: &Theme,
    width: u16,
) -> Line<'static> {
    let indent = "    ";
    let name_with_indent = format!("{}{}", indent, subtask.name);

    match subtask.status {
        StageStatus::Completed => {
            // Dotted leader to "done"
            let dots_needed = width.saturating_sub((name_with_indent.len() + 5) as u16) as usize;
            Line::from(vec![
                Span::raw(name_with_indent),
                Span::raw(" "),
                Span::styled("·".repeat(dots_needed), theme.dotted_leader_style()),
                Span::styled(" done", theme.completed_style()),
            ])
        }
        StageStatus::Active => {
            if let Some((current, total)) = subtask.progress {
                let progress = current as f64 / total as f64;
                let bar_width: usize = 30;
                let filled = (progress * bar_width as f64) as usize;
                let empty = bar_width.saturating_sub(filled);

                // Animate arrows
                let arrow = match (frame / 3) % 3 {
                    0 => "▸",
                    1 => "▹",
                    _ => "▸",
                };

                Line::from(vec![
                    Span::raw(name_with_indent),
                    Span::raw(" "),
                    Span::styled(arrow.repeat(filled), theme.arrow_style()),
                    Span::styled("·".repeat(empty), theme.dotted_leader_style()),
                    Span::raw(" "),
                    Span::styled(format!("{}/{}", current, total), theme.metric_style()),
                ])
            } else {
                Line::from(Span::raw(name_with_indent))
            }
        }
        StageStatus::Pending => {
            let dots_needed = width.saturating_sub(name_with_indent.len() as u16) as usize;
            Line::from(vec![
                Span::raw(name_with_indent),
                Span::styled("·".repeat(dots_needed), theme.dotted_leader_style()),
            ])
        }
    }
}

/// Render footer statistics bar
fn render_footer(frame: &mut Frame, app: &App, theme: &Theme, area: ratatui::layout::Rect) {
    let stats = format!(
        "functions {}  │  debt {}  │  coverage {:.1}%  │  threads {}",
        format_number(app.functions_count),
        app.debt_count,
        app.coverage_percent,
        app.thread_count
    );

    frame.render_widget(Paragraph::new(stats).style(theme.metric_style()), area);
}

/// Render a progress bar with gradient characters
fn render_progress_bar(progress: f64, width: usize) -> String {
    let filled = (progress * width as f64) as usize;
    let empty = width.saturating_sub(filled);

    format!("{}{}", "▓".repeat(filled), "░".repeat(empty))
}

/// Format large numbers with thousand separators
fn format_number(n: usize) -> String {
    n.to_string()
        .as_bytes()
        .rchunks(3)
        .rev()
        .map(std::str::from_utf8)
        .collect::<Result<Vec<&str>, _>>()
        .unwrap()
        .join(",")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_progress_bar_rendering() {
        let bar = render_progress_bar(0.5, 20);
        assert_eq!(bar.len(), 20 * "▓".len());
        assert!(bar.contains("▓"));
        assert!(bar.contains("░"));
    }

    #[test]
    fn test_progress_bar_bounds() {
        let bar_empty = render_progress_bar(0.0, 10);
        assert_eq!(bar_empty, "░░░░░░░░░░");

        let bar_full = render_progress_bar(1.0, 10);
        assert_eq!(bar_full, "▓▓▓▓▓▓▓▓▓▓");
    }

    #[test]
    fn test_format_number() {
        assert_eq!(format_number(0), "0");
        assert_eq!(format_number(123), "123");
        assert_eq!(format_number(1234), "1,234");
        assert_eq!(format_number(1234567), "1,234,567");
    }
}
