//! Responsive layout management for different terminal sizes.

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    Frame,
};

use super::app::App;
use super::renderer::{render_compact, render_minimal, render_ui};

/// Layout mode based on terminal width
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutMode {
    /// Full detail view (>120 cols)
    Full,
    /// Standard view (80-120 cols)
    Standard,
    /// Compact view without sub-tasks (40-80 cols)
    Compact,
    /// Minimal view with just progress bar (<40 cols)
    Minimal,
}

impl LayoutMode {
    /// Determine layout mode from terminal width
    pub fn from_terminal_width(width: u16) -> Self {
        match width {
            0..=39 => Self::Minimal,
            40..=79 => Self::Compact,
            80..=119 => Self::Standard,
            _ => Self::Full,
        }
    }

    /// Check if this mode shows sub-tasks
    pub fn shows_subtasks(&self) -> bool {
        matches!(self, Self::Full | Self::Standard)
    }

    /// Check if this mode shows full metrics
    pub fn shows_full_metrics(&self) -> bool {
        matches!(self, Self::Full)
    }
}

/// Calculate main layout constraints for the TUI
pub fn calculate_layout(area: Rect) -> Vec<Rect> {
    Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(5), // Header (title + progress bar)
            Constraint::Min(10),   // Main content (pipeline stages)
            Constraint::Length(3), // Footer (statistics)
        ])
        .split(area)
        .to_vec()
}

/// Render the TUI with adaptive layout
pub fn render_adaptive(frame: &mut Frame, app: &App) {
    let mode = LayoutMode::from_terminal_width(frame.area().width);

    match mode {
        LayoutMode::Full | LayoutMode::Standard => render_ui(frame, app),
        LayoutMode::Compact => render_compact(frame, app),
        LayoutMode::Minimal => render_minimal(frame, app),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_layout_mode_selection() {
        assert_eq!(LayoutMode::from_terminal_width(30), LayoutMode::Minimal);
        assert_eq!(LayoutMode::from_terminal_width(50), LayoutMode::Compact);
        assert_eq!(LayoutMode::from_terminal_width(90), LayoutMode::Standard);
        assert_eq!(LayoutMode::from_terminal_width(150), LayoutMode::Full);
    }

    #[test]
    fn test_subtask_visibility() {
        assert!(!LayoutMode::Minimal.shows_subtasks());
        assert!(!LayoutMode::Compact.shows_subtasks());
        assert!(LayoutMode::Standard.shows_subtasks());
        assert!(LayoutMode::Full.shows_subtasks());
    }

    #[test]
    fn test_metrics_visibility() {
        assert!(!LayoutMode::Minimal.shows_full_metrics());
        assert!(!LayoutMode::Compact.shows_full_metrics());
        assert!(!LayoutMode::Standard.shows_full_metrics());
        assert!(LayoutMode::Full.shows_full_metrics());
    }

    #[test]
    fn test_layout_constraints() {
        let area = Rect::new(0, 0, 100, 30);
        let chunks = calculate_layout(area);

        // Should have 3 sections
        assert_eq!(chunks.len(), 3);

        // Verify header is 5 lines
        assert_eq!(chunks[0].height, 5);

        // Verify footer is 3 lines
        assert_eq!(chunks[2].height, 3);
    }
}
