//! Color themes and styling for TUI components.

use ratatui::style::{Color, Modifier, Style};

/// Zen minimalist color scheme for debtmap TUI
pub struct Theme {
    /// Primary accent color (cyan for active elements)
    pub primary: Color,
    /// Success color (green for completed elements)
    pub success: Color,
    /// Muted color (dark gray for pending/inactive elements)
    pub muted: Color,
    /// Text color (white for normal text)
    pub text: Color,
    /// Background color (black/default)
    pub background: Color,
}

impl Theme {
    /// Create the default zen minimalist theme
    pub fn default_theme() -> Self {
        Self {
            primary: Color::Cyan,
            success: Color::Green,
            muted: Color::DarkGray,
            text: Color::White,
            background: Color::Reset,
        }
    }

    /// Accent color (alias for primary)
    pub fn accent(&self) -> Color {
        self.primary
    }

    /// Secondary color (alias for success)
    pub fn secondary(&self) -> Color {
        self.success
    }

    /// Success color (green for positive status messages)
    pub fn success(&self) -> Color {
        self.success
    }

    /// Warning color (yellow for warnings and errors)
    pub fn warning(&self) -> Color {
        Color::Yellow
    }

    /// Style for completed stage markers (✓)
    pub fn completed_style(&self) -> Style {
        Style::default().fg(self.success)
    }

    /// Style for active stage markers (▸)
    pub fn active_style(&self) -> Style {
        Style::default()
            .fg(self.primary)
            .add_modifier(Modifier::BOLD)
    }

    /// Style for pending stage markers (·)
    pub fn pending_style(&self) -> Style {
        Style::default().fg(self.muted)
    }

    /// Style for stage names (based on status)
    pub fn stage_name_style(&self, is_active: bool) -> Style {
        if is_active {
            Style::default()
                .fg(self.primary)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(self.text)
        }
    }

    /// Style for metrics and statistics
    pub fn metric_style(&self) -> Style {
        Style::default().fg(self.muted)
    }

    /// Style for progress bars
    pub fn progress_bar_style(&self) -> Style {
        Style::default().fg(self.primary)
    }

    /// Style for progress bar background
    pub fn progress_bar_bg_style(&self) -> Style {
        Style::default().fg(self.muted)
    }

    /// Style for dotted leaders
    pub fn dotted_leader_style(&self) -> Style {
        Style::default().fg(self.muted)
    }

    /// Style for animated arrows
    pub fn arrow_style(&self) -> Style {
        Style::default().fg(self.primary)
    }

    /// Style for elapsed time
    pub fn time_style(&self) -> Style {
        Style::default().fg(self.muted)
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::default_theme()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_theme_creation() {
        let theme = Theme::default_theme();
        assert_eq!(theme.primary, Color::Cyan);
        assert_eq!(theme.success, Color::Green);
        assert_eq!(theme.muted, Color::DarkGray);
    }

    #[test]
    fn test_style_consistency() {
        let theme = Theme::default_theme();
        let completed = theme.completed_style();
        let active = theme.active_style();

        // Verify colors are distinct
        assert_ne!(completed.fg, active.fg);
    }
}
