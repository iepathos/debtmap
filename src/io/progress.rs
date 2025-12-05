//! Unified progress display for debtmap analysis.
//!
//! Provides a clean, phase-based progress flow that shows users exactly what's happening
//! during analysis. Progress is displayed as numbered phases (1/4, 2/4, etc.) with
//! consistent formatting and time tracking.
//!
//! # Example Output
//!
//! ```text
//! → 1/4 Discovering files...        511 found
//! ✓ 1/4 Discovering files...        511 found - 0s
//! → 2/4 Analyzing complexity...     511/511 (100%) - 2s
//! ✓ 2/4 Analyzing complexity...     511/511 (100%) - 2s
//! → 3/4 Building call graph...      511/511 (100%) - 1s
//! ✓ 3/4 Building call graph...      511/511 (100%) - 1s
//! → 4/4 Refining analysis...   148769/148769 (100%) - 3s
//! ✓ 4/4 Refining analysis...   148769/148769 (100%) - 3s
//!
//! Analysis complete in 6.2s
//! ```

use once_cell::sync::Lazy;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Global progress tracker instance
static GLOBAL_UNIFIED_PROGRESS: Lazy<Mutex<Option<AnalysisProgress>>> =
    Lazy::new(|| Mutex::new(None));

/// Progress tracker for multi-phase analysis operations
pub struct AnalysisProgress {
    phases: Vec<AnalysisPhase>,
    current_phase: usize,
    start_time: Instant,
    interactive: bool,
    last_update: Instant,
}

#[derive(Debug)]
struct AnalysisPhase {
    name: &'static str,
    status: PhaseStatus,
    start_time: Option<Instant>,
    duration: Option<Duration>,
    progress: PhaseProgress,
    /// Track if we've already printed the in-progress message (for CI/CD mode)
    printed_start: bool,
}

#[derive(Debug, Clone, PartialEq)]
enum PhaseStatus {
    Pending,
    InProgress,
    Complete,
}

#[derive(Debug, Clone)]
pub enum PhaseProgress {
    Indeterminate,
    Count(usize),
    Progress { current: usize, total: usize },
}

impl AnalysisProgress {
    /// Create new progress tracker
    pub fn new() -> Self {
        // Check if stderr is a TTY using std::io::IsTerminal
        use std::io::IsTerminal;
        let is_interactive = std::io::stderr().is_terminal();

        Self {
            phases: vec![
                AnalysisPhase::new("Discovering files", PhaseProgress::Indeterminate),
                AnalysisPhase::new("Analyzing complexity", PhaseProgress::Indeterminate),
                AnalysisPhase::new("Building call graph", PhaseProgress::Indeterminate),
                AnalysisPhase::new("Refining analysis", PhaseProgress::Indeterminate),
            ],
            current_phase: 0,
            start_time: Instant::now(),
            interactive: is_interactive,
            last_update: Instant::now(),
        }
    }

    /// Initialize global progress tracker
    pub fn init_global() {
        *GLOBAL_UNIFIED_PROGRESS.lock().unwrap() = Some(Self::new());
    }

    /// Access global progress tracker with a closure
    pub fn with_global<F, R>(f: F) -> Option<R>
    where
        F: FnOnce(&mut AnalysisProgress) -> R,
    {
        GLOBAL_UNIFIED_PROGRESS
            .lock()
            .ok()
            .and_then(|mut guard| guard.as_mut().map(f))
    }

    /// Clear global progress tracker
    pub fn clear_global() {
        *GLOBAL_UNIFIED_PROGRESS.lock().unwrap() = None;
    }

    /// Start a specific phase (0-indexed)
    pub fn start_phase(&mut self, phase_index: usize) {
        if phase_index >= self.phases.len() {
            return;
        }

        self.current_phase = phase_index;
        self.phases[phase_index].status = PhaseStatus::InProgress;
        self.phases[phase_index].start_time = Some(Instant::now());
        self.phases[phase_index].printed_start = false; // Reset for new phase
        self.render();
    }

    /// Update progress for current phase
    pub fn update_progress(&mut self, progress: PhaseProgress) {
        if self.current_phase >= self.phases.len() {
            return;
        }

        self.phases[self.current_phase].progress = progress;

        // Throttle updates to at most 10 per second
        if self.should_update() {
            self.render();
            self.last_update = Instant::now();
        }
    }

    /// Mark current phase as complete
    pub fn complete_phase(&mut self) {
        if self.current_phase >= self.phases.len() {
            return;
        }

        let phase = &mut self.phases[self.current_phase];
        phase.status = PhaseStatus::Complete;
        if let Some(start) = phase.start_time {
            phase.duration = Some(start.elapsed());
        }
        self.render();
    }

    /// Finish all analysis and show total time
    pub fn finish(&self) {
        let total_duration = self.start_time.elapsed();
        eprintln!(
            "\nAnalysis complete in {:.1}s",
            total_duration.as_secs_f64()
        );
    }

    /// Check if we should update the display (throttling)
    fn should_update(&self) -> bool {
        // Update at most 10 times per second
        self.last_update.elapsed() > Duration::from_millis(100)
    }

    /// Render current progress state
    fn render(&mut self) {
        if self.current_phase >= self.phases.len() {
            return;
        }

        // Don't render if TUI is active (prevents interference)
        if crate::progress::ProgressManager::global()
            .and_then(|m| m.is_tui_active())
            .unwrap_or(false)
        {
            return;
        }

        let phase_index = self.current_phase;
        let phase = &self.phases[phase_index];
        let phase_num = self.current_phase + 1;
        let total_phases = self.phases.len();

        if self.interactive {
            // Interactive mode: overwrite line with carriage return
            let indicator = match phase.status {
                PhaseStatus::InProgress => "→",
                PhaseStatus::Complete => "✓",
                PhaseStatus::Pending => " ",
            };

            let progress_str = format_progress(&phase.progress);

            let duration_str = phase
                .duration
                .map(|d| format!(" - {}s", d.as_secs()))
                .unwrap_or_default();

            // Clear the line first, then write new content
            // Use ANSI escape code to clear from cursor to end of line: \x1b[K
            eprint!(
                "\r\x1b[K{} {}/{} {}...{}{}",
                indicator, phase_num, total_phases, phase.name, progress_str, duration_str
            );

            // Move to next line when phase completes
            if matches!(phase.status, PhaseStatus::Complete) {
                eprintln!();
            }
        } else {
            // CI/CD mode: print complete lines
            match phase.status {
                PhaseStatus::Complete => {
                    let duration = phase
                        .duration
                        .map(|d| format!("{}s", d.as_secs()))
                        .unwrap_or_else(|| "0s".to_string());
                    eprintln!(
                        "✓ {}/{} {} - {}",
                        phase_num, total_phases, phase.name, duration
                    );
                }
                PhaseStatus::InProgress => {
                    // Only print once per phase, not on every update
                    if !self.phases[phase_index].printed_start {
                        eprintln!("→ {}/{} {}...", phase_num, total_phases, phase.name);
                        self.phases[phase_index].printed_start = true;
                    }
                }
                _ => {}
            }
        }
    }
}

impl Default for AnalysisProgress {
    fn default() -> Self {
        Self::new()
    }
}

impl AnalysisPhase {
    fn new(name: &'static str, progress: PhaseProgress) -> Self {
        Self {
            name,
            status: PhaseStatus::Pending,
            start_time: None,
            duration: None,
            progress,
            printed_start: false,
        }
    }
}

/// Format progress display string
fn format_progress(progress: &PhaseProgress) -> String {
    match progress {
        PhaseProgress::Indeterminate => String::new(),
        PhaseProgress::Count(n) => format!("{} found", n),
        PhaseProgress::Progress { current, total } => {
            if *total == 0 {
                "0/0 (0%)".to_string()
            } else {
                let pct = (*current as f64 / *total as f64 * 100.0) as usize;
                format!("{}/{} ({}%)", current, total, pct)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_phase_lifecycle() {
        let mut progress = AnalysisProgress::new();

        progress.start_phase(0);
        assert_eq!(progress.current_phase, 0);
        assert!(matches!(progress.phases[0].status, PhaseStatus::InProgress));

        progress.update_progress(PhaseProgress::Count(100));
        assert!(matches!(
            progress.phases[0].progress,
            PhaseProgress::Count(100)
        ));

        progress.complete_phase();
        assert!(matches!(progress.phases[0].status, PhaseStatus::Complete));
        assert!(progress.phases[0].duration.is_some());
    }

    #[test]
    fn test_format_phase_progress_count() {
        let progress = PhaseProgress::Count(511);
        let formatted = format_progress(&progress);
        assert_eq!(formatted, "511 found");
    }

    #[test]
    fn test_format_phase_progress_fractional() {
        let progress = PhaseProgress::Progress {
            current: 256,
            total: 511,
        };
        let formatted = format_progress(&progress);
        assert_eq!(formatted, "256/511 (50%)");
    }

    #[test]
    fn test_format_phase_progress_complete() {
        let progress = PhaseProgress::Progress {
            current: 511,
            total: 511,
        };
        let formatted = format_progress(&progress);
        assert_eq!(formatted, "511/511 (100%)");
    }

    #[test]
    fn test_format_phase_progress_zero_total() {
        let progress = PhaseProgress::Progress {
            current: 0,
            total: 0,
        };
        let formatted = format_progress(&progress);
        assert_eq!(formatted, "0/0 (0%)");
    }

    #[test]
    fn test_indeterminate_progress() {
        let progress = PhaseProgress::Indeterminate;
        let formatted = format_progress(&progress);
        assert_eq!(formatted, "");
    }

    #[test]
    fn test_multiple_phases() {
        let mut progress = AnalysisProgress::new();

        // Phase 1: Discovery
        progress.start_phase(0);
        progress.update_progress(PhaseProgress::Count(511));
        progress.complete_phase();
        assert!(matches!(progress.phases[0].status, PhaseStatus::Complete));

        // Phase 2: Analysis
        progress.start_phase(1);
        progress.update_progress(PhaseProgress::Progress {
            current: 511,
            total: 511,
        });
        progress.complete_phase();
        assert!(matches!(progress.phases[1].status, PhaseStatus::Complete));

        // Both phases should have durations
        assert!(progress.phases[0].duration.is_some());
        assert!(progress.phases[1].duration.is_some());
    }
}
