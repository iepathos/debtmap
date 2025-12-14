//! Observability infrastructure for crash reports and debugging.
//!
//! This module provides structured error reporting and context tracking
//! to help diagnose issues during analysis. Following the Stillwater
//! principle: "Errors Should Tell Stories".
//!
//! ## Features
//!
//! - **Panic Hook**: Produces structured crash reports with context
//! - **Context Tracking**: Thread-local analysis phase and file tracking
//! - **Progress Tracking**: Atomic counters for overall analysis progress
//!
//! ## Usage
//!
//! Install the panic hook at application startup:
//!
//! ```ignore
//! use debtmap::observability::install_panic_hook;
//!
//! fn main() {
//!     install_panic_hook();
//!     // ... rest of application
//! }
//! ```
//!
//! Track context during analysis:
//!
//! ```ignore
//! use debtmap::observability::{set_phase, set_current_file, AnalysisPhase};
//!
//! fn analyze_files(files: &[PathBuf]) {
//!     let _phase = set_phase(AnalysisPhase::Parsing);
//!     for file in files {
//!         let _file_guard = set_current_file(file);
//!         // If panic occurs here, crash report shows phase and file
//!         parse_file(file)?;
//!     }
//! }
//! ```

pub mod context;
pub mod panic_hook;

pub use context::{
    get_current_context, get_progress, increment_processed, set_current_file, set_phase,
    set_phase_persistent, set_progress, AnalysisContext, AnalysisPhase, ContextGuard,
};
pub use panic_hook::install_panic_hook;
