//! Progress sink implementations for different output modes.
//!
//! This module provides concrete implementations of [`ProgressSink`]:
//!
//! - [`SilentProgressSink`]: No-op implementation for testing/CI
//! - [`CliProgressSink`]: Simple stderr output for CLI mode
//! - [`RecordingProgressSink`]: Captures events for testing
//!
//! # Choosing an Implementation
//!
//! | Use Case | Implementation |
//! |----------|----------------|
//! | Unit tests | [`SilentProgressSink`] or [`RecordingProgressSink`] |
//! | CI/CD | [`SilentProgressSink`] |
//! | CLI tool | [`CliProgressSink`] |
//! | TUI mode | See [`crate::tui::progress_sink::TuiProgressSink`] |
//!
//! # Example: Using RecordingProgressSink in Tests
//!
//! ```rust
//! use debtmap::progress::implementations::{RecordingProgressSink, ProgressEvent};
//! use debtmap::progress::traits::ProgressSink;
//!
//! let recorder = RecordingProgressSink::new();
//!
//! recorder.start_stage("Test Stage");
//! recorder.report("Test Stage", 0, 10);
//! recorder.report("Test Stage", 5, 10);
//! recorder.complete_stage("Test Stage");
//!
//! let events = recorder.events();
//! assert_eq!(events.len(), 4);
//! assert!(matches!(events[0], ProgressEvent::StartStage { .. }));
//! ```

use super::traits::ProgressSink;
use std::io::Write;
use std::sync::{Arc, Mutex};

/// Silent progress sink - no-op implementation for testing/CI.
///
/// This implementation does nothing for all methods, making it ideal for:
/// - Unit tests where progress output would be noise
/// - CI environments where no TTY is available
/// - Benchmarks where progress overhead should be eliminated
///
/// # Performance
///
/// All methods are completely inlined to no-ops, so there is zero runtime
/// overhead when using this sink.
///
/// # Example
///
/// ```rust
/// use debtmap::progress::implementations::SilentProgressSink;
/// use debtmap::progress::traits::ProgressSink;
/// use std::sync::Arc;
///
/// let sink: Arc<dyn ProgressSink> = Arc::new(SilentProgressSink);
///
/// // All calls are no-ops
/// sink.start_stage("Analysis");
/// sink.report("Analysis", 50, 100);
/// sink.complete_stage("Analysis");
/// ```
#[derive(Clone, Copy, Debug, Default)]
pub struct SilentProgressSink;

impl ProgressSink for SilentProgressSink {
    #[inline]
    fn report(&self, _stage: &str, _current: usize, _total: usize) {}

    #[inline]
    fn start_stage(&self, _name: &str) {}

    #[inline]
    fn complete_stage(&self, _name: &str) {}

    #[inline]
    fn warn(&self, _message: &str) {}

    #[inline]
    fn child(&self, _prefix: &str) -> Arc<dyn ProgressSink> {
        Arc::new(SilentProgressSink)
    }
}

/// CLI progress sink - simple stderr output.
///
/// This implementation writes progress updates to stderr in a simple format
/// suitable for non-interactive terminals. Progress is shown on a single line
/// that is overwritten with each update.
///
/// # Output Format
///
/// ```text
/// Analysis
/// Analysis: 1/100
/// Analysis: 50/100
/// Analysis: 100/100
/// Analysis complete
///
/// Warning: Some warning message
/// ```
///
/// # Quiet Mode
///
/// When `quiet` is set to `true`, all output is suppressed except for warnings.
///
/// # Example
///
/// ```rust
/// use debtmap::progress::implementations::CliProgressSink;
/// use debtmap::progress::traits::ProgressSink;
///
/// let sink = CliProgressSink::new(false); // verbose mode
///
/// sink.start_stage("Processing files");
/// for i in 0..10 {
///     sink.report("Processing files", i, 10);
/// }
/// sink.complete_stage("Processing files");
/// ```
#[derive(Clone, Debug)]
pub struct CliProgressSink {
    quiet: bool,
}

impl CliProgressSink {
    /// Create a new CLI progress sink.
    ///
    /// # Arguments
    ///
    /// * `quiet` - If true, suppress all progress output (warnings still shown)
    pub fn new(quiet: bool) -> Self {
        Self { quiet }
    }
}

impl Default for CliProgressSink {
    fn default() -> Self {
        Self::new(false)
    }
}

impl ProgressSink for CliProgressSink {
    fn report(&self, stage: &str, current: usize, total: usize) {
        if !self.quiet {
            // Use carriage return to overwrite the line
            eprint!("\r{}: {}/{}", stage, current + 1, total);
            let _ = std::io::stderr().flush();
        }
    }

    fn start_stage(&self, name: &str) {
        if !self.quiet {
            eprintln!("\n{}", name);
        }
    }

    fn complete_stage(&self, name: &str) {
        if !self.quiet {
            eprintln!("\n{} complete", name);
        }
    }

    fn warn(&self, message: &str) {
        // Always show warnings, even in quiet mode
        eprintln!("\nWarning: {}", message);
    }

    fn child(&self, _prefix: &str) -> Arc<dyn ProgressSink> {
        Arc::new(CliProgressSink { quiet: self.quiet })
    }
}

/// Progress event recorded by [`RecordingProgressSink`].
///
/// This enum represents all possible progress events that can be recorded
/// for testing and analysis purposes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProgressEvent {
    /// Progress report for a stage.
    Report {
        /// The stage name
        stage: String,
        /// Current progress (0-indexed)
        current: usize,
        /// Total items
        total: usize,
    },
    /// Stage started.
    StartStage {
        /// The stage name
        name: String,
    },
    /// Stage completed.
    CompleteStage {
        /// The stage name
        name: String,
    },
    /// Warning message.
    Warn {
        /// The warning message
        message: String,
    },
}

/// Recording progress sink - captures events for testing.
///
/// This implementation records all progress events to an internal vector,
/// allowing tests to verify that progress reporting is working correctly.
///
/// # Thread Safety
///
/// The internal event vector is protected by a `Mutex`, making this safe
/// to use with parallel analysis. However, the order of events may not
/// be deterministic when multiple threads are reporting simultaneously.
///
/// # Example
///
/// ```rust
/// use debtmap::progress::implementations::{RecordingProgressSink, ProgressEvent};
/// use debtmap::progress::traits::ProgressSink;
///
/// let recorder = RecordingProgressSink::new();
///
/// recorder.start_stage("Test");
/// recorder.report("Test", 0, 5);
/// recorder.complete_stage("Test");
///
/// assert_eq!(recorder.stages(), vec!["Test"]);
/// assert!(recorder.events().iter().any(|e| matches!(e, ProgressEvent::Report { .. })));
/// ```
#[derive(Clone, Debug, Default)]
pub struct RecordingProgressSink {
    events: Arc<Mutex<Vec<ProgressEvent>>>,
}

impl RecordingProgressSink {
    /// Create a new recording progress sink.
    pub fn new() -> Self {
        Self::default()
    }

    /// Get all recorded events.
    ///
    /// Returns a clone of the internal event vector.
    pub fn events(&self) -> Vec<ProgressEvent> {
        self.events.lock().unwrap().clone()
    }

    /// Get all stage names that were started.
    ///
    /// This is a convenience method for testing that extracts just the
    /// stage names from `StartStage` events.
    ///
    /// # Example
    ///
    /// ```rust
    /// use debtmap::progress::implementations::RecordingProgressSink;
    /// use debtmap::progress::traits::ProgressSink;
    ///
    /// let recorder = RecordingProgressSink::new();
    /// recorder.start_stage("Stage 1");
    /// recorder.start_stage("Stage 2");
    ///
    /// assert_eq!(recorder.stages(), vec!["Stage 1", "Stage 2"]);
    /// ```
    pub fn stages(&self) -> Vec<String> {
        self.events()
            .into_iter()
            .filter_map(|e| match e {
                ProgressEvent::StartStage { name } => Some(name),
                _ => None,
            })
            .collect()
    }

    /// Get all completed stage names.
    ///
    /// This is a convenience method for testing that extracts just the
    /// stage names from `CompleteStage` events.
    pub fn completed_stages(&self) -> Vec<String> {
        self.events()
            .into_iter()
            .filter_map(|e| match e {
                ProgressEvent::CompleteStage { name } => Some(name),
                _ => None,
            })
            .collect()
    }

    /// Get all warnings.
    ///
    /// This is a convenience method for testing that extracts just the
    /// warning messages.
    pub fn warnings(&self) -> Vec<String> {
        self.events()
            .into_iter()
            .filter_map(|e| match e {
                ProgressEvent::Warn { message } => Some(message),
                _ => None,
            })
            .collect()
    }

    /// Clear all recorded events.
    ///
    /// Useful for resetting state between test cases.
    pub fn clear(&self) {
        self.events.lock().unwrap().clear();
    }

    /// Get the number of recorded events.
    pub fn event_count(&self) -> usize {
        self.events.lock().unwrap().len()
    }
}

impl ProgressSink for RecordingProgressSink {
    fn report(&self, stage: &str, current: usize, total: usize) {
        self.events.lock().unwrap().push(ProgressEvent::Report {
            stage: stage.to_string(),
            current,
            total,
        });
    }

    fn start_stage(&self, name: &str) {
        self.events.lock().unwrap().push(ProgressEvent::StartStage {
            name: name.to_string(),
        });
    }

    fn complete_stage(&self, name: &str) {
        self.events
            .lock()
            .unwrap()
            .push(ProgressEvent::CompleteStage {
                name: name.to_string(),
            });
    }

    fn warn(&self, message: &str) {
        self.events.lock().unwrap().push(ProgressEvent::Warn {
            message: message.to_string(),
        });
    }

    fn child(&self, _prefix: &str) -> Arc<dyn ProgressSink> {
        // Share the same event recorder with children for testing
        Arc::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // SilentProgressSink Tests
    // =========================================================================

    #[test]
    fn test_silent_sink_is_no_op() {
        let sink = SilentProgressSink;

        // All these should be no-ops and not panic
        sink.start_stage("Test");
        sink.report("Test", 0, 100);
        sink.report("Test", 50, 100);
        sink.complete_stage("Test");
        sink.warn("Warning");
    }

    #[test]
    fn test_silent_sink_child_returns_silent() {
        let sink = SilentProgressSink;
        let child = sink.child("prefix");

        // Child should also be a no-op
        child.start_stage("Child Stage");
        child.complete_stage("Child Stage");
    }

    #[test]
    fn test_silent_sink_is_default() {
        let sink: SilentProgressSink = Default::default();
        sink.report("Test", 0, 1);
        // Just verify it compiles and doesn't panic
    }

    // =========================================================================
    // CliProgressSink Tests
    // =========================================================================

    #[test]
    fn test_cli_sink_quiet_mode() {
        let sink = CliProgressSink::new(true);

        // In quiet mode, these should not output anything
        // (We can't easily test stderr output, but we verify no panic)
        sink.start_stage("Test");
        sink.report("Test", 0, 100);
        sink.complete_stage("Test");
    }

    #[test]
    fn test_cli_sink_default_is_verbose() {
        let sink = CliProgressSink::default();
        assert!(!sink.quiet);
    }

    #[test]
    fn test_cli_sink_child_preserves_quiet() {
        let sink = CliProgressSink::new(true);
        let _child = sink.child("prefix");
        // Just verify it doesn't panic
    }

    // =========================================================================
    // RecordingProgressSink Tests
    // =========================================================================

    #[test]
    fn test_recording_sink_records_all_events() {
        let recorder = RecordingProgressSink::new();

        recorder.start_stage("Stage 1");
        recorder.report("Stage 1", 0, 10);
        recorder.report("Stage 1", 5, 10);
        recorder.complete_stage("Stage 1");
        recorder.warn("Test warning");

        let events = recorder.events();
        assert_eq!(events.len(), 5);

        assert!(matches!(
            &events[0],
            ProgressEvent::StartStage { name } if name == "Stage 1"
        ));
        assert!(matches!(
            &events[1],
            ProgressEvent::Report { stage, current: 0, total: 10 } if stage == "Stage 1"
        ));
        assert!(matches!(
            &events[2],
            ProgressEvent::Report { stage, current: 5, total: 10 } if stage == "Stage 1"
        ));
        assert!(matches!(
            &events[3],
            ProgressEvent::CompleteStage { name } if name == "Stage 1"
        ));
        assert!(matches!(
            &events[4],
            ProgressEvent::Warn { message } if message == "Test warning"
        ));
    }

    #[test]
    fn test_recording_sink_stages_helper() {
        let recorder = RecordingProgressSink::new();

        recorder.start_stage("Stage 1");
        recorder.start_stage("Stage 2");
        recorder.complete_stage("Stage 1");
        recorder.start_stage("Stage 3");

        assert_eq!(recorder.stages(), vec!["Stage 1", "Stage 2", "Stage 3"]);
    }

    #[test]
    fn test_recording_sink_completed_stages_helper() {
        let recorder = RecordingProgressSink::new();

        recorder.start_stage("Stage 1");
        recorder.complete_stage("Stage 1");
        recorder.start_stage("Stage 2");
        recorder.complete_stage("Stage 2");

        assert_eq!(recorder.completed_stages(), vec!["Stage 1", "Stage 2"]);
    }

    #[test]
    fn test_recording_sink_warnings_helper() {
        let recorder = RecordingProgressSink::new();

        recorder.warn("Warning 1");
        recorder.warn("Warning 2");

        assert_eq!(recorder.warnings(), vec!["Warning 1", "Warning 2"]);
    }

    #[test]
    fn test_recording_sink_clear() {
        let recorder = RecordingProgressSink::new();

        recorder.start_stage("Stage 1");
        recorder.complete_stage("Stage 1");
        assert_eq!(recorder.event_count(), 2);

        recorder.clear();
        assert_eq!(recorder.event_count(), 0);
        assert!(recorder.events().is_empty());
    }

    #[test]
    fn test_recording_sink_child_shares_events() {
        let recorder = RecordingProgressSink::new();
        let child = recorder.child("prefix");

        recorder.start_stage("Parent Stage");
        child.start_stage("Child Stage");

        let events = recorder.events();
        assert_eq!(events.len(), 2);
        assert_eq!(recorder.stages(), vec!["Parent Stage", "Child Stage"]);
    }

    #[test]
    fn test_recording_sink_is_clone() {
        let recorder = RecordingProgressSink::new();
        recorder.start_stage("Test");

        let cloned = recorder.clone();
        cloned.complete_stage("Test");

        // Both should see all events (shared Arc<Mutex>)
        assert_eq!(recorder.event_count(), 2);
        assert_eq!(cloned.event_count(), 2);
    }

    #[test]
    fn test_recording_sink_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<RecordingProgressSink>();
    }

    #[test]
    fn test_progress_event_equality() {
        let event1 = ProgressEvent::Report {
            stage: "Test".to_string(),
            current: 0,
            total: 10,
        };
        let event2 = ProgressEvent::Report {
            stage: "Test".to_string(),
            current: 0,
            total: 10,
        };
        let event3 = ProgressEvent::Report {
            stage: "Test".to_string(),
            current: 1,
            total: 10,
        };

        assert_eq!(event1, event2);
        assert_ne!(event1, event3);
    }
}
