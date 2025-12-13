//! Animation helpers for smooth TUI transitions.

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
}
