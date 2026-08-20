//! Layout utilities and context-specific help overlay.

use super::app::ResultsApp;
use super::detail_shortcuts::{DETAIL_SHORTCUTS, DetailShortcutSection};
use super::view_mode::ViewMode;
use crate::tui::theme::Theme;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HelpOrigin {
    List,
    Detail,
}

/// Render help over the view from which it was opened.
pub fn render_help_overlay(frame: &mut Frame, app: &ResultsApp) {
    let origin = help_origin(app);
    render_origin(frame, app, origin);

    let theme = Theme::default();
    let help_area = centered_rect(90, 90, frame.area());
    frame.render_widget(Clear, help_area);
    frame.render_widget(help_widget(origin, &theme), help_area);
}

fn help_origin(app: &ResultsApp) -> HelpOrigin {
    match app.nav().history.last() {
        Some(ViewMode::Detail) => HelpOrigin::Detail,
        _ => HelpOrigin::List,
    }
}

fn render_origin(frame: &mut Frame, app: &ResultsApp, origin: HelpOrigin) {
    match origin {
        HelpOrigin::List => super::list_view::render(frame, app),
        HelpOrigin::Detail => super::detail_view::render(frame, app),
    }
}

fn help_widget(origin: HelpOrigin, theme: &Theme) -> Paragraph<'static> {
    let lines = match origin {
        HelpOrigin::List => list_help_lines(theme),
        HelpOrigin::Detail => detail_help_lines(theme),
    };
    Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!("Help · debtmap v{}", env!("CARGO_PKG_VERSION")))
            .border_style(Style::default().fg(theme.accent())),
    )
}

fn list_help_lines(theme: &Theme) -> Vec<Line<'static>> {
    let navigation = [
        ("↑↓/j/k", "Move selection"),
        ("g/G", "First/last result"),
        ("PgUp/PgDn", "Move one page"),
        ("Enter/l/→", "View details"),
    ];
    let explore = [
        ("/", "Search"),
        ("s", "Sort"),
        ("f", "Filter"),
        ("u", "Group/ungroup locations"),
    ];
    let actions = [
        ("c / C", "Copy path / LLM finding"),
        ("e/o", "Open in editor"),
        ("q", "Quit"),
        ("?", "Show help"),
    ];
    compose_help_lines(
        theme,
        [
            ("Navigation", navigation.as_slice()),
            ("Search & Filter", explore.as_slice()),
            ("Actions", actions.as_slice()),
        ],
    )
}

fn detail_help_lines(theme: &Theme) -> Vec<Line<'static>> {
    let sections = [
        ("Navigation", DetailShortcutSection::Navigation),
        ("Content Scrolling", DetailShortcutSection::Scrolling),
        ("Actions", DetailShortcutSection::Actions),
    ];
    let mut lines = sections
        .into_iter()
        .enumerate()
        .flat_map(|(index, (title, section))| {
            detail_section_lines(title, section, theme, index > 0)
        })
        .collect::<Vec<_>>();
    lines.push(close_hint(theme));
    lines
}

fn detail_section_lines(
    title: &'static str,
    section: DetailShortcutSection,
    theme: &Theme,
    leading_blank: bool,
) -> Vec<Line<'static>> {
    let shortcuts = DETAIL_SHORTCUTS
        .iter()
        .filter(|shortcut| shortcut.section == section)
        .map(|shortcut| (shortcut.keys, shortcut.description));
    section_lines(title, shortcuts, theme, leading_blank)
}

fn compose_help_lines<const N: usize>(
    theme: &Theme,
    sections: [(&'static str, &[(&'static str, &'static str)]); N],
) -> Vec<Line<'static>> {
    let mut lines = sections
        .into_iter()
        .enumerate()
        .flat_map(|(index, (title, shortcuts))| {
            section_lines(title, shortcuts.iter().copied(), theme, index > 0)
        })
        .collect::<Vec<_>>();
    lines.push(close_hint(theme));
    lines
}

fn section_lines(
    title: &'static str,
    shortcuts: impl Iterator<Item = (&'static str, &'static str)>,
    theme: &Theme,
    leading_blank: bool,
) -> Vec<Line<'static>> {
    let mut lines = Vec::from_iter(leading_blank.then(|| Line::from("")));
    lines.push(section_title(title, theme));
    lines.extend(shortcuts.map(|(keys, description)| shortcut_line(keys, description)));
    lines
}

fn section_title(title: &'static str, theme: &Theme) -> Line<'static> {
    Line::from(Span::styled(
        title,
        Style::default()
            .fg(theme.accent())
            .add_modifier(Modifier::UNDERLINED),
    ))
}

fn shortcut_line(keys: &'static str, description: &'static str) -> Line<'static> {
    Line::from(format!("  {keys:<22}{description}"))
}

fn close_hint(theme: &Theme) -> Line<'static> {
    Line::from(Span::styled(
        "Press any key to close",
        Style::default().fg(theme.muted),
    ))
}

/// Create a centered rectangle.
fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vertical = ratatui::layout::Layout::vertical([
        ratatui::layout::Constraint::Percentage((100 - percent_y) / 2),
        ratatui::layout::Constraint::Percentage(percent_y),
        ratatui::layout::Constraint::Percentage((100 - percent_y) / 2),
    ])
    .split(area);

    ratatui::layout::Layout::horizontal([
        ratatui::layout::Constraint::Percentage((100 - percent_x) / 2),
        ratatui::layout::Constraint::Percentage(percent_x),
        ratatui::layout::Constraint::Percentage((100 - percent_x) / 2),
    ])
    .split(vertical[1])[1]
}
