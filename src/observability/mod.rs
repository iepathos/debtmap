//! Observability infrastructure for crash reports, tracing, and debugging.
//!
//! This module provides structured error reporting, tracing with spans, and context tracking
//! to help diagnose issues during analysis. Following the Stillwater
//! principle: "Errors Should Tell Stories" and "Pure Core, Imperative Shell".
//!
//! ## Features
//!
//! - **Panic Hook**: Produces structured crash reports with context
//! - **Structured Tracing**: Log levels and spans for hierarchical context
//! - **Context Tracking**: Thread-local analysis phase and file tracking
//! - **Progress Tracking**: Atomic counters for overall analysis progress
//! - **TUI Compatibility**: Automatic output suppression during TUI mode
//!
//! ## Usage
//!
//! Install the panic hook and initialize tracing at application startup:
//!
//! ```ignore
//! use debtmap::observability::{install_panic_hook, init_tracing};
//!
//! fn main() {
//!     install_panic_hook();
//!     init_tracing();
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
//!
//! ## Logging with Tracing
//!
//! Control verbosity with RUST_LOG environment variable:
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
//! # Debug specific modules
//! RUST_LOG=debtmap::builders=debug debtmap analyze .
//! ```

pub mod context;
pub mod panic_hook;
pub mod tracing;

pub use context::{
    get_current_context, get_progress, increment_processed, set_current_file, set_phase,
    set_phase_persistent, set_progress, AnalysisContext, AnalysisPhase, ContextGuard,
};
pub use panic_hook::install_panic_hook;
pub use tracing::{
    init_tracing, init_tracing_with_filter, is_debug_enabled, is_tui_active, set_tui_active,
};
