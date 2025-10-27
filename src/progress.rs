//! Progress feedback infrastructure for debtmap analysis.
//!
//! This module provides centralized progress management using the `indicatif` library.
//! Progress bars are shown for all major analysis phases including call graph building,
//! trait resolution, coverage loading, and unified analysis.
//!
//! # Progress Behavior
//!
//! - **Quiet Mode**: No progress output (respects `DEBTMAP_QUIET` env var and `--quiet` flag)
//! - **Non-TTY**: Gracefully disables progress bars in CI and piped output
//! - **Verbosity Levels**:
//!   - Level 0 (default): Main progress bars only
//!   - Level 1 (-v): Sub-phase progress and timing
//!   - Level 2 (-vv): Detailed per-phase metrics
//!
//! # Examples
//!
//! ```rust,no_run
//! use debtmap::progress::{ProgressConfig, ProgressManager, TEMPLATE_CALL_GRAPH};
//!
//! let config = ProgressConfig::from_env(false, 0);
//! let manager = ProgressManager::new(config);
//!
//! let progress = manager.create_bar(100, TEMPLATE_CALL_GRAPH);
//! progress.set_message("Building call graph");
//!
//! // Process files...
//! for _i in 0..100 {
//!     // Work...
//!     progress.inc(1);
//! }
//!
//! progress.finish_with_message("Call graph complete");
//! ```

use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use once_cell::sync::Lazy;
use std::sync::{Arc, Mutex};

// Progress bar templates
pub const TEMPLATE_CALL_GRAPH: &str = "🔗 {msg} {pos}/{len} files ({percent}%) - {eta}";
pub const TEMPLATE_TRAIT_RESOLUTION: &str = "🔍 {msg} {pos}/{len} traits - {eta}";
pub const TEMPLATE_COVERAGE: &str = "📊 {msg} {pos}/{len} files - {eta}";
pub const TEMPLATE_FUNCTION_ANALYSIS: &str =
    "⚙️  {msg} {pos}/{len} functions ({percent}%) - {per_sec}/sec - {eta}";
pub const TEMPLATE_FILE_ANALYSIS: &str = "📁 {msg} {pos}/{len} files ({percent}%) - {eta}";
pub const TEMPLATE_SPINNER: &str = "{spinner} {msg}";

/// Configuration for progress display behavior
#[derive(Debug, Clone, Default)]
pub struct ProgressConfig {
    /// Whether to suppress all progress output
    pub quiet_mode: bool,
    /// Verbosity level (0 = basic, 1 = detailed, 2 = very detailed)
    pub verbosity: u8,
}

impl ProgressConfig {
    /// Create progress configuration from environment and CLI arguments
    pub fn from_env(quiet: bool, verbosity: u8) -> Self {
        let env_quiet = std::env::var("DEBTMAP_QUIET").is_ok();
        Self {
            quiet_mode: quiet || env_quiet,
            verbosity,
        }
    }

    /// Determine if progress bars should be displayed
    pub fn should_show_progress(&self) -> bool {
        // Check if we're in quiet mode
        if self.quiet_mode {
            return false;
        }

        // Check if stderr is a TTY using std::io::IsTerminal
        use std::io::IsTerminal;
        std::io::stderr().is_terminal()
    }
}

/// Global progress manager instance
static GLOBAL_PROGRESS: Lazy<Arc<Mutex<Option<ProgressManager>>>> =
    Lazy::new(|| Arc::new(Mutex::new(None)));

/// Centralized progress manager for coordinating multiple progress bars
#[derive(Clone)]
pub struct ProgressManager {
    multi: Arc<MultiProgress>,
    config: ProgressConfig,
}

impl ProgressManager {
    /// Create a new progress manager with the given configuration
    pub fn new(config: ProgressConfig) -> Self {
        Self {
            multi: Arc::new(MultiProgress::new()),
            config,
        }
    }

    /// Initialize the global progress manager
    pub fn init_global(config: ProgressConfig) {
        let manager = Self::new(config);
        *GLOBAL_PROGRESS.lock().unwrap() = Some(manager);
    }

    /// Get a reference to the global progress manager
    pub fn global() -> Option<Self> {
        GLOBAL_PROGRESS.lock().unwrap().clone()
    }

    /// Create a progress bar with the given length and template
    ///
    /// Returns a hidden progress bar if progress should not be shown
    pub fn create_bar(&self, len: u64, template: &str) -> ProgressBar {
        if !self.config.should_show_progress() {
            return ProgressBar::hidden();
        }

        let pb = self.multi.add(ProgressBar::new(len));
        pb.set_style(
            ProgressStyle::default_bar()
                .template(template)
                .expect("Invalid progress bar template")
                .progress_chars("█▓▒░  "),
        );
        pb
    }

    /// Create a spinner progress bar with the given message
    ///
    /// Returns a hidden progress bar if progress should not be shown
    pub fn create_spinner(&self, msg: &str) -> ProgressBar {
        if !self.config.should_show_progress() {
            return ProgressBar::hidden();
        }

        let pb = self.multi.add(ProgressBar::new_spinner());
        pb.set_style(
            ProgressStyle::default_spinner()
                .template(TEMPLATE_SPINNER)
                .expect("Invalid spinner template")
                .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏"),
        );
        pb.set_message(msg.to_string());
        pb.enable_steady_tick(std::time::Duration::from_millis(100));
        pb
    }

    /// Create a progress bar that shows counts without a known total
    pub fn create_counter(&self, template: &str, msg: &str) -> ProgressBar {
        if !self.config.should_show_progress() {
            return ProgressBar::hidden();
        }

        let pb = self.multi.add(ProgressBar::new_spinner());
        pb.set_style(
            ProgressStyle::default_spinner()
                .template(template)
                .expect("Invalid counter template")
                .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏"),
        );
        pb.set_message(msg.to_string());
        pb.enable_steady_tick(std::time::Duration::from_millis(100));
        pb
    }

    /// Get the verbosity level
    pub fn verbosity(&self) -> u8 {
        self.config.verbosity
    }

    /// Clear all progress bars from the display
    ///
    /// This should be called before printing final output to ensure progress bars
    /// don't interfere with the terminal display.
    pub fn clear(&self) -> std::io::Result<()> {
        self.multi.clear()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quiet_mode_disables_progress() {
        std::env::set_var("DEBTMAP_QUIET", "1");
        let config = ProgressConfig::from_env(false, 0);
        assert!(!config.should_show_progress());
        std::env::remove_var("DEBTMAP_QUIET");
    }

    #[test]
    fn test_explicit_quiet_flag() {
        let config = ProgressConfig::from_env(true, 0);
        assert!(!config.should_show_progress());
    }

    #[test]
    fn test_verbosity_levels() {
        let config = ProgressConfig::from_env(false, 0);
        assert_eq!(config.verbosity, 0);

        let config = ProgressConfig::from_env(false, 2);
        assert_eq!(config.verbosity, 2);
    }

    #[test]
    fn test_progress_manager_creates_hidden_bars_in_quiet_mode() {
        let config = ProgressConfig {
            quiet_mode: true,
            verbosity: 0,
        };
        let manager = ProgressManager::new(config);

        let pb = manager.create_bar(100, TEMPLATE_CALL_GRAPH);
        assert!(pb.is_hidden());

        let spinner = manager.create_spinner("Test");
        assert!(spinner.is_hidden());
    }
}
