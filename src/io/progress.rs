//! Unified progress display for debtmap analysis.
//!
//! Provides a clean, phase-based progress flow that shows users exactly what's happening
//! during analysis. Progress is displayed as numbered phases (1/3, 2/3, etc.) with
//! consistent formatting and time tracking.
//!
//! # Example Output
//!
//! ```text
//! → 1/2 files parse...              511 found
//! → 1/2 files parse...              511/511 (100%) - 2s
//! ✓ 1/2 files parse...              511/511 (100%) - 2s
//! → 2/2 Building call graph...      511/511 (100%) - 1s
//! ✓ 2/2 Building call graph...      511/511 (100%) - 1s
//!
//! Analysis complete in 3.2s
//! ```

use once_cell::sync::Lazy;
use parking_lot::Mutex;
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

#[derive(Debug, Clone, Copy, PartialEq)]
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
                AnalysisPhase::new("files parse", PhaseProgress::Indeterminate),
                AnalysisPhase::new("Building call graph", PhaseProgress::Indeterminate),
            ],
            current_phase: 0,
            start_time: Instant::now(),
            interactive: is_interactive,
            last_update: Instant::now(),
        }
    }

    /// Initialize global progress tracker
    pub fn init_global() {
        // parking_lot::Mutex::lock() never fails (no poisoning)
        *GLOBAL_UNIFIED_PROGRESS.lock() = Some(Self::new());
    }

    /// Access global progress tracker with a closure
    pub fn with_global<F, R>(f: F) -> Option<R>
    where
        F: FnOnce(&mut AnalysisProgress) -> R,
    {
        // parking_lot::Mutex::lock() never fails (no poisoning)
        let mut guard = GLOBAL_UNIFIED_PROGRESS.lock();
        guard.as_mut().map(f)
    }

    /// Clear global progress tracker
    pub fn clear_global() {
        // parking_lot::Mutex::lock() never fails (no poisoning)
        *GLOBAL_UNIFIED_PROGRESS.lock() = None;
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
        if !self.can_render_current_phase() {
            return;
        }

        if self.interactive {
            render_interactive_phase(&self.current_render_state());
            return;
        }

        self.render_ci_phase();
    }

    fn can_render_current_phase(&self) -> bool {
        self.current_phase < self.phases.len() && !is_tui_active()
    }

    fn current_render_state(&self) -> PhaseRenderState {
        let phase = &self.phases[self.current_phase];

        PhaseRenderState {
            phase_index: self.current_phase,
            phase_num: self.current_phase + 1,
            total_phases: self.phases.len(),
            name: phase.name,
            status: phase.status,
            progress: format_progress(&phase.progress),
            duration_secs: phase.duration.map(|d| d.as_secs()),
        }
    }

    fn render_ci_phase(&mut self) {
        let state = self.current_render_state();
        let has_printed_start = self.phases[state.phase_index].printed_start;

        if let Some(line) = format_ci_phase_line(&state, has_printed_start) {
            eprintln!("{line}");
            self.mark_ci_start_printed(&state);
        }
    }

    fn mark_ci_start_printed(&mut self, state: &PhaseRenderState) {
        if matches!(state.status, PhaseStatus::InProgress) {
            self.phases[state.phase_index].printed_start = true;
        }
    }
}

#[derive(Debug, PartialEq)]
struct PhaseRenderState {
    phase_index: usize,
    phase_num: usize,
    total_phases: usize,
    name: &'static str,
    status: PhaseStatus,
    progress: String,
    duration_secs: Option<u64>,
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

fn is_tui_active() -> bool {
    crate::progress::ProgressManager::global()
        .and_then(|m| m.is_tui_active())
        .unwrap_or(false)
}

fn render_interactive_phase(state: &PhaseRenderState) {
    eprint!(
        "\r\x1b[K{} {}/{} {}...{}{}",
        phase_indicator(state.status),
        state.phase_num,
        state.total_phases,
        state.name,
        state.progress,
        format_duration_suffix(state.duration_secs)
    );

    if matches!(state.status, PhaseStatus::Complete) {
        eprintln!();
    }
}

fn phase_indicator(status: PhaseStatus) -> &'static str {
    match status {
        PhaseStatus::InProgress => "→",
        PhaseStatus::Complete => "✓",
        PhaseStatus::Pending => " ",
    }
}

fn format_duration_suffix(duration_secs: Option<u64>) -> String {
    duration_secs
        .map(|seconds| format!(" - {seconds}s"))
        .unwrap_or_default()
}

fn format_ci_phase_line(state: &PhaseRenderState, has_printed_start: bool) -> Option<String> {
    match state.status {
        PhaseStatus::Complete => Some(format!(
            "✓ {}/{} {} - {}s",
            state.phase_num,
            state.total_phases,
            state.name,
            state.duration_secs.unwrap_or(0)
        )),
        PhaseStatus::InProgress if !has_printed_start => Some(format!(
            "→ {}/{} {}...",
            state.phase_num, state.total_phases, state.name
        )),
        _ => None,
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
    fn test_phase_indicator() {
        assert_eq!(phase_indicator(PhaseStatus::Pending), " ");
        assert_eq!(phase_indicator(PhaseStatus::InProgress), "→");
        assert_eq!(phase_indicator(PhaseStatus::Complete), "✓");
    }

    #[test]
    fn test_format_duration_suffix() {
        assert_eq!(format_duration_suffix(None), "");
        assert_eq!(format_duration_suffix(Some(12)), " - 12s");
    }

    #[test]
    fn test_format_ci_phase_line_starts_once() {
        let state = test_render_state(PhaseStatus::InProgress, None);

        assert_eq!(
            format_ci_phase_line(&state, false),
            Some("→ 1/2 files parse...".to_string())
        );
        assert_eq!(format_ci_phase_line(&state, true), None);
    }

    #[test]
    fn test_format_ci_phase_line_completion_includes_duration() {
        let state = test_render_state(PhaseStatus::Complete, Some(7));

        assert_eq!(
            format_ci_phase_line(&state, true),
            Some("✓ 1/2 files parse - 7s".to_string())
        );
    }

    #[test]
    fn test_format_ci_phase_line_skips_pending() {
        let state = test_render_state(PhaseStatus::Pending, None);

        assert_eq!(format_ci_phase_line(&state, false), None);
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

    fn test_render_state(status: PhaseStatus, duration_secs: Option<u64>) -> PhaseRenderState {
        PhaseRenderState {
            phase_index: 0,
            phase_num: 1,
            total_phases: 2,
            name: "files parse",
            status,
            progress: String::new(),
            duration_secs,
        }
    }
}
