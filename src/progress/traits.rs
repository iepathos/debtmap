//! Progress sink trait definitions for effects-based progress reporting.
//!
//! This module defines the core abstractions for progress reporting in debtmap:
//!
//! - [`ProgressSink`]: The main trait for progress receivers (TUI, CLI, silent, etc.)
//! - [`HasProgress`]: Environment extension trait for progress capability
//!
//! # Design Principles
//!
//! The progress system follows the "Pure Core, Imperative Shell" principle:
//!
//! - **Pure computation** functions don't call progress methods directly
//! - **Progress effects** are composed using combinators from [`crate::effects::progress`]
//! - **Progress sinks** handle the actual I/O (display, logging, etc.)
//!
//! # Thread Safety
//!
//! All `ProgressSink` implementations must be `Send + Sync` to support parallel
//! analysis with rayon. Progress updates may come from multiple threads concurrently.
//!
//! # Example
//!
//! ```rust,ignore
//! use debtmap::progress::traits::{ProgressSink, HasProgress};
//! use std::sync::Arc;
//!
//! // Custom progress sink
//! struct LoggingProgressSink;
//!
//! impl ProgressSink for LoggingProgressSink {
//!     fn report(&self, stage: &str, current: usize, total: usize) {
//!         log::info!("{}: {}/{}", stage, current, total);
//!     }
//!
//!     fn start_stage(&self, name: &str) {
//!         log::info!("Starting: {}", name);
//!     }
//!
//!     fn complete_stage(&self, name: &str) {
//!         log::info!("Complete: {}", name);
//!     }
//!
//!     fn warn(&self, message: &str) {
//!         log::warn!("{}", message);
//!     }
//!
//!     fn child(&self, _prefix: &str) -> Arc<dyn ProgressSink> {
//!         Arc::new(LoggingProgressSink)
//!     }
//! }
//! ```

use std::sync::Arc;

/// Progress sink abstraction - receives progress updates.
///
/// Implementations handle progress visualization (TUI, CLI, logging).
/// All methods should be cheap - expensive work should be deferred.
///
/// # Implementation Requirements
///
/// - All methods must be non-blocking
/// - Methods may be called from multiple threads concurrently
/// - Methods should not panic on invalid input (e.g., current > total)
/// - The `child` method should return a sink that prefixes stage names
///
/// # Method Costs
///
/// | Method | Expected Cost |
/// |--------|---------------|
/// | `report` | O(1), minimal allocation |
/// | `start_stage` | O(1), may log or update UI |
/// | `complete_stage` | O(1), may log or update UI |
/// | `warn` | O(1), may log |
/// | `child` | O(1), creates new Arc |
pub trait ProgressSink: Send + Sync + 'static {
    /// Report progress for a named stage.
    ///
    /// # Arguments
    ///
    /// * `stage` - The name of the current stage
    /// * `current` - The current progress count (0-indexed)
    /// * `total` - The total number of items
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// sink.report("File Analysis", 42, 100);  // 42/100 files processed
    /// ```
    fn report(&self, stage: &str, current: usize, total: usize);

    /// Report a sub-stage starting.
    ///
    /// This is called at the beginning of a stage. Implementations may:
    /// - Display a message
    /// - Start a timer
    /// - Initialize UI elements
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the stage being started
    fn start_stage(&self, name: &str);

    /// Report a stage completing.
    ///
    /// This is called when a stage finishes (successfully or with error).
    /// Implementations should clean up any resources started in `start_stage`.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the stage that completed
    fn complete_stage(&self, name: &str);

    /// Report a warning without interrupting progress.
    ///
    /// Warnings should be displayed in a way that doesn't disrupt the
    /// progress display. For example:
    /// - Log to a separate area in TUI
    /// - Print to stderr in CLI mode
    /// - Collect for later display
    ///
    /// # Arguments
    ///
    /// * `message` - The warning message
    fn warn(&self, message: &str);

    /// Create a child sink for nested progress.
    ///
    /// Child sinks are used for nested stages. The child should prefix
    /// stage names with the provided prefix for context.
    ///
    /// # Arguments
    ///
    /// * `prefix` - A prefix to prepend to stage names
    ///
    /// # Returns
    ///
    /// A new progress sink that prefixes stage names
    fn child(&self, prefix: &str) -> Arc<dyn ProgressSink>;
}

/// Environment extension for progress capability.
///
/// This trait extends the environment with progress reporting capability.
/// Environments implementing this trait can be used with progress combinators
/// like [`with_stage`](crate::effects::progress::with_stage) and
/// [`traverse_with_progress`](crate::effects::progress::traverse_with_progress).
///
/// # Example
///
/// ```rust,ignore
/// use debtmap::progress::traits::HasProgress;
/// use debtmap::env::RealEnv;
///
/// fn report_status<E: HasProgress>(env: &E) {
///     env.progress().start_stage("Analysis");
///     // ... do work ...
///     env.progress().complete_stage("Analysis");
/// }
/// ```
pub trait HasProgress {
    /// Access the progress sink for this environment.
    ///
    /// Returns a reference to the progress sink that will receive
    /// progress updates.
    fn progress(&self) -> &dyn ProgressSink;
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test that ProgressSink is object-safe
    fn _assert_object_safe(_: &dyn ProgressSink) {}

    // Test that traits have correct bounds
    fn _assert_send_sync<T: Send + Sync>() {}

    #[test]
    fn test_progress_sink_is_object_safe() {
        // This test verifies that ProgressSink can be used as a trait object
        // If this compiles, the trait is object-safe
        fn _takes_trait_object(_sink: Arc<dyn ProgressSink>) {}
    }
}
