//! Animation helpers for smooth TUI transitions.

/// Zen bamboo ASCII art frames for gentle swaying animation.
/// Each frame is 6 characters wide and 3 lines tall.
/// The bamboo sways gently in the breeze - subtle and calming.
pub const BAMBOO_FRAMES: [&[&str]; 6] = [
    // Frame 0 - upright
    &[" \\|/  ", "  |   ", "  |   "],
    // Frame 1 - slight lean right
    &["  \\|/ ", "  |   ", "  |   "],
    // Frame 2 - lean right
    &["   \\|/", "   |  ", "  |   "],
    // Frame 3 - upright
    &[" \\|/  ", "  |   ", "  |   "],
    // Frame 4 - slight lean left
    &["\\|/   ", "  |   ", "  |   "],
    // Frame 5 - lean left
    &["\\|/   ", " |    ", "  |   "],
];

/// Animation controller for frame-based animations
pub struct AnimationController {
    frame: usize,
    fps: usize,
}

impl AnimationController {
    /// Create a new animation controller
    pub fn new(fps: usize) -> Self {
        Self { frame: 0, fps }
    }

    /// Advance to the next frame
    pub fn tick(&mut self) {
        self.frame = (self.frame + 1) % (self.fps * 10); // Loop every 10 seconds
    }

    /// Get the current frame number
    pub fn frame(&self) -> usize {
        self.frame
    }

    /// Get animated arrow character for progress
    pub fn arrow_char(&self) -> &'static str {
        // Cycle through arrow variants for subtle animation
        match (self.frame / 3) % 3 {
            0 => "▸",
            1 => "▹",
            _ => "▸",
        }
    }

    /// Get spinner character
    pub fn spinner_char(&self) -> &'static str {
        // Braille spinner for smooth animation
        match (self.frame / 8) % 4 {
            0 => "⠋",
            1 => "⠙",
            2 => "⠹",
            _ => "⠸",
        }
    }

    /// Get pulse alpha value (0.0 to 1.0)
    ///
    /// Used for pulsing effects on active elements
    pub fn pulse_alpha(&self) -> f32 {
        use std::f32::consts::PI;
        let phase = self.frame as f32 / self.fps as f32;
        (phase * PI * 2.0).sin() * 0.3 + 0.7
    }

    /// Get the current bamboo animation frame index (0-5)
    ///
    /// The bamboo cycles through 6 frames at a slow, meditative pace.
    pub fn bamboo_frame_index(&self) -> usize {
        // Change bamboo frame every ~20 frames (3 Hz at 60 FPS)
        // This gives a gentle, zen-like sway
        (self.frame / 20) % BAMBOO_FRAMES.len()
    }

    /// Get the current bamboo ASCII art lines
    pub fn bamboo_lines(&self) -> &'static [&'static str] {
        BAMBOO_FRAMES[self.bamboo_frame_index()]
    }
}

impl Default for AnimationController {
    fn default() -> Self {
        Self::new(60)
    }
}

/// Render an animated progress bar with arrows
pub fn render_progress_arrows(progress: f64, width: usize, arrow_char: &str) -> String {
    let filled = (progress * width as f64) as usize;
    let empty = width.saturating_sub(filled);

    format!("{}{}", arrow_char.repeat(filled), "·".repeat(empty))
}

/// Render a dotted leader line
pub fn render_dotted_leader(width: usize) -> String {
    "·".repeat(width)
}

/// Get bamboo ASCII art lines for a given animation frame
///
/// This is a convenience function for renderers that track frame count
/// separately from AnimationController.
pub fn get_bamboo_lines(frame: usize) -> &'static [&'static str] {
    // Same timing as AnimationController::bamboo_frame_index
    let bamboo_idx = (frame / 20) % BAMBOO_FRAMES.len();
    BAMBOO_FRAMES[bamboo_idx]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_animation_controller_cycles() {
        let mut ctrl = AnimationController::new(60);

        for _ in 0..60 {
            ctrl.tick();
        }

        // Should have cycled through different states
        assert!(ctrl.frame() > 0);
    }

    #[test]
    fn test_arrow_animation() {
        let ctrl = AnimationController::new(60);
        let arrow = ctrl.arrow_char();
        assert!(arrow == "▸" || arrow == "▹");
    }

    #[test]
    fn test_spinner_animation() {
        let ctrl = AnimationController::new(60);
        let spinner = ctrl.spinner_char();
        assert!(["⠋", "⠙", "⠹", "⠸"].contains(&spinner));
    }

    #[test]
    fn test_pulse_alpha_range() {
        let ctrl = AnimationController::new(60);
        let alpha = ctrl.pulse_alpha();
        assert!((0.0..=1.0).contains(&alpha));
    }

    #[test]
    fn test_progress_arrows_rendering() {
        let arrows = render_progress_arrows(0.5, 20, "▸");
        // At 50% of 20 width: 10 arrows + 10 dots (Unicode characters)
        // Each arrow (▸) is 3 bytes, each dot (·) is 2 bytes: 10*3 + 10*2 = 50 bytes
        assert_eq!(arrows.len(), 50);
        assert!(arrows.contains("▸"));
        assert!(arrows.contains("·"));
    }

    #[test]
    fn test_dotted_leader() {
        let leader = render_dotted_leader(10);
        assert_eq!(leader.len(), 10 * "·".len());
        assert_eq!(leader, "··········");
    }

    #[test]
    fn test_bamboo_frames_count() {
        assert_eq!(BAMBOO_FRAMES.len(), 6);
    }

    #[test]
    fn test_bamboo_frame_consistency() {
        // Each frame should have 3 lines of 6 characters
        for (i, bamboo_frame) in BAMBOO_FRAMES.iter().enumerate() {
            assert_eq!(bamboo_frame.len(), 3, "Frame {} should have 3 lines", i);
            for (j, line) in bamboo_frame.iter().enumerate() {
                assert_eq!(line.len(), 6, "Frame {} line {} should be 6 chars", i, j);
            }
        }
    }

    #[test]
    fn test_get_bamboo_lines_cycles() {
        // Frame 0 should give bamboo frame 0
        let lines0 = get_bamboo_lines(0);
        assert_eq!(lines0.len(), 3);

        // Frame 20 should give bamboo frame 1
        let lines1 = get_bamboo_lines(20);
        assert_eq!(lines1.len(), 3);

        // Frame 120 (20 * 6) should cycle back to frame 0
        let lines_cycled = get_bamboo_lines(120);
        assert_eq!(lines0, lines_cycled);
    }

    #[test]
    fn test_bamboo_animation_controller() {
        let mut ctrl = AnimationController::new(60);
        let initial_idx = ctrl.bamboo_frame_index();
        assert!(initial_idx < 6);

        // After 20 ticks, should advance to next frame
        for _ in 0..20 {
            ctrl.tick();
        }
        let next_idx = ctrl.bamboo_frame_index();
        assert_eq!((initial_idx + 1) % 6, next_idx);
    }
}
