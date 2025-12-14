//! Structured tracing with spans for debtmap.
//!
//! This module provides structured logging controlled by the RUST_LOG environment variable.
//! Following the Stillwater principle: "Pure Core, Imperative Shell" - logging should happen
//! at effect boundaries, not inside pure computation functions.
//!
//! ## Log Levels
//!
//! - `error!` - Actual errors affecting results
//! - `warn!` - Recoverable issues
//! - `info!` - Phase-level progress (analysis phases, major milestones)
//! - `debug!` - Detailed per-file progress
//! - `trace!` - Very verbose output
//!
//! ## Usage
//!
//! Control verbosity with RUST_LOG:
//!
//! ```bash
//! # Default: warnings and errors only
//! debtmap analyze .
//!
//! # Show phase-level progress
//! RUST_LOG=info debtmap analyze .
//!
//! # Detailed debugging output
//! RUST_LOG=debug debtmap analyze .
//!
//! # Debug only debtmap crate
//! RUST_LOG=debtmap=debug debtmap analyze .
//! ```
//!
//! ## TUI Compatibility
//!
//! When the TUI is active, tracing output is suppressed to prevent display corruption.
//! Use `set_tui_active(true)` when entering TUI mode and `set_tui_active(false)` when exiting.
//!
//! For debugging TUI issues, you can write to a log file:
//!
//! ```bash
//! DEBTMAP_LOG_FILE=debtmap.log debtmap analyze .
//! ```

use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Tracks whether TUI mode is active to suppress tracing output.
static TUI_ACTIVE: AtomicBool = AtomicBool::new(false);

/// Initialize the tracing subscriber for debtmap.
///
/// This sets up structured logging with environment-based filtering.
/// Default level is `warn` (warnings and errors only).
///
/// # Panics
///
/// Panics if the tracing subscriber cannot be initialized (e.g., if called twice).
pub fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn"));

    // Check if we should log to a file instead of stderr
    if let Ok(log_file_path) = std::env::var("DEBTMAP_LOG_FILE") {
        if let Ok(file) = std::fs::File::create(&log_file_path) {
            let file = std::sync::Mutex::new(file);
            tracing_subscriber::registry()
                .with(
                    fmt::layer()
                        .with_target(false)
                        .with_ansi(false)
                        .with_writer(move || FileWriter {
                            file: &file as *const _,
                        }),
                )
                .with(filter)
                .init();
            return;
        }
    }

    // Default: stderr with TUI-aware suppression
    tracing_subscriber::registry()
        .with(
            fmt::layer()
                .with_target(false)
                .with_writer(|| TuiAwareWriter),
        )
        .with(filter)
        .init();
}

/// Initialize tracing with a custom filter string.
///
/// Useful for tests or programmatic configuration.
///
/// # Arguments
///
/// * `filter` - A filter string like "debug" or "debtmap=debug,warn"
pub fn init_tracing_with_filter(filter: &str) {
    let filter = EnvFilter::new(filter);

    tracing_subscriber::registry()
        .with(
            fmt::layer()
                .with_target(false)
                .with_writer(|| TuiAwareWriter),
        )
        .with(filter)
        .init();
}

/// Set whether TUI mode is active.
///
/// When TUI mode is active, tracing output is suppressed to prevent
/// display corruption. Call this when entering/exiting TUI mode.
///
/// # Arguments
///
/// * `active` - `true` when entering TUI mode, `false` when exiting
pub fn set_tui_active(active: bool) {
    TUI_ACTIVE.store(active, Ordering::Relaxed);
}

/// Check if TUI mode is currently active.
pub fn is_tui_active() -> bool {
    TUI_ACTIVE.load(Ordering::Relaxed)
}

/// Check if debug logging is enabled.
///
/// Use this to avoid expensive formatting when debug logging is disabled.
///
/// # Example
///
/// ```ignore
/// if is_debug_enabled() {
///     debug!(data = ?expensive_debug_format(&item), "Processing item");
/// }
/// ```
pub fn is_debug_enabled() -> bool {
    tracing::enabled!(tracing::Level::DEBUG)
}

/// A writer that suppresses output when TUI is active.
struct TuiAwareWriter;

impl Write for TuiAwareWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if TUI_ACTIVE.load(Ordering::Relaxed) {
            // Suppress output when TUI is active
            Ok(buf.len())
        } else {
            std::io::stderr().write(buf)
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        if TUI_ACTIVE.load(Ordering::Relaxed) {
            Ok(())
        } else {
            std::io::stderr().flush()
        }
    }
}

impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for TuiAwareWriter {
    type Writer = TuiAwareWriter;

    fn make_writer(&'a self) -> Self::Writer {
        TuiAwareWriter
    }
}

/// A writer that writes to a file, for debugging TUI issues.
struct FileWriter {
    file: *const std::sync::Mutex<std::fs::File>,
}

// SAFETY: FileWriter is only used with a static Mutex<File>, which is Send + Sync
unsafe impl Send for FileWriter {}

impl Write for FileWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        // SAFETY: The file pointer is valid for the lifetime of the program
        let file = unsafe { &*self.file };
        let mut guard = file.lock().unwrap();
        guard.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        // SAFETY: The file pointer is valid for the lifetime of the program
        let file = unsafe { &*self.file };
        let mut guard = file.lock().unwrap();
        guard.flush()
    }
}

impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for FileWriter {
    type Writer = FileWriter;

    fn make_writer(&'a self) -> Self::Writer {
        FileWriter { file: self.file }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tui_active_flag() {
        // Initial state should be false
        assert!(!is_tui_active());

        // Set to true
        set_tui_active(true);
        assert!(is_tui_active());

        // Set back to false
        set_tui_active(false);
        assert!(!is_tui_active());
    }
}
